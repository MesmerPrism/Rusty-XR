package com.example.rustyxr.broker;

import android.os.SystemClock;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.concurrent.atomic.AtomicLong;

final class BrokerState {
    static final String BROKER_VERSION = "0.1.0-public-proof";
    static final int PROTOCOL_VERSION = 1;
    static final String CONTRACT_VERSION = "rusty.xr.broker.v1";

    final long startedElapsedNanos = SystemClock.elapsedRealtimeNanos();
    final long startedUnixMs = System.currentTimeMillis();
    final AtomicLong httpStatusRequests = new AtomicLong();
    final AtomicLong websocketConnections = new AtomicLong();
    final AtomicLong acceptedLatencySamples = new AtomicLong();
    final AtomicLong rejectedMessages = new AtomicLong();
    final AtomicLong acceptedCommands = new AtomicLong();
    final AtomicLong rejectedCommands = new AtomicLong();
    final AtomicLong brokerConsoleOpenRequests = new AtomicLong();
    final AtomicLong brokerConsoleCloseRequests = new AtomicLong();
    final AtomicLong oscIngressPackets = new AtomicLong();
    final AtomicLong oscIngressRejectedPackets = new AtomicLong();
    final AtomicLong oscIngressBroadcasts = new AtomicLong();
    final AtomicLong publishedStreamEvents = new AtomicLong();

    JSONObject toStatusJson(LatencyPublisher publisher, OscIngressServer oscIngressServer) throws Exception {
        JSONObject status = new JSONObject();
        status.put("type", "status");
        status.put("brokerVersion", BROKER_VERSION);
        status.put("protocolVersion", PROTOCOL_VERSION);
        status.put("contractVersion", CONTRACT_VERSION);
        status.put("uptimeMs", (SystemClock.elapsedRealtimeNanos() - startedElapsedNanos) / 1_000_000L);
        status.put("startedUnixMs", startedUnixMs);
        status.put("bindAddress", "127.0.0.1");
        status.put("port", BrokerService.DEFAULT_PORT);

        JSONArray capabilities = capabilitiesJson(publisher, oscIngressServer);
        status.put("capabilities", capabilities);
        status.put("streams", streamsJson(oscIngressServer));

        JSONObject commands = new JSONObject();
        commands.put("schema", "rusty.xr.broker.command.v1");
        commands.put("ackSchema", "rusty.xr.broker.command_ack.v1");
        JSONArray supportedCommands = new JSONArray();
        supportedCommands.put("status_request");
        supportedCommands.put("list_capabilities");
        supportedCommands.put("list_streams");
        supportedCommands.put("subscribe");
        supportedCommands.put("unsubscribe");
        supportedCommands.put("configure_osc_ingress");
        supportedCommands.put("publish_stream_event");
        supportedCommands.put("open_ui");
        supportedCommands.put("close_ui");
        commands.put("supported", supportedCommands);
        status.put("commands", commands);

        JSONObject counters = new JSONObject();
        counters.put("httpStatusRequests", httpStatusRequests.get());
        counters.put("websocketConnections", websocketConnections.get());
        counters.put("acceptedLatencySamples", acceptedLatencySamples.get());
        counters.put("rejectedMessages", rejectedMessages.get());
        counters.put("acceptedCommands", acceptedCommands.get());
        counters.put("rejectedCommands", rejectedCommands.get());
        counters.put("brokerConsoleOpenRequests", brokerConsoleOpenRequests.get());
        counters.put("brokerConsoleCloseRequests", brokerConsoleCloseRequests.get());
        counters.put("oscIngressPackets", oscIngressPackets.get());
        counters.put("oscIngressRejectedPackets", oscIngressRejectedPackets.get());
        counters.put("oscIngressBroadcasts", oscIngressBroadcasts.get());
        counters.put("publishedStreamEvents", publishedStreamEvents.get());
        status.put("counters", counters);

        JSONObject lsl = new JSONObject();
        lsl.put("enabled", publisher != null && publisher.isLslAvailable());
        lsl.put("publisher", publisher != null ? publisher.mode() : "none");
        lsl.put("streamName", NativeLslLatencyPublisher.STREAM_NAME);
        lsl.put("streamType", NativeLslLatencyPublisher.STREAM_TYPE);
        if (publisher != null && publisher.blocker() != null && publisher.blocker().length() > 0) {
            lsl.put("blocker", publisher.blocker());
        }
        status.put("lsl", lsl);

        JSONObject osc = new JSONObject();
        osc.put("egress", publisher != null ? publisher.oscStatus() : new JSONObject().put("enabled", false));
        osc.put("ingress", oscIngressServer != null ? oscIngressServer.toStatusJson() : new JSONObject().put("enabled", false));
        status.put("osc", osc);
        return status;
    }

    JSONArray capabilitiesJson(LatencyPublisher publisher, OscIngressServer oscIngressServer) {
        JSONArray capabilities = new JSONArray();
        capabilities.put("websocket.events");
        capabilities.put("websocket.control");
        capabilities.put("http.status");
        capabilities.put("broker.command.v1");
        capabilities.put("broker.subscription.v1");
        capabilities.put("broker.stream_event.v1");
        capabilities.put("broker.osc_ingress.configure");
        capabilities.put("broker.stream_event.publish");
        capabilities.put("broker.console.activity");
        capabilities.put("broker.console.return_to_previous_app");
        capabilities.put("broker.console.close_command");
        capabilities.put("latency.sample.accept");
        capabilities.put("logcat.diagnostics");
        if (publisher != null && publisher.isLslAvailable()) {
            capabilities.put("lsl.gateway");
        } else {
            capabilities.put("lsl.gateway.pending_native_binding");
        }
        if (publisher != null && publisher.isOscAvailable()) {
            capabilities.put("osc.udp.send");
        }
        if (oscIngressServer != null && oscIngressServer.isRunning()) {
            capabilities.put("osc.udp.receive");
            capabilities.put("osc.drive.websocket.broadcast");
        }
        return capabilities;
    }

    JSONArray streamsJson(OscIngressServer oscIngressServer) throws Exception {
        JSONArray streams = new JSONArray();
        streams.put(streamJson("broker:status", "status", "Broker status snapshots and capability reports.", true));
        streams.put(streamJson("latency:sample", "latency", "WebSocket latency samples accepted by the broker.", true));
        streams.put(streamJson("bio:polar_hr_rr", "bio", "Synthetic or adapter-published Polar-compatible heart-rate/RR events.", true));
        streams.put(streamJson("bio:polar_ecg", "bio", "Synthetic or adapter-published Polar-compatible ECG frame events.", true));
        streams.put(streamJson("bio:polar_acc", "bio", "Synthetic or adapter-published Polar-compatible accelerometer frame events.", true));

        if (oscIngressServer != null && oscIngressServer.isRunning()) {
            streams.put(streamJson(
                oscIngressServer.streamId(),
                "osc",
                "Accepted OSC ingress drive values normalized to 0..1.",
                true));
        } else {
            streams.put(streamJson("osc:/rusty-xr/drive/radius", "osc", "OSC ingress drive values when enabled.", false));
        }

        return streams;
    }

    private static JSONObject streamJson(String id, String kind, String description, boolean active) throws Exception {
        JSONObject stream = new JSONObject();
        stream.put("id", id);
        stream.put("kind", kind);
        stream.put("description", description);
        stream.put("active", active);
        return stream;
    }
}
