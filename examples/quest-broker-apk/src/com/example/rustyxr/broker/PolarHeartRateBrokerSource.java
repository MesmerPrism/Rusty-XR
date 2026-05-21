package com.example.rustyxr.broker;

import android.Manifest;
import android.annotation.SuppressLint;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothGatt;
import android.bluetooth.BluetoothGattCallback;
import android.bluetooth.BluetoothGattCharacteristic;
import android.bluetooth.BluetoothGattDescriptor;
import android.bluetooth.BluetoothManager;
import android.bluetooth.BluetoothProfile;
import android.bluetooth.BluetoothStatusCodes;
import android.bluetooth.le.ScanCallback;
import android.bluetooth.le.ScanResult;
import android.bluetooth.le.ScanSettings;
import android.content.Context;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.ParcelUuid;
import android.os.SystemClock;
import android.util.Base64;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.Closeable;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

final class PolarHeartRateBrokerSource implements Closeable {
    static final String STATUS_SCHEMA = "rusty.xr.bio.polar_hr.status.v1";
    static final String STREAM_ID = "bio:polar_hr_rr";
    private static final String PUBLISHER_CLIENT_ID = "broker_polar_hr";
    private static final long DEFAULT_SCAN_TIMEOUT_MS = 30_000L;
    private static final long MIN_SCAN_TIMEOUT_MS = 3_000L;
    private static final long MAX_SCAN_TIMEOUT_MS = 90_000L;
    private static final int CONNECT_ATTEMPTS = 3;
    private static final long CONNECT_RETRY_DELAY_MS = 1_500L;
    private static final long CONNECT_TIMEOUT_MS = 15_000L;
    private static final long SERVICE_DISCOVERY_TIMEOUT_MS = 12_000L;
    private static final long GATT_OPERATION_TIMEOUT_MS = 6_000L;
    private static final int GATT_START_FAILED = -1;
    private static final int GATT_CONNECTION_TIMEOUT = -2;
    private static final int STRONG_CANDIDATE_SCORE = 60;
    private static final int MAX_SCAN_CANDIDATE_SUMMARIES = 12;

    private final Context context;
    private final BrokerState state;
    private final LocalBrokerServer server;
    private final Object lock = new Object();

    private volatile boolean stopRequested;
    private Thread workerThread;
    private BluetoothGatt gatt;
    private HrGattCallback callback;
    private String requestedDeviceAddress = "";
    private long requestedScanTimeoutMs = DEFAULT_SCAN_TIMEOUT_MS;
    private String statusState = "idle";
    private boolean enabled;
    private String deviceAddress = "";
    private String deviceName = "";
    private int rssi = Integer.MIN_VALUE;
    private boolean heartRateServiceVisible;
    private int heartRateEventCount;
    private int rrIntervalCount;
    private int latestHeartRateBpm;
    private long latestEventUnixNs;
    private long latestEventElapsedNs;
    private String lastError = "";
    private String missingPermissions = "";
    private long scanReportCount;
    private long ignoredScanReportCount;
    private JSONArray recentScanCandidates = new JSONArray();
    private JSONArray notes = new JSONArray();

    PolarHeartRateBrokerSource(Context context, BrokerState state, LocalBrokerServer server) {
        this.context = context.getApplicationContext();
        this.state = state;
        this.server = server;
        publishStatus();
    }

    JSONObject start(String deviceAddress, long scanTimeoutMs) throws Exception {
        synchronized (lock) {
            requestedDeviceAddress = deviceAddress != null ? deviceAddress.trim() : "";
            requestedScanTimeoutMs = clampScanTimeout(scanTimeoutMs);
            enabled = true;
            stopRequested = false;
            this.deviceAddress = "";
            this.deviceName = "";
            rssi = Integer.MIN_VALUE;
            heartRateServiceVisible = false;
            heartRateEventCount = 0;
            rrIntervalCount = 0;
            latestHeartRateBpm = 0;
            latestEventUnixNs = 0L;
            latestEventElapsedNs = 0L;
            lastError = "";
            missingPermissions = "";
            scanReportCount = 0L;
            ignoredScanReportCount = 0L;
            recentScanCandidates = new JSONArray();
            notes = new JSONArray();
            if (workerThread != null && workerThread.isAlive()) {
                statusState = "streaming".equals(statusState) ? statusState : "starting";
                publishStatusLocked();
                return statusJsonLocked();
            }

            workerThread = new Thread(new Runnable() {
                @Override
                public void run() {
                    runWorker();
                }
            }, "RustyXrPolarHeartRateSource");
            workerThread.start();
            statusState = "starting";
            publishStatusLocked();
            return statusJsonLocked();
        }
    }

    JSONObject stop() throws Exception {
        BluetoothGatt closeGatt;
        Thread thread;
        synchronized (lock) {
            enabled = false;
            stopRequested = true;
            thread = workerThread;
            statusState = thread != null && thread.isAlive() ? "stopping" : "stopped";
            closeGatt = gatt;
            publishStatusLocked();
        }

        if (closeGatt != null) {
            try {
                closeGatt.disconnect();
            } catch (Exception ignored) {
            }
        }

        if (thread != null) {
            thread.interrupt();
        }
        return statusJson();
    }

    @Override
    public void close() {
        try {
            stop();
        } catch (Exception ignored) {
        }
    }

    JSONObject statusJson() throws Exception {
        synchronized (lock) {
            return statusJsonLocked();
        }
    }

    @SuppressLint("MissingPermission")
    private void runWorker() {
        BluetoothGatt localGatt = null;
        HrGattCallback localCallback = null;
        try {
            updateState("checking_permissions");
            List<String> missing = missingBluetoothPermissions();
            if (!missing.isEmpty()) {
                fail("permission_blocked", "Missing Bluetooth permission: " + join(missing, ", "));
                return;
            }

            if (!context.getPackageManager().hasSystemFeature(PackageManager.FEATURE_BLUETOOTH_LE)) {
                fail("ble_unavailable", "Android Bluetooth Low Energy support is unavailable.");
                return;
            }

            BluetoothManager manager = (BluetoothManager) context.getSystemService(Context.BLUETOOTH_SERVICE);
            BluetoothAdapter adapter = manager != null ? manager.getAdapter() : null;
            if (adapter == null) {
                fail("adapter_unavailable", "Android did not expose a Bluetooth adapter.");
                return;
            }
            if (!adapter.isEnabled()) {
                fail("bluetooth_disabled", "Bluetooth is disabled.");
                return;
            }

            PolarDeviceCandidate candidate = resolveCandidate(adapter);
            if (candidate == null) {
                fail("scan_timeout", "No Polar-compatible heart-rate advertisement was seen before scan timeout.");
                return;
            }
            applyCandidate(candidate);

            int connectStatus = GATT_START_FAILED;
            for (int attempt = 1; attempt <= CONNECT_ATTEMPTS && !stopRequested; attempt++) {
                if (attempt > 1) {
                    addNote("Retrying Polar HR GATT connection after previous status " + connectStatus + ".");
                    sleepQuietly(CONNECT_RETRY_DELAY_MS);
                }

                updateState("connecting");
                HrGattCallback attemptCallback = new HrGattCallback();
                BluetoothGatt attemptGatt;
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    attemptGatt = candidate.device.connectGatt(context, false, attemptCallback, BluetoothDevice.TRANSPORT_LE);
                } else {
                    attemptGatt = candidate.device.connectGatt(context, false, attemptCallback);
                }

                if (attemptGatt == null) {
                    connectStatus = GATT_START_FAILED;
                    continue;
                }

                connectStatus = awaitInteger(attemptCallback.connectStatuses, CONNECT_TIMEOUT_MS, GATT_CONNECTION_TIMEOUT);
                if (connectStatus == BluetoothGatt.GATT_SUCCESS) {
                    localGatt = attemptGatt;
                    localCallback = attemptCallback;
                    synchronized (lock) {
                        gatt = localGatt;
                        callback = localCallback;
                    }
                    break;
                }

                try {
                    attemptGatt.disconnect();
                    attemptGatt.close();
                } catch (Exception ignored) {
                }
            }

            if (localGatt == null || localCallback == null) {
                fail("connect_failed", "Bluetooth GATT status " + connectStatus + " while connecting.");
                return;
            }

            updateState("discovering_services");
            int serviceStatus = discoverServices(localGatt, localCallback);
            if (serviceStatus != BluetoothGatt.GATT_SUCCESS) {
                fail("service_discovery_failed", "Bluetooth GATT status " + serviceStatus + " while discovering services.");
                return;
            }

            android.bluetooth.BluetoothGattService hrService = localGatt.getService(PolarPmdProtocol.HEART_RATE_SERVICE);
            synchronized (lock) {
                heartRateServiceVisible = hrService != null;
                publishStatusLocked();
            }
            if (hrService == null) {
                fail("heart_rate_service_unavailable", "Connected BLE device did not expose the Heart Rate Service.");
                return;
            }

            BluetoothGattCharacteristic heartRate = hrService.getCharacteristic(PolarPmdProtocol.HEART_RATE_MEASUREMENT);
            if (heartRate == null) {
                fail("heart_rate_measurement_unavailable", "Heart Rate Service did not expose the measurement characteristic.");
                return;
            }

            updateState("enabling_notifications");
            enableCharacteristicUpdates(
                localGatt,
                localCallback,
                heartRate,
                BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE);
            updateState("streaming");

            while (!stopRequested) {
                Integer disconnectStatus = localCallback.disconnectStatuses.poll(1L, TimeUnit.SECONDS);
                if (disconnectStatus != null) {
                    fail("disconnected", "Polar HR GATT disconnected with status " + disconnectStatus.intValue() + ".");
                    return;
                }
            }
        } catch (InterruptedException ignored) {
            Thread.currentThread().interrupt();
        } catch (Exception ex) {
            Log.w(BrokerService.TAG, "Polar HR source failed: " + ex.getMessage(), ex);
            try {
                fail("failed", ex.getClass().getSimpleName() + ": " + ex.getMessage());
            } catch (Exception ignored) {
            }
        } finally {
            if (localGatt != null) {
                try {
                    localGatt.disconnect();
                    localGatt.close();
                } catch (Exception ignored) {
                }
            }
            if (localCallback != null) {
                localCallback.close();
            }

            synchronized (lock) {
                if ("stopping".equals(statusState) || stopRequested) {
                    statusState = "stopped";
                } else if ("streaming".equals(statusState)) {
                    statusState = "disconnected";
                }
                gatt = null;
                callback = null;
                workerThread = null;
                publishStatusLocked();
            }
        }
    }

    private PolarDeviceCandidate resolveCandidate(BluetoothAdapter adapter) throws Exception {
        String requestedAddress;
        long scanTimeout;
        synchronized (lock) {
            requestedAddress = requestedDeviceAddress;
            scanTimeout = requestedScanTimeoutMs;
        }

        if (requestedAddress != null && requestedAddress.trim().length() > 0) {
            try {
                BluetoothDevice device = adapter.getRemoteDevice(requestedAddress.trim());
                return new PolarDeviceCandidate(
                    device,
                    safeDeviceName(device, "Polar-compatible heart-rate sensor"),
                    safeDeviceAddress(device, requestedAddress.trim()),
                    Integer.MIN_VALUE,
                    false,
                    100);
            } catch (Exception ex) {
                addNote("Direct-address Polar HR lookup failed: " + ex.getMessage());
                return null;
            }
        }

        final android.bluetooth.le.BluetoothLeScanner scanner = adapter.getBluetoothLeScanner();
        if (scanner == null) {
            return null;
        }

        final LinkedBlockingQueue<PolarDeviceCandidate> queue = new LinkedBlockingQueue<>();
        final ScanCallback scanCallback = new ScanCallback() {
            @Override
            public void onScanResult(int callbackType, ScanResult result) {
                PolarDeviceCandidate candidate = toPolarCandidate(result);
                if (candidate != null) {
                    queue.offer(candidate);
                }
            }

            @Override
            public void onScanFailed(int errorCode) {
                addNote("BLE scan failed with code " + errorCode + ".");
            }
        };

        updateState("scanning");
        addNote("Scanning for Polar HR candidates. Unnamed BLE devices without Polar name or HR service UUID are ignored.");
        scanner.startScan(
            new ArrayList<android.bluetooth.le.ScanFilter>(),
            new ScanSettings.Builder()
                .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
                .build(),
            scanCallback);
        try {
            PolarDeviceCandidate bestCandidate = null;
            long deadlineMs = SystemClock.elapsedRealtime() + scanTimeout;
            while (SystemClock.elapsedRealtime() < deadlineMs && !stopRequested) {
                long waitMs = Math.max(1L, deadlineMs - SystemClock.elapsedRealtime());
                PolarDeviceCandidate candidate = queue.poll(waitMs, TimeUnit.MILLISECONDS);
                if (candidate == null) {
                    break;
                }

                if (isBetterCandidate(candidate, bestCandidate)) {
                    bestCandidate = candidate;
                }

                if (candidate.matchScore >= STRONG_CANDIDATE_SCORE) {
                    return candidate;
                }
            }

            return bestCandidate;
        } finally {
            try {
                scanner.stopScan(scanCallback);
            } catch (Exception ignored) {
            }
        }
    }

    private int discoverServices(BluetoothGatt localGatt, HrGattCallback localCallback) throws Exception {
        localCallback.serviceStatuses.clear();
        if (!localGatt.discoverServices()) {
            return GATT_START_FAILED;
        }
        return awaitInteger(localCallback.serviceStatuses, SERVICE_DISCOVERY_TIMEOUT_MS, GATT_CONNECTION_TIMEOUT);
    }

    private void enableCharacteristicUpdates(
        BluetoothGatt localGatt,
        HrGattCallback localCallback,
        BluetoothGattCharacteristic characteristic,
        byte[] cccdValue) throws Exception {
        if (!localGatt.setCharacteristicNotification(characteristic, true)) {
            throw new IllegalStateException("Android refused characteristic updates for " + characteristic.getUuid() + ".");
        }

        BluetoothGattDescriptor descriptor = characteristic.getDescriptor(PolarPmdProtocol.CCCD_DESCRIPTOR);
        if (descriptor == null) {
            throw new IllegalStateException("CCCD descriptor missing for " + characteristic.getUuid() + ".");
        }

        localCallback.descriptorWriteStatuses.clear();
        if (!writeDescriptorCompat(localGatt, descriptor, cccdValue)) {
            throw new IllegalStateException("CCCD descriptor write did not start for " + characteristic.getUuid() + ".");
        }
        int status = awaitInteger(localCallback.descriptorWriteStatuses, GATT_OPERATION_TIMEOUT_MS, GATT_CONNECTION_TIMEOUT);
        if (status != BluetoothGatt.GATT_SUCCESS) {
            throw new IllegalStateException("CCCD descriptor write failed for " + characteristic.getUuid() + ": GATT status " + status + ".");
        }
    }

    private void handleHeartRateData(byte[] value) {
        HeartRateMeasurement measurement = decodeHeartRate(value);
        if (measurement == null) {
            synchronized (lock) {
                lastError = "Ignored malformed Heart Rate Measurement length=" + (value != null ? value.length : 0) + ".";
                publishStatusLocked();
            }
            return;
        }

        long sequence;
        String address;
        String name;
        synchronized (lock) {
            heartRateEventCount++;
            rrIntervalCount += measurement.rrIntervalsMs.length();
            latestHeartRateBpm = measurement.heartRateBpm;
            latestEventUnixNs = System.currentTimeMillis() * 1_000_000L;
            latestEventElapsedNs = SystemClock.elapsedRealtimeNanos();
            sequence = heartRateEventCount;
            address = deviceAddress;
            name = deviceName;
            publishStatusLocked();
        }

        try {
            JSONObject payload = heartRatePayload(value, measurement, address, name);
            server.publishLocalStreamEvent(STREAM_ID, sequence, payload, PUBLISHER_CLIENT_ID);
        } catch (Exception ex) {
            synchronized (lock) {
                lastError = "HR publish failed: " + ex.getMessage();
                publishStatusLocked();
            }
            Log.w(BrokerService.TAG, "Polar HR publish failed: " + ex.getMessage(), ex);
        }
    }

    private static HeartRateMeasurement decodeHeartRate(byte[] bytes) {
        if (bytes == null || bytes.length < 2) {
            return null;
        }
        int flags = bytes[0] & 0xff;
        int index = 1;
        int heartRate;
        if ((flags & 0x01) != 0) {
            if (bytes.length < 3) {
                return null;
            }
            heartRate = (bytes[index] & 0xff) | ((bytes[index + 1] & 0xff) << 8);
            index += 2;
        } else {
            heartRate = bytes[index] & 0xff;
            index += 1;
        }

        if ((flags & 0x08) != 0) {
            index += 2;
        }

        JSONArray rr = new JSONArray();
        if ((flags & 0x10) != 0) {
            while (index + 1 < bytes.length) {
                int raw = (bytes[index] & 0xff) | ((bytes[index + 1] & 0xff) << 8);
                try {
                    rr.put(raw * 1000.0d / 1024.0d);
                } catch (Exception ex) {
                    return null;
                }
                index += 2;
            }
        }

        return new HeartRateMeasurement(flags, heartRate, rr);
    }

    private static JSONObject heartRatePayload(
        byte[] rawBytes,
        HeartRateMeasurement measurement,
        String deviceAddress,
        String deviceName) throws Exception {
        long nowUnixNs = System.currentTimeMillis() * 1_000_000L;
        JSONObject payload = new JSONObject();
        payload.put("schema", "rusty.xr.polar.hr_rr.v1");
        payload.put("stream_id", STREAM_ID);
        payload.put("source", "android_ble_hrs");
        payload.put("device_address", deviceAddress != null ? deviceAddress : "");
        payload.put("device_name", deviceName != null ? deviceName : "");
        payload.put("sample_time_unix_ns", nowUnixNs);
        payload.put("sample_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        payload.put("heart_rate_bpm", measurement.heartRateBpm);
        payload.put("rr_intervals_ms", new JSONArray(measurement.rrIntervalsMs.toString()));
        payload.put("rr_count", measurement.rrIntervalsMs.length());
        payload.put("flags", measurement.flags);
        payload.put("payload_base64", Base64.encodeToString(rawBytes, Base64.NO_WRAP));
        payload.put("payload_size_bytes", rawBytes != null ? rawBytes.length : 0);

        JSONObject decoded = new JSONObject();
        decoded.put("bpm", measurement.heartRateBpm);
        decoded.put("rr_count", measurement.rrIntervalsMs.length());
        payload.put("decoded", decoded);
        return payload;
    }

    private void applyCandidate(PolarDeviceCandidate candidate) {
        synchronized (lock) {
            deviceAddress = candidate.deviceAddress;
            deviceName = candidate.deviceName;
            rssi = candidate.rssi;
            heartRateServiceVisible = candidate.heartRateServiceVisible;
            publishStatusLocked();
        }
    }

    private void fail(String errorCode, String message) throws Exception {
        synchronized (lock) {
            statusState = errorCode;
            lastError = message;
            if ("permission_blocked".equals(errorCode)) {
                missingPermissions = message;
            }
            publishStatusLocked();
        }
    }

    private void updateState(String nextState) {
        synchronized (lock) {
            statusState = nextState;
            publishStatusLocked();
        }
    }

    private void addNote(String note) {
        if (note == null || note.length() == 0) {
            return;
        }
        synchronized (lock) {
            notes.put(note);
            publishStatusLocked();
        }
    }

    private void publishStatus() {
        synchronized (lock) {
            publishStatusLocked();
        }
    }

    private void publishStatusLocked() {
        try {
            state.updatePolarHeartRateStatus(statusJsonLocked());
        } catch (Exception ex) {
            Log.w(BrokerService.TAG, "Polar HR status update failed: " + ex.getMessage());
        }
    }

    private JSONObject statusJsonLocked() throws Exception {
        JSONObject status = new JSONObject();
        status.put("schema", STATUS_SCHEMA);
        status.put("enabled", enabled);
        status.put("state", statusState);
        status.put("input_stream", STREAM_ID);
        status.put("requested_device_address", requestedDeviceAddress);
        status.put("scan_timeout_ms", requestedScanTimeoutMs);
        status.put("device_address", deviceAddress);
        status.put("device_name", deviceName);
        if (rssi != Integer.MIN_VALUE) {
            status.put("rssi", rssi);
        }
        status.put("heart_rate_service_visible", heartRateServiceVisible);
        status.put("heart_rate_event_count", heartRateEventCount);
        status.put("rr_interval_count", rrIntervalCount);
        status.put("latest_heart_rate_bpm", latestHeartRateBpm);
        status.put("latest_event_unix_ns", latestEventUnixNs);
        status.put("latest_event_elapsed_ns", latestEventElapsedNs);
        status.put("last_error", lastError);
        status.put("missing_permissions", missingPermissions);
        status.put("scan_report_count", scanReportCount);
        status.put("ignored_scan_report_count", ignoredScanReportCount);
        status.put("recent_scan_candidates", new JSONArray(recentScanCandidates.toString()));
        status.put("notes", new JSONArray(notes.toString()));
        JSONArray limitations = new JSONArray();
        limitations.put("requires_android_ble_permissions");
        limitations.put("uses_standard_heart_rate_service_only");
        limitations.put("does_not_open_polar_pmd_control_or_data_characteristics");
        status.put("limitations", limitations);
        return status;
    }

    private List<String> missingBluetoothPermissions() {
        List<String> permissions;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            permissions = Arrays.asList(
                Manifest.permission.BLUETOOTH_SCAN,
                Manifest.permission.BLUETOOTH_CONNECT);
        } else {
            permissions = Arrays.asList(
                Manifest.permission.ACCESS_FINE_LOCATION,
                Manifest.permission.ACCESS_COARSE_LOCATION);
        }

        List<String> missing = new ArrayList<>();
        for (String permission : permissions) {
            if (context.checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
                missing.add(permission);
            }
        }
        synchronized (lock) {
            missingPermissions = join(missing, ", ");
            publishStatusLocked();
        }
        return missing;
    }

    private PolarDeviceCandidate toPolarCandidate(ScanResult result) {
        if (result == null || result.getDevice() == null) {
            return null;
        }

        String advertisedName = result.getScanRecord() != null ? result.getScanRecord().getDeviceName() : "";
        String deviceNameFromGatt = safeDeviceName(result.getDevice(), "");
        String matchedName = advertisedName != null && advertisedName.length() > 0
            ? advertisedName
            : deviceNameFromGatt;
        boolean hasHeartRate = advertisesService(result, PolarPmdProtocol.HEART_RATE_SERVICE);
        int matchScore = candidateScore(matchedName, hasHeartRate);
        String displayName = matchedName != null && matchedName.length() > 0
            ? matchedName
            : "Unnamed BLE device";
        if (matchScore <= 0) {
            recordScanCandidate(result, displayName, hasHeartRate, matchScore, false);
            return null;
        }

        recordScanCandidate(result, displayName, hasHeartRate, matchScore, true);
        return new PolarDeviceCandidate(
            result.getDevice(),
            displayName,
            safeDeviceAddress(result.getDevice(), ""),
            result.getRssi(),
            hasHeartRate,
            matchScore);
    }

    private boolean advertisesService(ScanResult result, UUID uuid) {
        if (result.getScanRecord() == null) {
            return false;
        }

        ParcelUuid parcelUuid = new ParcelUuid(uuid);
        return (result.getScanRecord().getServiceUuids() != null
                && result.getScanRecord().getServiceUuids().contains(parcelUuid))
            || (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q
                && result.getScanRecord().getServiceSolicitationUuids() != null
                && result.getScanRecord().getServiceSolicitationUuids().contains(parcelUuid));
    }

    private void recordScanCandidate(
        ScanResult result,
        String displayName,
        boolean hasHeartRate,
        int matchScore,
        boolean accepted) {
        synchronized (lock) {
            scanReportCount++;
            if (!accepted) {
                ignoredScanReportCount++;
            }

            try {
                JSONObject candidate = new JSONObject();
                candidate.put("accepted", accepted);
                candidate.put("name", displayName != null ? displayName : "");
                candidate.put("address", safeDeviceAddress(result.getDevice(), ""));
                candidate.put("rssi", result.getRssi());
                candidate.put("heart_rate_service", hasHeartRate);
                candidate.put("match_score", matchScore);
                appendRecentScanCandidateLocked(candidate);
                publishStatusLocked();
            } catch (Exception ex) {
                lastError = "Scan candidate summary failed: " + ex.getMessage();
                publishStatusLocked();
            }
        }
    }

    private void appendRecentScanCandidateLocked(JSONObject candidate) {
        String address = candidate.optString("address", "");
        String name = candidate.optString("name", "");
        boolean accepted = candidate.optBoolean("accepted", false);

        for (int i = recentScanCandidates.length() - 1; i >= 0; i--) {
            JSONObject existing = recentScanCandidates.optJSONObject(i);
            if (existing == null || existing.optBoolean("accepted", false) != accepted) {
                continue;
            }

            String existingAddress = existing.optString("address", "");
            boolean sameAddress = address.length() > 0 && address.equals(existingAddress);
            boolean sameUnnamedBucket = address.length() == 0
                && existingAddress.length() == 0
                && name.equals(existing.optString("name", ""));
            if (sameAddress || sameUnnamedBucket) {
                recentScanCandidates.remove(i);
            }
        }

        while (recentScanCandidates.length() >= MAX_SCAN_CANDIDATE_SUMMARIES) {
            recentScanCandidates.remove(0);
        }
        recentScanCandidates.put(candidate);
    }

    private static int candidateScore(String deviceName, boolean hasHeartRate) {
        int score = 0;
        if (deviceName != null && deviceName.toLowerCase(java.util.Locale.ROOT).contains("polar")) {
            score += 80;
        }
        if (hasHeartRate) {
            score += 60;
        }
        return score;
    }

    private static boolean isBetterCandidate(PolarDeviceCandidate candidate, PolarDeviceCandidate currentBest) {
        if (candidate == null) {
            return false;
        }
        if (currentBest == null) {
            return true;
        }
        if (candidate.matchScore != currentBest.matchScore) {
            return candidate.matchScore > currentBest.matchScore;
        }
        return candidate.rssi > currentBest.rssi;
    }

    @SuppressLint("MissingPermission")
    private static String safeDeviceName(BluetoothDevice device, String fallback) {
        try {
            String name = device != null ? device.getName() : "";
            return name != null && name.length() > 0 ? name : fallback;
        } catch (SecurityException ex) {
            return fallback;
        }
    }

    @SuppressLint("MissingPermission")
    private static String safeDeviceAddress(BluetoothDevice device, String fallback) {
        try {
            String address = device != null ? device.getAddress() : "";
            return address != null && address.length() > 0 ? address : fallback;
        } catch (SecurityException ex) {
            return fallback;
        }
    }

    private static int awaitInteger(LinkedBlockingQueue<Integer> queue, long timeoutMs, int fallback) throws Exception {
        Integer value = queue.poll(timeoutMs, TimeUnit.MILLISECONDS);
        return value != null ? value.intValue() : fallback;
    }

    @SuppressLint("MissingPermission")
    private static boolean writeDescriptorCompat(
        BluetoothGatt localGatt,
        BluetoothGattDescriptor descriptor,
        byte[] value) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            return localGatt.writeDescriptor(descriptor, value) == BluetoothStatusCodes.SUCCESS;
        }

        descriptor.setValue(value);
        return localGatt.writeDescriptor(descriptor);
    }

    private static long clampScanTimeout(long value) {
        long requested = value > 0L ? value : DEFAULT_SCAN_TIMEOUT_MS;
        return Math.max(MIN_SCAN_TIMEOUT_MS, Math.min(MAX_SCAN_TIMEOUT_MS, requested));
    }

    private static String join(List<String> values, String delimiter) {
        if (values == null || values.isEmpty()) {
            return "";
        }
        StringBuilder builder = new StringBuilder();
        for (int i = 0; i < values.size(); i++) {
            if (i > 0) {
                builder.append(delimiter);
            }
            builder.append(values.get(i));
        }
        return builder.toString();
    }

    private static void sleepQuietly(long delayMs) {
        try {
            Thread.sleep(delayMs);
        } catch (InterruptedException ignored) {
            Thread.currentThread().interrupt();
        }
    }

    private final class HrGattCallback extends BluetoothGattCallback {
        final LinkedBlockingQueue<Integer> connectStatuses = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<Integer> disconnectStatuses = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<Integer> serviceStatuses = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<Integer> descriptorWriteStatuses = new LinkedBlockingQueue<>();

        @Override
        public void onConnectionStateChange(BluetoothGatt gatt, int status, int newState) {
            if (newState == BluetoothProfile.STATE_CONNECTED || status != BluetoothGatt.GATT_SUCCESS) {
                connectStatuses.offer(Integer.valueOf(status));
            }
            if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                disconnectStatuses.offer(Integer.valueOf(status));
            }
        }

        @Override
        public void onServicesDiscovered(BluetoothGatt gatt, int status) {
            serviceStatuses.offer(Integer.valueOf(status));
        }

        @Override
        public void onDescriptorWrite(BluetoothGatt gatt, BluetoothGattDescriptor descriptor, int status) {
            descriptorWriteStatuses.offer(Integer.valueOf(status));
        }

        @Override
        public void onCharacteristicChanged(BluetoothGatt gatt, BluetoothGattCharacteristic characteristic) {
            handleNotification(characteristic, characteristic.getValue());
        }

        @Override
        public void onCharacteristicChanged(
            BluetoothGatt gatt,
            BluetoothGattCharacteristic characteristic,
            byte[] value) {
            handleNotification(characteristic, value);
        }

        void close() {
            descriptorWriteStatuses.clear();
        }

        private void handleNotification(BluetoothGattCharacteristic characteristic, byte[] value) {
            if (characteristic == null || value == null) {
                return;
            }
            if (PolarPmdProtocol.HEART_RATE_MEASUREMENT.equals(characteristic.getUuid())) {
                handleHeartRateData(value.clone());
            }
        }
    }

    private static final class PolarDeviceCandidate {
        final BluetoothDevice device;
        final String deviceName;
        final String deviceAddress;
        final int rssi;
        final boolean heartRateServiceVisible;
        final int matchScore;

        PolarDeviceCandidate(
            BluetoothDevice device,
            String deviceName,
            String deviceAddress,
            int rssi,
            boolean heartRateServiceVisible,
            int matchScore) {
            this.device = device;
            this.deviceName = deviceName;
            this.deviceAddress = deviceAddress;
            this.rssi = rssi;
            this.heartRateServiceVisible = heartRateServiceVisible;
            this.matchScore = matchScore;
        }
    }

    private static final class HeartRateMeasurement {
        final int flags;
        final int heartRateBpm;
        final JSONArray rrIntervalsMs;

        HeartRateMeasurement(int flags, int heartRateBpm, JSONArray rrIntervalsMs) {
            this.flags = flags;
            this.heartRateBpm = heartRateBpm;
            this.rrIntervalsMs = rrIntervalsMs;
        }
    }
}
