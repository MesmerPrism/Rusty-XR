package com.example.rustyxr.broker;

import org.json.JSONObject;

final class CompositeLatencyPublisher implements LatencyPublisher {
    private final LatencyPublisher lslPublisher;
    private final OscUdpLatencyPublisher oscPublisher;

    private CompositeLatencyPublisher(LatencyPublisher lslPublisher, OscUdpLatencyPublisher oscPublisher) {
        this.lslPublisher = lslPublisher;
        this.oscPublisher = oscPublisher;
    }

    static LatencyPublisher create(BrokerRuntimeConfig config) {
        LatencyPublisher lsl = NativeLslLatencyPublisher.createOrFallback(config);
        OscUdpLatencyPublisher osc = OscUdpLatencyPublisher.createOrNull(config);
        if (osc == null) {
            return lsl;
        }

        return new CompositeLatencyPublisher(lsl, osc);
    }

    @Override
    public String mode() {
        return lslPublisher.mode() + "+osc-udp";
    }

    @Override
    public boolean isLslAvailable() {
        return lslPublisher.isLslAvailable();
    }

    @Override
    public boolean isOscAvailable() {
        return oscPublisher != null && oscPublisher.isOscAvailable();
    }

    @Override
    public String blocker() {
        return lslPublisher.blocker();
    }

    @Override
    public String lslStreamName() {
        return lslPublisher.lslStreamName();
    }

    @Override
    public String lslStreamType() {
        return lslPublisher.lslStreamType();
    }

    @Override
    public String lslSourceId() {
        return lslPublisher.lslSourceId();
    }

    @Override
    public JSONObject oscStatus() throws Exception {
        if (oscPublisher == null) {
            return LatencyPublisher.super.oscStatus();
        }

        return oscPublisher.oscStatus();
    }

    @Override
    public void publish(JSONObject payload) {
        lslPublisher.publish(payload);
        if (oscPublisher != null) {
            oscPublisher.publish(payload);
        }
    }

    @Override
    public void close() {
        lslPublisher.close();
        if (oscPublisher != null) {
            oscPublisher.close();
        }
    }
}
