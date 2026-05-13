package com.example.rustyxr.broker;

import android.os.SystemClock;
import android.util.Log;

import org.json.JSONObject;

import java.io.BufferedInputStream;
import java.io.Closeable;
import java.io.EOFException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

final class BrokerH264TcpProxySession {
    private static final String TAG = "RustyXrBroker";
    private static final String STREAM_SCHEMA = "rusty.xr.video_lab.binary_stream.v1";
    private static final String MAGIC = "RXYRVID1";
    private static final int CODEC_H264 = 1;
    private static final int DEFAULT_REMOTE_PORT = 8879;
    private static final int DEFAULT_LOCAL_PORT = 8879;
    private static final int DEFAULT_HOST_PORT = 18879;
    private static final int DEFAULT_CONNECT_TIMEOUT_MS = 15000;
    private static final int DEFAULT_ACCEPT_TIMEOUT_MS = 30000;
    private static final int MAX_TIMEOUT_MS = 120000;
    private static final int MAX_PACKET_COUNT = 2400;
    private static final int MAX_PACKET_BYTES = 1024 * 1024;
    private static final int MAX_STREAM_HEADER_METADATA_BYTES = 256 * 1024;
    private static final int FLAG_KEY_FRAME = 1;
    private static final int FLAG_CODEC_CONFIG = 2;

    interface Sink {
        void registerManifest(JSONObject manifest) throws Exception;

        void recordSample(JSONObject sample) throws Exception;

        void recordMetric(JSONObject metric) throws Exception;
    }

    private BrokerH264TcpProxySession() {
    }

    static JSONObject start(JSONObject params, Sink sink) throws Exception {
        final String sessionId = "broker-h264-tcp-proxy-" + System.currentTimeMillis();
        final String remoteHost = params != null ? params.optString("remote_host", "").trim() : "";
        if (remoteHost.length() == 0) {
            throw new IllegalArgumentException("media.start_h264_tcp_proxy requires params.remote_host.");
        }

        final int remotePort = clamp(params.optInt("remote_port", DEFAULT_REMOTE_PORT), 1, 65535);
        final int localPort = clamp(
            params.optInt("local_port", params.optInt("device_port", DEFAULT_LOCAL_PORT)),
            1,
            65535);
        final int localHostPort = clamp(params.optInt("host_port", DEFAULT_HOST_PORT), 1, 65535);
        final boolean localLanEnabled = params.optBoolean("local_lan_enabled", false);
        final String localBindHost = normalizeLocalBindHost(params.optString("local_bind_host", ""), localLanEnabled);
        final int connectTimeoutMs = clamp(
            params.optInt("connect_timeout_ms", DEFAULT_CONNECT_TIMEOUT_MS),
            100,
            MAX_TIMEOUT_MS);
        final int acceptTimeoutMs = clamp(
            params.optInt("accept_timeout_ms", DEFAULT_ACCEPT_TIMEOUT_MS),
            100,
            MAX_TIMEOUT_MS);

        JSONObject remoteEndpoint = new JSONObject();
        remoteEndpoint.put("host", remoteHost);
        remoteEndpoint.put("port", remotePort);
        remoteEndpoint.put("framing", STREAM_SCHEMA);
        remoteEndpoint.put("magic", MAGIC);
        remoteEndpoint.put("codec_id", CODEC_H264);
        remoteEndpoint.put("codec", "h264");
        remoteEndpoint.put("header_metadata", "projection_metadata_json_utf8");

        JSONObject localEndpoint = new JSONObject();
        localEndpoint.put("host", localBindHost);
        localEndpoint.put("bind_host", localBindHost);
        localEndpoint.put("device_port", localPort);
        localEndpoint.put("host_port", localHostPort);
        localEndpoint.put("framing", STREAM_SCHEMA);
        localEndpoint.put("magic", MAGIC);
        localEndpoint.put("codec_id", CODEC_H264);
        localEndpoint.put("codec", "h264");
        localEndpoint.put("packet_header", "pts_us,flags,size,source_time_elapsed_ns,source_time_unix_ns");
        localEndpoint.put("header_metadata", "projection_metadata_json_utf8");

        JSONObject start = new JSONObject();
        start.put("schema", "rusty.xr.broker.h264_tcp_proxy_start.v1");
        start.put("session_id", sessionId);
        start.put("stream_id", "broker_peer.h264_tcp_proxy");
        start.put("source", "broker_peer_h264_tcp_proxy");
        start.put("state", "starting");
        start.put("remote_endpoint", remoteEndpoint);
        start.put("local_endpoint", localEndpoint);
        start.put("connect_timeout_ms", connectTimeoutMs);
        start.put("accept_timeout_ms", acceptTimeoutMs);
        Log.i(TAG, "H264 TCP proxy start requested session=" + sessionId +
            " local=" + localBindHost + ":" + localPort +
            " remote=" + remoteHost + ":" + remotePort +
            " connectTimeoutMs=" + connectTimeoutMs +
            " acceptTimeoutMs=" + acceptTimeoutMs);

        Thread thread = new Thread(new Runnable() {
            @Override
            public void run() {
                runProxy(
                    sink,
                    sessionId,
                    remoteHost,
                    remotePort,
                    localBindHost,
                    localPort,
                    remoteEndpoint,
                    localEndpoint,
                    connectTimeoutMs,
                    acceptTimeoutMs);
            }
        }, "RustyXrH264TcpProxy");
        thread.start();
        return start;
    }

    static JSONObject runProbe(JSONObject params, Sink sink) throws Exception {
        final String sessionId = "broker-h264-tcp-proxy-probe-" + System.currentTimeMillis();
        final String host = "127.0.0.1";
        final int remotePort = params != null && params.has("remote_port")
            ? clamp(params.optInt("remote_port", 0), 1, 65535)
            : allocateLoopbackPort();
        final int localPort = params != null && params.has("local_port")
            ? clamp(params.optInt("local_port", 0), 1, 65535)
            : allocateLoopbackPortExcluding(remotePort);
        final int width = clamp(params != null ? params.optInt("width", 64) : 64, 16, 4096);
        final int height = clamp(params != null ? params.optInt("height", 64) : 64, 16, 4096);
        final int packetCount = clamp(params != null ? params.optInt("packet_count", 4) : 4, 1, 32);
        final int packetBytes = clamp(params != null ? params.optInt("packet_bytes", 96) : 96, 1, 4096);
        final int timeoutMs = clamp(params != null ? params.optInt("timeout_ms", 10000) : 10000, 500, MAX_TIMEOUT_MS);

        final JSONObject remoteEndpoint = new JSONObject();
        remoteEndpoint.put("host", host);
        remoteEndpoint.put("port", remotePort);
        remoteEndpoint.put("framing", STREAM_SCHEMA);
        remoteEndpoint.put("magic", MAGIC);
        remoteEndpoint.put("codec_id", CODEC_H264);
        remoteEndpoint.put("codec", "h264");

        final JSONObject localEndpoint = new JSONObject();
        localEndpoint.put("host", host);
        localEndpoint.put("bind_host", host);
        localEndpoint.put("device_port", localPort);
        localEndpoint.put("host_port", localPort);
        localEndpoint.put("framing", STREAM_SCHEMA);
        localEndpoint.put("magic", MAGIC);
        localEndpoint.put("codec_id", CODEC_H264);
        localEndpoint.put("codec", "h264");
        localEndpoint.put("packet_header", "pts_us,flags,size,source_time_elapsed_ns,source_time_unix_ns");

        final ProbeSink probeSink = new ProbeSink(sink);
        final ProbeSourceResult sourceResult = new ProbeSourceResult();
        final ProbeConsumerResult consumerResult = new ProbeConsumerResult();
        final CountDownLatch sourceReady = new CountDownLatch(1);
        final Exception[] sourceError = new Exception[1];
        final Exception[] consumerError = new Exception[1];

        Thread sourceThread = new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    runSyntheticSource(
                        host,
                        remotePort,
                        width,
                        height,
                        packetCount,
                        packetBytes,
                        timeoutMs,
                        sourceReady,
                        sourceResult);
                } catch (Exception ex) {
                    sourceError[0] = ex;
                    sourceReady.countDown();
                }
            }
        }, "RustyXrH264ProxyProbeSource");
        sourceThread.start();

        if (!sourceReady.await(timeoutMs, TimeUnit.MILLISECONDS)) {
            throw new IllegalStateException("Synthetic H.264 proxy probe source did not start.");
        }
        if (sourceError[0] != null) {
            throw sourceError[0];
        }

        Thread proxyThread = new Thread(new Runnable() {
            @Override
            public void run() {
                runProxy(
                    probeSink,
                    sessionId,
                    host,
                    remotePort,
                    host,
                    localPort,
                    remoteEndpoint,
                    localEndpoint,
                    timeoutMs,
                    timeoutMs);
            }
        }, "RustyXrH264ProxyProbeProxy");
        proxyThread.start();

        Thread consumerThread = new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    runProbeConsumer(host, localPort, timeoutMs, consumerResult);
                } catch (Exception ex) {
                    consumerError[0] = ex;
                }
            }
        }, "RustyXrH264ProxyProbeConsumer");
        consumerThread.start();

        sourceThread.join(timeoutMs);
        proxyThread.join(timeoutMs);
        consumerThread.join(timeoutMs);

        JSONObject report = new JSONObject();
        report.put("schema", "rusty.xr.broker.h264_tcp_proxy_probe.v1");
        report.put("session_id", sessionId);
        report.put("remote_endpoint", remoteEndpoint);
        report.put("local_endpoint", localEndpoint);
        report.put("expected_packet_count", packetCount);
        report.put("expected_packet_bytes", packetBytes);
        report.put("expected_payload_size_bytes", sourceResult.payloadBytes);
        report.put("expected_payload_checksum", sourceResult.payloadChecksum);
        report.put("received_packet_count", consumerResult.packetCount);
        report.put("received_payload_size_bytes", consumerResult.payloadBytes);
        report.put("received_payload_checksum", consumerResult.payloadChecksum);
        report.put("manifest_count", probeSink.manifestCount());
        report.put("sample_count", probeSink.sampleCount());
        report.put("metric_count", probeSink.metricCount());
        report.put("proxy_metric", probeSink.latestMetric());

        String lastError = "";
        if (sourceThread.isAlive()) {
            lastError = "Synthetic source thread did not finish.";
        } else if (proxyThread.isAlive()) {
            lastError = "Proxy thread did not finish.";
        } else if (consumerThread.isAlive()) {
            lastError = "Synthetic consumer thread did not finish.";
        } else if (sourceError[0] != null) {
            lastError = sourceError[0].getClass().getSimpleName() + ": " + safeMessage(sourceError[0]);
        } else if (consumerError[0] != null) {
            lastError = consumerError[0].getClass().getSimpleName() + ": " + safeMessage(consumerError[0]);
        } else if (probeSink.latestMetric().optString("last_error", "").length() > 0) {
            lastError = probeSink.latestMetric().optString("last_error", "");
        }

        boolean succeeded = lastError.length() == 0 &&
            consumerResult.packetCount == packetCount &&
            consumerResult.payloadBytes == sourceResult.payloadBytes &&
            consumerResult.payloadChecksum == sourceResult.payloadChecksum &&
            probeSink.sampleCount() == packetCount;
        report.put("succeeded", succeeded);
        if (lastError.length() > 0) {
            report.put("last_error", lastError);
        }
        return report;
    }

    private static void runProxy(
        Sink sink,
        String sessionId,
        String remoteHost,
        int remotePort,
        String localBindHost,
        int localPort,
        JSONObject remoteEndpoint,
        JSONObject localEndpoint,
        int connectTimeoutMs,
        int acceptTimeoutMs) {
        ServerSocket localServer = null;
        Socket remoteSocket = null;
        Socket localSocket = null;
        ProxyStats stats = new ProxyStats();
        Header header = new Header();
        String lastError = "";
        try {
            stats.localListenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            localServer = new ServerSocket(localPort, 1, InetAddress.getByName(localBindHost));
            localServer.setSoTimeout(acceptTimeoutMs);
            Log.i(TAG, "H264 TCP proxy listening session=" + sessionId +
                " local=" + localBindHost + ":" + localPort +
                " remote=" + remoteHost + ":" + remotePort);

            localSocket = localServer.accept();
            stats.localAcceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            localSocket.setTcpNoDelay(true);
            Log.i(TAG, "H264 TCP proxy accepted local consumer session=" + sessionId +
                " localRemote=" + localSocket.getRemoteSocketAddress() +
                " remote=" + remoteHost + ":" + remotePort);
            OutputStream localOutput = localSocket.getOutputStream();

            // Attach the local consumer before opening the live remote source so
            // the sender does not start flowing into an unconsumed proxy socket.
            remoteSocket = new Socket();
            remoteSocket.setTcpNoDelay(true);
            Log.i(TAG, "H264 TCP proxy connecting remote session=" + sessionId +
                " remote=" + remoteHost + ":" + remotePort);
            remoteSocket.connect(new InetSocketAddress(remoteHost, remotePort), connectTimeoutMs);
            stats.remoteConnectElapsedNs = SystemClock.elapsedRealtimeNanos();
            Log.i(TAG, "H264 TCP proxy connected remote session=" + sessionId +
                " remoteLocal=" + remoteSocket.getLocalSocketAddress() +
                " remotePeer=" + remoteSocket.getRemoteSocketAddress());

            BufferedInputStream remoteInput = new BufferedInputStream(remoteSocket.getInputStream());
            header = readHeader(remoteInput);
            stats.wireBytes += header.raw.length;
            Log.i(TAG, "H264 TCP proxy header session=" + sessionId +
                " schema=" + header.schemaVersion +
                " codec=" + header.codecId +
                " size=" + header.width + "x" + header.height +
                " packetCount=" + header.packetCount +
                " headerMetadataBytes=" + header.headerMetadataBytes);
            registerManifest(sink, sessionId, remoteEndpoint, localEndpoint, header);
            stats.forwardStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            localOutput.write(header.raw);

            byte[] buffer = new byte[32 * 1024];
            for (int index = 0; header.packetCount == 0 || index < header.packetCount; index++) {
                PacketHeader packet;
                try {
                    packet = readPacketHeader(remoteInput, header.schemaVersion);
                } catch (EOFException eof) {
                    if (header.packetCount == 0) {
                        break;
                    }
                    throw eof;
                }
                if (header.declaredPacketBytes > 0 && packet.sizeBytes != header.declaredPacketBytes) {
                    throw new IllegalStateException("Remote packet size did not match declared packet bytes.");
                }
                localOutput.write(packet.raw);
                copyExactly(remoteInput, localOutput, packet.sizeBytes, buffer);
                stats.packetCount++;
                stats.payloadBytes += packet.sizeBytes;
                stats.wireBytes += packet.raw.length + packet.sizeBytes;
                recordSample(sink, sessionId, remoteEndpoint, localEndpoint, header, packet, index);
                localOutput.flush();
            }
            stats.forwardEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            Log.i(TAG, "H264 TCP proxy completed session=" + sessionId +
                " packets=" + stats.packetCount +
                " payloadBytes=" + stats.payloadBytes +
                " wireBytes=" + stats.wireBytes);
        } catch (Exception ex) {
            stats.forwardEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            lastError = ex.getClass().getSimpleName() + ": " + safeMessage(ex);
            Log.w(TAG, "H264 TCP proxy failed session=" + sessionId +
                " local=" + localBindHost + ":" + localPort +
                " remote=" + remoteHost + ":" + remotePort +
                " error=" + lastError, ex);
        } finally {
            closeQuietly(localSocket);
            closeQuietly(remoteSocket);
            closeQuietly(localServer);
            try {
                recordMetric(sink, sessionId, remoteEndpoint, localEndpoint, header, stats, lastError);
            } catch (Exception ignored) {
            }
            Log.i(TAG, "H264 TCP proxy final session=" + sessionId +
                " packets=" + stats.packetCount +
                " payloadBytes=" + stats.payloadBytes +
                " wireBytes=" + stats.wireBytes +
                " lastError=" + lastError);
        }
    }

    private static void runSyntheticSource(
        String host,
        int port,
        int width,
        int height,
        int packetCount,
        int packetBytes,
        int timeoutMs,
        CountDownLatch ready,
        ProbeSourceResult result) throws Exception {
        ServerSocket server = null;
        Socket socket = null;
        try {
            server = new ServerSocket(port, 1, InetAddress.getByName(host));
            server.setSoTimeout(timeoutMs);
            ready.countDown();
            socket = server.accept();
            socket.setTcpNoDelay(true);
            OutputStream output = socket.getOutputStream();
            writeSyntheticHeader(output, width, height, packetCount);
            for (int index = 0; index < packetCount; index++) {
                byte[] payload = syntheticPayload(index, packetBytes);
                int flags = index == 0 ? FLAG_CODEC_CONFIG : (index == 1 ? FLAG_KEY_FRAME : 0);
                writeSyntheticPacket(output, index * 33333L, flags, payload);
                result.packetCount++;
                result.payloadBytes += payload.length;
                result.payloadChecksum += checksum(payload);
            }
            output.flush();
        } finally {
            closeQuietly(socket);
            closeQuietly(server);
        }
    }

    private static void runProbeConsumer(String host, int port, int timeoutMs, ProbeConsumerResult result) throws Exception {
        Socket socket = connectWithRetry(host, port, timeoutMs);
        try {
            BufferedInputStream input = new BufferedInputStream(socket.getInputStream());
            Header header = readHeader(input);
            result.width = header.width;
            result.height = header.height;
            result.schemaVersion = header.schemaVersion;
            byte[] buffer = new byte[32 * 1024];
            for (int index = 0; index < header.packetCount; index++) {
                PacketHeader packet = readPacketHeader(input, header.schemaVersion);
                byte[] payload = readPacketPayload(input, packet.sizeBytes, buffer);
                result.packetCount++;
                result.payloadBytes += payload.length;
                result.payloadChecksum += checksum(payload);
            }
        } finally {
            closeQuietly(socket);
        }
    }

    private static Socket connectWithRetry(String host, int port, int timeoutMs) throws Exception {
        long deadline = SystemClock.elapsedRealtime() + timeoutMs;
        Exception last = null;
        while (SystemClock.elapsedRealtime() < deadline) {
            Socket socket = new Socket();
            try {
                socket.setTcpNoDelay(true);
                socket.connect(new InetSocketAddress(host, port), Math.min(500, timeoutMs));
                return socket;
            } catch (Exception ex) {
                last = ex;
                closeQuietly(socket);
                Thread.sleep(20);
            }
        }
        throw new IllegalStateException("Timed out connecting to proxy local stream: " + safeMessage(last));
    }

    private static Header readHeader(InputStream input) throws Exception {
        byte[] headerRaw = readExact(input, 32);
        String magic = new String(headerRaw, 0, 8, StandardCharsets.US_ASCII);
        if (!MAGIC.equals(magic)) {
            throw new IllegalStateException("Remote H.264 stream returned unexpected magic: " + magic);
        }

        int schemaVersion = readI32(headerRaw, 8);
        int codecId = readI32(headerRaw, 12);
        int width = readI32(headerRaw, 16);
        int height = readI32(headerRaw, 20);
        int packetCount = readI32(headerRaw, 24);
        int declaredPacketBytes = readI32(headerRaw, 28);
        int headerMetadataBytes = 0;
        byte[] raw = headerRaw;
        if (schemaVersion < 1 || schemaVersion > 3) {
            throw new IllegalStateException("Unsupported RXYRVID1 schema version: " + schemaVersion);
        }
        if (codecId != CODEC_H264) {
            throw new IllegalStateException("Unsupported proxy codec id: " + codecId);
        }
        if (width <= 0 || height <= 0) {
            throw new IllegalStateException("Invalid remote stream dimensions: " + width + "x" + height);
        }
        if (packetCount < 0 || packetCount > MAX_PACKET_COUNT) {
            throw new IllegalStateException("Invalid remote stream packet count: " + packetCount);
        }
        if (schemaVersion >= 3) {
            headerMetadataBytes = declaredPacketBytes;
            if (headerMetadataBytes < 0 || headerMetadataBytes > MAX_STREAM_HEADER_METADATA_BYTES) {
                throw new IllegalStateException("Invalid remote stream header metadata bytes: " + headerMetadataBytes);
            }
            byte[] metadataRaw = readExact(input, headerMetadataBytes);
            raw = new byte[headerRaw.length + metadataRaw.length];
            System.arraycopy(headerRaw, 0, raw, 0, headerRaw.length);
            System.arraycopy(metadataRaw, 0, raw, headerRaw.length, metadataRaw.length);
            declaredPacketBytes = 0;
        } else if (declaredPacketBytes < 0 || declaredPacketBytes > MAX_PACKET_BYTES) {
            throw new IllegalStateException("Invalid remote declared packet bytes: " + declaredPacketBytes);
        }

        Header header = new Header();
        header.raw = raw;
        header.schemaVersion = schemaVersion;
        header.codecId = codecId;
        header.width = width;
        header.height = height;
        header.packetCount = packetCount;
        header.declaredPacketBytes = declaredPacketBytes;
        header.headerMetadataBytes = headerMetadataBytes;
        return header;
    }

    private static PacketHeader readPacketHeader(InputStream input, int schemaVersion) throws Exception {
        int length = schemaVersion >= 2 ? 32 : 16;
        byte[] raw = readExact(input, length);
        PacketHeader packet = new PacketHeader();
        packet.raw = raw;
        packet.ptsUs = readI64(raw, 0);
        packet.flags = readI32(raw, 8);
        packet.sizeBytes = readI32(raw, 12);
        if (schemaVersion >= 2) {
            packet.sourceElapsedNs = readI64(raw, 16);
            packet.sourceUnixNs = readI64(raw, 24);
        }
        if (packet.sizeBytes <= 0 || packet.sizeBytes > MAX_PACKET_BYTES) {
            throw new IllegalStateException("Invalid remote packet size: " + packet.sizeBytes);
        }
        return packet;
    }

    private static void copyExactly(InputStream input, OutputStream output, int byteCount, byte[] buffer) throws Exception {
        int remaining = byteCount;
        while (remaining > 0) {
            int read = input.read(buffer, 0, Math.min(buffer.length, remaining));
            if (read < 0) {
                throw new EOFException("Remote stream ended before packet payload was complete.");
            }
            output.write(buffer, 0, read);
            remaining -= read;
        }
    }

    private static byte[] readPacketPayload(InputStream input, int byteCount, byte[] scratch) throws Exception {
        byte[] payload = new byte[byteCount];
        int offset = 0;
        while (offset < byteCount) {
            int read = input.read(scratch, 0, Math.min(scratch.length, byteCount - offset));
            if (read < 0) {
                throw new EOFException("Proxy stream ended before packet payload was complete.");
            }
            System.arraycopy(scratch, 0, payload, offset, read);
            offset += read;
        }
        return payload;
    }

    private static byte[] readExact(InputStream input, int byteCount) throws Exception {
        byte[] buffer = new byte[byteCount];
        int offset = 0;
        while (offset < byteCount) {
            int read = input.read(buffer, offset, byteCount - offset);
            if (read < 0) {
                throw new EOFException("Remote stream ended early.");
            }
            offset += read;
        }
        return buffer;
    }

    private static void registerManifest(
        Sink sink,
        String sessionId,
        JSONObject remoteEndpoint,
        JSONObject localEndpoint,
        Header header) throws Exception {
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", "broker_peer.h264_tcp_proxy");
        manifest.put("session_id", sessionId);
        manifest.put("source", "broker_peer_h264_tcp_proxy");
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", "broker_peer_tcp_binary_proxy");
        manifest.put("mime_type", "video/avc");
        manifest.put("codec", "h264");
        manifest.put("decoder_target", "surface");
        manifest.put("width", header.width);
        manifest.put("height", header.height);
        manifest.put("packet_count", header.packetCount);
        manifest.put("binary_schema_version", header.schemaVersion);
        manifest.put("stream_header_metadata_bytes", header.headerMetadataBytes);
        manifest.put("remote_endpoint", new JSONObject(remoteEndpoint.toString()));
        manifest.put("local_endpoint", new JSONObject(localEndpoint.toString()));
        sink.registerManifest(manifest);
    }

    private static void recordSample(
        Sink sink,
        String sessionId,
        JSONObject remoteEndpoint,
        JSONObject localEndpoint,
        Header header,
        PacketHeader packet,
        int index) throws Exception {
        JSONObject sample = new JSONObject();
        sample.put("schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
        sample.put("stream_id", "broker_peer.h264_tcp_proxy");
        sample.put("session_id", sessionId);
        sample.put("sequence_id", System.currentTimeMillis() * 1000L + index);
        sample.put("source", "broker_peer_h264_tcp_proxy");
        sample.put("transport", "metadata_only");
        sample.put("payload_transport", "broker_peer_tcp_binary_proxy");
        sample.put("mime_type", "video/avc");
        sample.put("codec", "h264");
        sample.put("encoded_size_bytes", packet.sizeBytes);
        sample.put("width", header.width);
        sample.put("height", header.height);
        sample.put("key_frame", (packet.flags & FLAG_KEY_FRAME) != 0);
        sample.put("codec_config", (packet.flags & FLAG_CODEC_CONFIG) != 0);
        sample.put("pts_us", packet.ptsUs);
        sample.put("dts_us", packet.ptsUs);
        sample.put("source_time_unix_ns", packet.sourceUnixNs);
        sample.put("source_time_elapsed_ns", packet.sourceElapsedNs);
        sample.put("remote_endpoint", new JSONObject(remoteEndpoint.toString()));
        sample.put("local_endpoint", new JSONObject(localEndpoint.toString()));
        sink.recordSample(sample);
    }

    private static void recordMetric(
        Sink sink,
        String sessionId,
        JSONObject remoteEndpoint,
        JSONObject localEndpoint,
        Header header,
        ProxyStats stats,
        String lastError) throws Exception {
        JSONObject metric = new JSONObject();
        metric.put("schema", "rusty.xr.video_lab.metric_sample.v1");
        metric.put("stream_id", "broker_peer.h264_tcp_proxy");
        metric.put("source", "broker_peer_h264_tcp_proxy");
        metric.put("transport", "metadata_only");
        metric.put("payload_transport", "broker_peer_tcp_binary_proxy");
        metric.put("codec", "h264");
        metric.put("session_id", sessionId);
        metric.put("sequence_id", System.currentTimeMillis() * 1000L);
        metric.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
        metric.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        metric.put("remote_connect_elapsed_ns", stats.remoteConnectElapsedNs);
        metric.put("local_listen_start_elapsed_ns", stats.localListenStartElapsedNs);
        metric.put("local_accept_elapsed_ns", stats.localAcceptElapsedNs);
        metric.put("proxy_forward_start_elapsed_ns", stats.forwardStartElapsedNs);
        metric.put("proxy_forward_end_elapsed_ns", stats.forwardEndElapsedNs);
        metric.put("proxy_forward_duration_ns", Math.max(0L, stats.forwardEndElapsedNs - stats.forwardStartElapsedNs));
        metric.put("packet_count", stats.packetCount);
        metric.put("payload_size_bytes", stats.payloadBytes);
        metric.put("wire_size_bytes", stats.wireBytes);
        metric.put("width", header.width);
        metric.put("height", header.height);
        metric.put("binary_schema_version", header.schemaVersion);
        metric.put("stream_header_metadata_bytes", header.headerMetadataBytes);
        metric.put("remote_endpoint", new JSONObject(remoteEndpoint.toString()));
        metric.put("local_endpoint", new JSONObject(localEndpoint.toString()));
        metric.put("dropped_frames", 0);
        metric.put("stale_frames", 0);
        metric.put("queue_depth", 0);
        if (lastError != null && lastError.length() > 0) {
            metric.put("last_error", lastError);
        }
        sink.recordMetric(metric);
    }

    private static String normalizeLocalBindHost(String requestedHost, boolean localLanEnabled) {
        String host = requestedHost != null ? requestedHost.trim() : "";
        if (host.length() == 0) {
            return localLanEnabled ? "0.0.0.0" : "127.0.0.1";
        }
        if (!localLanEnabled && !isLoopbackBindHost(host)) {
            throw new IllegalArgumentException("Non-loopback proxy local_bind_host requires local_lan_enabled=true.");
        }
        return host;
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
        if (value < min) {
            return min;
        }
        if (value > max) {
            return max;
        }
        return value;
    }

    private static int allocateLoopbackPort() throws Exception {
        ServerSocket server = new ServerSocket(0, 1, InetAddress.getByName("127.0.0.1"));
        try {
            return server.getLocalPort();
        } finally {
            server.close();
        }
    }

    private static int allocateLoopbackPortExcluding(int excludedPort) throws Exception {
        for (int attempt = 0; attempt < 16; attempt++) {
            int port = allocateLoopbackPort();
            if (port != excludedPort) {
                return port;
            }
        }

        throw new IllegalStateException("Could not allocate a distinct loopback port for the H.264 proxy probe.");
    }

    private static void writeSyntheticHeader(OutputStream output, int width, int height, int packetCount) throws Exception {
        output.write(MAGIC.getBytes(StandardCharsets.US_ASCII));
        writeI32(output, 2);
        writeI32(output, CODEC_H264);
        writeI32(output, width);
        writeI32(output, height);
        writeI32(output, packetCount);
        writeI32(output, 0);
    }

    private static void writeSyntheticPacket(OutputStream output, long ptsUs, int flags, byte[] payload) throws Exception {
        writeI64(output, ptsUs);
        writeI32(output, flags);
        writeI32(output, payload.length);
        writeI64(output, SystemClock.elapsedRealtimeNanos());
        writeI64(output, System.currentTimeMillis() * 1_000_000L);
        output.write(payload);
    }

    private static void writeI32(OutputStream output, int value) throws Exception {
        output.write((value >>> 24) & 0xff);
        output.write((value >>> 16) & 0xff);
        output.write((value >>> 8) & 0xff);
        output.write(value & 0xff);
    }

    private static void writeI64(OutputStream output, long value) throws Exception {
        output.write((int) ((value >>> 56) & 0xff));
        output.write((int) ((value >>> 48) & 0xff));
        output.write((int) ((value >>> 40) & 0xff));
        output.write((int) ((value >>> 32) & 0xff));
        output.write((int) ((value >>> 24) & 0xff));
        output.write((int) ((value >>> 16) & 0xff));
        output.write((int) ((value >>> 8) & 0xff));
        output.write((int) (value & 0xff));
    }

    private static byte[] syntheticPayload(int packetIndex, int byteCount) {
        byte[] payload = new byte[byteCount];
        for (int i = 0; i < payload.length; i++) {
            payload[i] = (byte) ((packetIndex * 31 + i) & 0xff);
        }
        return payload;
    }

    private static long checksum(byte[] payload) {
        long value = 0L;
        for (int i = 0; i < payload.length; i++) {
            value += payload[i] & 0xff;
        }
        return value;
    }

    private static int readI32(byte[] bytes, int offset) {
        return ((bytes[offset] & 0xff) << 24) |
            ((bytes[offset + 1] & 0xff) << 16) |
            ((bytes[offset + 2] & 0xff) << 8) |
            (bytes[offset + 3] & 0xff);
    }

    private static long readI64(byte[] bytes, int offset) {
        return ((long) (bytes[offset] & 0xff) << 56) |
            ((long) (bytes[offset + 1] & 0xff) << 48) |
            ((long) (bytes[offset + 2] & 0xff) << 40) |
            ((long) (bytes[offset + 3] & 0xff) << 32) |
            ((long) (bytes[offset + 4] & 0xff) << 24) |
            ((long) (bytes[offset + 5] & 0xff) << 16) |
            ((long) (bytes[offset + 6] & 0xff) << 8) |
            (long) (bytes[offset + 7] & 0xff);
    }

    private static void closeQuietly(Closeable closeable) {
        if (closeable == null) {
            return;
        }
        try {
            closeable.close();
        } catch (Exception ignored) {
        }
    }

    private static String safeMessage(Exception ex) {
        if (ex == null) {
            return "";
        }
        String message = ex.getMessage();
        return message != null ? message : "";
    }

    private static final class Header {
        byte[] raw = new byte[0];
        int schemaVersion;
        int codecId;
        int width;
        int height;
        int packetCount;
        int declaredPacketBytes;
        int headerMetadataBytes;
    }

    private static final class PacketHeader {
        byte[] raw = new byte[0];
        long ptsUs;
        int flags;
        int sizeBytes;
        long sourceElapsedNs;
        long sourceUnixNs;
    }

    private static final class ProxyStats {
        long localListenStartElapsedNs;
        long remoteConnectElapsedNs;
        long localAcceptElapsedNs;
        long forwardStartElapsedNs;
        long forwardEndElapsedNs;
        int packetCount;
        long payloadBytes;
        long wireBytes;
    }

    private static final class ProbeSourceResult {
        int packetCount;
        long payloadBytes;
        long payloadChecksum;
    }

    private static final class ProbeConsumerResult {
        int schemaVersion;
        int width;
        int height;
        int packetCount;
        long payloadBytes;
        long payloadChecksum;
    }

    private static final class ProbeSink implements Sink {
        private final Sink inner;
        private int manifestCount;
        private int sampleCount;
        private int metricCount;
        private JSONObject latestMetric = new JSONObject();

        ProbeSink(Sink inner) {
            this.inner = inner;
        }

        @Override
        public synchronized void registerManifest(JSONObject manifest) throws Exception {
            manifestCount++;
            if (inner != null) {
                inner.registerManifest(manifest);
            }
        }

        @Override
        public synchronized void recordSample(JSONObject sample) throws Exception {
            sampleCount++;
            if (inner != null) {
                inner.recordSample(sample);
            }
        }

        @Override
        public synchronized void recordMetric(JSONObject metric) throws Exception {
            metricCount++;
            latestMetric = new JSONObject(metric.toString());
            if (inner != null) {
                inner.recordMetric(metric);
            }
        }

        synchronized int manifestCount() {
            return manifestCount;
        }

        synchronized int sampleCount() {
            return sampleCount;
        }

        synchronized int metricCount() {
            return metricCount;
        }

        synchronized JSONObject latestMetric() throws Exception {
            return new JSONObject(latestMetric.toString());
        }
    }
}
