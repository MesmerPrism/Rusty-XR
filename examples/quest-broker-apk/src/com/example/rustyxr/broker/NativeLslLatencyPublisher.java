package com.example.rustyxr.broker;

import android.util.Log;

import org.json.JSONObject;

final class NativeLslLatencyPublisher implements LatencyPublisher {
    static final String STREAM_NAME = "rusty_xr_broker_latency";
    static final String STREAM_TYPE = "rusty.xr.latency";
    static final String SOURCE_ID = "rusty.xr.broker.latency";

    private final long outletHandle;
    private final String startupError;
    private final String streamName;
    private final String streamType;
    private final String sourceId;

    private NativeLslLatencyPublisher(
        long outletHandle,
        String startupError,
        String streamName,
        String streamType,
        String sourceId) {
        this.outletHandle = outletHandle;
        this.startupError = startupError != null ? startupError : "";
        this.streamName = streamName;
        this.streamType = streamType;
        this.sourceId = sourceId;
    }

    static LatencyPublisher createOrFallback(BrokerRuntimeConfig config) {
        String streamName = config != null ? config.lslStreamName : STREAM_NAME;
        String streamType = config != null ? config.lslStreamType : STREAM_TYPE;
        String sourceId = config != null ? config.lslSourceId : SOURCE_ID;
        try {
            System.loadLibrary("lsl");
            System.loadLibrary("rustyxr_broker_lsl_jni");
            long handle = nativeCreateOutlet(streamName, streamType, sourceId, 8);
            if (handle == 0L) {
                String error = nativeLastError();
                Log.w(BrokerService.TAG, "Native LSL outlet creation failed: " + error);
                return new LogcatLatencyPublisher("native-lsl-create-failed: " + error, streamName, streamType, sourceId);
            }

            Log.i(BrokerService.TAG, "Native LSL outlet ready: " + streamName + " source_id=" + sourceId);
            return new NativeLslLatencyPublisher(handle, "", streamName, streamType, sourceId);
        } catch (Throwable ex) {
            String error = ex.getClass().getSimpleName() + ": " + ex.getMessage();
            Log.w(BrokerService.TAG, "Native LSL unavailable: " + error);
            return new LogcatLatencyPublisher("native-lsl-unavailable: " + error, streamName, streamType, sourceId);
        }
    }

    @Override
    public String mode() {
        return "native-lsl";
    }

    @Override
    public boolean isLslAvailable() {
        return outletHandle != 0L;
    }

    @Override
    public String blocker() {
        return startupError;
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
        if (outletHandle == 0L || payload == null) {
            return;
        }

        int result = nativePushStringSample(outletHandle, payload.toString());
        if (result < 0) {
            Log.w(BrokerService.TAG, "Native LSL publish failed result=" + result + " error=" + nativeLastError());
        }
    }

    @Override
    public void close() {
        if (outletHandle != 0L) {
            nativeDestroyOutlet(outletHandle);
        }
    }

    private static native long nativeCreateOutlet(String name, String type, String sourceId, int maxBufferedSeconds);

    private static native int nativePushStringSample(long outletHandle, String payload);

    private static native void nativeDestroyOutlet(long outletHandle);

    private static native String nativeLastError();
}
