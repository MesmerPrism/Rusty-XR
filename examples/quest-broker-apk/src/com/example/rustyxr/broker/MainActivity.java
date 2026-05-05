package com.example.rustyxr.broker;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.text.TextUtils;
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
    private static final String[] PAGES = { "Dashboard", "Launcher", "Streams", "Commands", "Diagnostics" };
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
    private LinearLayout pagePanel;
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
