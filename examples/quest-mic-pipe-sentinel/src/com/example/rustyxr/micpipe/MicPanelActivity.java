package com.example.rustyxr.micpipe;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.graphics.Typeface;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.Locale;

public final class MicPanelActivity extends Activity {
    private static final int PERMISSION_REQUEST = 4101;
    private static final int TERMUX_PERMISSION_REQUEST = 4102;
    private static final String EXTRA_HOST = "rustyxr.micPipe.host";
    private static final String EXTRA_PORT = "rustyxr.micPipe.port";
    private static final String EXTRA_RUN_ID = "rustyxr.micPipe.runId";
    private static final String EXTRA_COMMAND = "rustyxr.micPipe.command";
    private static final String EXTRA_TERMUX_SCRIPT = "rustyxr.micPipe.termuxScript";
    private static final String EXTRA_TERMUX_WAV = "rustyxr.micPipe.termuxWav";
    private static final String EXTRA_TERMUX_DURATION_SECONDS = "rustyxr.micPipe.termuxDurationSeconds";
    private static final String TERMUX_RUN_COMMAND_PERMISSION = "com.termux.permission.RUN_COMMAND";
    private static final String TERMUX_RUN_COMMAND_ACTION = "com.termux.RUN_COMMAND";
    private static final String TERMUX_RUN_COMMAND_PATH = "com.termux.RUN_COMMAND_PATH";
    private static final String TERMUX_RUN_COMMAND_ARGUMENTS = "com.termux.RUN_COMMAND_ARGUMENTS";
    private static final String TERMUX_RUN_COMMAND_WORKDIR = "com.termux.RUN_COMMAND_WORKDIR";
    private static final String TERMUX_RUN_COMMAND_BACKGROUND = "com.termux.RUN_COMMAND_BACKGROUND";
    private static final String TERMUX_RUN_COMMAND_SESSION_ACTION = "com.termux.RUN_COMMAND_SESSION_ACTION";
    private static final String TERMUX_PACKAGE = "com.termux";
    private static final String TERMUX_RUN_COMMAND_SERVICE = "com.termux.app.RunCommandService";

    private final Handler handler = new Handler(Looper.getMainLooper());
    private TextView statusView;
    private TextView eventPathView;
    private EditText hostField;
    private EditText portField;
    private EditText runIdField;
    private Intent pendingLaunchCommandIntent;

    private final Runnable statusTick = new Runnable() {
        @Override
        public void run() {
            refreshStatus();
            handler.postDelayed(this, 1000L);
        }
    };

    @Override
    protected void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(28, 24, 28, 24);
        root.setGravity(Gravity.START);
        root.setBackgroundColor(Color.rgb(13, 17, 22));

        TextView title = textView("Mic Pipe Sentinel", 24, true);
        root.addView(title);

        TextView subtitle = textView(
            "Visible user-started microphone session. Start the Termux receiver before pressing Start.",
            14,
            false);
        root.addView(subtitle);

        LinearLayout controls = new LinearLayout(this);
        controls.setOrientation(LinearLayout.HORIZONTAL);
        controls.setGravity(Gravity.CENTER_VERTICAL);
        controls.setPadding(0, 8, 0, 8);

        hostField = editText("127.0.0.1");
        portField = editText("34567");
        runIdField = editText(defaultRunId());
        controls.addView(labeledField("Host", hostField), new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1.0f));
        controls.addView(labeledField("Port", portField), new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 0.7f));
        controls.addView(labeledField("Run", runIdField), new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1.2f));
        root.addView(controls);

        LinearLayout buttons = new LinearLayout(this);
        buttons.setOrientation(LinearLayout.HORIZONTAL);
        buttons.setGravity(Gravity.START);
        buttons.setPadding(0, 4, 0, 12);

        Button permissionsButton = button("Request permissions");
        permissionsButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                requestRuntimePermissions();
            }
        });
        buttons.addView(permissionsButton);

        Button startButton = button("Start mic pipe");
        startButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                startMicPipe("button");
            }
        });
        buttons.addView(startButton);

        Button stopButton = button("Stop");
        stopButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                stopMicPipe("button");
            }
        });
        buttons.addView(stopButton);

        root.addView(buttons);

        eventPathView = textView("", 12, false);
        eventPathView.setTypeface(Typeface.MONOSPACE);
        root.addView(eventPathView);

        statusView = textView("", 13, false);
        statusView.setTypeface(Typeface.MONOSPACE);
        ScrollView scroll = new ScrollView(this);
        scroll.addView(statusView);
        root.addView(scroll, new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            0,
            1.0f));

        setContentView(root);
        applyLaunchExtras(getIntent());
        pendingLaunchCommandIntent = getIntent();
        MicPipeEventLog.write(this, "panel_created", "{}");
        refreshStatus();
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        applyLaunchExtras(intent);
        pendingLaunchCommandIntent = intent;
        MicPipeEventLog.write(this, "panel_new_intent", "{}");
        refreshStatus();
    }

    @Override
    protected void onResume() {
        super.onResume();
        MicPipeStatus.setActivityVisible(true);
        handler.post(statusTick);
        runPendingLaunchCommand();
        refreshStatus();
    }

    @Override
    protected void onPause() {
        MicPipeStatus.setActivityVisible(false);
        handler.removeCallbacks(statusTick);
        super.onPause();
    }

    private void requestRuntimePermissions() {
        ArrayList<String> missing = new ArrayList<>();
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            missing.add(Manifest.permission.RECORD_AUDIO);
        }
        if (Build.VERSION.SDK_INT >= 33
            && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            missing.add(Manifest.permission.POST_NOTIFICATIONS);
        }
        if (missing.isEmpty()) {
            MicPipeEventLog.write(this, "permissions_already_granted", "{}");
            refreshStatus();
            return;
        }
        requestPermissions(missing.toArray(new String[0]), PERMISSION_REQUEST);
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode != PERMISSION_REQUEST) {
            if (requestCode == TERMUX_PERMISSION_REQUEST) {
                boolean termuxGranted = checkSelfPermission(TERMUX_RUN_COMMAND_PERMISSION) == PackageManager.PERMISSION_GRANTED;
                MicPipeEventLog.write(this, "termux_permission_result", MicPipeEventLog.object(
                    MicPipeEventLog.field("termux_run_command_granted", termuxGranted)));
                refreshStatus();
            }
            return;
        }
        boolean recordAudioGranted = checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED;
        MicPipeStatus.setRecordAudioGranted(recordAudioGranted);
        MicPipeEventLog.write(this, "permission_result", MicPipeEventLog.object(
            MicPipeEventLog.field("record_audio_granted", recordAudioGranted)));
        refreshStatus();
    }

    private void startMicPipe(String source) {
        boolean recordAudioGranted = checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED;
        MicPipeStatus.setRecordAudioGranted(recordAudioGranted);
        if (!recordAudioGranted) {
            MicPipeEventLog.write(this, "start_blocked_missing_record_audio", MicPipeEventLog.object(
                MicPipeEventLog.field("source", source)));
            requestRuntimePermissions();
            return;
        }

        Intent serviceIntent = new Intent(this, MicCaptureService.class)
            .setAction(MicCaptureService.ACTION_START)
            .putExtra(MicCaptureService.EXTRA_HOST, hostField.getText().toString().trim())
            .putExtra(MicCaptureService.EXTRA_PORT, parsePort())
            .putExtra(MicCaptureService.EXTRA_RUN_ID, runIdField.getText().toString().trim())
            .putExtra(MicCaptureService.EXTRA_CHUNK_MS, 20);

        MicPipeEventLog.write(this, "start_requested_from_panel", MicPipeEventLog.object(
            MicPipeEventLog.field("source", source),
            MicPipeEventLog.field("host", hostField.getText().toString().trim()),
            MicPipeEventLog.field("port", parsePort())));
        if (Build.VERSION.SDK_INT >= 26) {
            startForegroundService(serviceIntent);
        } else {
            startService(serviceIntent);
        }
        refreshStatus();
    }

    private void applyLaunchExtras(Intent intent) {
        if (intent == null) {
            return;
        }
        String host = intent.getStringExtra(EXTRA_HOST);
        if (host != null && !host.trim().isEmpty()) {
            hostField.setText(host.trim());
        }
        String port = intent.getStringExtra(EXTRA_PORT);
        if (port != null && !port.trim().isEmpty()) {
            portField.setText(port.trim());
        }
        String runId = intent.getStringExtra(EXTRA_RUN_ID);
        if (runId != null && !runId.trim().isEmpty()) {
            runIdField.setText(runId.trim());
        }
    }

    private void handleLaunchCommand(Intent intent) {
        if (intent == null) {
            return;
        }
        String command = intent.getStringExtra(EXTRA_COMMAND);
        if (command == null || command.trim().isEmpty()) {
            return;
        }
        String normalized = command.trim().toLowerCase(Locale.US);
        MicPipeEventLog.write(this, "panel_command_received", MicPipeEventLog.object(
            MicPipeEventLog.field("command", normalized)));
        if ("show".equals(normalized)) {
            return;
        }
        if ("request-permissions".equals(normalized) || "permissions".equals(normalized)) {
            requestRuntimePermissions();
            return;
        }
        if ("request-termux-permission".equals(normalized) || "termux-permission".equals(normalized)) {
            requestTermuxRunCommandPermission();
            return;
        }
        if ("start-termux-receiver".equals(normalized)) {
            startTermuxReceiver(intent);
            return;
        }
        if ("stop-termux-receiver".equals(normalized)) {
            stopTermuxReceiver();
            return;
        }
        if ("start".equals(normalized)) {
            startMicPipe("launch_extra");
            return;
        }
        if ("stop".equals(normalized)) {
            stopMicPipe("launch_extra");
            return;
        }
        MicPipeEventLog.write(this, "panel_command_rejected", MicPipeEventLog.object(
            MicPipeEventLog.field("command", normalized),
            MicPipeEventLog.field("reason", "unsupported_command")));
    }

    private void runPendingLaunchCommand() {
        final Intent commandIntent = pendingLaunchCommandIntent;
        pendingLaunchCommandIntent = null;
        if (commandIntent == null) {
            return;
        }
        if (commandIntent.getStringExtra(EXTRA_COMMAND) == null) {
            return;
        }
        handler.postDelayed(new Runnable() {
            @Override
            public void run() {
                handleLaunchCommand(commandIntent);
                refreshStatus();
            }
        }, 250L);
    }

    private void requestTermuxRunCommandPermission() {
        if (checkSelfPermission(TERMUX_RUN_COMMAND_PERMISSION) == PackageManager.PERMISSION_GRANTED) {
            MicPipeEventLog.write(this, "termux_permission_already_granted", "{}");
            return;
        }
        requestPermissions(new String[] { TERMUX_RUN_COMMAND_PERMISSION }, TERMUX_PERMISSION_REQUEST);
    }

    private void startTermuxReceiver(Intent sourceIntent) {
        if (checkSelfPermission(TERMUX_RUN_COMMAND_PERMISSION) != PackageManager.PERMISSION_GRANTED) {
            MicPipeEventLog.write(this, "termux_receiver_blocked_missing_permission", "{}");
            requestTermuxRunCommandPermission();
            return;
        }

        String scriptPath = valueOrDefault(
            sourceIntent.getStringExtra(EXTRA_TERMUX_SCRIPT),
            "/sdcard/Download/rustyxr_mic_recv_wav.py");
        String wavPath = valueOrDefault(
            sourceIntent.getStringExtra(EXTRA_TERMUX_WAV),
            "/sdcard/Download/rustyxr_mic_capture.wav");
        int durationSeconds = boundedInt(
            sourceIntent.getIntExtra(EXTRA_TERMUX_DURATION_SECONDS, 180),
            1,
            3600,
            180);
        String runId = runIdField.getText().toString().trim();
        if (runId.isEmpty()) {
            runId = defaultRunId();
            runIdField.setText(runId);
        }

        String receiverCommand = "python3 "
            + shellQuote(scriptPath)
            + " "
            + parsePort()
            + " "
            + shellQuote(wavPath)
            + " "
            + durationSeconds
            + " --run-id "
            + shellQuote(runId);
        String stdoutPath = wavPath + ".stdout.jsonl";
        String stderrPath = wavPath + ".stderr.txt";
        receiverCommand = "rm -f "
            + shellQuote(wavPath)
            + " "
            + shellQuote(stdoutPath)
            + " "
            + shellQuote(stderrPath)
            + "; "
            + receiverCommand
            + " > "
            + shellQuote(stdoutPath)
            + " 2> "
            + shellQuote(stderrPath);
        startTermuxRunCommand(receiverCommand, true, "start_termux_receiver");
    }

    private void stopTermuxReceiver() {
        if (checkSelfPermission(TERMUX_RUN_COMMAND_PERMISSION) != PackageManager.PERMISSION_GRANTED) {
            MicPipeEventLog.write(this, "termux_receiver_stop_blocked_missing_permission", "{}");
            return;
        }
        startTermuxRunCommand("pkill -f rustyxr_mic_recv_wav.py || true", true, "stop_termux_receiver");
    }

    private void startTermuxRunCommand(String shellCommand, boolean background, String eventName) {
        Intent termuxIntent = new Intent(TERMUX_RUN_COMMAND_ACTION)
            .setClassName(TERMUX_PACKAGE, TERMUX_RUN_COMMAND_SERVICE)
            .putExtra(TERMUX_RUN_COMMAND_PATH, "/data/data/com.termux/files/usr/bin/sh")
            .putExtra(TERMUX_RUN_COMMAND_ARGUMENTS, new String[] { "-lc", shellCommand })
            .putExtra(TERMUX_RUN_COMMAND_WORKDIR, "/data/data/com.termux/files/home")
            .putExtra(TERMUX_RUN_COMMAND_BACKGROUND, background)
            .putExtra(TERMUX_RUN_COMMAND_SESSION_ACTION, "0");
        try {
            if (Build.VERSION.SDK_INT >= 26) {
                startForegroundService(termuxIntent);
            } else {
                startService(termuxIntent);
            }
            MicPipeEventLog.write(this, eventName, MicPipeEventLog.object(
                MicPipeEventLog.field("shell_command", shellCommand),
                MicPipeEventLog.field("background", background)));
        } catch (RuntimeException error) {
            MicPipeEventLog.write(this, eventName + "_failed", MicPipeEventLog.object(
                MicPipeEventLog.field("error", error.getClass().getSimpleName() + ": " + error.getMessage())));
        }
    }

    private void stopMicPipe(String source) {
        Intent serviceIntent = new Intent(this, MicCaptureService.class)
            .setAction(MicCaptureService.ACTION_STOP);
        MicPipeEventLog.write(this, "stop_requested_from_panel", MicPipeEventLog.object(
            MicPipeEventLog.field("source", source)));
        startService(serviceIntent);
        refreshStatus();
    }

    private int parsePort() {
        try {
            int port = Integer.parseInt(portField.getText().toString().trim());
            if (port > 0 && port <= 65535) {
                return port;
            }
        } catch (NumberFormatException ignored) {
        }
        portField.setText("34567");
        return 34567;
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

    private static String shellQuote(String value) {
        return "'" + valueOrDefault(value, "").replace("'", "'\\''") + "'";
    }

    private void refreshStatus() {
        boolean recordAudioGranted = checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED;
        MicPipeStatus.setRecordAudioGranted(recordAudioGranted);
        eventPathView.setText("event log: " + MicPipeEventLog.eventFile(this).getAbsolutePath());
        statusView.setText(MicPipeStatus.snapshotJson());
    }

    private LinearLayout labeledField(String label, EditText field) {
        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        TextView text = textView(label, 11, false);
        layout.addView(text);
        layout.addView(field);
        layout.setPadding(0, 0, 12, 0);
        return layout;
    }

    private EditText editText(String value) {
        EditText field = new EditText(this);
        field.setSingleLine(true);
        field.setText(value);
        field.setTextColor(Color.rgb(238, 243, 238));
        field.setTextSize(13);
        field.setTypeface(Typeface.MONOSPACE);
        field.setSelectAllOnFocus(false);
        field.setPadding(10, 4, 10, 4);
        field.setBackgroundColor(Color.rgb(33, 40, 45));
        return field;
    }

    private Button button(String label) {
        Button button = new Button(this);
        button.setText(label);
        button.setAllCaps(false);
        button.setTextSize(13);
        button.setPadding(12, 6, 12, 6);
        return button;
    }

    private TextView textView(String text, int sizeSp, boolean header) {
        TextView view = new TextView(this);
        view.setText(text);
        view.setTextSize(sizeSp);
        view.setTextColor(header ? Color.rgb(246, 249, 241) : Color.rgb(202, 211, 208));
        view.setPadding(0, 0, 0, 8);
        return view;
    }

    private static String defaultRunId() {
        return String.format(Locale.US, "micpipe-%d", System.currentTimeMillis());
    }
}
