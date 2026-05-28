package com.example.rustyxr.broker;

import android.os.SystemClock;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import java.nio.charset.StandardCharsets;
import java.util.Locale;

final class NativeLslStringInletDiagnostics {
    private static volatile boolean loadAttempted;
    private static volatile boolean nativeAvailable;
    private static volatile String loadError = "";

    private NativeLslStringInletDiagnostics() {
    }

    static JSONObject capture(JSONObject params) throws Exception {
        JSONObject normalized = normalize(params);
        JSONObject result = new JSONObject();
        result.put("schema", "rusty.xr.lsl.string_capture.v1");
        result.put("resolve_property", normalized.optString("resolve_property"));
        result.put("resolve_value", normalized.optString("resolve_value"));
        result.put("required_type", normalized.optString("required_type"));
        result.put("required_stream", normalized.optString("required_stream"));
        result.put("duration_ms", normalized.optInt("duration_ms"));
        result.put("max_samples", normalized.optInt("max_samples"));
        result.put("capture_elapsed_start_ns", SystemClock.elapsedRealtimeNanos());

        if (!ensureNativeAvailable()) {
            result.put("lsl_available", false);
            result.put("error", loadError);
            result.put("samples", new JSONArray());
            return result;
        }

        result.put("lsl_available", true);
        long handle = nativeResolveStringInlet(
            normalized.optString("resolve_property"),
            normalized.optString("resolve_value"),
            normalized.optInt("resolve_timeout_ms") / 1000.0);
        if (handle == 0L) {
            result.put("error", nativeLastError());
            result.put("samples", new JSONArray());
            return result;
        }

        try {
            int openError = nativeOpenStream(handle, normalized.optInt("pull_timeout_ms") / 1000.0);
            if (openError != 0) {
                result.put("error", nativeLastError());
                result.put("samples", new JSONArray());
                return result;
            }

            double[] correction = nativeTimeCorrection(handle, normalized.optInt("pull_timeout_ms") / 1000.0);
            JSONObject correctionJson = new JSONObject();
            double correctionOffset = correction != null && correction.length > 0 ? correction[0] : 0.0;
            double correctionRemote = correction != null && correction.length > 1 ? correction[1] : 0.0;
            double correctionUncertainty = correction != null && correction.length > 2 ? correction[2] : 0.0;
            int correctionError = correction != null && correction.length > 3 ? (int) correction[3] : 0;
            correctionJson.put("offset_seconds", correctionOffset);
            correctionJson.put("remote_time_seconds", correctionRemote);
            correctionJson.put("uncertainty_seconds", correctionUncertainty);
            correctionJson.put("error_code", correctionError);
            result.put("time_correction", correctionJson);
            if (correctionError != 0) {
                result.put("time_correction_error", nativeLastError());
            }

            int warmupMs = normalized.optInt("warmup_ms");
            if (warmupMs > 0) {
                Thread.sleep(warmupMs);
            }

            JSONArray samples = new JSONArray();
            long deadline = SystemClock.elapsedRealtime() + normalized.optInt("duration_ms");
            int maxSamples = normalized.optInt("max_samples");
            int pullTimeoutMs = normalized.optInt("pull_timeout_ms");
            String requiredType = normalized.optString("required_type");
            String requiredStream = normalized.optString("required_stream");
            while (samples.length() < maxSamples && SystemClock.elapsedRealtime() < deadline) {
                long remainingMs = Math.max(1L, deadline - SystemClock.elapsedRealtime());
                String payload = nativePullStringSample(handle, Math.min(pullTimeoutMs, (int) remainingMs) / 1000.0);
                if (payload == null || payload.length() == 0) {
                    continue;
                }

                JSONObject parsed = tryParse(payload);
                String type = parsed != null ? parsed.optString("type", "") : "";
                String stream = resolveStream(parsed);
                if (!matches(requiredType, type) || !matches(requiredStream, stream)) {
                    continue;
                }

                double sampleTimestamp = nativeLastSampleTimestamp(handle);
                double receiveClock = nativeLocalClock();
                long receiveUnixNs = unixTimeNs();
                long receiveElapsedNs = SystemClock.elapsedRealtimeNanos();
                JSONObject sample = new JSONObject();
                sample.put("index", samples.length() + 1);
                sample.put("lsl_sample_timestamp_seconds", sampleTimestamp);
                sample.put("quest_receive_lsl_clock_seconds", receiveClock);
                sample.put("quest_receive_unix_ns", receiveUnixNs);
                sample.put("quest_receive_elapsed_ns", receiveElapsedNs);
                sample.put("lsl_corrected_sample_to_receive_ms", (receiveClock - (sampleTimestamp + correctionOffset)) * 1000.0);
                sample.put("time_correction_offset_ms", correctionOffset * 1000.0);
                sample.put("time_correction_uncertainty_ms", correctionUncertainty * 1000.0);
                sample.put("type", type);
                sample.put("stream", stream);
                putNullableLong(sample, "sequence_id", optLongObject(parsed, "sequence_id"));
                putNullableLong(sample, "broker_time_unix_ns", optLongObject(parsed, "broker_time_unix_ns"));
                putNullableLong(sample, "broker_time_elapsed_ns", optLongObject(parsed, "broker_time_elapsed_ns"));
                putNullableLong(sample, "source_sample_unix_ns", firstLong(parsed, "source_time_unix_ns", "payload", "sample_time_unix_ns"));
                putNullableLong(sample, "source_sample_elapsed_ns", nestedLong(parsed, "payload", "sample_time_elapsed_ns"));
                putNullableLong(sample, "broker_receive_unix_ns", nestedLong(parsed, "payload", "broker_receive_time_unix_ns"));
                putNullableLong(sample, "broker_receive_elapsed_ns", nestedLong(parsed, "payload", "broker_receive_time_elapsed_ns"));
                putNullableLong(sample, "sensor_timestamp_ns", firstLong(parsed, "source_sensor_timestamp_ns", "payload", "sensor_timestamp_ns"));
                putNullableLong(sample, "sample_count", firstLong(parsed, "source_sample_count", "payload", "sample_count"));
                sample.put("payload_schema", nestedString(parsed, "payload", "schema"));
                sample.put("payload_size_bytes", payload.getBytes(StandardCharsets.UTF_8).length);
                sample.put("payload", payload);
                samples.put(sample);
            }

            result.put("samples", samples);
            result.put("sample_count", samples.length());
            result.put("capture_elapsed_end_ns", SystemClock.elapsedRealtimeNanos());
            return result;
        } finally {
            nativeDestroyInlet(handle);
        }
    }

    private static JSONObject normalize(JSONObject params) throws Exception {
        JSONObject normalized = new JSONObject();
        normalized.put("resolve_property", optString(params, "resolve_property", "name"));
        normalized.put("resolve_value", optString(params, "resolve_value", "rusty_xr_polar_windows_bridge"));
        normalized.put("required_type", optString(params, "required_type", ""));
        normalized.put("required_stream", optString(params, "required_stream", ""));
        normalized.put("duration_ms", clamp(optInt(params, "duration_ms", 15000), 100, 300000));
        normalized.put("max_samples", clamp(optInt(params, "max_samples", 512), 1, 100000));
        normalized.put("resolve_timeout_ms", clamp(optInt(params, "resolve_timeout_ms", 10000), 100, 60000));
        normalized.put("pull_timeout_ms", clamp(optInt(params, "pull_timeout_ms", 5000), 100, 60000));
        normalized.put("warmup_ms", clamp(optInt(params, "warmup_ms", 0), 0, 60000));
        return normalized;
    }

    private static boolean ensureNativeAvailable() {
        if (loadAttempted) {
            return nativeAvailable;
        }

        synchronized (NativeLslStringInletDiagnostics.class) {
            if (loadAttempted) {
                return nativeAvailable;
            }

            try {
                System.loadLibrary("lsl");
                System.loadLibrary("rustyxr_broker_lsl_jni");
                nativeAvailable = true;
                loadError = "";
            } catch (Throwable ex) {
                nativeAvailable = false;
                loadError = ex.getClass().getSimpleName() + ": " + ex.getMessage();
                Log.w(BrokerService.TAG, "Native LSL inlet unavailable: " + loadError);
            } finally {
                loadAttempted = true;
            }
        }

        return nativeAvailable;
    }

    private static JSONObject tryParse(String payload) {
        try {
            return new JSONObject(payload);
        } catch (Exception ignored) {
            return null;
        }
    }

    private static String resolveStream(JSONObject parsed) {
        if (parsed == null) {
            return "";
        }

        String stream = parsed.optString("stream", "");
        if (stream.length() > 0) {
            return stream;
        }

        JSONObject payload = parsed.optJSONObject("payload");
        return payload != null ? payload.optString("stream_id", "") : "";
    }

    private static boolean matches(String required, String actual) {
        return required == null ||
            required.trim().length() == 0 ||
            required.trim().equals(actual != null ? actual : "");
    }

    private static Long firstLong(JSONObject root, String rootName, String nestedName, String nestedProperty) {
        Long rootValue = optLongObject(root, rootName);
        return rootValue != null ? rootValue : nestedLong(root, nestedName, nestedProperty);
    }

    private static Long nestedLong(JSONObject root, String objectName, String propertyName) {
        if (root == null) {
            return null;
        }

        JSONObject nested = root.optJSONObject(objectName);
        return optLongObject(nested, propertyName);
    }

    private static String nestedString(JSONObject root, String objectName, String propertyName) {
        if (root == null) {
            return "";
        }

        JSONObject nested = root.optJSONObject(objectName);
        return nested != null ? nested.optString(propertyName, "") : "";
    }

    private static Long optLongObject(JSONObject object, String propertyName) {
        if (object == null || !object.has(propertyName) || object.isNull(propertyName)) {
            return null;
        }

        Object value = object.opt(propertyName);
        if (value instanceof Number) {
            return Long.valueOf(((Number) value).longValue());
        }

        if (value instanceof String) {
            try {
                return Long.valueOf(Long.parseLong(((String) value).trim()));
            } catch (NumberFormatException ignored) {
                return null;
            }
        }

        return null;
    }

    private static void putNullableLong(JSONObject object, String key, Long value) throws Exception {
        object.put(key, value != null ? value : JSONObject.NULL);
    }

    private static String optString(JSONObject params, String key, String defaultValue) {
        if (params == null) {
            return defaultValue;
        }

        String value = params.optString(key, defaultValue);
        return value != null && value.trim().length() > 0 ? value.trim() : defaultValue;
    }

    private static int optInt(JSONObject params, String key, int defaultValue) {
        return params != null ? params.optInt(key, defaultValue) : defaultValue;
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static long unixTimeNs() {
        return System.currentTimeMillis() * 1000000L;
    }

    private static native long nativeResolveStringInlet(String property, String value, double timeoutSeconds);

    private static native int nativeOpenStream(long inletHandle, double timeoutSeconds);

    private static native double[] nativeTimeCorrection(long inletHandle, double timeoutSeconds);

    private static native String nativePullStringSample(long inletHandle, double timeoutSeconds);

    private static native double nativeLastSampleTimestamp(long inletHandle);

    private static native double nativeLocalClock();

    private static native void nativeDestroyInlet(long inletHandle);

    private static native String nativeLastError();
}
