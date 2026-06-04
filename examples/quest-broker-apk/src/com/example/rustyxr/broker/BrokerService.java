package com.example.rustyxr.broker;

import android.app.Service;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;
import android.os.SystemClock;
import android.util.Log;

import org.json.JSONObject;

public final class BrokerService extends Service {
    public static final String TAG = "RustyXrBroker";
    public static final int DEFAULT_PORT = 8765;
    private static final String CHANNEL_ID = "rusty_xr_broker";
    private static final int NOTIFICATION_ID = 8765;
    private static final long CONSOLE_SERVICE_READY_TIMEOUT_MS = 2_500L;
    private static final Object ACTIVE_SERVICE_LOCK = new Object();
    private static volatile BrokerService activeService;

    private BrokerState state;
    private LocalBrokerServer server;
    private LatencyPublisher publisher;
    private OscIngressServer oscIngressServer;
    private PolarPmdBrokerSource polarPmdSource;
    private PolarHeartRateBrokerSource polarHeartRateSource;
    private DeviceWatchdog deviceWatchdog;

    @Override
    public void onCreate() {
        super.onCreate();
        state = new BrokerState();
        deviceWatchdog = new DeviceWatchdog(getApplicationContext(), state);
        startForegroundServiceNotification();
        Log.i(TAG, "Broker service created");
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (publisher == null) {
            BrokerRuntimeConfig config = BrokerRuntimeConfig.fromIntent(intent);
            publisher = CompositeLatencyPublisher.create(config);
            server = new LocalBrokerServer(DEFAULT_PORT, state, publisher, getApplicationContext(), config.brokerBindHost);
            server.setDeviceWatchdog(deviceWatchdog);
            oscIngressServer = OscIngressServer.createOrNull(config, state, server);
            server.setOscIngressServer(oscIngressServer);
            polarPmdSource = new PolarPmdBrokerSource(getApplicationContext(), state, server);
            server.setPolarPmdSource(polarPmdSource);
            polarHeartRateSource = new PolarHeartRateBrokerSource(getApplicationContext(), state, server);
            server.setPolarHeartRateSource(polarHeartRateSource);
            publishConsoleReadyService();
            Log.i(TAG, "Broker publisher mode: " + publisher.mode());
            if (config.polarPmdEnabled) {
                try {
                    polarPmdSource.start(config.polarDeviceAddress, config.polarScanTimeoutMs);
                    Log.i(TAG, "Polar PMD direct BLE source requested");
                } catch (Exception ex) {
                    Log.e(TAG, "Polar PMD source failed to start: " + ex.getMessage(), ex);
                }
            }
            if (deviceWatchdog != null) {
                deviceWatchdog.restoreIfDesired();
            }
        } else if (intent != null && intent.getExtras() != null && intent.getExtras().size() > 0) {
            BrokerRuntimeConfig config = BrokerRuntimeConfig.fromIntent(intent);
            if (config.polarPmdEnabled && polarPmdSource != null) {
                try {
                    polarPmdSource.start(config.polarDeviceAddress, config.polarScanTimeoutMs);
                    Log.i(TAG, "Polar PMD direct BLE source requested on running broker");
                } catch (Exception ex) {
                    Log.e(TAG, "Polar PMD source failed to start: " + ex.getMessage(), ex);
                }
            } else {
                Log.i(TAG, "Broker already running; launch extras were ignored. Force-stop broker to reconfigure transports.");
            }
        }

        if (server != null && !server.isRunning()) {
            try {
                server.start();
                Log.i(TAG, "Broker listening on " + server.bindHost() + ":" + DEFAULT_PORT);
                if (oscIngressServer != null && !oscIngressServer.isRunning()) {
                    oscIngressServer.start();
                }
            } catch (Exception ex) {
                Log.e(TAG, "Broker failed to start: " + ex.getClass().getSimpleName() + ": " + ex.getMessage(), ex);
            }
        }

        return START_STICKY;
    }

    @Override
    public void onDestroy() {
        if (polarHeartRateSource != null) {
            polarHeartRateSource.close();
            polarHeartRateSource = null;
        }

        if (polarPmdSource != null) {
            polarPmdSource.close();
            polarPmdSource = null;
        }

        if (server != null) {
            server.close();
            server = null;
        }

        if (oscIngressServer != null) {
            oscIngressServer.close();
            oscIngressServer = null;
        }

        if (publisher != null) {
            publisher.close();
        }
        publisher = null;

        if (deviceWatchdog != null) {
            deviceWatchdog.shutdownForServiceDestroy();
        }

        clearConsoleReadyService();

        Log.i(TAG, "Broker service destroyed");
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    static JSONObject getPolarPmdStatusFromConsole() throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("polar_pmd.get_status", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.getPolarPmdStatusFromConsoleInternal();
    }

    static JSONObject startPolarPmdFromConsole(String deviceAddress, long scanTimeoutMs) throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("polar_pmd.start", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.startPolarPmdFromConsoleInternal(deviceAddress, scanTimeoutMs);
    }

    static JSONObject stopPolarPmdFromConsole() throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("polar_pmd.stop", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.stopPolarPmdFromConsoleInternal();
    }

    static JSONObject getBreathAssessmentStatusFromConsole() throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("breath_assessment.get_status", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.getBreathAssessmentStatusFromConsoleInternal();
    }

    static JSONObject setPolarBreathParamsFromConsole(JSONObject params) throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("set_polar_breath_params", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.setPolarBreathParamsFromConsoleInternal(params);
    }

    static JSONObject beginPolarBreathCalibrationFromConsole() throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("polar_breath_calibrate_begin", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.beginPolarBreathCalibrationFromConsoleInternal();
    }

    static JSONObject resetPolarBreathCalibrationFromConsole() throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("polar_breath_calibrate_reset", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.resetPolarBreathCalibrationFromConsoleInternal();
    }

    static JSONObject getExperimentControlFromConsole() throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("experiment.get_control", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.getExperimentControlFromConsoleInternal();
    }

    static JSONObject getDeviceWatchdogStatusFromConsole() throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("device_watchdog.get_status", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.getDeviceWatchdogStatusFromConsoleInternal();
    }

    static JSONObject startDeviceWatchdogFromConsole(long intervalMs, boolean wakeLock) throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("device_watchdog.start", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.startDeviceWatchdogFromConsoleInternal(intervalMs, wakeLock);
    }

    static JSONObject stopDeviceWatchdogFromConsole() throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("device_watchdog.stop", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.stopDeviceWatchdogFromConsoleInternal();
    }

    static JSONObject markDeviceWatchdogFromConsole(String label) throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("device_watchdog.mark", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.markDeviceWatchdogFromConsoleInternal(label);
    }

    static JSONObject configureExperimentControlFromConsole(JSONObject params) throws Exception {
        BrokerService service = waitForConsoleReadyService();
        if (service == null) {
            return consoleError("experiment.configure", "broker_service_unavailable", "Broker service is not active yet.");
        }

        return service.configureExperimentControlFromConsoleInternal(params);
    }

    private JSONObject getPolarPmdStatusFromConsoleInternal() throws Exception {
        JSONObject status;
        PolarPmdBrokerSource source = polarPmdSource;
        if (source != null) {
            status = source.statusJson();
        } else if (state != null) {
            status = state.polarPmdStatusJson();
        } else {
            return consoleError("polar_pmd.get_status", "broker_state_unavailable", "Broker state is not initialized yet.");
        }

        state.acceptedCommands.incrementAndGet();
        return consoleAck("polar_pmd.get_status", true, "polar_pmd_status", status);
    }

    private JSONObject getDeviceWatchdogStatusFromConsoleInternal() throws Exception {
        if (deviceWatchdog == null) {
            if (state != null) {
                state.rejectedCommands.incrementAndGet();
            }
            return consoleError("device_watchdog.get_status", "device_watchdog_unavailable", "Device watchdog is not initialized.");
        }

        JSONObject status = deviceWatchdog.statusJson();
        state.acceptedCommands.incrementAndGet();
        return consoleAck("device_watchdog.get_status", true, "device_watchdog_status", status);
    }

    private JSONObject startDeviceWatchdogFromConsoleInternal(long intervalMs, boolean wakeLock) throws Exception {
        if (deviceWatchdog == null) {
            if (state != null) {
                state.rejectedCommands.incrementAndGet();
            }
            return consoleError("device_watchdog.start", "device_watchdog_unavailable", "Device watchdog is not initialized.");
        }

        JSONObject params = new JSONObject();
        params.put("interval_ms", intervalMs > 0L ? intervalMs : 30_000L);
        params.put("wake_lock", wakeLock);
        JSONObject status = deviceWatchdog.start(params);
        state.acceptedCommands.incrementAndGet();
        return consoleAck("device_watchdog.start", true, "device_watchdog_started", status);
    }

    private JSONObject stopDeviceWatchdogFromConsoleInternal() throws Exception {
        if (deviceWatchdog == null) {
            if (state != null) {
                state.rejectedCommands.incrementAndGet();
            }
            return consoleError("device_watchdog.stop", "device_watchdog_unavailable", "Device watchdog is not initialized.");
        }

        JSONObject status = deviceWatchdog.stop("console_stop");
        state.acceptedCommands.incrementAndGet();
        return consoleAck("device_watchdog.stop", true, "device_watchdog_stopped", status);
    }

    private JSONObject markDeviceWatchdogFromConsoleInternal(String label) throws Exception {
        if (deviceWatchdog == null) {
            if (state != null) {
                state.rejectedCommands.incrementAndGet();
            }
            return consoleError("device_watchdog.mark", "device_watchdog_unavailable", "Device watchdog is not initialized.");
        }

        JSONObject params = new JSONObject();
        params.put("label", label != null && label.length() > 0 ? label : "console_marker");
        try {
            JSONObject status = deviceWatchdog.mark(params);
            state.acceptedCommands.incrementAndGet();
            return consoleAck("device_watchdog.mark", true, "device_watchdog_marker_recorded", status);
        } catch (Exception ex) {
            state.rejectedCommands.incrementAndGet();
            return consoleError("device_watchdog.mark", "device_watchdog_mark_failed", ex.getMessage());
        }
    }

    private JSONObject startPolarPmdFromConsoleInternal(String deviceAddress, long scanTimeoutMs) throws Exception {
        PolarPmdBrokerSource source = polarPmdSource;
        if (source == null) {
            if (state != null) {
                state.rejectedCommands.incrementAndGet();
            }
            return consoleError("polar_pmd.start", "polar_pmd_unavailable", "Polar PMD source is not attached to this broker.");
        }

        JSONObject status = source.start(deviceAddress, scanTimeoutMs);
        state.acceptedCommands.incrementAndGet();
        return consoleAck("polar_pmd.start", true, "polar_pmd_starting", status);
    }

    private JSONObject stopPolarPmdFromConsoleInternal() throws Exception {
        PolarPmdBrokerSource source = polarPmdSource;
        if (source == null) {
            if (state != null) {
                state.rejectedCommands.incrementAndGet();
            }
            return consoleError("polar_pmd.stop", "polar_pmd_unavailable", "Polar PMD source is not attached to this broker.");
        }

        JSONObject status = source.stop();
        state.acceptedCommands.incrementAndGet();
        return consoleAck("polar_pmd.stop", true, "polar_pmd_stopping", status);
    }

    private JSONObject getBreathAssessmentStatusFromConsoleInternal() throws Exception {
        if (state == null) {
            return consoleError("breath_assessment.get_status", "broker_state_unavailable", "Broker state is not initialized yet.");
        }

        JSONObject status = state.breathAssessmentStatusJson();
        state.acceptedCommands.incrementAndGet();
        return consoleAck("breath_assessment.get_status", true, "breath_assessment_status", status);
    }

    private JSONObject setPolarBreathParamsFromConsoleInternal(JSONObject params) throws Exception {
        if (state == null) {
            return consoleError("set_polar_breath_params", "broker_state_unavailable", "Broker state is not initialized yet.");
        }

        JSONObject status = state.setPolarBreathParams(params);
        state.acceptedCommands.incrementAndGet();
        return consoleAck("set_polar_breath_params", true, "polar_breath_params_set", status);
    }

    private JSONObject beginPolarBreathCalibrationFromConsoleInternal() throws Exception {
        if (state == null) {
            return consoleError("polar_breath_calibrate_begin", "broker_state_unavailable", "Broker state is not initialized yet.");
        }

        JSONObject status = state.beginPolarBreathCalibration(null);
        state.acceptedCommands.incrementAndGet();
        return consoleAck("polar_breath_calibrate_begin", true, "polar_breath_calibration_started", status);
    }

    private JSONObject resetPolarBreathCalibrationFromConsoleInternal() throws Exception {
        if (state == null) {
            return consoleError("polar_breath_calibrate_reset", "broker_state_unavailable", "Broker state is not initialized yet.");
        }

        JSONObject status = state.resetPolarBreathCalibration(null);
        state.acceptedCommands.incrementAndGet();
        return consoleAck("polar_breath_calibrate_reset", true, "polar_breath_calibration_reset", status);
    }

    private JSONObject getExperimentControlFromConsoleInternal() throws Exception {
        if (state == null) {
            return consoleError("experiment.get_control", "broker_state_unavailable", "Broker state is not initialized yet.");
        }

        JSONObject status = state.experimentControlJson();
        state.acceptedCommands.incrementAndGet();
        return consoleAck("experiment.get_control", true, "experiment_control_status", status);
    }

    private JSONObject configureExperimentControlFromConsoleInternal(JSONObject params) throws Exception {
        if (state == null) {
            return consoleError("experiment.configure", "broker_state_unavailable", "Broker state is not initialized yet.");
        }

        JSONObject status = state.configureExperimentControl(params);
        state.acceptedCommands.incrementAndGet();
        return consoleAck("experiment.configure", true, "experiment_control_configured", status);
    }

    private void publishConsoleReadyService() {
        synchronized (ACTIVE_SERVICE_LOCK) {
            activeService = this;
            ACTIVE_SERVICE_LOCK.notifyAll();
        }
    }

    private void clearConsoleReadyService() {
        synchronized (ACTIVE_SERVICE_LOCK) {
            if (activeService == this) {
                activeService = null;
            }
            ACTIVE_SERVICE_LOCK.notifyAll();
        }
    }

    private static BrokerService waitForConsoleReadyService() {
        long deadline = SystemClock.elapsedRealtime() + CONSOLE_SERVICE_READY_TIMEOUT_MS;
        synchronized (ACTIVE_SERVICE_LOCK) {
            while (activeService == null) {
                long remainingMs = deadline - SystemClock.elapsedRealtime();
                if (remainingMs <= 0L) {
                    return null;
                }
                try {
                    ACTIVE_SERVICE_LOCK.wait(remainingMs);
                } catch (InterruptedException ex) {
                    Thread.currentThread().interrupt();
                    return null;
                }
            }

            return activeService;
        }
    }

    private static JSONObject consoleAck(String command, boolean accepted, String message, JSONObject status) throws Exception {
        JSONObject result = new JSONObject();
        if (status != null) {
            result.put("status", status);
        }

        JSONObject ack = new JSONObject();
        ack.put("type", "command_ack");
        ack.put("schema", BrokerState.MANIFOLD_COMMAND_ACK_SCHEMA);
        ack.put("legacy_schema", BrokerState.LEGACY_RUSTY_XR_BROKER_COMMAND_ACK_SCHEMA);
        ack.put("request_id", "broker-console");
        ack.put("command", command != null ? command : "");
        ack.put("accepted", accepted);
        ack.put("message", message != null ? message : "");
        ack.put("result", result);
        return ack;
    }

    private static JSONObject consoleError(String command, String code, String message) throws Exception {
        JSONObject error = new JSONObject();
        error.put("schema", BrokerState.COMMAND_REJECTION_SCHEMA);
        error.put("code", code != null ? code : "");
        error.put("message", message != null ? message : "");
        error.put("retryable", false);

        JSONObject ack = new JSONObject();
        ack.put("type", "command_ack");
        ack.put("schema", BrokerState.MANIFOLD_COMMAND_ACK_SCHEMA);
        ack.put("legacy_schema", BrokerState.LEGACY_RUSTY_XR_BROKER_COMMAND_ACK_SCHEMA);
        ack.put("request_id", "broker-console");
        ack.put("command", command != null ? command : "");
        ack.put("accepted", false);
        ack.put("error", error);
        return ack;
    }

    private void startForegroundServiceNotification() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel channel = new NotificationChannel(
                CHANNEL_ID,
                "Rusty XR Broker",
                NotificationManager.IMPORTANCE_LOW);
            channel.setDescription("Rusty XR localhost broker diagnostics");
            NotificationManager manager = getSystemService(NotificationManager.class);
            if (manager != null) {
                manager.createNotificationChannel(channel);
            }
        }

        Notification.Builder builder = Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
            ? new Notification.Builder(this, CHANNEL_ID)
            : new Notification.Builder(this);

        Notification notification = builder
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle("Rusty XR Broker")
            .setContentText("Listening on 127.0.0.1:" + DEFAULT_PORT)
            .setOngoing(true)
            .build();

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC);
        } else {
            startForeground(NOTIFICATION_ID, notification);
        }
    }
}
