# Quest App Launching And ADB Shell Helpers

This note explains the difference between a normal headset launcher app and an
ADB-launched shell helper. It is intended for public Rusty XR examples and
downstream app shells that need to organize, launch, or diagnose side-loaded
Quest APKs.

## Short Version

A normal installed Android or Quest 2D app can launch another installed app
when that target exposes a normal front-door launch Activity. This is the right
baseline for a headset-local side-loaded app organizer.

An ADB shell helper is different. It runs as Android `shell` only when an
external authorized ADB host starts it, commonly by pushing a dex jar or APK to
`/data/local/tmp` and launching it with `app_process`. A normal installed APK
cannot promote itself to Android `shell` and cannot start that helper by
itself.

The helper boundary is also a tracking boundary. ADB shell identity can improve
package, launch, log, `dumpsys`, and port-forward diagnostics, but it does not
give a helper a supported public stream of fused headset or controller pose.
Fused HMD/controller pose should be sampled by the foreground OpenXR app and
exported over an app-owned channel when another process needs it. See
[Quest Tracking Access Boundary](QUEST_TRACKING_ACCESS_BOUNDARY.md).

The practical product shape is therefore:

```text
normal headset launcher APK
  -> organizes apps, icons, favorites, tags, recent launches, and profiles
  -> launches known front-door activities through Android PackageManager
  -> works without an ADB session

optional ADB shell helper
  -> started externally by a PC or phone companion through authorized ADB
  -> provides enhanced package/launch/diagnostic operations
  -> disappears when the helper process, ADB workflow, or headset reboot ends
```

## Normal App Launching

For packages that expose a launchable Activity, a normal app can use Android's
`PackageManager` and `startActivity` APIs. Typical launch resolution starts
with `getLaunchIntentForPackage(packageName)`. On Android API 33 and newer,
`getLaunchIntentSenderForPackage(packageName)` is also useful because it
returns an `IntentSender` for the same front-door launch concept.

Conceptually:

```kotlin
val intent = packageManager.getLaunchIntentForPackage(packageName)
if (intent != null) {
    intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    startActivity(intent)
}
```

This works for the common side-loaded-app case where the APK has a launcher
entrypoint. It is not guaranteed for every installed package. A normal app
cannot launch a target that has no front-door Activity, hides all relevant
activities as non-exported/internal implementation details, or requires
private app-specific launch state that the target does not document.

Package visibility is a separate concern. On modern Android, listing or
resolving other installed packages can require manifest visibility declarations
such as `<queries>`, or a design that stores known package IDs in a catalog.
That affects discovery and organization. It does not turn a normal app into a
shell process.

Good normal-launcher responsibilities:

- store a curated app library and known package IDs
- store named app lists or profiles directly on the headset
- show labels, icons, tags, notes, favorites, and recent launches
- use `ACTION_MAIN` plus `CATEGORY_LAUNCHER` discovery where visible
- also consider `CATEGORY_LEANBACK_LAUNCHER` for TV-style packages
- launch packages through `PackageManager` front-door launch APIs
- support documented extras only for apps that intentionally expose them
- show clear "not launchable from normal app mode" states

Avoid:

- accepting arbitrary remote package/activity launch commands without local
  user confirmation
- assuming a package is launchable just because it is installed
- treating package visibility as a permission bypass
- claiming normal app mode can force-stop, install, or inspect every package

## ADB Shell Helper Launching

Android's ADB shell can use Activity Manager commands such as `am start` to
start an Activity by intent or explicit component. Tools such as scrcpy use a
related but more structured pattern: an external client pushes a small Android
server artifact to `/data/local/tmp`, then starts it with `adb shell
CLASSPATH=... app_process ...`. Because ADB started that process, it runs under
the Android `shell` UID.

That identity is useful for development and operator tooling. An ADB-launched
helper can add:

- broader package enumeration
- launch-target resolution
- explicit `am start` launches
- force-stop before launch
- foreground/process checks after launch
- logcat and dumpsys diagnostics
- ADB-mediated install/update workflows
- shell-only media or device probes where the platform allows them

These are Developer Mode capabilities. They require the user to enable and
authorize ADB debugging, and they require an external ADB host such as a PC,
phone companion, or developer terminal.

Treat helper state as session-scoped. After headset reboot, restart the helper
from the external host and verify the run-specific readiness gates again.
Power-state signals such as wakefulness, display power, or virtual proximity
are not enough to prove OpenXR tracking, camera capture, or media-stream
readiness.

The headset app cannot start this helper by itself:

```text
installed headset APK
  -> app UID
  -> cannot become shell
  -> cannot directly execute adb shell app_process

external ADB host
  -> authorized by the user
  -> starts helper as Android shell
  -> helper can talk back to app/broker over an explicit channel
```

## Recommended Rusty XR Boundary

Use a two-tier boundary in public examples and downstream apps:

| Layer | Owns | Does not own |
| --- | --- | --- |
| Headset 2D launcher app | app library, normal package launch, user-facing organization, safe launch profiles | shell identity, ADB bootstrap, silent install/update |
| Companion or phone operator | ADB pairing, helper push/start/stop, APK install/update, diagnostics export | headset-local app organization UX |
| ADB shell helper | shell-backed package operations and diagnostics for the active session | permanent privileges, store-style app permissions, OpenXR session ownership, fused tracking ownership |
| Rusty XR core | public contracts, catalog shapes, launch/status diagnostics | private package IDs, signing, app-specific launch scripts |

Design the normal launcher to be useful without enhanced mode. Treat the shell
helper as optional acceleration and diagnostics, not as the base requirement
for opening side-loaded apps.

The public broker APK example includes this normal-mode pattern in its 2D
console `Launcher` page. It persists named lists in app-local storage, searches
visible launcher/leanback activities, and launches selected entries through
`PackageManager`/Activity intents. It intentionally does not install,
force-stop, or shell-launch packages.

Rusty XR also exposes developer-home menu contracts for this split:
`HomePanelDescriptor`, `LauncherEntry`, `SettingsShortcutDescriptor`,
`HomeSessionState`, and `FocusRecoveryEvent`. Those contracts let a broker,
immersive home shell, or companion describe panels and recovery state without
claiming shell privileges in normal mode. See
[Quest Developer Home Menu Contracts](QUEST_DEVELOPER_HOME_MENU.md).

## Distribution Boundary

Normal launcher behavior and shell-helper behavior should also stay split at
distribution time:

- A Store-style or release-channel 2D app should remain useful without ADB,
  use minimal permissions, avoid broad cleartext/debug manifest defaults, and
  not present shell-helper actions as normal app capabilities.
- SideQuest, GitHub, lab, and enterprise developer builds may document ADB
  shell helpers, focus/proximity diagnostics, `dumpsys`, `screenrecord`, and
  helper push/start flows because those channels expect Developer Mode and an
  external authorized ADB host.
- No public build should claim that a browser download or ordinary installed
  APK can enable Developer Mode, authorize ADB, become Android `shell`, or turn
  on Wi-Fi ADB from a locked retail headset.

For the full distribution and ADB bootstrap boundary, see
[Quest Distribution And ADB Boundary](QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md).

## References

- Android `PackageManager.getLaunchIntentForPackage` and
  `getLaunchIntentSenderForPackage`:
  <https://developer.android.com/reference/android/content/pm/PackageManager.html#getLaunchIntentForPackage(java.lang.String)>
- Android package visibility:
  <https://developer.android.com/training/package-visibility/automatic>
- Android ADB Activity Manager commands:
  <https://developer.android.com/tools/adb#am>
- scrcpy developer documentation for the `app_process` server model:
  <https://scrcpyapp.org/guides/develop/>
