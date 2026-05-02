package com.example.rustyxr.broker;

import android.os.SystemClock;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.concurrent.atomic.AtomicLong;

final class BrokerState {
    static final String BROKER_VERSION = "0.1.0-public-proof";
    static final int PROTOCOL_VERSION = 1;
    static final String CONTRACT_VERSION = "rusty.xr.broker.latency.v1";

    final long startedElapsedNanos = SystemClock.elapsedRealtimeNanos();
    final long startedUnixMs = System.currentTimeMillis();
    final AtomicLong httpStatusRequests = new AtomicLong();
    final AtomicLong websocketConnections = new AtomicLong();
    final AtomicLong acceptedLatencySamples = new AtomicLong();
    final AtomicLong rejectedMessages = new AtomicLong();
    final AtomicLong oscIngressPackets = new AtomicLong();
    final AtomicLong oscIngressRejectedPackets = new AtomicLong();
    final AtomicLong oscIngressBroadcasts = new AtomicLong();

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

        JSONArray capabilities = new JSONArray();
        capabilities.put("websocket.events");
        capabilities.put("http.status");
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
        status.put("capabilities", capabilities);

        JSONObject counters = new JSONObject();
        counters.put("httpStatusRequests", httpStatusRequests.get());
        counters.put("websocketConnections", websocketConnections.get());
        counters.put("acceptedLatencySamples", acceptedLatencySamples.get());
        counters.put("rejectedMessages", rejectedMessages.get());
        counters.put("oscIngressPackets", oscIngressPackets.get());
        counters.put("oscIngressRejectedPackets", oscIngressRejectedPackets.get());
        counters.put("oscIngressBroadcasts", oscIngressBroadcasts.get());
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
}
