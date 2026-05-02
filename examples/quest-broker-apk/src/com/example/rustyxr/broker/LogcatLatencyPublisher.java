package com.example.rustyxr.broker;

import android.util.Log;

import org.json.JSONObject;

final class LogcatLatencyPublisher implements LatencyPublisher {
    private final String blocker;

    LogcatLatencyPublisher() {
        this("native-lsl-unavailable: samples are accepted and logged but not emitted as LSL.");
    }

    LogcatLatencyPublisher(String blocker) {
        this.blocker = blocker != null ? blocker : "";
    }

    @Override
    public String mode() {
        return "logcat-fallback";
    }

    @Override
    public boolean isLslAvailable() {
        return false;
    }

    @Override
    public String blocker() {
        return blocker;
    }

    @Override
    public void publish(JSONObject payload) {
        Log.i(BrokerService.TAG, "latency_sample " + payload.toString());
    }
}
