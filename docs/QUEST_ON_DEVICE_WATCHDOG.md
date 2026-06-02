# Quest On-Device Watchdog

The Quest broker APK includes a public, app-owned device watchdog for sessions
where a development PC cannot stay attached for the whole run. It is a normal
Android foreground-service feature inside the broker app, not an ADB shell
process and not a privileged system service.

## What It Does

- Samples low-rate device health from APIs available to the broker app:
  power/idle flags, battery state, active network capabilities, memory, storage,
  thermal status when available, and watchdog log metadata.
- Writes retention-limited JSONL samples under the broker app's external-files
  directory.
- Publishes `device_watchdog.status` in broker status, capabilities, and the
  stream registry as module `diagnostics.device_watchdog` with kind
  `diagnostic`.
- Exposes headset-local console controls on the Diagnostics page.
- Exposes WebSocket commands:
  `device_watchdog.get_status`, `device_watchdog.start`,
  `device_watchdog.stop`, and `device_watchdog.mark`.

The default mode does not change power, proximity, tracking, or shell state. A
caller can request an optional partial wake lock for lab runs, but that should
be treated as a bounded diagnostic tool because Android documents wake locks as
battery-sensitive and recommends holding them for as short a time as possible:
[Set a wake lock](https://developer.android.com/develop/background-work/background-tasks/awake/wakelock/set).
A wake lock is not a Quest mounted/proximity control. It can keep the app's
CPU-side work alive within Android policy, but it cannot write virtual
proximity, force a mounted headset state, enable tracking, or prove camera
readiness. Off-face long-running validation that needs a forced
mounted/proximity state still belongs to an explicit authorized ADB or provider
helper.

## Launch Boundary

The on-device watchdog can be started from the broker console or broker command
API after the broker APK is installed and the broker app/service has been
launched. This is the useful no-PC-attached mode: start the broker from the
headset, start the watchdog from the Diagnostics page, then continue the
session without an attached host.

Treat visible or foreground startup as the normal path. Modern Android can
block foreground-service starts from background receivers, including some boot
or alarm-style flows, so public tests should not assume that a sideloaded APK
can silently restart the watchdog after reboot. A launcher, broker console,
notification, or other user-visible app state is the reliable activation model
for this public slice.

Lab debug helpers may still use `BOOT_COMPLETED` as visible launch evidence:
after the package has been manually launched once and remains unstopped, a
receiver can hold a bounded app wake lock, wait for app-visible readiness, start
the broker, and start an XR activity through normal Android launch APIs. That
does not make the app a reboot-persistent shell helper. Readiness gates should
use signals the app can actually observe, keep loopback network probes off the
broadcast main thread, configure cleartext loopback deliberately, and avoid
treating app-UID process probes for other packages as authoritative.

It does not replace ADB shell helpers. Android documents ADB as a client/server
tool where the client runs on the development machine and `adbd` runs on the
device; commands are issued from the workstation-side client:
[Android Debug Bridge](https://developer.android.com/tools/adb). Wireless
debugging still requires pairing/connection from a workstation-side ADB client.
So a normal broker APK should not claim that it can spawn or become an Android
`shell` user by itself. Shell-helper watchdogs remain externally launched
developer-mode tools.

Likewise, a pre-granted normal helper should not be positioned as a reboot
Wi-Fi ADB restarter. In lab validation, a helper could receive
`BOOT_COMPLETED` and write app-local status, but settings writes did not reopen
classic ADB TCP after reboot and app-UID adbd property changes were blocked.
That is enough for visible diagnostics, not enough for shell lease recovery.

If an external host re-enables Wi-Fi ADB after reboot, an on-device Linux or
Termux ADB client may connect and issue shell commands for that already
authorized transport. That is an externally leased shell path, not an app-owned
bootstrap.

## Sleep And Reboot Boundary

The watchdog is best-effort while the app process and foreground service are
alive. It cannot run while the headset is powered off. After a full reboot, the
broker app must be launched again by the user, an app-owned launch path, or
developer tooling before the watchdog can resume.

If no external ADB or provider helper re-establishes the shell-side wake and
proximity guard after reboot, an off-face headset remains subject to normal
platform timeout and sleep behavior. Treat app wake-lock evidence,
`sys.hmt.mounted`, power wakefulness, tracking, and camera frame flow as
separate signals.

Watchdog status is low-rate device-health evidence. It can show that the app
process, foreground service, wake lock, battery, network, and idle-state
samplers are alive, but it is not proof that hand/controller tracking, camera
capture, high-rate media, or an OpenXR session are ready. Camera and media tests
should keep separate readiness signals for those paths.

The broker targets Android foreground-service rules. Android 12 and newer limit
foreground-service starts from background apps, and Android 15 adds additional
restrictions for `dataSync` foreground services and `BOOT_COMPLETED` launches:
[Changes to foreground services](https://developer.android.com/develop/background-work/services/fgs/changes)
and [Foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types).
For that reason this public example does not add boot autostart behavior.

Sleep/deep-idle behavior is platform-owned. Android Doze can suspend network
access, defer jobs/alarms, and ignore wake locks while the device is idle:
[Optimize for Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby).
The watchdog records `isInteractive`, `isDeviceIdleMode`,
`isIgnoringBatteryOptimizations`, and wake-lock state so tests can see what the
platform actually did. It should not be used as proof that high-rate media,
Bluetooth, or relay paths will survive headset sleep without a dedicated test.

## Command Shape

```json
{
  "type": "command",
  "schema": "rusty.xr.broker.command.v1",
  "request_id": "watchdog-001",
  "command": "device_watchdog.start",
  "params": {
    "interval_ms": 30000,
    "wake_lock": false,
    "max_log_bytes": 8388608,
    "run_id": "lab-watchdog"
  }
}
```

The status payload uses schema `rusty.xr.device_watchdog.status.v1`; log samples
use `rusty.xr.device_watchdog.sample.v1`. The JSON command path carries only
control and low-rate status metadata. High-rate media and sensor payloads still
belong on binary or stream-specific transports.
