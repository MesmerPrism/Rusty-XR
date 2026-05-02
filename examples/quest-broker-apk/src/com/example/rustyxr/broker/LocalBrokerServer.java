package com.example.rustyxr.broker;

import android.os.SystemClock;
import android.util.Base64;
import android.util.Log;

import org.json.JSONObject;

import java.io.BufferedInputStream;
import java.io.Closeable;
import java.io.EOFException;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

final class LocalBrokerServer implements Closeable {
    private static final String WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    private static final int MAX_HTTP_LINE_BYTES = 8192;
    private static final long MAX_WEBSOCKET_PAYLOAD_BYTES = 1024 * 1024;

    private final int port;
    private final BrokerState state;
    private final LatencyPublisher publisher;
    private final Set<WebSocketClientConnection> websocketClients = new HashSet<>();
    private volatile OscIngressServer oscIngressServer;
    private volatile boolean running;
    private ServerSocket serverSocket;
    private Thread acceptThread;

    LocalBrokerServer(int port, BrokerState state, LatencyPublisher publisher) {
        this.port = port;
        this.state = state;
        this.publisher = publisher;
    }

    boolean isRunning() {
        return running;
    }

    void setOscIngressServer(OscIngressServer oscIngressServer) {
        this.oscIngressServer = oscIngressServer;
    }

    void start() throws IOException {
        if (running) {
            return;
        }

        serverSocket = new ServerSocket();
        serverSocket.setReuseAddress(true);
        serverSocket.bind(new InetSocketAddress(InetAddress.getByName("127.0.0.1"), port));
        running = true;
        acceptThread = new Thread(new Runnable() {
            @Override
            public void run() {
                acceptLoop();
            }
        }, "RustyXrBrokerAccept");
        acceptThread.start();
    }

    @Override
    public void close() {
        running = false;
        if (serverSocket != null) {
            try {
                serverSocket.close();
            } catch (IOException ignored) {
            }
            serverSocket = null;
        }

        synchronized (websocketClients) {
            for (WebSocketClientConnection client : websocketClients) {
                client.close();
            }
            websocketClients.clear();
        }
    }

    int broadcastText(String text) {
        if (text == null || text.length() == 0) {
            return 0;
        }

        int sent = 0;
        synchronized (websocketClients) {
            WebSocketClientConnection[] snapshot = websocketClients.toArray(new WebSocketClientConnection[0]);
            for (WebSocketClientConnection client : snapshot) {
                if (!client.sendText(text)) {
                    websocketClients.remove(client);
                    continue;
                }

                sent++;
            }
        }
        return sent;
    }

    private void acceptLoop() {
        while (running) {
            try {
                final Socket socket = serverSocket.accept();
                Thread clientThread = new Thread(new Runnable() {
                    @Override
                    public void run() {
                        handleClient(socket);
                    }
                }, "RustyXrBrokerClient");
                clientThread.start();
            } catch (IOException ex) {
                if (running) {
                    Log.w(BrokerService.TAG, "Accept failed: " + ex.getMessage());
                }
            }
        }
    }

    private void handleClient(Socket socket) {
        try {
            socket.setTcpNoDelay(true);
            BufferedInputStream input = new BufferedInputStream(socket.getInputStream());
            OutputStream output = socket.getOutputStream();

            String requestLine = readHttpLine(input);
            if (requestLine == null || requestLine.length() == 0) {
                return;
            }

            Map<String, String> headers = readHeaders(input);
            String[] requestParts = requestLine.split(" ");
            String method = requestParts.length > 0 ? requestParts[0] : "";
            String path = requestParts.length > 1 ? requestParts[1] : "/";

            if ("GET".equals(method) && "/status".equals(path)) {
                state.httpStatusRequests.incrementAndGet();
                writeJsonResponse(output, 200, state.toStatusJson(publisher, oscIngressServer).toString());
                return;
            }

            if ("GET".equals(method) && "/rustyxr/v1/events".equals(path) && wantsWebSocket(headers)) {
                handleWebSocket(headers, input, output);
                return;
            }

            writeJsonResponse(output, 404, "{\"type\":\"error\",\"message\":\"unknown endpoint\"}");
        } catch (Exception ex) {
            Log.w(BrokerService.TAG, "Client failed: " + ex.getClass().getSimpleName() + ": " + ex.getMessage());
        } finally {
            try {
                socket.close();
            } catch (IOException ignored) {
            }
        }
    }

    private void handleWebSocket(Map<String, String> headers, BufferedInputStream input, OutputStream output) throws Exception {
        String key = headers.get("sec-websocket-key");
        if (key == null || key.length() == 0) {
            writeJsonResponse(output, 400, "{\"type\":\"error\",\"message\":\"missing Sec-WebSocket-Key\"}");
            return;
        }

        String accept = websocketAccept(key);
        String response =
            "HTTP/1.1 101 Switching Protocols\r\n" +
            "Upgrade: websocket\r\n" +
            "Connection: Upgrade\r\n" +
            "Sec-WebSocket-Accept: " + accept + "\r\n" +
            "\r\n";
        output.write(response.getBytes(StandardCharsets.US_ASCII));
        output.flush();

        state.websocketConnections.incrementAndGet();
        WebSocketClientConnection connection = new WebSocketClientConnection(output);
        synchronized (websocketClients) {
            websocketClients.add(connection);
        }
        connection.sendText(state.toStatusJson(publisher, oscIngressServer).toString());
        Log.i(BrokerService.TAG, "WebSocket client connected");

        try {
            while (running) {
                WebSocketFrame frame = readFrame(input);
                if (frame == null || frame.opcode == 8) {
                    break;
                }

                if (frame.opcode == 9) {
                    connection.sendPong(frame.payload);
                    continue;
                }

                if (frame.opcode != 1) {
                    continue;
                }

                String text = new String(frame.payload, StandardCharsets.UTF_8);
                JSONObject reply = handleClientMessage(text);
                if (reply != null) {
                    connection.sendText(reply.toString());
                }
            }
        } finally {
            synchronized (websocketClients) {
                websocketClients.remove(connection);
            }
        }

        Log.i(BrokerService.TAG, "WebSocket client disconnected");
    }

    private JSONObject handleClientMessage(String text) {
        try {
            JSONObject message = new JSONObject(text);
            String type = message.optString("type", "");
            if ("hello".equals(type)) {
                return state.toStatusJson(publisher, oscIngressServer);
            }

            if ("latency_sample".equals(type)) {
                return acceptLatencySample(message, text);
            }

            state.rejectedMessages.incrementAndGet();
            JSONObject error = new JSONObject();
            error.put("type", "error");
            error.put("message", "unsupported message type");
            error.put("receivedType", type);
            return error;
        } catch (Exception ex) {
            state.rejectedMessages.incrementAndGet();
            JSONObject error = new JSONObject();
            try {
                error.put("type", "error");
                error.put("message", "invalid json: " + ex.getMessage());
            } catch (Exception ignored) {
            }
            return error;
        }
    }

    private JSONObject acceptLatencySample(JSONObject message, String originalText) throws Exception {
        long receiveUnixNs = unixNowNs();
        long receiveElapsedNs = SystemClock.elapsedRealtimeNanos();
        long sequence = message.optLong("sequence_id", -1L);
        String path = message.optString("path", "broker_lsl");
        if (path.length() == 0) {
            path = "broker_lsl";
        }

        message.put("type", "latency_sample");
        message.put("path", path);
        message.put("broker_receive_time_unix_ns", receiveUnixNs);
        message.put("broker_receive_time_elapsed_ns", receiveElapsedNs);
        if (!message.has("payload_size_bytes")) {
            message.put("payload_size_bytes", originalText.getBytes(StandardCharsets.UTF_8).length);
        }

        long publishUnixNs = unixNowNs();
        long publishElapsedNs = SystemClock.elapsedRealtimeNanos();
        message.put("broker_publish_time_unix_ns", publishUnixNs);
        message.put("broker_publish_time_elapsed_ns", publishElapsedNs);
        message.put("lsl_forwarded", publisher != null && publisher.isLslAvailable());
        message.put("osc_forwarded", publisher != null && publisher.isOscAvailable());
        message.put("fallback_transport", publisher != null ? publisher.mode() : "none");

        if (publisher != null) {
            publisher.publish(message);
        }

        long accepted = state.acceptedLatencySamples.incrementAndGet();
        JSONObject ack = new JSONObject();
        ack.put("type", "latency_ack");
        ack.put("sequence_id", sequence);
        ack.put("path", path);
        ack.put("accepted_count", accepted);
        ack.put("broker_receive_time_unix_ns", receiveUnixNs);
        ack.put("broker_publish_time_unix_ns", publishUnixNs);
        ack.put("lsl_forwarded", publisher != null && publisher.isLslAvailable());
        ack.put("osc_forwarded", publisher != null && publisher.isOscAvailable());
        ack.put("fallback_transport", publisher != null ? publisher.mode() : "none");
        return ack;
    }

    private static boolean wantsWebSocket(Map<String, String> headers) {
        String upgrade = headers.get("upgrade");
        String connection = headers.get("connection");
        return upgrade != null &&
            "websocket".equals(upgrade.toLowerCase(Locale.ROOT)) &&
            connection != null &&
            connection.toLowerCase(Locale.ROOT).contains("upgrade");
    }

    private static String websocketAccept(String key) throws Exception {
        MessageDigest sha1 = MessageDigest.getInstance("SHA-1");
        byte[] digest = sha1.digest((key.trim() + WS_GUID).getBytes(StandardCharsets.US_ASCII));
        return Base64.encodeToString(digest, Base64.NO_WRAP);
    }

    private static String readHttpLine(BufferedInputStream input) throws IOException {
        byte[] buffer = new byte[MAX_HTTP_LINE_BYTES];
        int count = 0;
        int previous = -1;
        while (count < buffer.length) {
            int next = input.read();
            if (next < 0) {
                if (count == 0) {
                    return null;
                }
                break;
            }

            if (previous == '\r' && next == '\n') {
                count -= 1;
                break;
            }

            buffer[count++] = (byte) next;
            previous = next;
        }

        return new String(buffer, 0, count, StandardCharsets.US_ASCII);
    }

    private static Map<String, String> readHeaders(BufferedInputStream input) throws IOException {
        Map<String, String> headers = new HashMap<>();
        while (true) {
            String line = readHttpLine(input);
            if (line == null || line.length() == 0) {
                break;
            }

            int colon = line.indexOf(':');
            if (colon <= 0) {
                continue;
            }

            String name = line.substring(0, colon).trim().toLowerCase(Locale.ROOT);
            String value = line.substring(colon + 1).trim();
            headers.put(name, value);
        }
        return headers;
    }

    private static void writeJsonResponse(OutputStream output, int status, String body) throws IOException {
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        String statusText = status == 200 ? "OK" : status == 400 ? "Bad Request" : "Not Found";
        String response =
            "HTTP/1.1 " + status + " " + statusText + "\r\n" +
            "Content-Type: application/json; charset=utf-8\r\n" +
            "Content-Length: " + bytes.length + "\r\n" +
            "Connection: close\r\n" +
            "\r\n";
        output.write(response.getBytes(StandardCharsets.US_ASCII));
        output.write(bytes);
        output.flush();
    }

    private static WebSocketFrame readFrame(BufferedInputStream input) throws IOException {
        int b0 = input.read();
        if (b0 < 0) {
            return null;
        }

        int b1 = input.read();
        if (b1 < 0) {
            throw new EOFException("truncated websocket frame");
        }

        int opcode = b0 & 0x0F;
        boolean masked = (b1 & 0x80) != 0;
        long length = b1 & 0x7F;
        if (length == 126) {
            length = readUnsignedShort(input);
        } else if (length == 127) {
            length = readLong(input);
        }

        if (length > MAX_WEBSOCKET_PAYLOAD_BYTES) {
            throw new IOException("websocket payload too large: " + length);
        }

        byte[] mask = null;
        if (masked) {
            mask = readExact(input, 4);
        }

        byte[] payload = readExact(input, (int) length);
        if (masked && mask != null) {
            for (int i = 0; i < payload.length; i++) {
                payload[i] = (byte) (payload[i] ^ mask[i % 4]);
            }
        }

        return new WebSocketFrame(opcode, payload);
    }

    private static int readUnsignedShort(BufferedInputStream input) throws IOException {
        int high = input.read();
        int low = input.read();
        if (high < 0 || low < 0) {
            throw new EOFException("truncated unsigned short");
        }
        return ((high & 0xFF) << 8) | (low & 0xFF);
    }

    private static long readLong(BufferedInputStream input) throws IOException {
        long value = 0L;
        for (int i = 0; i < 8; i++) {
            int next = input.read();
            if (next < 0) {
                throw new EOFException("truncated long");
            }
            value = (value << 8) | (next & 0xFFL);
        }
        return value;
    }

    private static byte[] readExact(BufferedInputStream input, int length) throws IOException {
        byte[] buffer = new byte[length];
        int offset = 0;
        while (offset < length) {
            int read = input.read(buffer, offset, length - offset);
            if (read < 0) {
                throw new EOFException("truncated payload");
            }
            offset += read;
        }
        return buffer;
    }

    private static void sendTextFrame(OutputStream output, String text) throws IOException {
        sendFrame(output, 1, text.getBytes(StandardCharsets.UTF_8));
    }

    private static void sendPong(OutputStream output, byte[] payload) throws IOException {
        sendFrame(output, 10, payload);
    }

    private static void sendFrame(OutputStream output, int opcode, byte[] payload) throws IOException {
        int length = payload != null ? payload.length : 0;
        output.write(0x80 | (opcode & 0x0F));
        if (length < 126) {
            output.write(length);
        } else if (length <= 65535) {
            output.write(126);
            output.write((length >> 8) & 0xFF);
            output.write(length & 0xFF);
        } else {
            output.write(127);
            long value = length;
            for (int i = 7; i >= 0; i--) {
                output.write((int) ((value >> (i * 8)) & 0xFF));
            }
        }

        if (length > 0) {
            output.write(payload);
        }
        output.flush();
    }

    private static long unixNowNs() {
        return System.currentTimeMillis() * 1_000_000L;
    }

    private static final class WebSocketFrame {
        final int opcode;
        final byte[] payload;

        WebSocketFrame(int opcode, byte[] payload) {
            this.opcode = opcode;
            this.payload = payload != null ? payload : new byte[0];
        }
    }

    private static final class WebSocketClientConnection {
        private final OutputStream output;
        private boolean closed;

        WebSocketClientConnection(OutputStream output) {
            this.output = output;
        }

        synchronized boolean sendText(String text) {
            if (closed) {
                return false;
            }

            try {
                sendTextFrame(output, text);
                return true;
            } catch (IOException ex) {
                closed = true;
                return false;
            }
        }

        synchronized boolean sendPong(byte[] payload) {
            if (closed) {
                return false;
            }

            try {
                LocalBrokerServer.sendPong(output, payload);
                return true;
            } catch (IOException ex) {
                closed = true;
                return false;
            }
        }

        synchronized void close() {
            closed = true;
        }
    }
}
