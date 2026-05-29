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

## Launch Boundary

The on-device watchdog can be started from the broker console or broker command
API after the broker APK is installed and the broker app/service has been
launched. This is the useful no-PC-attached mode: start the broker from the
headset, start the watchdog from the Diagnostics page, then continue the
session without an attached host.

It does not replace ADB shell helpers. Android documents ADB as a client/server
tool where the client runs on the development machine and `adbd` runs on the
device; commands are issued from the workstation-side client:
[Android Debug Bridge](https://developer.android.com/tools/adb). Wireless
debugging still requires pairing/connection from a workstation-side ADB client.
So a normal broker APK should not claim that it can spawn or become an Android
`shell` user by itself. Shell-helper watchdogs remain externally launched
developer-mode tools.

## Sleep And Reboot Boundary

The watchdog is best-effort while the app process and foreground service are
alive. It cannot run while the headset is powered off. After a full reboot, the
broker app must be launched again by the user, an app-owned launch path, or
developer tooling before the watchdog can resume.

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
