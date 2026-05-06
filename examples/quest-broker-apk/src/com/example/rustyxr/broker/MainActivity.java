package com.example.rustyxr.broker;

import android.Manifest;
import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.Editable;
import android.text.InputType;
import android.text.TextUtils;
import android.text.TextWatcher;
import android.util.Log;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.EditText;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedInputStream;
import java.lang.ref.WeakReference;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.ArrayList;
import java.util.List;
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
    private static final int REQUEST_POLAR_PERMISSIONS = 8766;
    private static final long DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS = 60_000L;
    private static final long DEFAULT_STATUS_REFRESH_MS = 2_000L;
    private static final long POLAR_STATUS_REFRESH_MS = 500L;
    private static final String[] PAGES = { "Dashboard", "Polar", "Launcher", "Streams", "Commands", "Diagnostics" };
    private static volatile WeakReference<MainActivity> activeActivity = new WeakReference<>(null);

    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Runnable refreshRunnable = new Runnable() {
        @Override
        public void run() {
            refreshStatus();
            handler.postDelayed(this, statusRefreshDelayMs());
        }
    };

    private LinearLayout navBar;
    private LinearLayout pagePanel;
    private ScrollView pageScroll;
    private TextView subtitleView;
    private TextView bodyView;
    private TextView footerView;
    private LauncherStore launcherStore;
    private JSONObject lastStatus;
    private String currentPage = PAGES[0];
    private String returnAppPackage = "";
    private String returnClientId = "";
    private String selectedLauncherListId = "";
    private String launcherQuery = "";
    private boolean openedByBrokerCommand;
    private boolean pendingPolarStartAfterPermission;
    private String pendingPolarDeviceAddress = "";
    private long pendingPolarScanTimeoutMs = DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS;
    private boolean polarBreathDraftLoaded;
    private String polarBreathAnalysisRateHz = "";
    private String polarBreathCalibrationFrames = "";
    private String polarBreathMinDeltaG = "";
    private String polarBreathMinTravelG = "";
    private String polarBreathSampleEma = "";
    private String polarBreathProjectionEma = "";
    private String polarBreathLowerQuantile = "";
    private String polarBreathUpperQuantile = "";
    private String polarBreathEdgeEase = "";
    private String polarBreathVolumeDelta = "";
    private String polarBreathAccBaseMode = "";
    private String polarBreathInvertVolume = "";
    private boolean statusRefreshInFlight;

    @Override
    protected void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        activeActivity = new WeakReference<>(this);
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        startBrokerService(getIntent());
        launcherStore = new LauncherStore(this);
        selectedLauncherListId = launcherStore.selectedListIdOrDefault(selectedLauncherListId);
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

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode != REQUEST_POLAR_PERMISSIONS) {
            return;
        }

        boolean granted = grantResults.length > 0;
        for (int i = 0; i < grantResults.length; i++) {
            if (grantResults[i] != PackageManager.PERMISSION_GRANTED) {
                granted = false;
                break;
            }
        }

        if (granted && pendingPolarStartAfterPermission) {
            String deviceAddress = pendingPolarDeviceAddress;
            long scanTimeoutMs = pendingPolarScanTimeoutMs;
            pendingPolarStartAfterPermission = false;
            pendingPolarDeviceAddress = "";
            startPolarPmdFromConsole(deviceAddress, scanTimeoutMs);
            return;
        }

        pendingPolarStartAfterPermission = false;
        if (granted) {
            showLaunchToast("Bluetooth permission granted");
        } else {
            showLaunchToast("Bluetooth permission is required for Polar PMD");
        }
        refreshStatus();
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
        navScroll.setFocusable(false);
        navScroll.setDescendantFocusability(ViewGroup.FOCUS_AFTER_DESCENDANTS);
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
        scroll.setFocusable(false);
        scroll.setDescendantFocusability(ViewGroup.FOCUS_AFTER_DESCENDANTS);
        pageScroll = scroll;
        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setPadding(22, 20, 22, 20);
        panel.setBackground(panelBackground(PANEL, 14, Color.rgb(45, 57, 63)));
        pagePanel = panel;
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
        if (statusRefreshInFlight) {
            return;
        }
        statusRefreshInFlight = true;
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
                        statusRefreshInFlight = false;
                        if (isEditingText()) {
                            return;
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
        if (pagePanel == null || bodyView == null) {
            return;
        }

        if ("Launcher".equals(currentPage)) {
            renderLauncherPage();
            return;
        }

        JSONObject status = lastStatus;
        if (status == null) {
            showTextPage("Starting broker service...\n\nStatus refresh runs every 2 seconds.");
            return;
        }

        if ("Streams".equals(currentPage)) {
            showTextPage(buildStreams(status));
        } else if ("Commands".equals(currentPage)) {
            showTextPage(buildCommands(status));
        } else if ("Polar".equals(currentPage)) {
            renderPolarPage(status);
        } else if ("Diagnostics".equals(currentPage)) {
            showTextPage(buildDiagnostics(status));
        } else {
            showTextPage(buildDashboard(status));
        }
    }

    private void showTextPage(String text) {
        pagePanel.removeAllViews();
        pagePanel.addView(bodyView);
        bodyView.setText(text);
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

    private void renderLauncherPage() {
        pagePanel.removeAllViews();
        if (launcherStore == null) {
            launcherStore = new LauncherStore(this);
        }

        selectedLauncherListId = launcherStore.selectedListIdOrDefault(selectedLauncherListId);
        final LauncherStore.AppList selectedList = launcherStore.selectedList(selectedLauncherListId);

        addSectionTitle("LAUNCHER");
        addBodyText("Create named shortcuts for launchable apps visible to this broker APK. This uses normal Android PackageManager launch paths; shell-helper enhanced mode is not required.");

        addSectionTitle("LISTS");
        HorizontalScrollView listScroll = new HorizontalScrollView(this);
        listScroll.setHorizontalScrollBarEnabled(false);
        listScroll.setFocusable(false);
        listScroll.setDescendantFocusability(ViewGroup.FOCUS_AFTER_DESCENDANTS);
        LinearLayout listRow = new LinearLayout(this);
        listRow.setOrientation(LinearLayout.HORIZONTAL);
        listScroll.addView(listRow);
        pagePanel.addView(listScroll, matchWrapParams(0, 0, 0, 12));

        List<LauncherStore.AppList> lists = launcherStore.lists();
        for (int i = 0; i < lists.size(); i++) {
            final LauncherStore.AppList list = lists.get(i);
            Button listButton = navButton(list.name + " (" + list.apps.size() + ")");
            boolean selected = list.id.equals(selectedList.id);
            listButton.setTextColor(selected ? Color.rgb(5, 23, 18) : TEXT);
            listButton.setBackground(panelBackground(
                selected ? ACCENT_STRONG : PANEL_ALT,
                12,
                selected ? ACCENT_STRONG : Color.rgb(55, 70, 76)));
            listButton.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View view) {
                    selectedLauncherListId = list.id;
                    renderLauncherPage();
                }
            });
            listRow.addView(listButton);
        }

        final EditText listNameEdit = editText("List name", selectedList.name);
        LinearLayout listEditRow = row();
        listEditRow.addView(listNameEdit, weightedParams(1f, 0, 0, 10, 0));

        Button saveNameButton = actionButton("Save Name");
        saveNameButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                LauncherStore.AppList renamed = launcherStore.renameList(selectedList.id, listNameEdit.getText().toString());
                selectedLauncherListId = renamed.id;
                renderLauncherPage();
            }
        });
        listEditRow.addView(saveNameButton);

        Button newListButton = actionButton("New List");
        newListButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                LauncherStore.AppList created = launcherStore.createList(listNameEdit.getText().toString());
                selectedLauncherListId = created.id;
                renderLauncherPage();
            }
        });
        listEditRow.addView(newListButton, wrapParams(10, 0, 0, 0));

        if (!"default".equals(selectedList.id) && lists.size() > 1) {
            Button deleteButton = actionButton("Delete");
            deleteButton.setTextColor(WARN);
            deleteButton.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View view) {
                    selectedLauncherListId = launcherStore.deleteList(selectedList.id);
                    renderLauncherPage();
                }
            });
            listEditRow.addView(deleteButton, wrapParams(10, 0, 0, 0));
        }
        pagePanel.addView(listEditRow, matchWrapParams(0, 0, 0, 16));

        addSectionTitle("APPS IN " + selectedList.name.toUpperCase(Locale.ROOT));
        if (selectedList.apps.isEmpty()) {
            addBodyText("No apps saved in this list yet. Search below and add visible launchable apps.");
        } else {
            for (int i = 0; i < selectedList.apps.size(); i++) {
                final LauncherStore.AppTarget app = selectedList.apps.get(i);
                LinearLayout appRow = appRow(app);

                Button launchButton = actionButton("Launch");
                launchButton.setOnClickListener(new View.OnClickListener() {
                    @Override
                    public void onClick(View view) {
                        launchApp(app);
                    }
                });
                appRow.addView(launchButton);

                Button removeButton = actionButton("Remove");
                removeButton.setTextColor(WARN);
                removeButton.setOnClickListener(new View.OnClickListener() {
                    @Override
                    public void onClick(View view) {
                        launcherStore.removeApp(selectedList.id, app.key());
                        renderLauncherPage();
                    }
                });
                appRow.addView(removeButton, wrapParams(10, 0, 0, 0));

                pagePanel.addView(appRow, matchWrapParams(0, 0, 0, 8));
            }
        }

        addSectionTitle("FIND APPS");
        final EditText searchEdit = editText("Search app name or package", launcherQuery);
        LinearLayout searchRow = row();
        searchRow.addView(searchEdit, weightedParams(1f, 0, 0, 10, 0));

        Button searchButton = actionButton("Search");
        searchButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                launcherQuery = searchEdit.getText().toString();
                renderLauncherPage();
            }
        });
        searchRow.addView(searchButton);

        Button clearButton = actionButton("Clear");
        clearButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                launcherQuery = "";
                renderLauncherPage();
            }
        });
        searchRow.addView(clearButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(searchRow, matchWrapParams(0, 0, 0, 12));

        List<LauncherStore.AppTarget> results = launcherStore.searchLaunchableApps(launcherQuery);
        if (TextUtils.isEmpty(launcherQuery)) {
            addBodyText("Showing the first visible launchable apps. Type a search term to narrow by app name, package, or activity.");
        } else if (results.isEmpty()) {
            addBodyText("No visible launchable apps matched \"" + launcherQuery + "\".");
        }

        for (int i = 0; i < results.size(); i++) {
            final LauncherStore.AppTarget app = results.get(i);
            final boolean alreadyAdded = listContains(selectedList, app.key());
            LinearLayout resultRow = appRow(app);

            Button addButton = actionButton(alreadyAdded ? "Added" : "Add");
            addButton.setEnabled(!alreadyAdded);
            addButton.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View view) {
                    launcherStore.addApp(selectedList.id, app);
                    renderLauncherPage();
                }
            });
            resultRow.addView(addButton);

            Button launchButton = actionButton("Launch");
            launchButton.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View view) {
                    launchApp(app);
                }
            });
            resultRow.addView(launchButton, wrapParams(10, 0, 0, 0));
            pagePanel.addView(resultRow, matchWrapParams(0, 0, 0, 8));
        }
    }

    private void renderPolarPage(final JSONObject status) {
        final int previousScrollY = pageScroll != null ? pageScroll.getScrollY() : 0;
        pagePanel.removeAllViews();

        JSONObject polarPmd = status != null ? status.optJSONObject("polarPmd") : null;
        String requestedAddress = polarPmd != null ? polarPmd.optString("requested_device_address", "") : "";
        long currentTimeoutMs = polarPmd != null
            ? polarPmd.optLong("scan_timeout_ms", DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS)
            : DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS;
        if (currentTimeoutMs <= 0L) {
            currentTimeoutMs = DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS;
        }

        addSectionTitle("POLAR PMD");
        addBodyText("Start the broker-owned Android BLE Polar PMD source here. When it reaches streaming, the broker publishes bio:polar_acc and derived bio:breath for localhost clients.");

        TextView statusText = textView(14, false, TEXT);
        statusText.setTypeface(Typeface.MONOSPACE);
        statusText.setLineSpacing(0f, 1.08f);
        statusText.setText(buildPolarConsoleStatus(status));
        pagePanel.addView(statusText, matchWrapParams(0, 0, 0, 14));

        final EditText deviceAddressEdit = editText("Device address (optional)", requestedAddress);
        pagePanel.addView(deviceAddressEdit, matchWrapParams(0, 0, 0, 8));

        final EditText scanTimeoutEdit = editText("Scan timeout ms", Long.toString(currentTimeoutMs));
        scanTimeoutEdit.setInputType(InputType.TYPE_CLASS_NUMBER);
        pagePanel.addView(scanTimeoutEdit, matchWrapParams(0, 0, 0, 12));

        LinearLayout controls = row();

        Button startButton = actionButton("Start Polar");
        startButton.setTextColor(Color.rgb(7, 24, 18));
        startButton.setBackground(panelBackground(ACCENT_STRONG, 12, ACCENT_STRONG));
        startButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                startPolarPmdFromConsole(
                    deviceAddressEdit.getText().toString(),
                    parseScanTimeoutMs(scanTimeoutEdit.getText().toString()));
            }
        });
        controls.addView(startButton);

        Button stopButton = actionButton("Stop Polar");
        stopButton.setTextColor(WARN);
        stopButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                stopPolarPmdFromConsole();
            }
        });
        controls.addView(stopButton, wrapParams(10, 0, 0, 0));

        Button refreshButton = actionButton("Refresh Status");
        refreshButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                refreshPolarPmdFromConsole();
            }
        });
        controls.addView(refreshButton, wrapParams(10, 0, 0, 0));

        List<String> missingPermissions = missingPolarRuntimePermissions();
        if (!missingPermissions.isEmpty()) {
            Button permissionButton = actionButton("Grant Bluetooth");
            permissionButton.setTextColor(WARN);
            permissionButton.setOnClickListener(new View.OnClickListener() {
                @Override
                public void onClick(View view) {
                    requestPolarRuntimePermissions();
                }
            });
            controls.addView(permissionButton, wrapParams(10, 0, 0, 0));
        }

        pagePanel.addView(controls, matchWrapParams(0, 0, 0, 14));

        if (!missingPermissions.isEmpty()) {
            addBodyText("Missing runtime permissions: " + TextUtils.join(", ", missingPermissions));
        }

        renderPolarBreathTuning(status);

        addSectionTitle("HANDOFF");
        addBodyText("After bio:breath is active, use the Launcher page to start a target XR app. The broker service remains active in the background.");

        if (pageScroll != null && previousScrollY > 0) {
            pageScroll.post(new Runnable() {
                @Override
                public void run() {
                    pageScroll.scrollTo(0, previousScrollY);
                }
            });
        }
    }

    private void renderPolarBreathTuning(final JSONObject status) {
        if (!polarBreathDraftLoaded) {
            loadPolarBreathDraftFromStatus(status);
        }

        addSectionTitle("POLAR BREATH TUNING");
        addBodyText("Tune the broker-side Polar accelerometer breath tracker. Field names match the Unity runtime config where the broker supports the same parameter.");

        final EditText analysisRateEdit = draftEditText("analysisRateHz", polarBreathAnalysisRateHz, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathAnalysisRateHz = value;
            }
        });
        setDecimalInput(analysisRateEdit);
        addBreathTuningRow("analysisRateHz", analysisRateEdit);

        final EditText calibrationFramesEdit = draftEditText("calibrationAcceptedFrames", polarBreathCalibrationFrames, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathCalibrationFrames = value;
            }
        });
        calibrationFramesEdit.setInputType(InputType.TYPE_CLASS_NUMBER);
        addBreathTuningRow("calibrationAcceptedFrames", calibrationFramesEdit);

        final EditText minDeltaEdit = draftEditText("minAcceptedDeltaG", polarBreathMinDeltaG, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathMinDeltaG = value;
            }
        });
        setDecimalInput(minDeltaEdit);
        addBreathTuningRow("minAcceptedDeltaG", minDeltaEdit);

        final EditText minTravelEdit = draftEditText("minCalibrationTravelG", polarBreathMinTravelG, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathMinTravelG = value;
            }
        });
        setDecimalInput(minTravelEdit);
        addBreathTuningRow("minCalibrationTravelG", minTravelEdit);

        final EditText sampleEmaEdit = draftEditText("sampleEmaAlpha", polarBreathSampleEma, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathSampleEma = value;
            }
        });
        setDecimalInput(sampleEmaEdit);
        addBreathTuningRow("sampleEmaAlpha", sampleEmaEdit);

        final EditText projectionEmaEdit = draftEditText("projectionEmaAlpha", polarBreathProjectionEma, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathProjectionEma = value;
            }
        });
        setDecimalInput(projectionEmaEdit);
        addBreathTuningRow("projectionEmaAlpha", projectionEmaEdit);

        final EditText lowerQuantileEdit = draftEditText("boundsLowerQuantile", polarBreathLowerQuantile, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathLowerQuantile = value;
            }
        });
        setDecimalInput(lowerQuantileEdit);
        addBreathTuningRow("boundsLowerQuantile", lowerQuantileEdit);

        final EditText upperQuantileEdit = draftEditText("boundsUpperQuantile", polarBreathUpperQuantile, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathUpperQuantile = value;
            }
        });
        setDecimalInput(upperQuantileEdit);
        addBreathTuningRow("boundsUpperQuantile", upperQuantileEdit);

        final EditText edgeEaseEdit = draftEditText("boundsEdgeEase", polarBreathEdgeEase, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathEdgeEase = value;
            }
        });
        setDecimalInput(edgeEaseEdit);
        addBreathTuningRow("boundsEdgeEase", edgeEaseEdit);

        final EditText volumeDeltaEdit = draftEditText("volumeEventMinDelta", polarBreathVolumeDelta, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathVolumeDelta = value;
            }
        });
        setDecimalInput(volumeDeltaEdit);
        addBreathTuningRow("volumeEventMinDelta", volumeDeltaEdit);

        final EditText accBaseModeEdit = draftEditText("accBaseMode", polarBreathAccBaseMode, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathAccBaseMode = value;
            }
        });
        addBreathTuningRow("accBaseMode", accBaseModeEdit);

        final EditText invertVolumeEdit = draftEditText("invertVolume", polarBreathInvertVolume, new DraftUpdater() {
            @Override
            public void update(String value) {
                polarBreathInvertVolume = value;
            }
        });
        addBreathTuningRow("invertVolume", invertVolumeEdit);

        LinearLayout firstRow = row();
        Button applyButton = actionButton("Apply Tuning");
        applyButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                applyPolarBreathTuning(false);
            }
        });
        firstRow.addView(applyButton);

        Button applyCalibrateButton = actionButton("Apply + Calibrate");
        applyCalibrateButton.setTextColor(Color.rgb(7, 24, 18));
        applyCalibrateButton.setBackground(panelBackground(ACCENT_STRONG, 12, ACCENT_STRONG));
        applyCalibrateButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                applyPolarBreathTuning(true);
            }
        });
        firstRow.addView(applyCalibrateButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(firstRow, matchWrapParams(0, 4, 0, 8));

        LinearLayout secondRow = row();
        Button beginButton = actionButton("Begin Calibration");
        beginButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                beginPolarBreathCalibrationFromConsole();
            }
        });
        secondRow.addView(beginButton);

        Button resetButton = actionButton("Reset Calibration");
        resetButton.setTextColor(WARN);
        resetButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                resetPolarBreathCalibrationFromConsole();
            }
        });
        secondRow.addView(resetButton, wrapParams(10, 0, 0, 0));

        Button loadButton = actionButton("Load Current");
        loadButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                loadPolarBreathDraftFromStatus(status);
                renderCurrentPage();
            }
        });
        secondRow.addView(loadButton, wrapParams(10, 0, 0, 0));

        Button defaultsButton = actionButton("Unity Defaults");
        defaultsButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                loadPolarUnityDefaults();
                renderCurrentPage();
            }
        });
        secondRow.addView(defaultsButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(secondRow, matchWrapParams(0, 0, 0, 14));
    }

    private String buildPolarConsoleStatus(JSONObject status) {
        StringBuilder builder = new StringBuilder(900);
        JSONObject polarPmd = status != null ? status.optJSONObject("polarPmd") : null;
        JSONObject breathAssessment = status != null ? status.optJSONObject("breathAssessment") : null;

        if (polarPmd == null) {
            builder.append("Polar PMD status is not reported yet.\n");
        } else {
            builder.append("state         ").append(polarPmd.optString("state", "unknown")).append('\n');
            builder.append("enabled       ").append(polarPmd.optBoolean("enabled")).append('\n');
            builder.append("scan timeout  ").append(polarPmd.optLong("scan_timeout_ms", 0L)).append(" ms\n");
            String requested = polarPmd.optString("requested_device_address", "");
            if (requested.length() > 0) {
                builder.append("requested     ").append(requested).append('\n');
            }
            builder.append("device        ").append(polarPmd.optString("device_name", "")).append('\n');
            builder.append("address       ").append(polarPmd.optString("device_address", "")).append('\n');
            if (polarPmd.has("battery_percent")) {
                builder.append("battery       ").append(polarPmd.optInt("battery_percent", 0)).append("%\n");
            }
            if (polarPmd.has("negotiated_mtu")) {
                builder.append("mtu           ").append(polarPmd.optInt("negotiated_mtu", 0)).append('\n');
            }
            builder.append("scan reports  ").append(polarPmd.optLong("scan_report_count", 0L)).append('\n');
            builder.append("ignored scan  ").append(polarPmd.optLong("ignored_scan_report_count", 0L)).append('\n');
            builder.append("acc frames    ").append(polarPmd.optLong("acc_frame_count", 0L)).append('\n');
            builder.append("acc samples   ").append(polarPmd.optLong("acc_sample_count", 0L)).append('\n');
            String missing = polarPmd.optString("missing_permissions", "");
            if (missing.length() > 0) {
                builder.append("permissions   ").append(missing).append('\n');
            }
            String lastError = polarPmd.optString("last_error", "");
            if (lastError.length() > 0) {
                builder.append("last error    ").append(lastError).append('\n');
            }
            appendRecentScanCandidates(builder, polarPmd);
        }

        builder.append('\n');
        if (breathAssessment == null) {
            builder.append("Breath assessment status is not reported yet.\n");
        } else {
            builder.append("breath state  ").append(breathAssessment.optString("state", "unknown")).append('\n');
            builder.append("output        ").append(breathAssessment.optString("output_stream", "bio:breath")).append('\n');
            builder.append("polar frames  ").append(breathAssessment.optLong("accepted_polar_frames", 0L)).append('\n');
            builder.append("assessments   ").append(breathAssessment.optLong("emitted_assessments", 0L)).append('\n');
            JSONObject latest = breathAssessment.optJSONObject("latest_assessment");
            if (latest != null) {
                builder.append("latest src    ").append(latest.optString("source", "")).append('\n');
                builder.append("latest state  ").append(latest.optString("state", "")).append('\n');
                builder.append("latest volume ")
                    .append(String.format(Locale.ROOT, "%.3f", latest.optDouble("volume01", 0.0d)))
                    .append('\n');
            }
            JSONObject polarSource = polarBreathSourceStatus(status);
            if (polarSource != null) {
                builder.append("polar state   ").append(polarSource.optString("state", "unknown")).append('\n');
                builder.append("calibrated    ").append(polarSource.optBoolean("is_calibrated", false)).append('\n');
                builder.append("cal samples   ")
                    .append(polarSource.optInt("calibration_samples", 0))
                    .append('/')
                    .append(polarSource.optInt("calibration_frame_count", 0))
                    .append('\n');
                builder.append("polar volume  ")
                    .append(String.format(Locale.ROOT, "%.3f", polarSource.optDouble("volume01", 0.0d)))
                    .append('\n');
                String sourceError = polarSource.optString("last_error", "");
                if (sourceError.length() > 0) {
                    builder.append("breath error  ").append(sourceError).append('\n');
                }
            }
        }

        return builder.toString();
    }

    private void appendRecentScanCandidates(StringBuilder builder, JSONObject polarPmd) {
        JSONArray candidates = polarPmd.optJSONArray("recent_scan_candidates");
        if (candidates == null || candidates.length() == 0) {
            return;
        }

        builder.append("recent scan\n");
        int count = Math.min(5, candidates.length());
        for (int i = 0; i < count; i++) {
            JSONObject candidate = candidates.optJSONObject(i);
            if (candidate == null) {
                continue;
            }

            builder.append(candidate.optBoolean("accepted") ? "+ " : "- ");
            builder.append(candidate.optString("name", ""));
            builder.append(" rssi=").append(candidate.optInt("rssi", 0));
            builder.append(" score=").append(candidate.optInt("match_score", 0));
            if (candidate.optBoolean("heart_rate_service", false)) {
                builder.append(" hr");
            }
            if (candidate.optBoolean("pmd_service", false)) {
                builder.append(" pmd");
            }
            builder.append('\n');
        }
    }

    private void addBreathTuningRow(String label, EditText editText) {
        LinearLayout row = row();
        TextView labelView = textView(13, false, MUTED);
        labelView.setText(label);
        row.addView(labelView, weightedParams(0.54f, 0, 0, 10, 0));
        row.addView(editText, weightedParams(0.46f, 0, 0, 0, 0));
        pagePanel.addView(row, matchWrapParams(0, 0, 0, 7));
    }

    private void loadPolarBreathDraftFromStatus(JSONObject status) {
        JSONObject config = polarBreathConfig(status);
        polarBreathAnalysisRateHz = formatConfigNumber(config.optDouble("nominal_analysis_rate_hz", 10.0d));
        polarBreathCalibrationFrames = Integer.toString(config.optInt("calibration_frame_count", 120));
        polarBreathMinDeltaG = formatConfigNumber(config.optDouble("min_accepted_delta", 0.0005d));
        polarBreathMinTravelG = formatConfigNumber(config.optDouble("min_travel", 0.010d));
        polarBreathSampleEma = formatConfigNumber(config.optDouble("sample_ema_alpha", 0.10d));
        polarBreathProjectionEma = formatConfigNumber(config.optDouble("projection_ema_alpha", 0.10d));
        polarBreathLowerQuantile = formatConfigNumber(config.optDouble("low_quantile", 0.05d));
        polarBreathUpperQuantile = formatConfigNumber(config.optDouble("high_quantile", 0.95d));
        polarBreathEdgeEase = formatConfigNumber(config.optDouble("edge_ease", 0.03d));
        polarBreathVolumeDelta = formatConfigNumber(config.optDouble("delta_threshold", 0.001d));
        polarBreathAccBaseMode = config.optString("acc_base_mode", "xz");
        polarBreathInvertVolume = Boolean.toString(config.optBoolean("invert_volume", false));
        polarBreathDraftLoaded = true;
    }

    private void loadPolarUnityDefaults() {
        polarBreathAnalysisRateHz = "10";
        polarBreathCalibrationFrames = "120";
        polarBreathMinDeltaG = "0.0005";
        polarBreathMinTravelG = "0.010";
        polarBreathSampleEma = "0.10";
        polarBreathProjectionEma = "0.10";
        polarBreathLowerQuantile = "0.05";
        polarBreathUpperQuantile = "0.95";
        polarBreathEdgeEase = "0.03";
        polarBreathVolumeDelta = "0.001";
        polarBreathAccBaseMode = "Xz";
        polarBreathInvertVolume = "false";
        polarBreathDraftLoaded = true;
    }

    private JSONObject polarBreathConfig(JSONObject status) {
        JSONObject source = polarBreathSourceStatus(status);
        JSONObject config = source != null ? source.optJSONObject("config") : null;
        return config != null ? config : new JSONObject();
    }

    private JSONObject polarBreathSourceStatus(JSONObject status) {
        JSONObject breathAssessment = status != null ? status.optJSONObject("breathAssessment") : null;
        JSONObject sources = breathAssessment != null ? breathAssessment.optJSONObject("sources") : null;
        return sources != null ? sources.optJSONObject("polar_acc") : null;
    }

    private String formatConfigNumber(double value) {
        String formatted = String.format(Locale.ROOT, "%.6f", value);
        while (formatted.indexOf('.') >= 0 && formatted.endsWith("0")) {
            formatted = formatted.substring(0, formatted.length() - 1);
        }
        if (formatted.endsWith(".")) {
            formatted = formatted.substring(0, formatted.length() - 1);
        }
        return formatted;
    }

    private JSONObject buildPolarBreathParams(boolean resetCalibration) throws Exception {
        JSONObject params = new JSONObject();
        params.put("source", "polar_acc");
        params.put("reset_calibration", resetCalibration);
        putDoubleParam(params, "analysisRateHz", polarBreathAnalysisRateHz);
        putIntParam(params, "calibrationAcceptedFrames", polarBreathCalibrationFrames);
        putDoubleParam(params, "minAcceptedDeltaG", polarBreathMinDeltaG);
        putDoubleParam(params, "minCalibrationTravelG", polarBreathMinTravelG);
        putDoubleParam(params, "sampleEmaAlpha", polarBreathSampleEma);
        putDoubleParam(params, "projectionEmaAlpha", polarBreathProjectionEma);
        putDoubleParam(params, "boundsLowerQuantile", polarBreathLowerQuantile);
        putDoubleParam(params, "boundsUpperQuantile", polarBreathUpperQuantile);
        putDoubleParam(params, "boundsEdgeEase", polarBreathEdgeEase);
        putDoubleParam(params, "volumeEventMinDelta", polarBreathVolumeDelta);
        if (polarBreathAccBaseMode != null && polarBreathAccBaseMode.trim().length() > 0) {
            params.put("accBaseMode", polarBreathAccBaseMode.trim());
        }
        if (polarBreathInvertVolume != null && polarBreathInvertVolume.trim().length() > 0) {
            params.put("invertVolume", parseBooleanLike(polarBreathInvertVolume, false));
        }
        return params;
    }

    private void putDoubleParam(JSONObject params, String key, String value) throws Exception {
        if (value == null || value.trim().length() == 0) {
            return;
        }
        params.put(key, Double.parseDouble(value.trim()));
    }

    private void putIntParam(JSONObject params, String key, String value) throws Exception {
        if (value == null || value.trim().length() == 0) {
            return;
        }
        params.put(key, Integer.parseInt(value.trim()));
    }

    private boolean parseBooleanLike(String value, boolean fallback) {
        if (value == null) {
            return fallback;
        }
        String normalized = value.trim().toLowerCase(Locale.ROOT);
        if ("true".equals(normalized) || "1".equals(normalized) || "yes".equals(normalized) || "on".equals(normalized)) {
            return true;
        }
        if ("false".equals(normalized) || "0".equals(normalized) || "no".equals(normalized) || "off".equals(normalized)) {
            return false;
        }
        return fallback;
    }

    private void refreshPolarPmdFromConsole() {
        runBrokerConsoleAction("Refreshing Polar status", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.getPolarPmdStatusFromConsole();
            }
        });
    }

    private void startPolarPmdFromConsole(final String deviceAddress, final long scanTimeoutMs) {
        List<String> missingPermissions = missingPolarRuntimePermissions();
        if (!missingPermissions.isEmpty()) {
            pendingPolarStartAfterPermission = true;
            pendingPolarDeviceAddress = deviceAddress != null ? deviceAddress : "";
            pendingPolarScanTimeoutMs = scanTimeoutMs > 0L ? scanTimeoutMs : DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS;
            requestPolarRuntimePermissions();
            return;
        }

        startBrokerService(getIntent());
        runBrokerConsoleAction("Starting Polar PMD", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.startPolarPmdFromConsole(deviceAddress, scanTimeoutMs);
            }
        });
    }

    private void stopPolarPmdFromConsole() {
        startBrokerService(getIntent());
        runBrokerConsoleAction("Stopping Polar PMD", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.stopPolarPmdFromConsole();
            }
        });
    }

    private void applyPolarBreathTuning(final boolean beginCalibrationAfterApply) {
        startBrokerService(getIntent());
        runBrokerConsoleAction(beginCalibrationAfterApply ? "Applying tuning and starting calibration" : "Applying breath tuning", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                JSONObject ack = BrokerService.setPolarBreathParamsFromConsole(
                    buildPolarBreathParams(beginCalibrationAfterApply));
                if (beginCalibrationAfterApply && ack != null && ack.optBoolean("accepted", false)) {
                    return BrokerService.beginPolarBreathCalibrationFromConsole();
                }
                return ack;
            }
        });
    }

    private void beginPolarBreathCalibrationFromConsole() {
        startBrokerService(getIntent());
        runBrokerConsoleAction("Starting breath calibration", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.beginPolarBreathCalibrationFromConsole();
            }
        });
    }

    private void resetPolarBreathCalibrationFromConsole() {
        startBrokerService(getIntent());
        runBrokerConsoleAction("Resetting breath calibration", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.resetPolarBreathCalibrationFromConsole();
            }
        });
    }

    private void runBrokerConsoleAction(final String progressMessage, final BrokerConsoleAction action) {
        showLaunchToast(progressMessage);
        Thread thread = new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    final JSONObject ack = action.run();
                    handler.post(new Runnable() {
                        @Override
                        public void run() {
                            handleBrokerConsoleAck(ack);
                        }
                    });
                } catch (final Exception ex) {
                    handler.post(new Runnable() {
                        @Override
                        public void run() {
                            showLaunchToast("Command failed: " + ex.getMessage());
                            refreshStatus();
                        }
                    });
                }
            }
        }, "RustyXrBrokerConsoleCommand");
        thread.start();
    }

    private long statusRefreshDelayMs() {
        if ("Polar".equals(currentPage) && !isEditingText()) {
            return POLAR_STATUS_REFRESH_MS;
        }
        return DEFAULT_STATUS_REFRESH_MS;
    }

    private void handleBrokerConsoleAck(JSONObject ack) {
        if (ack == null) {
            showLaunchToast("Command returned no status");
            refreshStatus();
            return;
        }

        if (ack.optBoolean("accepted", false)) {
            String message = ack.optString("message", "command accepted");
            showLaunchToast(message.length() > 0 ? message : "Command accepted");
        } else {
            JSONObject error = ack.optJSONObject("error");
            String message = error != null ? error.optString("message", "") : ack.optString("message", "");
            showLaunchToast(message.length() > 0 ? message : "Command rejected");
        }
        refreshStatus();
    }

    private void requestPolarRuntimePermissions() {
        List<String> missingPermissions = missingPolarRuntimePermissions();
        if (missingPermissions.isEmpty()) {
            showLaunchToast("Bluetooth permissions already granted");
            return;
        }

        requestPermissions(
            missingPermissions.toArray(new String[missingPermissions.size()]),
            REQUEST_POLAR_PERMISSIONS);
    }

    private List<String> missingPolarRuntimePermissions() {
        ArrayList<String> missing = new ArrayList<>();
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            addMissingPermission(missing, Manifest.permission.BLUETOOTH_SCAN);
            addMissingPermission(missing, Manifest.permission.BLUETOOTH_CONNECT);
        } else {
            addMissingPermission(missing, Manifest.permission.ACCESS_FINE_LOCATION);
        }
        return missing;
    }

    private void addMissingPermission(List<String> missing, String permission) {
        if (checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
            missing.add(permission);
        }
    }

    private long parseScanTimeoutMs(String raw) {
        if (raw == null || raw.trim().length() == 0) {
            return DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS;
        }

        try {
            long parsed = Long.parseLong(raw.trim());
            return parsed > 0L ? parsed : DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS;
        } catch (NumberFormatException ex) {
            return DEFAULT_POLAR_UI_SCAN_TIMEOUT_MS;
        }
    }

    private interface BrokerConsoleAction {
        JSONObject run() throws Exception;
    }

    private void addSectionTitle(String value) {
        TextView title = textView(15, true, ACCENT_STRONG);
        title.setText(value);
        pagePanel.addView(title, matchWrapParams(0, 10, 0, 6));
    }

    private void addBodyText(String value) {
        TextView text = textView(14, false, MUTED);
        text.setText(value);
        pagePanel.addView(text, matchWrapParams(0, 0, 0, 10));
    }

    private LinearLayout appRow(LauncherStore.AppTarget app) {
        LinearLayout row = row();
        row.setPadding(14, 12, 14, 12);
        row.setBackground(panelBackground(PANEL_ALT, 12, Color.rgb(55, 70, 76)));

        LinearLayout labels = new LinearLayout(this);
        labels.setOrientation(LinearLayout.VERTICAL);

        TextView label = textView(16, true, TEXT);
        label.setText(app.label);
        labels.addView(label);

        TextView details = textView(12, false, MUTED);
        String kind = app.systemApp ? "system" : "user";
        details.setText(app.packageName + "\n" + app.activityName + "    " + app.source + " / " + kind);
        labels.addView(details);

        row.addView(labels, weightedParams(1f, 0, 0, 12, 0));
        return row;
    }

    private LinearLayout row() {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        return row;
    }

    private EditText editText(String hint, String value) {
        EditText editText = new EditText(this);
        editText.setText(value);
        editText.setHint(hint);
        editText.setSingleLine(true);
        editText.setTextSize(14);
        editText.setTextColor(TEXT);
        editText.setHintTextColor(MUTED);
        editText.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        editText.setPadding(14, 6, 14, 6);
        editText.setBackground(panelBackground(PANEL_ALT, 12, Color.rgb(55, 70, 76)));
        return editText;
    }

    private EditText draftEditText(String hint, String value, final DraftUpdater updater) {
        EditText editText = editText(hint, value);
        editText.addTextChangedListener(new TextWatcher() {
            @Override
            public void beforeTextChanged(CharSequence sequence, int start, int count, int after) {
            }

            @Override
            public void onTextChanged(CharSequence sequence, int start, int before, int count) {
            }

            @Override
            public void afterTextChanged(Editable editable) {
                if (updater != null) {
                    updater.update(editable != null ? editable.toString() : "");
                }
            }
        });
        return editText;
    }

    private void setDecimalInput(EditText editText) {
        editText.setInputType(
            InputType.TYPE_CLASS_NUMBER
                | InputType.TYPE_NUMBER_FLAG_DECIMAL
                | InputType.TYPE_NUMBER_FLAG_SIGNED);
    }

    private boolean isEditingText() {
        return getCurrentFocus() instanceof EditText;
    }

    private interface DraftUpdater {
        void update(String value);
    }

    private LinearLayout.LayoutParams matchWrapParams(int left, int top, int right, int bottom) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT);
        params.setMargins(left, top, right, bottom);
        return params;
    }

    private LinearLayout.LayoutParams wrapParams(int left, int top, int right, int bottom) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            LinearLayout.LayoutParams.WRAP_CONTENT);
        params.setMargins(left, top, right, bottom);
        return params;
    }

    private LinearLayout.LayoutParams weightedParams(float weight, int left, int top, int right, int bottom) {
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
            0,
            LinearLayout.LayoutParams.WRAP_CONTENT,
            weight);
        params.setMargins(left, top, right, bottom);
        return params;
    }

    private boolean listContains(LauncherStore.AppList list, String key) {
        for (LauncherStore.AppTarget app : list.apps) {
            if (app.key().equals(key)) {
                return true;
            }
        }
        return false;
    }

    private void launchApp(LauncherStore.AppTarget app) {
        Intent intent = launcherStore.buildLaunchIntent(app);
        if (intent == null) {
            showLaunchToast("No launch intent for " + app.packageName);
            return;
        }

        try {
            startActivity(intent);
            Log.i(BrokerService.TAG, "Launcher started " + app.packageName + "/" + app.activityName);
        } catch (ActivityNotFoundException | SecurityException ex) {
            showLaunchToast("Launch failed: " + ex.getMessage());
            Log.w(BrokerService.TAG, "Launcher failed for " + app.packageName + ": " + ex.getMessage());
        }
    }

    private void showLaunchToast(String message) {
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show();
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
            appendCounter(builder, counters, "videoLabMetricSamples");
            appendCounter(builder, counters, "videoLabEncodedStreamManifests");
            appendCounter(builder, counters, "videoLabEncodedSampleMetadata");
            builder.append('\n');
        }

        JSONObject lsl = status.optJSONObject("lsl");
        JSONObject osc = status.optJSONObject("osc");
        JSONObject cameraProvider = status.optJSONObject("cameraProvider");
        JSONObject shellHelper = status.optJSONObject("shellHelper");
        JSONObject polarPmd = status.optJSONObject("polarPmd");
        JSONObject breathAssessment = status.optJSONObject("breathAssessment");
        JSONObject videoLab = status.optJSONObject("videoLab");
        builder.append("TRANSPORTS\n");
        builder.append("LSL           ").append(lsl != null && lsl.optBoolean("enabled") ? "enabled" : "fallback/logcat").append('\n');
        builder.append("OSC ingress   ").append(transportEnabled(osc != null ? osc.optJSONObject("ingress") : null)).append('\n');
        builder.append("OSC egress    ").append(transportEnabled(osc != null ? osc.optJSONObject("egress") : null)).append('\n');
        builder.append("Camera meta   ").append(cameraProvider != null ? cameraProvider.optString("state", "unknown") : "unknown").append('\n');
        builder.append("Shell helper  ").append(shellHelper != null && shellHelper.optBoolean("connected") ? "connected" : "disconnected").append('\n');
        builder.append("Polar PMD     ").append(polarPmd != null ? polarPmd.optString("state", "unknown") : "not reported").append('\n');
        builder.append("Breath        ").append(breathAssessment != null ? breathAssessment.optString("state", "unknown") : "not reported").append('\n');
        builder.append("Video lab     ").append(videoLab != null ? videoLab.optString("state", "unknown") : "not reported").append('\n');
        builder.append('\n');
        builder.append("Use Polar to start broker-owned Polar PMD before launching a target XR app.\n");
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

        JSONObject cameraProvider = status.optJSONObject("cameraProvider");
        if (cameraProvider != null) {
            builder.append("\nCAMERA PROVIDER\n");
            builder.append("state         ").append(cameraProvider.optString("state", "")).append('\n');
            builder.append("tier          ").append(cameraProvider.optString("tier", "")).append('\n');
            builder.append("profile       ").append(cameraProvider.optString("projection_profile_id", "")).append('\n');
            JSONObject appProbe = cameraProvider.optJSONObject("app_camera_probe");
            if (appProbe != null) {
                builder.append("app cameras   ").append(appProbe.optInt("camera_id_count", 0)).append('\n');
                builder.append("app opens     ").append(appProbe.optInt("open_success_count", 0)).append('\n');
                builder.append("app captures  ").append(appProbe.optInt("capture_success_count", 0)).append('\n');
            }
            builder.append("visual OK     ").append(cameraProvider.optBoolean("visual_release_accepted")).append('\n');
            JSONArray limitations = cameraProvider.optJSONArray("limitations");
            if (limitations != null && limitations.length() > 0) {
                builder.append("limitations\n");
                for (int i = 0; i < limitations.length(); i++) {
                    builder.append("- ").append(limitations.optString(i)).append('\n');
                }
            }
        }

        JSONObject shellHelper = status.optJSONObject("shellHelper");
        if (shellHelper != null) {
            builder.append("\nSHELL HELPER\n");
            builder.append("connected     ").append(shellHelper.optBoolean("connected")).append('\n');
            builder.append("version       ").append(shellHelper.optString("helper_version", "")).append('\n');
            builder.append("uid           ").append(shellHelper.optString("uid", "")).append('\n');
            builder.append("requires ADB  ").append(shellHelper.optBoolean("requires_adb_authorization")).append('\n');
            builder.append("broker shell  ").append(shellHelper.optBoolean("normal_broker_apk_is_shell")).append('\n');
            JSONObject diagnostics = shellHelper.optJSONObject("diagnostics");
            JSONObject codecProbe = diagnostics != null ? diagnostics.optJSONObject("codec_probe") : null;
            if (codecProbe != null) {
                builder.append("codec count   ").append(codecProbe.optLong("codec_count", 0L)).append('\n');
                builder.append("surface fmt   ").append(codecProbe.optLong("surface_capable_count", 0L)).append('\n');
            }
        }

        JSONObject breathAssessment = status.optJSONObject("breathAssessment");
        JSONObject polarPmd = status.optJSONObject("polarPmd");
        if (polarPmd != null) {
            builder.append("\nPOLAR PMD\n");
            builder.append("state         ").append(polarPmd.optString("state", "")).append('\n');
            builder.append("enabled       ").append(polarPmd.optBoolean("enabled")).append('\n');
            builder.append("device        ").append(polarPmd.optString("device_name", "")).append('\n');
            builder.append("address       ").append(polarPmd.optString("device_address", "")).append('\n');
            if (polarPmd.has("battery_percent")) {
                builder.append("battery       ").append(polarPmd.optInt("battery_percent", 0)).append("%\n");
            }
            if (polarPmd.has("negotiated_mtu")) {
                builder.append("mtu           ").append(polarPmd.optInt("negotiated_mtu", 0)).append('\n');
            }
            builder.append("acc frames    ").append(polarPmd.optLong("acc_frame_count", 0L)).append('\n');
            builder.append("acc samples   ").append(polarPmd.optLong("acc_sample_count", 0L)).append('\n');
            builder.append("malformed     ").append(polarPmd.optLong("malformed_frame_count", 0L)).append('\n');
            builder.append("scan reports  ").append(polarPmd.optLong("scan_report_count", 0L)).append('\n');
            builder.append("ignored scan  ").append(polarPmd.optLong("ignored_scan_report_count", 0L)).append('\n');
            String missingPermissions = polarPmd.optString("missing_permissions", "");
            if (missingPermissions.length() > 0) {
                builder.append("permissions   ").append(missingPermissions).append('\n');
            }
            String lastError = polarPmd.optString("last_error", "");
            if (lastError.length() > 0) {
                builder.append("last error    ").append(lastError).append('\n');
            }
        }

        if (breathAssessment != null) {
            builder.append("\nBREATH ASSESSMENT\n");
            builder.append("state         ").append(breathAssessment.optString("state", "")).append('\n');
            builder.append("output        ").append(breathAssessment.optString("output_stream", "")).append('\n');
            builder.append("polar frames  ").append(breathAssessment.optLong("accepted_polar_frames", 0L)).append('\n');
            builder.append("controller    ").append(breathAssessment.optLong("accepted_controller_samples", 0L)).append('\n');
            builder.append("assessments   ").append(breathAssessment.optLong("emitted_assessments", 0L)).append('\n');
            JSONObject latest = breathAssessment.optJSONObject("latest_assessment");
            if (latest != null) {
                builder.append("latest src    ").append(latest.optString("source", "")).append('\n');
                builder.append("latest state  ").append(latest.optString("state", "")).append('\n');
                builder.append("latest volume ").append(String.format(Locale.ROOT, "%.3f", latest.optDouble("volume01", 0.0d))).append('\n');
            }
        }

        JSONObject videoLab = status.optJSONObject("videoLab");
        if (videoLab != null) {
            builder.append("\nVIDEO LAB\n");
            builder.append("state         ").append(videoLab.optString("state", "")).append('\n');
            builder.append("metric stream ").append(videoLab.optString("metric_stream", "")).append('\n');
            builder.append("samples       ").append(videoLab.optLong("accepted_metric_samples", 0L)).append('\n');
            builder.append("manifests     ").append(videoLab.optLong("accepted_encoded_stream_manifests", 0L)).append('\n');
            builder.append("sample meta   ").append(videoLab.optLong("accepted_encoded_sample_metadata", 0L)).append('\n');
            JSONObject latestManifest = videoLab.optJSONObject("latest_encoded_stream_manifest");
            if (latestManifest != null) {
                builder.append("encoded mime  ").append(latestManifest.optString("mime_type", "")).append('\n');
                builder.append("encoded size  ")
                    .append(latestManifest.optInt("width", 0))
                    .append('x')
                    .append(latestManifest.optInt("height", 0))
                    .append('\n');
            }
            JSONArray limitations = videoLab.optJSONArray("limitations");
            if (limitations != null && limitations.length() > 0) {
                builder.append("limitations\n");
                for (int i = 0; i < limitations.length(); i++) {
                    builder.append("- ").append(limitations.optString(i)).append('\n');
                }
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
