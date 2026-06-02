package com.example.rustyxr.micpipe;

import android.content.Context;
import android.os.SystemClock;
import android.util.Log;

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;

final class MicPipeEventLog {
    private static final String TAG = "MicPipeSentinel";
    private static final Object LOCK = new Object();

    private MicPipeEventLog() {
    }

    static File eventFile(Context context) {
        return new File(context.getFilesDir(), "micpipe-events.jsonl");
    }

    static void write(Context context, String event, String fieldsJson) {
        String line = "{"
            + "\"schema\":\"rusty.xr.mic_pipe.android_event.v1\","
            + "\"event\":\"" + escape(event) + "\","
            + "\"elapsed_realtime_ms\":" + SystemClock.elapsedRealtime() + ","
            + "\"fields\":" + (fieldsJson == null || fieldsJson.isEmpty() ? "{}" : fieldsJson)
            + "}";

        synchronized (LOCK) {
            try (FileWriter writer = new FileWriter(eventFile(context), true)) {
                writer.write(line);
                writer.write('\n');
            } catch (IOException error) {
                Log.w(TAG, "could not write event log", error);
            }
        }

        Log.i(TAG, event + " " + (fieldsJson == null ? "{}" : fieldsJson));
    }

    static String field(String key, String value) {
        return "\"" + escape(key) + "\":\"" + escape(value) + "\"";
    }

    static String field(String key, int value) {
        return "\"" + escape(key) + "\":" + value;
    }

    static String field(String key, long value) {
        return "\"" + escape(key) + "\":" + value;
    }

    static String field(String key, boolean value) {
        return "\"" + escape(key) + "\":" + value;
    }

    static String object(String... fields) {
        StringBuilder builder = new StringBuilder();
        builder.append('{');
        for (int i = 0; i < fields.length; i += 1) {
            if (i > 0) {
                builder.append(',');
            }
            builder.append(fields[i]);
        }
        builder.append('}');
        return builder.toString();
    }

    private static String escape(String value) {
        if (value == null) {
            return "";
        }
        return value
            .replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\r", "\\r")
            .replace("\n", "\\n");
    }
}
