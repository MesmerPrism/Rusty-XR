package com.example.rustyxr.opengles;

import android.content.Intent;
import android.media.projection.MediaProjectionManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

public final class GlesOpenXrActivity extends android.app.NativeActivity {
    private static final String TAG = "RustyXrGlesActivity";
    private static final int MEDIA_PROJECTION_REQUEST = 8711;
    private static final long DEFAULT_MEDIA_PROJECTION_DELAY_MS = 1600L;

    private MediaProjectionManager mediaProjectionManager;
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        mediaProjectionManager = (MediaProjectionManager) getSystemService(MEDIA_PROJECTION_SERVICE);
        requestMediaProjectionIfEnabled();
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        requestMediaProjectionIfEnabled();
    }

    private void requestMediaProjectionIfEnabled() {
        if (!booleanExtra("rustyxr.mediaProjection", false)) {
            return;
        }
        long delayMs = longExtra("rustyxr.mediaProjectionDelayMs", DEFAULT_MEDIA_PROJECTION_DELAY_MS);
        mainHandler.postDelayed(new Runnable() {
            @Override
            public void run() {
                requestMediaProjection();
            }
        }, Math.max(0L, delayMs));
    }

    private void requestMediaProjection() {
        if (mediaProjectionManager == null) {
            Log.w(TAG, "MediaProjectionManager is unavailable");
            return;
        }
        Log.i(TAG, "Requesting MediaProjection consent");
        startActivityForResult(
            mediaProjectionManager.createScreenCaptureIntent(),
            MEDIA_PROJECTION_REQUEST);
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != MEDIA_PROJECTION_REQUEST) {
            return;
        }
        if (resultCode != RESULT_OK || data == null) {
            Log.w(TAG, "MediaProjection consent denied or cancelled");
            return;
        }
        Log.i(TAG, "MediaProjection consent granted; starting stream service");
        Intent serviceIntent = new Intent(this, MediaProjectionStreamService.class);
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_RESULT_CODE, resultCode);
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_RESULT_DATA, data);
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_HOST, "127.0.0.1");
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_PORT, intExtra("rustyxr.mediaProjectionPort", 8787));
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_WIDTH, intExtra("rustyxr.mediaProjectionWidth", 512));
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_HEIGHT, intExtra("rustyxr.mediaProjectionHeight", 288));
        startForegroundService(serviceIntent);
    }

    @Override
    protected void onDestroy() {
        stopService(new Intent(this, MediaProjectionStreamService.class));
        super.onDestroy();
    }

    private boolean booleanExtra(String key, boolean fallback) {
        Intent intent = getIntent();
        if (intent == null || !intent.hasExtra(key)) {
            return fallback;
        }
        Object value = intent.getExtras().get(key);
        if (value instanceof Boolean) {
            return ((Boolean) value).booleanValue();
        }
        if (value instanceof String) {
            String text = ((String) value).trim().toLowerCase();
            return "true".equals(text) || "1".equals(text) || "yes".equals(text) || "on".equals(text);
        }
        return fallback;
    }

    private int intExtra(String key, int fallback) {
        Intent intent = getIntent();
        if (intent == null || !intent.hasExtra(key)) {
            return fallback;
        }
        try {
            Object value = intent.getExtras().get(key);
            if (value instanceof Number) {
                return ((Number) value).intValue();
            }
            if (value instanceof String) {
                return Integer.parseInt(((String) value).trim());
            }
        } catch (RuntimeException ignored) {
        }
        return fallback;
    }

    private long longExtra(String key, long fallback) {
        Intent intent = getIntent();
        if (intent == null || !intent.hasExtra(key)) {
            return fallback;
        }
        try {
            Object value = intent.getExtras().get(key);
            if (value instanceof Number) {
                return ((Number) value).longValue();
            }
            if (value instanceof String) {
                return Long.parseLong(((String) value).trim());
            }
        } catch (RuntimeException ignored) {
        }
        return fallback;
    }
}
