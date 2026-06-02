package com.example.rustyxr.micpipe;

import android.media.AudioRecord;
import android.os.SystemClock;

final class MicPipeStatus {
    static final int SAMPLE_RATE_HZ = 16000;
    static final int CHANNELS = 1;
    static final String ENCODING = "pcm_s16le";

    private static final Object LOCK = new Object();

    private static String runId = "micpipe-unstarted";
    private static String host = "127.0.0.1";
    private static int port = 34567;
    private static boolean activityVisible;
    private static boolean serviceForeground;
    private static boolean recording;
    private static boolean termuxConnected;
    private static boolean clientSilenced;
    private static boolean recordAudioGranted;
    private static int audioRecordState = AudioRecord.RECORDSTATE_STOPPED;
    private static int chunkBytes;
    private static int bufferBytes;
    private static long bytesReadTotal;
    private static long bytesSentTotal;
    private static int recentRms;
    private static String error = "";
    private static long updatedElapsedMs = SystemClock.elapsedRealtime();

    private MicPipeStatus() {
    }

    static void configure(String nextRunId, String nextHost, int nextPort, int nextChunkBytes, int nextBufferBytes) {
        synchronized (LOCK) {
            runId = nextRunId;
            host = nextHost;
            port = nextPort;
            chunkBytes = nextChunkBytes;
            bufferBytes = nextBufferBytes;
            bytesReadTotal = 0L;
            bytesSentTotal = 0L;
            recentRms = 0;
            error = "";
            termuxConnected = false;
            recording = false;
            clientSilenced = false;
            audioRecordState = AudioRecord.RECORDSTATE_STOPPED;
            touchLocked();
        }
    }

    static void setActivityVisible(boolean visible) {
        synchronized (LOCK) {
            activityVisible = visible;
            touchLocked();
        }
    }

    static void setRecordAudioGranted(boolean granted) {
        synchronized (LOCK) {
            recordAudioGranted = granted;
            touchLocked();
        }
    }

    static void setServiceForeground(boolean foreground) {
        synchronized (LOCK) {
            serviceForeground = foreground;
            touchLocked();
        }
    }

    static void setRecording(boolean nextRecording, int state) {
        synchronized (LOCK) {
            recording = nextRecording;
            audioRecordState = state;
            touchLocked();
        }
    }

    static void setTermuxConnected(boolean connected) {
        synchronized (LOCK) {
            termuxConnected = connected;
            touchLocked();
        }
    }

    static void setClientSilenced(boolean silenced) {
        synchronized (LOCK) {
            clientSilenced = silenced;
            touchLocked();
        }
    }

    static void addPcm(int bytesRead, int bytesSent, int rms) {
        synchronized (LOCK) {
            bytesReadTotal += Math.max(0, bytesRead);
            bytesSentTotal += Math.max(0, bytesSent);
            recentRms = Math.max(0, rms);
            error = "";
            touchLocked();
        }
    }

    static void setError(String message) {
        synchronized (LOCK) {
            error = message == null ? "" : message;
            touchLocked();
        }
    }

    static String snapshotJson() {
        synchronized (LOCK) {
            return "{\n"
                + "  \"schema\": \"rusty.xr.mic_pipe.android.v1\",\n"
                + "  \"run_id\": \"" + escape(runId) + "\",\n"
                + "  \"activity_visible\": " + activityVisible + ",\n"
                + "  \"service_foreground\": " + serviceForeground + ",\n"
                + "  \"foreground_service_type\": \"microphone\",\n"
                + "  \"record_audio_permission\": \"" + (recordAudioGranted ? "granted" : "missing") + "\",\n"
                + "  \"audio_record_state\": \"" + recordStateLabel(audioRecordState) + "\",\n"
                + "  \"sample_rate_hz\": " + SAMPLE_RATE_HZ + ",\n"
                + "  \"channels\": " + CHANNELS + ",\n"
                + "  \"encoding\": \"" + ENCODING + "\",\n"
                + "  \"chunk_bytes\": " + chunkBytes + ",\n"
                + "  \"buffer_bytes\": " + bufferBytes + ",\n"
                + "  \"bytes_read_total\": " + bytesReadTotal + ",\n"
                + "  \"bytes_sent_total\": " + bytesSentTotal + ",\n"
                + "  \"client_silenced\": " + clientSilenced + ",\n"
                + "  \"rms\": " + recentRms + ",\n"
                + "  \"termux_connected\": " + termuxConnected + ",\n"
                + "  \"host\": \"" + escape(host) + "\",\n"
                + "  \"port\": " + port + ",\n"
                + "  \"updated_elapsed_ms\": " + updatedElapsedMs + ",\n"
                + "  \"recording\": " + recording + ",\n"
                + "  \"error\": " + nullableString(error) + "\n"
                + "}";
        }
    }

    static String compactJsonLine() {
        return snapshotJson().replace("\n", "").replace("  ", "");
    }

    private static void touchLocked() {
        updatedElapsedMs = SystemClock.elapsedRealtime();
    }

    private static String recordStateLabel(int state) {
        if (state == AudioRecord.RECORDSTATE_RECORDING) {
            return "recording";
        }
        if (state == AudioRecord.RECORDSTATE_STOPPED) {
            return "stopped";
        }
        return "unknown_" + state;
    }

    private static String nullableString(String value) {
        if (value == null || value.isEmpty()) {
            return "null";
        }
        return "\"" + escape(value) + "\"";
    }

    private static String escape(String value) {
        return value
            .replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\r", "\\r")
            .replace("\n", "\\n");
    }
}
