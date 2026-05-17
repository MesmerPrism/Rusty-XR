package com.example.rustyxr.broker;

import android.content.Context;
import android.os.SystemClock;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.Closeable;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.security.KeyStore;
import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.atomic.AtomicLong;

import javax.net.ssl.SSLContext;
import javax.net.ssl.SSLParameters;
import javax.net.ssl.SSLSocket;
import javax.net.ssl.SSLSocketFactory;
import javax.net.ssl.TrustManager;
import javax.net.ssl.TrustManagerFactory;
import javax.net.ssl.X509TrustManager;

final class BrokerQ2QRelayClientSession {
    private static final String TAG = "RustyXrBroker";
    private static final String HELLO_SCHEMA = "rusty.xr.q2q.relay.hello.v1";
    private static final String ACK_SCHEMA = "rusty.xr.q2q.relay.ack.v1";
    private static final String STATUS_SCHEMA = "rusty.xr.broker.q2q_relay.status.v1";
    private static final String START_SCHEMA = "rusty.xr.broker.q2q_relay.start.v1";
    private static final int DEFAULT_RELAY_PORT = 9443;
    private static final int DEFAULT_SENDER_LEFT_SOURCE_PORT = 8879;
    private static final int DEFAULT_SENDER_RIGHT_SOURCE_PORT = 8880;
    private static final int DEFAULT_RECEIVER_LEFT_LOCAL_PORT = 8979;
    private static final int DEFAULT_RECEIVER_RIGHT_LOCAL_PORT = 8980;
    private static final int DEFAULT_CONNECT_TIMEOUT_MS = 15000;
    private static final int DEFAULT_LOCAL_ACCEPT_TIMEOUT_MS = 120000;
    private static final int DEFAULT_SOURCE_ACCEPT_TIMEOUT_MS = 120000;
    private static final int DEFAULT_CAPTURE_MS = 60000;
    private static final int DEFAULT_MAX_PACKETS = 0;
    private static final int DEFAULT_WIDTH = 720;
    private static final int DEFAULT_HEIGHT = 480;
    private static final int MAX_TIMEOUT_MS = 120000;
    private static final int MAX_HELLO_BYTES = 16 * 1024;
    private static final int BUFFER_BYTES = 64 * 1024;

    private static final Object LOCK = new Object();
    private static final Map<String, Lane> LANES = new LinkedHashMap<>();
    private static final AtomicLong NEXT_LANE_ID = new AtomicLong(1L);
    private static long createdLanes;
    private static long closedLanes;
    private static long failedLanes;

    private BrokerQ2QRelayClientSession() {
    }

    static JSONObject startSender(
        Context context,
        JSONObject params,
        BrokerAppCameraH264StreamSession.Sink sink) throws Exception {
        JSONObject safeParams = params != null ? params : new JSONObject();
        String sessionId = requiredString(safeParams, "session_id");
        String sourceMode = normalizeSourceMode(safeParams.optString("source_mode", "synthetic"));
        boolean startSource = safeParams.optBoolean("start_source", true);
        List<String> eyes = parseEyes(safeParams);

        JSONArray sourceStarts = new JSONArray();
        JSONArray laneStarts = new JSONArray();
        for (String eye : eyes) {
            int sourcePort = senderSourcePort(safeParams, eye);
            String sourceHost = safeParams.optString("source_host", "127.0.0.1").trim();
            if (sourceHost.length() == 0) {
                sourceHost = "127.0.0.1";
            }

            if (startSource) {
                JSONObject sourceParams = buildSourceParams(safeParams, sessionId, eye, sourceHost, sourcePort, sourceMode);
                JSONObject sourceStart = "synthetic".equals(sourceMode)
                    ? BrokerAppCameraH264StreamSession.startSynthetic(context, sourceParams, sink)
                    : BrokerAppCameraH264StreamSession.start(context, sourceParams, sink);
                sourceStarts.put(sourceStart);
            }

            Lane lane = Lane.sender(safeParams, sessionId, eye, sourceHost, sourcePort);
            registerAndStart(lane);
            laneStarts.put(lane.toJson(false));
        }

        JSONObject result = new JSONObject();
        result.put("schema", START_SCHEMA);
        result.put("role", "sender");
        result.put("session_id", sessionId);
        result.put("source_mode", sourceMode);
        result.put("started_source_count", sourceStarts.length());
        result.put("source_starts", sourceStarts);
        result.put("lanes", laneStarts);
        result.put("status", statusJson(null));
        return result;
    }

    static JSONObject startReceiver(JSONObject params) throws Exception {
        JSONObject safeParams = params != null ? params : new JSONObject();
        String sessionId = requiredString(safeParams, "session_id");
        List<String> eyes = parseEyes(safeParams);
        JSONArray laneStarts = new JSONArray();
        for (String eye : eyes) {
            Lane lane = Lane.receiver(safeParams, sessionId, eye);
            registerAndStart(lane);
            laneStarts.put(lane.toJson(false));
        }

        JSONObject result = new JSONObject();
        result.put("schema", START_SCHEMA);
        result.put("role", "receiver");
        result.put("session_id", sessionId);
        result.put("lanes", laneStarts);
        result.put("status", statusJson(null));
        return result;
    }

    static JSONObject stop(JSONObject params) throws Exception {
        JSONObject safeParams = params != null ? params : new JSONObject();
        JSONArray stopped = new JSONArray();
        List<Lane> matches = new ArrayList<>();
        synchronized (LOCK) {
            for (Lane lane : LANES.values()) {
                if (matchesFilter(lane, safeParams)) {
                    matches.add(lane);
                }
            }
        }
        for (Lane lane : matches) {
            lane.stopRequested = true;
            lane.state = "stopping";
            closeQuietly(lane.serverSocket);
            closeQuietly(lane.sourceSocket);
            closeQuietly(lane.localClientSocket);
            closeQuietly(lane.relaySocket);
            Thread thread = lane.thread;
            if (thread != null) {
                thread.interrupt();
            }
            stopped.put(lane.toJson(false));
        }

        JSONObject result = new JSONObject();
        result.put("schema", STATUS_SCHEMA);
        result.put("stopped_count", stopped.length());
        result.put("stopped", stopped);
        result.put("status", statusJson(safeParams));
        return result;
    }

    static JSONObject statusJson(JSONObject params) throws Exception {
        JSONObject safeParams = params != null ? params : new JSONObject();
        JSONArray lanes = new JSONArray();
        long created;
        long closed;
        long failed;
        synchronized (LOCK) {
            created = createdLanes;
            closed = closedLanes;
            failed = failedLanes;
            for (Lane lane : LANES.values()) {
                if (matchesFilter(lane, safeParams)) {
                    lanes.put(lane.toJson(false));
                }
            }
        }

        JSONObject status = new JSONObject();
        status.put("schema", STATUS_SCHEMA);
        status.put("active_count", activeLaneCount());
        status.put("matched_count", lanes.length());
        status.put("created_count", created);
        status.put("closed_count", closed);
        status.put("failed_count", failed);
        status.put("lanes", lanes);
        return status;
    }

    private static void registerAndStart(final Lane lane) {
        synchronized (LOCK) {
            createdLanes++;
            LANES.put(lane.laneId, lane);
        }
        Thread thread = new Thread(new Runnable() {
            @Override
            public void run() {
                if ("sender".equals(lane.role)) {
                    runSender(lane);
                } else {
                    runReceiver(lane);
                }
            }
        }, "RustyXrQ2QRelay-" + lane.role + "-" + lane.eye);
        lane.thread = thread;
        thread.start();
    }

    private static void runSender(Lane lane) {
        RelayConnection relay = null;
        Socket source = null;
        try {
            lane.setState("relay_connecting");
            relay = connectRelay(lane);
            lane.relaySocket = relay.socket;
            lane.relayAck = relay.ack;
            lane.setState("source_connecting");
            source = connectTcpWithRetry(lane.sourceHost, lane.sourcePort, lane.connectTimeoutMs);
            lane.sourceSocket = source;
            source.setTcpNoDelay(true);
            lane.setState("copying");
            copyLoop(source.getInputStream(), relay.output, lane);
            relay.output.flush();
            shutdownOutput(relay.socket);
            lane.markClosed("source_eof");
        } catch (Exception ex) {
            lane.markFailed(ex);
        } finally {
            closeQuietly(source);
            closeQuietly(relay);
        }
    }

    private static void runReceiver(Lane lane) {
        RelayConnection relay = null;
        ServerSocket server = null;
        Socket local = null;
        try {
            lane.setState("waiting_for_local_client");
            server = new ServerSocket();
            server.setReuseAddress(true);
            server.bind(new InetSocketAddress(InetAddress.getByName(lane.localBindHost), lane.localPort));
            server.setSoTimeout(lane.localAcceptTimeoutMs);
            lane.serverSocket = server;
            local = server.accept();
            lane.localClientSocket = local;
            local.setTcpNoDelay(true);
            lane.setState("relay_connecting");
            relay = connectRelay(lane);
            lane.relaySocket = relay.socket;
            lane.relayAck = relay.ack;
            lane.setState("copying");
            copyLoop(relay.input, local.getOutputStream(), lane);
            local.getOutputStream().flush();
            shutdownOutput(local);
            lane.markClosed("relay_eof");
        } catch (Exception ex) {
            lane.markFailed(ex);
        } finally {
            closeQuietly(local);
            closeQuietly(server);
            closeQuietly(relay);
        }
    }

    private static RelayConnection connectRelay(Lane lane) throws Exception {
        Socket socket = new Socket();
        socket.setTcpNoDelay(true);
        socket.connect(new InetSocketAddress(lane.relayHost, lane.relayPort), lane.connectTimeoutMs);
        if (lane.tls) {
            SSLSocketFactory factory = sslSocketFactory(lane.caFile, lane.insecureTls);
            String serverName = lane.serverName.length() > 0 ? lane.serverName : lane.relayHost;
            SSLSocket sslSocket = (SSLSocket) factory.createSocket(socket, serverName, lane.relayPort, true);
            if (!lane.insecureTls) {
                SSLParameters parameters = sslSocket.getSSLParameters();
                parameters.setEndpointIdentificationAlgorithm("HTTPS");
                sslSocket.setSSLParameters(parameters);
            }
            sslSocket.startHandshake();
            socket = sslSocket;
        }

        BufferedInputStream input = new BufferedInputStream(socket.getInputStream());
        OutputStream output = socket.getOutputStream();
        JSONObject hello = new JSONObject();
        hello.put("schema", HELLO_SCHEMA);
        hello.put("role", lane.role);
        hello.put("session_id", lane.sessionId);
        hello.put("eye", lane.eye);
        hello.put("token", lane.token);
        hello.put("label", lane.label);
        hello.put("client_unix_ns", System.currentTimeMillis() * 1000000L);
        byte[] helloBytes = (hello.toString() + "\n").getBytes(StandardCharsets.UTF_8);
        output.write(helloBytes);
        output.flush();

        String ackLine = readLineLimited(input, MAX_HELLO_BYTES);
        JSONObject ack = new JSONObject(ackLine);
        if (!ACK_SCHEMA.equals(ack.optString("schema", ""))) {
            throw new IllegalStateException("Unexpected relay ack schema: " + ack.optString("schema", ""));
        }
        if (!ack.optBoolean("ok", false)) {
            throw new IllegalStateException("Relay rejected registration: " + ack.optString("message", "not ok"));
        }
        Log.i(TAG, "Q2Q relay registered lane=" + lane.laneId + " role=" + lane.role +
            " session=" + lane.sessionId + " eye=" + lane.eye);
        return new RelayConnection(socket, input, output, ack);
    }

    private static SSLSocketFactory sslSocketFactory(String caFile, boolean insecureTls) throws Exception {
        SSLContext context = SSLContext.getInstance("TLS");
        if (insecureTls) {
            context.init(null, new TrustManager[] { new InsecureTrustManager() }, null);
            return context.getSocketFactory();
        }
        if (caFile == null || caFile.trim().length() == 0) {
            return (SSLSocketFactory) SSLSocketFactory.getDefault();
        }

        CertificateFactory certificateFactory = CertificateFactory.getInstance("X.509");
        FileInputStream input = new FileInputStream(caFile);
        Certificate certificate;
        try {
            certificate = certificateFactory.generateCertificate(input);
        } finally {
            input.close();
        }
        KeyStore keyStore = KeyStore.getInstance(KeyStore.getDefaultType());
        keyStore.load(null, null);
        keyStore.setCertificateEntry("relay-ca", certificate);
        TrustManagerFactory trustManagerFactory = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm());
        trustManagerFactory.init(keyStore);
        context.init(null, trustManagerFactory.getTrustManagers(), null);
        return context.getSocketFactory();
    }

    private static Socket connectTcpWithRetry(String host, int port, int timeoutMs) throws Exception {
        long deadline = SystemClock.elapsedRealtime() + timeoutMs;
        Exception last = null;
        while (SystemClock.elapsedRealtime() <= deadline) {
            Socket socket = new Socket();
            try {
                socket.setTcpNoDelay(true);
                socket.connect(new InetSocketAddress(host, port), Math.min(2000, Math.max(100, timeoutMs)));
                return socket;
            } catch (Exception ex) {
                last = ex;
                closeQuietly(socket);
                sleepQuietly(100);
            }
        }
        throw new IllegalStateException("Timed out connecting to local H.264 source " + host + ":" + port +
            (last != null ? " after " + last.getClass().getSimpleName() + ": " + safeMessage(last) : ""));
    }

    private static void copyLoop(InputStream input, OutputStream output, Lane lane) throws Exception {
        byte[] buffer = new byte[BUFFER_BYTES];
        while (!lane.stopRequested) {
            int read = input.read(buffer);
            if (read < 0) {
                break;
            }
            if (read == 0) {
                continue;
            }
            output.write(buffer, 0, read);
            lane.bytesCopied.addAndGet(read);
            lane.lastByteElapsedMs = SystemClock.elapsedRealtime();
        }
        output.flush();
    }

    private static JSONObject buildSourceParams(
        JSONObject params,
        String sessionId,
        String eye,
        String sourceHost,
        int sourcePort,
        String sourceMode) throws Exception {
        JSONObject source = new JSONObject();
        copyIfPresent(params, source, "preferred_width");
        copyIfPresent(params, source, "preferred_height");
        copyIfPresent(params, source, "bitrate_bps");
        copyIfPresent(params, source, "frame_rate_hz");
        copyIfPresent(params, source, "writer_queue_depth");
        copyIfPresent(params, source, "quality_profile");
        copyIfPresent(params, source, "synthetic_pattern");
        copyIfPresent(params, source, "color_format");
        source.put("session_id", sessionId + "-" + eye + "-source");
        source.put("device_port", sourcePort);
        source.put("host_port", params.optInt(eye + "_source_host_port", sourcePort));
        source.put("preferred_width", params.optInt("preferred_width", DEFAULT_WIDTH));
        source.put("preferred_height", params.optInt("preferred_height", DEFAULT_HEIGHT));
        source.put("capture_ms", params.optInt("capture_ms", DEFAULT_CAPTURE_MS));
        source.put("max_packets", params.optInt("max_packets", DEFAULT_MAX_PACKETS));
        source.put(
            "accept_timeout_ms",
            params.optInt("source_accept_timeout_ms", params.optInt("accept_timeout_ms", DEFAULT_SOURCE_ACCEPT_TIMEOUT_MS)));
        source.put("live_stream", true);
        source.put("bind_host", sourceHost);
        source.put("advertised_host", sourceHost);
        source.put("lan_stream_enabled", false);
        source.put("source_mode", "camera".equals(sourceMode) ? "camera2" : "synthetic_surface");
        if ("synthetic".equals(sourceMode)) {
            source.put("synthetic_side_marker", eye);
        } else {
            String cameraId = params.optString(eye + "_camera_id", params.optString("camera_id", "")).trim();
            if (cameraId.length() > 0) {
                source.put("camera_id", cameraId);
            }
        }
        return source;
    }

    private static List<String> parseEyes(JSONObject params) throws Exception {
        ArrayList<String> eyes = new ArrayList<>();
        JSONArray array = params != null ? params.optJSONArray("eyes") : null;
        if (array != null) {
            for (int i = 0; i < array.length(); i++) {
                addEye(eyes, array.optString(i, ""));
            }
        } else {
            addEye(eyes, params != null ? params.optString("eye", "") : "");
        }
        if (eyes.isEmpty()) {
            if (params != null && params.has("left_eye") && !params.optBoolean("left_eye", true)) {
                // explicit false skips left
            } else {
                addEye(eyes, "left");
            }
            if (params != null && params.has("right_eye") && !params.optBoolean("right_eye", true)) {
                // explicit false skips right
            } else {
                addEye(eyes, "right");
            }
        }
        if (eyes.isEmpty()) {
            throw new IllegalArgumentException("q2q_relay requires at least one eye.");
        }
        return eyes;
    }

    private static void addEye(List<String> eyes, String value) {
        String eye = value != null ? value.trim().toLowerCase(Locale.US) : "";
        if ("both".equals(eye) || "stereo".equals(eye)) {
            addEye(eyes, "left");
            addEye(eyes, "right");
            return;
        }
        if (!"left".equals(eye) && !"right".equals(eye) && !"mono".equals(eye)) {
            return;
        }
        if (!eyes.contains(eye)) {
            eyes.add(eye);
        }
    }

    private static int senderSourcePort(JSONObject params, String eye) {
        if ("right".equals(eye)) {
            return clamp(params.optInt("right_source_port", DEFAULT_SENDER_RIGHT_SOURCE_PORT), 1, 65535);
        }
        if ("mono".equals(eye)) {
            return clamp(params.optInt("mono_source_port", DEFAULT_SENDER_LEFT_SOURCE_PORT), 1, 65535);
        }
        return clamp(params.optInt("left_source_port", DEFAULT_SENDER_LEFT_SOURCE_PORT), 1, 65535);
    }

    private static int receiverLocalPort(JSONObject params, String eye) {
        if ("right".equals(eye)) {
            return clamp(params.optInt("right_local_port", DEFAULT_RECEIVER_RIGHT_LOCAL_PORT), 1, 65535);
        }
        if ("mono".equals(eye)) {
            return clamp(params.optInt("mono_local_port", DEFAULT_RECEIVER_LEFT_LOCAL_PORT), 1, 65535);
        }
        return clamp(params.optInt("left_local_port", DEFAULT_RECEIVER_LEFT_LOCAL_PORT), 1, 65535);
    }

    private static String normalizeSourceMode(String value) {
        String mode = value != null ? value.trim().toLowerCase(Locale.US) : "";
        if ("camera".equals(mode) || "camera2".equals(mode) || "app_camera".equals(mode)) {
            return "camera";
        }
        return "synthetic";
    }

    private static String requiredString(JSONObject params, String key) {
        String value = params != null ? params.optString(key, "").trim() : "";
        if (value.length() == 0) {
            throw new IllegalArgumentException("q2q_relay requires params." + key + ".");
        }
        return value;
    }

    private static String readToken(JSONObject params) throws Exception {
        String token = params.optString("token", "");
        if (token.trim().length() > 0) {
            return token.trim();
        }
        String tokenFile = params.optString("token_file", "").trim();
        if (tokenFile.length() > 0) {
            FileInputStream input = new FileInputStream(tokenFile);
            try {
                ByteArrayOutputStream output = new ByteArrayOutputStream();
                byte[] buffer = new byte[4096];
                int read;
                while ((read = input.read(buffer)) >= 0) {
                    output.write(buffer, 0, read);
                }
                return new String(output.toByteArray(), StandardCharsets.UTF_8).trim();
            } finally {
                input.close();
            }
        }
        if (params.optBoolean("allow_empty_token", false)) {
            return "";
        }
        throw new IllegalArgumentException("q2q_relay requires params.token or params.token_file.");
    }

    private static boolean matchesFilter(Lane lane, JSONObject params) {
        if (params == null) {
            return true;
        }
        String laneId = params.optString("lane_id", "").trim();
        if (laneId.length() > 0 && !lane.laneId.equals(laneId)) {
            return false;
        }
        String sessionId = params.optString("session_id", "").trim();
        if (sessionId.length() > 0 && !lane.sessionId.equals(sessionId)) {
            return false;
        }
        String role = params.optString("role", "").trim();
        if (role.length() > 0 && !lane.role.equals(role)) {
            return false;
        }
        String eye = params.optString("eye", "").trim();
        return eye.length() == 0 || lane.eye.equals(eye);
    }

    private static int activeLaneCount() {
        int count = 0;
        synchronized (LOCK) {
            for (Lane lane : LANES.values()) {
                if (!lane.isTerminal()) {
                    count++;
                }
            }
        }
        return count;
    }

    private static void noteClosed(Lane lane, boolean failed) {
        synchronized (LOCK) {
            if (lane.terminalCounted) {
                return;
            }
            lane.terminalCounted = true;
            if (failed) {
                failedLanes++;
            } else {
                closedLanes++;
            }
        }
    }

    private static void copyIfPresent(JSONObject source, JSONObject target, String key) throws Exception {
        if (source != null && source.has(key)) {
            target.put(key, source.get(key));
        }
    }

    private static String readLineLimited(InputStream input, int maxBytes) throws Exception {
        ByteArrayOutputStream line = new ByteArrayOutputStream();
        while (line.size() < maxBytes) {
            int next = input.read();
            if (next < 0) {
                break;
            }
            if (next == '\n') {
                return new String(line.toByteArray(), StandardCharsets.UTF_8).trim();
            }
            if (next != '\r') {
                line.write(next);
            }
        }
        throw new IllegalStateException("Relay ack line missing or too large.");
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static int timeoutMs(JSONObject params, String key, int defaultValue) {
        return clamp(params.optInt(key, defaultValue), 100, MAX_TIMEOUT_MS);
    }

    private static void shutdownOutput(Socket socket) {
        if (socket != null) {
            try {
                socket.shutdownOutput();
            } catch (Exception ignored) {
            }
        }
    }

    private static void closeQuietly(Closeable closeable) {
        if (closeable != null) {
            try {
                closeable.close();
            } catch (Exception ignored) {
            }
        }
    }

    private static void sleepQuietly(long millis) {
        try {
            Thread.sleep(millis);
        } catch (InterruptedException ignored) {
            Thread.currentThread().interrupt();
        }
    }

    private static String safeMessage(Throwable throwable) {
        String message = throwable != null ? throwable.getMessage() : "";
        return message != null ? message : "";
    }

    private static final class RelayConnection implements Closeable {
        final Socket socket;
        final BufferedInputStream input;
        final OutputStream output;
        final JSONObject ack;

        RelayConnection(Socket socket, BufferedInputStream input, OutputStream output, JSONObject ack) {
            this.socket = socket;
            this.input = input;
            this.output = output;
            this.ack = ack;
        }

        @Override
        public void close() {
            closeQuietly(socket);
        }
    }

    private static final class Lane {
        final String laneId;
        final String role;
        final String sessionId;
        final String eye;
        final String relayHost;
        final int relayPort;
        final boolean tls;
        final boolean insecureTls;
        final String caFile;
        final String serverName;
        final String token;
        final String label;
        final int connectTimeoutMs;
        final String sourceHost;
        final int sourcePort;
        final String localBindHost;
        final int localPort;
        final int localAcceptTimeoutMs;
        final long startedElapsedMs;
        final long startedUnixMs;
        final AtomicLong bytesCopied = new AtomicLong();
        volatile long lastByteElapsedMs;
        volatile String state = "starting";
        volatile String closeReason = "";
        volatile String error = "";
        volatile boolean stopRequested;
        volatile boolean terminalCounted;
        volatile Thread thread;
        volatile Socket relaySocket;
        volatile Socket sourceSocket;
        volatile Socket localClientSocket;
        volatile ServerSocket serverSocket;
        volatile JSONObject relayAck;

        static Lane sender(JSONObject params, String sessionId, String eye, String sourceHost, int sourcePort) throws Exception {
            return new Lane(params, "sender", sessionId, eye, sourceHost, sourcePort, "", 0, 0);
        }

        static Lane receiver(JSONObject params, String sessionId, String eye) throws Exception {
            String bindHost = params.optString("local_bind_host", params.optString("bind_host", "127.0.0.1")).trim();
            if (bindHost.length() == 0) {
                bindHost = "127.0.0.1";
            }
            return new Lane(
                params,
                "receiver",
                sessionId,
                eye,
                "",
                0,
                bindHost,
                receiverLocalPort(params, eye),
                timeoutMs(params, "local_accept_timeout_ms", DEFAULT_LOCAL_ACCEPT_TIMEOUT_MS));
        }

        private Lane(
            JSONObject params,
            String role,
            String sessionId,
            String eye,
            String sourceHost,
            int sourcePort,
            String localBindHost,
            int localPort,
            int localAcceptTimeoutMs) throws Exception {
            this.laneId = "q2q-" + role + "-" + eye + "-" + System.currentTimeMillis() + "-" + NEXT_LANE_ID.getAndIncrement();
            this.role = role;
            this.sessionId = sessionId;
            this.eye = eye;
            this.relayHost = requiredString(params, "relay_host");
            this.relayPort = clamp(params.optInt("relay_port", DEFAULT_RELAY_PORT), 1, 65535);
            this.tls = params.optBoolean("tls", true);
            this.insecureTls = params.optBoolean("insecure_tls", false);
            this.caFile = params.optString("ca_file", "").trim();
            this.serverName = params.optString("server_name", params.optString("relay_server_name", relayHost)).trim();
            this.token = readToken(params);
            this.label = params.optString("label", "quest-native-" + role + "-" + eye).trim();
            this.connectTimeoutMs = timeoutMs(params, "connect_timeout_ms", DEFAULT_CONNECT_TIMEOUT_MS);
            this.sourceHost = sourceHost;
            this.sourcePort = sourcePort;
            this.localBindHost = localBindHost;
            this.localPort = localPort;
            this.localAcceptTimeoutMs = localAcceptTimeoutMs;
            this.startedElapsedMs = SystemClock.elapsedRealtime();
            this.startedUnixMs = System.currentTimeMillis();
            this.lastByteElapsedMs = 0L;
        }

        void setState(String nextState) {
            state = nextState;
        }

        void markClosed(String reason) {
            state = stopRequested ? "stopped" : "closed";
            closeReason = reason;
            noteClosed(this, false);
        }

        void markFailed(Exception ex) {
            state = stopRequested ? "stopped" : "failed";
            error = ex.getClass().getSimpleName() + ": " + safeMessage(ex);
            Log.w(TAG, "Q2Q relay lane failed lane=" + laneId + " " + error);
            noteClosed(this, !stopRequested);
        }

        boolean isTerminal() {
            return "closed".equals(state) || "failed".equals(state) || "stopped".equals(state);
        }

        JSONObject toJson(boolean includeToken) throws Exception {
            JSONObject json = new JSONObject();
            json.put("schema", "rusty.xr.broker.q2q_relay.lane.v1");
            json.put("lane_id", laneId);
            json.put("role", role);
            json.put("session_id", sessionId);
            json.put("eye", eye);
            json.put("state", state);
            json.put("relay_host", relayHost);
            json.put("relay_port", relayPort);
            json.put("tls", tls);
            json.put("server_name", serverName);
            json.put("ca_file", caFile);
            json.put("token_present", token != null && token.length() > 0);
            if (includeToken) {
                json.put("token", token);
            }
            json.put("label", label);
            json.put("connect_timeout_ms", connectTimeoutMs);
            if ("sender".equals(role)) {
                json.put("source_host", sourceHost);
                json.put("source_port", sourcePort);
            } else {
                json.put("local_bind_host", localBindHost);
                json.put("local_port", localPort);
                json.put("local_accept_timeout_ms", localAcceptTimeoutMs);
            }
            json.put("bytes_copied", bytesCopied.get());
            json.put("started_unix_ms", startedUnixMs);
            json.put("age_ms", SystemClock.elapsedRealtime() - startedElapsedMs);
            if (lastByteElapsedMs > 0L) {
                json.put("last_byte_age_ms", SystemClock.elapsedRealtime() - lastByteElapsedMs);
            }
            if (closeReason.length() > 0) {
                json.put("close_reason", closeReason);
            }
            if (error.length() > 0) {
                json.put("error", error);
            }
            if (relayAck != null) {
                json.put("relay_ack", relayAck);
            }
            return json;
        }
    }

    private static final class InsecureTrustManager implements X509TrustManager {
        @Override
        public void checkClientTrusted(java.security.cert.X509Certificate[] chain, String authType) {
        }

        @Override
        public void checkServerTrusted(java.security.cert.X509Certificate[] chain, String authType) {
        }

        @Override
        public java.security.cert.X509Certificate[] getAcceptedIssuers() {
            return new java.security.cert.X509Certificate[0];
        }
    }
}
