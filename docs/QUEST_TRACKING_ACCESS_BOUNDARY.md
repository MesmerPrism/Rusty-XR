# Quest Tracking Access Boundary

This note records the public utility boundary for headset and controller
tracking on stock retail Quest devices. It is meant for diagnostics, broker,
companion, and public example design. It is not a promise about private system
services or rooted/development firmware.

## Short Version

Use the active XR/3D app for fused headset and controller pose. The sampler can
live in that app's OpenXR shell, or in a plugin/module loaded by that app, but
it should run in the same OpenXR session as the visible immersive experience.
`adb shell`, `dumpsys`, ADB-launched shell helpers, and a separate broker app
are useful diagnostics and operator tools, but they are not a supported public
backdoor to Meta's fused tracking stream on a retail headset.

The practical design is:

```text
active XR / 3D app
  -> owns OpenXR session, frame loop, reference spaces, and action focus
  -> samples VIEW / eye views / controller action spaces
  -> pose, velocity, validity flags, tracked flags, timestamps
  -> app-owned UDP/TCP/WebSocket/log stream to broker if needed

broker / 2D console / background service
  -> receives samples
  -> stores, analyzes, forwards, visualizes
  -> owns BLE / OSC / LSL / launcher / diagnostics where appropriate
```

Do not design this as:

```text
background broker app
  -> owns OpenXR session while another immersive app is active
  -> samples HMD/controller tracking
  -> forwards tracking into the active XR app
```

That second model conflicts with OpenXR focus/session ownership and Quest
immersive app lifecycle expectations.

## Access Matrix

| Data | Supported public path | Available from `adb shell` directly? | Notes |
| --- | --- | --- | --- |
| Headset view pose | OpenXR `xrLocateViews`, or locate `XR_REFERENCE_SPACE_TYPE_VIEW` relative to `LOCAL`, `LOCAL_FLOOR`, or `STAGE` | No | `VIEW` tracks the primary viewer/view-origin basis. `xrLocateViews` is the normal per-frame render path. |
| Headset linear/angular velocity | OpenXR `xrLocateSpace` with `XrSpaceVelocity`, or OpenXR 1.1 batched `xrLocateSpaces` with `XrSpaceVelocities` | No | Check velocity-valid flags before consuming values. |
| Headset Android accelerometer/gyro/rotation-vector sensors | Android `SensorManager`, only if the runtime exposes those sensors to the app | Maybe inspectable, not a stable pose stream | These are Android-level device-frame sensors, not fused OpenXR 6DoF tracking or predicted compositor poses. |
| Headset acceleration from tracked pose | Derive from sampled velocity or pose history | No | Core OpenXR defines pose and velocity structures, not linear/angular acceleration fields. |
| Controller position/orientation | OpenXR pose actions and action spaces, usually grip and aim pose paths | No | Bind `/user/hand/left/input/grip/pose`, `/user/hand/right/input/grip/pose`, and/or aim pose paths, then locate the created action spaces. |
| Controller linear/angular velocity | OpenXR `xrLocateSpace` with `XrSpaceVelocity` | No | Runtime-fused value when valid, not raw controller IMU. |
| Controller raw accelerometer/gyro | No stable public Android/ADB API | No | Meta documents controller accelerometers/gyroscopes as part of controller technology, but does not expose them as normal Android sensor streams for third-party apps. |
| Tracking validity/state | OpenXR location valid/tracked flags, velocity valid flags, action active state, and session state | No | Public flags are available. Meta-internal confidence, SLAM state, calibration details, and private tracking diagnostics are not part of this public contract. |
| Tracking while another XR app is foregrounded | Generally unsupported for a background app | No | OpenXR input focus is runtime-managed. An unfocused visible session should not expect active XR input actions. |

## Active App Versus Broker

Rusty XR's public shell split follows the same rule. The Android/OpenXR
renderer shell owns the OpenXR loader/runtime integration, Android lifecycle,
swapchains, frame loop, platform timing, renderer backend, and target-device
validation. Core crates and experiment logic receive plain snapshots such as
poses, eye views, frame timing, camera metadata, runtime config, and counters.

The Quest broker example is intentionally a sidecar service and 2D console. It
owns messages, status, diagnostics, launcher flows, BLE/Polar sources, OSC,
optional LSL forwarding, and generic stream-event routing. It does not own the
active XR app's OpenXR frame timing, eye views, controller action spaces,
texture import/decode, shaders, or layer submission.

That includes the broker's `xr:controller_pose` stream and
`breath_assessment.submit_controller_pose` command. Those are ingress routes
for samples published by a thin XR adapter in the active app:

```json
{
  "controller": "right",
  "connected": true,
  "tracked": true,
  "sample_time_elapsed_ns": 123456789000,
  "position_m": [0.12, 1.08, -0.34],
  "orientation_xyzw": [0.0, 0.0, 0.0, 1.0]
}
```

The broker can then store, derive breath/motion diagnostics, forward to
LSL/OSC/WebSocket, or visualize the sample. The broker is not harvesting that
controller pose from OpenXR in the background.

Recommended flow:

```text
1. Start broker service / 2D console.
2. Broker requests the ordinary Android permissions it owns, such as BLE.
3. Broker, Companion, launcher, or ADB starts the target immersive XR activity.
4. Target XR app creates the OpenXR session and reaches visible/focused state.
5. Target XR app samples HMD/controller pose and velocity.
6. Target XR app sends samples to the broker over localhost.
7. Broker forwards, logs, analyzes, or visualizes samples.
```

## Quest Activity And Launch Shape

On Quest, an OpenXR app is not just an arbitrary background service using a
library. A reliable custom shell uses the current foreground Android Activity
when initializing the Android OpenXR loader and creating the instance. The
public Quest bring-up note records the practical requirement to pass the active
`AndroidApp::vm_as_ptr()` and `AndroidApp::activity_as_ptr()`, then wait until
the app is resumed, focused, and has a native window before creating the
OpenXR/Vulkan session.

The expected success ladder is an immersive activity progressing through
OpenXR states such as `READY`, `SYNCHRONIZED`, `VISIBLE`, and `FOCUSED`. A
foreground-service notification or a broker 2D panel is not equivalent to an
immersive OpenXR activity with XR input focus.

Keep the immersive activity separate from a normal launcher entrypoint when
needed. A public custom shell can expose a normal launcher alias or small
launcher Activity for app-library visibility, while the OpenXR Activity remains
focused on VR lifecycle requirements. For direct development launch, target the
immersive activity:

```powershell
adb shell am start -n <package>/<immersive-xr-activity> --ez rustyxr.camera false
```

Use public example package names only in public docs and examples. Downstream
apps should replace the component with their own immersive XR activity.

## ADB And Android Sensors

ADB is a command-line/debugging bridge. It can install and debug apps, open a
Unix shell on the device, run Activity Manager commands, forward ports, collect
logs, and call diagnostic services such as `dumpsys`.

Useful diagnostics include:

```powershell
adb -s <serial> shell id
adb -s <serial> shell dumpsys -l
adb -s <serial> shell dumpsys sensorservice
adb -s <serial> shell dumpsys input
```

Treat `dumpsys sensorservice` as an inspection tool, not as a durable high-rate
tracking API. If Android exposes accelerometer, gyroscope, magnetometer, or
rotation-vector sensors to an app, consume them through `SensorManager` in an
app that declares the right permissions and lifecycle behavior. Android limits
motion-sensor sampling for apps targeting Android 12 or later unless the app
declares `HIGH_SAMPLING_RATE_SENSORS`, and Android 9 or later restricts
continuous motion sensors for background apps.

Developer Mode and ADB authorization do not make a normal installed APK run as
root or as Android `shell`. ADB's daemon drops to the `shell` UID on normal
secure/user builds unless the build is debuggable and explicitly allows root.
Design public Quest utilities so enhanced shell behavior is optional and
operator-started.

## OpenXR Sampling Pattern

For a headset sample:

1. Create a base reference space such as `LOCAL`, `LOCAL_FLOOR`, or `STAGE`.
2. Use `xrLocateViews` each frame for render eye poses and FOV.
3. If a single viewer-origin pose is useful for logging or control, create a
   `VIEW` reference space and locate it relative to the base space at the same
   `XrTime`.
4. Chain `XrSpaceVelocity` to `XrSpaceLocation` when velocity is needed.
5. Record location flags, velocity flags, and the requested `XrTime` with every
   sample.

For controller samples:

```cpp
// Conceptual flow only.
create pose action: XR_ACTION_TYPE_POSE_INPUT
bind:
  /user/hand/left/input/grip/pose
  /user/hand/right/input/grip/pose
  /user/hand/left/input/aim/pose
  /user/hand/right/input/aim/pose
create XrSpace for each pose action with xrCreateActionSpace

each frame:
  xrSyncActions(...)
  XrSpaceVelocity velocity{XR_TYPE_SPACE_VELOCITY};
  XrSpaceLocation location{XR_TYPE_SPACE_LOCATION, &velocity};
  xrLocateSpace(handSpace, baseSpace, sampleTime, &location);
```

Use `XR_SPACE_LOCATION_POSITION_VALID_BIT`,
`XR_SPACE_LOCATION_ORIENTATION_VALID_BIT`,
`XR_SPACE_LOCATION_POSITION_TRACKED_BIT`,
`XR_SPACE_LOCATION_ORIENTATION_TRACKED_BIT`,
`XR_SPACE_VELOCITY_LINEAR_VALID_BIT`, and
`XR_SPACE_VELOCITY_ANGULAR_VALID_BIT` to decide which values are meaningful.

## Acceleration And Telemetry

Core OpenXR does not provide linear or angular acceleration. Derive it only when
the sample cadence and timestamps are good enough:

```text
linear_accel ~= (linear_velocity_now - linear_velocity_prev) / dt
angular_accel ~= (angular_velocity_now - angular_velocity_prev) / dt
```

OpenXR locations may be historical or predicted depending on the requested
time. For render-frame telemetry, the requested time is often the predicted
display time from `xrWaitFrame`. Do not mix predicted OpenXR samples with raw
Android sensor timestamps without explicitly recording both clock domains and
the conversion policy.

If another process needs tracking values, build the tracking sampler into the
active XR app or a plugin loaded by that app, and stream the sanitized
telemetry out over an app-owned channel such as UDP, TCP, WebSocket, or a
broker command stream. An ADB shell helper can start the app, forward ports,
and collect logs; it should not be documented as the owner of fused tracking.

## Implementation Pattern

Add a small tracking bridge to the active app:

```text
OpenXR frame loop:
  predicted_display_time = xrWaitFrame(...)
  locate HMD/view space at predicted_display_time
  locate controller grip/aim action spaces at predicted_display_time
  read XrSpaceVelocity if valid
  compute acceleration from velocity deltas if needed
  publish sample to broker
```

For Unity, this can be a MonoBehaviour plus native/plugin bridge. For Unreal,
use an XR subsystem or plugin. For Rust, Makepad, or another native shell, keep
it as a thin OpenXR adapter in the app shell. The important property is not
where the code is organized in the source tree; it is that the code executes in
the active XR app's OpenXR session and uses that app's reference spaces,
predicted display time, tracking flags, and input focus.

## References

- OpenXR `xrLocateSpace`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/xrLocateSpace.html>
- OpenXR `XrSpaceVelocity`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrSpaceVelocity.html>
- OpenXR `XR_REFERENCE_SPACE_TYPE_VIEW`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XR_REFERENCE_SPACE_TYPE_VIEW.html>
- OpenXR `XrSessionState` focus rules:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrSessionState.html>
- Meta OpenXR core concepts:
  <https://developers.meta.com/horizon/documentation/native/android/mobile-openxr-core-concepts/>
- Meta OpenXR actions and controller pose bindings:
  <https://developers.meta.com/horizon/documentation/native/android/mobile-openxr-actions-actionsets-bindings/>
- Meta controller technology overview:
  <https://developers.meta.com/horizon/design/controllers-technology/>
- Rusty XR Android / Quest shell responsibility split:
  [ANDROID_QUEST_APK_BUILDING.md](ANDROID_QUEST_APK_BUILDING.md)
- Rusty XR broker example:
  [examples/quest-broker-apk/README.md](../examples/quest-broker-apk/README.md)
- Android sensor overview:
  <https://developer.android.com/develop/sensors-and-location/sensors/sensors_overview>
- Android Debug Bridge:
  <https://developer.android.com/tools/adb>
- Android `dumpsys`:
  <https://developer.android.com/tools/dumpsys>
- ADB daemon root/shell behavior:
  <https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/daemon/main.cpp>
