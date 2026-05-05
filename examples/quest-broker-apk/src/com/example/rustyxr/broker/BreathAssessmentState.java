package com.example.rustyxr.broker;

import android.os.SystemClock;
import android.util.Base64;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

final class BreathAssessmentState {
    static final String STATUS_SCHEMA = "rusty.xr.bio.breath_assessment.status.v1";
    static final String BREATH_SCHEMA = "rusty.xr.bio.breath.v1";
    static final String SOURCE_STATUS_SCHEMA = "rusty.xr.bio.breath_source.status.v1";
    static final String OUTPUT_STREAM = "bio:breath";
    static final String POLAR_INPUT_STREAM = "bio:polar_acc";
    static final String CONTROLLER_INPUT_STREAM = "xr:controller_pose";
    static final String POLAR_SOURCE = "polar_acc";
    static final String CONTROLLER_SOURCE = "controller_pose";

    private final ProjectionBreathTracker polarTracker = ProjectionBreathTracker.createPolar();
    private final ProjectionBreathTracker controllerTracker = ProjectionBreathTracker.createController();
    private long revision = 1L;
    private long polarPublishedFrames;
    private long polarAcceptedFrames;
    private long controllerSubmittedSamples;
    private long controllerAcceptedSamples;
    private long rejectedSamples;
    private long emittedAssessments;
    private long latestAssessmentUnixNs;
    private JSONObject latestAssessment = new JSONObject();

    synchronized JSONObject toStatusJson() throws Exception {
        JSONObject status = new JSONObject();
        status.put("schema", STATUS_SCHEMA);
        status.put("state", overallState());
        status.put("revision", revision);
        status.put("enabled", true);
        status.put("output_stream", OUTPUT_STREAM);
        status.put("supported_sources", arrayOf(POLAR_SOURCE, CONTROLLER_SOURCE));
        status.put("input_streams", arrayOf(POLAR_INPUT_STREAM, CONTROLLER_INPUT_STREAM));
        status.put("accepted_polar_frames", polarAcceptedFrames);
        status.put("accepted_controller_samples", controllerAcceptedSamples);
        status.put("emitted_assessments", emittedAssessments);
        status.put("rejected_samples", rejectedSamples);
        status.put("latest_assessment_unix_ns", latestAssessmentUnixNs);
        if (latestAssessment.length() > 0) {
            status.put("latest_assessment", copyObject(latestAssessment));
        }

        JSONObject sources = new JSONObject();
        sources.put(POLAR_SOURCE, polarTracker.toStatusJson());
        sources.put(CONTROLLER_SOURCE, controllerTracker.toStatusJson());
        status.put("sources", sources);

        JSONObject payloadSchemas = new JSONObject();
        payloadSchemas.put("status", STATUS_SCHEMA);
        payloadSchemas.put("assessment", BREATH_SCHEMA);
        payloadSchemas.put("source_status", SOURCE_STATUS_SCHEMA);
        status.put("payload_schemas", payloadSchemas);

        JSONArray limitations = new JSONArray();
        limitations.put("diagnostic_breath_estimate_not_medical");
        limitations.put("polar_acc_requires_direct_ble_or_adapter_published_accelerometer_frames");
        limitations.put("controller_pose_requires_thin_adapter");
        limitations.put("volume_direction_may_need_per-app_inversion");
        status.put("limitations", limitations);
        return status;
    }

    synchronized boolean hasAssessments() {
        return emittedAssessments > 0;
    }

    synchronized JSONObject configure(JSONObject params) throws Exception {
        if (params == null) {
            return toStatusJson();
        }

        boolean changed = false;
        String source = normalizeSource(params.optString("source", "all"));
        if (appliesTo(source, POLAR_SOURCE)) {
            changed = polarTracker.configure(params) || changed;
            JSONObject nested = params.optJSONObject(POLAR_SOURCE);
            if (nested != null) {
                changed = polarTracker.configure(nested) || changed;
            }
        }
        if (appliesTo(source, CONTROLLER_SOURCE)) {
            changed = controllerTracker.configure(params) || changed;
            JSONObject nested = params.optJSONObject(CONTROLLER_SOURCE);
            if (nested != null) {
                changed = controllerTracker.configure(nested) || changed;
            }
        }

        if (changed) {
            revision++;
        }
        return toStatusJson();
    }

    synchronized JSONObject reset(JSONObject params) throws Exception {
        String source = normalizeSource(params != null ? params.optString("source", "all") : "all");
        boolean clearCounters = params != null && params.optBoolean("clear_counters", false);
        if (appliesTo(source, POLAR_SOURCE)) {
            polarTracker.resetCalibration();
        }
        if (appliesTo(source, CONTROLLER_SOURCE)) {
            controllerTracker.resetCalibration();
        }
        if (clearCounters) {
            polarPublishedFrames = 0L;
            polarAcceptedFrames = 0L;
            controllerSubmittedSamples = 0L;
            controllerAcceptedSamples = 0L;
            rejectedSamples = 0L;
            emittedAssessments = 0L;
        }
        latestAssessment = new JSONObject();
        latestAssessmentUnixNs = 0L;
        revision++;
        return toStatusJson();
    }

    synchronized JSONObject processPublishedStreamEvent(
        String stream,
        JSONObject payload,
        long sequence,
        long receiveUnixNs,
        long receiveElapsedNs) throws Exception {
        if (POLAR_INPUT_STREAM.equals(stream)) {
            return processPolarAccPayload(payload, sequence, receiveUnixNs, receiveElapsedNs);
        }
        if (CONTROLLER_INPUT_STREAM.equals(stream)) {
            return processControllerPose(payload, sequence, receiveUnixNs, receiveElapsedNs);
        }
        return null;
    }

    synchronized JSONObject processPolarAccPayload(
        JSONObject payload,
        long sequence,
        long receiveUnixNs,
        long receiveElapsedNs) throws Exception {
        polarPublishedFrames++;
        VectorReadResult read = readPolarAccVector(payload, receiveUnixNs, receiveElapsedNs);
        if (!read.accepted) {
            rejectedSamples++;
            polarTracker.recordRejected(read.errorCode, read.message);
            return processingRejected(POLAR_SOURCE, POLAR_INPUT_STREAM, read);
        }

        polarAcceptedFrames++;
        JSONObject assessment = polarTracker.submitValid(
            read.toSample(),
            sequence,
            receiveUnixNs,
            receiveElapsedNs,
            payload != null ? payload.optString("publisher_client_id", "") : "");
        recordAssessment(assessment);
        return processingAccepted(POLAR_SOURCE, POLAR_INPUT_STREAM, assessment);
    }

    synchronized JSONObject processControllerPose(
        JSONObject params,
        long sequence,
        long receiveUnixNs,
        long receiveElapsedNs) throws Exception {
        controllerSubmittedSamples++;
        VectorReadResult read = readControllerPose(params, receiveUnixNs, receiveElapsedNs);
        if (!read.accepted && !read.badTracking) {
            rejectedSamples++;
            controllerTracker.recordRejected(read.errorCode, read.message);
            return processingRejected(CONTROLLER_SOURCE, CONTROLLER_INPUT_STREAM, read);
        }

        JSONObject assessment;
        if (read.badTracking) {
            assessment = controllerTracker.submitBadTracking(
                read.sampleUnixNs,
                read.sampleElapsedNs,
                sequence,
                receiveUnixNs,
                receiveElapsedNs,
                read.sourceDetail);
        } else {
            controllerAcceptedSamples++;
            assessment = controllerTracker.submitValid(
                read.toSample(),
                sequence,
                receiveUnixNs,
                receiveElapsedNs,
                params != null ? params.optString("publisher_client_id", "") : "");
        }
        recordAssessment(assessment);
        return processingAccepted(CONTROLLER_SOURCE, CONTROLLER_INPUT_STREAM, assessment);
    }

    private void recordAssessment(JSONObject assessment) throws Exception {
        emittedAssessments++;
        latestAssessmentUnixNs = assessment.optLong("broker_publish_time_unix_ns", unixNowNs());
        latestAssessment = copyObject(assessment);
        revision++;
    }

    private String overallState() {
        if (polarTracker.isCalibrated() || controllerTracker.isCalibrated()) {
            return "ready";
        }
        if (polarTracker.isCalibrating() || controllerTracker.isCalibrating()) {
            return "calibrating";
        }
        if (latestAssessment.length() > 0) {
            return latestAssessment.optString("state", "active");
        }
        return "idle";
    }

    private static JSONObject processingAccepted(String source, String inputStream, JSONObject assessment) throws Exception {
        JSONObject result = new JSONObject();
        result.put("accepted", true);
        result.put("source", source);
        result.put("input_stream", inputStream);
        result.put("output_stream", OUTPUT_STREAM);
        result.put("assessment", copyObject(assessment));
        return result;
    }

    private static JSONObject processingRejected(String source, String inputStream, VectorReadResult read) throws Exception {
        JSONObject result = new JSONObject();
        result.put("accepted", false);
        result.put("source", source);
        result.put("input_stream", inputStream);
        result.put("output_stream", OUTPUT_STREAM);
        result.put("error_code", read.errorCode);
        result.put("message", read.message);
        return result;
    }

    private static VectorReadResult readPolarAccVector(
        JSONObject payload,
        long receiveUnixNs,
        long receiveElapsedNs) {
        if (payload == null) {
            return VectorReadResult.rejected("missing_payload", "Polar ACC assessment requires a JSON payload.");
        }

        long sampleUnixNs = firstLong(payload, receiveUnixNs, "sample_time_unix_ns", "client_send_time_unix_ns");
        long sampleElapsedNs = firstLong(payload, receiveElapsedNs, "sample_time_elapsed_ns", "source_time_elapsed_ns");

        String payloadBase64 = payload.optString("payload_base64", "");
        if (payloadBase64.length() > 0) {
            VectorReadResult decoded = readPolarPmdAccPayload(payloadBase64, sampleUnixNs, sampleElapsedNs);
            if (decoded.accepted) {
                return decoded;
            }
        }

        VectorReadResult direct = readVectorSamples(payload.optJSONArray("samples_mg"), 0.001d, sampleUnixNs, sampleElapsedNs, "samples_mg");
        if (direct.accepted) {
            return direct;
        }

        direct = readVectorArray(payload.optJSONArray("acc_mg"), 0.001d, sampleUnixNs, sampleElapsedNs, "acc_mg");
        if (direct.accepted) {
            return direct;
        }

        direct = readVectorArray(payload.optJSONArray("acc_g"), 1.0d, sampleUnixNs, sampleElapsedNs, "acc_g");
        if (direct.accepted) {
            return direct;
        }

        direct = readVectorFields(payload, "x_g", "y_g", "z_g", 1.0d, sampleUnixNs, sampleElapsedNs, "g_fields");
        if (direct.accepted) {
            return direct;
        }

        direct = readVectorFields(payload, "x_mg", "y_mg", "z_mg", 0.001d, sampleUnixNs, sampleElapsedNs, "mg_fields");
        if (direct.accepted) {
            return direct;
        }

        JSONObject decoded = payload.optJSONObject("decoded");
        if (decoded != null) {
            direct = readVectorFields(decoded, "first_x_mg", "first_y_mg", "first_z_mg", 0.001d, sampleUnixNs, sampleElapsedNs, "decoded_first_mg");
            if (direct.accepted) {
                return direct;
            }
        }

        return VectorReadResult.rejected(
            "missing_acc_vector",
            "Polar ACC payload must include payload_base64, samples_mg, acc_mg, acc_g, or x/y/z fields.");
    }

    private static VectorReadResult readPolarPmdAccPayload(
        String payloadBase64,
        long sampleUnixNs,
        long sampleElapsedNs) {
        try {
            byte[] bytes = Base64.decode(payloadBase64, Base64.DEFAULT);
            if (bytes.length < 16) {
                return VectorReadResult.rejected("short_pmd_payload", "Polar PMD ACC payload is too short.");
            }
            int measurementType = bytes[0] & 0xff;
            int frameType = bytes[9] & 0xff;
            if (measurementType != 0x02 || frameType != 0x01) {
                return VectorReadResult.rejected("unsupported_pmd_frame", "Only Polar PMD ACC frame type 1 is supported.");
            }

            int sampleCount = (bytes.length - 10) / 6;
            if (sampleCount <= 0) {
                return VectorReadResult.rejected("empty_pmd_payload", "Polar PMD ACC payload has no samples.");
            }

            double xMg = 0.0d;
            double yMg = 0.0d;
            double zMg = 0.0d;
            for (int i = 0; i < sampleCount; i++) {
                int offset = 10 + i * 6;
                xMg += readInt16LittleEndian(bytes, offset);
                yMg += readInt16LittleEndian(bytes, offset + 2);
                zMg += readInt16LittleEndian(bytes, offset + 4);
            }

            return VectorReadResult.accepted(
                xMg / sampleCount * 0.001d,
                yMg / sampleCount * 0.001d,
                zMg / sampleCount * 0.001d,
                sampleUnixNs,
                sampleElapsedNs,
                sampleCount,
                "polar_pmd_payload_base64");
        } catch (Exception ex) {
            return VectorReadResult.rejected("invalid_pmd_payload", ex.getClass().getSimpleName() + ": " + ex.getMessage());
        }
    }

    private static VectorReadResult readControllerPose(
        JSONObject params,
        long receiveUnixNs,
        long receiveElapsedNs) {
        if (params == null) {
            return VectorReadResult.rejected("missing_params", "Controller breath assessment requires params.");
        }

        long sampleUnixNs = firstLong(params, receiveUnixNs, "sample_time_unix_ns", "source_time_unix_ns", "client_send_time_unix_ns");
        long sampleElapsedNs = firstLong(params, receiveElapsedNs, "sample_time_elapsed_ns", "source_time_elapsed_ns");
        boolean connected = params.optBoolean("connected", true);
        boolean tracked = params.optBoolean("tracked", params.optBoolean("is_tracked", true));
        String controllerId = firstString(params, "controller", "hand", "source");
        if (controllerId.length() == 0) {
            controllerId = "controller";
        }
        if (!connected || !tracked) {
            return VectorReadResult.badTracking(sampleUnixNs, sampleElapsedNs, controllerId);
        }

        JSONObject pose = params.optJSONObject("pose");
        VectorReadResult read = readVectorArray(params.optJSONArray("position_m"), 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
        if (read.accepted) {
            return read;
        }

        JSONObject positionObject = params.optJSONObject("position_m");
        read = readVectorObject(positionObject, 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
        if (read.accepted) {
            return read;
        }

        read = readVectorArray(params.optJSONArray("position"), 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
        if (read.accepted) {
            return read;
        }

        read = readVectorObject(params.optJSONObject("position"), 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
        if (read.accepted) {
            return read;
        }

        if (pose != null) {
            read = readVectorArray(pose.optJSONArray("position_m"), 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
            if (read.accepted) {
                return read;
            }
            read = readVectorObject(pose.optJSONObject("position_m"), 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
            if (read.accepted) {
                return read;
            }
            read = readVectorArray(pose.optJSONArray("position"), 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
            if (read.accepted) {
                return read;
            }
            read = readVectorObject(pose.optJSONObject("position"), 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
            if (read.accepted) {
                return read;
            }
        }

        read = readVectorFields(params, "x_m", "y_m", "z_m", 1.0d, sampleUnixNs, sampleElapsedNs, controllerId);
        if (read.accepted) {
            return read;
        }

        return VectorReadResult.rejected(
            "missing_controller_position",
            "Controller pose must include position_m, position, pose.position_m, or x_m/y_m/z_m.");
    }

    private static VectorReadResult readVectorSamples(
        JSONArray samples,
        double scale,
        long sampleUnixNs,
        long sampleElapsedNs,
        String sourceDetail) {
        if (samples == null || samples.length() == 0) {
            return VectorReadResult.rejected("missing_vector_samples", "Vector sample array is missing.");
        }

        double x = 0.0d;
        double y = 0.0d;
        double z = 0.0d;
        int count = 0;
        for (int i = 0; i < samples.length(); i++) {
            JSONArray arraySample = samples.optJSONArray(i);
            VectorReadResult read;
            if (arraySample != null) {
                read = readVectorArray(arraySample, scale, sampleUnixNs, sampleElapsedNs, sourceDetail);
            } else {
                read = readVectorObject(samples.optJSONObject(i), scale, sampleUnixNs, sampleElapsedNs, sourceDetail);
            }
            if (!read.accepted) {
                continue;
            }
            x += read.x;
            y += read.y;
            z += read.z;
            count++;
        }

        if (count == 0) {
            return VectorReadResult.rejected("invalid_vector_samples", "Vector sample array has no valid x/y/z samples.");
        }
        return VectorReadResult.accepted(x / count, y / count, z / count, sampleUnixNs, sampleElapsedNs, count, sourceDetail);
    }

    private static VectorReadResult readVectorArray(
        JSONArray values,
        double scale,
        long sampleUnixNs,
        long sampleElapsedNs,
        String sourceDetail) {
        if (values == null || values.length() < 3) {
            return VectorReadResult.rejected("missing_vector_array", "Vector array must contain at least 3 values.");
        }
        return VectorReadResult.accepted(
            values.optDouble(0, 0.0d) * scale,
            values.optDouble(1, 0.0d) * scale,
            values.optDouble(2, 0.0d) * scale,
            sampleUnixNs,
            sampleElapsedNs,
            1,
            sourceDetail);
    }

    private static VectorReadResult readVectorObject(
        JSONObject value,
        double scale,
        long sampleUnixNs,
        long sampleElapsedNs,
        String sourceDetail) {
        if (value == null) {
            return VectorReadResult.rejected("missing_vector_object", "Vector object is missing.");
        }
        return readVectorFields(value, "x", "y", "z", scale, sampleUnixNs, sampleElapsedNs, sourceDetail);
    }

    private static VectorReadResult readVectorFields(
        JSONObject value,
        String xName,
        String yName,
        String zName,
        double scale,
        long sampleUnixNs,
        long sampleElapsedNs,
        String sourceDetail) {
        if (value == null || !value.has(xName) || !value.has(yName) || !value.has(zName)) {
            return VectorReadResult.rejected("missing_vector_fields", "Vector fields are missing.");
        }
        return VectorReadResult.accepted(
            value.optDouble(xName, 0.0d) * scale,
            value.optDouble(yName, 0.0d) * scale,
            value.optDouble(zName, 0.0d) * scale,
            sampleUnixNs,
            sampleElapsedNs,
            1,
            sourceDetail);
    }

    private static int readInt16LittleEndian(byte[] bytes, int offset) {
        return (short) ((bytes[offset] & 0xff) | ((bytes[offset + 1] & 0xff) << 8));
    }

    private static String normalizeSource(String source) {
        if (source == null || source.trim().length() == 0) {
            return "all";
        }
        String normalized = source.trim().toLowerCase();
        if ("polar".equals(normalized) || POLAR_INPUT_STREAM.equals(normalized)) {
            return POLAR_SOURCE;
        }
        if ("controller".equals(normalized) || "xr_controller_pose".equals(normalized) || CONTROLLER_INPUT_STREAM.equals(normalized)) {
            return CONTROLLER_SOURCE;
        }
        return normalized;
    }

    private static boolean appliesTo(String requested, String source) {
        return "all".equals(requested) || source.equals(requested);
    }

    private static long firstLong(JSONObject json, long fallback, String... names) {
        if (json == null) {
            return fallback;
        }
        for (String name : names) {
            if (json.has(name)) {
                return json.optLong(name, fallback);
            }
        }
        return fallback;
    }

    private static String firstString(JSONObject json, String... names) {
        if (json == null) {
            return "";
        }
        for (String name : names) {
            String value = json.optString(name, "");
            if (value.trim().length() > 0) {
                return value.trim();
            }
        }
        return "";
    }

    private static JSONObject copyObject(JSONObject source) throws Exception {
        return source == null ? new JSONObject() : new JSONObject(source.toString());
    }

    private static JSONArray arrayOf(String... values) {
        JSONArray array = new JSONArray();
        for (String value : values) {
            array.put(value);
        }
        return array;
    }

    private static long unixNowNs() {
        return System.currentTimeMillis() * 1_000_000L;
    }

    private static double clamp01(double value) {
        if (value < 0.0d) {
            return 0.0d;
        }
        if (value > 1.0d) {
            return 1.0d;
        }
        return value;
    }

    private static double clamp(double value, double min, double max) {
        return Math.max(min, Math.min(max, value));
    }

    private static final class ProjectionBreathTracker {
        private final String source;
        private final String inputStream;
        private final String units;
        private int calibrationFrameCount;
        private double nominalAnalysisRateHz;
        private double minAcceptedDelta;
        private double minTravel;
        private double sampleEmaAlpha;
        private double projectionEmaAlpha;
        private double lowQuantile = 0.05d;
        private double highQuantile = 0.95d;
        private double edgeEase = 0.12d;
        private double deltaThreshold;
        private boolean emitWhileCalibrating = true;
        private boolean invertVolume;
        private long revision = 1L;
        private long acceptedSamples;
        private long rejectedSamples;
        private long emittedAssessments;
        private long calibrationResets;
        private boolean calibrated;
        private boolean calibrating;
        private String state = "idle";
        private String lastError = "";
        private long lastSampleUnixNs;
        private long lastSampleElapsedNs;
        private Vector3 filtered;
        private Vector3 axis = new Vector3(0.0d, 1.0d, 0.0d);
        private double lowerProjection;
        private double upperProjection = 1.0d;
        private double filteredProjection;
        private boolean hasFilteredProjection;
        private double lastVolume01 = 0.5d;
        private final List<Vector3> calibrationSamples = new ArrayList<>();
        private Vector3 previousCalibrationSample;

        private ProjectionBreathTracker(
            String source,
            String inputStream,
            String units,
            int calibrationFrameCount,
            double nominalAnalysisRateHz,
            double minAcceptedDelta,
            double minTravel,
            double sampleEmaAlpha,
            double projectionEmaAlpha,
            double deltaThreshold) {
            this.source = source;
            this.inputStream = inputStream;
            this.units = units;
            this.calibrationFrameCount = calibrationFrameCount;
            this.nominalAnalysisRateHz = nominalAnalysisRateHz;
            this.minAcceptedDelta = minAcceptedDelta;
            this.minTravel = minTravel;
            this.sampleEmaAlpha = sampleEmaAlpha;
            this.projectionEmaAlpha = projectionEmaAlpha;
            this.deltaThreshold = deltaThreshold;
        }

        static ProjectionBreathTracker createPolar() {
            return new ProjectionBreathTracker(
                POLAR_SOURCE,
                POLAR_INPUT_STREAM,
                "g",
                120,
                20.0d,
                0.0005d,
                0.010d,
                0.15d,
                0.18d,
                0.012d);
        }

        static ProjectionBreathTracker createController() {
            return new ProjectionBreathTracker(
                CONTROLLER_SOURCE,
                CONTROLLER_INPUT_STREAM,
                "m",
                24,
                90.0d,
                0.0008d,
                0.025d,
                0.35d,
                0.35d,
                0.018d);
        }

        boolean isCalibrated() {
            return calibrated;
        }

        boolean isCalibrating() {
            return calibrating;
        }

        boolean configure(JSONObject params) {
            if (params == null) {
                return false;
            }

            boolean changed = false;
            if (params.has("calibration_frame_count")) {
                calibrationFrameCount = (int) clamp(params.optInt("calibration_frame_count", calibrationFrameCount), 4, 600);
                changed = true;
            }
            if (params.has("nominal_analysis_rate_hz")) {
                nominalAnalysisRateHz = clamp(params.optDouble("nominal_analysis_rate_hz", nominalAnalysisRateHz), 1.0d, 240.0d);
                changed = true;
            }
            if (params.has("min_accepted_delta")) {
                minAcceptedDelta = clamp(params.optDouble("min_accepted_delta", minAcceptedDelta), 0.0d, 10.0d);
                changed = true;
            }
            if (params.has("min_travel")) {
                minTravel = clamp(params.optDouble("min_travel", minTravel), 0.000001d, 10.0d);
                changed = true;
            }
            if (params.has("sample_ema_alpha")) {
                sampleEmaAlpha = clamp(params.optDouble("sample_ema_alpha", sampleEmaAlpha), 0.001d, 1.0d);
                changed = true;
            }
            if (params.has("projection_ema_alpha")) {
                projectionEmaAlpha = clamp(params.optDouble("projection_ema_alpha", projectionEmaAlpha), 0.001d, 1.0d);
                changed = true;
            }
            if (params.has("low_quantile")) {
                lowQuantile = clamp(params.optDouble("low_quantile", lowQuantile), 0.0d, 0.49d);
                changed = true;
            }
            if (params.has("high_quantile")) {
                highQuantile = clamp(params.optDouble("high_quantile", highQuantile), 0.51d, 1.0d);
                changed = true;
            }
            if (params.has("edge_ease")) {
                edgeEase = clamp(params.optDouble("edge_ease", edgeEase), 0.0d, 0.45d);
                changed = true;
            }
            if (params.has("delta_threshold")) {
                deltaThreshold = clamp(params.optDouble("delta_threshold", deltaThreshold), 0.0d, 1.0d);
                changed = true;
            }
            if (params.has("emit_while_calibrating")) {
                emitWhileCalibrating = params.optBoolean("emit_while_calibrating", emitWhileCalibrating);
                changed = true;
            }
            if (params.has("invert_volume")) {
                invertVolume = params.optBoolean("invert_volume", invertVolume);
                changed = true;
            }
            if (changed) {
                revision++;
            }
            return changed;
        }

        void resetCalibration() {
            calibrationSamples.clear();
            previousCalibrationSample = null;
            filtered = null;
            calibrated = false;
            calibrating = false;
            state = "idle";
            lastError = "";
            hasFilteredProjection = false;
            filteredProjection = 0.0d;
            lastVolume01 = 0.5d;
            calibrationResets++;
            revision++;
        }

        void recordRejected(String errorCode, String message) {
            rejectedSamples++;
            lastError = errorCode + ": " + message;
            state = "input_rejected";
            revision++;
        }

        JSONObject submitBadTracking(
            long sampleUnixNs,
            long sampleElapsedNs,
            long sequence,
            long receiveUnixNs,
            long receiveElapsedNs,
            String sourceDetail) throws Exception {
            state = "bad_tracking";
            lastError = "";
            lastSampleUnixNs = sampleUnixNs;
            lastSampleElapsedNs = sampleElapsedNs;
            emittedAssessments++;
            revision++;
            return buildAssessment(
                sequence,
                sampleUnixNs,
                sampleElapsedNs,
                receiveUnixNs,
                receiveElapsedNs,
                "bad_tracking",
                false,
                lastVolume01,
                0.0d,
                sourceDetail,
                1);
        }

        JSONObject submitValid(
            VectorSample sample,
            long sequence,
            long receiveUnixNs,
            long receiveElapsedNs,
            String publisherClientId) throws Exception {
            acceptedSamples++;
            lastSampleUnixNs = sample.sampleUnixNs;
            lastSampleElapsedNs = sample.sampleElapsedNs;
            lastError = "";
            filtered = filtered == null ? sample.value : filtered.lerp(sample.value, sampleEmaAlpha);

            if (!calibrated) {
                if (!calibrating) {
                    calibrating = true;
                    state = "calibrating";
                }
                maybeAddCalibrationSample(filtered);
                if (calibrationSamples.size() >= calibrationFrameCount) {
                    completeCalibrationIfReady();
                }
            }

            JSONObject assessment;
            if (!calibrated) {
                state = lastError.length() > 0 ? "calibrating" : "calibrating";
                assessment = buildAssessment(
                    sequence,
                    sample.sampleUnixNs,
                    sample.sampleElapsedNs,
                    receiveUnixNs,
                    receiveElapsedNs,
                    emitWhileCalibrating ? "calibrating" : "unavailable",
                    false,
                    lastVolume01,
                    calibrationProgress01(),
                    sample.sourceDetail,
                    sample.sampleCount);
            } else {
                double projection = filtered.dot(axis);
                filteredProjection = hasFilteredProjection
                    ? filteredProjection + (projection - filteredProjection) * projectionEmaAlpha
                    : projection;
                hasFilteredProjection = true;
                double rawVolume = (filteredProjection - lowerProjection) / Math.max(0.000001d, upperProjection - lowerProjection);
                double volume = easeVolume(clamp01(rawVolume));
                if (invertVolume) {
                    volume = 1.0d - volume;
                }
                double delta = volume - lastVolume01;
                String phase = phaseForDelta(delta);
                lastVolume01 = volume;
                state = phase;
                assessment = buildAssessment(
                    sequence,
                    sample.sampleUnixNs,
                    sample.sampleElapsedNs,
                    receiveUnixNs,
                    receiveElapsedNs,
                    phase,
                    true,
                    volume,
                    quality01(),
                    sample.sourceDetail,
                    sample.sampleCount);
            }

            if (publisherClientId != null && publisherClientId.length() > 0) {
                assessment.put("publisher_client_id", publisherClientId);
            }
            emittedAssessments++;
            revision++;
            return assessment;
        }

        JSONObject toStatusJson() throws Exception {
            JSONObject status = new JSONObject();
            status.put("schema", SOURCE_STATUS_SCHEMA);
            status.put("source", source);
            status.put("input_stream", inputStream);
            status.put("output_stream", OUTPUT_STREAM);
            status.put("state", state);
            status.put("units", units);
            status.put("revision", revision);
            status.put("is_calibrated", calibrated);
            status.put("is_calibrating", calibrating);
            status.put("accepted_samples", acceptedSamples);
            status.put("rejected_samples", rejectedSamples);
            status.put("emitted_assessments", emittedAssessments);
            status.put("calibration_samples", calibrationSamples.size());
            status.put("calibration_frame_count", calibrationFrameCount);
            status.put("calibration_progress01", calibrationProgress01());
            status.put("calibration_resets", calibrationResets);
            status.put("last_sample_time_unix_ns", lastSampleUnixNs);
            status.put("last_sample_time_elapsed_ns", lastSampleElapsedNs);
            status.put("last_error", lastError);
            status.put("volume01", lastVolume01);
            status.put("axis", axis.toJson());
            status.put("lower_projection", lowerProjection);
            status.put("upper_projection", upperProjection);
            status.put("quality01", quality01());
            status.put("config", configJson());
            return status;
        }

        private JSONObject buildAssessment(
            long sequence,
            long sampleUnixNs,
            long sampleElapsedNs,
            long receiveUnixNs,
            long receiveElapsedNs,
            String phase,
            boolean hasVolume,
            double volume01,
            double quality01,
            String sourceDetail,
            int sampleCount) throws Exception {
            long publishUnixNs = unixNowNs();
            long publishElapsedNs = SystemClock.elapsedRealtimeNanos();
            JSONObject assessment = new JSONObject();
            assessment.put("schema", BREATH_SCHEMA);
            assessment.put("source", source);
            assessment.put("source_detail", sourceDetail != null ? sourceDetail : "");
            assessment.put("input_stream", inputStream);
            assessment.put("output_stream", OUTPUT_STREAM);
            assessment.put("sequence_id", sequence);
            assessment.put("sample_count", sampleCount);
            assessment.put("sample_time_unix_ns", sampleUnixNs);
            assessment.put("sample_time_elapsed_ns", sampleElapsedNs);
            assessment.put("broker_receive_time_unix_ns", receiveUnixNs);
            assessment.put("broker_receive_time_elapsed_ns", receiveElapsedNs);
            assessment.put("broker_publish_time_unix_ns", publishUnixNs);
            assessment.put("broker_publish_time_elapsed_ns", publishElapsedNs);
            assessment.put("state", phase);
            assessment.put("state01", state01ForPhase(phase));
            assessment.put("tracking01", "bad_tracking".equals(phase) ? 0.0d : 1.0d);
            assessment.put("volume01", clamp01(volume01));
            assessment.put("has_volume", hasVolume);
            assessment.put("is_calibrated", calibrated);
            assessment.put("is_calibrating", calibrating && !calibrated);
            assessment.put("quality01", clamp01(quality01));
            assessment.put("units", units);
            assessment.put("algorithm", "projection_breath_assessment.v1");
            assessment.put("diagnostic_only", true);
            return assessment;
        }

        private JSONObject configJson() throws Exception {
            JSONObject config = new JSONObject();
            config.put("calibration_frame_count", calibrationFrameCount);
            config.put("nominal_analysis_rate_hz", nominalAnalysisRateHz);
            config.put("min_accepted_delta", minAcceptedDelta);
            config.put("min_travel", minTravel);
            config.put("sample_ema_alpha", sampleEmaAlpha);
            config.put("projection_ema_alpha", projectionEmaAlpha);
            config.put("low_quantile", lowQuantile);
            config.put("high_quantile", highQuantile);
            config.put("edge_ease", edgeEase);
            config.put("delta_threshold", deltaThreshold);
            config.put("emit_while_calibrating", emitWhileCalibrating);
            config.put("invert_volume", invertVolume);
            return config;
        }

        private void maybeAddCalibrationSample(Vector3 value) {
            if (previousCalibrationSample == null || value.distanceTo(previousCalibrationSample) >= minAcceptedDelta) {
                calibrationSamples.add(value);
                previousCalibrationSample = value;
            }
        }

        private void completeCalibrationIfReady() {
            AxisFit fit = AxisFit.fromSamples(calibrationSamples, lowQuantile, highQuantile);
            double span = fit.upperProjection - fit.lowerProjection;
            if (span < minTravel) {
                lastError = "insufficient_motion: calibration span " + span + " " + units + " is below min_travel.";
                state = "calibrating";
                while (calibrationSamples.size() > Math.max(4, calibrationFrameCount / 2)) {
                    calibrationSamples.remove(0);
                }
                previousCalibrationSample = calibrationSamples.isEmpty()
                    ? null
                    : calibrationSamples.get(calibrationSamples.size() - 1);
                return;
            }

            axis = fit.axis;
            lowerProjection = fit.lowerProjection;
            upperProjection = fit.upperProjection;
            calibrated = true;
            calibrating = false;
            state = "ready";
            lastError = "";
            hasFilteredProjection = false;
        }

        private double calibrationProgress01() {
            return clamp01(calibrationSamples.size() / Math.max(1.0d, calibrationFrameCount));
        }

        private double quality01() {
            if (!calibrated) {
                return calibrationProgress01();
            }
            return clamp01((upperProjection - lowerProjection) / Math.max(minTravel, 0.000001d));
        }

        private double easeVolume(double value) {
            if (edgeEase <= 0.0d) {
                return value;
            }
            double eased = value * value * (3.0d - 2.0d * value);
            return value + (eased - value) * edgeEase;
        }

        private String phaseForDelta(double delta) {
            if (delta > deltaThreshold) {
                return "inhaling";
            }
            if (delta < -deltaThreshold) {
                return "exhaling";
            }
            return "pausing";
        }

        private double state01ForPhase(String phase) {
            if ("inhaling".equals(phase)) {
                return 1.0d;
            }
            if ("exhaling".equals(phase)) {
                return 0.0d;
            }
            return 0.5d;
        }
    }

    private static final class AxisFit {
        final Vector3 axis;
        final double lowerProjection;
        final double upperProjection;

        private AxisFit(Vector3 axis, double lowerProjection, double upperProjection) {
            this.axis = axis;
            this.lowerProjection = lowerProjection;
            this.upperProjection = upperProjection;
        }

        static AxisFit fromSamples(List<Vector3> samples, double lowQuantile, double highQuantile) {
            if (samples.isEmpty()) {
                return new AxisFit(new Vector3(0.0d, 1.0d, 0.0d), 0.0d, 1.0d);
            }

            Vector3 mean = new Vector3(0.0d, 0.0d, 0.0d);
            for (Vector3 sample : samples) {
                mean = mean.add(sample);
            }
            mean = mean.scale(1.0d / samples.size());

            double xx = 0.0d;
            double xy = 0.0d;
            double xz = 0.0d;
            double yy = 0.0d;
            double yz = 0.0d;
            double zz = 0.0d;
            for (Vector3 sample : samples) {
                Vector3 d = sample.subtract(mean);
                xx += d.x * d.x;
                xy += d.x * d.y;
                xz += d.x * d.z;
                yy += d.y * d.y;
                yz += d.y * d.z;
                zz += d.z * d.z;
            }

            Vector3 axis = new Vector3(1.0d, 0.0d, 0.0d);
            for (int i = 0; i < 16; i++) {
                Vector3 next = new Vector3(
                    xx * axis.x + xy * axis.y + xz * axis.z,
                    xy * axis.x + yy * axis.y + yz * axis.z,
                    xz * axis.x + yz * axis.y + zz * axis.z);
                if (next.length() < 0.000000001d) {
                    Vector3 span = samples.get(samples.size() - 1).subtract(samples.get(0));
                    axis = span.normalizedOr(new Vector3(0.0d, 1.0d, 0.0d));
                    break;
                }
                axis = next.normalizedOr(axis);
            }

            List<Double> projections = new ArrayList<>();
            for (Vector3 sample : samples) {
                projections.add(sample.dot(axis));
            }
            Collections.sort(projections);
            double lower = quantile(projections, lowQuantile);
            double upper = quantile(projections, highQuantile);
            if (upper < lower) {
                double tmp = lower;
                lower = upper;
                upper = tmp;
            }
            return new AxisFit(axis, lower, upper);
        }

        private static double quantile(List<Double> sortedValues, double q) {
            if (sortedValues.isEmpty()) {
                return 0.0d;
            }
            if (sortedValues.size() == 1) {
                return sortedValues.get(0);
            }
            double index = clamp(q, 0.0d, 1.0d) * (sortedValues.size() - 1);
            int lo = (int) Math.floor(index);
            int hi = (int) Math.ceil(index);
            if (lo == hi) {
                return sortedValues.get(lo);
            }
            double t = index - lo;
            return sortedValues.get(lo) * (1.0d - t) + sortedValues.get(hi) * t;
        }
    }

    private static final class VectorReadResult {
        final boolean accepted;
        final boolean badTracking;
        final double x;
        final double y;
        final double z;
        final long sampleUnixNs;
        final long sampleElapsedNs;
        final int sampleCount;
        final String sourceDetail;
        final String errorCode;
        final String message;

        private VectorReadResult(
            boolean accepted,
            boolean badTracking,
            double x,
            double y,
            double z,
            long sampleUnixNs,
            long sampleElapsedNs,
            int sampleCount,
            String sourceDetail,
            String errorCode,
            String message) {
            this.accepted = accepted;
            this.badTracking = badTracking;
            this.x = x;
            this.y = y;
            this.z = z;
            this.sampleUnixNs = sampleUnixNs;
            this.sampleElapsedNs = sampleElapsedNs;
            this.sampleCount = sampleCount;
            this.sourceDetail = sourceDetail;
            this.errorCode = errorCode;
            this.message = message;
        }

        static VectorReadResult accepted(
            double x,
            double y,
            double z,
            long sampleUnixNs,
            long sampleElapsedNs,
            int sampleCount,
            String sourceDetail) {
            return new VectorReadResult(true, false, x, y, z, sampleUnixNs, sampleElapsedNs, sampleCount, sourceDetail, "", "");
        }

        static VectorReadResult badTracking(long sampleUnixNs, long sampleElapsedNs, String sourceDetail) {
            return new VectorReadResult(
                false,
                true,
                0.0d,
                0.0d,
                0.0d,
                sampleUnixNs,
                sampleElapsedNs,
                1,
                sourceDetail,
                "bad_tracking",
                "Controller is disconnected or untracked.");
        }

        static VectorReadResult rejected(String errorCode, String message) {
            return new VectorReadResult(false, false, 0.0d, 0.0d, 0.0d, 0L, 0L, 0, "", errorCode, message);
        }

        VectorSample toSample() {
            return new VectorSample(new Vector3(x, y, z), sampleUnixNs, sampleElapsedNs, sampleCount, sourceDetail);
        }
    }

    private static final class VectorSample {
        final Vector3 value;
        final long sampleUnixNs;
        final long sampleElapsedNs;
        final int sampleCount;
        final String sourceDetail;

        private VectorSample(Vector3 value, long sampleUnixNs, long sampleElapsedNs, int sampleCount, String sourceDetail) {
            this.value = value;
            this.sampleUnixNs = sampleUnixNs;
            this.sampleElapsedNs = sampleElapsedNs;
            this.sampleCount = sampleCount;
            this.sourceDetail = sourceDetail;
        }
    }

    private static final class Vector3 {
        final double x;
        final double y;
        final double z;

        Vector3(double x, double y, double z) {
            this.x = x;
            this.y = y;
            this.z = z;
        }

        Vector3 add(Vector3 other) {
            return new Vector3(x + other.x, y + other.y, z + other.z);
        }

        Vector3 subtract(Vector3 other) {
            return new Vector3(x - other.x, y - other.y, z - other.z);
        }

        Vector3 scale(double value) {
            return new Vector3(x * value, y * value, z * value);
        }

        Vector3 lerp(Vector3 other, double alpha) {
            return new Vector3(
                x + (other.x - x) * alpha,
                y + (other.y - y) * alpha,
                z + (other.z - z) * alpha);
        }

        double dot(Vector3 other) {
            return x * other.x + y * other.y + z * other.z;
        }

        double length() {
            return Math.sqrt(dot(this));
        }

        double distanceTo(Vector3 other) {
            return subtract(other).length();
        }

        Vector3 normalizedOr(Vector3 fallback) {
            double len = length();
            if (len < 0.000000001d) {
                return fallback;
            }
            return scale(1.0d / len);
        }

        JSONArray toJson() throws Exception {
            JSONArray array = new JSONArray();
            array.put(x);
            array.put(y);
            array.put(z);
            return array;
        }
    }
}
