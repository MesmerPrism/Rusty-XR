package com.example.rustyxr.broker;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.Rect;
import android.graphics.ImageFormat;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CameraMetadata;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaCodecList;
import android.media.MediaFormat;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;
import android.util.Base64;
import android.util.Range;
import android.util.Size;
import android.view.Surface;

import org.json.JSONObject;

import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.ArrayDeque;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

final class BrokerAppCameraH264StreamSession {
    private static final String STREAM_SCHEMA = "rusty.xr.video_lab.binary_stream.v1";
    private static final String MAGIC = "RXYRVID1";
    private static final int SCHEMA_VERSION = 2;
    private static final int CODEC_H264 = 1;
    private static final int DEFAULT_PORT = 8879;
    private static final int DEFAULT_HOST_PORT = 18879;
    private static final int DEFAULT_WIDTH = 720;
    private static final int DEFAULT_HEIGHT = 480;
    private static final int DEFAULT_CAPTURE_MS = 900;
    private static final int MAX_CAPTURE_MS = 3000;
    private static final int MAX_LIVE_CAPTURE_MS = 60000;
    private static final int DEFAULT_MAX_PACKETS = 12;
    private static final int MAX_PACKETS = 30;
    private static final int MAX_LIVE_PACKETS = 2400;
    private static final int DEFAULT_BITRATE_BPS = 1_000_000;
    private static final int FRAME_RATE_HZ = 30;
    private static final int OPEN_TIMEOUT_MS = 4000;
    private static final int SESSION_TIMEOUT_MS = 4000;
    private static final int ENCODER_DRAIN_TIMEOUT_US = 10000;
    private static final int BINARY_STREAM_MAX_PACKET_BYTES = 1024 * 1024;
    private static final int MAX_CODEC_CONFIG_PACKETS = 8;
    private static final int DEFAULT_LIVE_WRITER_QUEUE_DEPTH = 48;
    private static final int MAX_LIVE_WRITER_QUEUE_DEPTH = 512;
    private static final int WRITER_QUEUE_POLL_MS = 100;
    private static final int WRITER_JOIN_TIMEOUT_MS = 5000;
    private static final String MIME_H264 = "video/avc";

    interface Sink {
        void registerManifest(JSONObject manifest) throws Exception;

        void recordSample(JSONObject sample) throws Exception;

        void recordMetric(JSONObject metric) throws Exception;
    }

    private BrokerAppCameraH264StreamSession() {
    }

    static JSONObject start(Context context, JSONObject params, Sink sink) throws Exception {
        final Context appContext = context != null ? context.getApplicationContext() : null;
        final String sessionId = normalizeSessionId(
            params != null ? params.optString("session_id", "") : "",
            "broker-app-camera-h264-");
        final int devicePort = clamp(params != null ? params.optInt("device_port", DEFAULT_PORT) : DEFAULT_PORT, 1, 65535);
        final int hostPort = clamp(params != null ? params.optInt("host_port", DEFAULT_HOST_PORT) : DEFAULT_HOST_PORT, 1, 65535);
        final int preferredWidth = clamp(params != null ? params.optInt("preferred_width", DEFAULT_WIDTH) : DEFAULT_WIDTH, 16, 4096);
        final int preferredHeight = clamp(params != null ? params.optInt("preferred_height", DEFAULT_HEIGHT) : DEFAULT_HEIGHT, 16, 4096);
        final boolean liveStream = params != null && params.optBoolean("live_stream", false);
        final int requestedCaptureMs = params != null ? params.optInt("capture_ms", DEFAULT_CAPTURE_MS) : DEFAULT_CAPTURE_MS;
        final int requestedMaxPackets = params != null ? params.optInt("max_packets", DEFAULT_MAX_PACKETS) : DEFAULT_MAX_PACKETS;
        final int captureMs = liveStream && requestedCaptureMs <= 0
            ? 0
            : clamp(
                requestedCaptureMs,
                100,
                liveStream ? MAX_LIVE_CAPTURE_MS : MAX_CAPTURE_MS);
        final int maxPackets = liveStream && requestedMaxPackets <= 0
            ? 0
            : clamp(
                requestedMaxPackets,
                1,
                liveStream ? MAX_LIVE_PACKETS : MAX_PACKETS);
        final int writerQueueDepth = clamp(
            params != null ? params.optInt("writer_queue_depth", DEFAULT_LIVE_WRITER_QUEUE_DEPTH) : DEFAULT_LIVE_WRITER_QUEUE_DEPTH,
            1,
            MAX_LIVE_WRITER_QUEUE_DEPTH);
        final int bitrateBps = clamp(params != null ? params.optInt("bitrate_bps", DEFAULT_BITRATE_BPS) : DEFAULT_BITRATE_BPS, 100_000, 20_000_000);
        final String requestedCameraId = params != null ? params.optString("camera_id", "").trim() : "";
        final boolean lanStreamEnabled = params != null && params.optBoolean("lan_stream_enabled", false);
        final String bindHost = normalizeBindHost(
            params != null ? params.optString("bind_host", "") : "",
            lanStreamEnabled);
        final String advertisedHost = normalizeAdvertisedHost(
            params != null ? params.optString("advertised_host", "") : "",
            bindHost);

        JSONObject endpoint = new JSONObject();
        endpoint.put("host", advertisedHost);
        endpoint.put("bind_host", bindHost);
        endpoint.put("lan_stream_enabled", lanStreamEnabled);
        endpoint.put("device_port", devicePort);
        endpoint.put("host_port", hostPort);
        endpoint.put("framing", STREAM_SCHEMA);
        endpoint.put("magic", MAGIC);
        endpoint.put("codec_id", CODEC_H264);
        endpoint.put("codec", "h264");
        endpoint.put("schema_version", SCHEMA_VERSION);
        endpoint.put("packet_header", "pts_us,flags,size,source_time_elapsed_ns,source_time_unix_ns");
        endpoint.put("writer_queue_depth", writerQueueDepth);

        JSONObject start = new JSONObject();
        start.put("schema", "rusty.xr.camera_provider.app_camera_h264_stream_start.v1");
        start.put("session_id", sessionId);
        start.put("stream_id", "broker_app.camera_h264");
        start.put("source", "broker_app_camera2_mediacodec_surface");
        start.put("state", "starting");
        start.put("camera_id", requestedCameraId);
        start.put("preferred_width", preferredWidth);
        start.put("preferred_height", preferredHeight);
        start.put("capture_ms", captureMs);
        start.put("max_packets", maxPackets);
        start.put("writer_queue_depth", writerQueueDepth);
        start.put("bitrate_bps", bitrateBps);
        start.put("live_stream", liveStream);
        start.put("stream_mode", streamMode(liveStream, captureMs, maxPackets));
        start.put("binary_endpoint", endpoint);
        try {
            if (appContext != null &&
                    appContext.checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
                CameraManager manager = (CameraManager) appContext.getSystemService(Context.CAMERA_SERVICE);
                if (manager != null) {
                    CameraSelection selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight);
                    start.put("selected_camera_id", selection.cameraId);
                    start.put("selected_width", selection.size.getWidth());
                    start.put("selected_height", selection.size.getHeight());
                    start.put("selection_score", selection.score);
                    start.put("projection_metadata", buildProjectionMetadata(selection));
                }
            }
        } catch (Exception ex) {
            start.put("projection_metadata_error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
        }

        Thread thread = new Thread(new Runnable() {
            @Override
            public void run() {
                runSession(
                    appContext,
                    sink,
                    sessionId,
                    requestedCameraId,
                    devicePort,
                    bindHost,
                    endpoint,
                    preferredWidth,
                    preferredHeight,
                    captureMs,
                    maxPackets,
                    writerQueueDepth,
                    bitrateBps,
                    liveStream);
            }
        }, "RustyXrAppCameraH264Stream");
        thread.start();
        return start;
    }

    static CaptureResult capturePacketsForProbe(Context context, JSONObject params) throws Exception {
        final Context appContext = context != null ? context.getApplicationContext() : null;
        final String sessionId = normalizeSessionId(
            params != null ? params.optString("session_id", "") : "",
            "broker-app-camera-h264-decode-");
        final int preferredWidth = clamp(params != null ? params.optInt("preferred_width", DEFAULT_WIDTH) : DEFAULT_WIDTH, 16, 4096);
        final int preferredHeight = clamp(params != null ? params.optInt("preferred_height", DEFAULT_HEIGHT) : DEFAULT_HEIGHT, 16, 4096);
        final int captureMs = clamp(params != null ? params.optInt("capture_ms", DEFAULT_CAPTURE_MS) : DEFAULT_CAPTURE_MS, 100, MAX_CAPTURE_MS);
        final int maxPackets = clamp(params != null ? params.optInt("max_packets", DEFAULT_MAX_PACKETS) : DEFAULT_MAX_PACKETS, 1, MAX_PACKETS);
        final int bitrateBps = clamp(params != null ? params.optInt("bitrate_bps", DEFAULT_BITRATE_BPS) : DEFAULT_BITRATE_BPS, 100_000, 20_000_000);
        final String requestedCameraId = params != null ? params.optString("camera_id", "").trim() : "";

        if (appContext == null) {
            throw new IllegalStateException("Broker app context is unavailable.");
        }
        if (appContext.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
            throw new SecurityException("Broker app camera permission is not granted.");
        }

        CameraManager manager = (CameraManager) appContext.getSystemService(Context.CAMERA_SERVICE);
        if (manager == null) {
            throw new IllegalStateException("CameraManager is unavailable.");
        }

        CameraSelection selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight);
        EncoderMetadata encoderMetadata = new EncoderMetadata();
        encoderMetadata.sensorTimestampSource = sensorTimestampSourceLabel(
            selection.characteristics.get(CameraCharacteristics.SENSOR_INFO_TIMESTAMP_SOURCE));
        long encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        List<EncodedPacket> packets = encodeCameraPackets(
            manager,
            selection.cameraId,
            selection.size,
            captureMs,
            maxPackets,
            bitrateBps,
            encoderMetadata);
        long encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
        return new CaptureResult(
            sessionId,
            requestedCameraId,
            selection.cameraId,
            selection.size,
            captureMs,
            maxPackets,
            bitrateBps,
            encodeStartElapsedNs,
            encodeEndElapsedNs,
            packets,
            encoderMetadata);
    }

    private static void runSession(
        Context context,
        Sink sink,
        String sessionId,
        String requestedCameraId,
        int devicePort,
        String bindHost,
        JSONObject endpoint,
        int preferredWidth,
        int preferredHeight,
        int captureMs,
        int maxPackets,
        int writerQueueDepth,
        int bitrateBps,
        boolean liveStream) {
        long encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long encodeEndElapsedNs = encodeStartElapsedNs;
        StreamWriteStats writeStats = new StreamWriteStats(0L, 0L, 0L, 0L);
        List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        EncoderMetadata encoderMetadata = new EncoderMetadata();
        String cameraId = requestedCameraId;
        Size size = null;
        String lastError = "";
        try {
            if (context == null) {
                throw new IllegalStateException("Broker app context is unavailable.");
            }
            if (context.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
                throw new SecurityException("Broker app camera permission is not granted.");
            }

            CameraManager manager = (CameraManager) context.getSystemService(Context.CAMERA_SERVICE);
            if (manager == null) {
                throw new IllegalStateException("CameraManager is unavailable.");
            }
            CameraSelection selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight);
            cameraId = selection.cameraId;
            size = selection.size;
            encoderMetadata.sensorTimestampSource = sensorTimestampSourceLabel(
                selection.characteristics.get(CameraCharacteristics.SENSOR_INFO_TIMESTAMP_SOURCE));
            registerManifest(sink, sessionId, cameraId, size, captureMs, maxPackets, bitrateBps, liveStream, endpoint, encoderMetadata);
            encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (liveStream) {
                LiveStreamResult liveResult = streamCameraPacketsLive(
                    manager,
                    cameraId,
                    size,
                    captureMs,
                    maxPackets,
                    writerQueueDepth,
                    bitrateBps,
                    devicePort,
                    bindHost,
                    sink,
                    sessionId,
                    endpoint,
                    encoderMetadata);
                packets = liveResult.packets;
                writeStats = liveResult.writeStats;
                encodeEndElapsedNs = liveResult.encodeEndElapsedNs;
            } else {
                packets = encodeCameraPackets(manager, cameraId, size, captureMs, maxPackets, bitrateBps, encoderMetadata);
                encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
                registerManifest(sink, sessionId, cameraId, size, captureMs, maxPackets, bitrateBps, liveStream, endpoint, encoderMetadata);
                for (int i = 0; i < packets.size(); i++) {
                    recordSample(sink, sessionId, cameraId, size, i, packets.get(i), false);
                }
                writeStats = writePackets(devicePort, bindHost, size, packets);
            }
        } catch (Exception ex) {
            encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            lastError = ex.getClass().getSimpleName() + ": " + safeMessage(ex);
        } finally {
            try {
                recordMetric(
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    packets,
                    encodeStartElapsedNs,
                    encodeEndElapsedNs,
                    writeStats,
                    captureMs,
                    maxPackets,
                    liveStream,
                    encoderMetadata,
                    lastError);
            } catch (Exception ignored) {
            }
        }
    }

    private static List<EncodedPacket> encodeCameraPackets(
        final CameraManager manager,
        final String cameraId,
        final Size size,
        final int captureMs,
        final int maxPackets,
        final int bitrateBps,
        final EncoderMetadata encoderMetadata) throws Exception {
        final List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        HandlerThread thread = new HandlerThread("RustyXrAppCameraH264Capture");
        thread.start();
        Handler handler = new Handler(thread.getLooper());
        EncoderSelection encoderSelection = selectH264Encoder(size, bitrateBps);
        MediaCodec encoder = createH264Encoder(encoderSelection, encoderMetadata);
        Surface encoderSurface = null;
        CaptureTimingTracker captureTiming = new CaptureTimingTracker();
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        try {
            applyEncoderSelectionMetadata(encoderSelection, encoderMetadata, encoder);
            configureH264Encoder(encoder, size, bitrateBps, encoderMetadata);
            encoderSurface = encoder.createInputSurface();
            encoder.start();
            requestSyncFrameOnStart(encoder, encoderMetadata);

            deviceRef[0] = openCamera(manager, cameraId, handler);
            sessionRef[0] = configureSession(deviceRef[0], encoderSurface, handler);
            CaptureRequest.Builder builder = createRecordRequest(deviceRef[0]);
            builder.addTarget(encoderSurface);
            sessionRef[0].setRepeatingRequest(builder.build(), captureTiming, handler);

            long deadlineElapsedNs = SystemClock.elapsedRealtimeNanos() + captureMs * 1_000_000L;
            while (SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs && videoPacketCount(packets) < maxPackets) {
                drainEncoder(encoder, packets, false, maxPackets, encoderMetadata);
                Thread.sleep(10);
            }
            try {
                sessionRef[0].stopRepeating();
            } catch (Exception ignored) {
            }
            encoder.signalEndOfInputStream();
            drainEncoder(encoder, packets, true, maxPackets, encoderMetadata);
            encoderMetadata.copyCaptureTiming(captureTiming);
            if (videoPacketCount(packets) == 0) {
                throw new IllegalStateException("MediaCodec produced no app-camera H.264 packets.");
            }
            return packets;
        } finally {
            closeQuietly(sessionRef[0]);
            closeQuietly(deviceRef[0]);
            if (encoderSurface != null) {
                encoderSurface.release();
            }
            try {
                encoder.stop();
            } catch (Exception ignored) {
            }
            encoder.release();
            thread.quitSafely();
        }
    }

    private static LiveStreamResult streamCameraPacketsLive(
        final CameraManager manager,
        final String cameraId,
        final Size size,
        final int captureMs,
        final int maxPackets,
        final int writerQueueDepth,
        final int bitrateBps,
        final int devicePort,
        final String bindHost,
        final Sink sink,
        final String sessionId,
        final JSONObject endpoint,
        final EncoderMetadata encoderMetadata) throws Exception {
        final List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        HandlerThread thread = new HandlerThread("RustyXrAppCameraH264LiveCapture");
        thread.start();
        Handler handler = new Handler(thread.getLooper());
        EncoderSelection encoderSelection = selectH264Encoder(size, bitrateBps);
        MediaCodec encoder = createH264Encoder(encoderSelection, encoderMetadata);
        Surface encoderSurface = null;
        CaptureTimingTracker captureTiming = new CaptureTimingTracker();
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        ServerSocket server = null;
        Socket client = null;
        long listenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long acceptElapsedNs = 0L;
        long writeStartElapsedNs = 0L;
        long writeEndElapsedNs = 0L;
        long encodeEndElapsedNs = listenStartElapsedNs;
        final LivePacketQueue packetQueue = new LivePacketQueue(writerQueueDepth);
        final LiveStreamWriter[] writerRef = new LiveStreamWriter[1];
        Thread writerThread = null;
        try {
            server = new ServerSocket(devicePort, 1, InetAddress.getByName(bindHost));
            server.setSoTimeout(15000);
            client = server.accept();
            acceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            client.setTcpNoDelay(true);
            OutputStream output = client.getOutputStream();
            LiveStreamWriter writer = new LiveStreamWriter(
                output,
                size,
                maxPackets,
                packetQueue,
                packets,
                sink,
                sessionId,
                cameraId,
                true);
            writerRef[0] = writer;
            writerThread = new Thread(writer, "RustyXrAppCameraH264Writer");
            writerThread.start();

            applyEncoderSelectionMetadata(encoderSelection, encoderMetadata, encoder);
            configureH264Encoder(encoder, size, bitrateBps, encoderMetadata);
            encoderSurface = encoder.createInputSurface();
            encoder.start();
            requestSyncFrameOnStart(encoder, encoderMetadata);

            deviceRef[0] = openCamera(manager, cameraId, handler);
            sessionRef[0] = configureSession(deviceRef[0], encoderSurface, handler);
            CaptureRequest.Builder builder = createRecordRequest(deviceRef[0]);
            builder.addTarget(encoderSurface);
            sessionRef[0].setRepeatingRequest(builder.build(), captureTiming, handler);

            long deadlineElapsedNs = captureMs > 0
                ? SystemClock.elapsedRealtimeNanos() + captureMs * 1_000_000L
                : Long.MAX_VALUE;
            while (SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs &&
                    (maxPackets <= 0 || packetQueue.acceptedPacketCount() < maxPackets) &&
                    (maxPackets <= 0 || writer.writtenPacketCount() < maxPackets) &&
                    !writer.hasError()) {
                drainEncoderToQueue(
                    encoder,
                    packetQueue,
                    false,
                    maxPackets,
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    captureMs,
                    bitrateBps,
                    endpoint,
                    encoderMetadata);
                Thread.sleep(5);
            }
            try {
                sessionRef[0].stopRepeating();
            } catch (Exception ignored) {
            }
            encoder.signalEndOfInputStream();
            drainEncoderToQueue(
                encoder,
                packetQueue,
                true,
                maxPackets,
                sink,
                sessionId,
                cameraId,
                size,
                captureMs,
                bitrateBps,
                endpoint,
                encoderMetadata);
            encoderMetadata.copyCaptureTiming(captureTiming);
            packetQueue.close();
            joinWriterThread(writerThread, client);
            if (writer.hasError() && packets.size() == 0) {
                throw writer.error();
            }
            writeStartElapsedNs = writer.writeStartElapsedNs();
            writeEndElapsedNs = writer.writeEndElapsedNs();
            encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (packets.size() == 0) {
                throw new IllegalStateException("MediaCodec produced no live app-camera H.264 packets.");
            }
            return new LiveStreamResult(
                packets,
                new StreamWriteStats(
                    listenStartElapsedNs,
                    acceptElapsedNs,
                    writeStartElapsedNs,
                    writeEndElapsedNs,
                    packetQueue,
                    writer),
                encodeEndElapsedNs);
        } finally {
            packetQueue.close();
            if (writerThread != null && writerThread.isAlive()) {
                closeQuietly(client);
                joinQuietly(writerThread, 250L);
            }
            closeQuietly(sessionRef[0]);
            closeQuietly(deviceRef[0]);
            if (encoderSurface != null) {
                encoderSurface.release();
            }
            try {
                encoder.stop();
            } catch (Exception ignored) {
            }
            encoder.release();
            closeQuietly(client);
            if (server != null) {
                try {
                    server.close();
                } catch (Exception ignored) {
                }
            }
            thread.quitSafely();
        }
    }

    private static CameraDevice openCamera(CameraManager manager, String cameraId, Handler handler) throws Exception {
        final CountDownLatch latch = new CountDownLatch(1);
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final Exception[] errorRef = new Exception[1];
        manager.openCamera(cameraId, new CameraDevice.StateCallback() {
            @Override
            public void onOpened(CameraDevice camera) {
                deviceRef[0] = camera;
                latch.countDown();
            }

            @Override
            public void onDisconnected(CameraDevice camera) {
                errorRef[0] = new IllegalStateException("Camera disconnected.");
                if (camera != null) {
                    camera.close();
                }
                latch.countDown();
            }

            @Override
            public void onError(CameraDevice camera, int error) {
                errorRef[0] = new CameraAccessException(CameraAccessException.CAMERA_ERROR, "Camera error " + error);
                if (camera != null) {
                    camera.close();
                }
                latch.countDown();
            }
        }, handler);
        if (!latch.await(OPEN_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
            throw new IllegalStateException("Timed out opening camera " + cameraId + ".");
        }
        if (deviceRef[0] == null) {
            throw errorRef[0] != null ? errorRef[0] : new IllegalStateException("Camera open failed.");
        }
        return deviceRef[0];
    }

    private static CameraCaptureSession configureSession(
        CameraDevice device,
        Surface surface,
        Handler handler) throws Exception {
        final CountDownLatch latch = new CountDownLatch(1);
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        final String[] errorRef = new String[1];
        device.createCaptureSession(
            Arrays.asList(surface),
            new CameraCaptureSession.StateCallback() {
                @Override
                public void onConfigured(CameraCaptureSession session) {
                    sessionRef[0] = session;
                    latch.countDown();
                }

                @Override
                public void onConfigureFailed(CameraCaptureSession session) {
                    errorRef[0] = "Camera capture session configure failed.";
                    latch.countDown();
                }
            },
            handler);
        if (!latch.await(SESSION_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
            throw new IllegalStateException("Timed out configuring camera capture session.");
        }
        if (sessionRef[0] == null) {
            throw new IllegalStateException(errorRef[0] != null ? errorRef[0] : "Camera capture session failed.");
        }
        return sessionRef[0];
    }

    private static CaptureRequest.Builder createRecordRequest(CameraDevice device) throws CameraAccessException {
        try {
            return device.createCaptureRequest(CameraDevice.TEMPLATE_RECORD);
        } catch (IllegalArgumentException ex) {
            return device.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
        }
    }

    private static EncoderSelection selectH264Encoder(Size size, int bitrateBps) {
        EncoderSelection best = null;
        try {
            MediaCodecList codecList = new MediaCodecList(MediaCodecList.ALL_CODECS);
            MediaCodecInfo[] infos = codecList.getCodecInfos();
            for (int i = 0; i < infos.length; i++) {
                MediaCodecInfo info = infos[i];
                if (info == null || !info.isEncoder() || !supportsType(info, MIME_H264)) {
                    continue;
                }
                EncoderSelection candidate = inspectH264Encoder(info, size, bitrateBps);
                if (candidate != null && (best == null || candidate.score > best.score)) {
                    best = candidate;
                }
            }
        } catch (Exception ignored) {
            return null;
        }
        return best;
    }

    private static EncoderSelection inspectH264Encoder(MediaCodecInfo info, Size size, int bitrateBps) {
        try {
            MediaCodecInfo.CodecCapabilities capabilities = info.getCapabilitiesForType(MIME_H264);
            MediaCodecInfo.VideoCapabilities videoCapabilities = capabilities.getVideoCapabilities();
            MediaCodecInfo.EncoderCapabilities encoderCapabilities = capabilities.getEncoderCapabilities();
            boolean sizeAndRateSupported = videoCapabilities == null ||
                videoCapabilities.areSizeAndRateSupported(size.getWidth(), size.getHeight(), (double) FRAME_RATE_HZ);
            boolean sizeSupported = videoCapabilities == null ||
                videoCapabilities.isSizeSupported(size.getWidth(), size.getHeight());
            int widthAlignment = videoCapabilities != null ? videoCapabilities.getWidthAlignment() : 1;
            int heightAlignment = videoCapabilities != null ? videoCapabilities.getHeightAlignment() : 1;
            int bitrateLower = 0;
            int bitrateUpper = 0;
            boolean bitrateSupported = true;
            if (videoCapabilities != null) {
                Range<Integer> bitrateRange = videoCapabilities.getBitrateRange();
                bitrateLower = bitrateRange.getLower();
                bitrateUpper = bitrateRange.getUpper();
                bitrateSupported = bitrateBps >= bitrateLower && bitrateBps <= bitrateUpper;
            }
            boolean cbrSupported = encoderCapabilities != null &&
                encoderCapabilities.isBitrateModeSupported(MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR);
            boolean cbrFdSupported = encoderCapabilities != null &&
                encoderCapabilities.isBitrateModeSupported(MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR_FD);
            boolean vbrSupported = encoderCapabilities != null &&
                encoderCapabilities.isBitrateModeSupported(MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_VBR);
            boolean hardwareAccelerated = safeIsHardwareAccelerated(info);
            boolean softwareOnly = safeIsSoftwareOnly(info);
            long score = 0L;
            if (sizeAndRateSupported) {
                score += 1_000_000_000L;
            } else if (sizeSupported) {
                score += 500_000_000L;
            }
            if (bitrateSupported) {
                score += 100_000_000L;
            }
            if (hardwareAccelerated) {
                score += 10_000_000L;
            }
            if (softwareOnly) {
                score -= 10_000_000L;
            }
            if (cbrSupported) {
                score += 1_000_000L;
            }
            return new EncoderSelection(
                info.getName(),
                hardwareAccelerated,
                softwareOnly,
                sizeSupported,
                sizeAndRateSupported,
                bitrateSupported,
                widthAlignment,
                heightAlignment,
                bitrateLower,
                bitrateUpper,
                cbrSupported,
                cbrFdSupported,
                vbrSupported,
                score);
        } catch (Exception ignored) {
            return null;
        }
    }

    private static boolean supportsType(MediaCodecInfo info, String mimeType) {
        String[] types = info.getSupportedTypes();
        for (int i = 0; i < types.length; i++) {
            if (mimeType.equalsIgnoreCase(types[i])) {
                return true;
            }
        }
        return false;
    }

    private static MediaCodec createH264Encoder(
        EncoderSelection encoderSelection,
        EncoderMetadata encoderMetadata) throws Exception {
        if (encoderSelection != null && encoderSelection.codecName.length() > 0) {
            try {
                return MediaCodec.createByCodecName(encoderSelection.codecName);
            } catch (Exception ex) {
                encoderMetadata.encoderSelectionFallbackReason =
                    ex.getClass().getSimpleName() + ": " + safeMessage(ex);
            }
        }
        return MediaCodec.createEncoderByType(MIME_H264);
    }

    private static void applyEncoderSelectionMetadata(
        EncoderSelection encoderSelection,
        EncoderMetadata encoderMetadata,
        MediaCodec encoder) {
        encoderMetadata.encoderName = safeCodecName(encoder);
        if (encoderSelection == null) {
            encoderMetadata.encoderSelectionSource = "create_encoder_by_type";
            encoderMetadata.bitrateModeRequested = "default";
            encoderMetadata.bitrateModeApplied = "default";
            return;
        }
        encoderMetadata.encoderSelectionSource = "mediacodec_list";
        encoderMetadata.encoderSelectedName = encoderSelection.codecName;
        encoderMetadata.encoderHardwareAccelerated = encoderSelection.hardwareAccelerated;
        encoderMetadata.encoderSoftwareOnly = encoderSelection.softwareOnly;
        encoderMetadata.encoderSizeSupported = encoderSelection.sizeSupported;
        encoderMetadata.encoderSizeAndRateSupported = encoderSelection.sizeAndRateSupported;
        encoderMetadata.encoderBitrateSupported = encoderSelection.bitrateSupported;
        encoderMetadata.encoderWidthAlignment = encoderSelection.widthAlignment;
        encoderMetadata.encoderHeightAlignment = encoderSelection.heightAlignment;
        encoderMetadata.encoderBitrateLower = encoderSelection.bitrateLower;
        encoderMetadata.encoderBitrateUpper = encoderSelection.bitrateUpper;
        encoderMetadata.encoderCbrSupported = encoderSelection.cbrSupported;
        encoderMetadata.encoderCbrFdSupported = encoderSelection.cbrFdSupported;
        encoderMetadata.encoderVbrSupported = encoderSelection.vbrSupported;
        encoderMetadata.bitrateModeRequested = encoderSelection.cbrSupported ? "cbr" : "default";
    }

    private static void configureH264Encoder(
        MediaCodec encoder,
        Size size,
        int bitrateBps,
        EncoderMetadata encoderMetadata) throws Exception {
        encoderMetadata.prependHeadersToSyncFramesRequested = true;
        encoderMetadata.optionalLowLatencyHintsRequested = true;
        int bitrateMode = "cbr".equals(encoderMetadata.bitrateModeRequested)
            ? MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR
            : -1;
        try {
            encoder.configure(
                buildH264EncoderFormat(size, bitrateBps, true, bitrateMode),
                null,
                null,
                MediaCodec.CONFIGURE_FLAG_ENCODE);
            encoderMetadata.prependHeadersToSyncFramesApplied = true;
            encoderMetadata.optionalLowLatencyHintsApplied = true;
            encoderMetadata.bitrateModeApplied = bitrateModeName(bitrateMode);
        } catch (Exception firstError) {
            encoderMetadata.prependHeadersToSyncFramesApplied = false;
            encoderMetadata.optionalLowLatencyHintsApplied = false;
            encoderMetadata.configureFallbackReason =
                firstError.getClass().getSimpleName() + ": " + safeMessage(firstError);
            try {
                encoder.reset();
            } catch (Exception ignored) {
            }
            try {
                encoder.configure(
                    buildH264EncoderFormat(size, bitrateBps, false, bitrateMode),
                    null,
                    null,
                    MediaCodec.CONFIGURE_FLAG_ENCODE);
                encoderMetadata.bitrateModeApplied = bitrateModeName(bitrateMode);
            } catch (Exception secondError) {
                if (bitrateMode < 0) {
                    throw secondError;
                }
                encoderMetadata.bitrateModeApplied = "default";
                encoderMetadata.bitrateModeFallbackReason =
                    secondError.getClass().getSimpleName() + ": " + safeMessage(secondError);
                try {
                    encoder.reset();
                } catch (Exception ignored) {
                }
                encoder.configure(
                    buildH264EncoderFormat(size, bitrateBps, false, -1),
                    null,
                    null,
                    MediaCodec.CONFIGURE_FLAG_ENCODE);
            }
        }
    }

    private static MediaFormat buildH264EncoderFormat(
        Size size,
        int bitrateBps,
        boolean includeOptionalHints,
        int bitrateMode) {
        MediaFormat format = MediaFormat.createVideoFormat(MIME_H264, size.getWidth(), size.getHeight());
        format.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
        format.setInteger(MediaFormat.KEY_BIT_RATE, bitrateBps);
        format.setInteger(MediaFormat.KEY_FRAME_RATE, FRAME_RATE_HZ);
        format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1);
        if (bitrateMode >= 0) {
            format.setInteger(MediaFormat.KEY_BITRATE_MODE, bitrateMode);
        }
        if (includeOptionalHints) {
            format.setInteger(MediaFormat.KEY_PREPEND_HEADER_TO_SYNC_FRAMES, 1);
            format.setInteger(MediaFormat.KEY_PRIORITY, 0);
            format.setInteger(MediaFormat.KEY_LATENCY, 1);
            format.setInteger(MediaFormat.KEY_MAX_B_FRAMES, 0);
            format.setInteger(MediaFormat.KEY_OUTPUT_REORDER_DEPTH, 0);
            format.setFloat(MediaFormat.KEY_MAX_FPS_TO_ENCODER, (float) FRAME_RATE_HZ);
        }
        return format;
    }

    private static String bitrateModeName(int mode) {
        if (mode == MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR) {
            return "cbr";
        }
        if (mode == MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR_FD) {
            return "cbr_fd";
        }
        if (mode == MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_VBR) {
            return "vbr";
        }
        if (mode == MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CQ) {
            return "cq";
        }
        return "default";
    }

    private static boolean safeIsHardwareAccelerated(MediaCodecInfo info) {
        try {
            return info.isHardwareAccelerated();
        } catch (Exception ignored) {
            return false;
        }
    }

    private static boolean safeIsSoftwareOnly(MediaCodecInfo info) {
        try {
            return info.isSoftwareOnly();
        } catch (Exception ignored) {
            return false;
        }
    }

    private static void requestSyncFrameOnStart(MediaCodec encoder, EncoderMetadata encoderMetadata) {
        encoderMetadata.syncFrameRequestOnStartRequested = true;
        Bundle params = new Bundle();
        params.putInt(MediaCodec.PARAMETER_KEY_REQUEST_SYNC_FRAME, 0);
        try {
            encoder.setParameters(params);
            encoderMetadata.syncFrameRequestOnStartSucceeded = true;
        } catch (Exception ex) {
            encoderMetadata.syncFrameRequestOnStartSucceeded = false;
            encoderMetadata.syncFrameRequestOnStartError =
                ex.getClass().getSimpleName() + ": " + safeMessage(ex);
        }
    }

    private static void drainEncoder(
        MediaCodec encoder,
        List<EncodedPacket> packets,
        boolean endOfStream,
        int maxVideoPackets,
        EncoderMetadata encoderMetadata) throws Exception {
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        int emptyPolls = 0;
        while (videoPacketCount(packets) < maxVideoPackets) {
            int status = encoder.dequeueOutputBuffer(info, ENCODER_DRAIN_TIMEOUT_US);
            if (status == MediaCodec.INFO_TRY_AGAIN_LATER) {
                if (!endOfStream || emptyPolls++ > 50) {
                    break;
                }
                continue;
            }
            if (status == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                captureEncoderOutputFormat(encoder, encoderMetadata);
                continue;
            }
            if (status < 0) {
                continue;
            }

            ByteBuffer outputBuffer = encoder.getOutputBuffer(status);
            if (outputBuffer != null && info.size > 0) {
                if (info.size > BINARY_STREAM_MAX_PACKET_BYTES) {
                    throw new IllegalStateException("Encoded packet too large: " + info.size);
                }
                byte[] payload = new byte[info.size];
                outputBuffer.position(info.offset);
                outputBuffer.limit(info.offset + info.size);
                outputBuffer.get(payload);
                EncodedPacket packet = new EncodedPacket(
                        info.presentationTimeUs,
                        info.flags,
                        payload,
                        SystemClock.elapsedRealtimeNanos(),
                        System.currentTimeMillis() * 1_000_000L);
                if (!packet.isCodecConfig() || codecConfigPacketCount(packets) < MAX_CODEC_CONFIG_PACKETS) {
                    packets.add(packet);
                }
            }
            boolean reachedEos = (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
            encoder.releaseOutputBuffer(status, false);
            if (reachedEos) {
                break;
            }
        }
    }

    private static void drainEncoderToQueue(
        MediaCodec encoder,
        LivePacketQueue packetQueue,
        boolean endOfStream,
        int maxPackets,
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int captureMs,
        int bitrateBps,
        JSONObject endpoint,
        EncoderMetadata encoderMetadata) throws Exception {
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        int emptyPolls = 0;
        while (maxPackets <= 0 || packetQueue.acceptedPacketCount() < maxPackets) {
            int status = encoder.dequeueOutputBuffer(info, ENCODER_DRAIN_TIMEOUT_US);
            if (status == MediaCodec.INFO_TRY_AGAIN_LATER) {
                if (!endOfStream || emptyPolls++ > 50) {
                    break;
                }
                continue;
            }
            if (status == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                captureEncoderOutputFormat(encoder, encoderMetadata);
                registerManifest(
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    captureMs,
                    maxPackets,
                    bitrateBps,
                    true,
                    endpoint,
                    encoderMetadata);
                continue;
            }
            if (status < 0) {
                continue;
            }

            ByteBuffer outputBuffer = encoder.getOutputBuffer(status);
            if (outputBuffer != null && info.size > 0) {
                if (info.size > BINARY_STREAM_MAX_PACKET_BYTES) {
                    throw new IllegalStateException("Encoded packet too large: " + info.size);
                }
                byte[] payload = new byte[info.size];
                outputBuffer.position(info.offset);
                outputBuffer.limit(info.offset + info.size);
                outputBuffer.get(payload);
                EncodedPacket packet = new EncodedPacket(
                    info.presentationTimeUs,
                    info.flags,
                    payload,
                    SystemClock.elapsedRealtimeNanos(),
                    System.currentTimeMillis() * 1_000_000L);
                if (!packet.isCodecConfig() || packetQueue.codecConfigAcceptedCount() < MAX_CODEC_CONFIG_PACKETS) {
                    packetQueue.offer(packet);
                }
            }
            boolean reachedEos = (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
            encoder.releaseOutputBuffer(status, false);
            if (reachedEos) {
                break;
            }
        }
    }

    private static CameraSelection chooseCamera(
        CameraManager manager,
        String requestedCameraId,
        int preferredWidth,
        int preferredHeight) throws Exception {
        String[] ids = manager.getCameraIdList();
        CameraSelection best = null;
        for (int i = 0; i < ids.length; i++) {
            String id = ids[i];
            if (requestedCameraId != null && requestedCameraId.length() > 0 && !requestedCameraId.equals(id)) {
                continue;
            }
            CameraCharacteristics characteristics = manager.getCameraCharacteristics(id);
            Size size = chooseSize(characteristics, preferredWidth, preferredHeight);
            if (size == null) {
                continue;
            }
            Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
            boolean back = facing != null && facing == CameraCharacteristics.LENS_FACING_BACK;
            float[] translation = characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION);
            double translationX = translation != null && translation.length > 0 ? translation[0] : 0.0;
            long score = scoreCamera(back, translationX, size, preferredWidth, preferredHeight);
            if (best == null || score > best.score) {
                best = new CameraSelection(id, size, score, characteristics);
            }
        }
        if (best == null) {
            throw new IllegalStateException(requestedCameraId != null && requestedCameraId.length() > 0
                ? "Requested camera has no encoder-compatible output: " + requestedCameraId
                : "No app-visible encoder-compatible camera source found.");
        }
        return best;
    }

    private static Size chooseSize(CameraCharacteristics characteristics, int preferredWidth, int preferredHeight) {
        StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
        if (map == null) {
            return null;
        }
        Size[] sizes = map.getOutputSizes(ImageFormat.PRIVATE);
        if (sizes == null || sizes.length == 0) {
            sizes = map.getOutputSizes(Surface.class);
        }
        if (sizes == null || sizes.length == 0) {
            return null;
        }
        Size best = null;
        long bestScore = Long.MAX_VALUE;
        for (int i = 0; i < sizes.length; i++) {
            Size size = sizes[i];
            long area = (long) size.getWidth() * (long) size.getHeight();
            long preferredArea = (long) preferredWidth * (long) preferredHeight;
            long distance = Math.abs((long) size.getWidth() - preferredWidth) +
                Math.abs((long) size.getHeight() - preferredHeight);
            long underPreferred = area < preferredArea ? preferredArea - area : 0L;
            long score = distance * 10000L + underPreferred;
            if (score < bestScore) {
                bestScore = score;
                best = size;
            }
        }
        return best;
    }

    private static long scoreCamera(
        boolean back,
        double translationX,
        Size size,
        int preferredWidth,
        int preferredHeight) {
        long score = back ? 1_000_000_000L : 0L;
        score -= (long) (Math.abs(translationX) * 1_000_000.0);
        score -= Math.abs((long) size.getWidth() - preferredWidth) * 1000L;
        score -= Math.abs((long) size.getHeight() - preferredHeight) * 1000L;
        return score;
    }

    private static JSONObject buildProjectionMetadata(CameraSelection selection) throws Exception {
        CameraCharacteristics characteristics = selection.characteristics;
        JSONObject metadata = new JSONObject();
        metadata.put("schema", "rusty.xr.camera_projection.stream_source_metadata.v1");
        metadata.put("source", "broker_app.camera2_h264_stream");
        metadata.put("sourceLabel", "Broker app Camera2 H.264 source " + selection.cameraId);
        metadata.put("cameraId", selection.cameraId);
        metadata.put("selectionScore", selection.score);
        metadata.put("deliveredWidth", selection.size.getWidth());
        metadata.put("deliveredHeight", selection.size.getHeight());

        Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
        metadata.put("lensFacing", lensFacingLabel(facing));
        metadata.put("lensFacingRank", lensFacingRank(facing));
        Integer sensorOrientation = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION);
        if (sensorOrientation != null) {
            metadata.put("sensorOrientationDegrees", sensorOrientation.intValue());
        }

        Rect activeArray = characteristics.get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE);
        Size sensorPixelArray = characteristics.get(CameraCharacteristics.SENSOR_INFO_PIXEL_ARRAY_SIZE);
        float[] calibration = characteristics.get(CameraCharacteristics.LENS_INTRINSIC_CALIBRATION);
        int intrinsicsWidth = activeArray != null ? activeArray.width() : (sensorPixelArray != null ? sensorPixelArray.getWidth() : 0);
        int intrinsicsHeight = activeArray != null ? activeArray.height() : (sensorPixelArray != null ? sensorPixelArray.getHeight() : 0);
        boolean hasIntrinsics = calibration != null && calibration.length >= 4 && intrinsicsWidth > 0 && intrinsicsHeight > 0;
        if (hasIntrinsics) {
            JSONObject intrinsics = new JSONObject();
            intrinsics.put("fx", calibration[0]);
            intrinsics.put("fy", calibration[1]);
            intrinsics.put("cx", calibration[2]);
            intrinsics.put("cy", calibration[3]);
            intrinsics.put("skew", calibration.length >= 5 ? calibration[4] : 0.0f);
            metadata.put("intrinsics", intrinsics);
            metadata.put(
                "intrinsicsDomain",
                pixelDomain(activeArray != null ? "activeArray" : "sensorPixelArray", intrinsicsWidth, intrinsicsHeight));
        }
        if (activeArray != null) {
            metadata.put("activeArrayDomain", pixelDomain("activeArray", activeArray.width(), activeArray.height()));
        }
        if (sensorPixelArray != null) {
            metadata.put("sensorPixelDomain", pixelDomain("sensorPixelArray", sensorPixelArray.getWidth(), sensorPixelArray.getHeight()));
        }

        float[] translation = characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION);
        float[] rotation = characteristics.get(CameraCharacteristics.LENS_POSE_ROTATION);
        Integer reference = characteristics.get(CameraCharacteristics.LENS_POSE_REFERENCE);
        float[] normalizedRotation = normalizeQuaternionOrNull(rotation);
        boolean hasPose = isFiniteArray(translation, 3) &&
            normalizedRotation != null &&
            isAcceptedLensPoseReference(reference);
        metadata.put("missingIntrinsics", !hasIntrinsics);
        metadata.put("missingPose", !hasPose);
        metadata.put("poseSource", hasPose ? "platform" : "missing");
        metadata.put(
            "poseCoordinateConvention",
            hasPose
                ? "android-camera2-lens-pose-reference-from-camera"
                : "broker-decoded-h264-image-space");
        metadata.put("lensPoseReferenceLabel", lensPoseReferenceLabel(reference));
        if (hasPose) {
            JSONObject extrinsics = new JSONObject();
            extrinsics.put("px", translation[0]);
            extrinsics.put("py", translation[1]);
            extrinsics.put("pz", translation[2]);
            extrinsics.put("qx", normalizedRotation[0]);
            extrinsics.put("qy", normalizedRotation[1]);
            extrinsics.put("qz", normalizedRotation[2]);
            extrinsics.put("qw", normalizedRotation[3]);
            metadata.put("extrinsics", extrinsics);
        }
        metadata.put("projectionMetadataReady", hasIntrinsics && hasPose);
        return metadata;
    }

    private static JSONObject pixelDomain(String kind, int width, int height) throws Exception {
        JSONObject domain = new JSONObject();
        domain.put("kind", kind);
        domain.put("width", width);
        domain.put("height", height);
        return domain;
    }

    private static String lensFacingLabel(Integer facing) {
        if (facing == null) {
            return "unknown";
        }
        int value = facing.intValue();
        if (value == CameraCharacteristics.LENS_FACING_BACK) {
            return "back";
        }
        if (value == CameraCharacteristics.LENS_FACING_FRONT) {
            return "front";
        }
        if (value == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return "external";
        }
        return "unknown";
    }

    private static int lensFacingRank(Integer facing) {
        if (facing == null) {
            return 0;
        }
        int value = facing.intValue();
        if (value == CameraCharacteristics.LENS_FACING_BACK) {
            return 3;
        }
        if (value == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return 2;
        }
        if (value == CameraCharacteristics.LENS_FACING_FRONT) {
            return 1;
        }
        return 0;
    }

    private static boolean isAcceptedLensPoseReference(Integer reference) {
        if (reference == null) {
            return false;
        }
        int value = reference.intValue();
        return value == CameraCharacteristics.LENS_POSE_REFERENCE_PRIMARY_CAMERA
            || value == CameraCharacteristics.LENS_POSE_REFERENCE_GYROSCOPE;
    }

    private static String lensPoseReferenceLabel(Integer reference) {
        if (reference == null) {
            return "missing";
        }
        int value = reference.intValue();
        if (value == CameraCharacteristics.LENS_POSE_REFERENCE_PRIMARY_CAMERA) {
            return "PRIMARY_CAMERA";
        }
        if (value == CameraCharacteristics.LENS_POSE_REFERENCE_GYROSCOPE) {
            return "GYROSCOPE";
        }
        if (value == CameraCharacteristics.LENS_POSE_REFERENCE_UNDEFINED) {
            return "UNDEFINED";
        }
        return "other";
    }

    private static String sensorTimestampSourceLabel(Integer source) {
        if (source == null) {
            return "missing";
        }
        int value = source.intValue();
        if (value == CameraMetadata.SENSOR_INFO_TIMESTAMP_SOURCE_REALTIME) {
            return "REALTIME";
        }
        if (value == CameraMetadata.SENSOR_INFO_TIMESTAMP_SOURCE_UNKNOWN) {
            return "UNKNOWN";
        }
        return "other";
    }

    private static boolean isFiniteArray(float[] values, int expectedLength) {
        if (values == null || values.length < expectedLength) {
            return false;
        }
        for (int i = 0; i < expectedLength; i++) {
            if (Float.isNaN(values[i]) || Float.isInfinite(values[i])) {
                return false;
            }
        }
        return true;
    }

    private static float[] normalizeQuaternionOrNull(float[] quaternion) {
        if (!isFiniteArray(quaternion, 4)) {
            return null;
        }
        double normSquared =
            quaternion[0] * quaternion[0] +
            quaternion[1] * quaternion[1] +
            quaternion[2] * quaternion[2] +
            quaternion[3] * quaternion[3];
        if (Double.isNaN(normSquared) || Double.isInfinite(normSquared) || normSquared <= 0.000001) {
            return null;
        }
        double invNorm = 1.0 / Math.sqrt(normSquared);
        return new float[] {
            (float) (quaternion[0] * invNorm),
            (float) (quaternion[1] * invNorm),
            (float) (quaternion[2] * invNorm),
            (float) (quaternion[3] * invNorm)
        };
    }

    private static StreamWriteStats writePackets(int devicePort, String bindHost, Size size, List<EncodedPacket> packets) throws Exception {
        if (packets.size() == 0) {
            throw new IllegalStateException("No H.264 packets were available to stream.");
        }
        ServerSocket server = new ServerSocket(devicePort, 1, InetAddress.getByName(bindHost));
        long listenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long acceptElapsedNs = 0L;
        long writeStartElapsedNs = 0L;
        long writeEndElapsedNs = 0L;
        try {
            server.setSoTimeout(15000);
            Socket client = server.accept();
            acceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            try {
                client.setTcpNoDelay(true);
                OutputStream output = client.getOutputStream();
                writeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                writeEncodedPacketStream(output, size, packets);
                output.flush();
                writeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            } finally {
                client.close();
            }
        } finally {
            server.close();
        }
        return new StreamWriteStats(listenStartElapsedNs, acceptElapsedNs, writeStartElapsedNs, writeEndElapsedNs);
    }

    private static void writeEncodedPacketStream(OutputStream output, Size size, List<EncodedPacket> packets) throws Exception {
        writeStreamHeader(output, size, packets.size());
        for (int i = 0; i < packets.size(); i++) {
            writeEncodedPacket(output, packets.get(i));
        }
    }

    private static void writeStreamHeader(OutputStream output, Size size, int packetCount) throws Exception {
        output.write(MAGIC.getBytes(StandardCharsets.US_ASCII));
        writeU32(output, SCHEMA_VERSION);
        writeU32(output, CODEC_H264);
        writeU32(output, size.getWidth());
        writeU32(output, size.getHeight());
        writeU32(output, packetCount);
        writeU32(output, 0);
    }

    private static void writeEncodedPacket(OutputStream output, EncodedPacket packet) throws Exception {
        writeU64(output, packet.ptsUs);
        writeU32(output, packet.flags);
        writeU32(output, packet.payload.length);
        writeU64(output, packet.encoderOutputElapsedNs);
        writeU64(output, packet.encoderOutputUnixNs);
        output.write(packet.payload);
    }

    private static void captureEncoderOutputFormat(MediaCodec encoder, EncoderMetadata encoderMetadata) {
        encoderMetadata.outputFormatChangeCount++;
        MediaFormat outputFormat = encoder.getOutputFormat();
        encoderMetadata.outputMime = mediaFormatString(outputFormat, MediaFormat.KEY_MIME, MIME_H264);
        encoderMetadata.outputWidth = mediaFormatInt(outputFormat, MediaFormat.KEY_WIDTH, 0);
        encoderMetadata.outputHeight = mediaFormatInt(outputFormat, MediaFormat.KEY_HEIGHT, 0);
        encoderMetadata.profile = mediaFormatInt(outputFormat, MediaFormat.KEY_PROFILE, -1);
        encoderMetadata.level = mediaFormatInt(outputFormat, MediaFormat.KEY_LEVEL, -1);
        encoderMetadata.colorStandard = mediaFormatInt(outputFormat, MediaFormat.KEY_COLOR_STANDARD, -1);
        encoderMetadata.colorRange = mediaFormatInt(outputFormat, MediaFormat.KEY_COLOR_RANGE, -1);
        encoderMetadata.colorTransfer = mediaFormatInt(outputFormat, MediaFormat.KEY_COLOR_TRANSFER, -1);
        encoderMetadata.appliedLatencyFrames = mediaFormatInt(outputFormat, MediaFormat.KEY_LATENCY, -1);
        encoderMetadata.appliedOutputReorderDepth = mediaFormatInt(outputFormat, MediaFormat.KEY_OUTPUT_REORDER_DEPTH, -1);
        encoderMetadata.outputBitrateMode = bitrateModeName(mediaFormatInt(outputFormat, MediaFormat.KEY_BITRATE_MODE, -1));
        encoderMetadata.csdSps = mediaFormatByteBuffer(outputFormat, "csd-0");
        encoderMetadata.csdPps = mediaFormatByteBuffer(outputFormat, "csd-1");
    }

    private static void registerManifest(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int captureMs,
        int maxPackets,
        int bitrateBps,
        boolean liveStream,
        JSONObject endpoint,
        EncoderMetadata encoderMetadata) throws Exception {
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", "broker_app.camera_h264");
        manifest.put("session_id", sessionId);
        manifest.put("source", "broker_app_camera2_mediacodec_surface");
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", "adb_forwarded_tcp_binary");
        manifest.put("mime_type", "video/avc");
        manifest.put("codec", "h264");
        manifest.put("decoder_target", "surface");
        manifest.put("width", size.getWidth());
        manifest.put("height", size.getHeight());
        manifest.put("frame_rate_hz", FRAME_RATE_HZ);
        manifest.put("bitrate_bps", bitrateBps);
        manifest.put("source_kind", "broker_app_camera2_mediacodec_surface");
        manifest.put("camera_id", cameraId);
        manifest.put("capture_ms", captureMs);
        manifest.put("max_packets", maxPackets);
        manifest.put("live_stream", liveStream);
        manifest.put("stream_mode", streamMode(liveStream, captureMs, maxPackets));
        manifest.put("writer_backpressure_isolated", liveStream);
        manifest.put("writer_queue_depth", liveStream && endpoint != null ? endpoint.optInt("writer_queue_depth", 0) : 0);
        manifest.put("binary_schema_version", SCHEMA_VERSION);
        manifest.put("binary_endpoint", endpoint);
        putEncoderMetadata(manifest, encoderMetadata);
        sink.registerManifest(manifest);
    }

    private static void recordSample(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int index,
        EncodedPacket packet,
        boolean liveStream) throws Exception {
        JSONObject sample = new JSONObject();
        sample.put("schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
        sample.put("stream_id", "broker_app.camera_h264");
        sample.put("session_id", sessionId);
        sample.put("sequence_id", System.currentTimeMillis() * 1000L + index);
        sample.put("source", "broker_app_camera2_mediacodec_surface");
        sample.put("transport", "metadata_only");
        sample.put("payload_transport", "adb_forwarded_tcp_binary");
        sample.put("mime_type", "video/avc");
        sample.put("codec", "h264");
        sample.put("camera_id", cameraId);
        sample.put("encoded_size_bytes", packet.payload.length);
        sample.put("width", size.getWidth());
        sample.put("height", size.getHeight());
        sample.put("key_frame", (packet.flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0);
        sample.put("codec_config", packet.isCodecConfig());
        sample.put("video_frame", !packet.isCodecConfig());
        sample.put("pts_us", packet.ptsUs);
        sample.put("dts_us", packet.ptsUs);
        sample.put("source_time_unix_ns", packet.encoderOutputUnixNs);
        sample.put("source_time_elapsed_ns", packet.encoderOutputElapsedNs);
        sample.put("encoder_output_unix_ns", packet.encoderOutputUnixNs);
        sample.put("encoder_output_elapsed_ns", packet.encoderOutputElapsedNs);
        sample.put("stream_mode", liveStream ? "live_stream" : "bounded_capture_then_write");
        sink.recordSample(sample);
    }

    private static String streamMode(boolean liveStream, int captureMs, int maxPackets) {
        if (!liveStream) {
            return "bounded_capture_then_write";
        }
        if (captureMs <= 0 && maxPackets <= 0) {
            return "live_unbounded";
        }
        if (captureMs <= 0) {
            return "live_until_packet_count";
        }
        if (maxPackets <= 0) {
            return "live_until_capture_timeout";
        }
        return "live_bounded";
    }

    private static void recordMetric(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        List<EncodedPacket> packets,
        long encodeStartElapsedNs,
        long encodeEndElapsedNs,
        StreamWriteStats writeStats,
        int captureMs,
        int maxPackets,
        boolean liveStream,
        EncoderMetadata encoderMetadata,
        String lastError) throws Exception {
        long payloadBytes = 0L;
        for (int i = 0; i < packets.size(); i++) {
            payloadBytes += packets.get(i).payload.length;
        }
        JSONObject metric = new JSONObject();
        metric.put("schema", "rusty.xr.video_lab.metric_sample.v1");
        metric.put("stream_id", "broker_app.camera_h264");
        metric.put("source", "broker_app_camera2_mediacodec_surface");
        metric.put("transport", "metadata_only");
        metric.put("payload_transport", "adb_forwarded_tcp_binary");
        metric.put("codec", "h264");
        metric.put("session_id", sessionId);
        metric.put("camera_id", cameraId != null ? cameraId : "");
        metric.put("sequence_id", System.currentTimeMillis() * 1000L);
        metric.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
        metric.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        metric.put("camera_encode_start_elapsed_ns", encodeStartElapsedNs);
        metric.put("camera_encode_end_elapsed_ns", encodeEndElapsedNs);
        metric.put("camera_encode_duration_ns", Math.max(0L, encodeEndElapsedNs - encodeStartElapsedNs));
        metric.put("live_stream", liveStream);
        metric.put("stream_mode", streamMode(liveStream, captureMs, maxPackets));
        metric.put("binary_listen_start_elapsed_ns", writeStats.listenStartElapsedNs);
        metric.put("binary_accept_elapsed_ns", writeStats.acceptElapsedNs);
        metric.put("binary_write_start_elapsed_ns", writeStats.writeStartElapsedNs);
        metric.put("binary_write_end_elapsed_ns", writeStats.writeEndElapsedNs);
        metric.put("binary_write_duration_ns", Math.max(0L, writeStats.writeEndElapsedNs - writeStats.writeStartElapsedNs));
        metric.put("packet_count", packets.size());
        metric.put("video_packet_count", videoPacketCount(packets));
        metric.put("codec_config_packet_count", codecConfigPacketCount(packets));
        metric.put("payload_size_bytes", payloadBytes);
        metric.put("dropped_frames", writeStats.writerQueueDroppedVideoPackets);
        metric.put("stale_frames", 0);
        metric.put("queue_depth", writeStats.writerQueueFinalDepth);
        metric.put("writer_backpressure_isolated", writeStats.writerBackpressureIsolated);
        metric.put("writer_packet_count", writeStats.writerPacketCount);
        metric.put("writer_queue_capacity", writeStats.writerQueueCapacity);
        metric.put("writer_queue_enqueued_packets", writeStats.writerQueueEnqueuedPackets);
        metric.put("writer_queue_max_depth", writeStats.writerQueueMaxDepth);
        metric.put("writer_queue_final_depth", writeStats.writerQueueFinalDepth);
        metric.put("writer_queue_dropped_packets", writeStats.writerQueueDroppedPackets);
        metric.put("writer_queue_dropped_video_packets", writeStats.writerQueueDroppedVideoPackets);
        metric.put("writer_queue_dropped_non_keyframe_packets", writeStats.writerQueueDroppedNonKeyframePackets);
        metric.put("writer_queue_dropped_keyframe_packets", writeStats.writerQueueDroppedKeyframePackets);
        metric.put("writer_queue_dropped_codec_config_packets", writeStats.writerQueueDroppedCodecConfigPackets);
        metric.put("writer_queue_dropped_incoming_packets", writeStats.writerQueueDroppedIncomingPackets);
        if (writeStats.writerError.length() > 0) {
            metric.put("writer_error", writeStats.writerError);
        }
        metric.put("width", size != null ? size.getWidth() : 0);
        metric.put("height", size != null ? size.getHeight() : 0);
        if (lastError != null && lastError.length() > 0) {
            metric.put("last_error", lastError);
        }
        putEncoderMetadata(metric, encoderMetadata);
        sink.recordMetric(metric);
    }

    private static void putEncoderMetadata(JSONObject target, EncoderMetadata encoderMetadata) throws Exception {
        if (encoderMetadata == null) {
            return;
        }
        target.put("encoder_name", encoderMetadata.encoderName);
        target.put("encoder_selection_source", encoderMetadata.encoderSelectionSource);
        target.put("encoder_selected_name", encoderMetadata.encoderSelectedName);
        target.put("encoder_hardware_accelerated", encoderMetadata.encoderHardwareAccelerated);
        target.put("encoder_software_only", encoderMetadata.encoderSoftwareOnly);
        target.put("encoder_size_supported", encoderMetadata.encoderSizeSupported);
        target.put("encoder_size_and_rate_supported", encoderMetadata.encoderSizeAndRateSupported);
        target.put("encoder_bitrate_supported", encoderMetadata.encoderBitrateSupported);
        target.put("encoder_width_alignment", encoderMetadata.encoderWidthAlignment);
        target.put("encoder_height_alignment", encoderMetadata.encoderHeightAlignment);
        target.put("encoder_bitrate_lower", encoderMetadata.encoderBitrateLower);
        target.put("encoder_bitrate_upper", encoderMetadata.encoderBitrateUpper);
        target.put("encoder_cbr_supported", encoderMetadata.encoderCbrSupported);
        target.put("encoder_cbr_fd_supported", encoderMetadata.encoderCbrFdSupported);
        target.put("encoder_vbr_supported", encoderMetadata.encoderVbrSupported);
        target.put("bitrate_mode_requested", encoderMetadata.bitrateModeRequested);
        target.put("bitrate_mode_applied", encoderMetadata.bitrateModeApplied);
        target.put("bitrate_mode_output_format", encoderMetadata.outputBitrateMode);
        target.put("encoder_output_format_changes", encoderMetadata.outputFormatChangeCount);
        target.put("encoder_output_mime", encoderMetadata.outputMime);
        target.put("encoder_output_width", encoderMetadata.outputWidth);
        target.put("encoder_output_height", encoderMetadata.outputHeight);
        target.put("encoder_profile", encoderMetadata.profile);
        target.put("encoder_level", encoderMetadata.level);
        target.put("encoder_color_standard", encoderMetadata.colorStandard);
        target.put("encoder_color_range", encoderMetadata.colorRange);
        target.put("encoder_color_transfer", encoderMetadata.colorTransfer);
        target.put("encoder_latency_requested_frames", 1);
        target.put("encoder_latency_applied_frames", encoderMetadata.appliedLatencyFrames);
        target.put("encoder_output_reorder_depth_requested", 0);
        target.put("encoder_output_reorder_depth_applied", encoderMetadata.appliedOutputReorderDepth);
        target.put("prepend_headers_to_sync_frames_requested", encoderMetadata.prependHeadersToSyncFramesRequested);
        target.put("prepend_headers_to_sync_frames_applied", encoderMetadata.prependHeadersToSyncFramesApplied);
        target.put("encoder_optional_low_latency_hints_requested", encoderMetadata.optionalLowLatencyHintsRequested);
        target.put("encoder_optional_low_latency_hints_applied", encoderMetadata.optionalLowLatencyHintsApplied);
        target.put("sync_frame_request_on_start_requested", encoderMetadata.syncFrameRequestOnStartRequested);
        target.put("sync_frame_request_on_start_succeeded", encoderMetadata.syncFrameRequestOnStartSucceeded);
        target.put("csd_sps_bytes", byteLength(encoderMetadata.csdSps));
        target.put("csd_pps_bytes", byteLength(encoderMetadata.csdPps));
        target.put("csd_sps_base64", base64NoWrap(encoderMetadata.csdSps));
        target.put("csd_pps_base64", base64NoWrap(encoderMetadata.csdPps));
        target.put("sensor_timestamp_source", encoderMetadata.sensorTimestampSource);
        target.put("camera_capture_started_count", encoderMetadata.cameraCaptureStartedCount);
        target.put("camera_first_capture_started_ns", encoderMetadata.cameraFirstCaptureStartedNs);
        target.put("camera_last_capture_started_ns", encoderMetadata.cameraLastCaptureStartedNs);
        target.put("camera_first_frame_number", encoderMetadata.cameraFirstFrameNumber);
        target.put("camera_last_frame_number", encoderMetadata.cameraLastFrameNumber);
        target.put("camera_first_capture_callback_elapsed_ns", encoderMetadata.cameraFirstCaptureCallbackElapsedNs);
        target.put("camera_last_capture_callback_elapsed_ns", encoderMetadata.cameraLastCaptureCallbackElapsedNs);
        if (encoderMetadata.configureFallbackReason.length() > 0) {
            target.put("encoder_configure_fallback_reason", encoderMetadata.configureFallbackReason);
        }
        if (encoderMetadata.encoderSelectionFallbackReason.length() > 0) {
            target.put("encoder_selection_fallback_reason", encoderMetadata.encoderSelectionFallbackReason);
        }
        if (encoderMetadata.bitrateModeFallbackReason.length() > 0) {
            target.put("bitrate_mode_fallback_reason", encoderMetadata.bitrateModeFallbackReason);
        }
        if (encoderMetadata.syncFrameRequestOnStartError.length() > 0) {
            target.put("sync_frame_request_on_start_error", encoderMetadata.syncFrameRequestOnStartError);
        }
    }

    private static void writeU32(OutputStream output, int value) throws Exception {
        output.write((value >>> 24) & 0xff);
        output.write((value >>> 16) & 0xff);
        output.write((value >>> 8) & 0xff);
        output.write(value & 0xff);
    }

    private static void writeU64(OutputStream output, long value) throws Exception {
        output.write((int) ((value >>> 56) & 0xff));
        output.write((int) ((value >>> 48) & 0xff));
        output.write((int) ((value >>> 40) & 0xff));
        output.write((int) ((value >>> 32) & 0xff));
        output.write((int) ((value >>> 24) & 0xff));
        output.write((int) ((value >>> 16) & 0xff));
        output.write((int) ((value >>> 8) & 0xff));
        output.write((int) (value & 0xff));
    }

    private static int videoPacketCount(List<EncodedPacket> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if (!packets.get(i).isCodecConfig()) {
                count++;
            }
        }
        return count;
    }

    private static int codecConfigPacketCount(List<EncodedPacket> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if (packets.get(i).isCodecConfig()) {
                count++;
            }
        }
        return count;
    }

    private static int mediaFormatInt(MediaFormat format, String key, int fallback) {
        try {
            return format.containsKey(key) ? format.getInteger(key) : fallback;
        } catch (Exception ignored) {
            return fallback;
        }
    }

    private static String mediaFormatString(MediaFormat format, String key, String fallback) {
        try {
            String value = format.containsKey(key) ? format.getString(key) : fallback;
            return value != null ? value : fallback;
        } catch (Exception ignored) {
            return fallback;
        }
    }

    private static byte[] mediaFormatByteBuffer(MediaFormat format, String key) {
        try {
            if (!format.containsKey(key)) {
                return null;
            }
            ByteBuffer buffer = format.getByteBuffer(key);
            if (buffer == null) {
                return null;
            }
            ByteBuffer duplicate = buffer.duplicate();
            byte[] bytes = new byte[duplicate.remaining()];
            duplicate.get(bytes);
            return bytes.length > 0 ? bytes : null;
        } catch (Exception ignored) {
            return null;
        }
    }

    private static int byteLength(byte[] bytes) {
        return bytes != null ? bytes.length : 0;
    }

    private static String base64NoWrap(byte[] bytes) {
        return bytes != null && bytes.length > 0
            ? Base64.encodeToString(bytes, Base64.NO_WRAP)
            : "";
    }

    private static String safeCodecName(MediaCodec codec) {
        try {
            String name = codec.getName();
            return name != null ? name : "";
        } catch (Exception ignored) {
            return "";
        }
    }

    private static String normalizeBindHost(String requestedHost, boolean lanStreamEnabled) {
        String host = requestedHost != null ? requestedHost.trim() : "";
        if (host.length() == 0) {
            return lanStreamEnabled ? "0.0.0.0" : "127.0.0.1";
        }

        if (!lanStreamEnabled && !isLoopbackBindHost(host)) {
            throw new IllegalArgumentException("Non-loopback H.264 stream bind_host requires lan_stream_enabled=true.");
        }
        return host;
    }

    private static String normalizeAdvertisedHost(String requestedHost, String bindHost) {
        String host = requestedHost != null ? requestedHost.trim() : "";
        return host.length() > 0 ? host : bindHost;
    }

    private static boolean isLoopbackBindHost(String host) {
        if (host == null) {
            return false;
        }
        String normalized = host.trim().toLowerCase();
        return "127.0.0.1".equals(normalized) ||
            "localhost".equals(normalized) ||
            "::1".equals(normalized);
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static void joinWriterThread(Thread writerThread, Socket client) {
        if (writerThread == null) {
            return;
        }
        joinQuietly(writerThread, WRITER_JOIN_TIMEOUT_MS);
        if (writerThread.isAlive()) {
            closeQuietly(client);
            joinQuietly(writerThread, WRITER_JOIN_TIMEOUT_MS);
        }
    }

    private static void joinQuietly(Thread thread, long timeoutMs) {
        if (thread == null) {
            return;
        }
        try {
            thread.join(timeoutMs);
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
        }
    }

    private static void closeQuietly(CameraCaptureSession session) {
        if (session != null) {
            try {
                session.close();
            } catch (Exception ignored) {
            }
        }
    }

    private static void closeQuietly(CameraDevice device) {
        if (device != null) {
            try {
                device.close();
            } catch (Exception ignored) {
            }
        }
    }

    private static void closeQuietly(Socket socket) {
        if (socket != null) {
            try {
                socket.close();
            } catch (Exception ignored) {
            }
        }
    }

    private static String safeMessage(Exception ex) {
        String message = ex.getMessage();
        return message != null ? message : "";
    }

    private static String normalizeSessionId(String requested, String prefix) {
        if (requested != null && requested.trim().length() > 0) {
            return requested.trim();
        }

        return prefix + System.currentTimeMillis();
    }

    private static final class CameraSelection {
        final String cameraId;
        final Size size;
        final long score;
        final CameraCharacteristics characteristics;

        CameraSelection(String cameraId, Size size, long score, CameraCharacteristics characteristics) {
            this.cameraId = cameraId;
            this.size = size;
            this.score = score;
            this.characteristics = characteristics;
        }
    }

    private static final class EncoderSelection {
        final String codecName;
        final boolean hardwareAccelerated;
        final boolean softwareOnly;
        final boolean sizeSupported;
        final boolean sizeAndRateSupported;
        final boolean bitrateSupported;
        final int widthAlignment;
        final int heightAlignment;
        final int bitrateLower;
        final int bitrateUpper;
        final boolean cbrSupported;
        final boolean cbrFdSupported;
        final boolean vbrSupported;
        final long score;

        EncoderSelection(
            String codecName,
            boolean hardwareAccelerated,
            boolean softwareOnly,
            boolean sizeSupported,
            boolean sizeAndRateSupported,
            boolean bitrateSupported,
            int widthAlignment,
            int heightAlignment,
            int bitrateLower,
            int bitrateUpper,
            boolean cbrSupported,
            boolean cbrFdSupported,
            boolean vbrSupported,
            long score) {
            this.codecName = codecName != null ? codecName : "";
            this.hardwareAccelerated = hardwareAccelerated;
            this.softwareOnly = softwareOnly;
            this.sizeSupported = sizeSupported;
            this.sizeAndRateSupported = sizeAndRateSupported;
            this.bitrateSupported = bitrateSupported;
            this.widthAlignment = widthAlignment;
            this.heightAlignment = heightAlignment;
            this.bitrateLower = bitrateLower;
            this.bitrateUpper = bitrateUpper;
            this.cbrSupported = cbrSupported;
            this.cbrFdSupported = cbrFdSupported;
            this.vbrSupported = vbrSupported;
            this.score = score;
        }
    }

    static final class EncoderMetadata {
        String encoderName = "";
        String encoderSelectionSource = "";
        String encoderSelectedName = "";
        boolean encoderHardwareAccelerated;
        boolean encoderSoftwareOnly;
        boolean encoderSizeSupported;
        boolean encoderSizeAndRateSupported;
        boolean encoderBitrateSupported;
        int encoderWidthAlignment;
        int encoderHeightAlignment;
        int encoderBitrateLower;
        int encoderBitrateUpper;
        boolean encoderCbrSupported;
        boolean encoderCbrFdSupported;
        boolean encoderVbrSupported;
        String encoderSelectionFallbackReason = "";
        String bitrateModeRequested = "default";
        String bitrateModeApplied = "default";
        String outputBitrateMode = "default";
        String bitrateModeFallbackReason = "";
        String outputMime = "";
        String sensorTimestampSource = "";
        int outputWidth;
        int outputHeight;
        int outputFormatChangeCount;
        int profile = -1;
        int level = -1;
        int colorStandard = -1;
        int colorRange = -1;
        int colorTransfer = -1;
        int appliedLatencyFrames = -1;
        int appliedOutputReorderDepth = -1;
        byte[] csdSps;
        byte[] csdPps;
        boolean prependHeadersToSyncFramesRequested;
        boolean prependHeadersToSyncFramesApplied;
        boolean optionalLowLatencyHintsRequested;
        boolean optionalLowLatencyHintsApplied;
        String configureFallbackReason = "";
        boolean syncFrameRequestOnStartRequested;
        boolean syncFrameRequestOnStartSucceeded;
        String syncFrameRequestOnStartError = "";
        int cameraCaptureStartedCount;
        long cameraFirstCaptureStartedNs;
        long cameraLastCaptureStartedNs;
        long cameraFirstFrameNumber = -1L;
        long cameraLastFrameNumber = -1L;
        long cameraFirstCaptureCallbackElapsedNs;
        long cameraLastCaptureCallbackElapsedNs;

        void copyCaptureTiming(CaptureTimingTracker tracker) {
            if (tracker == null) {
                return;
            }
            cameraCaptureStartedCount = tracker.startedCount;
            cameraFirstCaptureStartedNs = tracker.firstCaptureStartedNs;
            cameraLastCaptureStartedNs = tracker.lastCaptureStartedNs;
            cameraFirstFrameNumber = tracker.firstFrameNumber;
            cameraLastFrameNumber = tracker.lastFrameNumber;
            cameraFirstCaptureCallbackElapsedNs = tracker.firstCallbackElapsedNs;
            cameraLastCaptureCallbackElapsedNs = tracker.lastCallbackElapsedNs;
        }
    }

    private static final class CaptureTimingTracker extends CameraCaptureSession.CaptureCallback {
        int startedCount;
        long firstCaptureStartedNs;
        long lastCaptureStartedNs;
        long firstFrameNumber = -1L;
        long lastFrameNumber = -1L;
        long firstCallbackElapsedNs;
        long lastCallbackElapsedNs;

        @Override
        public void onCaptureStarted(
            CameraCaptureSession session,
            CaptureRequest request,
            long timestamp,
            long frameNumber) {
            long callbackElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (startedCount == 0) {
                firstCaptureStartedNs = timestamp;
                firstFrameNumber = frameNumber;
                firstCallbackElapsedNs = callbackElapsedNs;
            }
            startedCount++;
            lastCaptureStartedNs = timestamp;
            lastFrameNumber = frameNumber;
            lastCallbackElapsedNs = callbackElapsedNs;
        }
    }

    static final class CaptureResult {
        final String sessionId;
        final String requestedCameraId;
        final String cameraId;
        final Size size;
        final int captureMs;
        final int maxPackets;
        final int bitrateBps;
        final long encodeStartElapsedNs;
        final long encodeEndElapsedNs;
        final List<EncodedPacket> packets;
        final EncoderMetadata encoderMetadata;

        CaptureResult(
            String sessionId,
            String requestedCameraId,
            String cameraId,
            Size size,
            int captureMs,
            int maxPackets,
            int bitrateBps,
            long encodeStartElapsedNs,
            long encodeEndElapsedNs,
            List<EncodedPacket> packets,
            EncoderMetadata encoderMetadata) {
            this.sessionId = sessionId;
            this.requestedCameraId = requestedCameraId;
            this.cameraId = cameraId;
            this.size = size;
            this.captureMs = captureMs;
            this.maxPackets = maxPackets;
            this.bitrateBps = bitrateBps;
            this.encodeStartElapsedNs = encodeStartElapsedNs;
            this.encodeEndElapsedNs = encodeEndElapsedNs;
            this.packets = packets;
            this.encoderMetadata = encoderMetadata;
        }
    }

    static final class EncodedPacket {
        final long ptsUs;
        final int flags;
        final byte[] payload;
        final long encoderOutputElapsedNs;
        final long encoderOutputUnixNs;

        EncodedPacket(
            long ptsUs,
            int flags,
            byte[] payload,
            long encoderOutputElapsedNs,
            long encoderOutputUnixNs) {
            this.ptsUs = ptsUs;
            this.flags = flags;
            this.payload = payload;
            this.encoderOutputElapsedNs = encoderOutputElapsedNs;
            this.encoderOutputUnixNs = encoderOutputUnixNs;
        }

        boolean isCodecConfig() {
            return (flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0;
        }

        boolean isKeyFrame() {
            return (flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0;
        }
    }

    private static final class LivePacketQueue {
        private final int capacity;
        private final ArrayDeque<EncodedPacket> queue = new ArrayDeque<EncodedPacket>();
        private boolean closed;
        private int acceptedPacketCount;
        private int codecConfigAcceptedCount;
        private int enqueuedPacketCount;
        private int maxDepth;
        private int droppedPacketCount;
        private int droppedVideoPacketCount;
        private int droppedNonKeyframePacketCount;
        private int droppedKeyframePacketCount;
        private int droppedCodecConfigPacketCount;
        private int droppedIncomingPacketCount;

        LivePacketQueue(int capacity) {
            this.capacity = Math.max(1, capacity);
        }

        synchronized boolean offer(EncodedPacket packet) {
            if (closed || packet == null) {
                return false;
            }
            if (queue.size() >= capacity) {
                EncodedPacket dropped = removeFirstDropCandidate(false);
                if (dropped == null && (packet.isCodecConfig() || packet.isKeyFrame())) {
                    dropped = removeFirstDropCandidate(true);
                }
                if (dropped == null) {
                    droppedIncomingPacketCount++;
                    trackDropped(packet);
                    return false;
                }
                trackDropped(dropped);
            }
            queue.addLast(packet);
            acceptedPacketCount++;
            enqueuedPacketCount++;
            if (packet.isCodecConfig()) {
                codecConfigAcceptedCount++;
            }
            if (queue.size() > maxDepth) {
                maxDepth = queue.size();
            }
            notifyAll();
            return true;
        }

        synchronized EncodedPacket poll(long timeoutMs) throws InterruptedException {
            if (queue.isEmpty() && !closed) {
                wait(timeoutMs);
            }
            if (queue.isEmpty()) {
                return null;
            }
            return queue.removeFirst();
        }

        synchronized void close() {
            closed = true;
            notifyAll();
        }

        synchronized boolean isClosed() {
            return closed;
        }

        synchronized int acceptedPacketCount() {
            return acceptedPacketCount;
        }

        synchronized int codecConfigAcceptedCount() {
            return codecConfigAcceptedCount;
        }

        synchronized int finalDepth() {
            return queue.size();
        }

        synchronized int capacity() {
            return capacity;
        }

        synchronized int enqueuedPacketCount() {
            return enqueuedPacketCount;
        }

        synchronized int maxDepth() {
            return maxDepth;
        }

        synchronized int droppedPacketCount() {
            return droppedPacketCount;
        }

        synchronized int droppedVideoPacketCount() {
            return droppedVideoPacketCount;
        }

        synchronized int droppedNonKeyframePacketCount() {
            return droppedNonKeyframePacketCount;
        }

        synchronized int droppedKeyframePacketCount() {
            return droppedKeyframePacketCount;
        }

        synchronized int droppedCodecConfigPacketCount() {
            return droppedCodecConfigPacketCount;
        }

        synchronized int droppedIncomingPacketCount() {
            return droppedIncomingPacketCount;
        }

        private EncodedPacket removeFirstDropCandidate(boolean allowKeyFrame) {
            int count = queue.size();
            for (int i = 0; i < count; i++) {
                EncodedPacket candidate = queue.removeFirst();
                boolean canDrop = !candidate.isCodecConfig() && (allowKeyFrame || !candidate.isKeyFrame());
                if (canDrop) {
                    return candidate;
                }
                queue.addLast(candidate);
            }
            return null;
        }

        private void trackDropped(EncodedPacket packet) {
            droppedPacketCount++;
            if (packet.isCodecConfig()) {
                droppedCodecConfigPacketCount++;
            } else {
                droppedVideoPacketCount++;
            }
            if (packet.isKeyFrame()) {
                droppedKeyframePacketCount++;
            } else if (!packet.isCodecConfig()) {
                droppedNonKeyframePacketCount++;
            }
        }
    }

    private static final class LiveStreamWriter implements Runnable {
        private final OutputStream output;
        private final Size size;
        private final int maxPackets;
        private final LivePacketQueue queue;
        private final List<EncodedPacket> writtenPackets;
        private final Sink sink;
        private final String sessionId;
        private final String cameraId;
        private final boolean liveStream;
        private volatile int writtenPacketCount;
        private volatile long writeStartElapsedNs;
        private volatile long writeEndElapsedNs;
        private volatile Exception error;

        LiveStreamWriter(
            OutputStream output,
            Size size,
            int maxPackets,
            LivePacketQueue queue,
            List<EncodedPacket> writtenPackets,
            Sink sink,
            String sessionId,
            String cameraId,
            boolean liveStream) {
            this.output = output;
            this.size = size;
            this.maxPackets = maxPackets;
            this.queue = queue;
            this.writtenPackets = writtenPackets;
            this.sink = sink;
            this.sessionId = sessionId;
            this.cameraId = cameraId;
            this.liveStream = liveStream;
        }

        @Override
        public void run() {
            try {
                writeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                writeStreamHeader(output, size, maxPackets);
                output.flush();
                while (maxPackets <= 0 || writtenPacketCount < maxPackets) {
                    EncodedPacket packet = queue.poll(WRITER_QUEUE_POLL_MS);
                    if (packet == null) {
                        if (queue.isClosed()) {
                            break;
                        }
                        continue;
                    }
                    writeEncodedPacket(output, packet);
                    output.flush();
                    int index = writtenPacketCount;
                    synchronized (writtenPackets) {
                        writtenPackets.add(packet);
                    }
                    recordSample(sink, sessionId, cameraId, size, index, packet, liveStream);
                    writtenPacketCount++;
                }
            } catch (Exception ex) {
                error = ex;
                queue.close();
            } finally {
                writeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            }
        }

        boolean hasError() {
            return error != null;
        }

        Exception error() {
            return error;
        }

        int writtenPacketCount() {
            return writtenPacketCount;
        }

        long writeStartElapsedNs() {
            return writeStartElapsedNs;
        }

        long writeEndElapsedNs() {
            return writeEndElapsedNs;
        }

        String errorMessage() {
            Exception ex = error;
            return ex != null ? ex.getClass().getSimpleName() + ": " + safeMessage(ex) : "";
        }
    }

    private static final class LiveStreamResult {
        final List<EncodedPacket> packets;
        final StreamWriteStats writeStats;
        final long encodeEndElapsedNs;

        LiveStreamResult(List<EncodedPacket> packets, StreamWriteStats writeStats, long encodeEndElapsedNs) {
            this.packets = packets;
            this.writeStats = writeStats;
            this.encodeEndElapsedNs = encodeEndElapsedNs;
        }
    }

    private static final class StreamWriteStats {
        final long listenStartElapsedNs;
        final long acceptElapsedNs;
        final long writeStartElapsedNs;
        final long writeEndElapsedNs;
        final boolean writerBackpressureIsolated;
        final int writerPacketCount;
        final int writerQueueCapacity;
        final int writerQueueEnqueuedPackets;
        final int writerQueueMaxDepth;
        final int writerQueueFinalDepth;
        final int writerQueueDroppedPackets;
        final int writerQueueDroppedVideoPackets;
        final int writerQueueDroppedNonKeyframePackets;
        final int writerQueueDroppedKeyframePackets;
        final int writerQueueDroppedCodecConfigPackets;
        final int writerQueueDroppedIncomingPackets;
        final String writerError;

        StreamWriteStats(long listenStartElapsedNs, long acceptElapsedNs, long writeStartElapsedNs, long writeEndElapsedNs) {
            this.listenStartElapsedNs = listenStartElapsedNs;
            this.acceptElapsedNs = acceptElapsedNs;
            this.writeStartElapsedNs = writeStartElapsedNs;
            this.writeEndElapsedNs = writeEndElapsedNs;
            this.writerBackpressureIsolated = false;
            this.writerPacketCount = 0;
            this.writerQueueCapacity = 0;
            this.writerQueueEnqueuedPackets = 0;
            this.writerQueueMaxDepth = 0;
            this.writerQueueFinalDepth = 0;
            this.writerQueueDroppedPackets = 0;
            this.writerQueueDroppedVideoPackets = 0;
            this.writerQueueDroppedNonKeyframePackets = 0;
            this.writerQueueDroppedKeyframePackets = 0;
            this.writerQueueDroppedCodecConfigPackets = 0;
            this.writerQueueDroppedIncomingPackets = 0;
            this.writerError = "";
        }

        StreamWriteStats(
            long listenStartElapsedNs,
            long acceptElapsedNs,
            long writeStartElapsedNs,
            long writeEndElapsedNs,
            LivePacketQueue queue,
            LiveStreamWriter writer) {
            this.listenStartElapsedNs = listenStartElapsedNs;
            this.acceptElapsedNs = acceptElapsedNs;
            this.writeStartElapsedNs = writeStartElapsedNs;
            this.writeEndElapsedNs = writeEndElapsedNs;
            this.writerBackpressureIsolated = true;
            this.writerPacketCount = writer != null ? writer.writtenPacketCount() : 0;
            this.writerQueueCapacity = queue != null ? queue.capacity() : 0;
            this.writerQueueEnqueuedPackets = queue != null ? queue.enqueuedPacketCount() : 0;
            this.writerQueueMaxDepth = queue != null ? queue.maxDepth() : 0;
            this.writerQueueFinalDepth = queue != null ? queue.finalDepth() : 0;
            this.writerQueueDroppedPackets = queue != null ? queue.droppedPacketCount() : 0;
            this.writerQueueDroppedVideoPackets = queue != null ? queue.droppedVideoPacketCount() : 0;
            this.writerQueueDroppedNonKeyframePackets = queue != null ? queue.droppedNonKeyframePacketCount() : 0;
            this.writerQueueDroppedKeyframePackets = queue != null ? queue.droppedKeyframePacketCount() : 0;
            this.writerQueueDroppedCodecConfigPackets = queue != null ? queue.droppedCodecConfigPacketCount() : 0;
            this.writerQueueDroppedIncomingPackets = queue != null ? queue.droppedIncomingPacketCount() : 0;
            this.writerError = writer != null ? writer.errorMessage() : "";
        }
    }
}
