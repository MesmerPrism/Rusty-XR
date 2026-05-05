package com.example.rustyxr.broker;

import android.app.Service;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;
import android.util.Log;

public final class BrokerService extends Service {
    public static final String TAG = "RustyXrBroker";
    public static final int DEFAULT_PORT = 8765;
    private static final String CHANNEL_ID = "rusty_xr_broker";
    private static final int NOTIFICATION_ID = 8765;

    private BrokerState state;
    private LocalBrokerServer server;
    private LatencyPublisher publisher;
    private OscIngressServer oscIngressServer;
    private PolarPmdBrokerSource polarPmdSource;

    @Override
    public void onCreate() {
        super.onCreate();
        state = new BrokerState();
        startForegroundServiceNotification();
        Log.i(TAG, "Broker service created");
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (publisher == null) {
            BrokerRuntimeConfig config = BrokerRuntimeConfig.fromIntent(intent);
            publisher = CompositeLatencyPublisher.create(config);
            server = new LocalBrokerServer(DEFAULT_PORT, state, publisher, getApplicationContext(), config.brokerBindHost);
            oscIngressServer = OscIngressServer.createOrNull(config, state, server);
            server.setOscIngressServer(oscIngressServer);
            polarPmdSource = new PolarPmdBrokerSource(getApplicationContext(), state, server);
            server.setPolarPmdSource(polarPmdSource);
            Log.i(TAG, "Broker publisher mode: " + publisher.mode());
            if (config.polarPmdEnabled) {
                try {
                    polarPmdSource.start(config.polarDeviceAddress, config.polarScanTimeoutMs);
                    Log.i(TAG, "Polar PMD direct BLE source requested");
                } catch (Exception ex) {
                    Log.e(TAG, "Polar PMD source failed to start: " + ex.getMessage(), ex);
                }
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

        Log.i(TAG, "Broker service destroyed");
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
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
