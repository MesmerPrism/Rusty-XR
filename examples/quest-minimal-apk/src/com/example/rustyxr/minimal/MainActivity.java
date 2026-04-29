package com.example.rustyxr.minimal;

import android.app.Activity;
import android.graphics.Color;
import android.graphics.Typeface;
import android.os.Bundle;
import android.util.Log;
import android.view.Choreographer;
import android.view.Gravity;
import android.view.Window;
import android.view.WindowManager;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

public final class MainActivity extends Activity {
    private static final String TAG = "RustyXrMinimal";
    private static final String NATIVE_STATUS = loadNativeLibrary();

    private TextView statusView;
    private TextView jsonView;
    private long firstFrameNanos;
    private long lastFrameNanos;
    private int frameCount;

    private static native String sessionJson();

    @Override
    protected void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        statusView = textView(18, true);
        jsonView = textView(12, false);
        jsonView.setTypeface(Typeface.MONOSPACE);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setGravity(Gravity.START);
        root.setPadding(32, 28, 32, 28);
        root.setBackgroundColor(Color.rgb(15, 18, 21));

        TextView title = textView(24, true);
        title.setText("Rusty XR Minimal Quest APK");
        root.addView(title);

        statusView.setText("Native library: " + NATIVE_STATUS);
        root.addView(statusView);

        ScrollView scrollView = new ScrollView(this);
        scrollView.addView(jsonView);
        root.addView(scrollView, new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            0,
            1.0f));

        setContentView(root);
        updateContractText();
        Choreographer.getInstance().postFrameCallback(frameCallback);
        Log.i(TAG, "Rusty XR minimal APK launched");
    }

    @Override
    protected void onDestroy() {
        Choreographer.getInstance().removeFrameCallback(frameCallback);
        Log.i(TAG, "Rusty XR minimal APK stopped after " + frameCount + " frame callback(s)");
        super.onDestroy();
    }

    private final Choreographer.FrameCallback frameCallback = new Choreographer.FrameCallback() {
        @Override
        public void doFrame(long frameTimeNanos) {
            if (firstFrameNanos == 0L) {
                firstFrameNanos = frameTimeNanos;
            }

            lastFrameNanos = frameTimeNanos;
            frameCount += 1;

            if (frameCount == 1 || frameCount % 60 == 0) {
                updateStatusText();
                Log.i(TAG, "frames=" + frameCount + " approxFps=" + approximateFps());
            }

            Choreographer.getInstance().postFrameCallback(this);
        }
    };

    private static String loadNativeLibrary() {
        try {
            System.loadLibrary("rusty_xr_quest_minimal_native");
            return "loaded";
        } catch (UnsatisfiedLinkError error) {
            Log.e(TAG, "Could not load Rust native library", error);
            return "failed: " + error.getMessage();
        }
    }

    private void updateContractText() {
        if (!"loaded".equals(NATIVE_STATUS)) {
            jsonView.setText("Rust native library did not load.\n" + NATIVE_STATUS);
            return;
        }

        try {
            jsonView.setText(sessionJson());
        } catch (RuntimeException error) {
            jsonView.setText("Rust session JSON failed:\n" + error);
        }
    }

    private void updateStatusText() {
        statusView.setText(
            "Native library: " + NATIVE_STATUS +
            "\nPackage: com.example.rustyxr.minimal" +
            "\nActivity: .MainActivity" +
            "\nFrame callbacks: " + frameCount +
            "\nApprox FPS: " + approximateFps());
    }

    private String approximateFps() {
        if (firstFrameNanos == 0L || lastFrameNanos <= firstFrameNanos) {
            return "starting";
        }

        double seconds = (lastFrameNanos - firstFrameNanos) / 1_000_000_000.0;
        return String.format("%.1f", frameCount / seconds);
    }

    private TextView textView(int sizeSp, boolean header) {
        TextView view = new TextView(this);
        view.setTextColor(header ? Color.rgb(245, 246, 238) : Color.rgb(205, 214, 210));
        view.setTextSize(sizeSp);
        view.setPadding(0, 0, 0, 14);
        return view;
    }
}
