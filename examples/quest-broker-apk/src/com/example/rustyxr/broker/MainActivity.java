package com.example.rustyxr.broker;

import android.Manifest;
import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
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
    static final String EXTRA_CONSOLE_PAGE = "rustyxr.consolePage";
    static final String EXTRA_INITIAL_PAGE = "rustyxr.initialPage";
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
    private static final long EXPERIMENT_STATUS_REFRESH_MS = 1_000L;
    private static final long DEFAULT_DEVICE_WATCHDOG_INTERVAL_MS = 30_000L;
    private static final String DEFAULT_EXPERIMENT_TARGET_PACKAGE = "io.github.mesmerprism.rustyxr.makepad.camera";
    private static final String DEFAULT_EXPERIMENT_TARGET_ACTIVITY = "";
    private static final int DEFAULT_EXPERIMENT_LAUNCH_GUARD_TIMEOUT_MS = 20_000;
    private static final boolean DEFAULT_EXPERIMENT_LAUNCH_GUARD_PREVIEW_TIMEOUT_ENABLED = false;
    private static final String[] PAGES = { "Dashboard", "Clock", "System", "Experiment", "Polar", "Launcher", "Streams", "Commands", "Diagnostics" };
    private static volatile WeakReference<MainActivity> activeActivity = new WeakReference<>(null);
    private static volatile boolean activeActivityResumed;

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
    private volatile String currentPage = PAGES[0];
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
    private boolean experimentDraftLoaded;
    private String experimentTargetPackage = DEFAULT_EXPERIMENT_TARGET_PACKAGE;
    private String experimentTargetActivity = DEFAULT_EXPERIMENT_TARGET_ACTIVITY;
    private String experimentMode = "observe";
    private String experimentStrength = "0";
    private String experimentGlobalUv = "0";
    private String experimentLeftUv = "0";
    private String experimentRightUv = "0";
    private String experimentVerticalUv = "0";
    private String experimentSymmetricUv = "";
    private String experimentContentScale = "1.60";
    private boolean statusRefreshInFlight;

    static boolean isConsoleVisible() {
        return activeActivity.get() != null && activeActivityResumed;
    }

    static String activePageName() {
        MainActivity activity = activeActivity.get();
        if (activity == null || activity.currentPage == null || activity.currentPage.length() == 0) {
            return "";
        }
        return activity.currentPage;
    }

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
        applyRequestedPage(getIntent());
        renderCurrentPage();
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        activeActivity = new WeakReference<>(this);
        setIntent(intent);
        startBrokerService(intent);
        updateLaunchSubtitle(intent);
        applyRequestedPage(intent);
        refreshStatus();
    }

    @Override
    protected void onResume() {
        super.onResume();
        activeActivity = new WeakReference<>(this);
        activeActivityResumed = true;
        handler.removeCallbacks(refreshRunnable);
        handler.post(refreshRunnable);
    }

    @Override
    protected void onPause() {
        activeActivityResumed = false;
        handler.removeCallbacks(refreshRunnable);
        super.onPause();
    }

    @Override
    protected void onDestroy() {
        MainActivity activity = activeActivity.get();
        if (activity == this) {
            activeActivity = new WeakReference<>(null);
            activeActivityResumed = false;
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
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(serviceIntent);
        } else {
            startService(serviceIntent);
        }
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
        footerView.setText("Endpoint: http://127.0.0.1:8765/status    Clock: /clock/now    WebSocket: ws://127.0.0.1:8765/rustyxr/v1/events");
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
        } else if ("Clock".equals(currentPage)) {
            showTextPage(buildClock(status));
        } else if ("System".equals(currentPage)) {
            renderSystemPage(status);
        } else if ("Polar".equals(currentPage)) {
            renderPolarPage(status);
        } else if ("Experiment".equals(currentPage)) {
            renderExperimentPage(status);
        } else if ("Diagnostics".equals(currentPage)) {
            renderDiagnosticsPage(status);
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

    private void applyRequestedPage(Intent launchIntent) {
        if (launchIntent == null) {
            return;
        }

        String requestedPage = launchIntent.getStringExtra(EXTRA_CONSOLE_PAGE);
        if (TextUtils.isEmpty(requestedPage)) {
            requestedPage = launchIntent.getStringExtra(EXTRA_INITIAL_PAGE);
        }

        String page = normalizePageName(requestedPage);
        if (!TextUtils.isEmpty(page)) {
            currentPage = page;
        }
    }

    private static String normalizePageName(String value) {
        if (TextUtils.isEmpty(value)) {
            return "";
        }

        String normalized = value.trim();
        for (int i = 0; i < PAGES.length; i++) {
            if (PAGES[i].equalsIgnoreCase(normalized)) {
                return PAGES[i];
            }
        }
        return "";
    }

    private void renderSystemPage(final JSONObject status) {
        final int previousScrollY = pageScroll != null ? pageScroll.getScrollY() : 0;
        pagePanel.removeAllViews();

        addSectionTitle("SYSTEM SURFACE");
        addBodyText("This console is a normal Horizon OS 2D app panel. Home/Menu, system overlays, permissions, and managed-device policy remain system-owned.");

        TextView statusText = textView(14, false, TEXT);
        statusText.setTypeface(Typeface.MONOSPACE);
        statusText.setLineSpacing(0f, 1.08f);
        statusText.setText(buildSystemSurfaceStatus(status));
        pagePanel.addView(statusText, matchWrapParams(0, 0, 0, 14));

        addSectionTitle("SYSTEM SHORTCUTS");
        addBodyText("These buttons request standard Android settings activities. Availability and final presentation are device-build dependent.");

        LinearLayout firstRow = row();
        Button settingsButton = actionButton("Settings");
        settingsButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                openSystemSettings("Settings", Settings.ACTION_SETTINGS);
            }
        });
        firstRow.addView(settingsButton);

        Button wifiButton = actionButton("Wi-Fi");
        wifiButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                openSystemSettings("Wi-Fi", Settings.ACTION_WIFI_SETTINGS);
            }
        });
        firstRow.addView(wifiButton, wrapParams(10, 0, 0, 0));

        Button bluetoothButton = actionButton("Bluetooth");
        bluetoothButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                openSystemSettings("Bluetooth", Settings.ACTION_BLUETOOTH_SETTINGS);
            }
        });
        firstRow.addView(bluetoothButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(firstRow, matchWrapParams(0, 0, 0, 10));

        LinearLayout secondRow = row();
        Button appInfoButton = actionButton("App Info");
        appInfoButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                openBrokerAppDetails();
            }
        });
        secondRow.addView(appInfoButton);

        Button closeButton = actionButton("Close Console");
        closeButton.setTextColor(WARN);
        closeButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                closeConsole("system_page_close_console");
            }
        });
        secondRow.addView(closeButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(secondRow, matchWrapParams(0, 0, 0, 14));

        addSectionTitle("FOCUS CONTROL");
        addBodyText("These controls update broker experiment state only. Helper-side focus recovery is reactive and requires an authorized shell helper.");

        LinearLayout focusRow = row();
        Button observeButton = actionButton("Observe");
        observeButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                experimentMode = "observe";
                applyExperimentControl("broker", experimentMode, false, false);
            }
        });
        focusRow.addView(observeButton);

        Button brokerButton = actionButton("Broker Focus");
        brokerButton.setTextColor(Color.rgb(7, 24, 18));
        brokerButton.setBackground(panelBackground(ACCENT_STRONG, 12, ACCENT_STRONG));
        brokerButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                applyExperimentControl("broker", null, false, false);
            }
        });
        focusRow.addView(brokerButton, wrapParams(10, 0, 0, 0));

        Button offButton = actionButton("Control Off");
        offButton.setTextColor(WARN);
        offButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                experimentMode = "off";
                applyExperimentControl("broker", experimentMode, false, false);
            }
        });
        focusRow.addView(offButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(focusRow, matchWrapParams(0, 0, 0, 14));

        if (pageScroll != null) {
            pageScroll.post(new Runnable() {
                @Override
                public void run() {
                    pageScroll.setScrollY(previousScrollY);
                }
            });
        }
    }

    private void renderExperimentPage(final JSONObject status) {
        final int previousScrollY = pageScroll != null ? pageScroll.getScrollY() : 0;
        pagePanel.removeAllViews();
        if (!experimentDraftLoaded) {
            loadExperimentDraftFromStatus(status);
        }

        addSectionTitle("EXPERIMENT CONTROL");
        addBodyText("Configure a target app, apply hotload properties, and let the ADB shell helper reactively recover focus when Meta shell takes foreground.");

        TextView statusText = textView(14, false, TEXT);
        statusText.setTypeface(Typeface.MONOSPACE);
        statusText.setLineSpacing(0f, 1.08f);
        statusText.setText(buildExperimentConsoleStatus(status));
        pagePanel.addView(statusText, matchWrapParams(0, 0, 0, 14));

        addSectionTitle("TARGET");
        final EditText targetPackageEdit = draftEditText("Target package", experimentTargetPackage, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentTargetPackage = value;
            }
        });
        pagePanel.addView(targetPackageEdit, matchWrapParams(0, 0, 0, 8));

        final EditText targetActivityEdit = draftEditText("Target activity (optional)", experimentTargetActivity, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentTargetActivity = value;
            }
        });
        pagePanel.addView(targetActivityEdit, matchWrapParams(0, 0, 0, 8));

        final EditText modeEdit = draftEditText("Mode: off, observe, recover_target, recover_broker, toggle_broker_target, launch_target_guard, strict", experimentMode, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentMode = value;
            }
        });
        pagePanel.addView(modeEdit, matchWrapParams(0, 0, 0, 12));

        LinearLayout modeRow = row();
        Button observeButton = actionButton("Observe");
        observeButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                experimentMode = "observe";
                applyExperimentControl("broker", experimentMode, false, false);
            }
        });
        modeRow.addView(observeButton);

        Button toggleButton = actionButton("Toggle Kiosk");
        toggleButton.setTextColor(Color.rgb(7, 24, 18));
        toggleButton.setBackground(panelBackground(ACCENT_STRONG, 12, ACCENT_STRONG));
        toggleButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                experimentMode = "toggle_broker_target";
                applyExperimentControl("broker", experimentMode, false, false);
            }
        });
        modeRow.addView(toggleButton, wrapParams(10, 0, 0, 0));

        Button offButton = actionButton("Off");
        offButton.setTextColor(WARN);
        offButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                experimentMode = "off";
                applyExperimentControl("broker", experimentMode, false, false);
            }
        });
        modeRow.addView(offButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(modeRow, matchWrapParams(0, 0, 0, 14));

        addSectionTitle("MAKEPAD HOTLOAD");
        final EditText strengthEdit = draftEditText("strength", experimentStrength, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentStrength = value;
            }
        });
        setDecimalInput(strengthEdit);
        addExperimentTuningRow("Strength", strengthEdit);

        final EditText globalUvEdit = draftEditText("globalUv", experimentGlobalUv, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentGlobalUv = value;
            }
        });
        setDecimalInput(globalUvEdit);
        addExperimentTuningRow("Global UV", globalUvEdit);

        final EditText leftUvEdit = draftEditText("leftUv", experimentLeftUv, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentLeftUv = value;
            }
        });
        setDecimalInput(leftUvEdit);
        addExperimentTuningRow("Left UV", leftUvEdit);

        final EditText rightUvEdit = draftEditText("rightUv", experimentRightUv, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentRightUv = value;
            }
        });
        setDecimalInput(rightUvEdit);
        addExperimentTuningRow("Right UV", rightUvEdit);

        final EditText verticalUvEdit = draftEditText("verticalUv", experimentVerticalUv, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentVerticalUv = value;
            }
        });
        setDecimalInput(verticalUvEdit);
        addExperimentTuningRow("Vertical UV", verticalUvEdit);

        final EditText symmetricUvEdit = draftEditText("symmetricUv optional", experimentSymmetricUv, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentSymmetricUv = value;
            }
        });
        setDecimalInput(symmetricUvEdit);
        addExperimentTuningRow("Symmetric UV", symmetricUvEdit);

        final EditText contentScaleEdit = draftEditText("contentScale", experimentContentScale, new DraftUpdater() {
            @Override
            public void update(String value) {
                experimentContentScale = value;
            }
        });
        setDecimalInput(contentScaleEdit);
        addExperimentTuningRow("Content scale", contentScaleEdit);

        LinearLayout tuningRow = row();
        Button applyButton = actionButton("Apply Knobs");
        applyButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                applyExperimentControl(null, null, false, false);
            }
        });
        tuningRow.addView(applyButton);

        Button resetButton = actionButton("S108 Reset");
        resetButton.setTextColor(WARN);
        resetButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                loadExperimentS108Defaults();
                applyExperimentControl(null, null, true, false);
            }
        });
        tuningRow.addView(resetButton, wrapParams(10, 0, 0, 0));

        Button applyTargetButton = actionButton("Apply + Target");
        applyTargetButton.setTextColor(Color.rgb(7, 24, 18));
        applyTargetButton.setBackground(panelBackground(ACCENT_STRONG, 12, ACCENT_STRONG));
        applyTargetButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                experimentMode = "launch_target_guard";
                if (!isShellHelperConnected(status)) {
                    showLaunchToast("Shell helper required for guarded target launch");
                }
                applyExperimentControl("target", experimentMode, false, false);
            }
        });
        tuningRow.addView(applyTargetButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(tuningRow, matchWrapParams(0, 4, 0, 10));

        LinearLayout focusRow = row();
        Button launchButton = actionButton("Launch Target");
        launchButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                experimentMode = "launch_target_guard";
                if (!isShellHelperConnected(status)) {
                    showLaunchToast("Shell helper required for guarded target launch");
                }
                applyExperimentControl("target", experimentMode, false, false);
            }
        });
        focusRow.addView(launchButton);

        Button brokerButton = actionButton("Broker Focus");
        brokerButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                applyExperimentControl("broker", null, false, false);
            }
        });
        focusRow.addView(brokerButton, wrapParams(10, 0, 0, 0));

        Button loadButton = actionButton("Load Current");
        loadButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                loadExperimentDraftFromStatus(status);
                renderCurrentPage();
            }
        });
        focusRow.addView(loadButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(focusRow, matchWrapParams(0, 0, 0, 14));

        if (pageScroll != null && previousScrollY > 0) {
            pageScroll.post(new Runnable() {
                @Override
                public void run() {
                    pageScroll.scrollTo(0, previousScrollY);
                }
            });
        }
    }

    private void loadExperimentDraftFromStatus(JSONObject status) {
        JSONObject control = experimentControlStatus(status);
        if (control != null) {
            String targetPackage = control.optString("target_package", "");
            String targetActivity = control.optString("target_activity", "");
            if (!TextUtils.isEmpty(targetPackage)) {
                experimentTargetPackage = targetPackage;
            }
            experimentTargetActivity = targetActivity != null ? targetActivity : "";
            experimentMode = control.optString("mode", experimentMode);

            JSONObject tuning = control.optJSONObject("makepad_tuning");
            if (tuning != null) {
                experimentStrength = formatConfigNumber(tuning.optDouble("strength", 0.0d));
                experimentGlobalUv = formatConfigNumber(tuning.optDouble("global_uv", 0.0d));
                experimentLeftUv = formatConfigNumber(tuning.optDouble("left_uv", 0.0d));
                experimentRightUv = formatConfigNumber(tuning.optDouble("right_uv", 0.0d));
                experimentVerticalUv = formatConfigNumber(tuning.optDouble("vertical_uv", 0.0d));
                experimentContentScale = formatConfigNumber(tuning.optDouble("content_scale", 1.60d));
            }
        }
        if (TextUtils.isEmpty(experimentTargetPackage)) {
            experimentTargetPackage = DEFAULT_EXPERIMENT_TARGET_PACKAGE;
        }
        experimentDraftLoaded = true;
    }

    private void loadExperimentS108Defaults() {
        experimentStrength = "0";
        experimentGlobalUv = "0";
        experimentLeftUv = "0";
        experimentRightUv = "0";
        experimentVerticalUv = "0";
        experimentSymmetricUv = "";
        experimentContentScale = "1.60";
        experimentDraftLoaded = true;
    }

    private JSONObject experimentControlStatus(JSONObject status) {
        return status != null ? status.optJSONObject("experimentControl") : null;
    }

    private boolean isShellHelperConnected(JSONObject status) {
        JSONObject shellHelper = status != null ? status.optJSONObject("shellHelper") : null;
        return shellHelper != null && shellHelper.optBoolean("connected", false);
    }

    private String buildExperimentConsoleStatus(JSONObject status) {
        StringBuilder builder = new StringBuilder(900);
        JSONObject control = experimentControlStatus(status);
        JSONObject shellHelper = status != null ? status.optJSONObject("shellHelper") : null;
        if (control == null) {
            builder.append("Experiment control status is not reported yet.\n");
        } else {
            builder.append("enabled       ").append(control.optBoolean("enabled")).append('\n');
            builder.append("mode          ").append(control.optString("mode", "")).append('\n');
            builder.append("desired focus ").append(control.optString("desired_focus", "")).append('\n');
            builder.append("revision      ").append(control.optLong("revision", 0L)).append('\n');
            builder.append("target pkg    ").append(control.optString("target_package", "")).append('\n');
            builder.append("target act    ").append(control.optString("target_activity", "")).append('\n');
            builder.append("guard ms      ").append(control.optInt("launch_guard_timeout_ms", 0)).append('\n');
            builder.append("preview timer ")
                .append(control.optBoolean("launch_guard_preview_timeout_enabled", false) ? "on" : "off")
                .append('\n');
            JSONObject tuning = control.optJSONObject("makepad_tuning");
            if (tuning != null) {
                builder.append("strength      ").append(formatConfigNumber(tuning.optDouble("strength", 0.0d))).append('\n');
                builder.append("global uv     ").append(formatConfigNumber(tuning.optDouble("global_uv", 0.0d))).append('\n');
                builder.append("left uv       ").append(formatConfigNumber(tuning.optDouble("left_uv", 0.0d))).append('\n');
                builder.append("right uv      ").append(formatConfigNumber(tuning.optDouble("right_uv", 0.0d))).append('\n');
                builder.append("vertical uv   ").append(formatConfigNumber(tuning.optDouble("vertical_uv", 0.0d))).append('\n');
                builder.append("content scale ").append(formatConfigNumber(tuning.optDouble("content_scale", 1.60d))).append('\n');
            }

            JSONObject helperStatus = control.optJSONObject("helper_status");
            if (helperStatus != null && helperStatus.length() > 0) {
                builder.append('\n');
                builder.append("guardian      ").append(helperStatus.optString("mode", "")).append('\n');
                builder.append("active side   ").append(helperStatus.optString("active_side", "")).append('\n');
                builder.append("foreground    ").append(helperStatus.optString("foreground_package", "")).append('\n');
                builder.append("last action   ").append(helperStatus.optString("last_action", "")).append('\n');
                builder.append("applied rev   ").append(helperStatus.optLong("applied_revision", 0L)).append('\n');
                String lastError = helperStatus.optString("last_error", "");
                if (lastError.length() > 0) {
                    builder.append("last error    ").append(lastError).append('\n');
                }
            }
        }

        builder.append('\n');
        builder.append("shell helper  ")
            .append(shellHelper != null && shellHelper.optBoolean("connected") ? "connected" : "disconnected")
            .append('\n');
        if (shellHelper != null) {
            builder.append("helper uid    ").append(shellHelper.optString("uid", "")).append('\n');
        }
        builder.append("setprop path  ADB shell-helper required");
        return builder.toString();
    }

    private void addExperimentTuningRow(String label, EditText editText) {
        LinearLayout row = row();
        TextView labelView = textView(13, false, MUTED);
        labelView.setText(label);
        row.addView(labelView, weightedParams(0.42f, 0, 0, 10, 0));
        row.addView(editText, weightedParams(0.58f, 0, 0, 0, 0));
        pagePanel.addView(row, matchWrapParams(0, 0, 0, 7));
    }

    private void applyExperimentControl(
        final String desiredFocusOverride,
        final String modeOverride,
        final boolean resetMakepadTuning,
        final boolean launchTargetAfterApply) {
        startBrokerService(getIntent());
        runBrokerConsoleAction("Applying experiment control", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.configureExperimentControlFromConsole(
                    buildExperimentParams(desiredFocusOverride, modeOverride, resetMakepadTuning));
            }
        });

        if (launchTargetAfterApply) {
            handler.postDelayed(new Runnable() {
                @Override
                public void run() {
                    launchExperimentTarget();
                }
            }, 250L);
        }
    }

    private JSONObject buildExperimentParams(
        String desiredFocusOverride,
        String modeOverride,
        boolean resetMakepadTuning) throws Exception {
        JSONObject params = new JSONObject();
        params.put("target_package", experimentTargetPackage != null ? experimentTargetPackage.trim() : "");
        params.put("target_activity", experimentTargetActivity != null ? experimentTargetActivity.trim() : "");
        params.put("mode", !TextUtils.isEmpty(modeOverride) ? modeOverride : experimentMode);
        params.put("launch_guard_timeout_ms", DEFAULT_EXPERIMENT_LAUNCH_GUARD_TIMEOUT_MS);
        params.put(
            "launch_guard_preview_timeout_enabled",
            DEFAULT_EXPERIMENT_LAUNCH_GUARD_PREVIEW_TIMEOUT_ENABLED);
        if (!TextUtils.isEmpty(desiredFocusOverride)) {
            params.put("desired_focus", desiredFocusOverride);
        }
        params.put("reset_makepad_tuning", resetMakepadTuning);
        if (!resetMakepadTuning) {
            putDoubleParam(params, "strength", experimentStrength);
            putDoubleParam(params, "global_uv", experimentGlobalUv);
            putDoubleParam(params, "left_uv", experimentLeftUv);
            putDoubleParam(params, "right_uv", experimentRightUv);
            putDoubleParam(params, "vertical_uv", experimentVerticalUv);
            putDoubleParam(params, "symmetric_uv", experimentSymmetricUv);
            putDoubleParam(params, "content_scale", experimentContentScale);
        }
        return params;
    }

    private void launchExperimentTarget() {
        String packageName = experimentTargetPackage != null ? experimentTargetPackage.trim() : "";
        String activityName = experimentTargetActivity != null ? experimentTargetActivity.trim() : "";
        if (TextUtils.isEmpty(packageName)) {
            showLaunchToast("Target package is required");
            return;
        }

        Intent intent = null;
        if (!TextUtils.isEmpty(activityName)) {
            intent = new Intent(Intent.ACTION_MAIN);
            intent.addCategory(Intent.CATEGORY_LAUNCHER);
            String className = activityName.startsWith(".") ? packageName + activityName : activityName;
            intent.setClassName(packageName, className);
        } else {
            intent = getPackageManager().getLaunchIntentForPackage(packageName);
        }

        if (intent == null) {
            showLaunchToast("No launch intent for " + packageName);
            return;
        }

        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED);
        try {
            startActivity(intent);
            Log.i(BrokerService.TAG, "Experiment target launched " + packageName + "/" + activityName);
        } catch (ActivityNotFoundException | SecurityException ex) {
            showLaunchToast("Target launch failed: " + ex.getMessage());
            Log.w(BrokerService.TAG, "Experiment target launch failed: " + ex.getMessage());
        }
    }

    private void openSystemSettings(String label, String action) {
        if (TextUtils.isEmpty(action)) {
            showLaunchToast("Missing settings action");
            return;
        }

        Intent intent = new Intent(action);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        try {
            startActivity(intent);
            showLaunchToast(label + " requested");
            Log.i(BrokerService.TAG, "System settings shortcut requested: " + action);
        } catch (ActivityNotFoundException | SecurityException ex) {
            showLaunchToast(label + " unavailable: " + ex.getMessage());
            Log.w(BrokerService.TAG, "System settings shortcut failed for " + action + ": " + ex.getMessage());
        }
    }

    private void openBrokerAppDetails() {
        Intent intent = new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS);
        intent.setData(Uri.fromParts("package", getPackageName(), null));
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        try {
            startActivity(intent);
            showLaunchToast("App info requested");
            Log.i(BrokerService.TAG, "Broker app info shortcut requested");
        } catch (ActivityNotFoundException | SecurityException ex) {
            showLaunchToast("App info unavailable: " + ex.getMessage());
            Log.w(BrokerService.TAG, "Broker app info shortcut failed: " + ex.getMessage());
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
            builder.append("pmd stream    ").append(polarPmd.optString("active_pmd_stream", "acc")).append('\n');
            builder.append("scan reports  ").append(polarPmd.optLong("scan_report_count", 0L)).append('\n');
            builder.append("ignored scan  ").append(polarPmd.optLong("ignored_scan_report_count", 0L)).append('\n');
            builder.append("acc frames    ").append(polarPmd.optLong("acc_frame_count", 0L)).append('\n');
            builder.append("acc samples   ").append(polarPmd.optLong("acc_sample_count", 0L)).append('\n');
            builder.append("ecg frames    ").append(polarPmd.optLong("ecg_frame_count", 0L)).append('\n');
            builder.append("ecg samples   ").append(polarPmd.optLong("ecg_sample_count", 0L)).append('\n');
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

    private void renderDiagnosticsPage(final JSONObject status) {
        final int previousScrollY = pageScroll != null ? pageScroll.getScrollY() : 0;
        pagePanel.removeAllViews();

        JSONObject watchdog = status != null ? status.optJSONObject("deviceWatchdog") : null;
        long currentIntervalMs = watchdog != null
            ? watchdog.optLong("interval_ms", DEFAULT_DEVICE_WATCHDOG_INTERVAL_MS)
            : DEFAULT_DEVICE_WATCHDOG_INTERVAL_MS;
        if (currentIntervalMs <= 0L) {
            currentIntervalMs = DEFAULT_DEVICE_WATCHDOG_INTERVAL_MS;
        }

        addSectionTitle("DEVICE WATCHDOG");

        TextView watchdogText = textView(14, false, TEXT);
        watchdogText.setTypeface(Typeface.MONOSPACE);
        watchdogText.setLineSpacing(0f, 1.08f);
        watchdogText.setText(buildDeviceWatchdogStatus(status));
        pagePanel.addView(watchdogText, matchWrapParams(0, 0, 0, 12));

        final EditText intervalEdit = editText("Sample interval ms", Long.toString(currentIntervalMs));
        intervalEdit.setInputType(InputType.TYPE_CLASS_NUMBER);
        pagePanel.addView(intervalEdit, matchWrapParams(0, 0, 0, 10));

        LinearLayout firstRow = row();
        Button startButton = actionButton("Start");
        startButton.setTextColor(Color.rgb(7, 24, 18));
        startButton.setBackground(panelBackground(ACCENT_STRONG, 12, ACCENT_STRONG));
        startButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                startDeviceWatchdogFromConsole(parseDeviceWatchdogIntervalMs(intervalEdit.getText().toString()), false);
            }
        });
        firstRow.addView(startButton);

        Button wakeLockButton = actionButton("Start + Wake Lock");
        wakeLockButton.setTextColor(WARN);
        wakeLockButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                startDeviceWatchdogFromConsole(parseDeviceWatchdogIntervalMs(intervalEdit.getText().toString()), true);
            }
        });
        firstRow.addView(wakeLockButton, wrapParams(10, 0, 0, 0));

        Button stopButton = actionButton("Stop");
        stopButton.setTextColor(WARN);
        stopButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                stopDeviceWatchdogFromConsole();
            }
        });
        firstRow.addView(stopButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(firstRow, matchWrapParams(0, 0, 0, 10));

        LinearLayout secondRow = row();
        Button markerButton = actionButton("Mark");
        markerButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                markDeviceWatchdogFromConsole();
            }
        });
        secondRow.addView(markerButton);

        Button refreshButton = actionButton("Refresh");
        refreshButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                refreshDeviceWatchdogFromConsole();
            }
        });
        secondRow.addView(refreshButton, wrapParams(10, 0, 0, 0));
        pagePanel.addView(secondRow, matchWrapParams(0, 0, 0, 14));

        addSectionTitle("BROKER DIAGNOSTICS");
        TextView diagnosticsText = textView(14, false, TEXT);
        diagnosticsText.setTypeface(Typeface.MONOSPACE);
        diagnosticsText.setLineSpacing(0f, 1.08f);
        diagnosticsText.setText(buildDiagnostics(status));
        pagePanel.addView(diagnosticsText, matchWrapParams(0, 0, 0, 0));

        if (pageScroll != null) {
            pageScroll.post(new Runnable() {
                @Override
                public void run() {
                    pageScroll.setScrollY(previousScrollY);
                }
            });
        }
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

    private void refreshDeviceWatchdogFromConsole() {
        runBrokerConsoleAction("Refreshing watchdog status", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.getDeviceWatchdogStatusFromConsole();
            }
        });
    }

    private void startDeviceWatchdogFromConsole(final long intervalMs, final boolean wakeLock) {
        startBrokerService(getIntent());
        runBrokerConsoleAction(wakeLock ? "Starting watchdog with wake lock" : "Starting watchdog", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.startDeviceWatchdogFromConsole(intervalMs, wakeLock);
            }
        });
    }

    private void stopDeviceWatchdogFromConsole() {
        startBrokerService(getIntent());
        runBrokerConsoleAction("Stopping watchdog", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.stopDeviceWatchdogFromConsole();
            }
        });
    }

    private void markDeviceWatchdogFromConsole() {
        startBrokerService(getIntent());
        runBrokerConsoleAction("Recording watchdog marker", new BrokerConsoleAction() {
            @Override
            public JSONObject run() throws Exception {
                return BrokerService.markDeviceWatchdogFromConsole("console_marker");
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
        if ("Experiment".equals(currentPage) && !isEditingText()) {
            return EXPERIMENT_STATUS_REFRESH_MS;
        }
        if ("System".equals(currentPage) && !isEditingText()) {
            return EXPERIMENT_STATUS_REFRESH_MS;
        }
        if ("Diagnostics".equals(currentPage) && !isEditingText()) {
            return EXPERIMENT_STATUS_REFRESH_MS;
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

    private String buildSystemSurfaceStatus(JSONObject status) {
        StringBuilder builder = new StringBuilder(1200);
        builder.append("SYSTEM SURFACE\n\n");

        JSONObject kiosk = status != null ? status.optJSONObject("rustyKiosk") : null;
        if (kiosk == null) {
            builder.append("Rusty Kiosk status is not reported yet.\n\n");
        } else {
            builder.append("phase         ").append(kiosk.optString("phase", "unknown")).append('\n');
            builder.append("surface       ").append(kiosk.optString("surface_intent", "unknown")).append('\n');
            builder.append("panel visible ").append(kiosk.optBoolean("broker_panel_visible")).append('\n');
            builder.append("active panel  ").append(kiosk.optString("active_panel", "")).append('\n');
            builder.append("foreground    ").append(kiosk.optString("foreground_package", "")).append('\n');
            builder.append("activity      ").append(kiosk.optString("foreground_activity", "")).append('\n');
            builder.append("meta menu     ").append(kiosk.optBoolean("meta_menu_active")).append('\n');
            builder.append("clock epoch   ").append(kiosk.optString("clock_epoch_id", "")).append('\n');
            JSONArray limitations = kiosk.optJSONArray("limitations");
            if (limitations != null && limitations.length() > 0) {
                builder.append("limits\n");
                for (int i = 0; i < limitations.length(); i++) {
                    builder.append("- ").append(limitations.optString(i)).append('\n');
                }
            }
            builder.append('\n');
        }

        JSONObject shellHelper = status != null ? status.optJSONObject("shellHelper") : null;
        builder.append("HELPER\n");
        builder.append("connected     ").append(shellHelper != null && shellHelper.optBoolean("connected")).append('\n');
        if (shellHelper != null) {
            builder.append("version       ").append(shellHelper.optString("helper_version", "")).append('\n');
            builder.append("uid           ").append(shellHelper.optString("uid", "")).append('\n');
            builder.append("requires adb  ").append(shellHelper.optBoolean("requires_adb_authorization")).append('\n');
            builder.append("broker shell  ").append(shellHelper.optBoolean("normal_broker_apk_is_shell")).append('\n');
        }

        JSONObject control = experimentControlStatus(status);
        builder.append('\n');
        builder.append("FOCUS CONTROL\n");
        if (control == null) {
            builder.append("state         unavailable\n");
        } else {
            builder.append("enabled       ").append(control.optBoolean("enabled")).append('\n');
            builder.append("mode          ").append(control.optString("mode", "")).append('\n');
            builder.append("desired focus ").append(control.optString("desired_focus", "")).append('\n');
            builder.append("target        ").append(control.optString("target_component", "")).append('\n');
            JSONObject helperStatus = control.optJSONObject("helper_status");
            if (helperStatus != null && helperStatus.length() > 0) {
                builder.append("helper fg     ").append(helperStatus.optString("foreground_package", "")).append('\n');
                builder.append("last action   ").append(helperStatus.optString("last_action", "")).append('\n');
            }
        }

        JSONObject watchdog = status != null ? status.optJSONObject("deviceWatchdog") : null;
        builder.append('\n');
        builder.append("WATCHDOG\n");
        if (watchdog == null) {
            builder.append("state         unavailable\n");
        } else {
            builder.append("running       ").append(watchdog.optBoolean("running")).append('\n');
            builder.append("samples       ").append(watchdog.optLong("sample_count", 0L)).append('\n');
            builder.append("wake request  ").append(watchdog.optBoolean("wake_lock_requested")).append('\n');
            builder.append("wake held     ").append(watchdog.optBoolean("wake_lock_held")).append('\n');
        }

        JSONObject clock = status != null ? status.optJSONObject("clock") : null;
        builder.append('\n');
        builder.append("CLOCK\n");
        builder.append("health        ").append(clock != null ? clock.optString("health", "unknown") : "unknown").append('\n');
        builder.append("epoch         ").append(clock != null ? clock.optString("clock_epoch_id", "") : "").append('\n');

        builder.append('\n');
        builder.append("BOUNDARY\n");
        builder.append("- Broker console is a normal 2D app panel, not Android Home.\n");
        builder.append("- Home/Menu, Guardian, permission prompts, and system overlays are system-owned.\n");
        builder.append("- Shell-helper focus recovery is reactive; it does not preempt physical Home/Menu.\n");
        builder.append("- Settings shortcuts use public Android intents and may be ignored or remapped by the device build.");
        return builder.toString();
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

        JSONObject kiosk = status.optJSONObject("rustyKiosk");
        if (kiosk != null) {
            builder.append("RUSTY KIOSK\n");
            builder.append("Phase         ").append(kiosk.optString("phase", "unknown")).append('\n');
            builder.append("Intent        ").append(kiosk.optString("surface_intent", "unknown")).append('\n');
            builder.append("Broker panel  ").append(kiosk.optBoolean("broker_panel_visible")).append('\n');
            builder.append("Immersive     ").append(kiosk.optBoolean("immersive_home_visible")).append('\n');
            builder.append("Shell helper  ").append(kiosk.optBoolean("shell_helper_connected")).append('\n');
            builder.append("Focus guard   ").append(kiosk.optBoolean("focus_guardian_active")).append('\n');
            builder.append("Panel         ").append(kiosk.optString("active_panel", "")).append('\n');
            builder.append("Foreground    ").append(kiosk.optString("foreground_package", "")).append('\n');
            builder.append('\n');
        }

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
        JSONObject deviceWatchdog = status.optJSONObject("deviceWatchdog");
        JSONObject clock = status.optJSONObject("clock");
        builder.append("TRANSPORTS\n");
        builder.append("Clock        ").append(clock != null ? clock.optString("health", "unknown") : "unknown").append('\n');
        builder.append("LSL           ").append(lsl != null && lsl.optBoolean("enabled") ? "enabled" : "fallback/logcat").append('\n');
        builder.append("OSC ingress   ").append(transportEnabled(osc != null ? osc.optJSONObject("ingress") : null)).append('\n');
        builder.append("OSC egress    ").append(transportEnabled(osc != null ? osc.optJSONObject("egress") : null)).append('\n');
        builder.append("Camera meta   ").append(cameraProvider != null ? cameraProvider.optString("state", "unknown") : "unknown").append('\n');
        builder.append("Shell helper  ").append(shellHelper != null && shellHelper.optBoolean("connected") ? "connected" : "disconnected").append('\n');
        builder.append("Polar PMD     ").append(polarPmd != null ? polarPmd.optString("state", "unknown") : "not reported").append('\n');
        builder.append("Breath        ").append(breathAssessment != null ? breathAssessment.optString("state", "unknown") : "not reported").append('\n');
        builder.append("Video lab     ").append(videoLab != null ? videoLab.optString("state", "unknown") : "not reported").append('\n');
        builder.append("Watchdog      ").append(deviceWatchdog != null && deviceWatchdog.optBoolean("running") ? "running" : "idle").append('\n');
        builder.append('\n');
        builder.append("Use Polar to start broker-owned Polar PMD before launching a target XR app.\n");
        builder.append("Use Return to XR App to close this console while the broker service keeps running.");
        return builder.toString();
    }

    private long parseDeviceWatchdogIntervalMs(String raw) {
        if (raw == null || raw.trim().length() == 0) {
            return DEFAULT_DEVICE_WATCHDOG_INTERVAL_MS;
        }

        try {
            long parsed = Long.parseLong(raw.trim());
            return parsed > 0L ? parsed : DEFAULT_DEVICE_WATCHDOG_INTERVAL_MS;
        } catch (NumberFormatException ex) {
            return DEFAULT_DEVICE_WATCHDOG_INTERVAL_MS;
        }
    }

    private String buildClock(JSONObject status) {
        StringBuilder builder = new StringBuilder(1000);
        builder.append("CLOCK\n\n");

        JSONObject clock = status.optJSONObject("clock");
        if (clock == null) {
            builder.append("Clock service status is not reported yet.");
            return builder.toString();
        }

        JSONObject snapshot = clock.optJSONObject("snapshot");
        builder.append("Health       ").append(clock.optString("health", "unknown")).append('\n');
        builder.append("Clock id     ").append(clock.optString("clock_id", "")).append('\n');
        builder.append("Epoch        ").append(clock.optString("clock_epoch_id", "")).append('\n');
        builder.append("Primary      ").append(clock.optString("primary_domain", "")).append('\n');
        if (snapshot != null) {
            long elapsedNs = snapshot.optLong("android_elapsed_realtime_ns", 0L);
            long unixNs = snapshot.optLong("android_realtime_unix_ns", 0L);
            builder.append("Sequence     ").append(snapshot.optLong("sequence_number", 0L)).append('\n');
            builder.append("Elapsed      ").append(elapsedNs).append(" ns (")
                .append(formatNsAsMs(elapsedNs)).append(")\n");
            builder.append("Unix label   ").append(unixNs > 0L ? Long.toString(unixNs) : "not reported").append(" ns\n");
            builder.append("Uncertainty  ").append(snapshot.optLong("read_uncertainty_ns", 0L)).append(" ns\n");
            builder.append("Wall jumps   ").append(snapshot.optLong("wall_clock_adjustment_counter", 0L)).append('\n');
        }

        JSONObject openXr = clock.optJSONObject("openxr_comparison");
        if (openXr != null) {
            builder.append('\n');
            builder.append("OPENXR COMPARISON\n");
            builder.append("State        ").append(openXr.optString("state", "")).append('\n');
            builder.append("Available    ").append(openXr.optBoolean("available")).append('\n');
            builder.append("Reason       ").append(openXr.optString("reason", "")).append('\n');
        }

        JSONArray correlations = clock.optJSONArray("correlations");
        if (correlations != null && correlations.length() > 0) {
            builder.append('\n');
            builder.append("CORRELATIONS\n");
            for (int i = 0; i < correlations.length(); i++) {
                JSONObject correlation = correlations.optJSONObject(i);
                if (correlation == null) {
                    continue;
                }
                builder.append("- ").append(correlation.optString("correlation_id", "unknown")).append('\n');
                builder.append("  ")
                    .append(correlation.optString("source_domain", ""))
                    .append(" -> ")
                    .append(correlation.optString("target_domain", ""))
                    .append(" quality=")
                    .append(correlation.optString("quality", ""))
                    .append(" samples=")
                    .append(correlation.optInt("sample_count", 0))
                    .append('\n');
                builder.append("  offset_ns=").append(correlation.optLong("offset_ns", 0L))
                    .append(" uncertainty_ns=")
                    .append(correlation.optLong("uncertainty_ns", 0L))
                    .append('\n');
            }
        }

        JSONArray domains = clock.optJSONArray("domains");
        if (domains != null && domains.length() > 0) {
            builder.append('\n');
            builder.append("DOMAINS\n");
            for (int i = 0; i < domains.length(); i++) {
                JSONObject domain = domains.optJSONObject(i);
                if (domain == null) {
                    continue;
                }
                builder.append(domain.optBoolean("available") ? "[available] " : "[offline]   ");
                builder.append(domain.optString("id", "unknown")).append(" - ")
                    .append(domain.optString("role", "")).append('\n');
            }
        }

        builder.append('\n');
        builder.append("HTTP\n");
        builder.append("GET /clock/status\n");
        builder.append("GET /clock/now\n");
        builder.append("GET /clock/domains\n");
        builder.append("GET /clock/correlations\n");
        builder.append("GET /clock/compare/openxr\n");
        builder.append("GET /clock/health\n");
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

    private String buildDeviceWatchdogStatus(JSONObject status) {
        StringBuilder builder = new StringBuilder(700);
        JSONObject watchdog = status != null ? status.optJSONObject("deviceWatchdog") : null;
        if (watchdog == null) {
            builder.append("state         unavailable\n");
            return builder.toString();
        }

        builder.append("running       ").append(watchdog.optBoolean("running")).append('\n');
        builder.append("run id        ").append(watchdog.optString("run_id", "")).append('\n');
        builder.append("interval ms   ").append(watchdog.optLong("interval_ms", 0L)).append('\n');
        builder.append("uptime ms     ").append(watchdog.optLong("uptime_ms", 0L)).append('\n');
        builder.append("samples       ").append(watchdog.optLong("sample_count", 0L)).append('\n');
        builder.append("wake request  ").append(watchdog.optBoolean("wake_lock_requested")).append('\n');
        builder.append("wake held     ").append(watchdog.optBoolean("wake_lock_held")).append('\n');
        builder.append("max bytes     ").append(watchdog.optLong("max_log_bytes", 0L)).append('\n');
        builder.append("log path      ").append(watchdog.optString("log_path", "")).append('\n');
        String stopReason = watchdog.optString("stop_reason", "");
        if (stopReason.length() > 0) {
            builder.append("stop reason   ").append(stopReason).append('\n');
        }
        String lastError = watchdog.optString("last_error", "");
        if (lastError.length() > 0) {
            builder.append("last error    ").append(lastError).append('\n');
        }

        JSONObject sample = watchdog.optJSONObject("latest_sample");
        JSONObject power = sample != null ? sample.optJSONObject("power") : null;
        JSONObject battery = sample != null ? sample.optJSONObject("battery") : null;
        JSONObject network = sample != null ? sample.optJSONObject("network") : null;
        if (power != null || battery != null || network != null) {
            builder.append('\n').append("latest sample\n");
        }
        if (power != null) {
            builder.append("interactive   ").append(power.optBoolean("interactive")).append('\n');
            builder.append("idle mode     ").append(power.optBoolean("device_idle_mode")).append('\n');
            builder.append("power save    ").append(power.optBoolean("power_save_mode")).append('\n');
            if (power.has("thermal_status_label")) {
                builder.append("thermal       ").append(power.optString("thermal_status_label", "")).append('\n');
            }
        }
        if (battery != null) {
            if (battery.has("percent")) {
                builder.append("battery       ")
                    .append(String.format(Locale.ROOT, "%.1f", battery.optDouble("percent", 0.0d)))
                    .append("%\n");
            }
            builder.append("plugged       ").append(battery.optInt("plugged", -1)).append('\n');
        }
        if (network != null) {
            builder.append("network       ").append(network.optBoolean("connected")).append('\n');
            builder.append("validated     ").append(network.optBoolean("validated")).append('\n');
            if (network.has("wifi_rssi_dbm")) {
                builder.append("wifi rssi     ").append(network.optInt("wifi_rssi_dbm", 0)).append(" dBm\n");
            }
        }

        JSONArray limitations = watchdog.optJSONArray("limitations");
        if (limitations != null && limitations.length() > 0) {
            builder.append('\n').append("limitations\n");
            for (int i = 0; i < limitations.length(); i++) {
                builder.append("- ").append(limitations.optString(i)).append('\n');
            }
        }
        return builder.toString();
    }

    private String buildDiagnostics(JSONObject status) {
        StringBuilder builder = new StringBuilder(900);
        builder.append("DIAGNOSTICS\n\n");
        builder.append("Logcat tag    ").append(BrokerService.TAG).append('\n');
        builder.append("HTTP status   http://127.0.0.1:8765/status\n");
        builder.append("Clock now     http://127.0.0.1:8765/clock/now\n");
        builder.append("Clock health  http://127.0.0.1:8765/clock/health\n");
        builder.append("Kiosk status  http://127.0.0.1:8765/kiosk/status\n");
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
            builder.append("pmd stream    ").append(polarPmd.optString("active_pmd_stream", "acc")).append('\n');
            builder.append("acc frames    ").append(polarPmd.optLong("acc_frame_count", 0L)).append('\n');
            builder.append("acc samples   ").append(polarPmd.optLong("acc_sample_count", 0L)).append('\n');
            builder.append("ecg frames    ").append(polarPmd.optLong("ecg_frame_count", 0L)).append('\n');
            builder.append("ecg samples   ").append(polarPmd.optLong("ecg_sample_count", 0L)).append('\n');
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

    private String formatNsAsMs(long nanos) {
        return String.format(Locale.ROOT, "%.3f ms", nanos / 1_000_000.0d);
    }

    private static String safeJson(String value) {
        if (value == null) {
            return "";
        }
        return value.replace("\\", "\\\\").replace("\"", "\\\"");
    }
}
