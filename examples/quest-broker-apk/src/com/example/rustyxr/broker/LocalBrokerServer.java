package com.example.rustyxr.broker;

import android.content.Context;
import android.content.Intent;
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
import java.util.concurrent.atomic.AtomicLong;

final class LocalBrokerServer implements Closeable {
    private static final String WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    private static final int MAX_HTTP_LINE_BYTES = 8192;
    private static final long MAX_WEBSOCKET_PAYLOAD_BYTES = 1024 * 1024;

    private final int port;
    private final BrokerState state;
    private final LatencyPublisher publisher;
    private final Context context;
    private final Set<WebSocketClientConnection> websocketClients = new HashSet<>();
    private final AtomicLong nextConnectionId = new AtomicLong(1L);
    private volatile OscIngressServer oscIngressServer;
    private volatile boolean running;
    private ServerSocket serverSocket;
    private Thread acceptThread;

    LocalBrokerServer(int port, BrokerState state, LatencyPublisher publisher, Context context) {
        this.port = port;
        this.state = state;
        this.publisher = publisher;
        this.context = context != null ? context.getApplicationContext() : null;
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

        if (oscIngressServer != null) {
            oscIngressServer.close();
            oscIngressServer = null;
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

    int broadcastStreamEvent(String stream, long sequenceId, long receiveUnixNs, JSONObject payload) {
        if (stream == null || stream.length() == 0 || payload == null) {
            return 0;
        }

        int sent = 0;
        synchronized (websocketClients) {
            WebSocketClientConnection[] snapshot = websocketClients.toArray(new WebSocketClientConnection[0]);
            for (WebSocketClientConnection client : snapshot) {
                if (!client.isSubscribedTo(stream)) {
                    continue;
                }

                try {
                    JSONObject event = new JSONObject();
                    event.put("type", "stream_event");
                    event.put("schema", "rusty.xr.broker.stream_event.v1");
                    event.put("stream", stream);
                    event.put("subscription_id", client.subscriptionIdFor(stream));
                    event.put("sequence_id", sequenceId);
                    event.put("broker_time_unix_ns", receiveUnixNs);
                    event.put("broker_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
                    event.put("payload", payload);
                    if (!client.sendText(event.toString())) {
                        websocketClients.remove(client);
                        continue;
                    }

                    sent++;
                } catch (Exception ex) {
                    Log.w(BrokerService.TAG, "Stream event build failed: " + ex.getMessage());
                }
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
        WebSocketClientConnection connection = new WebSocketClientConnection(nextConnectionId.getAndIncrement(), output);
        synchronized (websocketClients) {
            websocketClients.add(connection);
        }
        connection.sendText(statusForConnection(connection).toString());
        Log.i(BrokerService.TAG, "WebSocket client connected id=" + connection.connectionId);

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
                JSONObject reply = handleClientMessage(connection, text);
                if (reply != null) {
                    connection.sendText(reply.toString());
                }
            }
        } finally {
            synchronized (websocketClients) {
                websocketClients.remove(connection);
            }
        }

        Log.i(BrokerService.TAG, "WebSocket client disconnected id=" + connection.connectionId);
    }

    private JSONObject handleClientMessage(WebSocketClientConnection connection, String text) {
        try {
            JSONObject message = new JSONObject(text);
            String type = message.optString("type", "");
            if ("hello".equals(type)) {
                updateClientIdentity(connection, message);
                return statusForConnection(connection);
            }

            if ("status_request".equals(type)) {
                state.acceptedCommands.incrementAndGet();
                return statusForConnection(connection);
            }

            if ("command".equals(type)) {
                return handleCommand(connection, message);
            }

            if ("subscribe".equals(type) || "unsubscribe".equals(type)) {
                return handleLegacySubscriptionCommand(connection, type, message);
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

    private JSONObject handleCommand(WebSocketClientConnection connection, JSONObject message) throws Exception {
        String command = message.optString("command", "");
        String requestId = message.optString("request_id", "");
        updateClientIdentity(connection, message);

        if ("status_request".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            return commandAck(requestId, command, true, "status", statusForConnection(connection));
        }

        if ("list_capabilities".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("capabilities", state.capabilitiesJson(publisher, oscIngressServer));
            return commandAck(requestId, command, true, "capabilities", result);
        }

        if ("list_streams".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("streams", state.streamsJson(oscIngressServer));
            return commandAck(requestId, command, true, "streams", result);
        }

        if ("subscribe".equals(command)) {
            JSONObject params = message.optJSONObject("params");
            String stream = params != null ? params.optString("stream", "") : message.optString("stream", "");
            return subscribe(connection, requestId, command, stream);
        }

        if ("unsubscribe".equals(command)) {
            JSONObject params = message.optJSONObject("params");
            String stream = params != null ? params.optString("stream", "") : message.optString("stream", "");
            return unsubscribe(connection, requestId, command, stream);
        }

        if ("configure_osc_ingress".equals(command)) {
            return configureOscIngress(requestId, command, message.optJSONObject("params"));
        }

        if ("publish_stream_event".equals(command)) {
            return publishStreamEvent(connection, requestId, command, message.optJSONObject("params"));
        }

        if ("open_ui".equals(command) ||
            "broker_console_open".equals(command) ||
            "ui.open".equals(command)) {
            return openBrokerConsole(connection, requestId, command);
        }

        if ("close_ui".equals(command) ||
            "broker_console_close".equals(command) ||
            "ui.close".equals(command)) {
            return closeBrokerConsole(connection, requestId, command);
        }

        state.rejectedCommands.incrementAndGet();
        return commandError(requestId, command, "unsupported_command", "Unknown command: " + command);
    }

    private JSONObject handleLegacySubscriptionCommand(
        WebSocketClientConnection connection,
        String type,
        JSONObject message) throws Exception {
        String stream = message.optString("stream", "");
        String requestId = message.optString("request_id", "");
        if ("subscribe".equals(type)) {
            return subscribe(connection, requestId, type, stream);
        }

        return unsubscribe(connection, requestId, type, stream);
    }

    private synchronized JSONObject configureOscIngress(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        boolean enabled = params == null || params.optBoolean("enabled", true);
        int requestedPort = params != null
            ? params.optInt("port", BrokerRuntimeConfig.DEFAULT_OSC_PORT)
            : BrokerRuntimeConfig.DEFAULT_OSC_PORT;
        String address = normalizeOscAddress(params != null
            ? params.optString("address", BrokerRuntimeConfig.DEFAULT_OSC_INGRESS_ADDRESS)
            : BrokerRuntimeConfig.DEFAULT_OSC_INGRESS_ADDRESS);

        if (requestedPort <= 0 || requestedPort > 65535) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "invalid_port", "OSC ingress port must be between 1 and 65535.");
        }

        if (!address.startsWith("/")) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "invalid_address", "OSC ingress address must start with '/'.");
        }

        if (enabled &&
            oscIngressServer != null &&
            oscIngressServer.isRunning() &&
            oscIngressServer.port() == requestedPort &&
            address.equals(oscIngressServer.acceptedAddress())) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("enabled", true);
            result.put("port", requestedPort);
            result.put("address", address);
            result.put("stream", oscIngressServer.streamId());
            result.put("status", oscIngressServer.toStatusJson());
            return commandAck(requestId, command, true, "osc_ingress_already_configured", result);
        }

        if (oscIngressServer != null) {
            oscIngressServer.close();
            oscIngressServer = null;
        }

        if (enabled) {
            BrokerRuntimeConfig config = BrokerRuntimeConfig.oscIngressConfig(true, requestedPort, address);
            OscIngressServer next = OscIngressServer.createOrNull(config, state, this);
            if (next == null) {
                state.rejectedCommands.incrementAndGet();
                return commandError(requestId, command, "create_failed", "OSC ingress server could not be created.");
            }

            try {
                next.start();
            } catch (Exception ex) {
                next.close();
                state.rejectedCommands.incrementAndGet();
                return commandError(
                    requestId,
                    command,
                    "start_failed",
                    ex.getClass().getSimpleName() + ": " + ex.getMessage());
            }

            oscIngressServer = next;
        }

        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("enabled", oscIngressServer != null && oscIngressServer.isRunning());
        result.put("port", requestedPort);
        result.put("address", address);
        result.put("stream", "osc:" + address);
        result.put("status", oscIngressServer != null
            ? oscIngressServer.toStatusJson()
            : new JSONObject().put("enabled", false));
        Log.i(BrokerService.TAG, "OSC ingress runtime config enabled=" +
            (oscIngressServer != null && oscIngressServer.isRunning()) +
            " port=" + requestedPort + " address=" + address);
        return commandAck(requestId, command, true, "osc_ingress_configured", result);
    }

    private JSONObject publishStreamEvent(
        WebSocketClientConnection connection,
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires params.");
        }

        String stream = params.optString("stream", "");
        if (stream.trim().length() == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_stream", "Command requires params.stream.");
        }

        JSONObject payload = params.optJSONObject("payload");
        if (payload == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_payload", "Command requires params.payload.");
        }

        long sequence = params.optLong("sequence_id", state.publishedStreamEvents.get() + 1L);
        long receiveUnixNs = unixNowNs();
        payload.put("publisher_client_id", connection != null ? connection.clientId : "");
        payload.put("broker_receive_time_unix_ns", receiveUnixNs);
        int broadcasts = broadcastStreamEvent(stream, sequence, receiveUnixNs, payload);
        long accepted = state.publishedStreamEvents.incrementAndGet();
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("stream", stream);
        result.put("sequence_id", sequence);
        result.put("published_count", accepted);
        result.put("broadcasts", broadcasts);
        return commandAck(requestId, command, true, "stream_event_published", result);
    }

    private JSONObject subscribe(
        WebSocketClientConnection connection,
        String requestId,
        String command,
        String stream) throws Exception {
        if (stream == null || stream.length() == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_stream", "Command requires a stream.");
        }

        connection.subscribe(stream);
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("stream", stream);
        result.put("subscription_id", connection.subscriptionIdFor(stream));
        result.put("subscriptions", connection.subscriptionsJson());
        return commandAck(requestId, command, true, "subscribed", result);
    }

    private JSONObject unsubscribe(
        WebSocketClientConnection connection,
        String requestId,
        String command,
        String stream) throws Exception {
        if (stream == null || stream.length() == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_stream", "Command requires a stream.");
        }

        connection.unsubscribe(stream);
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("stream", stream);
        result.put("subscriptions", connection.subscriptionsJson());
        return commandAck(requestId, command, true, "unsubscribed", result);
    }

    private JSONObject openBrokerConsole(
        WebSocketClientConnection connection,
        String requestId,
        String command) throws Exception {
        if (context == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_context", "Broker context is not available.");
        }

        Intent intent = new Intent(context, MainActivity.class);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        intent.addFlags(Intent.FLAG_ACTIVITY_REORDER_TO_FRONT);
        intent.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP);
        intent.putExtra("rustyxr.openedByBrokerCommand", true);
        intent.putExtra("rustyxr.requestId", requestId != null ? requestId : "");
        intent.putExtra("rustyxr.clientId", connection != null ? connection.clientId : "");
        intent.putExtra("rustyxr.appPackage", connection != null ? connection.appPackage : "");
        context.startActivity(intent);

        state.acceptedCommands.incrementAndGet();
        long requests = state.brokerConsoleOpenRequests.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("activity", "broker_console");
        result.put("open_requests", requests);
        result.put("return_command", "Use the console Return to XR App button.");
        Log.i(BrokerService.TAG, "Broker console opened by command from client=" +
            (connection != null ? connection.clientId : ""));
        return commandAck(requestId, command, true, "broker_console_opened", result);
    }

    private JSONObject closeBrokerConsole(
        WebSocketClientConnection connection,
        String requestId,
        String command) throws Exception {
        String clientId = connection != null ? connection.clientId : "";
        boolean closeRequested = MainActivity.requestCloseFromBrokerCommand("command:" + command + " client=" + clientId);

        state.acceptedCommands.incrementAndGet();
        long requests = state.brokerConsoleCloseRequests.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("activity", "broker_console");
        result.put("close_requested", closeRequested);
        result.put("close_requests", requests);
        Log.i(BrokerService.TAG, "Broker console close command from client=" + clientId +
            " requested=" + closeRequested);
        return commandAck(
            requestId,
            command,
            true,
            closeRequested ? "broker_console_close_requested" : "broker_console_not_open",
            result);
    }

    private JSONObject commandAck(
        String requestId,
        String command,
        boolean accepted,
        String message,
        JSONObject result) throws Exception {
        JSONObject ack = new JSONObject();
        ack.put("type", "command_ack");
        ack.put("schema", "rusty.xr.broker.command_ack.v1");
        ack.put("request_id", requestId != null ? requestId : "");
        ack.put("command", command != null ? command : "");
        ack.put("accepted", accepted);
        ack.put("message", message != null ? message : "");
        if (result != null) {
            ack.put("result", result);
        }
        return ack;
    }

    private JSONObject commandError(
        String requestId,
        String command,
        String code,
        String message) throws Exception {
        JSONObject error = new JSONObject();
        error.put("code", code);
        error.put("message", message);

        JSONObject ack = new JSONObject();
        ack.put("type", "command_ack");
        ack.put("schema", "rusty.xr.broker.command_ack.v1");
        ack.put("request_id", requestId != null ? requestId : "");
        ack.put("command", command != null ? command : "");
        ack.put("accepted", false);
        ack.put("error", error);
        return ack;
    }

    private static String normalizeOscAddress(String address) {
        if (address == null || address.trim().length() == 0) {
            return BrokerRuntimeConfig.DEFAULT_OSC_INGRESS_ADDRESS;
        }

        return address.trim();
    }

    private JSONObject statusForConnection(WebSocketClientConnection connection) throws Exception {
        JSONObject status = state.toStatusJson(publisher, oscIngressServer);
        if (connection != null) {
            JSONObject client = new JSONObject();
            client.put("connection_id", connection.connectionId);
            client.put("client_id", connection.clientId);
            client.put("app_package", connection.appPackage);
            client.put("app_label", connection.appLabel);
            client.put("app_version", connection.appVersion);
            client.put("subscriptions", connection.subscriptionsJson());
            status.put("client", client);
        }

        synchronized (websocketClients) {
            status.put("activeWebSocketClients", websocketClients.size());
        }
        return status;
    }

    private static void updateClientIdentity(WebSocketClientConnection connection, JSONObject message) {
        if (connection == null || message == null) {
            return;
        }

        String clientId = message.optString("client_id", message.optString("clientId", ""));
        if (clientId.length() > 0) {
            connection.clientId = clientId;
        }

        String appPackage = message.optString("app_package", message.optString("appPackage", ""));
        if (appPackage.length() > 0) {
            connection.appPackage = appPackage;
        }

        String appLabel = message.optString("app_label", message.optString("appLabel", ""));
        if (appLabel.length() > 0) {
            connection.appLabel = appLabel;
        }

        String appVersion = message.optString("app_version", message.optString("appVersion", ""));
        if (appVersion.length() > 0) {
            connection.appVersion = appVersion;
        }

        connection.lastSeenElapsedNs = SystemClock.elapsedRealtimeNanos();
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

        JSONObject payload = new JSONObject();
        payload.put("path", path);
        payload.put("payload_size_bytes", message.optLong("payload_size_bytes", 0L));
        payload.put("lsl_forwarded", publisher != null && publisher.isLslAvailable());
        payload.put("osc_forwarded", publisher != null && publisher.isOscAvailable());
        payload.put("fallback_transport", publisher != null ? publisher.mode() : "none");
        int streamBroadcasts = broadcastStreamEvent("latency:sample", sequence, receiveUnixNs, payload);

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
        ack.put("stream_event_broadcasts", streamBroadcasts);
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
        final long connectionId;
        private final OutputStream output;
        private final Set<String> subscriptions = new HashSet<>();
        volatile String clientId = "";
        volatile String appPackage = "";
        volatile String appLabel = "";
        volatile String appVersion = "";
        volatile long lastSeenElapsedNs;
        private boolean closed;

        WebSocketClientConnection(long connectionId, OutputStream output) {
            this.connectionId = connectionId;
            this.output = output;
            this.lastSeenElapsedNs = SystemClock.elapsedRealtimeNanos();
        }

        synchronized void subscribe(String stream) {
            if (stream != null && stream.length() > 0) {
                subscriptions.add(stream);
            }
        }

        synchronized void unsubscribe(String stream) {
            if (stream != null && stream.length() > 0) {
                subscriptions.remove(stream);
            }
        }

        synchronized boolean isSubscribedTo(String stream) {
            return subscriptions.contains(stream);
        }

        synchronized String subscriptionIdFor(String stream) {
            int hash = stream != null ? stream.hashCode() : 0;
            return "conn-" + connectionId + "-" + Integer.toHexString(hash);
        }

        synchronized org.json.JSONArray subscriptionsJson() {
            org.json.JSONArray values = new org.json.JSONArray();
            for (String subscription : subscriptions) {
                values.put(subscription);
            }
            return values;
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
