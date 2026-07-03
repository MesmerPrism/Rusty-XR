package com.example.rustyxr.broker;

import org.json.JSONObject;

interface LatencyPublisher {
    String mode();

    boolean isLslAvailable();

    default boolean isOscAvailable() {
        return false;
    }

    String blocker();

    default String lslStreamName() {
        return NativeLslLatencyPublisher.STREAM_NAME;
    }

    default String lslStreamType() {
        return NativeLslLatencyPublisher.STREAM_TYPE;
    }

    default String lslSourceId() {
        return NativeLslLatencyPublisher.SOURCE_ID;
    }

    default JSONObject oscStatus() throws Exception {
        JSONObject status = new JSONObject();
        status.put("enabled", false);
        return status;
    }

    void publish(JSONObject payload);

    default void close() {
    }
}
