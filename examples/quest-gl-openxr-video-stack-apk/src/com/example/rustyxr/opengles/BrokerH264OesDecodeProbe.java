package com.example.rustyxr.opengles;

import android.graphics.SurfaceTexture;
import android.media.MediaCodec;
import android.media.MediaFormat;
import android.os.Build;
import android.os.SystemClock;
import android.util.Base64;
import android.util.Log;
import android.view.Surface;

import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.BufferedInputStream;
import java.io.DataInputStream;
import java.io.EOFException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Random;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

public final class BrokerH264OesDecodeProbe {
    private static final String TAG = "RustyXrGles";
    private static final String STREAM_MAGIC = "RXYRVID1";
    private static final String REPORT_SCHEMA =
        "rusty.xr.quest.broker_h264_oes_decode_probe.v1";
    private static final int CODEC_H264 = 1;
    private static final int MAX_PACKET_BYTES = 1024 * 1024;
    private static final int MAX_STREAM_HEADER_METADATA_BYTES = 256 * 1024;
    private static final int DEQUEUE_TIMEOUT_US = 10000;
    private static final int BROKER_COMMAND_PORT = 8765;
    private static final int SYNTHETIC_WIDTH = 1280;
    private static final int SYNTHETIC_HEIGHT = 1280;
    private static final int SYNTHETIC_FPS = 30;
    private static final int SYNTHETIC_BITRATE_BPS = 6000000;
    private static final int SYNTHETIC_CAPTURE_MS = 45000;
    private static final String SYNTHETIC_PATTERN = "diagnostic-grid";

    private final AtomicBoolean running = new AtomicBoolean(true);
    private final EyeDecoder[] eyes;

    static {
        System.loadLibrary("rusty_xr_quest_gl_openxr_video_stack_native");
    }

    private static native void nativeBrokerH264FrameAvailable(
        int viewIndex,
        long sequence,
        long queuedPtsUs);

    private static native void nativeBrokerH264DecodeReport(String reportJson);

    private BrokerH264OesDecodeProbe(EyeDecoder left, EyeDecoder right) {
        this.eyes = new EyeDecoder[] { left, right };
    }

    public static BrokerH264OesDecodeProbe start(
        String host,
        int leftPort,
        int rightPort,
        Surface leftSurface,
        Surface rightSurface,
        SurfaceTexture leftSurfaceTexture,
        SurfaceTexture rightSurfaceTexture,
        int maxPackets,
        int connectTimeoutMs,
        int decodeTimeoutMs) {
        String targetHost = normalizeHost(host);
        int targetMaxPackets = Math.max(0, maxPackets);
        int targetConnectTimeoutMs = connectTimeoutMs > 0 ? connectTimeoutMs : 5000;
        int targetDecodeTimeoutMs = Math.max(0, decodeTimeoutMs);
        EyeDecoder left = new EyeDecoder(
            0,
            "left",
            targetHost,
            leftPort,
            leftSurface,
            leftSurfaceTexture,
            targetMaxPackets,
            targetConnectTimeoutMs,
            targetDecodeTimeoutMs);
        EyeDecoder right = new EyeDecoder(
            1,
            "right",
            targetHost,
            rightPort,
            rightSurface,
            rightSurfaceTexture,
            targetMaxPackets,
            targetConnectTimeoutMs,
            targetDecodeTimeoutMs);
        BrokerH264OesDecodeProbe probe = new BrokerH264OesDecodeProbe(left, right);
        prepareBrokerSyntheticStream(targetHost, "left", leftPort, targetMaxPackets);
        prepareBrokerSyntheticStream(targetHost, "right", rightPort, targetMaxPackets);
        emitReport(probeReport("start", targetHost, leftPort, rightPort, targetMaxPackets));
        left.start(probe.running);
        right.start(probe.running);
        return probe;
    }

    public void stop() {
        running.set(false);
        for (EyeDecoder eye : eyes) {
            eye.stop();
        }
    }

    private static String normalizeHost(String host) {
        if (host == null || host.trim().length() == 0) {
            return "127.0.0.1";
        }
        return host.trim();
    }

    private static JSONObject probeReport(
        String event,
        String host,
        int leftPort,
        int rightPort,
        int maxPackets) {
        JSONObject report = new JSONObject();
        try {
            report.put("schema", REPORT_SCHEMA);
            report.put("event", event);
            report.put("host", host);
            report.put("left_port", leftPort);
            report.put("right_port", rightPort);
            report.put("max_packets", maxPackets);
        } catch (Exception error) {
            Log.w(TAG, "Could not build broker H.264 OES probe report", error);
        }
        return report;
    }

    private static void emitReport(JSONObject report) {
        String reportJson = report.toString();
        Log.i(TAG, "Rusty XR broker H.264 OES decode report " + reportJson);
        nativeBrokerH264DecodeReport(reportJson);
    }

    private static void prepareBrokerSyntheticStream(
        String host,
        String label,
        int streamPort,
        int maxPackets) {
        JSONObject report = new JSONObject();
        try {
            sendStartCommand(host, label, streamPort, maxPackets);
            report.put("schema", REPORT_SCHEMA);
            report.put("event", "broker_prepare");
            report.put("label", label);
            report.put("host", host);
            report.put("broker_port", BROKER_COMMAND_PORT);
            report.put("stream_port", streamPort);
            report.put("width", SYNTHETIC_WIDTH);
            report.put("height", SYNTHETIC_HEIGHT);
            report.put("bitrate_bps", SYNTHETIC_BITRATE_BPS);
            report.put("synthetic_pattern", SYNTHETIC_PATTERN);
            report.put("max_packets", maxPackets);
            report.put("accepted", true);
        } catch (Throwable error) {
            try {
                report.put("schema", REPORT_SCHEMA);
                report.put("event", "broker_prepare");
                report.put("label", label);
                report.put("host", host);
                report.put("broker_port", BROKER_COMMAND_PORT);
                report.put("stream_port", streamPort);
                report.put("accepted", false);
                report.put("error", error.toString());
            } catch (Exception jsonError) {
                Log.w(TAG, "Could not build broker prepare failure report", jsonError);
            }
            Log.w(TAG, "Broker synthetic H.264 prepare failed for " + label, error);
        }
        emitReport(report);
    }

    private static void sendStartCommand(
        String host,
        String label,
        int streamPort,
        int maxPackets) throws Exception {
        Socket socket = new Socket();
        try {
            socket.connect(new InetSocketAddress(host, BROKER_COMMAND_PORT), 5000);
            socket.setSoTimeout(5000);
            InputStream input = socket.getInputStream();
            OutputStream output = socket.getOutputStream();
            byte[] nonce = ("rusty-xr-gles-h264-" + label + "-" + System.nanoTime())
                .getBytes(StandardCharsets.US_ASCII);
            String key = Base64.encodeToString(nonce, Base64.NO_WRAP);
            String request =
                "GET /rustyxr/v1/events HTTP/1.1\r\n" +
                "Host: " + host + ":" + BROKER_COMMAND_PORT + "\r\n" +
                "Upgrade: websocket\r\n" +
                "Connection: Upgrade\r\n" +
                "Sec-WebSocket-Version: 13\r\n" +
                "Sec-WebSocket-Key: " + key + "\r\n" +
                "\r\n";
            output.write(request.getBytes(StandardCharsets.US_ASCII));
            output.flush();
            String status = readHttpLine(input);
            if (status == null || !status.contains("101")) {
                throw new IllegalStateException("Broker WebSocket upgrade failed: " + status);
            }
            while (true) {
                String line = readHttpLine(input);
                if (line == null || line.length() == 0) {
                    break;
                }
            }

            readWebSocketTextFrame(input);
            sendMaskedTextFrame(output, startCommandJson(label, streamPort, maxPackets).toString());
            long deadline = SystemClock.elapsedRealtimeNanos() + 5000L * 1_000_000L;
            while (SystemClock.elapsedRealtimeNanos() < deadline) {
                String text = readWebSocketTextFrame(input);
                if (text == null || text.length() == 0) {
                    continue;
                }
                JSONObject message = new JSONObject(text);
                if ("command_ack".equals(message.optString("type", ""))) {
                    if (!message.optBoolean("accepted", false)) {
                        throw new IllegalStateException(
                            "Broker rejected " + label + " stream: " +
                                message.optString("message", ""));
                    }
                    Log.i(TAG, "Broker synthetic H.264 OES command accepted label=" +
                        label + " port=" + streamPort);
                    return;
                }
            }
            throw new IllegalStateException("Timed out waiting for broker command ack: " + label);
        } finally {
            try {
                socket.close();
            } catch (Exception ignored) {
            }
        }
    }

    private static JSONObject startCommandJson(
        String label,
        int streamPort,
        int maxPackets) throws Exception {
        JSONObject params = new JSONObject();
        params.put("device_port", streamPort);
        params.put("host_port", streamPort);
        params.put("preferred_width", SYNTHETIC_WIDTH);
        params.put("preferred_height", SYNTHETIC_HEIGHT);
        params.put("capture_ms", SYNTHETIC_CAPTURE_MS);
        params.put("max_packets", maxPackets);
        params.put("bitrate_bps", SYNTHETIC_BITRATE_BPS);
        params.put("live_stream", true);
        params.put("source_mode", "synthetic_surface");
        params.put("synthetic_pattern", SYNTHETIC_PATTERN);
        params.put("accept_timeout_ms", 60000);
        params.put("writer_queue_depth", 64);
        params.put("camera_id", "synthetic-" + label);

        JSONObject command = new JSONObject();
        command.put("type", "command");
        command.put("schema", "rusty.xr.broker.command.v1");
        command.put(
            "request_id",
            "rusty-xr-gles-synthetic-h264-" + label + "-" + System.currentTimeMillis());
        command.put("command", "media.start_synthetic_h264_stream");
        command.put("client_id", "rusty-xr-gles-broker-synthetic-h264-" + label);
        command.put("app_label", "Rusty XR GLES");
        command.put("app_version", "public-opengl-openxr-video-stack");
        command.put("params", params);
        return command;
    }

    private static final class EyeDecoder implements Runnable, SurfaceTexture.OnFrameAvailableListener {
        private final int viewIndex;
        private final String label;
        private final String host;
        private final int port;
        private final Surface surface;
        private final SurfaceTexture surfaceTexture;
        private final int maxPackets;
        private final int connectTimeoutMs;
        private final int decodeTimeoutMs;
        private final AtomicLong latestReleasedSequence = new AtomicLong(0L);
        private final AtomicLong latestReleasedPtsUs = new AtomicLong(-1L);
        private final AtomicLong frameAvailableCount = new AtomicLong(0L);
        private volatile AtomicBoolean running;
        private volatile Thread thread;
        private volatile Socket socket;

        EyeDecoder(
            int viewIndex,
            String label,
            String host,
            int port,
            Surface surface,
            SurfaceTexture surfaceTexture,
            int maxPackets,
            int connectTimeoutMs,
            int decodeTimeoutMs) {
            this.viewIndex = viewIndex;
            this.label = label;
            this.host = host;
            this.port = port;
            this.surface = surface;
            this.surfaceTexture = surfaceTexture;
            this.maxPackets = maxPackets;
            this.connectTimeoutMs = connectTimeoutMs;
            this.decodeTimeoutMs = decodeTimeoutMs;
        }

        void start(AtomicBoolean sharedRunning) {
            this.running = sharedRunning;
            surfaceTexture.setOnFrameAvailableListener(this);
            Thread newThread = new Thread(this, "RustyXrH264Oes-" + label);
            thread = newThread;
            newThread.start();
        }

        void stop() {
            surfaceTexture.setOnFrameAvailableListener(null);
            closeSocket();
            Thread currentThread = thread;
            if (currentThread != null && currentThread != Thread.currentThread()) {
                try {
                    currentThread.join(250L);
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                }
            }
        }

        @Override
        public void onFrameAvailable(SurfaceTexture ignored) {
            long count = frameAvailableCount.incrementAndGet();
            nativeBrokerH264FrameAvailable(
                viewIndex,
                latestReleasedSequence.get(),
                latestReleasedPtsUs.get());
            if (count == 1L || count % 60L == 0L) {
                report("frame_available", null, null);
            }
        }

        @Override
        public void run() {
            DecodeStats stats = new DecodeStats(viewIndex, label, host, port);
            stats.maxPackets = maxPackets;
            report("connect_start", stats, null);
            try {
                decodeStream(stats);
                report("complete", stats, null);
            } catch (Throwable error) {
                stats.errorCount++;
                report("error", stats, error);
            } finally {
                closeSocket();
            }
        }

        private void decodeStream(DecodeStats stats) throws Exception {
            Socket activeSocket = connectStreamSocket(stats);
            activeSocket.setSoTimeout(connectTimeoutMs);
            stats.connected = true;
            report("connected", stats, null);

            DataInputStream input = new DataInputStream(
                new BufferedInputStream(activeSocket.getInputStream()));
            StreamHeader header = StreamHeader.read(input);
            stats.schemaVersion = header.schemaVersion;
            stats.width = header.width;
            stats.height = header.height;
            stats.declaredPacketCount = header.declaredPacketCount;
            stats.headerMetadataBytes = header.headerMetadataBytes;
            stats.headerProjectionMetadataAttached = header.headerProjectionMetadata != null;
            stats.headerProjectionMetadata = header.headerProjectionMetadata;
            report("stream_header", stats, null);

            List<Packet> pendingPackets = new ArrayList<Packet>();
            boolean unboundedStream = header.declaredPacketCount == 0;
            while (isRunning() && shouldReadMore(header, stats, unboundedStream) &&
                pendingPackets.size() < 8) {
                Packet packet = readPacket(input, header.schemaVersion);
                recordPacket(stats, packet);
                pendingPackets.add(packet);
                if (findNalUnit(pendingPackets, 7) != null &&
                    findNalUnit(pendingPackets, 8) != null) {
                    break;
                }
            }
            if (pendingPackets.isEmpty()) {
                throw new IllegalStateException(
                    "Broker H.264 stream ended before any packets were received.");
            }

            NalUnit sps = findNalUnit(pendingPackets, 7);
            NalUnit pps = findNalUnit(pendingPackets, 8);
            stats.spsBytes = sps != null ? sps.bytes.length : 0;
            stats.ppsBytes = pps != null ? pps.bytes.length : 0;
            boolean hasCompleteCsd = sps != null && pps != null;

            MediaFormat format = MediaFormat.createVideoFormat("video/avc", header.width, header.height);
            if (sps != null) {
                format.setByteBuffer("csd-0", ByteBuffer.wrap(sps.bytes));
            }
            if (pps != null) {
                format.setByteBuffer("csd-1", ByteBuffer.wrap(pps.bytes));
            }
            if (Build.VERSION.SDK_INT >= 30) {
                format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1);
                stats.lowLatencyRequested = true;
            }

            MediaCodec decoder = MediaCodec.createDecoderByType("video/avc");
            try {
                stats.decoderName = decoder.getName();
                decoder.configure(format, surface, null, 0);
                stats.decoderConfigured = true;
                decoder.start();
                stats.decoderStarted = true;
                stats.decodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                report("decoder_started", stats, null);

                MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
                int nextPending = 0;
                long deadlineNs = decodeTimeoutMs > 0
                    ? SystemClock.elapsedRealtimeNanos() + decodeTimeoutMs * 1_000_000L
                    : Long.MAX_VALUE;
                while (isRunning() && !stats.outputEosSeen &&
                    SystemClock.elapsedRealtimeNanos() < deadlineNs) {
                    if (!stats.inputEosQueued) {
                        int inputIndex = decoder.dequeueInputBuffer(DEQUEUE_TIMEOUT_US);
                        if (inputIndex >= 0) {
                            NextPacket next = nextPacket(
                                input,
                                header,
                                stats,
                                pendingPackets,
                                nextPending,
                                unboundedStream,
                                hasCompleteCsd);
                            nextPending = next.nextPending;
                            if (next.packet != null) {
                                queuePacket(decoder, inputIndex, next.packet);
                                stats.inputBufferCount++;
                                stats.inputBytes += next.packet.payload.length;
                                stats.lastQueuedPtsUs = next.packet.ptsUs;
                            } else {
                                decoder.queueInputBuffer(
                                    inputIndex,
                                    0,
                                    0,
                                    stats.lastQueuedPtsUs,
                                    MediaCodec.BUFFER_FLAG_END_OF_STREAM);
                                stats.inputEosQueued = true;
                            }
                        }
                    }

                    int outputIndex = decoder.dequeueOutputBuffer(info, DEQUEUE_TIMEOUT_US);
                    if (outputIndex == MediaCodec.INFO_TRY_AGAIN_LATER) {
                        continue;
                    }
                    if (outputIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                        stats.outputFormatChanges++;
                        applyOutputFormat(stats, decoder.getOutputFormat());
                        continue;
                    }
                    if (outputIndex < 0) {
                        continue;
                    }

                    boolean codecConfig =
                        (info.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0;
                    boolean eos =
                        (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
                    if (!codecConfig && !eos) {
                        stats.outputBufferCount++;
                        stats.surfaceReleaseCount++;
                        long sequence = stats.surfaceReleaseCount;
                        latestReleasedSequence.set(sequence);
                        latestReleasedPtsUs.set(info.presentationTimeUs);
                        decoder.releaseOutputBuffer(outputIndex, true);
                        stats.decodedFrameCount++;
                    } else {
                        decoder.releaseOutputBuffer(outputIndex, false);
                    }
                    if (eos) {
                        stats.outputEosSeen = true;
                    }
                }
                if (stats.outputWidth == 0 || stats.outputHeight == 0) {
                    applyOutputFormat(stats, decoder.getOutputFormat());
                }
                if (stats.decodedFrameCount == 0 && stats.errorCount == 0) {
                    stats.lastError = stats.outputEosSeen
                        ? "Decoder reached end-of-stream without output frames."
                        : "Timed out before a decoded output frame was produced.";
                }
            } finally {
                stats.decodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
                try {
                    decoder.stop();
                } catch (Exception ignored) {
                }
                decoder.release();
            }
        }

        private Socket connectStreamSocket(DecodeStats stats) throws Exception {
            long deadlineNs = SystemClock.elapsedRealtimeNanos()
                + Math.max(connectTimeoutMs, 30000) * 1_000_000L;
            Throwable lastError = null;
            while ((running == null || running.get())
                && SystemClock.elapsedRealtimeNanos() < deadlineNs) {
                Socket candidate = new Socket();
                socket = candidate;
                stats.connectAttemptCount++;
                try {
                    candidate.connect(
                        new InetSocketAddress(host, port),
                        Math.max(250, Math.min(connectTimeoutMs, 1000)));
                    return candidate;
                } catch (Throwable error) {
                    lastError = error;
                    try {
                        candidate.close();
                    } catch (Exception ignored) {
                    }
                    if (socket == candidate) {
                        socket = null;
                    }
                    if (stats.connectAttemptCount == 1 || stats.connectAttemptCount % 10 == 0) {
                        Log.i(TAG, "Waiting for broker H.264 stream listener label=" +
                            label + " port=" + port +
                            " attempts=" + stats.connectAttemptCount);
                    }
                    SystemClock.sleep(100L);
                }
            }
            throw new IllegalStateException(
                "Timed out waiting for broker H.264 stream listener label=" +
                    label + " port=" + port +
                    " attempts=" + stats.connectAttemptCount,
                lastError);
        }

        private NextPacket nextPacket(
            DataInputStream input,
            StreamHeader header,
            DecodeStats stats,
            List<Packet> pendingPackets,
            int nextPending,
            boolean unboundedStream,
            boolean hasCompleteCsd) throws Exception {
            int index = nextPending;
            while (isRunning()) {
                Packet packet = null;
                if (index < pendingPackets.size()) {
                    packet = pendingPackets.get(index);
                    index++;
                } else if (shouldReadMore(header, stats, unboundedStream)) {
                    try {
                        packet = readPacket(input, header.schemaVersion);
                        recordPacket(stats, packet);
                    } catch (EOFException eof) {
                        stats.streamEndedByEof = true;
                        break;
                    }
                } else {
                    break;
                }
                if (hasCompleteCsd && packet.isCodecConfig()) {
                    stats.skippedCodecConfigInputCount++;
                    continue;
                }
                return new NextPacket(packet, index);
            }
            return new NextPacket(null, index);
        }

        private boolean shouldReadMore(
            StreamHeader header,
            DecodeStats stats,
            boolean unboundedStream) {
            if (maxPackets > 0 && stats.packetCount >= maxPackets) {
                return false;
            }
            return unboundedStream || stats.packetCount < header.declaredPacketCount;
        }

        private void report(String event, DecodeStats stats, Throwable error) {
            JSONObject report = new JSONObject();
            try {
                report.put("schema", REPORT_SCHEMA);
                report.put("event", event);
                report.put("view_index", viewIndex);
                report.put("source_eye", label);
                report.put("host", host);
                report.put("port", port);
                report.put("frame_available_count", frameAvailableCount.get());
                report.put("latest_released_sequence", latestReleasedSequence.get());
                report.put("latest_released_pts_us", latestReleasedPtsUs.get());
                if (stats != null) {
                    stats.put(report);
                }
                if (error != null) {
                    report.put("error", error.toString());
                }
            } catch (Exception jsonError) {
                Log.w(TAG, "Could not build broker H.264 OES eye report", jsonError);
            }
            emitReport(report);
        }

        private boolean isRunning() {
            AtomicBoolean sharedRunning = running;
            return sharedRunning != null && sharedRunning.get();
        }

        private void closeSocket() {
            Socket activeSocket = socket;
            socket = null;
            if (activeSocket != null) {
                try {
                    activeSocket.close();
                } catch (Exception ignored) {
                }
            }
        }
    }

    private static StreamHeader readHeaderFields(DataInputStream input) throws Exception {
        byte[] magicBytes = new byte[8];
        input.readFully(magicBytes);
        String magic = new String(magicBytes, StandardCharsets.US_ASCII);
        if (!STREAM_MAGIC.equals(magic)) {
            throw new IllegalStateException("Unexpected broker H.264 stream magic: " + magic);
        }
        int schemaVersion = input.readInt();
        int codecId = input.readInt();
        int width = input.readInt();
        int height = input.readInt();
        int declaredPacketCount = input.readInt();
        int tailWord = input.readInt();
        if (schemaVersion < 1 || schemaVersion > 3) {
            throw new IllegalStateException(
                "Unsupported broker H.264 stream schema version: " + schemaVersion);
        }
        if (codecId != CODEC_H264) {
            throw new IllegalStateException("Broker stream codec is not H.264: " + codecId);
        }
        if (width <= 0 || height <= 0) {
            throw new IllegalStateException(
                "Broker H.264 stream dimensions are invalid: " + width + "x" + height);
        }
        int headerMetadataBytes = schemaVersion >= 3 ? tailWord : 0;
        return new StreamHeader(
            schemaVersion,
            width,
            height,
            declaredPacketCount,
            headerMetadataBytes,
            null);
    }

    private static Packet readPacket(DataInputStream input, int schemaVersion) throws Exception {
        long ptsUs = input.readLong();
        int flags = input.readInt();
        int size = input.readInt();
        if (size < 0 || size > MAX_PACKET_BYTES) {
            throw new IllegalStateException("Broker H.264 packet size is out of range: " + size);
        }
        long sourceElapsedNs = 0L;
        long sourceUnixNs = 0L;
        if (schemaVersion >= 2) {
            sourceElapsedNs = input.readLong();
            sourceUnixNs = input.readLong();
        }
        byte[] payload = new byte[size];
        input.readFully(payload);
        return new Packet(ptsUs, flags, sourceElapsedNs, sourceUnixNs, payload);
    }

    private static void sendMaskedTextFrame(OutputStream output, String text) throws Exception {
        byte[] payload = text.getBytes(StandardCharsets.UTF_8);
        output.write(0x81);
        if (payload.length < 126) {
            output.write(0x80 | payload.length);
        } else if (payload.length <= 65535) {
            output.write(0x80 | 126);
            output.write((payload.length >>> 8) & 0xff);
            output.write(payload.length & 0xff);
        } else {
            output.write(0x80 | 127);
            long length = payload.length;
            for (int i = 7; i >= 0; i--) {
                output.write((int) ((length >>> (i * 8)) & 0xff));
            }
        }

        byte[] mask = new byte[4];
        new Random(System.nanoTime()).nextBytes(mask);
        output.write(mask);
        for (int i = 0; i < payload.length; i++) {
            output.write(payload[i] ^ mask[i % 4]);
        }
        output.flush();
    }

    private static String readWebSocketTextFrame(InputStream input) throws Exception {
        int first = input.read();
        if (first < 0) {
            return "";
        }
        int second = input.read();
        if (second < 0) {
            return "";
        }

        int opcode = first & 0x0f;
        boolean masked = (second & 0x80) != 0;
        long length = second & 0x7f;
        if (length == 126) {
            length = readUnsignedShort(input);
        } else if (length == 127) {
            length = readLong(input);
        }
        if (length < 0 || length > 1024 * 1024) {
            throw new IllegalStateException("Broker WebSocket frame is too large.");
        }

        byte[] mask = null;
        if (masked) {
            mask = readExact(input, 4);
        }
        byte[] payload = readExact(input, (int) length);
        if (mask != null) {
            for (int i = 0; i < payload.length; i++) {
                payload[i] = (byte) (payload[i] ^ mask[i % 4]);
            }
        }
        return opcode == 1 ? new String(payload, StandardCharsets.UTF_8) : "";
    }

    private static String readHttpLine(InputStream input) throws Exception {
        ByteArrayOutputStream buffer = new ByteArrayOutputStream();
        int previous = -1;
        while (true) {
            int value = input.read();
            if (value < 0) {
                break;
            }
            if (previous == '\r' && value == '\n') {
                break;
            }
            buffer.write(value);
            previous = value;
            if (buffer.size() > 8192) {
                throw new IllegalStateException("HTTP line exceeded 8192 bytes.");
            }
        }
        byte[] bytes = buffer.toByteArray();
        int length = bytes.length;
        if (length > 0 && bytes[length - 1] == '\r') {
            length--;
        }
        return new String(bytes, 0, length, StandardCharsets.US_ASCII);
    }

    private static int readUnsignedShort(InputStream input) throws Exception {
        int high = input.read();
        int low = input.read();
        if (high < 0 || low < 0) {
            throw new IllegalStateException("Truncated unsigned short.");
        }
        return ((high & 0xff) << 8) | (low & 0xff);
    }

    private static long readLong(InputStream input) throws Exception {
        long value = 0L;
        for (int i = 0; i < 8; i++) {
            int next = input.read();
            if (next < 0) {
                throw new IllegalStateException("Truncated long.");
            }
            value = (value << 8) | (next & 0xffL);
        }
        return value;
    }

    private static byte[] readExact(InputStream input, int length) throws Exception {
        byte[] bytes = new byte[length];
        int offset = 0;
        while (offset < length) {
            int read = input.read(bytes, offset, length - offset);
            if (read < 0) {
                throw new IllegalStateException("Truncated frame payload.");
            }
            offset += read;
        }
        return bytes;
    }

    private static void queuePacket(MediaCodec decoder, int inputIndex, Packet packet)
        throws Exception {
        ByteBuffer inputBuffer = decoder.getInputBuffer(inputIndex);
        if (inputBuffer == null) {
            throw new IllegalStateException("Decoder input buffer is unavailable.");
        }
        if (packet.payload.length > inputBuffer.capacity()) {
            throw new IllegalStateException("Encoded packet exceeds decoder input capacity.");
        }
        inputBuffer.clear();
        inputBuffer.put(packet.payload);
        int queueFlags = packet.isCodecConfig() ? MediaCodec.BUFFER_FLAG_CODEC_CONFIG : 0;
        decoder.queueInputBuffer(inputIndex, 0, packet.payload.length, packet.ptsUs, queueFlags);
    }

    private static void recordPacket(DecodeStats stats, Packet packet) {
        stats.packetCount++;
        stats.payloadBytes += packet.payload.length;
        stats.lastQueuedPtsUs = packet.ptsUs;
        if (packet.isCodecConfig()) {
            stats.codecConfigPacketCount++;
        }
        if ((packet.flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0) {
            stats.keyFramePacketCount++;
        }
        if (packet.sourceElapsedNs > 0L) {
            if (stats.firstSourceElapsedNs == 0L) {
                stats.firstSourceElapsedNs = packet.sourceElapsedNs;
            }
            stats.lastSourceElapsedNs = packet.sourceElapsedNs;
        }
    }

    private static NalUnit findNalUnit(List<Packet> packets, int nalType) {
        for (int i = 0; i < packets.size(); i++) {
            byte[] payload = packets.get(i).payload;
            int start = findStartCode(payload, 0);
            while (start >= 0) {
                int startCodeLength = startCodeLengthAt(payload, start);
                int nalStart = start + startCodeLength;
                if (nalStart >= payload.length) {
                    break;
                }
                int nextStart = findStartCode(payload, nalStart);
                int nalEnd = nextStart >= 0 ? nextStart : payload.length;
                if ((payload[nalStart] & 0x1f) == nalType) {
                    byte[] bytes = new byte[nalEnd - start];
                    System.arraycopy(payload, start, bytes, 0, bytes.length);
                    return new NalUnit(bytes);
                }
                start = nextStart;
            }
        }
        return null;
    }

    private static int findStartCode(byte[] data, int offset) {
        for (int i = Math.max(0, offset); i < data.length - 2; i++) {
            if (startCodeLengthAt(data, i) > 0) {
                return i;
            }
        }
        return -1;
    }

    private static int startCodeLengthAt(byte[] data, int offset) {
        if (offset + 4 <= data.length &&
            data[offset] == 0 &&
            data[offset + 1] == 0 &&
            data[offset + 2] == 0 &&
            data[offset + 3] == 1) {
            return 4;
        }
        if (offset + 3 <= data.length &&
            data[offset] == 0 &&
            data[offset + 1] == 0 &&
            data[offset + 2] == 1) {
            return 3;
        }
        return 0;
    }

    private static void applyOutputFormat(DecodeStats stats, MediaFormat format) {
        stats.outputMime = mediaFormatString(format, MediaFormat.KEY_MIME, "video/raw");
        stats.outputWidth = mediaFormatInt(format, MediaFormat.KEY_WIDTH, stats.width);
        stats.outputHeight = mediaFormatInt(format, MediaFormat.KEY_HEIGHT, stats.height);
    }

    private static String mediaFormatString(MediaFormat format, String key, String fallback) {
        try {
            if (format.containsKey(key)) {
                return format.getString(key);
            }
        } catch (Exception ignored) {
        }
        return fallback;
    }

    private static int mediaFormatInt(MediaFormat format, String key, int fallback) {
        try {
            if (format.containsKey(key)) {
                return format.getInteger(key);
            }
        } catch (Exception ignored) {
        }
        return fallback;
    }

    private static final class StreamHeader {
        final int schemaVersion;
        final int width;
        final int height;
        final int declaredPacketCount;
        final int headerMetadataBytes;
        final JSONObject headerProjectionMetadata;

        StreamHeader(
            int schemaVersion,
            int width,
            int height,
            int declaredPacketCount,
            int headerMetadataBytes,
            JSONObject headerProjectionMetadata) {
            this.schemaVersion = schemaVersion;
            this.width = width;
            this.height = height;
            this.declaredPacketCount = declaredPacketCount;
            this.headerMetadataBytes = headerMetadataBytes;
            this.headerProjectionMetadata = headerProjectionMetadata;
        }

        static StreamHeader read(DataInputStream input) throws Exception {
            StreamHeader header = readHeaderFields(input);
            JSONObject metadata = null;
            if (header.schemaVersion >= 3 && header.headerMetadataBytes > 0) {
                if (header.headerMetadataBytes > MAX_STREAM_HEADER_METADATA_BYTES) {
                    throw new IllegalStateException(
                        "Broker H.264 header metadata is too large: " +
                            header.headerMetadataBytes);
                }
                byte[] payload = new byte[header.headerMetadataBytes];
                input.readFully(payload);
                metadata = new JSONObject(new String(payload, StandardCharsets.UTF_8));
            }
            return new StreamHeader(
                header.schemaVersion,
                header.width,
                header.height,
                header.declaredPacketCount,
                header.headerMetadataBytes,
                metadata);
        }
    }

    private static final class Packet {
        final long ptsUs;
        final int flags;
        final long sourceElapsedNs;
        final long sourceUnixNs;
        final byte[] payload;

        Packet(long ptsUs, int flags, long sourceElapsedNs, long sourceUnixNs, byte[] payload) {
            this.ptsUs = ptsUs;
            this.flags = flags;
            this.sourceElapsedNs = sourceElapsedNs;
            this.sourceUnixNs = sourceUnixNs;
            this.payload = payload;
        }

        boolean isCodecConfig() {
            return (flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0;
        }
    }

    private static final class NalUnit {
        final byte[] bytes;

        NalUnit(byte[] bytes) {
            this.bytes = bytes;
        }
    }

    private static final class NextPacket {
        final Packet packet;
        final int nextPending;

        NextPacket(Packet packet, int nextPending) {
            this.packet = packet;
            this.nextPending = nextPending;
        }
    }

    private static final class DecodeStats {
        final int viewIndex;
        final String label;
        final String host;
        final int port;
        int maxPackets;
        boolean connected;
        int schemaVersion;
        int width;
        int height;
        int declaredPacketCount;
        int headerMetadataBytes;
        boolean headerProjectionMetadataAttached;
        JSONObject headerProjectionMetadata;
        int packetCount;
        int codecConfigPacketCount;
        int keyFramePacketCount;
        int payloadBytes;
        int spsBytes;
        int ppsBytes;
        String decoderName = "";
        boolean lowLatencyRequested;
        boolean decoderConfigured;
        boolean decoderStarted;
        int inputBufferCount;
        int inputBytes;
        int skippedCodecConfigInputCount;
        boolean inputEosQueued;
        boolean outputEosSeen;
        int outputBufferCount;
        int outputFormatChanges;
        int outputWidth;
        int outputHeight;
        String outputMime = "";
        int decodedFrameCount;
        long surfaceReleaseCount;
        long lastQueuedPtsUs = -1L;
        long firstSourceElapsedNs;
        long lastSourceElapsedNs;
        boolean streamEndedByEof;
        long decodeStartElapsedNs;
        long decodeEndElapsedNs;
        int connectAttemptCount;
        int errorCount;
        String lastError = "";

        DecodeStats(int viewIndex, String label, String host, int port) {
            this.viewIndex = viewIndex;
            this.label = label;
            this.host = host;
            this.port = port;
        }

        void put(JSONObject report) throws Exception {
            report.put("view_index", viewIndex);
            report.put("source_eye", label);
            report.put("host", host);
            report.put("port", port);
            report.put("max_packets", maxPackets);
            report.put("connected", connected);
            report.put("schema_version", schemaVersion);
            report.put("width", width);
            report.put("height", height);
            report.put("declared_packet_count", declaredPacketCount);
            report.put("header_metadata_bytes", headerMetadataBytes);
            report.put("header_projection_metadata_attached", headerProjectionMetadataAttached);
            if (headerProjectionMetadata != null) {
                report.put(
                    "header_projection_metadata_ready",
                    headerProjectionMetadata.optBoolean("projectionMetadataReady", false));
                report.put(
                    "header_projection_metadata_source",
                    headerProjectionMetadata.optString("source", ""));
                report.put(
                    "header_projection_metadata_camera_id",
                    headerProjectionMetadata.optString("cameraId", ""));
                report.put(
                    "header_projection_metadata_synthetic_pattern",
                    headerProjectionMetadata.optString("syntheticPattern", ""));
                report.put("header_projection_metadata", headerProjectionMetadata);
            }
            report.put("packet_count", packetCount);
            report.put("codec_config_packet_count", codecConfigPacketCount);
            report.put("key_frame_packet_count", keyFramePacketCount);
            report.put("payload_bytes", payloadBytes);
            report.put("sps_bytes", spsBytes);
            report.put("pps_bytes", ppsBytes);
            report.put("decoder_name", decoderName);
            report.put("low_latency_requested", lowLatencyRequested);
            report.put("decoder_configured", decoderConfigured);
            report.put("decoder_started", decoderStarted);
            report.put("input_buffer_count", inputBufferCount);
            report.put("input_bytes", inputBytes);
            report.put("skipped_codec_config_input_count", skippedCodecConfigInputCount);
            report.put("input_eos_queued", inputEosQueued);
            report.put("output_eos_seen", outputEosSeen);
            report.put("output_buffer_count", outputBufferCount);
            report.put("output_format_changes", outputFormatChanges);
            report.put("output_width", outputWidth);
            report.put("output_height", outputHeight);
            report.put("output_mime", outputMime);
            report.put("decoded_frame_count", decodedFrameCount);
            report.put("surface_release_count", surfaceReleaseCount);
            report.put("last_queued_pts_us", lastQueuedPtsUs);
            report.put("first_source_elapsed_ns", firstSourceElapsedNs);
            report.put("last_source_elapsed_ns", lastSourceElapsedNs);
            report.put("stream_ended_by_eof", streamEndedByEof);
            report.put("decode_start_elapsed_ns", decodeStartElapsedNs);
            report.put("decode_end_elapsed_ns", decodeEndElapsedNs);
            report.put("connect_attempt_count", connectAttemptCount);
            report.put("error_count", errorCount);
            if (lastError.length() > 0) {
                report.put("last_error", lastError);
            }
        }
    }
}
