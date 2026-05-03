# BLE, LSL, And Polar H10 Pipeline

Rusty XR includes public contracts for routing BLE bio-sensor data into XR
applications and Lab Streaming Layer (LSL) workflows. The core repository owns
the data models, decoders, stream schemas, and documentation. App shells own
Android Bluetooth adapters, permission prompts, native `liblsl` linkage,
network policy, package identity, signing, rendering behavior, and device
validation.

This document is intentionally limited to general BLE, LSL, and Polar H10 data
handling. It does not include app-specific simulation logic, private visual tuning,
project-specific stream names, signing material, generated captures, or release
payloads.

## Public Crate Roles

- `rusty-xr-ble`: framework-neutral BLE UUIDs, scan results, GATT
  characteristic paths, CCCD notification modes, GATT operation descriptors, and
  Android Bluetooth permission plans.
- `rusty-xr-lsl`: pure LSL descriptors, stream roles, channel schemas, endpoint
  status, staleness checks, roundtrip probes, telemetry samples, and normalized
  biofeedback readings.
- `rusty-xr-polar`: Polar H10 GATT UUIDs, PMD command builders, standard
  Heart Rate Measurement decoding, ECG PMD frame decoding, uncompressed ACC PMD
  frame decoding, and public LSL schemas for Polar streams.

These crates are pure Rust models. They do not link Android, Makepad, OpenXR,
Meta SDKs, Polar SDKs, or native `liblsl`.

## Data Flow Options

### Direct APK BLE Path

Use this when the headset app connects directly to a Polar H10 or compatible
BLE heart-rate sensor.

```text
Polar H10
  -> Android BLE/GATT adapter in the app shell
  -> rusty-xr-ble operation model
  -> rusty-xr-polar HR/RR, ECG, and ACC decoders
  -> app simulation/render state
  -> optional rusty-xr-lsl outlet to Windows
```

The APK shell owns scanning, connecting, service discovery, MTU negotiation,
notification subscription, Android runtime permissions, reconnect policy, and
foreground UI. Rusty XR owns the data shapes and deterministic decoding helpers.

### Windows/LSL Biofeedback Path

Use this when a Windows tool or another host already publishes a normalized
biofeedback stream.

```text
Polar H10 or other source
  -> Windows acquisition/analysis tool
  -> LSL outlet: name HRV_Biofeedback, type HRV
  -> Android/Quest app shell with native liblsl inlet
  -> rusty-xr-lsl LslBiofeedbackReading
  -> app simulation/render state
```

The public `HRV_Biofeedback` / `HRV` stream convention follows the public
PolarH10 workflow documentation. Rusty XR treats it as a normalized one-channel
`float32` value in the `0..1` range.

### APK To Windows LSL Path

Use this when the headset app is the acquisition or telemetry source and Windows
is the observer.

```text
Android/Quest app shell
  -> direct BLE/GATT Polar data or app telemetry
  -> rusty-xr-polar / rusty-xr-lsl schemas
  -> native liblsl outlet
  -> Windows LSL tools, recorders, dashboards, or analysis notebooks
```

LSL discovery usually requires both devices to be on the same reachable network
with multicast discovery and firewall rules permitting the LSL process. This is
an app/deployment concern, not a core crate dependency.

### Broker Bio Diagnostic Path

Use this when validating broker consumers before a native Bluetooth adapter or a
real sensor is available.

```text
Windows companion diagnostic command
  -> Polar-compatible GATT payload bytes
  -> broker publish_stream_event command
  -> subscribed Quest or desktop clients
  -> optional LSL schema mapping in the consumer
```

The companion diagnostic publisher emits three public stream ids:

- `bio:polar_hr_rr`: standard Heart Rate Measurement notifications from the
  Bluetooth Heart Rate service.
- `bio:polar_ecg`: Polar PMD Data notifications carrying uncompressed ECG
  frames.
- `bio:polar_acc`: Polar PMD Data notifications carrying uncompressed
  accelerometer frames.

Each event includes the source service UUID, characteristic UUID, notification
mode, raw payload bytes as base64, decoded summary fields, and the intended LSL
stream type. This is a protocol-level broker diagnostic. It does not make the
Windows host advertise as a Bluetooth peripheral, and it does not replace real
BLE adapter validation for scan, connect, MTU, permission, reconnect, or radio
behavior.

## Polar H10 GATT Model

The public Polar helpers expose these UUIDs:

- Heart Rate Service: `0000180d-0000-1000-8000-00805f9b34fb`
- Heart Rate Measurement: `00002a37-0000-1000-8000-00805f9b34fb`
- Battery Service: `0000180f-0000-1000-8000-00805f9b34fb`
- Battery Level: `00002a19-0000-1000-8000-00805f9b34fb`
- Polar PMD Service: `fb005c80-02e7-f387-1cad-8acd2d8df0c8`
- PMD Control Point: `fb005c81-02e7-f387-1cad-8acd2d8df0c8`
- PMD Data: `fb005c82-02e7-f387-1cad-8acd2d8df0c8`
- CCCD descriptor: `00002902-0000-1000-8000-00805f9b34fb`

A typical shell lifecycle is:

1. Scan for a device name or advertised service that matches the intended
   sensor.
2. Connect and discover GATT services.
3. Request a larger MTU when the platform permits it.
4. Enable notifications by writing the CCCD value `0x01 0x00` for notify or
   `0x02 0x00` for indicate.
5. Subscribe to Heart Rate Measurement notifications for BPM and RR intervals.
6. Subscribe to PMD Control Point and PMD Data when ECG or ACC is required.
7. Query PMD settings, start the requested stream, decode frames, then stop and
   disable notifications when done.

## Polar Data Handling

### Heart Rate And RR

`rusty-xr-polar::decode_heart_rate_measurement` decodes the standard BLE Heart
Rate Measurement characteristic. It handles 8-bit and 16-bit BPM fields,
optional energy-expended data, sensor-contact flags, and RR intervals.

RR intervals are encoded by the BLE characteristic in `1/1024 s` units and are
exposed by Rusty XR as milliseconds.

### ECG PMD Frames

The public ECG decoder handles uncompressed PMD ECG frames:

- byte `0`: measurement type, `0x00` for ECG
- bytes `1..=8`: little-endian sensor timestamp in nanoseconds
- byte `9`: frame type, `0x00` for uncompressed ECG
- payload: signed 24-bit little-endian samples in microvolts

The default LSL schema for ECG is one `float32` channel named `microvolts` with
nominal sample rate `130 Hz`.

### ACC PMD Frames

The public ACC decoder currently handles uncompressed PMD ACC frames:

- byte `0`: measurement type, `0x02` for ACC
- bytes `1..=8`: little-endian sensor timestamp in nanoseconds
- byte `9`: frame type, `0x01` for uncompressed ACC
- payload: repeated `x`, `y`, `z` little-endian signed 16-bit samples in
  milli-g

The default LSL schema for ACC is three `float32` channels named `x_mg`,
`y_mg`, and `z_mg` with nominal sample rate `200 Hz`.

Compressed ACC delta decoding is intentionally not part of the first public
Rusty XR helper surface. Add it only with independent synthetic tests and clear
source attribution.

### PMD Control Commands

`rusty-xr-polar` builds these PMD control-point commands:

- `0x01 <measurement>`: get settings
- `0x02 <measurement> ...settings`: start stream
- `0x03 <measurement>`: stop stream

Public measurement types included now:

- ECG: `0x00`
- ACC: `0x02`

Public setting types included now:

- sample rate: `0x00`
- resolution: `0x01`
- range: `0x02`
- channels: `0x04`

## LSL Stream Schemas

Recommended public stream shapes:

| Role | Name | Type | Channels | Units | Rate |
| --- | --- | --- | --- | --- | --- |
| Normalized biofeedback | `HRV_Biofeedback` | `HRV` | `value01` | normalized | irregular |
| Polar HR/RR | app-chosen | `rusty.xr.polar.heart_rate` | `bpm`, `last_rr_ms` | bpm/ms | irregular |
| Polar ECG | app-chosen | `rusty.xr.polar.ecg` | `microvolts` | uV | `130 Hz` |
| Polar ACC | app-chosen | `rusty.xr.polar.acc` | `x_mg`, `y_mg`, `z_mg` | mg | `200 Hz` |

Do not reuse private project stream names in public examples. Use app-chosen
names and the public stream types above.

## Android Bluetooth Permissions

For Android 12/API 31 and higher, apps that scan for BLE devices need
`android.permission.BLUETOOTH_SCAN`, and apps that connect or communicate with
Bluetooth devices need `android.permission.BLUETOOTH_CONNECT`. If the app
advertises itself over Bluetooth, it also needs
`android.permission.BLUETOOTH_ADVERTISE`. These are runtime permissions and must
be requested by the foreground app to spawn the headset or system "Nearby
devices" popup.

If the app can strongly assert that BLE scan results are not used to derive
physical location, declare `BLUETOOTH_SCAN` with `android:usesPermissionFlags`
set to `neverForLocation` and set legacy location permissions to
`android:maxSdkVersion="30"`. If scan results are used to derive physical
location, declare and request `android.permission.ACCESS_FINE_LOCATION` too.

For Android 11/API 30 and lower, BLE communication uses legacy
`android.permission.BLUETOOTH` and often `android.permission.BLUETOOTH_ADMIN`,
and BLE scanning requires runtime location permission such as
`android.permission.ACCESS_FINE_LOCATION`.

## What Can Be Granted Where

- Manifest-only / install-time: normal permissions such as `INTERNET` and
  `ACCESS_NETWORK_STATE` are declared in the APK and do not need a runtime
  headset popup.
- Runtime Bluetooth permissions: `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`,
  `BLUETOOTH_ADVERTISE`, and legacy scan-location permission must be declared in
  the manifest and requested from a foreground Activity.
- Development launcher grants: ADB can often grant declared runtime permissions
  with `adb shell pm grant <package> <permission>` or `adb install -g <apk>`.
  This is useful for local testing but is not production UX.
- User/headset-only consent: the production app should request runtime
  Bluetooth permissions when the user starts scan/connect. The resulting system
  popup must be accepted in the headset or Android UI.

Rusty XR crates only model which permissions are required. The APK shell must
perform the manifest declaration, runtime checks, request calls, denial handling,
and feature-specific retry UI.

## Attribution

Rusty XR's Polar H10 helpers are independent Rust data models informed by the
public PolarH10 project and public protocol references. The PolarH10 project is
available at <https://mesmerprism.github.io/PolarH10/> and its protocol pages
start at <https://mesmerprism.github.io/PolarH10/reference/protocol/overview.html>.

PolarH10 is an unofficial independent project and is not affiliated with or
endorsed by Polar Electro. Polar and Polar H10 are trademarks of their
respective owners.

Protocol background should be cross-checked against the Polar BLE SDK
open-source repository and technical documentation:
<https://github.com/polarofficial/polar-ble-sdk>. The Polar BLE SDK is MIT
licensed. Rusty XR does not reproduce proprietary Polar documentation.

Android Bluetooth permission behavior should be checked against the official
Android documentation:
<https://developer.android.com/guide/topics/connectivity/bluetooth/permissions>.
