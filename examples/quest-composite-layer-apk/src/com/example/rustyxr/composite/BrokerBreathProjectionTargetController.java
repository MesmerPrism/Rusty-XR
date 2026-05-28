package com.example.rustyxr.composite;

import android.os.SystemClock;
import android.util.Base64;
import android.util.Log;
import org.json.JSONObject;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.Locale;
import java.util.Random;

final class BrokerBreathProjectionTargetController {
    private static final String TAG = "RustyXrComposite";
    private static final String STREAM_EVENT_SCHEMA = "rusty.xr.broker.stream_event.v1";
    private static final String CONTROL_SCHEMA = "rusty.xr.projection-target-breath-control.v1";
    private static final String EVENT_PATH = "/rustyxr/v1/events";

    interface Sink {
        void onProjectionTargetBreathControl(JSONObject update);
        void onProjectionTargetBreathStatus(JSONObject status);
    }

    static final class Config {
        final String brokerHost;
        final int brokerPort;
        final String stream;
        final float minScale;
        final float maxScale;
        final float smoothingAlpha;
        final boolean invert;
        final float minQuality;
        final int reconnectDelayMs;
        final int socketTimeoutMs;
        final int updateIntervalMs;
        final int logIntervalMs;
        final int maxFrameBytes;

        Config(
            String brokerHost,
            int brokerPort,
            String stream,
            float minScale,
            float maxScale,
            float smoothingAlpha,
            boolean invert,
            float minQuality,
            int reconnectDelayMs,
            int socketTimeoutMs,
            int updateIntervalMs,
            int logIntervalMs,
            int maxFrameBytes) {
            this.brokerHost = brokerHost != null && brokerHost.length() > 0 ? brokerHost : "127.0.0.1";
            this.brokerPort = Math.max(1, brokerPort);
            this.stream = stream != null && stream.length() > 0 ? stream : "bio:breath";
            this.minScale = clamp(minScale, 0.05f, 1.5f);
            this.maxScale = clamp(maxScale, this.minScale, 1.5f);
            this.smoothingAlpha = clamp(smoothingAlpha, 0.0f, 1.0f);
            this.invert = invert;
            this.minQuality = clamp(minQuality, 0.0f, 1.0f);
            this.reconnectDelayMs = Math.max(100, reconnectDelayMs);
            this.socketTimeoutMs = Math.max(500, socketTimeoutMs);
            this.updateIntervalMs = Math.max(16, updateIntervalMs);
            this.logIntervalMs = Math.max(100, logIntervalMs);
            this.maxFrameBytes = Math.max(1024, maxFrameBytes);
        }
    }

    private final Config config;
    private final Sink sink;
    private final Object socketLock = new Object();
    private volatile boolean running;
    private Thread worker;
    private Socket activeSocket;
    private Float smoothedScale;
    private long lastApplyMs;
    private long lastLogMs;
    private long appliedCount;
    private long ignoredCount;

    private BrokerBreathProjectionTargetController(Config config, Sink sink) {
        this.config = config;
        this.sink = sink;
    }

    static BrokerBreathProjectionTargetController start(Config config, Sink sink) {
        BrokerBreathProjectionTargetController controller =
            new BrokerBreathProjectionTargetController(config, sink);
        controller.start();
        return controller;
    }

    void stop() {
        running = false;
        closeActiveSocket();
        Thread thread = worker;
        if (thread != null) {
            thread.interrupt();
        }
    }

    private void start() {
        running = true;
        worker = new Thread(
            new Runnable() {
                @Override
                public void run() {
                    runLoop();
                }
            },
            "RustyXrBreathTarget");
        worker.start();
    }

    private void runLoop() {
        publishStatus("starting", null);
        while (running) {
            try {
                connectAndPump();
            } catch (Exception error) {
                if (running) {
                    Log.w(TAG, String.format(
                        Locale.US,
                        "Rusty XR projection-target breath controller disconnected target=%s:%d stream=%s error=%s",
                        config.brokerHost,
                        config.brokerPort,
                        config.stream,
                        safeMessage(error)));
                    publishStatus("disconnected", safeMessage(error));
                    sleepQuietly(config.reconnectDelayMs);
                }
            } finally {
                closeActiveSocket();
            }
        }
        publishStatus("stopped", null);
    }

    private void connectAndPump() throws Exception {
        Socket socket = new Socket();
        socket.connect(new InetSocketAddress(config.brokerHost, config.brokerPort), config.socketTimeoutMs);
        socket.setTcpNoDelay(true);
        socket.setSoTimeout(config.socketTimeoutMs);
        synchronized (socketLock) {
            activeSocket = socket;
        }

        InputStream input = socket.getInputStream();
        OutputStream output = socket.getOutputStream();
        writeWebSocketHandshake(output);
        readWebSocketHandshake(input);
        publishStatus("connected", null);

        JSONObject subscribe = new JSONObject();
        subscribe.put("type", "command");
        subscribe.put("command", "subscribe");
        subscribe.put("request_id", "projection-target-breath-" + SystemClock.elapsedRealtimeNanos());
        JSONObject params = new JSONObject();
        params.put("stream", config.stream);
        subscribe.put("params", params);
        sendMaskedTextFrame(output, subscribe.toString());
        Log.i(TAG, String.format(
            Locale.US,
            "Rusty XR projection-target breath controller subscribed stream=%s broker=%s:%d minScale=%.3f maxScale=%.3f smoothingAlpha=%.3f invert=%s minQuality=%.3f",
            config.stream,
            config.brokerHost,
            config.brokerPort,
            config.minScale,
            config.maxScale,
            config.smoothingAlpha,
            Boolean.toString(config.invert),
            config.minQuality));

        while (running) {
            String text = readWebSocketTextFrame(input, config.maxFrameBytes);
            if (text == null) {
                return;
            }
            if (text.length() == 0) {
                continue;
            }
            handleBrokerMessage(text);
        }
    }

    private void handleBrokerMessage(String text) throws Exception {
        JSONObject event = new JSONObject(text);
        String type = event.optString("type", "");
        if (!"stream_event".equals(type)) {
            return;
        }
        if (!STREAM_EVENT_SCHEMA.equals(event.optString("schema", STREAM_EVENT_SCHEMA))) {
            return;
        }
        if (!config.stream.equals(event.optString("stream", ""))) {
            return;
        }

        JSONObject payload = event.optJSONObject("payload");
        if (payload == null) {
            ignoredCount++;
            return;
        }
        if (!payload.optBoolean("has_volume", payload.has("volume01"))) {
            ignoredCount++;
            return;
        }

        double rawVolume = payload.optDouble("volume01", Double.NaN);
        if (!isFinite(rawVolume)) {
            ignoredCount++;
            return;
        }
        double rawQuality = payload.optDouble("quality01", 1.0d);
        float quality = isFinite(rawQuality) ? clamp((float) rawQuality, 0.0f, 1.0f) : 1.0f;
        if (quality < config.minQuality) {
            ignoredCount++;
            return;
        }

        float volume01 = clamp((float) rawVolume, 0.0f, 1.0f);
        float control01 = config.invert ? 1.0f - volume01 : volume01;
        float targetScale = config.minScale + control01 * (config.maxScale - config.minScale);
        float scale = smoothScale(targetScale);
        long nowMs = SystemClock.elapsedRealtime();
        if (nowMs - lastApplyMs < config.updateIntervalMs) {
            return;
        }
        lastApplyMs = nowMs;
        appliedCount++;

        JSONObject update = new JSONObject();
        update.put("schemaVersion", CONTROL_SCHEMA);
        update.put("source", "broker-breath");
        update.put("stream", config.stream);
        update.put("sequenceId", event.optLong("sequence_id", payload.optLong("sequence_id", 0L)));
        update.put("state", payload.optString("state", "unknown"));
        update.put("volume01", volume01);
        update.put("quality01", quality);
        double tracking = payload.optDouble("tracking01", Double.NaN);
        if (isFinite(tracking)) {
            update.put("tracking01", tracking);
        }
        update.put("projectionTargetScale", scale);
        update.put("projectionTargetBreathControl01", control01);
        update.put("projectionTargetBreathMinScale", config.minScale);
        update.put("projectionTargetBreathMaxScale", config.maxScale);
        update.put("projectionTargetBreathInvert", config.invert);
        sink.onProjectionTargetBreathControl(update);

        if (nowMs - lastLogMs >= config.logIntervalMs) {
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR projection-target breath tuning source=broker stream=%s sequenceId=%d state=%s volume01=%.4f quality01=%.4f targetScale=%.4f projectionTargetScale=%.4f applied=%d ignored=%d",
                config.stream,
                update.optLong("sequenceId", 0L),
                update.optString("state", "unknown"),
                volume01,
                quality,
                targetScale,
                scale,
                appliedCount,
                ignoredCount));
            lastLogMs = nowMs;
        }
    }

    private float smoothScale(float targetScale) {
        if (smoothedScale == null || config.smoothingAlpha >= 1.0f) {
            smoothedScale = targetScale;
            return targetScale;
        }
        if (config.smoothingAlpha <= 0.0f) {
            return smoothedScale.floatValue();
        }
        float next = smoothedScale.floatValue() + config.smoothingAlpha * (targetScale - smoothedScale.floatValue());
        smoothedScale = next;
        return next;
    }

    private void writeWebSocketHandshake(OutputStream output) throws Exception {
        String key = randomWebSocketKey();
        String request =
            "GET " + EVENT_PATH + " HTTP/1.1\r\n" +
            "Host: " + config.brokerHost + ":" + config.brokerPort + "\r\n" +
            "Upgrade: websocket\r\n" +
            "Connection: Upgrade\r\n" +
            "Sec-WebSocket-Key: " + key + "\r\n" +
            "Sec-WebSocket-Version: 13\r\n" +
            "\r\n";
        output.write(request.getBytes(StandardCharsets.US_ASCII));
        output.flush();
    }

    private void readWebSocketHandshake(InputStream input) throws Exception {
        String statusLine = readHttpLine(input);
        while (true) {
            String line = readHttpLine(input);
            if (line.length() == 0) {
                break;
            }
        }
        if (statusLine == null || !statusLine.contains("101")) {
            throw new IllegalStateException("Broker WebSocket handshake failed: " + statusLine);
        }
    }

    private void publishStatus(String state, String error) {
        try {
            JSONObject status = new JSONObject();
            status.put("schemaVersion", "rusty.xr.projection-target-breath-status.v1");
            status.put("source", "broker-breath");
            status.put("state", state);
            status.put("stream", config.stream);
            status.put("brokerHost", config.brokerHost);
            status.put("brokerPort", config.brokerPort);
            status.put("minScale", config.minScale);
            status.put("maxScale", config.maxScale);
            status.put("smoothingAlpha", config.smoothingAlpha);
            status.put("invert", config.invert);
            status.put("minQuality", config.minQuality);
            if (error != null && error.length() > 0) {
                status.put("error", error);
            }
            sink.onProjectionTargetBreathStatus(status);
        } catch (Exception ignored) {
        }
    }

    private void closeActiveSocket() {
        synchronized (socketLock) {
            closeQuietly(activeSocket);
            activeSocket = null;
        }
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

    private static String readWebSocketTextFrame(InputStream input, int maxFrameBytes) throws Exception {
        int first = input.read();
        if (first < 0) {
            return null;
        }
        int second = input.read();
        if (second < 0) {
            return null;
        }

        int opcode = first & 0x0f;
        boolean masked = (second & 0x80) != 0;
        long length = second & 0x7f;
        if (length == 126) {
            length = readUnsignedShort(input);
        } else if (length == 127) {
            length = readLong(input);
        }
        if (length < 0 || length > maxFrameBytes) {
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
        if (opcode == 8) {
            return null;
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

    private static String randomWebSocketKey() {
        byte[] bytes = new byte[16];
        new Random(System.nanoTime()).nextBytes(bytes);
        return Base64.encodeToString(bytes, Base64.NO_WRAP);
    }

    private static void closeQuietly(Socket socket) {
        if (socket == null) {
            return;
        }
        try {
            socket.close();
        } catch (Exception ignored) {
        }
    }

    private static void sleepQuietly(int millis) {
        try {
            Thread.sleep(millis);
        } catch (InterruptedException ignored) {
            Thread.currentThread().interrupt();
        }
    }

    private static boolean isFinite(double value) {
        return !Double.isNaN(value) && !Double.isInfinite(value);
    }

    private static float clamp(float value, float min, float max) {
        if (Float.isNaN(value) || Float.isInfinite(value)) {
            return min;
        }
        return Math.max(min, Math.min(max, value));
    }

    private static String safeMessage(Exception error) {
        String message = error.getMessage();
        return message != null ? message : error.getClass().getSimpleName();
    }
}
