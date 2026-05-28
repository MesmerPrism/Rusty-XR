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

final class PolarPmdBrokerSource implements Closeable {
    static final String STATUS_SCHEMA = "rusty.xr.bio.polar_pmd.status.v1";
    static final String PMD_STREAM_ACC = "acc";
    static final String PMD_STREAM_ECG = "ecg";
    static final String PMD_STREAM_ACC_ID = "bio:polar_acc";
    static final String PMD_STREAM_ECG_ID = "bio:polar_ecg";
    private static final String PUBLISHER_CLIENT_ID = "broker_polar_pmd";
    private static final long DEFAULT_SCAN_TIMEOUT_MS = 30_000L;
    private static final long MIN_SCAN_TIMEOUT_MS = 3_000L;
    private static final long MAX_SCAN_TIMEOUT_MS = 90_000L;
    private static final int PREFERRED_MTU = 232;
    private static final int CONNECT_ATTEMPTS = 3;
    private static final long CONNECT_RETRY_DELAY_MS = 1_500L;
    private static final long CONNECT_TIMEOUT_MS = 15_000L;
    private static final long SERVICE_DISCOVERY_TIMEOUT_MS = 12_000L;
    private static final long MTU_TIMEOUT_MS = 5_000L;
    private static final long GATT_OPERATION_TIMEOUT_MS = 6_000L;
    private static final long CONTROL_RESPONSE_TIMEOUT_MS = 8_000L;
    private static final int GATT_START_FAILED = -1;
    private static final int GATT_CONNECTION_TIMEOUT = -2;
    private static final int STRONG_CANDIDATE_SCORE = 80;
    private static final int MAX_SCAN_CANDIDATE_SUMMARIES = 12;

    private final Context context;
    private final BrokerState state;
    private final LocalBrokerServer server;
    private final Object lock = new Object();

    private volatile boolean stopRequested;
    private Thread workerThread;
    private BluetoothGatt gatt;
    private PmdGattCallback callback;
    private BluetoothGattCharacteristic controlPoint;
    private boolean pmdStreamStarted;
    private byte activeMeasurementType = PolarPmdProtocol.MEASUREMENT_TYPE_ACC;
    private String requestedDeviceAddress = "";
    private long requestedScanTimeoutMs = DEFAULT_SCAN_TIMEOUT_MS;
    private String requestedPmdStream = PMD_STREAM_ACC;
    private int requestedAccSampleRateHz = 200;
    private boolean requestedHighConnectionPriority;
    private String statusState = "idle";
    private boolean enabled;
    private String deviceAddress = "";
    private String deviceName = "";
    private int rssi = Integer.MIN_VALUE;
    private boolean heartRateServiceVisible;
    private boolean pmdServiceVisible;
    private int batteryPercent = -1;
    private int negotiatedMtu;
    private long accFrameCount;
    private long accSampleCount;
    private long ecgFrameCount;
    private long ecgSampleCount;
    private long malformedFrameCount;
    private long latestFrameUnixNs;
    private long latestFrameElapsedNs;
    private long latestSensorTimestampNs;
    private int latestSampleCount;
    private String lastError = "";
    private String missingPermissions = "";
    private long scanReportCount;
    private long ignoredScanReportCount;
    private JSONArray recentScanCandidates = new JSONArray();
    private JSONArray controlResponses = new JSONArray();
    private JSONArray settings = new JSONArray();
    private JSONArray notes = new JSONArray();

    PolarPmdBrokerSource(Context context, BrokerState state, LocalBrokerServer server) {
        this.context = context.getApplicationContext();
        this.state = state;
        this.server = server;
        publishStatus();
    }

    JSONObject start(String deviceAddress, long scanTimeoutMs) throws Exception {
        return start(deviceAddress, scanTimeoutMs, PMD_STREAM_ACC, false, 200);
    }

    JSONObject start(
        String deviceAddress,
        long scanTimeoutMs,
        String pmdStream,
        boolean requestHighConnectionPriority,
        int accSampleRateHz) throws Exception {
        synchronized (lock) {
            requestedDeviceAddress = deviceAddress != null ? deviceAddress.trim() : "";
            requestedScanTimeoutMs = clampScanTimeout(scanTimeoutMs);
            requestedPmdStream = normalizePmdStream(pmdStream);
            requestedAccSampleRateHz = normalizeAccSampleRate(accSampleRateHz);
            requestedHighConnectionPriority = requestHighConnectionPriority;
            enabled = true;
            stopRequested = false;
            this.deviceAddress = "";
            this.deviceName = "";
            rssi = Integer.MIN_VALUE;
            heartRateServiceVisible = false;
            pmdServiceVisible = false;
            batteryPercent = -1;
            negotiatedMtu = 0;
            accFrameCount = 0L;
            accSampleCount = 0L;
            ecgFrameCount = 0L;
            ecgSampleCount = 0L;
            malformedFrameCount = 0L;
            latestFrameUnixNs = 0L;
            latestFrameElapsedNs = 0L;
            latestSensorTimestampNs = 0L;
            latestSampleCount = 0;
            lastError = "";
            missingPermissions = "";
            scanReportCount = 0L;
            ignoredScanReportCount = 0L;
            recentScanCandidates = new JSONArray();
            controlResponses = new JSONArray();
            settings = new JSONArray();
            notes = new JSONArray();
            activeMeasurementType = measurementTypeForStream(requestedPmdStream);
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
            }, "RustyXrPolarPmdSource");
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
        PmdGattCallback localCallback = null;
        BluetoothGattCharacteristic localControlPoint = null;
        String streamKind;
        int accSampleRateHz;
        boolean highConnectionPriority;
        byte measurementType;
        synchronized (lock) {
            streamKind = requestedPmdStream;
            accSampleRateHz = requestedAccSampleRateHz;
            highConnectionPriority = requestedHighConnectionPriority;
            measurementType = activeMeasurementType;
        }
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
                fail("scan_timeout", "No Polar-compatible BLE advertisement was seen before scan timeout.");
                return;
            }
            applyCandidate(candidate);

            int connectStatus = GATT_START_FAILED;
            for (int attempt = 1; attempt <= CONNECT_ATTEMPTS && !stopRequested; attempt++) {
                if (attempt > 1) {
                    addNote("Retrying Polar GATT connection after previous status " + connectStatus + ".");
                    sleepQuietly(CONNECT_RETRY_DELAY_MS);
                }

                updateState("connecting");
                PmdGattCallback attemptCallback = new PmdGattCallback();
                BluetoothGatt attemptGatt;
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    attemptGatt = candidate.device.connectGatt(context, false, attemptCallback, BluetoothDevice.TRANSPORT_LE);
                } else {
                    attemptGatt = candidate.device.connectGatt(context, false, attemptCallback);
                }

                if (attemptGatt == null) {
                    connectStatus = GATT_START_FAILED;
                    attemptCallback.close();
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
                } catch (Exception ignored) {
                }
                try {
                    attemptGatt.close();
                } catch (Exception ignored) {
                }
                attemptCallback.close();
            }

            if (localGatt == null || localCallback == null) {
                fail("connect_failed", "Bluetooth GATT status " + connectStatus + " while connecting.");
                return;
            }

            if (highConnectionPriority) {
                requestHighConnectionPriority(localGatt);
            }

            updateState("negotiating_mtu");
            Integer mtu = requestMtu(localGatt, localCallback);
            if (mtu != null && mtu.intValue() > 0) {
                synchronized (lock) {
                    negotiatedMtu = mtu.intValue();
                    publishStatusLocked();
                }
            }

            updateState("discovering_services");
            int serviceStatus = discoverServices(localGatt, localCallback);
            if (serviceStatus != BluetoothGatt.GATT_SUCCESS) {
                fail("service_discovery_failed", "Bluetooth GATT status " + serviceStatus + " while discovering services.");
                return;
            }

            boolean hasHeartRate = localGatt.getService(PolarPmdProtocol.HEART_RATE_SERVICE) != null;
            android.bluetooth.BluetoothGattService pmdService = localGatt.getService(PolarPmdProtocol.PMD_SERVICE);
            synchronized (lock) {
                heartRateServiceVisible = hasHeartRate;
                pmdServiceVisible = pmdService != null;
                publishStatusLocked();
            }
            if (pmdService == null) {
                fail("pmd_service_unavailable", "Connected BLE device did not expose the Polar PMD service.");
                return;
            }

            Integer battery = readBatteryPercent(localGatt, localCallback);
            if (battery != null) {
                synchronized (lock) {
                    batteryPercent = battery.intValue();
                    publishStatusLocked();
                }
            }

            localControlPoint = pmdService.getCharacteristic(PolarPmdProtocol.PMD_CONTROL_POINT);
            BluetoothGattCharacteristic data = pmdService.getCharacteristic(PolarPmdProtocol.PMD_DATA);
            if (localControlPoint == null) {
                fail("control_point_unavailable", "PMD service did not expose the control point characteristic.");
                return;
            }
            if (data == null) {
                fail("data_characteristic_unavailable", "PMD service did not expose the data notification characteristic.");
                return;
            }
            synchronized (lock) {
                controlPoint = localControlPoint;
                publishStatusLocked();
            }

            updateState("enabling_notifications");
            enableCharacteristicUpdates(
                localGatt,
                localCallback,
                localControlPoint,
                BluetoothGattDescriptor.ENABLE_INDICATION_VALUE);
            enableCharacteristicUpdates(
                localGatt,
                localCallback,
                data,
                BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE);

            updateState("reading_settings");
            byte[] settingsResponse = sendPmdCommand(
                localGatt,
                localCallback,
                localControlPoint,
                PolarPmdProtocol.buildGetSettingsRequest(measurementType),
                PolarPmdProtocol.OPCODE_GET_SETTINGS,
                measurementType);
            PolarPmdProtocol.SettingsSummary settingsSummary = PolarPmdProtocol.parseSettingsResponse(settingsResponse);
            if (settingsSummary != null) {
                addSettings(settingsSummary.toJson());
            }

            updateState("starting_stream");
            byte[] startCommand = PolarPmdProtocol.MEASUREMENT_TYPE_ECG == measurementType
                ? PolarPmdProtocol.buildStartEcgRequest(130, 14)
                : PolarPmdProtocol.buildStartAccRequest(accSampleRateHz, 16, 8);
            sendPmdCommand(
                localGatt,
                localCallback,
                localControlPoint,
                startCommand,
                PolarPmdProtocol.OPCODE_START_STREAM,
                measurementType);
            pmdStreamStarted = true;
            addNote("Started Polar PMD " + streamKind + " stream.");
            updateState("streaming");

            while (!stopRequested) {
                Integer disconnectStatus = localCallback.disconnectStatuses.poll(1L, TimeUnit.SECONDS);
                if (disconnectStatus != null) {
                    fail("disconnected", "Polar GATT disconnected with status " + disconnectStatus.intValue() + ".");
                    return;
                }
            }
        } catch (InterruptedException ignored) {
            Thread.currentThread().interrupt();
        } catch (Exception ex) {
            Log.w(BrokerService.TAG, "Polar PMD source failed: " + ex.getMessage(), ex);
            try {
                fail("failed", ex.getClass().getSimpleName() + ": " + ex.getMessage());
            } catch (Exception ignored) {
            }
        } finally {
            if (pmdStreamStarted && localGatt != null && localCallback != null && localControlPoint != null) {
                try {
                    sendPmdCommand(
                        localGatt,
                        localCallback,
                        localControlPoint,
                        PolarPmdProtocol.buildStopRequest(measurementType),
                        PolarPmdProtocol.OPCODE_STOP_STREAM,
                        measurementType);
                } catch (Exception ex) {
                    addNote(streamKind.toUpperCase(java.util.Locale.ROOT) + " stop command failed: " + ex.getMessage());
                }
            }

            if (localGatt != null) {
                try {
                    localGatt.disconnect();
                } catch (Exception ignored) {
                }
                try {
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
                controlPoint = null;
                pmdStreamStarted = false;
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
                    safeDeviceName(device, "Polar-compatible BLE sensor"),
                    safeDeviceAddress(device, requestedAddress.trim()),
                    Integer.MIN_VALUE,
                    false,
                    false,
                    100);
            } catch (Exception ex) {
                addNote("Direct-address Polar lookup failed: " + ex.getMessage());
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
        addNote("Scanning for Polar PMD candidates. Unnamed BLE devices without Polar name or Polar/HR service UUIDs are ignored.");
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

    private Integer requestMtu(BluetoothGatt localGatt, PmdGattCallback localCallback) throws Exception {
        localCallback.mtuValues.clear();
        if (!localGatt.requestMtu(PREFERRED_MTU)) {
            addNote("Android refused to start MTU negotiation.");
            return null;
        }
        Integer mtu = localCallback.mtuValues.poll(MTU_TIMEOUT_MS, TimeUnit.MILLISECONDS);
        if (mtu == null) {
            addNote("MTU negotiation timed out; continuing with Android's current MTU.");
        }
        return mtu;
    }

    private void requestHighConnectionPriority(BluetoothGatt localGatt) {
        if (localGatt == null) {
            return;
        }
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.LOLLIPOP) {
            addNote("Android BLE connection-priority request is unavailable on this API level.");
            return;
        }
        try {
            boolean started = localGatt.requestConnectionPriority(BluetoothGatt.CONNECTION_PRIORITY_HIGH);
            addNote(started
                ? "Requested Android high BLE connection priority for Polar PMD diagnostics."
                : "Android refused to start high BLE connection-priority request.");
        } catch (Exception ex) {
            addNote("Android high BLE connection-priority request failed: " + ex.getMessage());
        }
    }

    private int discoverServices(BluetoothGatt localGatt, PmdGattCallback localCallback) throws Exception {
        localCallback.serviceStatuses.clear();
        if (!localGatt.discoverServices()) {
            return GATT_START_FAILED;
        }
        return awaitInteger(localCallback.serviceStatuses, SERVICE_DISCOVERY_TIMEOUT_MS, GATT_CONNECTION_TIMEOUT);
    }

    private Integer readBatteryPercent(BluetoothGatt localGatt, PmdGattCallback localCallback) throws Exception {
        android.bluetooth.BluetoothGattService batteryService = localGatt.getService(PolarPmdProtocol.BATTERY_SERVICE);
        BluetoothGattCharacteristic characteristic = batteryService != null
            ? batteryService.getCharacteristic(PolarPmdProtocol.BATTERY_LEVEL)
            : null;
        if (characteristic == null) {
            return null;
        }

        localCallback.readResults.clear();
        if (!localGatt.readCharacteristic(characteristic)) {
            addNote("Battery characteristic read did not start.");
            return null;
        }
        CharacteristicReadResult read = localCallback.readResults.poll(GATT_OPERATION_TIMEOUT_MS, TimeUnit.MILLISECONDS);
        if (read == null || read.status != BluetoothGatt.GATT_SUCCESS || read.value.length == 0) {
            addNote("Battery characteristic read failed.");
            return null;
        }
        return Integer.valueOf(read.value[0] & 0xff);
    }

    private void enableCharacteristicUpdates(
        BluetoothGatt localGatt,
        PmdGattCallback localCallback,
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

    private byte[] sendPmdCommand(
        BluetoothGatt localGatt,
        PmdGattCallback localCallback,
        BluetoothGattCharacteristic localControlPoint,
        byte[] command,
        byte expectedOpCode,
        byte expectedMeasurementType) throws Exception {
        localCallback.controlNotifications.clear();
        localCallback.characteristicWriteStatuses.clear();
        if (!writeCharacteristicCompat(localGatt, localControlPoint, command)) {
            throw new IllegalStateException("PMD command write did not start.");
        }
        int writeStatus = awaitInteger(localCallback.characteristicWriteStatuses, GATT_OPERATION_TIMEOUT_MS, GATT_CONNECTION_TIMEOUT);
        if (writeStatus != BluetoothGatt.GATT_SUCCESS) {
            throw new IllegalStateException("PMD command write failed: GATT status " + writeStatus + ".");
        }

        byte[] responseBytes = awaitControlResponse(localCallback, expectedOpCode, expectedMeasurementType);
        PolarPmdProtocol.ControlResponse response = PolarPmdProtocol.parseControlResponse(responseBytes);
        if (response == null) {
            throw new IllegalStateException("PMD control response was malformed.");
        }
        addControlResponse(response.toJson());
        if (!response.success()) {
            throw new IllegalStateException(
                "PMD control response failed for op=" + response.opCode
                    + " measurement=" + response.measurementType
                    + ": error=" + response.errorCode + ".");
        }
        return responseBytes;
    }

    private byte[] awaitControlResponse(
        PmdGattCallback localCallback,
        byte expectedOpCode,
        byte expectedMeasurementType) throws Exception {
        int expectedOp = expectedOpCode & 0xff;
        int expectedType = expectedMeasurementType & 0xff;
        long deadline = SystemClock.elapsedRealtime() + CONTROL_RESPONSE_TIMEOUT_MS;
        while (SystemClock.elapsedRealtime() < deadline) {
            long waitMs = Math.max(1L, deadline - SystemClock.elapsedRealtime());
            byte[] bytes = localCallback.controlNotifications.poll(waitMs, TimeUnit.MILLISECONDS);
            if (bytes == null) {
                break;
            }
            PolarPmdProtocol.ControlResponse response = PolarPmdProtocol.parseControlResponse(bytes);
            if (response != null && response.opCode == expectedOp && response.measurementType == expectedType) {
                return bytes;
            }
        }
        throw new IllegalStateException("Timed out waiting for PMD control response.");
    }

    private void handlePmdData(byte[] value) {
        if (value == null || value.length == 0) {
            return;
        }
        if (value[0] == PolarPmdProtocol.MEASUREMENT_TYPE_ECG) {
            handleEcgData(value);
            return;
        }
        if (value[0] != PolarPmdProtocol.MEASUREMENT_TYPE_ACC) {
            return;
        }

        PolarPmdProtocol.AccFrame frame = PolarPmdProtocol.decodeAccFrame(value);
        if (frame == null) {
            synchronized (lock) {
                malformedFrameCount++;
                lastError = "Ignored malformed ACC PMD frame length=" + value.length + ".";
                publishStatusLocked();
            }
            return;
        }

        long sequence;
        String address;
        String name;
        synchronized (lock) {
            accFrameCount++;
            accSampleCount += frame.samples.size();
            latestFrameUnixNs = System.currentTimeMillis() * 1_000_000L;
            latestFrameElapsedNs = SystemClock.elapsedRealtimeNanos();
            latestSensorTimestampNs = frame.sensorTimestampNs;
            latestSampleCount = frame.samples.size();
            sequence = accFrameCount;
            address = deviceAddress;
            name = deviceName;
            publishStatusLocked();
        }

        try {
            JSONObject payload = PolarPmdProtocol.accFramePayload(value, frame, address, name);
            JSONObject result = server.publishLocalStreamEvent(
                BreathAssessmentState.POLAR_INPUT_STREAM,
                sequence,
                payload,
                PUBLISHER_CLIENT_ID);
            JSONObject breathProcessing = result.optJSONObject("breath_assessment");
            if (breathProcessing != null && breathProcessing.optBoolean("accepted", false)) {
                JSONObject assessment = breathProcessing.optJSONObject("assessment");
                String source = assessment != null ? assessment.optString("source", "") : "";
                if (sequence <= 3L || sequence % 60L == 0L) {
                    Log.i(
                        BrokerService.TAG,
                        "Polar PMD ACC frame=" + sequence
                            + " samples=" + frame.samples.size()
                            + " breath_source=" + source);
                }
            }
        } catch (Exception ex) {
            synchronized (lock) {
                lastError = "ACC publish failed: " + ex.getMessage();
                publishStatusLocked();
            }
            Log.w(BrokerService.TAG, "Polar PMD ACC publish failed: " + ex.getMessage(), ex);
        }
    }

    private void handleEcgData(byte[] value) {
        PolarPmdProtocol.EcgFrame frame = PolarPmdProtocol.decodeEcgFrame(value);
        if (frame == null) {
            synchronized (lock) {
                malformedFrameCount++;
                lastError = "Ignored malformed ECG PMD frame length=" + value.length + ".";
                publishStatusLocked();
            }
            return;
        }

        long sequence;
        String address;
        String name;
        synchronized (lock) {
            ecgFrameCount++;
            ecgSampleCount += frame.samplesMicrovolts.size();
            latestFrameUnixNs = System.currentTimeMillis() * 1_000_000L;
            latestFrameElapsedNs = SystemClock.elapsedRealtimeNanos();
            latestSensorTimestampNs = frame.sensorTimestampNs;
            latestSampleCount = frame.samplesMicrovolts.size();
            sequence = ecgFrameCount;
            address = deviceAddress;
            name = deviceName;
            publishStatusLocked();
        }

        try {
            JSONObject payload = PolarPmdProtocol.ecgFramePayload(value, frame, address, name);
            server.publishLocalStreamEvent(PMD_STREAM_ECG_ID, sequence, payload, PUBLISHER_CLIENT_ID);
            if (sequence <= 3L || sequence % 30L == 0L) {
                Log.i(
                    BrokerService.TAG,
                    "Polar PMD ECG frame=" + sequence
                        + " samples=" + frame.samplesMicrovolts.size());
            }
        } catch (Exception ex) {
            synchronized (lock) {
                lastError = "ECG publish failed: " + ex.getMessage();
                publishStatusLocked();
            }
            Log.w(BrokerService.TAG, "Polar PMD ECG publish failed: " + ex.getMessage(), ex);
        }
    }

    private void applyCandidate(PolarDeviceCandidate candidate) {
        synchronized (lock) {
            deviceAddress = candidate.deviceAddress;
            deviceName = candidate.deviceName;
            rssi = candidate.rssi;
            heartRateServiceVisible = candidate.heartRateServiceVisible;
            pmdServiceVisible = candidate.pmdServiceVisible;
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

    private void addControlResponse(JSONObject response) {
        synchronized (lock) {
            controlResponses.put(response);
            publishStatusLocked();
        }
    }

    private void addSettings(JSONObject setting) {
        synchronized (lock) {
            settings.put(setting);
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
            state.updatePolarPmdStatus(statusJsonLocked());
        } catch (Exception ex) {
            Log.w(BrokerService.TAG, "Polar PMD status update failed: " + ex.getMessage());
        }
    }

    private JSONObject statusJsonLocked() throws Exception {
        JSONObject status = new JSONObject();
        status.put("schema", STATUS_SCHEMA);
        status.put("enabled", enabled);
        status.put("state", statusState);
        status.put("input_stream", BreathAssessmentState.POLAR_INPUT_STREAM);
        status.put("requested_pmd_stream", requestedPmdStream);
        status.put("active_pmd_stream", streamForMeasurementType(activeMeasurementType));
        status.put("active_measurement_type", activeMeasurementType & 0xff);
        status.put("requested_acc_sample_rate_hz", requestedAccSampleRateHz);
        status.put("requested_high_connection_priority", requestedHighConnectionPriority);
        status.put("output_stream", BreathAssessmentState.OUTPUT_STREAM);
        status.put("requested_device_address", requestedDeviceAddress);
        status.put("scan_timeout_ms", requestedScanTimeoutMs);
        status.put("device_address", deviceAddress);
        status.put("device_name", deviceName);
        if (rssi != Integer.MIN_VALUE) {
            status.put("rssi", rssi);
        }
        status.put("heart_rate_service_visible", heartRateServiceVisible);
        status.put("pmd_service_visible", pmdServiceVisible);
        if (batteryPercent >= 0) {
            status.put("battery_percent", batteryPercent);
        }
        if (negotiatedMtu > 0) {
            status.put("negotiated_mtu", negotiatedMtu);
        }
        status.put("acc_frame_count", accFrameCount);
        status.put("acc_sample_count", accSampleCount);
        status.put("ecg_frame_count", ecgFrameCount);
        status.put("ecg_sample_count", ecgSampleCount);
        status.put("malformed_frame_count", malformedFrameCount);
        status.put("latest_frame_unix_ns", latestFrameUnixNs);
        status.put("latest_frame_elapsed_ns", latestFrameElapsedNs);
        status.put("latest_sensor_timestamp_ns", latestSensorTimestampNs);
        status.put("latest_sample_count", latestSampleCount);
        status.put("last_error", lastError);
        status.put("missing_permissions", missingPermissions);
        status.put("scan_report_count", scanReportCount);
        status.put("ignored_scan_report_count", ignoredScanReportCount);
        status.put("recent_scan_candidates", new JSONArray(recentScanCandidates.toString()));
        status.put("control_responses", new JSONArray(controlResponses.toString()));
        status.put("settings", new JSONArray(settings.toString()));
        status.put("notes", new JSONArray(notes.toString()));
        JSONArray limitations = new JSONArray();
        limitations.put("requires_android_ble_permissions");
        limitations.put("requires_polar_pmd_compatible_sensor");
        limitations.put("diagnostic_breath_estimate_not_medical");
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
        boolean hasPmd = advertisesService(result, PolarPmdProtocol.PMD_SERVICE);
        int matchScore = candidateScore(matchedName, hasHeartRate, hasPmd);
        String displayName = matchedName != null && matchedName.length() > 0
            ? matchedName
            : "Unnamed BLE device";
        if (matchScore <= 0) {
            recordScanCandidate(result, displayName, hasHeartRate, hasPmd, matchScore, false);
            return null;
        }

        recordScanCandidate(result, displayName, hasHeartRate, hasPmd, matchScore, true);
        return new PolarDeviceCandidate(
            result.getDevice(),
            displayName,
            safeDeviceAddress(result.getDevice(), ""),
            result.getRssi(),
            hasHeartRate,
            hasPmd,
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
        boolean hasPmd,
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
                candidate.put("pmd_service", hasPmd);
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

    private static int candidateScore(String deviceName, boolean hasHeartRate, boolean hasPmd) {
        int score = 0;
        if (hasPmd) {
            score += 100;
        }
        if (deviceName != null && deviceName.toLowerCase(java.util.Locale.ROOT).contains("polar")) {
            score += 80;
        }
        if (hasHeartRate) {
            score += 20;
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
    private static boolean writeCharacteristicCompat(
        BluetoothGatt localGatt,
        BluetoothGattCharacteristic characteristic,
        byte[] value) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            return localGatt.writeCharacteristic(
                characteristic,
                value,
                BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT) == BluetoothStatusCodes.SUCCESS;
        }

        characteristic.setValue(value);
        characteristic.setWriteType(BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT);
        return localGatt.writeCharacteristic(characteristic);
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

    private static String normalizePmdStream(String value) {
        String normalized = value != null ? value.trim().toLowerCase(java.util.Locale.ROOT) : "";
        if ("ecg".equals(normalized) || "bio:polar_ecg".equals(normalized) || "polar_ecg".equals(normalized)) {
            return PMD_STREAM_ECG;
        }
        return PMD_STREAM_ACC;
    }

    private static int normalizeAccSampleRate(int value) {
        if (value == 25 || value == 50 || value == 100 || value == 200) {
            return value;
        }
        return 200;
    }

    private static byte measurementTypeForStream(String pmdStream) {
        return PMD_STREAM_ECG.equals(normalizePmdStream(pmdStream))
            ? PolarPmdProtocol.MEASUREMENT_TYPE_ECG
            : PolarPmdProtocol.MEASUREMENT_TYPE_ACC;
    }

    private static String streamForMeasurementType(byte measurementType) {
        return measurementType == PolarPmdProtocol.MEASUREMENT_TYPE_ECG ? PMD_STREAM_ECG : PMD_STREAM_ACC;
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

    private final class PmdGattCallback extends BluetoothGattCallback {
        final LinkedBlockingQueue<Integer> connectStatuses = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<Integer> disconnectStatuses = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<Integer> serviceStatuses = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<Integer> mtuValues = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<Integer> descriptorWriteStatuses = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<Integer> characteristicWriteStatuses = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<CharacteristicReadResult> readResults = new LinkedBlockingQueue<>();
        final LinkedBlockingQueue<byte[]> controlNotifications = new LinkedBlockingQueue<>();

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
        public void onMtuChanged(BluetoothGatt gatt, int mtu, int status) {
            mtuValues.offer(Integer.valueOf(status == BluetoothGatt.GATT_SUCCESS ? mtu : 0));
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
        public void onCharacteristicWrite(
            BluetoothGatt gatt,
            BluetoothGattCharacteristic characteristic,
            int status) {
            characteristicWriteStatuses.offer(Integer.valueOf(status));
        }

        @Override
        public void onCharacteristicRead(
            BluetoothGatt gatt,
            BluetoothGattCharacteristic characteristic,
            int status) {
            readResults.offer(new CharacteristicReadResult(
                characteristic.getUuid(),
                characteristic.getValue() != null ? characteristic.getValue().clone() : new byte[0],
                status));
        }

        @Override
        public void onCharacteristicRead(
            BluetoothGatt gatt,
            BluetoothGattCharacteristic characteristic,
            byte[] value,
            int status) {
            readResults.offer(new CharacteristicReadResult(
                characteristic.getUuid(),
                value != null ? value.clone() : new byte[0],
                status));
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
            controlNotifications.clear();
        }

        private void handleNotification(BluetoothGattCharacteristic characteristic, byte[] value) {
            if (characteristic == null || value == null) {
                return;
            }
            UUID uuid = characteristic.getUuid();
            if (PolarPmdProtocol.PMD_CONTROL_POINT.equals(uuid)) {
                controlNotifications.offer(value.clone());
            } else if (PolarPmdProtocol.PMD_DATA.equals(uuid)) {
                handlePmdData(value.clone());
            }
        }
    }

    private static final class PolarDeviceCandidate {
        final BluetoothDevice device;
        final String deviceName;
        final String deviceAddress;
        final int rssi;
        final boolean heartRateServiceVisible;
        final boolean pmdServiceVisible;
        final int matchScore;

        PolarDeviceCandidate(
            BluetoothDevice device,
            String deviceName,
            String deviceAddress,
            int rssi,
            boolean heartRateServiceVisible,
            boolean pmdServiceVisible,
            int matchScore) {
            this.device = device;
            this.deviceName = deviceName;
            this.deviceAddress = deviceAddress;
            this.rssi = rssi;
            this.heartRateServiceVisible = heartRateServiceVisible;
            this.pmdServiceVisible = pmdServiceVisible;
            this.matchScore = matchScore;
        }
    }

    private static final class CharacteristicReadResult {
        final UUID uuid;
        final byte[] value;
        final int status;

        CharacteristicReadResult(UUID uuid, byte[] value, int status) {
            this.uuid = uuid;
            this.value = value;
            this.status = status;
        }
    }
}
