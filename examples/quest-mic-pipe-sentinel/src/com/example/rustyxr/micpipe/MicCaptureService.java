package com.example.rustyxr.micpipe;

import android.Manifest;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ServiceInfo;
import android.media.AudioFormat;
import android.media.AudioManager;
import android.media.AudioRecord;
import android.media.AudioRecordingConfiguration;
import android.media.MediaRecorder;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.IBinder;
import android.util.Log;

import java.io.OutputStream;
import java.net.Socket;
import java.util.List;
import java.util.Locale;
import java.util.concurrent.atomic.AtomicBoolean;

public final class MicCaptureService extends Service {
    static final String ACTION_START = "com.example.rustyxr.micpipe.START_MIC_PIPE";
    static final String ACTION_STOP = "com.example.rustyxr.micpipe.STOP_MIC_PIPE";
    static final String EXTRA_RUN_ID = "rustyxr.micPipe.runId";
    static final String EXTRA_HOST = "rustyxr.micPipe.host";
    static final String EXTRA_PORT = "rustyxr.micPipe.port";
    static final String EXTRA_CHUNK_MS = "rustyxr.micPipe.chunkMs";

    private static final String TAG = "MicPipeSentinel";
    private static final String CHANNEL_ID = "rusty_xr_mic_pipe";
    private static final int NOTIFICATION_ID = 1201;

    private final AtomicBoolean stopRequested = new AtomicBoolean(false);
    private Thread captureThread;
    private AudioRecord activeRecorder;
    private AudioManager.AudioRecordingCallback recordingCallback;
    private HandlerThread callbackThread;
    private int activeAudioSessionId = -1;

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        String action = intent == null ? ACTION_START : intent.getAction();
        if (ACTION_STOP.equals(action)) {
            stopCapture("stop_action");
            return START_NOT_STICKY;
        }

        if (!hasRecordAudioPermission()) {
            MicPipeStatus.setRecordAudioGranted(false);
            MicPipeStatus.setError("RECORD_AUDIO permission is not granted");
            MicPipeEventLog.write(this, "start_failed_missing_record_audio", "{}");
            stopSelf();
            return START_NOT_STICKY;
        }

        startForegroundNotification();
        startCapture(intent == null ? new Intent() : intent);
        return START_NOT_STICKY;
    }

    @Override
    public void onDestroy() {
        stopCapture("service_destroy");
        super.onDestroy();
    }

    private void startCapture(Intent intent) {
        if (captureThread != null && captureThread.isAlive()) {
            MicPipeEventLog.write(this, "start_ignored_already_running", "{}");
            return;
        }

        String runId = valueOrDefault(intent.getStringExtra(EXTRA_RUN_ID), defaultRunId());
        String host = valueOrDefault(intent.getStringExtra(EXTRA_HOST), "127.0.0.1");
        int port = boundedInt(intent.getIntExtra(EXTRA_PORT, 34567), 1, 65535, 34567);
        int chunkMs = boundedInt(intent.getIntExtra(EXTRA_CHUNK_MS, 20), 10, 100, 20);
        int chunkBytes = MicPipeStatus.SAMPLE_RATE_HZ * 2 * chunkMs / 1000;
        int minBuffer = AudioRecord.getMinBufferSize(
            MicPipeStatus.SAMPLE_RATE_HZ,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT);
        int bufferBytes = Math.max(minBuffer, chunkBytes * 8);

        MicPipeStatus.configure(runId, host, port, chunkBytes, bufferBytes);
        MicPipeStatus.setServiceForeground(true);
        MicPipeStatus.setRecordAudioGranted(hasRecordAudioPermission());

        MicPipeEventLog.write(this, "start_requested", MicPipeEventLog.object(
            MicPipeEventLog.field("run_id", runId),
            MicPipeEventLog.field("host", host),
            MicPipeEventLog.field("port", port),
            MicPipeEventLog.field("chunk_ms", chunkMs),
            MicPipeEventLog.field("chunk_bytes", chunkBytes),
            MicPipeEventLog.field("buffer_bytes", bufferBytes)));

        stopRequested.set(false);
        captureThread = new Thread(new Runnable() {
            @Override
            public void run() {
                captureLoop(host, port, chunkBytes, bufferBytes);
            }
        }, "RustyXrMicPipeCapture");
        captureThread.start();
    }

    private void captureLoop(String host, int port, int chunkBytes, int bufferBytes) {
        AudioRecord recorder = null;
        Socket socket = null;
        try {
            AudioFormat format = new AudioFormat.Builder()
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .setSampleRate(MicPipeStatus.SAMPLE_RATE_HZ)
                .setChannelMask(AudioFormat.CHANNEL_IN_MONO)
                .build();

            recorder = new AudioRecord.Builder()
                .setAudioSource(MediaRecorder.AudioSource.VOICE_RECOGNITION)
                .setAudioFormat(format)
                .setBufferSizeInBytes(bufferBytes)
                .build();
            activeRecorder = recorder;
            activeAudioSessionId = recorder.getAudioSessionId();
            registerRecordingCallback();

            socket = new Socket(host, port);
            socket.setTcpNoDelay(true);
            MicPipeStatus.setTermuxConnected(true);
            MicPipeEventLog.write(this, "termux_connected", MicPipeEventLog.object(
                MicPipeEventLog.field("host", host),
                MicPipeEventLog.field("port", port)));

            OutputStream output = socket.getOutputStream();
            byte[] buffer = new byte[chunkBytes];
            long lastEventMs = 0L;

            recorder.startRecording();
            MicPipeStatus.setRecording(true, recorder.getRecordingState());
            MicPipeEventLog.write(this, "recording_started", MicPipeEventLog.object(
                MicPipeEventLog.field("audio_session_id", activeAudioSessionId),
                MicPipeEventLog.field("recording_state", recorder.getRecordingState())));

            while (!stopRequested.get()) {
                int read = recorder.read(buffer, 0, buffer.length, AudioRecord.READ_BLOCKING);
                if (read > 0) {
                    output.write(buffer, 0, read);
                    output.flush();
                    int rms = rmsPcm16(buffer, read);
                    MicPipeStatus.addPcm(read, read, rms);

                    long now = android.os.SystemClock.elapsedRealtime();
                    if (now - lastEventMs >= 1000L) {
                        MicPipeEventLog.write(this, "pcm_progress", MicPipeStatus.compactJsonLine());
                        lastEventMs = now;
                    }
                } else {
                    MicPipeStatus.setError("AudioRecord.read returned " + read);
                    MicPipeEventLog.write(this, "read_result", MicPipeEventLog.object(
                        MicPipeEventLog.field("value", read)));
                }
            }
        } catch (Exception error) {
            String message = error.getClass().getSimpleName() + ": " + error.getMessage();
            Log.w(TAG, "capture failed", error);
            MicPipeStatus.setError(message);
            MicPipeEventLog.write(this, "capture_failed", MicPipeEventLog.object(
                MicPipeEventLog.field("error", message)));
        } finally {
            if (recorder != null) {
                try {
                    if (recorder.getRecordingState() == AudioRecord.RECORDSTATE_RECORDING) {
                        recorder.stop();
                    }
                } catch (IllegalStateException error) {
                    Log.w(TAG, "recorder stop failed", error);
                }
                recorder.release();
            }
            if (socket != null) {
                try {
                    socket.close();
                } catch (Exception error) {
                    Log.w(TAG, "socket close failed", error);
                }
            }
            unregisterRecordingCallback();
            activeRecorder = null;
            activeAudioSessionId = -1;
            MicPipeStatus.setRecording(false, AudioRecord.RECORDSTATE_STOPPED);
            MicPipeStatus.setTermuxConnected(false);
            MicPipeStatus.setServiceForeground(false);
            MicPipeEventLog.write(this, "recording_stopped", MicPipeStatus.compactJsonLine());
            stopForeground(true);
            stopSelf();
        }
    }

    private void stopCapture(String reason) {
        stopRequested.set(true);
        MicPipeEventLog.write(this, "stop_requested", MicPipeEventLog.object(
            MicPipeEventLog.field("reason", reason)));
    }

    private void registerRecordingCallback() {
        if (Build.VERSION.SDK_INT < 29) {
            return;
        }

        AudioManager audioManager = (AudioManager) getSystemService(AUDIO_SERVICE);
        if (audioManager == null) {
            return;
        }

        callbackThread = new HandlerThread("RustyXrMicPipeAudioCallback");
        callbackThread.start();
        recordingCallback = new AudioManager.AudioRecordingCallback() {
            @Override
            public void onRecordingConfigChanged(List<AudioRecordingConfiguration> configs) {
                boolean matched = false;
                boolean silenced = false;
                int sampleRate = 0;
                int channelCount = 0;
                int encoding = 0;
                for (AudioRecordingConfiguration config : configs) {
                    if (config.getClientAudioSessionId() == activeAudioSessionId) {
                        matched = true;
                        silenced = config.isClientSilenced();
                        AudioFormat format = config.getFormat();
                        if (format != null) {
                            sampleRate = format.getSampleRate();
                            channelCount = format.getChannelCount();
                            encoding = format.getEncoding();
                        }
                    }
                }
                MicPipeStatus.setClientSilenced(silenced);
                if (matched) {
                    MicPipeEventLog.write(MicCaptureService.this, "recording_config", MicPipeEventLog.object(
                        MicPipeEventLog.field("audio_session_id", activeAudioSessionId),
                        MicPipeEventLog.field("client_silenced", silenced),
                        MicPipeEventLog.field("sample_rate_hz", sampleRate),
                        MicPipeEventLog.field("channel_count", channelCount),
                        MicPipeEventLog.field("encoding", encoding)));
                }
            }
        };
        audioManager.registerAudioRecordingCallback(
            recordingCallback,
            new Handler(callbackThread.getLooper()));
    }

    private void unregisterRecordingCallback() {
        if (recordingCallback == null) {
            return;
        }
        AudioManager audioManager = (AudioManager) getSystemService(AUDIO_SERVICE);
        if (audioManager != null) {
            audioManager.unregisterAudioRecordingCallback(recordingCallback);
        }
        recordingCallback = null;
        if (callbackThread != null) {
            callbackThread.quitSafely();
            callbackThread = null;
        }
    }

    private void startForegroundNotification() {
        NotificationManager manager = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        if (Build.VERSION.SDK_INT >= 26 && manager != null) {
            NotificationChannel channel = new NotificationChannel(
                CHANNEL_ID,
                "Rusty XR Mic Pipe",
                NotificationManager.IMPORTANCE_LOW);
            channel.setDescription("Visible microphone foreground service for the mic-pipe sentinel.");
            manager.createNotificationChannel(channel);
        }

        Intent openIntent = new Intent(this, MicPanelActivity.class);
        PendingIntent pendingIntent = PendingIntent.getActivity(
            this,
            0,
            openIntent,
            Build.VERSION.SDK_INT >= 23 ? PendingIntent.FLAG_IMMUTABLE : 0);

        Notification notification = new Notification.Builder(this, CHANNEL_ID)
            .setContentTitle("Mic Pipe Sentinel")
            .setContentText("Streaming user-started microphone PCM to localhost.")
            .setSmallIcon(android.R.drawable.ic_btn_speak_now)
            .setOngoing(true)
            .setContentIntent(pendingIntent)
            .build();

        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE);
        } else {
            startForeground(NOTIFICATION_ID, notification);
        }
    }

    private boolean hasRecordAudioPermission() {
        return checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED;
    }

    private static int rmsPcm16(byte[] buffer, int length) {
        int samples = length / 2;
        if (samples <= 0) {
            return 0;
        }
        double sumSquares = 0.0;
        for (int i = 0; i + 1 < length; i += 2) {
            int low = buffer[i] & 0xff;
            int high = buffer[i + 1];
            int sample = (short) ((high << 8) | low);
            sumSquares += sample * (double) sample;
        }
        return (int) Math.sqrt(sumSquares / samples);
    }

    private static int boundedInt(int value, int min, int max, int fallback) {
        if (value < min || value > max) {
            return fallback;
        }
        return value;
    }

    private static String valueOrDefault(String value, String fallback) {
        if (value == null || value.trim().isEmpty()) {
            return fallback;
        }
        return value.trim();
    }

    private static String defaultRunId() {
        return String.format(Locale.US, "micpipe-%d", System.currentTimeMillis());
    }
}
