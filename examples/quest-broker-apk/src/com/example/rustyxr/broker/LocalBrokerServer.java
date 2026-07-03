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
    private final String bindHost;
    private final Set<WebSocketClientConnection> websocketClients = new HashSet<>();
    private final AtomicLong nextConnectionId = new AtomicLong(1L);
    private volatile OscIngressServer oscIngressServer;
    private volatile boolean running;
    private ServerSocket serverSocket;
    private Thread acceptThread;
    private volatile PolarPmdBrokerSource polarPmdSource;
    private volatile PolarHeartRateBrokerSource polarHeartRateSource;
    private volatile DeviceWatchdog deviceWatchdog;

    LocalBrokerServer(int port, BrokerState state, LatencyPublisher publisher, Context context) {
        this(port, state, publisher, context, "127.0.0.1");
    }

    LocalBrokerServer(int port, BrokerState state, LatencyPublisher publisher, Context context, String bindHost) {
        this.port = port;
        this.state = state;
        this.publisher = publisher;
        this.context = context != null ? context.getApplicationContext() : null;
        this.bindHost = bindHost != null && bindHost.trim().length() > 0
            ? bindHost.trim()
            : "127.0.0.1";
    }

    boolean isRunning() {
        return running;
    }

    String bindHost() {
        return bindHost;
    }

    void setOscIngressServer(OscIngressServer oscIngressServer) {
        this.oscIngressServer = oscIngressServer;
    }

    void setPolarPmdSource(PolarPmdBrokerSource polarPmdSource) {
        this.polarPmdSource = polarPmdSource;
    }

    void setPolarHeartRateSource(PolarHeartRateBrokerSource polarHeartRateSource) {
        this.polarHeartRateSource = polarHeartRateSource;
    }

    void setDeviceWatchdog(DeviceWatchdog deviceWatchdog) {
        this.deviceWatchdog = deviceWatchdog;
    }

    void start() throws IOException {
        if (running) {
            return;
        }

        serverSocket = new ServerSocket();
        serverSocket.setReuseAddress(true);
        serverSocket.bind(new InetSocketAddress(InetAddress.getByName(bindHost), port));
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

        publishBioStreamEventToLsl(stream, sequenceId, receiveUnixNs, payload);

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
                    event.put("schema", BrokerState.MANIFOLD_STREAM_EVENT_SCHEMA);
                    event.put("legacy_schema", BrokerState.LEGACY_RUSTY_XR_BROKER_STREAM_EVENT_SCHEMA);
                    event.put("stream", stream);
                    event.put("subscription_id", client.subscriptionIdFor(stream));
                    event.put("sequence_id", sequenceId);
                    JSONObject clockStamp = state.clockStampJson();
                    event.put("broker_time_unix_ns", clockStamp.optLong("event_unix_ns", receiveUnixNs));
                    event.put(
                        "broker_time_elapsed_ns",
                        clockStamp.optLong("event_elapsed_realtime_ns", SystemClock.elapsedRealtimeNanos()));
                    event.put("clock_stamp", clockStamp);
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

    private void publishBioStreamEventToLsl(String stream, long sequenceId, long receiveUnixNs, JSONObject payload) {
        if (publisher == null || !publisher.isLslAvailable() || !shouldMirrorStreamEventToLsl(stream)) {
            return;
        }

        try {
            JSONObject event = new JSONObject();
            event.put("type", "stream_event");
            event.put("schema", BrokerState.MANIFOLD_STREAM_EVENT_SCHEMA);
            event.put("legacy_schema", BrokerState.LEGACY_RUSTY_XR_BROKER_STREAM_EVENT_SCHEMA);
            event.put("stream", stream);
            event.put("subscription_id", "lsl:broker");
            event.put("sequence_id", sequenceId);
            JSONObject clockStamp = state.clockStampJson();
            event.put("broker_time_unix_ns", clockStamp.optLong("event_unix_ns", receiveUnixNs));
            event.put(
                "broker_time_elapsed_ns",
                clockStamp.optLong("event_elapsed_realtime_ns", SystemClock.elapsedRealtimeNanos()));
            event.put("clock_stamp", clockStamp);
            event.put("lsl_mirror", true);
            event.put("payload", payload);
            publisher.publish(event);
        } catch (Exception ex) {
            Log.w(BrokerService.TAG, "LSL stream event mirror failed: " + ex.getMessage());
        }
    }

    private static boolean shouldMirrorStreamEventToLsl(String stream) {
        return stream != null && stream.startsWith("bio:");
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
            String endpointPath = path;
            int queryIndex = endpointPath.indexOf('?');
            if (queryIndex >= 0) {
                endpointPath = endpointPath.substring(0, queryIndex);
            }

            if ("GET".equals(method) && "/status".equals(endpointPath)) {
                state.httpStatusRequests.incrementAndGet();
                writeJsonResponse(output, 200, statusForConnection(null).toString());
                return;
            }

            if ("GET".equals(method) && "/clock/status".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.clockStatusJson().toString());
                return;
            }

            if ("GET".equals(method) && "/clock/now".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.clockSnapshotJson().toString());
                return;
            }

            if ("GET".equals(method) && "/clock/domains".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.clockDomainsJson().toString());
                return;
            }

            if ("GET".equals(method) && "/clock/correlations".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.clockCorrelationsJson().toString());
                return;
            }

            if ("GET".equals(method) && "/clock/health".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.clockHealthJson().toString());
                return;
            }

            if ("GET".equals(method) && "/clock/compare/openxr".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.clockOpenXrComparisonJson().toString());
                return;
            }

            if ("GET".equals(method) && "/clock/sync_probe".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.clockSyncProbeJson(new JSONObject()).toString());
                return;
            }

            if ("GET".equals(method) && "/kiosk/status".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.rustyKioskStatusJson().toString());
                return;
            }

            if ("GET".equals(method) && "/stream_registry/snapshot".equals(endpointPath)) {
                writeJsonResponse(output, 200, state.streamRegistrySnapshotJson(oscIngressServer).toString());
                return;
            }

            if ("GET".equals(method) && BrokerState.HOST_MANIFEST_HTTP_PATH.equals(endpointPath)) {
                writeJsonResponse(output, 200, state.hostManifestJson(bindHost, port, publisher, oscIngressServer).toString());
                return;
            }

            if ("GET".equals(method) && isEventsWebSocketPath(endpointPath) && wantsWebSocket(headers)) {
                handleWebSocket(headers, input, output, path);
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

    private void handleWebSocket(Map<String, String> headers, BufferedInputStream input, OutputStream output, String path) throws Exception {
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
        String startupMode = webSocketStartupMode(headers, path);
        if (!"none".equals(startupMode)) {
            if ("compact".equals(startupMode)) {
                connection.sendText(webSocketReadyJson(connection, path).toString());
            } else {
                connection.sendText(statusForConnection(connection).toString());
            }
        }
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

        if ("stream_registry.snapshot".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("registry", state.streamRegistrySnapshotJson(oscIngressServer));
            return commandAck(requestId, command, true, "stream_registry_snapshot", result);
        }

        if (BrokerState.HOST_MANIFEST_COMMAND.equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("host_manifest", state.hostManifestJson(bindHost, port, publisher, oscIngressServer));
            return commandAck(requestId, command, true, "host_manifest", result);
        }

        if (BrokerState.CONTROL_LEASE_REQUEST_COMMAND.equals(command)) {
            try {
                JSONObject result = state.requestControlLease(
                    message.optJSONObject("params"),
                    connection != null ? connection.clientId : message.optString("client_id", ""));
                state.acceptedCommands.incrementAndGet();
                return commandAck(
                    requestId,
                    command,
                    true,
                    result.optString("outcome", "control_lease_granted"),
                    result);
            } catch (BrokerState.CommandRejection ex) {
                state.rejectedCommands.incrementAndGet();
                return commandError(requestId, command, ex);
            }
        }

        if (BrokerState.CONTROL_LEASE_RELEASE_COMMAND.equals(command)) {
            try {
                JSONObject result = state.releaseControlLease(
                    message.optJSONObject("params"),
                    connection != null ? connection.clientId : message.optString("client_id", ""));
                state.acceptedCommands.incrementAndGet();
                return commandAck(
                    requestId,
                    command,
                    true,
                    result.optString("outcome", "control_lease_released"),
                    result);
            } catch (BrokerState.CommandRejection ex) {
                state.rejectedCommands.incrementAndGet();
                return commandError(requestId, command, ex);
            }
        }

        if ("clock.status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("clock", state.clockStatusJson());
            return commandAck(requestId, command, true, "clock_status", result);
        }

        if ("clock.now".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("snapshot", state.clockSnapshotJson());
            return commandAck(requestId, command, true, "clock_now", result);
        }

        if ("clock.domains".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("domains", state.clockDomainsJson());
            return commandAck(requestId, command, true, "clock_domains", result);
        }

        if ("clock.correlations".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("correlations", state.clockCorrelationsJson());
            return commandAck(requestId, command, true, "clock_correlations", result);
        }

        if ("clock.health".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("health", state.clockHealthJson());
            return commandAck(requestId, command, true, "clock_health", result);
        }

        if ("clock.compare_openxr".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("openxr_comparison", state.clockOpenXrComparisonJson());
            return commandAck(requestId, command, true, "clock_openxr_comparison", result);
        }

        if ("clock.sync_probe".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("probe", state.clockSyncProbeJson(message.optJSONObject("params")));
            return commandAck(requestId, command, true, "clock_sync_probe", result);
        }

        if ("lsl.capture_string".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("capture", NativeLslStringInletDiagnostics.capture(message.optJSONObject("params")));
            return commandAck(requestId, command, true, "lsl_string_capture", result);
        }

        if ("kiosk.get_status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject kioskStatus = state.rustyKioskStatusJson();
            JSONObject result = new JSONObject();
            result.put("status", kioskStatus);
            result.put(
                "command_run_record",
                state.rustyKioskCommandRunRecordJson(
                    requestId != null && requestId.length() > 0 ? requestId : "broker-ws-kiosk-get-status",
                    "websocket kiosk.get_status",
                    JSONObject.NULL,
                    kioskStatus,
                    "broker_websocket_kiosk_status"));
            return commandAck(requestId, command, true, "kiosk_status", result);
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

        if ("breath_feedback.received".equals(command)) {
            return recordBreathFeedbackReceipt(connection, requestId, command, message.optJSONObject("params"));
        }

        if ("polar.get_status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("heart_rate", state.polarHeartRateStatusJson());
            result.put("pmd", state.polarPmdStatusJson());
            return commandAck(requestId, command, true, "polar_status", result);
        }

        if ("polar.start".equals(command)) {
            return startPolar(requestId, command, message.optJSONObject("params"));
        }

        if ("polar.stop".equals(command)) {
            return stopPolar(requestId, command, message.optJSONObject("params"));
        }

        if ("polar_hr.get_status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("status", state.polarHeartRateStatusJson());
            return commandAck(requestId, command, true, "polar_hr_status", result);
        }

        if ("polar_hr.start".equals(command)) {
            return startPolarHeartRate(requestId, command, message.optJSONObject("params"));
        }

        if ("polar_hr.stop".equals(command)) {
            return stopPolarHeartRate(requestId, command);
        }

        if ("polar_pmd.get_status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("status", state.polarPmdStatusJson());
            return commandAck(requestId, command, true, "polar_pmd_status", result);
        }

        if ("polar_pmd.start".equals(command)) {
            return startPolarPmd(requestId, command, message.optJSONObject("params"));
        }

        if ("polar_pmd.stop".equals(command)) {
            return stopPolarPmd(requestId, command);
        }

        if ("breath_assessment.get_status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("status", state.breathAssessmentStatusJson());
            return commandAck(requestId, command, true, "breath_assessment_status", result);
        }

        if ("breath_assessment.configure".equals(command)) {
            return configureBreathAssessment(requestId, command, message.optJSONObject("params"));
        }

        if ("breath_assessment.reset".equals(command)) {
            return resetBreathAssessment(requestId, command, message.optJSONObject("params"));
        }

        if ("set_polar_breath_params".equals(command) || "polar_breath.set_params".equals(command)) {
            return setPolarBreathParams(requestId, command, message.optJSONObject("params"));
        }

        if ("polar_breath_calibrate_begin".equals(command) || "polar_breath.calibrate_begin".equals(command)) {
            return beginPolarBreathCalibration(requestId, command, message.optJSONObject("params"));
        }

        if ("polar_breath_calibrate_reset".equals(command) || "polar_breath.calibrate_reset".equals(command)) {
            return resetPolarBreathCalibration(requestId, command, message.optJSONObject("params"));
        }

        if ("breath_assessment.submit_controller_pose".equals(command)) {
            return submitControllerBreathPose(requestId, command, message.optJSONObject("params"));
        }

        if ("device_watchdog.get_status".equals(command)) {
            return getDeviceWatchdogStatus(requestId, command);
        }

        if ("device_watchdog.start".equals(command)) {
            return startDeviceWatchdog(requestId, command, message.optJSONObject("params"));
        }

        if ("device_watchdog.stop".equals(command)) {
            return stopDeviceWatchdog(requestId, command, message.optJSONObject("params"));
        }

        if ("device_watchdog.mark".equals(command)) {
            return markDeviceWatchdog(requestId, command, message.optJSONObject("params"));
        }

        if ("camera_provider.get_status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("status", state.cameraProviderStatusJson());
            return commandAck(requestId, command, true, "camera_provider_status", result);
        }

        if ("camera_provider.get_projection_profile".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("projection_profile", state.projectionProfileJson());
            return commandAck(requestId, command, true, "camera_provider_projection_profile", result);
        }

        if ("camera_provider.run_app_camera_probe".equals(command)) {
            return runCameraProviderAppCameraProbe(requestId, command, message.optJSONObject("params"));
        }

        if ("camera_provider.start_app_camera_luma_stream".equals(command)) {
            return startCameraProviderAppCameraLumaStream(requestId, command, message.optJSONObject("params"));
        }

        if ("camera_provider.start_app_camera_h264_stream".equals(command)) {
            return startCameraProviderAppCameraH264Stream(requestId, command, message.optJSONObject("params"));
        }

        if ("camera_provider.run_app_camera_h264_decode_probe".equals(command)) {
            return runCameraProviderAppCameraH264DecodeProbe(requestId, command, message.optJSONObject("params"));
        }

        if ("media.request_keyframe".equals(command)) {
            return requestMediaKeyframe(requestId, command, message.optJSONObject("params"));
        }

        if ("media.set_video_bitrate".equals(command)) {
            return setMediaVideoBitrate(requestId, command, message.optJSONObject("params"));
        }

        if ("media.set_quality_profile".equals(command)) {
            return setMediaQualityProfile(requestId, command, message.optJSONObject("params"));
        }

        if ("media.start_synthetic_h264_stream".equals(command)) {
            return startMediaSyntheticH264Stream(requestId, command, message.optJSONObject("params"));
        }

        if ("media.start_h264_tcp_proxy".equals(command)) {
            return startMediaH264TcpProxy(requestId, command, message.optJSONObject("params"));
        }

        if ("media.run_h264_tcp_proxy_probe".equals(command)) {
            return runMediaH264TcpProxyProbe(requestId, command, message.optJSONObject("params"));
        }

        if ("q2q_relay.start_sender".equals(command)) {
            return startQ2QRelaySender(requestId, command, message.optJSONObject("params"));
        }

        if ("q2q_relay.start_receiver".equals(command)) {
            return startQ2QRelayReceiver(requestId, command, message.optJSONObject("params"));
        }

        if ("q2q_relay.get_status".equals(command)) {
            return getQ2QRelayStatus(requestId, command, message.optJSONObject("params"));
        }

        if ("q2q_relay.stop".equals(command)) {
            return stopQ2QRelay(requestId, command, message.optJSONObject("params"));
        }

        if ("camera_provider.set_source_eye_mapping".equals(command)) {
            return setCameraProviderSourceEyeMapping(requestId, command, message.optJSONObject("params"));
        }

        if ("camera_provider.set_texture_transform".equals(command)) {
            return setCameraProviderTextureTransform(requestId, command, message.optJSONObject("params"));
        }

        if ("camera_provider.record_visual_acceptance".equals(command)) {
            return recordCameraProviderVisualAcceptance(requestId, command, message.optJSONObject("params"));
        }

        if ("shell_helper.get_status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("status", state.shellHelperStatusJson());
            return commandAck(requestId, command, true, "shell_helper_status", result);
        }

        if ("shell_helper.report_status".equals(command)) {
            return reportShellHelperStatus(requestId, command, message.optJSONObject("params"));
        }

        if ("experiment.get_control".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("control", state.experimentControlJson());
            return commandAck(requestId, command, true, "experiment_control_status", result);
        }

        if ("experiment.configure".equals(command)) {
            return configureExperimentControl(requestId, command, message.optJSONObject("params"));
        }

        if ("experiment.report_status".equals(command)) {
            return reportExperimentStatus(requestId, command, message.optJSONObject("params"));
        }

        if ("transport.describe_capabilities".equals(command)) {
            return describeTransportCapabilities(requestId, command);
        }

        if ("transport.create_session".equals(command)) {
            return createTransportSession(connection, requestId, command, message.optJSONObject("params"));
        }

        if ("transport.get_session".equals(command)) {
            return getTransportSession(requestId, command, message.optJSONObject("params"));
        }

        if ("transport.list_sessions".equals(command)) {
            return listTransportSessions(requestId, command);
        }

        if ("transport.close_session".equals(command)) {
            return closeTransportSession(requestId, command, message.optJSONObject("params"));
        }

        if ("video_lab.get_status".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("status", state.videoLabStatusJson());
            return commandAck(requestId, command, true, "video_lab_status", result);
        }

        if ("video_lab.get_scorecard".equals(command)) {
            state.acceptedCommands.incrementAndGet();
            JSONObject result = new JSONObject();
            result.put("scorecard", state.videoLabScorecardJson());
            return commandAck(requestId, command, true, "video_lab_scorecard", result);
        }

        if ("video_lab.register_encoded_stream_manifest".equals(command)) {
            return registerVideoLabEncodedStreamManifest(requestId, command, message.optJSONObject("params"));
        }

        if ("video_lab.record_encoded_sample_metadata".equals(command)) {
            return recordVideoLabEncodedSampleMetadata(requestId, command, message.optJSONObject("params"));
        }

        if ("video_lab.record_metric_sample".equals(command)) {
            return recordVideoLabMetricSample(requestId, command, message.optJSONObject("params"));
        }

        if ("open_ui".equals(command) ||
            "broker_console_open".equals(command) ||
            "ui.open".equals(command)) {
            return openBrokerConsole(connection, requestId, command, message.optJSONObject("params"));
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

    private JSONObject setCameraProviderSourceEyeMapping(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null || params.optString("source_eye_mapping", "").trim().length() == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_source_eye_mapping", "Command requires params.source_eye_mapping.");
        }

        JSONObject profile = state.setSourceEyeMapping(params.optString("source_eye_mapping", ""));
        JSONObject status = state.cameraProviderStatusJson();
        long now = unixNowNs();
        int profileBroadcasts = broadcastStreamEvent("camera_provider.projection_profile", profile.optLong("revision", 0L), now, profile);
        int statusBroadcasts = broadcastStreamEvent("camera_provider.status", status.optLong("revision", 0L), now, status);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("projection_profile", profile);
        result.put("status", status);
        result.put("profile_broadcasts", profileBroadcasts);
        result.put("status_broadcasts", statusBroadcasts);
        return commandAck(requestId, command, true, "camera_provider_source_eye_mapping_set", result);
    }

    private JSONObject setCameraProviderTextureTransform(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires params.");
        }

        String left = params.optString("left_texture_transform", "");
        String right = params.optString("right_texture_transform", "");
        if (left.trim().length() == 0 && right.trim().length() == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(
                requestId,
                command,
                "missing_texture_transform",
                "Command requires left_texture_transform or right_texture_transform.");
        }

        JSONObject profile = state.setTextureTransform(left, right);
        JSONObject status = state.cameraProviderStatusJson();
        long now = unixNowNs();
        int profileBroadcasts = broadcastStreamEvent("camera_provider.projection_profile", profile.optLong("revision", 0L), now, profile);
        int statusBroadcasts = broadcastStreamEvent("camera_provider.status", status.optLong("revision", 0L), now, status);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("projection_profile", profile);
        result.put("status", status);
        result.put("profile_broadcasts", profileBroadcasts);
        result.put("status_broadcasts", statusBroadcasts);
        return commandAck(requestId, command, true, "camera_provider_texture_transform_set", result);
    }

    private JSONObject recordCameraProviderVisualAcceptance(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        boolean accepted = params != null && params.optBoolean("accepted", true);
        String note = params != null ? params.optString("note", "") : "";
        String source = params != null ? params.optString("source", "") : "";
        JSONObject profile = state.recordVisualAcceptance(accepted, note, source);
        JSONObject status = state.cameraProviderStatusJson();
        long now = unixNowNs();
        int profileBroadcasts = broadcastStreamEvent("camera_provider.projection_profile", profile.optLong("revision", 0L), now, profile);
        int acceptanceBroadcasts = broadcastStreamEvent("camera_provider.visual_acceptance", profile.optLong("revision", 0L), now, profile);
        int statusBroadcasts = broadcastStreamEvent("camera_provider.status", status.optLong("revision", 0L), now, status);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("projection_profile", profile);
        result.put("status", status);
        result.put("profile_broadcasts", profileBroadcasts);
        result.put("acceptance_broadcasts", acceptanceBroadcasts);
        result.put("status_broadcasts", statusBroadcasts);
        return commandAck(requestId, command, true, "camera_provider_visual_acceptance_recorded", result);
    }

    private JSONObject runCameraProviderAppCameraProbe(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (context == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_context", "Broker app context is not available.");
        }

        JSONObject probe = BrokerAppCameraProbe.run(context, params);
        JSONObject status = state.recordAppCameraProbe(probe);
        JSONObject profile = state.projectionProfileJson();
        long now = unixNowNs();
        int statusBroadcasts = broadcastStreamEvent("camera_provider.status", status.optLong("revision", 0L), now, status);
        int profileBroadcasts = broadcastStreamEvent("camera_provider.projection_profile", profile.optLong("revision", 0L), now, profile);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("app_camera_probe", probe);
        result.put("status", status);
        result.put("projection_profile", profile);
        result.put("status_broadcasts", statusBroadcasts);
        result.put("profile_broadcasts", profileBroadcasts);
        return commandAck(requestId, command, true, "camera_provider_app_camera_probe_recorded", result);
    }

    private JSONObject startCameraProviderAppCameraLumaStream(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (context == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_context", "Broker app context is not available.");
        }

        JSONObject start = BrokerAppCameraLumaStreamSession.start(
            context,
            params,
            new BrokerAppCameraLumaStreamSession.Sink() {
                @Override
                public void registerManifest(JSONObject manifest) throws Exception {
                    recordAppCameraLumaManifest(manifest);
                }

                @Override
                public void recordSample(JSONObject sample) throws Exception {
                    recordAppCameraLumaSample(sample);
                }

                @Override
                public void recordMetric(JSONObject metric) throws Exception {
                    recordAppCameraLumaMetric(metric);
                }
            });
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("stream_start", start);
        return commandAck(requestId, command, true, "camera_provider_app_camera_luma_stream_started", result);
    }

    private JSONObject startCameraProviderAppCameraH264Stream(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (context == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_context", "Broker app context is not available.");
        }

        JSONObject start;
        try {
            start = BrokerAppCameraH264StreamSession.start(
                context,
                params,
                new BrokerAppCameraH264StreamSession.Sink() {
                    @Override
                    public void registerManifest(JSONObject manifest) throws Exception {
                        recordAppCameraLumaManifest(manifest);
                    }

                    @Override
                    public void recordSample(JSONObject sample) throws Exception {
                        recordAppCameraLumaSample(sample);
                    }

                    @Override
                    public void recordMetric(JSONObject metric) throws Exception {
                        recordAppCameraLumaMetric(metric);
                    }
                });
        } catch (IllegalArgumentException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "invalid_h264_stream_params", safeMessage(ex));
        } catch (SecurityException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "h264_stream_not_allowed", safeMessage(ex));
        }
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("stream_start", start);
        result.put("projection_profile", state.projectionProfileJson());
        return commandAck(requestId, command, true, "camera_provider_app_camera_h264_stream_started", result);
    }

    private JSONObject requestMediaKeyframe(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject control = BrokerAppCameraH264StreamSession.requestKeyframe(params);
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("control", control);
        return commandAck(
            requestId,
            command,
            true,
            control.optBoolean("applied", false) ? "media_keyframe_requested" : "media_keyframe_not_applied",
            result);
    }

    private JSONObject setMediaVideoBitrate(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject control = BrokerAppCameraH264StreamSession.setVideoBitrate(params);
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("control", control);
        return commandAck(
            requestId,
            command,
            true,
            control.optBoolean("applied", false) ? "media_video_bitrate_applied" : "media_video_bitrate_not_applied",
            result);
    }

    private JSONObject setMediaQualityProfile(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject control = BrokerAppCameraH264StreamSession.setQualityProfile(params);
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("control", control);
        return commandAck(
            requestId,
            command,
            true,
            control.optBoolean("applied", false) ? "media_quality_profile_applied" : "media_quality_profile_not_applied",
            result);
    }

    private JSONObject startMediaSyntheticH264Stream(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject start;
        try {
            start = BrokerAppCameraH264StreamSession.startSynthetic(
                context,
                params,
                new BrokerAppCameraH264StreamSession.Sink() {
                    @Override
                    public void registerManifest(JSONObject manifest) throws Exception {
                        recordAppCameraLumaManifest(manifest);
                    }

                    @Override
                    public void recordSample(JSONObject sample) throws Exception {
                        recordAppCameraLumaSample(sample);
                    }

                    @Override
                    public void recordMetric(JSONObject metric) throws Exception {
                        recordAppCameraLumaMetric(metric);
                    }
                });
        } catch (IllegalArgumentException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "invalid_synthetic_h264_stream_params", safeMessage(ex));
        } catch (SecurityException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "synthetic_h264_stream_not_allowed", safeMessage(ex));
        }

        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("stream_start", start);
        return commandAck(requestId, command, true, "media_synthetic_h264_stream_started", result);
    }

    private JSONObject startMediaH264TcpProxy(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject start;
        try {
            start = BrokerH264TcpProxySession.start(
                params,
                new BrokerH264TcpProxySession.Sink() {
                    @Override
                    public void registerManifest(JSONObject manifest) throws Exception {
                        recordAppCameraLumaManifest(manifest);
                    }

                    @Override
                    public void recordSample(JSONObject sample) throws Exception {
                        recordAppCameraLumaSample(sample);
                    }

                    @Override
                    public void recordMetric(JSONObject metric) throws Exception {
                        recordAppCameraLumaMetric(metric);
                    }
                });
        } catch (IllegalArgumentException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "invalid_h264_proxy_params", safeMessage(ex));
        } catch (SecurityException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "h264_proxy_not_allowed", safeMessage(ex));
        }

        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("proxy_start", start);
        return commandAck(requestId, command, true, "media_h264_tcp_proxy_started", result);
    }

    private JSONObject runMediaH264TcpProxyProbe(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject probe;
        try {
            probe = BrokerH264TcpProxySession.runProbe(
                params,
                new BrokerH264TcpProxySession.Sink() {
                    @Override
                    public void registerManifest(JSONObject manifest) throws Exception {
                        recordAppCameraLumaManifest(manifest);
                    }

                    @Override
                    public void recordSample(JSONObject sample) throws Exception {
                        recordAppCameraLumaSample(sample);
                    }

                    @Override
                    public void recordMetric(JSONObject metric) throws Exception {
                        recordAppCameraLumaMetric(metric);
                    }
                });
        } catch (IllegalArgumentException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "invalid_h264_proxy_probe_params", safeMessage(ex));
        } catch (SecurityException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "h264_proxy_probe_not_allowed", safeMessage(ex));
        }

        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("proxy_probe", probe);
        return commandAck(
            requestId,
            command,
            true,
            probe.optBoolean("succeeded", false)
                ? "media_h264_tcp_proxy_probe_succeeded"
                : "media_h264_tcp_proxy_probe_completed",
            result);
    }

    private JSONObject startQ2QRelaySender(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (context == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_context", "Broker app context is not available.");
        }

        JSONObject start;
        try {
            start = BrokerQ2QRelayClientSession.startSender(
                context,
                params,
                new BrokerAppCameraH264StreamSession.Sink() {
                    @Override
                    public void registerManifest(JSONObject manifest) throws Exception {
                        recordAppCameraLumaManifest(manifest);
                    }

                    @Override
                    public void recordSample(JSONObject sample) throws Exception {
                        recordAppCameraLumaSample(sample);
                    }

                    @Override
                    public void recordMetric(JSONObject metric) throws Exception {
                        recordAppCameraLumaMetric(metric);
                    }
                });
        } catch (IllegalArgumentException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "invalid_q2q_relay_sender_params", safeMessage(ex));
        } catch (SecurityException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "q2q_relay_sender_not_allowed", safeMessage(ex));
        } catch (Exception ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "q2q_relay_sender_start_failed", safeMessage(ex));
        }

        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("relay_start", start);
        return commandAck(requestId, command, true, "q2q_relay_sender_started", result);
    }

    private JSONObject startQ2QRelayReceiver(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject start;
        try {
            start = BrokerQ2QRelayClientSession.startReceiver(params);
        } catch (IllegalArgumentException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "invalid_q2q_relay_receiver_params", safeMessage(ex));
        } catch (SecurityException ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "q2q_relay_receiver_not_allowed", safeMessage(ex));
        } catch (Exception ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "q2q_relay_receiver_start_failed", safeMessage(ex));
        }

        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("relay_start", start);
        return commandAck(requestId, command, true, "q2q_relay_receiver_started", result);
    }

    private JSONObject getQ2QRelayStatus(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = BrokerQ2QRelayClientSession.statusJson(params);
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "q2q_relay_status", result);
    }

    private JSONObject stopQ2QRelay(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject stopped = BrokerQ2QRelayClientSession.stop(params);
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("stop", stopped);
        return commandAck(requestId, command, true, "q2q_relay_stopped", result);
    }

    private JSONObject runCameraProviderAppCameraH264DecodeProbe(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (context == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_context", "Broker app context is not available.");
        }

        JSONObject probe = BrokerAppCameraH264DecodeProbe.run(context, params);
        JSONObject manifest = buildAppCameraH264DecodeProbeManifest(probe);
        JSONObject metric = buildAppCameraH264DecodeProbeMetric(probe);
        recordAppCameraLumaManifest(manifest);
        recordAppCameraLumaMetric(metric);
        JSONObject videoLabStatus = state.videoLabStatusJson();
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("decode_probe", probe);
        result.put("manifest", manifest);
        result.put("metric", metric);
        result.put("video_lab_status", videoLabStatus);
        return commandAck(
            requestId,
            command,
            true,
            probe.optBoolean("decode_succeeded", false)
                ? "camera_provider_app_camera_h264_decode_probe_succeeded"
                : "camera_provider_app_camera_h264_decode_probe_completed",
            result);
    }

    private JSONObject buildAppCameraH264DecodeProbeManifest(JSONObject probe) throws Exception {
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", "broker_app.camera_h264_decode_probe");
        manifest.put("session_id", probe.optString("session_id", "broker-app-camera-h264-decode-" + System.currentTimeMillis()));
        manifest.put("source", "broker_app_camera2_mediacodec_decode_probe");
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", "in_process_android_mediacodec_decode");
        manifest.put("mime_type", "video/avc");
        manifest.put("codec", "h264");
        manifest.put("decoder_target", "byte_buffer");
        manifest.put("width", probe.optInt("width", 0));
        manifest.put("height", probe.optInt("height", 0));
        manifest.put("frame_rate_hz", 30);
        manifest.put("bitrate_bps", probe.optInt("bitrate_bps", 0));
        manifest.put("source_kind", "broker_app_camera2_mediacodec_decode_probe");
        manifest.put("camera_id", probe.optString("camera_id", ""));
        manifest.put("camera_source_id", "camera2:" + probe.optString("camera_id", ""));
        manifest.put("source_api_path", "AndroidCamera2");
        manifest.put("camera_permission_state", "Granted");
        manifest.put("headset_camera_permission_state", "Granted");
        manifest.put("selected_camera_id", probe.optString("camera_id", ""));
        manifest.put("selected_width", probe.optInt("width", 0));
        manifest.put("selected_height", probe.optInt("height", 0));
        manifest.put("selected_reason", "decode_probe_capture_selection");
        manifest.put("timestamp_domain", "REALTIME".equals(probe.optString("sensor_timestamp_source", "")) ? "ElapsedRealtime" : "Unknown");
        manifest.put("capture_ms", probe.optInt("capture_ms", 0));
        manifest.put("max_packets", probe.optInt("max_packets", 0));
        manifest.put("decoder_api", probe.optString("decoder_api", "android.media.MediaCodec"));
        manifest.put("decoder_name", probe.optString("decoder_name", ""));
        manifest.put("decoder_output_mode", probe.optString("decoder_output_mode", "byte_buffer"));
        manifest.put("decoder_low_latency_feature_supported", probe.optBoolean("decoder_low_latency_feature_supported", false));
        manifest.put("decoder_low_latency_config_requested", probe.optBoolean("decoder_low_latency_config_requested", false));
        manifest.put("decoder_low_latency_parameter_succeeded", probe.optBoolean("decoder_low_latency_parameter_succeeded", false));
        manifest.put("encoder_name", probe.optString("encoder_name", ""));
        manifest.put("encoder_selection_source", probe.optString("encoder_selection_source", ""));
        manifest.put("encoder_selected_name", probe.optString("encoder_selected_name", ""));
        manifest.put("encoder_hardware_accelerated", probe.optBoolean("encoder_hardware_accelerated", false));
        manifest.put("encoder_software_only", probe.optBoolean("encoder_software_only", false));
        manifest.put("encoder_size_supported", probe.optBoolean("encoder_size_supported", false));
        manifest.put("encoder_size_and_rate_supported", probe.optBoolean("encoder_size_and_rate_supported", false));
        manifest.put("encoder_bitrate_supported", probe.optBoolean("encoder_bitrate_supported", false));
        manifest.put("encoder_cbr_supported", probe.optBoolean("encoder_cbr_supported", false));
        manifest.put("encoder_cbr_fd_supported", probe.optBoolean("encoder_cbr_fd_supported", false));
        manifest.put("encoder_vbr_supported", probe.optBoolean("encoder_vbr_supported", false));
        manifest.put("bitrate_mode_requested", probe.optString("bitrate_mode_requested", ""));
        manifest.put("bitrate_mode_applied", probe.optString("bitrate_mode_applied", ""));
        manifest.put("bitrate_mode_output_format", probe.optString("bitrate_mode_output_format", ""));
        manifest.put("encoder_output_format_changes", probe.optInt("encoder_output_format_changes", 0));
        manifest.put("encoder_output_mime", probe.optString("encoder_output_mime", ""));
        manifest.put("encoder_output_width", probe.optInt("encoder_output_width", 0));
        manifest.put("encoder_output_height", probe.optInt("encoder_output_height", 0));
        manifest.put("prepend_headers_to_sync_frames_applied", probe.optBoolean("prepend_headers_to_sync_frames_applied", false));
        manifest.put("sync_frame_request_on_start_succeeded", probe.optBoolean("sync_frame_request_on_start_succeeded", false));
        manifest.put("csd_source", probe.optString("csd_source", ""));
        manifest.put("csd_sps_bytes", probe.optInt("csd_sps_bytes", 0));
        manifest.put("csd_pps_bytes", probe.optInt("csd_pps_bytes", 0));
        manifest.put("sps_present", probe.optBoolean("csd_sps_found", false));
        manifest.put("pps_present", probe.optBoolean("csd_pps_found", false));
        manifest.put("keyframe_count", probe.optInt("keyframe_count", 0));
        manifest.put("csd_sps_base64", probe.optString("csd_sps_base64", ""));
        manifest.put("csd_pps_base64", probe.optString("csd_pps_base64", ""));
        manifest.put("sensor_timestamp_source", probe.optString("sensor_timestamp_source", ""));
        manifest.put("camera_capture_started_count", probe.optInt("camera_capture_started_count", 0));
        manifest.put("camera_first_capture_started_ns", probe.optLong("camera_first_capture_started_ns", 0L));
        manifest.put("camera_last_capture_started_ns", probe.optLong("camera_last_capture_started_ns", 0L));
        manifest.put("camera_first_frame_number", probe.optLong("camera_first_frame_number", -1L));
        manifest.put("camera_last_frame_number", probe.optLong("camera_last_frame_number", -1L));
        manifest.put("camera_first_capture_callback_elapsed_ns", probe.optLong("camera_first_capture_callback_elapsed_ns", 0L));
        manifest.put("camera_last_capture_callback_elapsed_ns", probe.optLong("camera_last_capture_callback_elapsed_ns", 0L));
        return manifest;
    }

    private JSONObject buildAppCameraH264DecodeProbeMetric(JSONObject probe) throws Exception {
        long now = unixNowNs();
        JSONObject metric = new JSONObject();
        metric.put("schema", "rusty.xr.video_lab.metric_sample.v1");
        metric.put("stream_id", "broker_app.camera_h264_decode_probe");
        metric.put("source", "broker_app_camera2_mediacodec_decode_probe");
        metric.put("transport", "metadata_only");
        metric.put("payload_transport", "in_process_android_mediacodec_decode");
        metric.put("codec", "h264");
        metric.put("session_id", probe.optString("session_id", "broker-app-camera-h264-decode-" + System.currentTimeMillis()));
        metric.put("camera_id", probe.optString("camera_id", ""));
        metric.put("camera_source_id", "camera2:" + probe.optString("camera_id", ""));
        metric.put("source_api_path", "AndroidCamera2");
        metric.put("camera_permission_state", "Granted");
        metric.put("headset_camera_permission_state", "Granted");
        metric.put("selected_camera_id", probe.optString("camera_id", ""));
        metric.put("selected_width", probe.optInt("width", 0));
        metric.put("selected_height", probe.optInt("height", 0));
        metric.put("selected_reason", "decode_probe_capture_selection");
        metric.put("timestamp_domain", "REALTIME".equals(probe.optString("sensor_timestamp_source", "")) ? "ElapsedRealtime" : "Unknown");
        metric.put("sequence_id", System.currentTimeMillis() * 1000L);
        metric.put("source_time_unix_ns", now);
        metric.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        metric.put("camera_encode_start_elapsed_ns", probe.optLong("camera_encode_start_elapsed_ns", 0L));
        metric.put("camera_encode_end_elapsed_ns", probe.optLong("camera_encode_end_elapsed_ns", 0L));
        metric.put("camera_encode_duration_ns", probe.optLong("camera_encode_duration_ns", 0L));
        metric.put("decoder_start_elapsed_ns", probe.optLong("decode_start_elapsed_ns", 0L));
        metric.put("decoder_end_elapsed_ns", probe.optLong("decode_end_elapsed_ns", 0L));
        metric.put("decoder_duration_ns", probe.optLong("decode_duration_ns", 0L));
        metric.put("packet_count", probe.optInt("encoded_packet_count", 0));
        metric.put("video_packet_count", probe.optInt("encoded_video_packet_count", 0));
        metric.put("codec_config_packet_count", probe.optInt("codec_config_packet_count", 0));
        metric.put("keyframe_count", probe.optInt("keyframe_count", 0));
        metric.put("payload_size_bytes", probe.optLong("encoded_payload_bytes", 0L));
        metric.put("decoder_input_buffers", probe.optInt("input_buffer_count", 0));
        metric.put("decoder_input_bytes", probe.optLong("input_bytes", 0L));
        metric.put("decoder_output_buffers", probe.optInt("output_buffer_count", 0));
        metric.put("decoded_frame_count", probe.optInt("decoded_frame_count", 0));
        metric.put("decoder_output_bytes", probe.optLong("decoder_output_bytes", 0L));
        metric.put("decoder_name", probe.optString("decoder_name", ""));
        metric.put("decoder_output_mode", probe.optString("decoder_output_mode", "byte_buffer"));
        metric.put("decoder_low_latency_feature_supported", probe.optBoolean("decoder_low_latency_feature_supported", false));
        metric.put("decoder_low_latency_config_requested", probe.optBoolean("decoder_low_latency_config_requested", false));
        metric.put("decoder_low_latency_parameter_succeeded", probe.optBoolean("decoder_low_latency_parameter_succeeded", false));
        metric.put("decode_succeeded", probe.optBoolean("decode_succeeded", false));
        metric.put("dropped_frames", 0);
        metric.put("stale_frames", 0);
        metric.put("queue_depth", 0);
        metric.put("width", probe.optInt("width", 0));
        metric.put("height", probe.optInt("height", 0));
        metric.put("encoder_name", probe.optString("encoder_name", ""));
        metric.put("encoder_selection_source", probe.optString("encoder_selection_source", ""));
        metric.put("encoder_selected_name", probe.optString("encoder_selected_name", ""));
        metric.put("encoder_hardware_accelerated", probe.optBoolean("encoder_hardware_accelerated", false));
        metric.put("encoder_software_only", probe.optBoolean("encoder_software_only", false));
        metric.put("encoder_size_supported", probe.optBoolean("encoder_size_supported", false));
        metric.put("encoder_size_and_rate_supported", probe.optBoolean("encoder_size_and_rate_supported", false));
        metric.put("encoder_bitrate_supported", probe.optBoolean("encoder_bitrate_supported", false));
        metric.put("encoder_cbr_supported", probe.optBoolean("encoder_cbr_supported", false));
        metric.put("encoder_cbr_fd_supported", probe.optBoolean("encoder_cbr_fd_supported", false));
        metric.put("encoder_vbr_supported", probe.optBoolean("encoder_vbr_supported", false));
        metric.put("bitrate_mode_requested", probe.optString("bitrate_mode_requested", ""));
        metric.put("bitrate_mode_applied", probe.optString("bitrate_mode_applied", ""));
        metric.put("bitrate_mode_output_format", probe.optString("bitrate_mode_output_format", ""));
        metric.put("encoder_output_format_changes", probe.optInt("encoder_output_format_changes", 0));
        metric.put("encoder_output_mime", probe.optString("encoder_output_mime", ""));
        metric.put("encoder_output_width", probe.optInt("encoder_output_width", 0));
        metric.put("encoder_output_height", probe.optInt("encoder_output_height", 0));
        metric.put("prepend_headers_to_sync_frames_applied", probe.optBoolean("prepend_headers_to_sync_frames_applied", false));
        metric.put("sync_frame_request_on_start_succeeded", probe.optBoolean("sync_frame_request_on_start_succeeded", false));
        metric.put("csd_source", probe.optString("csd_source", ""));
        metric.put("csd_sps_bytes", probe.optInt("csd_sps_bytes", 0));
        metric.put("csd_pps_bytes", probe.optInt("csd_pps_bytes", 0));
        metric.put("sps_present", probe.optBoolean("csd_sps_found", false));
        metric.put("pps_present", probe.optBoolean("csd_pps_found", false));
        metric.put("sensor_timestamp_source", probe.optString("sensor_timestamp_source", ""));
        metric.put("camera_capture_started_count", probe.optInt("camera_capture_started_count", 0));
        metric.put("camera_first_capture_started_ns", probe.optLong("camera_first_capture_started_ns", 0L));
        metric.put("camera_last_capture_started_ns", probe.optLong("camera_last_capture_started_ns", 0L));
        metric.put("camera_first_frame_number", probe.optLong("camera_first_frame_number", -1L));
        metric.put("camera_last_frame_number", probe.optLong("camera_last_frame_number", -1L));
        metric.put("camera_first_capture_callback_elapsed_ns", probe.optLong("camera_first_capture_callback_elapsed_ns", 0L));
        metric.put("camera_last_capture_callback_elapsed_ns", probe.optLong("camera_last_capture_callback_elapsed_ns", 0L));
        if (!probe.optBoolean("decode_succeeded", false) || probe.optString("last_error", "").length() > 0) {
            metric.put("last_error", probe.optString("last_error", "Decode probe completed without a decoded frame."));
        }
        return metric;
    }

    private void recordAppCameraLumaManifest(JSONObject params) throws Exception {
        long receiveUnixNs = unixNowNs();
        long receiveElapsedNs = SystemClock.elapsedRealtimeNanos();
        long revision = state.videoLabEncodedStreamManifests.incrementAndGet();
        JSONObject manifest = state.registerVideoLabEncodedStreamManifest(params, revision, receiveUnixNs, receiveElapsedNs);
        broadcastStreamEvent("video_lab.encoded_stream_manifest", revision, receiveUnixNs, manifest);
    }

    private void recordAppCameraLumaSample(JSONObject params) throws Exception {
        long receiveUnixNs = unixNowNs();
        long receiveElapsedNs = SystemClock.elapsedRealtimeNanos();
        long accepted = state.videoLabEncodedSampleMetadata.incrementAndGet();
        long sequence = params.optLong("sequence_id", accepted);
        JSONObject sample = state.recordVideoLabEncodedSampleMetadata(params, sequence, receiveUnixNs, receiveElapsedNs);
        broadcastStreamEvent("video_lab.encoded_sample_metadata", sequence, receiveUnixNs, sample);
    }

    private void recordAppCameraLumaMetric(JSONObject params) throws Exception {
        long receiveUnixNs = unixNowNs();
        long sequence = params.optLong("sequence_id", receiveUnixNs);
        JSONObject payload = new JSONObject(params.toString());
        payload.put("broker_receive_time_unix_ns", receiveUnixNs);
        payload.put("broker_receive_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        payload.put("broker_publish_time_unix_ns", unixNowNs());
        payload.put("broker_publish_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        state.videoLabMetricSamples.incrementAndGet();
        JSONObject metric = state.recordVideoLabMetricSample(payload);
        broadcastStreamEvent("video_lab.metric_sample", sequence, receiveUnixNs, metric);
    }

    private JSONObject reportShellHelperStatus(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = state.reportShellHelperStatus(params);
        long now = unixNowNs();
        int broadcasts = broadcastStreamEvent("shell_helper.status", now, now, status);
        JSONObject kioskStatus = state.rustyKioskStatusJson();
        int kioskBroadcasts = broadcastStreamEvent("kiosk:control_plane", now, now, kioskStatus);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("status", status);
        result.put("broadcasts", broadcasts);
        result.put("kiosk_status", kioskStatus);
        result.put("kiosk_broadcasts", kioskBroadcasts);
        return commandAck(requestId, command, true, "shell_helper_status_reported", result);
    }

    private JSONObject configureExperimentControl(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = state.configureExperimentControl(params);
        long now = unixNowNs();
        int broadcasts = broadcastStreamEvent("experiment.control", now, now, status);
        JSONObject kioskStatus = state.rustyKioskStatusJson();
        int kioskBroadcasts = broadcastStreamEvent("kiosk:control_plane", now, now, kioskStatus);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("control", status);
        result.put("broadcasts", broadcasts);
        result.put("kiosk_status", kioskStatus);
        result.put("kiosk_broadcasts", kioskBroadcasts);
        return commandAck(requestId, command, true, "experiment_control_configured", result);
    }

    private JSONObject reportExperimentStatus(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = state.reportExperimentStatus(params);
        long now = unixNowNs();
        int broadcasts = broadcastStreamEvent("experiment.control", now, now, status);
        JSONObject kioskStatus = state.rustyKioskStatusJson();
        int kioskBroadcasts = broadcastStreamEvent("kiosk:control_plane", now, now, kioskStatus);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("control", status);
        result.put("broadcasts", broadcasts);
        result.put("kiosk_status", kioskStatus);
        result.put("kiosk_broadcasts", kioskBroadcasts);
        return commandAck(requestId, command, true, "experiment_status_reported", result);
    }

    private JSONObject recordVideoLabMetricSample(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires video-lab metric params.");
        }

        long receiveUnixNs = unixNowNs();
        long receiveElapsedNs = SystemClock.elapsedRealtimeNanos();
        long accepted = state.videoLabMetricSamples.incrementAndGet();
        long sequence = params.optLong("sequence_id", accepted);

        JSONObject payload = new JSONObject(params.toString());
        if (!payload.has("schema")) {
            payload.put("schema", "rusty.xr.video_lab.metric_sample.v1");
        }
        if (!payload.has("stream_id")) {
            payload.put("stream_id", "video_lab.synthetic");
        }
        if (!payload.has("source")) {
            payload.put("source", "synthetic");
        }
        if (!payload.has("transport")) {
            payload.put("transport", "metadata_only");
        }
        if (!payload.has("codec")) {
            payload.put("codec", "none");
        }
        payload.put("broker_receive_time_unix_ns", receiveUnixNs);
        payload.put("broker_receive_time_elapsed_ns", receiveElapsedNs);
        payload.put("broker_publish_time_unix_ns", unixNowNs());
        payload.put("broker_publish_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());

        JSONObject metric = state.recordVideoLabMetricSample(payload);
        int broadcasts = broadcastStreamEvent("video_lab.metric_sample", sequence, receiveUnixNs, metric);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("stream", "video_lab.metric_sample");
        result.put("sequence_id", sequence);
        result.put("accepted_metric_samples", accepted);
        result.put("broadcasts", broadcasts);
        result.put("metric", metric);
        return commandAck(requestId, command, true, "video_lab_metric_sample_recorded", result);
    }

    private JSONObject registerVideoLabEncodedStreamManifest(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires encoded-stream manifest params.");
        }

        long receiveUnixNs = unixNowNs();
        long receiveElapsedNs = SystemClock.elapsedRealtimeNanos();
        long revision = state.videoLabEncodedStreamManifests.incrementAndGet();
        JSONObject manifest = state.registerVideoLabEncodedStreamManifest(params, revision, receiveUnixNs, receiveElapsedNs);
        int broadcasts = broadcastStreamEvent("video_lab.encoded_stream_manifest", revision, receiveUnixNs, manifest);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("stream", "video_lab.encoded_stream_manifest");
        result.put("revision", revision);
        result.put("accepted_encoded_stream_manifests", revision);
        result.put("broadcasts", broadcasts);
        result.put("manifest", manifest);
        return commandAck(requestId, command, true, "video_lab_encoded_stream_manifest_registered", result);
    }

    private JSONObject recordVideoLabEncodedSampleMetadata(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires encoded-sample metadata params.");
        }

        long receiveUnixNs = unixNowNs();
        long receiveElapsedNs = SystemClock.elapsedRealtimeNanos();
        long accepted = state.videoLabEncodedSampleMetadata.incrementAndGet();
        long sequence = params.optLong("sequence_id", accepted);
        JSONObject sample = state.recordVideoLabEncodedSampleMetadata(params, sequence, receiveUnixNs, receiveElapsedNs);
        int broadcasts = broadcastStreamEvent("video_lab.encoded_sample_metadata", sequence, receiveUnixNs, sample);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("stream", "video_lab.encoded_sample_metadata");
        result.put("sequence_id", sequence);
        result.put("accepted_encoded_sample_metadata", accepted);
        result.put("broadcasts", broadcasts);
        result.put("sample", sample);
        return commandAck(requestId, command, true, "video_lab_encoded_sample_metadata_recorded", result);
    }

    private JSONObject startPolarPmd(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        PolarPmdBrokerSource source = polarPmdSource;
        if (source == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "polar_pmd_unavailable", "Polar PMD source is not attached to this broker.");
        }

        String deviceAddress = polarDeviceAddress(params);
        long scanTimeoutMs = polarScanTimeoutMs(params);
        String pmdStream = polarPmdStream(params);
        boolean highPriority = optBooleanParam(params, "high_connection_priority", "highConnectionPriority", false)
            || optBooleanParam(params, "android_high_connection_priority", "androidHighConnectionPriority", false);
        int accSampleRateHz = optIntParam(params, "acc_sample_rate_hz", "accSampleRateHz", 200);

        JSONObject status = source.start(deviceAddress, scanTimeoutMs, pmdStream, highPriority, accSampleRateHz);
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "polar_pmd_starting", result);
    }

    private JSONObject stopPolarPmd(String requestId, String command) throws Exception {
        PolarPmdBrokerSource source = polarPmdSource;
        if (source == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "polar_pmd_unavailable", "Polar PMD source is not attached to this broker.");
        }

        JSONObject status = source.stop();
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "polar_pmd_stopping", result);
    }

    private JSONObject getDeviceWatchdogStatus(String requestId, String command) throws Exception {
        DeviceWatchdog watchdog = deviceWatchdog;
        if (watchdog == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "device_watchdog_unavailable", "Device watchdog is not attached to this broker.");
        }

        JSONObject result = new JSONObject();
        result.put("status", watchdog.statusJson());
        state.acceptedCommands.incrementAndGet();
        return commandAck(requestId, command, true, "device_watchdog_status", result);
    }

    private JSONObject startDeviceWatchdog(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        DeviceWatchdog watchdog = deviceWatchdog;
        if (watchdog == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "device_watchdog_unavailable", "Device watchdog is not attached to this broker.");
        }

        try {
            JSONObject result = new JSONObject();
            result.put("status", watchdog.start(params != null ? params : new JSONObject()));
            state.acceptedCommands.incrementAndGet();
            return commandAck(requestId, command, true, "device_watchdog_started", result);
        } catch (Exception ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "device_watchdog_start_failed", safeMessage(ex));
        }
    }

    private JSONObject stopDeviceWatchdog(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        DeviceWatchdog watchdog = deviceWatchdog;
        if (watchdog == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "device_watchdog_unavailable", "Device watchdog is not attached to this broker.");
        }

        String reason = params != null ? params.optString("reason", "websocket_stop") : "websocket_stop";
        JSONObject result = new JSONObject();
        result.put("status", watchdog.stop(reason));
        state.acceptedCommands.incrementAndGet();
        return commandAck(requestId, command, true, "device_watchdog_stopped", result);
    }

    private JSONObject markDeviceWatchdog(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        DeviceWatchdog watchdog = deviceWatchdog;
        if (watchdog == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "device_watchdog_unavailable", "Device watchdog is not attached to this broker.");
        }

        try {
            JSONObject result = new JSONObject();
            result.put("status", watchdog.mark(params));
            state.acceptedCommands.incrementAndGet();
            return commandAck(requestId, command, true, "device_watchdog_marker_recorded", result);
        } catch (Exception ex) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "device_watchdog_mark_failed", safeMessage(ex));
        }
    }

    private JSONObject startPolar(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        boolean includeHeartRate = optBooleanParam(params, "include_hr", "includeHeartRate", true);
        boolean includePmd = optBooleanParam(params, "include_pmd", "includePmd", false);
        String deviceAddress = polarDeviceAddress(params);
        long scanTimeoutMs = polarScanTimeoutMs(params);
        String pmdStream = polarPmdStream(params);
        boolean highPriority = optBooleanParam(params, "high_connection_priority", "highConnectionPriority", false)
            || optBooleanParam(params, "android_high_connection_priority", "androidHighConnectionPriority", false);
        int accSampleRateHz = optIntParam(params, "acc_sample_rate_hz", "accSampleRateHz", 200);

        JSONObject result = new JSONObject();
        int started = 0;
        if (includeHeartRate) {
            PolarHeartRateBrokerSource source = polarHeartRateSource;
            if (source == null) {
                result.put("heart_rate_error", "Polar heart-rate source is not attached to this broker.");
            } else {
                result.put("heart_rate", source.start(deviceAddress, scanTimeoutMs));
                started++;
            }
        }

        if (includePmd) {
            PolarPmdBrokerSource source = polarPmdSource;
            if (source == null) {
                result.put("pmd_error", "Polar PMD source is not attached to this broker.");
            } else {
                result.put("pmd", source.start(deviceAddress, scanTimeoutMs, pmdStream, highPriority, accSampleRateHz));
                started++;
            }
        }

        result.put("include_hr", includeHeartRate);
        result.put("include_pmd", includePmd);
        result.put("pmd_stream", pmdStream);
        result.put("acc_sample_rate_hz", accSampleRateHz);
        result.put("high_connection_priority", highPriority);
        result.put("pmd_default", "disabled unless include_pmd is true");
        if (started == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "polar_source_unavailable", "No requested Polar source could be started.");
        }

        state.acceptedCommands.incrementAndGet();
        return commandAck(requestId, command, true, "polar_starting", result);
    }

    private JSONObject stopPolar(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        boolean stopHeartRate = optBooleanParam(params, "stop_hr", "stopHeartRate", true);
        boolean stopPmd = optBooleanParam(params, "stop_pmd", "stopPmd", true);
        JSONObject result = new JSONObject();
        int stopped = 0;

        if (stopHeartRate) {
            PolarHeartRateBrokerSource source = polarHeartRateSource;
            if (source == null) {
                result.put("heart_rate_error", "Polar heart-rate source is not attached to this broker.");
            } else {
                result.put("heart_rate", source.stop());
                stopped++;
            }
        }

        if (stopPmd) {
            PolarPmdBrokerSource source = polarPmdSource;
            if (source == null) {
                result.put("pmd_error", "Polar PMD source is not attached to this broker.");
            } else {
                result.put("pmd", source.stop());
                stopped++;
            }
        }

        if (stopped == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "polar_source_unavailable", "No requested Polar source could be stopped.");
        }

        state.acceptedCommands.incrementAndGet();
        return commandAck(requestId, command, true, "polar_stopping", result);
    }

    private JSONObject startPolarHeartRate(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        PolarHeartRateBrokerSource source = polarHeartRateSource;
        if (source == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "polar_hr_unavailable", "Polar heart-rate source is not attached to this broker.");
        }

        JSONObject status = source.start(polarDeviceAddress(params), polarScanTimeoutMs(params));
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("status", status);
        result.put("pmd_default", "disabled");
        return commandAck(requestId, command, true, "polar_hr_starting", result);
    }

    private JSONObject stopPolarHeartRate(String requestId, String command) throws Exception {
        PolarHeartRateBrokerSource source = polarHeartRateSource;
        if (source == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "polar_hr_unavailable", "Polar heart-rate source is not attached to this broker.");
        }

        JSONObject status = source.stop();
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "polar_hr_stopping", result);
    }

    private static String polarDeviceAddress(JSONObject params) {
        if (params == null) {
            return "";
        }

        String deviceAddress = params.optString("device_address", "");
        if (deviceAddress.trim().length() == 0) {
            deviceAddress = params.optString("deviceAddress", "");
        }
        return deviceAddress;
    }

    private static long polarScanTimeoutMs(JSONObject params) {
        long scanTimeoutMs = BrokerRuntimeConfig.DEFAULT_POLAR_SCAN_TIMEOUT_MS;
        if (params != null) {
            scanTimeoutMs = params.optLong("scan_timeout_ms", scanTimeoutMs);
            if (!params.has("scan_timeout_ms")) {
                scanTimeoutMs = params.optLong("scanTimeoutMs", scanTimeoutMs);
            }
        }
        return scanTimeoutMs;
    }

    private static String polarPmdStream(JSONObject params) {
        if (params == null) {
            return PolarPmdBrokerSource.PMD_STREAM_ACC;
        }
        String stream = params.optString("pmd_stream", "");
        if (stream.trim().length() == 0) {
            stream = params.optString("pmdStream", "");
        }
        if (stream.trim().length() == 0) {
            stream = params.optString("measurement", "");
        }
        return stream.trim().length() == 0 ? PolarPmdBrokerSource.PMD_STREAM_ACC : stream;
    }

    private static boolean optBooleanParam(
        JSONObject params,
        String snakeName,
        String camelName,
        boolean defaultValue) {
        if (params == null) {
            return defaultValue;
        }
        if (params.has(snakeName)) {
            return params.optBoolean(snakeName, defaultValue);
        }
        if (params.has(camelName)) {
            return params.optBoolean(camelName, defaultValue);
        }
        return defaultValue;
    }

    private static int optIntParam(
        JSONObject params,
        String snakeName,
        String camelName,
        int defaultValue) {
        if (params == null) {
            return defaultValue;
        }
        if (params.has(snakeName)) {
            return params.optInt(snakeName, defaultValue);
        }
        if (params.has(camelName)) {
            return params.optInt(camelName, defaultValue);
        }
        return defaultValue;
    }

    private JSONObject configureBreathAssessment(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = state.configureBreathAssessment(params);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "breath_assessment_configured", result);
    }

    private JSONObject resetBreathAssessment(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = state.resetBreathAssessment(params);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "breath_assessment_reset", result);
    }

    private JSONObject setPolarBreathParams(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = state.setPolarBreathParams(params);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "polar_breath_params_set", result);
    }

    private JSONObject beginPolarBreathCalibration(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = state.beginPolarBreathCalibration(params);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "polar_breath_calibration_started", result);
    }

    private JSONObject resetPolarBreathCalibration(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        JSONObject status = state.resetPolarBreathCalibration(params);
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("status", status);
        return commandAck(requestId, command, true, "polar_breath_calibration_reset", result);
    }

    private JSONObject submitControllerBreathPose(
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires controller-pose params.");
        }

        long receiveUnixNs = unixNowNs();
        long receiveElapsedNs = SystemClock.elapsedRealtimeNanos();
        long sequence = params.optLong("sequence_id", receiveElapsedNs);
        JSONObject processing = state.processControllerBreathPose(params, sequence, receiveUnixNs, receiveElapsedNs);
        if (!processing.optBoolean("accepted", false)) {
            state.rejectedCommands.incrementAndGet();
            return commandError(
                requestId,
                command,
                processing.optString("error_code", "controller_pose_rejected"),
                processing.optString("message", "Controller pose was not accepted for breath assessment."));
        }

        JSONObject assessment = processing.optJSONObject("assessment");
        int broadcasts = assessment != null
            ? broadcastStreamEvent(
                BreathAssessmentState.OUTPUT_STREAM,
                assessment.optLong("sequence_id", sequence),
                receiveUnixNs,
                assessment)
            : 0;
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("stream", BreathAssessmentState.OUTPUT_STREAM);
        result.put("sequence_id", sequence);
        result.put("broadcasts", broadcasts);
        result.put("breath_assessment", processing);
        return commandAck(requestId, command, true, "controller_pose_assessed", result);
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
        JSONObject result = publishLocalStreamEvent(
            stream,
            sequence,
            payload,
            connection != null ? connection.clientId : "");
        state.acceptedCommands.incrementAndGet();
        return commandAck(requestId, command, true, "stream_event_published", result);
    }

    private JSONObject recordBreathFeedbackReceipt(
        WebSocketClientConnection connection,
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires params.");
        }

        String receivedStream = params.optString("received_stream", "").trim();
        if (receivedStream.length() == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_received_stream", "Command requires params.received_stream.");
        }

        long receivedSequenceId = params.optLong("received_sequence_id", 0L);
        if (receivedSequenceId <= 0L) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_received_sequence_id", "Command requires a positive params.received_sequence_id.");
        }

        if (!params.optBoolean("acknowledged", false)) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "receipt_not_acknowledged", "Command requires params.acknowledged=true.");
        }

        String receiver = params.optString("receiver", "");
        if (receiver.length() == 0 && connection != null) {
            receiver = connection.clientId;
        }

        JSONObject receipt = new JSONObject();
        String schema = params.optString("schema", "rusty.manifold.breath.feedback_receipt.v1");
        if (schema.length() == 0) {
            schema = "rusty.manifold.breath.feedback_receipt.v1";
        }
        receipt.put("schema", schema);
        receipt.put("received_stream", receivedStream);
        receipt.put("received_sequence_id", receivedSequenceId);
        receipt.put("received_sample_time_unix_ns", params.optLong("received_sample_time_unix_ns", 0L));
        receipt.put("receiver", receiver);
        receipt.put("acknowledged", true);
        if (params.has("volume01")) {
            receipt.put("volume01", params.optDouble("volume01"));
        }
        if (params.has("phase")) {
            receipt.put("phase", params.optString("phase", ""));
        }
        if (params.has("quality")) {
            receipt.put("quality", params.optString("quality", ""));
        }
        if (params.has("payload_hash")) {
            receipt.put("payload_hash", params.optString("payload_hash", ""));
        }

        JSONObject result = publishLocalStreamEvent(
            "stream.breath.feedback_receipt",
            receivedSequenceId,
            receipt,
            connection != null ? connection.clientId : "");
        result.put("receipt", receipt);
        state.acceptedCommands.incrementAndGet();
        return commandAck(requestId, command, true, "breath_feedback_receipt_recorded", result);
    }

    JSONObject publishLocalStreamEvent(
        String stream,
        long sequence,
        JSONObject payload,
        String publisherClientId) throws Exception {
        JSONObject receiveStamp = state.clockStampJson("RelayReceive", SystemClock.elapsedRealtimeNanos(), "");
        long receiveUnixNs = receiveStamp.optLong("event_unix_ns", unixNowNs());
        long receiveElapsedNs = receiveStamp.optLong("event_elapsed_realtime_ns", SystemClock.elapsedRealtimeNanos());
        payload.put("publisher_client_id", publisherClientId != null ? publisherClientId : "");
        payload.put("broker_receive_time_unix_ns", receiveUnixNs);
        payload.put("broker_receive_time_elapsed_ns", receiveElapsedNs);
        payload.put("clock_stamp", receiveStamp);
        int broadcasts = broadcastStreamEvent(stream, sequence, receiveUnixNs, payload);
        JSONObject breathProcessing = state.processBreathAssessmentStreamEvent(
            stream,
            payload,
            sequence,
            receiveUnixNs,
            receiveElapsedNs);
        int breathBroadcasts = 0;
        if (breathProcessing != null && breathProcessing.optBoolean("accepted", false)) {
            JSONObject assessment = breathProcessing.optJSONObject("assessment");
            if (assessment != null) {
                breathBroadcasts = broadcastStreamEvent(
                    BreathAssessmentState.OUTPUT_STREAM,
                    assessment.optLong("sequence_id", sequence),
                    receiveUnixNs,
                    assessment);
            }
        }
        long accepted = state.publishedStreamEvents.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("stream", stream);
        result.put("sequence_id", sequence);
        result.put("published_count", accepted);
        result.put("broadcasts", broadcasts);
        if (breathProcessing != null) {
            result.put("breath_assessment", breathProcessing);
            result.put("breath_broadcasts", breathBroadcasts);
        }
        return result;
    }

    private JSONObject describeTransportCapabilities(String requestId, String command) throws Exception {
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("capabilities", state.transportCapabilitiesJson());
        return commandAck(requestId, command, true, "transport_capabilities", result);
    }

    private JSONObject createTransportSession(
        WebSocketClientConnection connection,
        String requestId,
        String command,
        JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires transport session params.");
        }

        String clientId = connection != null ? connection.clientId : "";
        JSONObject answer = state.createTransportSession(params, clientId);
        int broadcasts = broadcastStreamEvent(
            "transport.session_created",
            answer.optLong("created_elapsed_ns", SystemClock.elapsedRealtimeNanos()),
            unixNowNs(),
            new JSONObject(answer.toString()));
        state.acceptedCommands.incrementAndGet();

        JSONObject result = new JSONObject();
        result.put("answer", answer);
        result.put("broadcasts", broadcasts);
        return commandAck(requestId, command, true, "transport_session_created", result);
    }

    private JSONObject getTransportSession(String requestId, String command, JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires params.session_id.");
        }

        String sessionId = params.optString("session_id", "");
        if (sessionId.trim().length() == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_session_id", "Command requires params.session_id.");
        }

        JSONObject result = state.getTransportSession(sessionId);
        state.acceptedCommands.incrementAndGet();
        return commandAck(requestId, command, true, "transport_session", result);
    }

    private JSONObject listTransportSessions(String requestId, String command) throws Exception {
        state.acceptedCommands.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("registry", state.listTransportSessions());
        return commandAck(requestId, command, true, "transport_sessions", result);
    }

    private JSONObject closeTransportSession(String requestId, String command, JSONObject params) throws Exception {
        if (params == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_params", "Command requires params.session_id.");
        }

        String sessionId = params.optString("session_id", "");
        if (sessionId.trim().length() == 0) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_session_id", "Command requires params.session_id.");
        }

        JSONObject session = state.closeTransportSession(sessionId, params.optString("reason", ""));
        boolean found = session.optBoolean("found", true);
        int broadcasts = broadcastStreamEvent(
            found ? "transport.session_closed" : "transport.session_failed",
            SystemClock.elapsedRealtimeNanos(),
            unixNowNs(),
            new JSONObject(session.toString()));

        JSONObject result = new JSONObject();
        result.put("session", session);
        result.put("broadcasts", broadcasts);
        if (!found) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "unknown_session", "Transport session was not found.");
        }

        state.acceptedCommands.incrementAndGet();
        return commandAck(requestId, command, true, "transport_session_closed", result);
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
        String command,
        JSONObject params) throws Exception {
        if (context == null) {
            state.rejectedCommands.incrementAndGet();
            return commandError(requestId, command, "missing_context", "Broker context is not available.");
        }

        String page = params != null ? params.optString("page", "") : "";
        Intent intent = new Intent(context, MainActivity.class);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        intent.addFlags(Intent.FLAG_ACTIVITY_REORDER_TO_FRONT);
        intent.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP);
        intent.putExtra("rustyxr.openedByBrokerCommand", true);
        intent.putExtra("rustyxr.requestId", requestId != null ? requestId : "");
        intent.putExtra("rustyxr.clientId", connection != null ? connection.clientId : "");
        intent.putExtra("rustyxr.appPackage", connection != null ? connection.appPackage : "");
        if (page.length() > 0) {
            intent.putExtra(MainActivity.EXTRA_CONSOLE_PAGE, page);
        }
        context.startActivity(intent);

        state.acceptedCommands.incrementAndGet();
        long requests = state.brokerConsoleOpenRequests.incrementAndGet();
        JSONObject result = new JSONObject();
        result.put("activity", "broker_console");
        result.put("open_requests", requests);
        if (page.length() > 0) {
            result.put("requested_page", page);
        }
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
        ack.put("schema", BrokerState.MANIFOLD_COMMAND_ACK_SCHEMA);
        ack.put("legacy_schema", BrokerState.LEGACY_RUSTY_XR_BROKER_COMMAND_ACK_SCHEMA);
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
        error.put("schema", BrokerState.COMMAND_REJECTION_SCHEMA);
        error.put("code", code);
        error.put("message", message);
        error.put("retryable", false);

        return commandError(requestId, command, error);
    }

    private JSONObject commandError(
        String requestId,
        String command,
        BrokerState.CommandRejection rejection) throws Exception {
        return commandError(requestId, command, rejection.toErrorJson());
    }

    private JSONObject commandError(
        String requestId,
        String command,
        JSONObject error) throws Exception {
        JSONObject ack = new JSONObject();
        ack.put("type", "command_ack");
        ack.put("schema", BrokerState.MANIFOLD_COMMAND_ACK_SCHEMA);
        ack.put("legacy_schema", BrokerState.LEGACY_RUSTY_XR_BROKER_COMMAND_ACK_SCHEMA);
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
        status.put("bindAddress", bindHost);
        status.put("lanControlEnabled", !isLoopbackBindHost(bindHost));
        status.put("hostManifest", state.hostManifestJson(bindHost, port, publisher, oscIngressServer));
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

    private static boolean isLoopbackBindHost(String host) {
        if (host == null) {
            return false;
        }
        String normalized = host.trim().toLowerCase(Locale.US);
        return "127.0.0.1".equals(normalized) ||
            "localhost".equals(normalized) ||
            "::1".equals(normalized);
    }

    private static String safeMessage(Exception ex) {
        String message = ex.getMessage();
        return message != null ? message : "";
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
        JSONObject receiveStamp = state.clockStampJson("RelayReceive", SystemClock.elapsedRealtimeNanos(), "");
        long receiveUnixNs = receiveStamp.optLong("event_unix_ns", unixNowNs());
        long receiveElapsedNs = receiveStamp.optLong("event_elapsed_realtime_ns", SystemClock.elapsedRealtimeNanos());
        long sequence = message.optLong("sequence_id", -1L);
        String path = message.optString("path", "broker_lsl");
        if (path.length() == 0) {
            path = "broker_lsl";
        }

        message.put("type", "latency_sample");
        message.put("path", path);
        message.put("broker_receive_time_unix_ns", receiveUnixNs);
        message.put("broker_receive_time_elapsed_ns", receiveElapsedNs);
        message.put("clock_stamp", receiveStamp);
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
        ack.put("clock_stamp", receiveStamp);
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

    private static boolean isEventsWebSocketPath(String path) {
        return BrokerState.MANIFOLD_EVENTS_PATH.equals(path) ||
            BrokerState.LEGACY_RUSTY_XR_EVENTS_PATH.equals(path);
    }

    private JSONObject webSocketReadyJson(WebSocketClientConnection connection, String path) throws Exception {
        JSONObject ready = new JSONObject();
        ready.put("type", "websocket_ready");
        ready.put("connection_id", connection != null ? connection.connectionId : 0L);
        ready.put("path", stripQuery(path));
        ready.put("startup_mode", "compact");
        ready.put("broker_unix_ns", unixNowNs());
        ready.put("clock_stamp", state.clockStampJson());
        return ready;
    }

    private static String webSocketStartupMode(Map<String, String> headers, String path) {
        String headerMode = headers.get("x-rusty-websocket-startup");
        String mode = normalizeWebSocketStartupMode(headerMode);
        if (mode != null) {
            return mode;
        }

        int queryIndex = path.indexOf('?');
        if (queryIndex < 0 || queryIndex >= path.length() - 1) {
            return "status";
        }

        String[] parameters = path.substring(queryIndex + 1).split("&");
        for (String parameter : parameters) {
            String[] parts = parameter.split("=", 2);
            String name = parts.length > 0 ? parts[0] : "";
            String value = parts.length > 1 ? parts[1] : "";
            if ("startup".equalsIgnoreCase(name) || "initial".equalsIgnoreCase(name)) {
                mode = normalizeWebSocketStartupMode(value);
                if (mode != null) {
                    return mode;
                }
            } else if ("no_initial_status".equalsIgnoreCase(name) || "no-startup-status".equalsIgnoreCase(name)) {
                return "none";
            }
        }

        return "status";
    }

    private static String normalizeWebSocketStartupMode(String value) {
        if (value == null) {
            return null;
        }
        String normalized = value.trim().toLowerCase(Locale.ROOT);
        if ("none".equals(normalized) || "off".equals(normalized) || "false".equals(normalized)) {
            return "none";
        }
        if ("compact".equals(normalized) || "ready".equals(normalized)) {
            return "compact";
        }
        if ("status".equals(normalized) || "full".equals(normalized) || "true".equals(normalized)) {
            return "status";
        }
        return null;
    }

    private static String stripQuery(String path) {
        int queryIndex = path.indexOf('?');
        return queryIndex >= 0 ? path.substring(0, queryIndex) : path;
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
