package com.example.rustyxr.broker;

import android.util.Log;

import org.json.JSONObject;

final class LogcatLatencyPublisher implements LatencyPublisher {
    private final String blocker;
    private final String streamName;
    private final String streamType;
    private final String sourceId;

    LogcatLatencyPublisher() {
        this("native-lsl-unavailable: samples are accepted and logged but not emitted as LSL.");
    }

    LogcatLatencyPublisher(String blocker) {
        this(
            blocker,
            NativeLslLatencyPublisher.STREAM_NAME,
            NativeLslLatencyPublisher.STREAM_TYPE,
            NativeLslLatencyPublisher.SOURCE_ID);
    }

    LogcatLatencyPublisher(String blocker, String streamName, String streamType, String sourceId) {
        this.blocker = blocker != null ? blocker : "";
        this.streamName = streamName != null ? streamName : NativeLslLatencyPublisher.STREAM_NAME;
        this.streamType = streamType != null ? streamType : NativeLslLatencyPublisher.STREAM_TYPE;
        this.sourceId = sourceId != null ? sourceId : NativeLslLatencyPublisher.SOURCE_ID;
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
    public String lslStreamName() {
        return streamName;
    }

    @Override
    public String lslStreamType() {
        return streamType;
    }

    @Override
    public String lslSourceId() {
        return sourceId;
    }

    @Override
    public void publish(JSONObject payload) {
        Log.i(BrokerService.TAG, "latency_sample " + payload.toString());
    }
}
