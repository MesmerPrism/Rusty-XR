package com.example.rustyxr.broker;

import android.app.ActivityManager;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;
import android.os.BatteryManager;
import android.os.Build;
import android.os.PowerManager;
import android.os.SystemClock;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.io.FileWriter;
import java.util.Locale;
import java.util.concurrent.atomic.AtomicLong;

final class DeviceWatchdog {
    static final String STATUS_SCHEMA = "rusty.xr.device_watchdog.status.v1";
    static final String SAMPLE_SCHEMA = "rusty.xr.device_watchdog.sample.v1";
    private static final String PREFS = "rusty_xr_device_watchdog";
    private static final String PREF_ENABLED = "enabled";
    private static final String PREF_INTERVAL_MS = "interval_ms";
    private static final String PREF_WAKE_LOCK = "wake_lock";
    private static final String PREF_MAX_BYTES = "max_log_bytes";
    private static final long DEFAULT_INTERVAL_MS = 30_000L;
    private static final long MIN_INTERVAL_MS = 1_000L;
    private static final long DEFAULT_MAX_LOG_BYTES = 8L * 1024L * 1024L;

    private final Context context;
    private final BrokerState state;
    private final Object lock = new Object();
    private final AtomicLong sequence = new AtomicLong();

    private volatile boolean running;
    private volatile boolean stopRequested;
    private volatile Thread thread;
    private volatile long intervalMs = DEFAULT_INTERVAL_MS;
    private volatile long maxLogBytes = DEFAULT_MAX_LOG_BYTES;
    private volatile boolean wakeLockRequested;
    private volatile String runId = "";
    private volatile String logPath = "";
    private volatile String lastError = "";
    private volatile String stopReason = "";
    private volatile JSONObject latestSample = new JSONObject();
    private long startedUnixMs;
    private long startedElapsedMs;
    private PowerManager.WakeLock wakeLock;

    DeviceWatchdog(Context context, BrokerState state) {
        this.context = context.getApplicationContext();
        this.state = state;
        publishStatusQuietly();
    }

    JSONObject start(JSONObject params) throws Exception {
        synchronized (lock) {
            if (running) {
                return statusJson();
            }

            intervalMs = sanitizeInterval(params != null ? params.optLong("interval_ms", DEFAULT_INTERVAL_MS) : DEFAULT_INTERVAL_MS);
            if (params != null && !params.has("interval_ms")) {
                intervalMs = sanitizeInterval(params.optLong("intervalMs", intervalMs));
            }
            wakeLockRequested = params != null
                && (params.optBoolean("wake_lock", false) || params.optBoolean("wakeLock", false));
            maxLogBytes = params != null
                ? Math.max(64L * 1024L, params.optLong("max_log_bytes", DEFAULT_MAX_LOG_BYTES))
                : DEFAULT_MAX_LOG_BYTES;
            if (params != null && !params.has("max_log_bytes")) {
                maxLogBytes = Math.max(64L * 1024L, params.optLong("maxLogBytes", maxLogBytes));
            }
            runId = params != null ? cleanRunId(params.optString("run_id", "")) : "";
            if (runId.length() == 0 && params != null) {
                runId = cleanRunId(params.optString("runId", ""));
            }
            if (runId.length() == 0) {
                runId = "device-watchdog-" + System.currentTimeMillis();
            }

            File dir = watchdogDirectory();
            if (!dir.exists() && !dir.mkdirs()) {
                throw new IllegalStateException("Could not create watchdog directory: " + dir.getAbsolutePath());
            }
            File logFile = new File(dir, runId + ".jsonl");
            logPath = logFile.getAbsolutePath();
            lastError = "";
            stopReason = "";
            startedUnixMs = System.currentTimeMillis();
            startedElapsedMs = SystemClock.elapsedRealtime();
            sequence.set(0L);
            latestSample = new JSONObject();
            stopRequested = false;

            maybeAcquireWakeLock();
            running = true;
            persistDesiredState(true);
            thread = new Thread(new Runnable() {
                @Override
                public void run() {
                    sampleLoop(logFile);
                }
            }, "RustyXrDeviceWatchdog");
            thread.start();
            publishStatusQuietly();
            return statusJson();
        }
    }

    JSONObject stop(String reason) throws Exception {
        Thread toJoin;
        synchronized (lock) {
            stopReason = reason != null && reason.length() > 0 ? reason : "operator_stop";
            stopRequested = true;
            running = false;
            persistDesiredState(false);
            toJoin = thread;
            thread = null;
            releaseWakeLock();
            publishStatusQuietly();
        }
        if (toJoin != null) {
            try {
                toJoin.join(1_000L);
            } catch (InterruptedException ex) {
                Thread.currentThread().interrupt();
            }
        }
        return statusJson();
    }

    void shutdownForServiceDestroy() {
        synchronized (lock) {
            stopReason = "broker_service_destroyed";
            stopRequested = true;
            running = false;
            thread = null;
            releaseWakeLock();
            publishStatusQuietly();
        }
    }

    void restoreIfDesired() {
        SharedPreferences prefs = prefs();
        if (!prefs.getBoolean(PREF_ENABLED, false)) {
            return;
        }

        JSONObject params = new JSONObject();
        try {
            params.put("interval_ms", prefs.getLong(PREF_INTERVAL_MS, DEFAULT_INTERVAL_MS));
            params.put("wake_lock", prefs.getBoolean(PREF_WAKE_LOCK, false));
            params.put("max_log_bytes", prefs.getLong(PREF_MAX_BYTES, DEFAULT_MAX_LOG_BYTES));
            params.put("run_id", "device-watchdog-restored-" + System.currentTimeMillis());
            start(params);
        } catch (Exception ex) {
            lastError = ex.getClass().getSimpleName() + ": " + ex.getMessage();
            Log.w(BrokerService.TAG, "Device watchdog restore failed: " + lastError, ex);
            publishStatusQuietly();
        }
    }

    JSONObject mark(JSONObject params) throws Exception {
        if (logPath == null || logPath.length() == 0) {
            throw new IllegalStateException("Device watchdog has no active log file.");
        }

        JSONObject marker = baseEvent("marker");
        if (params != null) {
            marker.put("label", params.optString("label", ""));
            marker.put("note", params.optString("note", ""));
            JSONObject payload = params.optJSONObject("payload");
            if (payload != null) {
                marker.put("payload", payload);
            }
        }
        appendJsonLine(new File(logPath), marker);
        return statusJson();
    }

    synchronized JSONObject statusJson() throws Exception {
        JSONObject status = new JSONObject();
        status.put("schema", STATUS_SCHEMA);
        status.put("running", running);
        status.put("run_id", runId);
        status.put("interval_ms", intervalMs);
        status.put("started_unix_ms", startedUnixMs);
        status.put("started_elapsed_ms", startedElapsedMs);
        status.put("uptime_ms", running ? Math.max(0L, SystemClock.elapsedRealtime() - startedElapsedMs) : 0L);
        status.put("sample_count", sequence.get());
        status.put("log_path", logPath);
        status.put("wake_lock_requested", wakeLockRequested);
        status.put("wake_lock_held", wakeLock != null && wakeLock.isHeld());
        status.put("max_log_bytes", maxLogBytes);
        status.put("last_error", lastError);
        status.put("stop_reason", stopReason);
        status.put("latest_sample", latestSample != null ? new JSONObject(latestSample.toString()) : new JSONObject());
        JSONArray limitations = new JSONArray();
        limitations.put("normal_app_uid_not_android_shell");
        limitations.put("requires_broker_or_activity_launch_after_full_reboot");
        limitations.put("sleep_and_doze_policy_is_platform_owned");
        limitations.put("powered_off_device_cannot_run_watchdog");
        status.put("limitations", limitations);
        return status;
    }

    private void sampleLoop(File logFile) {
        while (!stopRequested) {
            try {
                if (logFile.length() >= maxLogBytes) {
                    stopReason = "max_log_bytes_reached";
                    break;
                }
                JSONObject sample = sampleJson();
                latestSample = new JSONObject(sample.toString());
                appendJsonLine(logFile, sample);
                publishStatusQuietly();
            } catch (Exception ex) {
                lastError = ex.getClass().getSimpleName() + ": " + ex.getMessage();
                Log.w(BrokerService.TAG, "Device watchdog sample failed: " + lastError, ex);
                publishStatusQuietly();
            }

            long sleepMs = intervalMs;
            long slept = 0L;
            while (!stopRequested && slept < sleepMs) {
                long chunk = Math.min(500L, sleepMs - slept);
                SystemClock.sleep(chunk);
                slept += chunk;
            }
        }

        synchronized (lock) {
            running = false;
            releaseWakeLock();
            if (stopReason.length() == 0) {
                stopReason = "loop_exited";
            }
            if ("max_log_bytes_reached".equals(stopReason)) {
                persistDesiredState(false);
            }
            publishStatusQuietly();
        }
    }

    private JSONObject sampleJson() throws Exception {
        JSONObject sample = baseEvent("sample");
        sample.put("sequence", sequence.incrementAndGet());
        sample.put("power", powerJson());
        sample.put("battery", batteryJson());
        sample.put("network", networkJson());
        sample.put("memory", memoryJson());
        sample.put("storage", storageJson());
        return sample;
    }

    private JSONObject baseEvent(String event) throws Exception {
        JSONObject sample = new JSONObject();
        sample.put("schema", SAMPLE_SCHEMA);
        sample.put("event", event);
        sample.put("run_id", runId);
        sample.put("unix_ms", System.currentTimeMillis());
        sample.put("elapsed_realtime_ms", SystemClock.elapsedRealtime());
        sample.put("elapsed_realtime_ns", SystemClock.elapsedRealtimeNanos());
        sample.put("thread", Thread.currentThread().getName());
        return sample;
    }

    private JSONObject powerJson() throws Exception {
        PowerManager power = (PowerManager) context.getSystemService(Context.POWER_SERVICE);
        JSONObject object = new JSONObject();
        if (power == null) {
            object.put("available", false);
            return object;
        }
        object.put("available", true);
        object.put("interactive", power.isInteractive());
        object.put("power_save_mode", power.isPowerSaveMode());
        object.put("device_idle_mode", Build.VERSION.SDK_INT >= 23 && power.isDeviceIdleMode());
        object.put("ignoring_battery_optimizations", Build.VERSION.SDK_INT >= 23 && power.isIgnoringBatteryOptimizations(context.getPackageName()));
        object.put("wake_lock_requested", wakeLockRequested);
        object.put("wake_lock_held", wakeLock != null && wakeLock.isHeld());
        if (Build.VERSION.SDK_INT >= 29) {
            object.put("thermal_status", power.getCurrentThermalStatus());
            object.put("thermal_status_label", thermalStatusLabel(power.getCurrentThermalStatus()));
        }
        return object;
    }

    private JSONObject batteryJson() throws Exception {
        JSONObject object = new JSONObject();
        Intent battery = context.registerReceiver(null, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
        if (battery == null) {
            object.put("available", false);
            return object;
        }
        object.put("available", true);
        int level = battery.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
        int scale = battery.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
        object.put("level", level);
        object.put("scale", scale);
        if (level >= 0 && scale > 0) {
            object.put("percent", (100.0 * level) / scale);
        }
        object.put("status", battery.getIntExtra(BatteryManager.EXTRA_STATUS, -1));
        object.put("status_label", batteryStatusLabel(battery.getIntExtra(BatteryManager.EXTRA_STATUS, -1)));
        object.put("health", battery.getIntExtra(BatteryManager.EXTRA_HEALTH, -1));
        object.put("plugged", battery.getIntExtra(BatteryManager.EXTRA_PLUGGED, -1));
        object.put("temperature_c", battery.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, 0) / 10.0);
        object.put("voltage_mv", battery.getIntExtra(BatteryManager.EXTRA_VOLTAGE, 0));
        return object;
    }

    private JSONObject networkJson() throws Exception {
        JSONObject object = new JSONObject();
        ConnectivityManager manager = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
        if (manager == null) {
            object.put("available", false);
            return object;
        }
        Network network = manager.getActiveNetwork();
        NetworkCapabilities capabilities = network != null ? manager.getNetworkCapabilities(network) : null;
        object.put("available", true);
        object.put("connected", network != null && capabilities != null);
        if (capabilities != null) {
            object.put("has_internet", capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET));
            object.put("validated", capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_VALIDATED));
            object.put("transport_wifi", capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI));
            object.put("transport_cellular", capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR));
            object.put("downstream_kbps", capabilities.getLinkDownstreamBandwidthKbps());
            object.put("upstream_kbps", capabilities.getLinkUpstreamBandwidthKbps());
        }
        WifiManager wifi = (WifiManager) context.getApplicationContext().getSystemService(Context.WIFI_SERVICE);
        if (wifi != null) {
            WifiInfo info = wifi.getConnectionInfo();
            if (info != null) {
                object.put("wifi_rssi_dbm", info.getRssi());
                object.put("wifi_link_speed_mbps", info.getLinkSpeed());
            }
        }
        return object;
    }

    private JSONObject memoryJson() throws Exception {
        JSONObject object = new JSONObject();
        ActivityManager manager = (ActivityManager) context.getSystemService(Context.ACTIVITY_SERVICE);
        if (manager == null) {
            object.put("available", false);
            return object;
        }
        ActivityManager.MemoryInfo info = new ActivityManager.MemoryInfo();
        manager.getMemoryInfo(info);
        object.put("available", true);
        object.put("avail_mem_bytes", info.availMem);
        object.put("total_mem_bytes", info.totalMem);
        object.put("threshold_bytes", info.threshold);
        object.put("low_memory", info.lowMemory);
        return object;
    }

    private JSONObject storageJson() throws Exception {
        JSONObject object = new JSONObject();
        File file = logPath != null && logPath.length() > 0 ? new File(logPath) : context.getFilesDir();
        File parent = file.getParentFile();
        if (parent == null) {
            parent = context.getFilesDir();
        }
        object.put("path", parent.getAbsolutePath());
        object.put("usable_space_bytes", parent.getUsableSpace());
        object.put("total_space_bytes", parent.getTotalSpace());
        object.put("log_file_bytes", file.exists() ? file.length() : 0L);
        return object;
    }

    private void appendJsonLine(File file, JSONObject object) throws Exception {
        if (file == null || file.getPath().length() == 0) {
            return;
        }
        File parent = file.getParentFile();
        if (parent != null && !parent.exists()) {
            parent.mkdirs();
        }
        FileWriter writer = new FileWriter(file, true);
        try {
            writer.write(object.toString());
            writer.write('\n');
        } finally {
            writer.close();
        }
    }

    private File watchdogDirectory() {
        File base = context.getExternalFilesDir(null);
        if (base == null) {
            base = context.getFilesDir();
        }
        return new File(base, "device-watchdog");
    }

    private void maybeAcquireWakeLock() {
        releaseWakeLock();
        if (!wakeLockRequested) {
            return;
        }
        PowerManager power = (PowerManager) context.getSystemService(Context.POWER_SERVICE);
        if (power == null) {
            return;
        }
        wakeLock = power.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "RustyXR:DeviceWatchdog");
        wakeLock.setReferenceCounted(false);
        wakeLock.acquire();
    }

    private void releaseWakeLock() {
        if (wakeLock != null) {
            try {
                if (wakeLock.isHeld()) {
                    wakeLock.release();
                }
            } catch (RuntimeException ignored) {
            }
            wakeLock = null;
        }
    }

    private void persistDesiredState(boolean enabled) {
        prefs().edit()
            .putBoolean(PREF_ENABLED, enabled)
            .putLong(PREF_INTERVAL_MS, intervalMs)
            .putBoolean(PREF_WAKE_LOCK, wakeLockRequested)
            .putLong(PREF_MAX_BYTES, maxLogBytes)
            .apply();
    }

    private SharedPreferences prefs() {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    private static long sanitizeInterval(long requested) {
        if (requested <= 0L) {
            return DEFAULT_INTERVAL_MS;
        }
        return Math.max(MIN_INTERVAL_MS, requested);
    }

    private static String cleanRunId(String value) {
        if (value == null) {
            return "";
        }
        StringBuilder builder = new StringBuilder();
        for (int i = 0; i < value.length() && builder.length() < 80; i++) {
            char ch = value.charAt(i);
            if ((ch >= 'a' && ch <= 'z')
                || (ch >= 'A' && ch <= 'Z')
                || (ch >= '0' && ch <= '9')
                || ch == '-'
                || ch == '_'
                || ch == '.') {
                builder.append(ch);
            }
        }
        return builder.toString();
    }

    private static String batteryStatusLabel(int status) {
        switch (status) {
            case BatteryManager.BATTERY_STATUS_CHARGING:
                return "charging";
            case BatteryManager.BATTERY_STATUS_DISCHARGING:
                return "discharging";
            case BatteryManager.BATTERY_STATUS_FULL:
                return "full";
            case BatteryManager.BATTERY_STATUS_NOT_CHARGING:
                return "not_charging";
            default:
                return "unknown";
        }
    }

    private static String thermalStatusLabel(int status) {
        switch (status) {
            case PowerManager.THERMAL_STATUS_NONE:
                return "none";
            case PowerManager.THERMAL_STATUS_LIGHT:
                return "light";
            case PowerManager.THERMAL_STATUS_MODERATE:
                return "moderate";
            case PowerManager.THERMAL_STATUS_SEVERE:
                return "severe";
            case PowerManager.THERMAL_STATUS_CRITICAL:
                return "critical";
            case PowerManager.THERMAL_STATUS_EMERGENCY:
                return "emergency";
            case PowerManager.THERMAL_STATUS_SHUTDOWN:
                return "shutdown";
            default:
                return String.format(Locale.US, "unknown_%d", status);
        }
    }

    private void publishStatusQuietly() {
        if (state == null) {
            return;
        }
        try {
            state.updateDeviceWatchdogStatus(statusJson());
        } catch (Exception ignored) {
        }
    }
}
