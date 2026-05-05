package com.example.rustyxr.broker;

import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.ActivityInfo;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.text.TextUtils;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

final class LauncherStore {
    private static final String PREFS_NAME = "rusty_xr_broker_launcher";
    private static final String PREFS_STATE = "state_json";
    private static final String DEFAULT_LIST_ID = "default";
    private static final int MAX_SEARCH_RESULTS = 40;

    private final Context context;
    private final SharedPreferences prefs;
    private final List<AppList> lists = new ArrayList<>();

    LauncherStore(Context context) {
        this.context = context.getApplicationContext();
        this.prefs = this.context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
        load();
    }

    synchronized List<AppList> lists() {
        return new ArrayList<>(lists);
    }

    synchronized AppList selectedList(String preferredId) {
        AppList selected = findList(preferredId);
        if (selected != null) {
            return selected.copy();
        }

        return lists.get(0).copy();
    }

    synchronized String selectedListIdOrDefault(String preferredId) {
        AppList selected = findList(preferredId);
        return selected != null ? selected.id : lists.get(0).id;
    }

    synchronized AppList createList(String requestedName) {
        String name = cleanName(requestedName);
        if (TextUtils.isEmpty(name)) {
            name = "List " + (lists.size() + 1);
        }

        AppList list = new AppList("list-" + System.currentTimeMillis(), name);
        lists.add(list);
        save();
        return list.copy();
    }

    synchronized AppList renameList(String id, String requestedName) {
        AppList list = findList(id);
        if (list == null) {
            list = lists.get(0);
        }

        String name = cleanName(requestedName);
        if (!TextUtils.isEmpty(name)) {
            list.name = name;
            save();
        }
        return list.copy();
    }

    synchronized String deleteList(String id) {
        if (DEFAULT_LIST_ID.equals(id) || lists.size() <= 1) {
            return selectedListIdOrDefault(id);
        }

        for (int i = 0; i < lists.size(); i++) {
            if (lists.get(i).id.equals(id)) {
                lists.remove(i);
                save();
                return lists.get(Math.max(0, i - 1)).id;
            }
        }
        return selectedListIdOrDefault(id);
    }

    synchronized boolean addApp(String listId, AppTarget target) {
        AppList list = findList(listId);
        if (list == null || target == null || !target.isLaunchable()) {
            return false;
        }

        String key = target.key();
        for (AppTarget existing : list.apps) {
            if (existing.key().equals(key)) {
                return false;
            }
        }

        list.apps.add(target.copy());
        save();
        return true;
    }

    synchronized boolean removeApp(String listId, String key) {
        AppList list = findList(listId);
        if (list == null || TextUtils.isEmpty(key)) {
            return false;
        }

        for (int i = 0; i < list.apps.size(); i++) {
            if (list.apps.get(i).key().equals(key)) {
                list.apps.remove(i);
                save();
                return true;
            }
        }
        return false;
    }

    List<AppTarget> searchLaunchableApps(String query) {
        String normalizedQuery = normalize(query);
        List<AppTarget> apps = discoverLaunchableApps();
        if (TextUtils.isEmpty(normalizedQuery)) {
            return trim(apps, MAX_SEARCH_RESULTS);
        }

        List<AppTarget> matches = new ArrayList<>();
        for (AppTarget app : apps) {
            if (normalize(app.label).contains(normalizedQuery) ||
                normalize(app.packageName).contains(normalizedQuery) ||
                normalize(app.activityName).contains(normalizedQuery)) {
                matches.add(app);
            }
        }
        return trim(matches, MAX_SEARCH_RESULTS);
    }

    Intent buildLaunchIntent(AppTarget target) {
        if (target == null || TextUtils.isEmpty(target.packageName)) {
            return null;
        }

        Intent intent;
        if (!TextUtils.isEmpty(target.activityName)) {
            intent = new Intent(Intent.ACTION_MAIN);
            if ("leanback".equals(target.source)) {
                intent.addCategory(Intent.CATEGORY_LEANBACK_LAUNCHER);
            } else {
                intent.addCategory(Intent.CATEGORY_LAUNCHER);
            }
            intent.setClassName(target.packageName, target.activityName);
        } else {
            intent = context.getPackageManager().getLaunchIntentForPackage(target.packageName);
        }

        if (intent != null) {
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED);
        }
        return intent;
    }

    private List<AppTarget> discoverLaunchableApps() {
        PackageManager packageManager = context.getPackageManager();
        Map<String, AppTarget> byKey = new LinkedHashMap<>();
        collectLaunchableActivities(packageManager, Intent.CATEGORY_LAUNCHER, "launcher", byKey);
        collectLaunchableActivities(packageManager, Intent.CATEGORY_LEANBACK_LAUNCHER, "leanback", byKey);

        List<AppTarget> apps = new ArrayList<>(byKey.values());
        Collections.sort(apps, new Comparator<AppTarget>() {
            @Override
            public int compare(AppTarget left, AppTarget right) {
                int label = left.label.compareToIgnoreCase(right.label);
                if (label != 0) {
                    return label;
                }
                return left.packageName.compareToIgnoreCase(right.packageName);
            }
        });
        return apps;
    }

    private void collectLaunchableActivities(
        PackageManager packageManager,
        String category,
        String source,
        Map<String, AppTarget> byKey) {
        Intent queryIntent = new Intent(Intent.ACTION_MAIN);
        queryIntent.addCategory(category);
        List<ResolveInfo> resolved = packageManager.queryIntentActivities(queryIntent, 0);
        for (ResolveInfo resolveInfo : resolved) {
            ActivityInfo activityInfo = resolveInfo.activityInfo;
            if (activityInfo == null || TextUtils.isEmpty(activityInfo.packageName) || TextUtils.isEmpty(activityInfo.name)) {
                continue;
            }
            if (!activityInfo.exported && !context.getPackageName().equals(activityInfo.packageName)) {
                continue;
            }

            CharSequence labelValue = resolveInfo.loadLabel(packageManager);
            String label = labelValue != null ? labelValue.toString() : activityInfo.packageName;
            ApplicationInfo applicationInfo = activityInfo.applicationInfo;
            boolean systemApp = applicationInfo != null && (applicationInfo.flags & ApplicationInfo.FLAG_SYSTEM) != 0;
            AppTarget target = new AppTarget(
                label,
                activityInfo.packageName,
                activityInfo.name,
                source,
                systemApp);
            byKey.put(target.key(), target);
        }
    }

    private synchronized void load() {
        lists.clear();
        String raw = prefs.getString(PREFS_STATE, "");
        if (!TextUtils.isEmpty(raw)) {
            try {
                JSONObject state = new JSONObject(raw);
                JSONArray storedLists = state.optJSONArray("lists");
                if (storedLists != null) {
                    for (int i = 0; i < storedLists.length(); i++) {
                        JSONObject item = storedLists.optJSONObject(i);
                        AppList list = AppList.fromJson(item);
                        if (list != null) {
                            lists.add(list);
                        }
                    }
                }
            } catch (Exception ignored) {
                lists.clear();
            }
        }

        if (lists.isEmpty()) {
            lists.add(new AppList(DEFAULT_LIST_ID, "Favorites"));
            save();
        }
    }

    private synchronized void save() {
        try {
            JSONObject state = new JSONObject();
            state.put("schema", "rusty.xr.broker.launcher_lists.v1");
            JSONArray storedLists = new JSONArray();
            for (AppList list : lists) {
                storedLists.put(list.toJson());
            }
            state.put("lists", storedLists);
            prefs.edit().putString(PREFS_STATE, state.toString()).apply();
        } catch (Exception ignored) {
        }
    }

    private AppList findList(String id) {
        for (AppList list : lists) {
            if (list.id.equals(id)) {
                return list;
            }
        }
        return null;
    }

    private static String cleanName(String value) {
        if (value == null) {
            return "";
        }
        return value.trim();
    }

    private static String normalize(String value) {
        if (value == null) {
            return "";
        }
        return value.toLowerCase(Locale.ROOT).trim();
    }

    private static List<AppTarget> trim(List<AppTarget> values, int max) {
        if (values.size() <= max) {
            return values;
        }
        return new ArrayList<>(values.subList(0, max));
    }

    static final class AppList {
        final String id;
        String name;
        final List<AppTarget> apps = new ArrayList<>();

        AppList(String id, String name) {
            this.id = !TextUtils.isEmpty(id) ? id : DEFAULT_LIST_ID;
            this.name = !TextUtils.isEmpty(name) ? name : "Favorites";
        }

        AppList copy() {
            AppList copy = new AppList(id, name);
            for (AppTarget app : apps) {
                copy.apps.add(app.copy());
            }
            return copy;
        }

        JSONObject toJson() throws Exception {
            JSONObject json = new JSONObject();
            json.put("id", id);
            json.put("name", name);
            JSONArray storedApps = new JSONArray();
            for (AppTarget app : apps) {
                storedApps.put(app.toJson());
            }
            json.put("apps", storedApps);
            return json;
        }

        static AppList fromJson(JSONObject json) {
            if (json == null) {
                return null;
            }

            String id = json.optString("id", "");
            String name = json.optString("name", "");
            if (TextUtils.isEmpty(id)) {
                return null;
            }

            AppList list = new AppList(id, name);
            JSONArray storedApps = json.optJSONArray("apps");
            if (storedApps != null) {
                for (int i = 0; i < storedApps.length(); i++) {
                    AppTarget app = AppTarget.fromJson(storedApps.optJSONObject(i));
                    if (app != null && app.isLaunchable()) {
                        list.apps.add(app);
                    }
                }
            }
            return list;
        }
    }

    static final class AppTarget {
        final String label;
        final String packageName;
        final String activityName;
        final String source;
        final boolean systemApp;

        AppTarget(String label, String packageName, String activityName, String source, boolean systemApp) {
            this.label = !TextUtils.isEmpty(label) ? label : packageName;
            this.packageName = packageName;
            this.activityName = activityName;
            this.source = !TextUtils.isEmpty(source) ? source : "launcher";
            this.systemApp = systemApp;
        }

        boolean isLaunchable() {
            return !TextUtils.isEmpty(packageName);
        }

        String key() {
            return packageName + "/" + (activityName != null ? activityName : "");
        }

        AppTarget copy() {
            return new AppTarget(label, packageName, activityName, source, systemApp);
        }

        JSONObject toJson() throws Exception {
            JSONObject json = new JSONObject();
            json.put("label", label);
            json.put("package", packageName);
            json.put("activity", activityName);
            json.put("source", source);
            json.put("system_app", systemApp);
            return json;
        }

        static AppTarget fromJson(JSONObject json) {
            if (json == null) {
                return null;
            }
            return new AppTarget(
                json.optString("label", ""),
                json.optString("package", ""),
                json.optString("activity", ""),
                json.optString("source", "launcher"),
                json.optBoolean("system_app", false));
        }
    }
}
