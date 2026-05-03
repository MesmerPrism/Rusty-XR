package com.example.rustyxr.broker;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.TextUtils;
import android.util.Log;
import android.view.Gravity;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedInputStream;
import java.lang.ref.WeakReference;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.Locale;

public final class MainActivity extends Activity {
    private static final int BACKGROUND = Color.rgb(9, 12, 14);
    private static final int PANEL = Color.rgb(20, 26, 30);
    private static final int PANEL_ALT = Color.rgb(27, 36, 42);
    private static final int ACCENT = Color.rgb(68, 168, 141);
    private static final int ACCENT_STRONG = Color.rgb(106, 210, 171);
    private static final int TEXT = Color.rgb(236, 242, 239);
    private static final int MUTED = Color.rgb(168, 184, 178);
    private static final int WARN = Color.rgb(246, 198, 105);
    private static final String[] PAGES = { "Dashboard", "Streams", "Commands", "Diagnostics" };
    private static volatile WeakReference<MainActivity> activeActivity = new WeakReference<>(null);

    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Runnable refreshRunnable = new Runnable() {
        @Override
        public void run() {
            refreshStatus();
            handler.postDelayed(this, 2000L);
        }
    };

    private LinearLayout navBar;
    private TextView subtitleView;
    private TextView bodyView;
    private TextView footerView;
    private JSONObject lastStatus;
    private String currentPage = PAGES[0];
    private String returnAppPackage = "";
    private String returnClientId = "";
    private boolean openedByBrokerCommand;

    @Override
    protected void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        activeActivity = new WeakReference<>(this);
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        startBrokerService(getIntent());
        setContentView(buildConsoleLayout());
        updateLaunchSubtitle(getIntent());
        renderCurrentPage();
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        activeActivity = new WeakReference<>(this);
        setIntent(intent);
        startBrokerService(intent);
        updateLaunchSubtitle(intent);
        refreshStatus();
    }

    @Override
    protected void onResume() {
        super.onResume();
        activeActivity = new WeakReference<>(this);
        handler.removeCallbacks(refreshRunnable);
        handler.post(refreshRunnable);
    }

    @Override
    protected void onPause() {
        handler.removeCallbacks(refreshRunnable);
        super.onPause();
    }

    @Override
    protected void onDestroy() {
        MainActivity activity = activeActivity.get();
        if (activity == this) {
            activeActivity = new WeakReference<>(null);
        }

        super.onDestroy();
    }

    static boolean requestCloseFromBrokerCommand(final String reason) {
        final MainActivity activity = activeActivity.get();
        if (activity == null || activity.isFinishing() || activity.isDestroyed()) {
            return false;
        }

        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                activity.closeConsole(reason);
            }
        });
        return true;
    }

    private void startBrokerService(Intent launchIntent) {
        Intent serviceIntent = new Intent(this, BrokerService.class);
        if (launchIntent != null && launchIntent.getExtras() != null) {
            serviceIntent.putExtras(launchIntent);
            openedByBrokerCommand = launchIntent.getBooleanExtra("rustyxr.openedByBrokerCommand", openedByBrokerCommand);
            rememberReturnTarget(launchIntent);
        }
        startService(serviceIntent);
        Log.i(BrokerService.TAG, "MainActivity launched broker service");
    }

    private void rememberReturnTarget(Intent launchIntent) {
        String appPackage = launchIntent.getStringExtra("rustyxr.appPackage");
        if (!TextUtils.isEmpty(appPackage) && !getPackageName().equals(appPackage)) {
            returnAppPackage = appPackage;
        }

        String clientId = launchIntent.getStringExtra("rustyxr.clientId");
        if (!TextUtils.isEmpty(clientId)) {
            returnClientId = clientId;
        }
    }

    private View buildConsoleLayout() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(28, 24, 28, 24);
        root.setBackgroundColor(BACKGROUND);

        LinearLayout top = new LinearLayout(this);
        top.setOrientation(LinearLayout.HORIZONTAL);
        top.setGravity(Gravity.CENTER_VERTICAL);
        top.setPadding(0, 0, 0, 16);
        root.addView(top, new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT));

        LinearLayout titleColumn = new LinearLayout(this);
        titleColumn.setOrientation(LinearLayout.VERTICAL);
        LinearLayout.LayoutParams titleParams = new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f);
        top.addView(titleColumn, titleParams);

        TextView title = textView(28, true, TEXT);
        title.setText("Rusty XR Broker Console");
        titleColumn.addView(title);

        subtitleView = textView(14, false, MUTED);
        titleColumn.addView(subtitleView);

        Button refreshButton = actionButton("Refresh");
        refreshButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                refreshStatus();
            }
        });
        top.addView(refreshButton);

        Button returnButton = actionButton("Return to XR App");
        returnButton.setTextColor(Color.rgb(7, 24, 18));
        returnButton.setBackground(panelBackground(ACCENT_STRONG, 12, ACCENT_STRONG));
        returnButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                returnToXrApp();
            }
        });
        LinearLayout.LayoutParams returnParams = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT);
        returnParams.setMargins(12, 0, 0, 0);
        top.addView(returnButton, returnParams);

        HorizontalScrollView navScroll = new HorizontalScrollView(this);
        navScroll.setHorizontalScrollBarEnabled(false);
        navBar = new LinearLayout(this);
        navBar.setOrientation(LinearLayout.HORIZONTAL);
        navScroll.addView(navBar);
        root.addView(navScroll, new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT));

        for (int i = 0; i < PAGES.length; i++) {
            final String page = PAGES[i];
            Button button = navButton(page);
            button.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View view) {
                    currentPage = page;
                    renderCurrentPage();
                }
            });
            navBar.addView(button);
        }

        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setPadding(22, 20, 22, 20);
        panel.setBackground(panelBackground(PANEL, 14, Color.rgb(45, 57, 63)));
        scroll.addView(panel, new ScrollView.LayoutParams(
            ScrollView.LayoutParams.MATCH_PARENT,
            ScrollView.LayoutParams.WRAP_CONTENT));

        bodyView = textView(16, false, TEXT);
        bodyView.setTypeface(Typeface.MONOSPACE);
        bodyView.setLineSpacing(0f, 1.08f);
        panel.addView(bodyView);

        LinearLayout.LayoutParams scrollParams = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            0,
            1f);
        scrollParams.setMargins(0, 18, 0, 14);
        root.addView(scroll, scrollParams);

        footerView = textView(13, false, MUTED);
        footerView.setText("Endpoint: http://127.0.0.1:8765/status    WebSocket: ws://127.0.0.1:8765/rustyxr/v1/events");
        root.addView(footerView);

        return root;
    }

    private void updateLaunchSubtitle(Intent intent) {
        String client = intent != null ? intent.getStringExtra("rustyxr.clientId") : "";
        if (TextUtils.isEmpty(client)) {
            client = returnClientId;
        }
        String source = openedByBrokerCommand ? "opened by broker command" : "opened from launcher";
        if (!TextUtils.isEmpty(client)) {
            source = source + " from " + client;
        }

        if (subtitleView != null) {
            String returnState = !TextUtils.isEmpty(resolveReturnAppPackage()) ? "Return target registered." : "No return target yet.";
            subtitleView.setText(source + "    " + returnState + " Service stays active in the background.");
        }
    }

    private void returnToXrApp() {
        closeConsole("return_button");
    }

    private void closeConsole(String reason) {
        Log.i(BrokerService.TAG, "Broker console closing; reason=" + reason + "; broker service remains active");
        finish();
        overridePendingTransition(0, 0);
    }

    private String resolveReturnAppPackage() {
        if (!TextUtils.isEmpty(returnAppPackage) && !getPackageName().equals(returnAppPackage)) {
            return returnAppPackage;
        }

        JSONObject client = lastStatus != null ? lastStatus.optJSONObject("client") : null;
        String appPackage = client != null ? client.optString("app_package", "") : "";
        if (!TextUtils.isEmpty(appPackage) && !getPackageName().equals(appPackage)) {
            returnAppPackage = appPackage;
            return appPackage;
        }

        return "";
    }

    private Button actionButton(String label) {
        Button button = new Button(this);
        button.setText(label);
        button.setTextSize(14);
        button.setTextColor(TEXT);
        button.setAllCaps(false);
        button.setMinHeight(48);
        button.setPadding(18, 8, 18, 8);
        button.setBackground(panelBackground(PANEL_ALT, 12, Color.rgb(55, 70, 76)));
        return button;
    }

    private Button navButton(String label) {
        Button button = actionButton(label);
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT);
        params.setMargins(0, 0, 10, 0);
        button.setLayoutParams(params);
        return button;
    }

    private TextView textView(int sizeSp, boolean header, int color) {
        TextView view = new TextView(this);
        view.setTextColor(color);
        view.setTextSize(sizeSp);
        view.setTypeface(header ? Typeface.DEFAULT_BOLD : Typeface.DEFAULT);
        view.setIncludeFontPadding(true);
        return view;
    }

    private GradientDrawable panelBackground(int fill, int radius, int stroke) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(fill);
        drawable.setCornerRadius(radius);
        drawable.setStroke(1, stroke);
        return drawable;
    }

    private void refreshStatus() {
        Thread thread = new Thread(new Runnable() {
            @Override
            public void run() {
                final String body = readStatusBody();
                handler.post(new Runnable() {
                    @Override
                    public void run() {
                        try {
                            lastStatus = new JSONObject(body);
                        } catch (Exception ex) {
                            lastStatus = null;
                            bodyView.setText("Broker status unavailable.\n\n" + ex.getMessage());
                        }
                        renderCurrentPage();
                    }
                });
            }
        }, "RustyXrBrokerConsoleRefresh");
        thread.start();
    }

    private String readStatusBody() {
        HttpURLConnection connection = null;
        try {
            URL url = new URL("http://127.0.0.1:" + BrokerService.DEFAULT_PORT + "/status");
            connection = (HttpURLConnection) url.openConnection();
            connection.setConnectTimeout(700);
            connection.setReadTimeout(700);
            connection.setRequestMethod("GET");
            BufferedInputStream input = new BufferedInputStream(connection.getInputStream());
            byte[] buffer = new byte[8192];
            StringBuilder builder = new StringBuilder();
            int read;
            while ((read = input.read(buffer)) >= 0) {
                builder.append(new String(buffer, 0, read, "UTF-8"));
            }
            input.close();
            return builder.toString();
        } catch (Exception ex) {
            return "{\"type\":\"status_error\",\"message\":\"" + safeJson(ex.getMessage()) + "\"}";
        } finally {
            if (connection != null) {
                connection.disconnect();
            }
        }
    }

    private void renderCurrentPage() {
        updateNavButtons();
        if (bodyView == null) {
            return;
        }

        JSONObject status = lastStatus;
        if (status == null) {
            bodyView.setText("Starting broker service...\n\nStatus refresh runs every 2 seconds.");
            return;
        }

        if ("Streams".equals(currentPage)) {
            bodyView.setText(buildStreams(status));
        } else if ("Commands".equals(currentPage)) {
            bodyView.setText(buildCommands(status));
        } else if ("Diagnostics".equals(currentPage)) {
            bodyView.setText(buildDiagnostics(status));
        } else {
            bodyView.setText(buildDashboard(status));
        }
    }

    private void updateNavButtons() {
        if (navBar == null) {
            return;
        }

        for (int i = 0; i < navBar.getChildCount(); i++) {
            View child = navBar.getChildAt(i);
            if (!(child instanceof Button)) {
                continue;
            }

            Button button = (Button) child;
            boolean selected = currentPage.contentEquals(button.getText());
            button.setTextColor(selected ? Color.rgb(5, 23, 18) : TEXT);
            button.setBackground(panelBackground(
                selected ? ACCENT_STRONG : PANEL_ALT,
                12,
                selected ? ACCENT_STRONG : Color.rgb(55, 70, 76)));
        }
    }

    private String buildDashboard(JSONObject status) {
        StringBuilder builder = new StringBuilder(800);
        builder.append("DASHBOARD\n\n");
        builder.append("Protocol      ").append(status.optString("contractVersion", "unknown")).append('\n');
        builder.append("Broker        ").append(status.optString("brokerVersion", "unknown")).append('\n');
        builder.append("Uptime        ").append(formatUptime(status.optLong("uptimeMs", 0L))).append('\n');
        builder.append("Bind          ").append(status.optString("bindAddress", "127.0.0.1"))
            .append(':').append(status.optInt("port", BrokerService.DEFAULT_PORT)).append('\n');
        builder.append('\n');

        JSONObject counters = status.optJSONObject("counters");
        if (counters != null) {
            builder.append("COUNTERS\n");
            appendCounter(builder, counters, "httpStatusRequests");
            appendCounter(builder, counters, "websocketConnections");
            appendCounter(builder, counters, "acceptedCommands");
            appendCounter(builder, counters, "rejectedCommands");
            appendCounter(builder, counters, "brokerConsoleOpenRequests");
            appendCounter(builder, counters, "brokerConsoleCloseRequests");
            appendCounter(builder, counters, "acceptedLatencySamples");
            appendCounter(builder, counters, "oscIngressPackets");
            appendCounter(builder, counters, "oscIngressBroadcasts");
            appendCounter(builder, counters, "oscIngressRejectedPackets");
            builder.append('\n');
        }

        JSONObject lsl = status.optJSONObject("lsl");
        JSONObject osc = status.optJSONObject("osc");
        builder.append("TRANSPORTS\n");
        builder.append("LSL           ").append(lsl != null && lsl.optBoolean("enabled") ? "enabled" : "fallback/logcat").append('\n');
        builder.append("OSC ingress   ").append(transportEnabled(osc != null ? osc.optJSONObject("ingress") : null)).append('\n');
        builder.append("OSC egress    ").append(transportEnabled(osc != null ? osc.optJSONObject("egress") : null)).append('\n');
        builder.append('\n');
        builder.append("Use Return to XR App to close this console while the broker service keeps running.");
        return builder.toString();
    }

    private String buildStreams(JSONObject status) {
        StringBuilder builder = new StringBuilder(800);
        builder.append("STREAMS\n\n");
        JSONArray streams = status.optJSONArray("streams");
        if (streams == null || streams.length() == 0) {
            builder.append("No streams reported by broker status.");
            return builder.toString();
        }

        for (int i = 0; i < streams.length(); i++) {
            JSONObject stream = streams.optJSONObject(i);
            if (stream == null) {
                continue;
            }

            builder.append(stream.optBoolean("active") ? "[active]   " : "[offline]  ");
            builder.append(stream.optString("id", "unknown")).append('\n');
            builder.append("kind       ").append(stream.optString("kind", "unknown")).append('\n');
            builder.append("details    ").append(stream.optString("description", "")).append("\n\n");
        }
        return builder.toString();
    }

    private String buildCommands(JSONObject status) {
        StringBuilder builder = new StringBuilder(900);
        builder.append("COMMANDS\n\n");
        JSONObject commands = status.optJSONObject("commands");
        if (commands != null) {
            builder.append("Schema        ").append(commands.optString("schema", "")).append('\n');
            builder.append("Ack schema    ").append(commands.optString("ackSchema", "")).append("\n\n");
            JSONArray supported = commands.optJSONArray("supported");
            if (supported != null) {
                builder.append("SUPPORTED\n");
                for (int i = 0; i < supported.length(); i++) {
                    builder.append("- ").append(supported.optString(i)).append('\n');
                }
                builder.append('\n');
            }
        }

        builder.append("XR apps can open this console through the broker command envelope:\n\n");
        builder.append("{\n");
        builder.append("  \"type\": \"command\",\n");
        builder.append("  \"schema\": \"rusty.xr.broker.command.v1\",\n");
        builder.append("  \"request_id\": \"ui-001\",\n");
        builder.append("  \"command\": \"open_ui\"\n");
        builder.append("}\n\n");
        builder.append("Use command \"close_ui\" to close this console from the XR app without starting any target app.\n\n");
        builder.append("The broker service remains active after the console returns to the background.");
        return builder.toString();
    }

    private String buildDiagnostics(JSONObject status) {
        StringBuilder builder = new StringBuilder(900);
        builder.append("DIAGNOSTICS\n\n");
        builder.append("Logcat tag    ").append(BrokerService.TAG).append('\n');
        builder.append("HTTP status   http://127.0.0.1:8765/status\n");
        builder.append("WebSocket     ws://127.0.0.1:8765/rustyxr/v1/events\n\n");

        JSONObject client = status.optJSONObject("client");
        if (client != null) {
            builder.append("CLIENT\n");
            builder.append("id            ").append(client.optString("client_id", "")).append('\n');
            builder.append("package       ").append(client.optString("app_package", "")).append('\n');
            builder.append("version       ").append(client.optString("app_version", "")).append("\n\n");
        }

        JSONArray capabilities = status.optJSONArray("capabilities");
        builder.append("CAPABILITIES\n");
        if (capabilities == null || capabilities.length() == 0) {
            builder.append("- none reported\n");
        } else {
            for (int i = 0; i < capabilities.length(); i++) {
                builder.append("- ").append(capabilities.optString(i)).append('\n');
            }
        }

        JSONObject statusError = "status_error".equals(status.optString("type")) ? status : null;
        if (statusError != null) {
            builder.append('\n').append("LAST ERROR\n").append(statusError.optString("message", ""));
        }
        return builder.toString();
    }

    private void appendCounter(StringBuilder builder, JSONObject counters, String name) {
        builder.append(String.format(Locale.ROOT, "%-28s %d\n", name, counters.optLong(name, 0L)));
    }

    private String transportEnabled(JSONObject value) {
        if (value == null) {
            return "not reported";
        }
        return value.optBoolean("enabled") ? "enabled" : "disabled";
    }

    private String formatUptime(long uptimeMs) {
        long seconds = Math.max(0L, uptimeMs / 1000L);
        long minutes = seconds / 60L;
        long remainingSeconds = seconds % 60L;
        return minutes + "m " + remainingSeconds + "s";
    }

    private static String safeJson(String value) {
        if (value == null) {
            return "";
        }
        return value.replace("\\", "\\\\").replace("\"", "\\\"");
    }
}
