package com.example.rustyxr.broker;

import android.util.Log;

import org.json.JSONObject;

final class NativeLslLatencyPublisher implements LatencyPublisher {
    static final String STREAM_NAME = "rusty_xr_broker_latency";
    static final String STREAM_TYPE = "rusty.xr.latency";
    private static final String SOURCE_ID = "rusty.xr.broker.latency";

    private final long outletHandle;
    private final String startupError;

    private NativeLslLatencyPublisher(long outletHandle, String startupError) {
        this.outletHandle = outletHandle;
        this.startupError = startupError != null ? startupError : "";
    }

    static LatencyPublisher createOrFallback() {
        try {
            System.loadLibrary("lsl");
            System.loadLibrary("rustyxr_broker_lsl_jni");
            long handle = nativeCreateOutlet(STREAM_NAME, STREAM_TYPE, SOURCE_ID, 8);
            if (handle == 0L) {
                String error = nativeLastError();
                Log.w(BrokerService.TAG, "Native LSL outlet creation failed: " + error);
                return new LogcatLatencyPublisher("native-lsl-create-failed: " + error);
            }

            Log.i(BrokerService.TAG, "Native LSL outlet ready: " + STREAM_NAME);
            return new NativeLslLatencyPublisher(handle, "");
        } catch (Throwable ex) {
            String error = ex.getClass().getSimpleName() + ": " + ex.getMessage();
            Log.w(BrokerService.TAG, "Native LSL unavailable: " + error);
            return new LogcatLatencyPublisher("native-lsl-unavailable: " + error);
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
