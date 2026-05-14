package com.example.rustyxr.broker;

import android.os.SystemClock;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.Locale;
import java.util.concurrent.atomic.AtomicLong;

final class ClockCore {
    static final String CLOCK_ID = "broker-clock";
    static final String PRIMARY_DOMAIN = "ElapsedRealtime";
    static final String UNIX_DOMAIN = "Unix";
    static final String OPENXR_DOMAIN = "OpenXrPredictedDisplay";

    private static final String SNAPSHOT_SCHEMA = "rusty.xr.clock.snapshot.v1";
    private static final String STAMP_SCHEMA = "rusty.xr.clock.stamp.v1";
    private static final String CORRELATION_SCHEMA = "rusty.xr.clock.correlation.v1";
    private static final String HEALTH_SCHEMA = "rusty.xr.clock.health.v1";
    private static final String SYNC_PROBE_SCHEMA = "rusty.xr.clock.sync_probe.v1";
    private static final long WALL_CLOCK_JUMP_THRESHOLD_NS = 2_000_000_000L;
    private static final long DEGRADATION_HOLD_NS = 10_000_000_000L;
    private static final long UNKNOWN_OFFSET_NS = Long.MIN_VALUE;

    private final String clockEpochId;
    private final AtomicLong nextSequence = new AtomicLong(1L);
    private long lastWallClockOffsetNs = UNKNOWN_OFFSET_NS;
    private long wallClockAdjustmentCounter;
    private long lastElapsedRealtimeNs;
    private long degradedUntilElapsedNs;
    private String healthState = "Healthy";
    private String lastDiscontinuityReason = "None";

    ClockCore() {
        ClockRead read = readClocks();
        clockEpochId = "epoch-" + Long.toHexString(read.elapsedRealtimeNs) + "-" + Long.toHexString(read.unixNs);
        lastElapsedRealtimeNs = read.elapsedRealtimeNs;
        lastWallClockOffsetNs = read.unixNs - read.elapsedRealtimeNs;
    }

    synchronized JSONObject snapshotJson() throws Exception {
        ClockRead read = readClocks();
        updateHealth(read);
        return snapshotJson(read, nextSequence.getAndIncrement());
    }

    synchronized JSONObject stampJson() throws Exception {
        return stampJson(null, null, null);
    }

    synchronized JSONObject stampJson(String sourceDomain, Long sourceTimeNs, String correlationId) throws Exception {
        JSONObject snapshot = snapshotJson();
        JSONObject stamp = new JSONObject();
        stamp.put("schema", STAMP_SCHEMA);
        stamp.put("clock_id", CLOCK_ID);
        stamp.put("clock_epoch_id", clockEpochId);
        stamp.put("canonical_domain", PRIMARY_DOMAIN);
        stamp.put("event_elapsed_realtime_ns", snapshot.optLong("android_elapsed_realtime_ns", 0L));
        stamp.put("event_unix_ns", snapshot.has("android_realtime_unix_ns")
            ? snapshot.optLong("android_realtime_unix_ns", 0L)
            : JSONObject.NULL);
        stamp.put("source_domain", sourceDomain != null && sourceDomain.trim().length() > 0
            ? sourceDomain.trim()
            : JSONObject.NULL);
        stamp.put("source_time_ns", sourceTimeNs != null ? sourceTimeNs.longValue() : JSONObject.NULL);
        stamp.put("correlation_id", correlationId != null && correlationId.trim().length() > 0
            ? correlationId.trim()
            : JSONObject.NULL);
        stamp.put("uncertainty_ns", snapshot.optLong("read_uncertainty_ns", 0L));
        stamp.put("sequence_number", snapshot.optLong("sequence_number", 0L));
        return stamp;
    }

    synchronized JSONObject statusJson() throws Exception {
        JSONObject snapshot = snapshotJson();
        JSONObject status = new JSONObject();
        status.put("schema", "rusty.xr.clock.status.v1");
        status.put("clock_id", CLOCK_ID);
        status.put("clock_epoch_id", clockEpochId);
        status.put("primary_domain", PRIMARY_DOMAIN);
        status.put("health", snapshot.optString("health", "Unavailable"));
        status.put("snapshot", snapshot);
        status.put("domains", domainsArray());
        status.put("correlations", correlationsArray(snapshot));
        status.put("openxr_comparison", openXrComparisonJson(snapshot));
        status.put("storage_stamping", "stream_events_and_published_payloads");
        return status;
    }

    synchronized JSONObject domainsJson() throws Exception {
        JSONObject result = new JSONObject();
        result.put("schema", "rusty.xr.clock.domains.v1");
        result.put("clock_id", CLOCK_ID);
        result.put("clock_epoch_id", clockEpochId);
        result.put("primary_domain", PRIMARY_DOMAIN);
        result.put("domains", domainsArray());
        return result;
    }

    synchronized JSONObject correlationsJson() throws Exception {
        JSONObject snapshot = snapshotJson();
        JSONObject result = new JSONObject();
        result.put("schema", "rusty.xr.clock.correlations.v1");
        result.put("clock_id", CLOCK_ID);
        result.put("clock_epoch_id", clockEpochId);
        result.put("snapshot", snapshot);
        result.put("correlations", correlationsArray(snapshot));
        return result;
    }

    synchronized JSONObject healthJson() throws Exception {
        JSONObject snapshot = snapshotJson();
        JSONObject health = new JSONObject();
        health.put("schema", HEALTH_SCHEMA);
        health.put("clock_id", CLOCK_ID);
        health.put("clock_epoch_id", clockEpochId);
        health.put("health", snapshot.optString("health", "Unavailable"));
        health.put("wall_clock_adjustment_counter", wallClockAdjustmentCounter);
        health.put("last_snapshot", snapshot);
        health.put("active_correlations", correlationsArray(snapshot));
        health.put("last_discontinuity_reason", lastDiscontinuityReason);
        return health;
    }

    synchronized JSONObject openXrComparisonJson() throws Exception {
        return openXrComparisonJson(snapshotJson());
    }

    synchronized JSONObject syncProbeJson(JSONObject params) throws Exception {
        JSONObject options = params != null ? params : new JSONObject();
        long sequence = nextSequence.getAndIncrement();
        String probeId = options.optString("probe_id", "");
        if (probeId.trim().length() == 0) {
            probeId = String.format(Locale.ROOT, "clock-probe-%d", sequence);
        }

        long hostSendUnixNs = options.optLong("host_send_unix_ns", 0L);
        ClockRead receive = readClocks();
        updateHealth(receive);
        ClockRead send = readClocks();
        updateHealth(send);

        JSONObject probe = new JSONObject();
        probe.put("schema", SYNC_PROBE_SCHEMA);
        probe.put("probe_id", probeId);
        probe.put("sequence_number", sequence);
        probe.put("host_send_unix_ns", hostSendUnixNs);
        probe.put("target_receive_elapsed_ns", receive.elapsedRealtimeNs);
        probe.put("target_receive_unix_ns", receive.unixNs);
        probe.put("target_send_elapsed_ns", send.elapsedRealtimeNs);
        probe.put("target_send_unix_ns", send.unixNs);
        if (options.has("host_receive_unix_ns")) {
            probe.put("host_receive_unix_ns", options.optLong("host_receive_unix_ns", 0L));
        } else {
            probe.put("host_receive_unix_ns", JSONObject.NULL);
        }
        probe.put("target_processing_ns", Math.max(0L, send.elapsedRealtimeNs - receive.elapsedRealtimeNs));
        probe.put("clock_id", CLOCK_ID);
        probe.put("clock_epoch_id", clockEpochId);
        return probe;
    }

    private JSONObject snapshotJson(ClockRead read, long sequence) throws Exception {
        JSONObject snapshot = new JSONObject();
        snapshot.put("schema", SNAPSHOT_SCHEMA);
        snapshot.put("clock_id", CLOCK_ID);
        snapshot.put("clock_epoch_id", clockEpochId);
        snapshot.put("sequence_number", sequence);
        snapshot.put("canonical_domain", PRIMARY_DOMAIN);
        snapshot.put("android_elapsed_realtime_ns", read.elapsedRealtimeNs);
        snapshot.put("android_realtime_unix_ns", read.unixNs);
        snapshot.put("read_uncertainty_ns", read.uncertaintyNs);
        snapshot.put("wall_clock_adjustment_counter", wallClockAdjustmentCounter);
        snapshot.put("health", healthState);
        return snapshot;
    }

    private JSONArray domainsArray() throws Exception {
        JSONArray domains = new JSONArray();
        domains.put(domainJson(
            PRIMARY_DOMAIN,
            "Android SystemClock.elapsedRealtimeNanos",
            true,
            "canonical_ordering",
            "Primary broker storage and ordering domain."));
        domains.put(domainJson(
            UNIX_DOMAIN,
            "Android wall clock",
            true,
            "human_labels",
            "May jump when wall-clock sync or user settings change."));
        domains.put(domainJson(
            "CameraSensor",
            "Camera2 or NDK camera frame timestamps",
            true,
            "source_timestamp",
            "Available only on camera samples that report sensor timestamps."));
        domains.put(domainJson(
            "MediaPts",
            "MediaCodec presentation timestamps",
            true,
            "source_timestamp",
            "Stream-relative unless a producer declares otherwise."));
        domains.put(domainJson(
            OPENXR_DOMAIN,
            "OpenXR predicted display time",
            false,
            "runtime_comparison",
            "Unavailable until a Rusty XR-owned immersive session publishes frame samples."));
        domains.put(domainJson(
            "RelayReceive",
            "Broker receive-side sample stamp",
            true,
            "transport_diagnostics",
            "Used for stream-event receipt and forwarding diagnostics."));
        return domains;
    }

    private JSONArray correlationsArray(JSONObject snapshot) throws Exception {
        JSONArray correlations = new JSONArray();
        long elapsedNs = snapshot.optLong("android_elapsed_realtime_ns", 0L);
        long unixNs = snapshot.optLong("android_realtime_unix_ns", 0L);
        long uncertaintyNs = snapshot.optLong("read_uncertainty_ns", 0L);

        JSONObject unixCorrelation = correlationJson(
            "unix-to-elapsed-current",
            UNIX_DOMAIN,
            PRIMARY_DOMAIN,
            1,
            elapsedNs,
            elapsedNs,
            elapsedNs - unixNs,
            0.0d,
            uncertaintyNs,
            uncertaintyNs,
            uncertaintyNs,
            uncertaintyNs,
            wallClockAdjustmentCounter == 0L ? "Medium" : "Low",
            wallClockAdjustmentCounter == 0L ? "None" : lastDiscontinuityReason);
        unixCorrelation.put("method", "single_bracketed_wall_clock_sample");
        correlations.put(unixCorrelation);

        JSONObject openXrCorrelation = correlationJson(
            "openxr-to-elapsed-unavailable",
            OPENXR_DOMAIN,
            PRIMARY_DOMAIN,
            0,
            elapsedNs,
            elapsedNs,
            0L,
            0.0d,
            0L,
            0L,
            0L,
            0L,
            "Unavailable",
            "RuntimeLoss");
        openXrCorrelation.put("method", "no_active_openxr_frame_sampler");
        correlations.put(openXrCorrelation);
        return correlations;
    }

    private JSONObject openXrComparisonJson(JSONObject snapshot) throws Exception {
        JSONObject comparison = new JSONObject();
        comparison.put("schema", "rusty.xr.clock.openxr_comparison.v1");
        comparison.put("available", false);
        comparison.put("source_domain", OPENXR_DOMAIN);
        comparison.put("target_domain", PRIMARY_DOMAIN);
        comparison.put("clock_epoch_id", clockEpochId);
        comparison.put("last_checked_elapsed_ns", snapshot.optLong("android_elapsed_realtime_ns", 0L));
        comparison.put("state", "unavailable");
        comparison.put("reason", "no_active_rusty_xr_owned_openxr_session");
        return comparison;
    }

    private JSONObject domainJson(
        String id,
        String source,
        boolean available,
        String role,
        String note) throws Exception {
        JSONObject domain = new JSONObject();
        domain.put("id", id);
        domain.put("source", source);
        domain.put("available", available);
        domain.put("role", role);
        domain.put("note", note);
        return domain;
    }

    private JSONObject correlationJson(
        String correlationId,
        String sourceDomain,
        String targetDomain,
        int sampleCount,
        long windowStartElapsedNs,
        long windowEndElapsedNs,
        long offsetNs,
        double driftPpm,
        long rmsErrorNs,
        long maxErrorNs,
        long p95ErrorNs,
        long uncertaintyNs,
        String quality,
        String discontinuityReason) throws Exception {
        JSONObject correlation = new JSONObject();
        correlation.put("schema", CORRELATION_SCHEMA);
        correlation.put("correlation_id", correlationId);
        correlation.put("source_domain", sourceDomain);
        correlation.put("target_domain", targetDomain);
        correlation.put("sample_count", sampleCount);
        correlation.put("window_start_elapsed_ns", windowStartElapsedNs);
        correlation.put("window_end_elapsed_ns", windowEndElapsedNs);
        correlation.put("offset_ns", offsetNs);
        correlation.put("drift_ppm", driftPpm);
        correlation.put("rms_error_ns", rmsErrorNs);
        correlation.put("max_error_ns", maxErrorNs);
        correlation.put("p95_error_ns", p95ErrorNs);
        correlation.put("uncertainty_ns", uncertaintyNs);
        correlation.put("quality", quality);
        correlation.put("last_discontinuity_reason", discontinuityReason);
        return correlation;
    }

    private ClockRead readClocks() {
        long beforeElapsedNs = SystemClock.elapsedRealtimeNanos();
        long unixNs = unixNowNs();
        long afterElapsedNs = SystemClock.elapsedRealtimeNanos();
        if (afterElapsedNs < beforeElapsedNs) {
            afterElapsedNs = beforeElapsedNs;
        }
        long uncertaintyNs = afterElapsedNs - beforeElapsedNs;
        long elapsedNs = beforeElapsedNs + (uncertaintyNs / 2L);
        return new ClockRead(elapsedNs, unixNs, uncertaintyNs);
    }

    private void updateHealth(ClockRead read) {
        if (read.elapsedRealtimeNs < lastElapsedRealtimeNs) {
            healthState = "Degraded";
            degradedUntilElapsedNs = read.elapsedRealtimeNs + DEGRADATION_HOLD_NS;
            lastDiscontinuityReason = "Unknown";
        }
        lastElapsedRealtimeNs = Math.max(lastElapsedRealtimeNs, read.elapsedRealtimeNs);

        long offsetNs = read.unixNs - read.elapsedRealtimeNs;
        if (lastWallClockOffsetNs != UNKNOWN_OFFSET_NS &&
            safeAbsDiff(offsetNs, lastWallClockOffsetNs) > WALL_CLOCK_JUMP_THRESHOLD_NS) {
            wallClockAdjustmentCounter++;
            healthState = "Degraded";
            degradedUntilElapsedNs = read.elapsedRealtimeNs + DEGRADATION_HOLD_NS;
            lastDiscontinuityReason = "WallClockJump";
        }
        lastWallClockOffsetNs = offsetNs;

        if ("Degraded".equals(healthState) && read.elapsedRealtimeNs > degradedUntilElapsedNs) {
            healthState = "Healthy";
            if (wallClockAdjustmentCounter == 0L) {
                lastDiscontinuityReason = "None";
            }
        }
    }

    private static long safeAbsDiff(long left, long right) {
        return left >= right ? left - right : right - left;
    }

    private static long unixNowNs() {
        return System.currentTimeMillis() * 1_000_000L;
    }

    private static final class ClockRead {
        final long elapsedRealtimeNs;
        final long unixNs;
        final long uncertaintyNs;

        ClockRead(long elapsedRealtimeNs, long unixNs, long uncertaintyNs) {
            this.elapsedRealtimeNs = elapsedRealtimeNs;
            this.unixNs = unixNs;
            this.uncertaintyNs = uncertaintyNs;
        }
    }
}
