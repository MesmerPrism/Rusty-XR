# Rusty XR Quest Broker APK

Public proof-of-concept for a minimal Quest sidecar broker. The broker is a
separate Android APK/service that stays focused on messages, status,
diagnostics, latency samples, LSL forwarding when the native dependency is
supplied, OSC transport probes, and generic stream-event publishing for
diagnostic adapters. It also exposes the first camera-projection metadata
provider shape so XR clients can discover a projection profile while keeping
actual rendering inside the active XR app.

This is not the final runtime architecture and does not own production camera
buffers, depth textures, OpenXR frame timing, MediaProjection, media session
policy, or rendering. The broker reports those boundaries explicitly so clients
do not mistake diagnostic transport and metadata support for cross-app layer
injection.

For the bounded app-camera H.264 side channel, the stream-start command returns
best-effort selected Camera2 projection metadata alongside the binary endpoint
when the broker has camera permission. A consuming XR app can use that metadata
to tag decoded hardware buffers with intrinsics and lens pose while still
owning decode, Vulkan import, projection, and OpenXR layer submission itself.

## Endpoints

- HTTP status: `http://127.0.0.1:8765/status`
- Broker clock: `http://127.0.0.1:8765/clock/now`
- Rusty Kiosk control-plane status: `http://127.0.0.1:8765/kiosk/status`
- WebSocket samples/events: `ws://127.0.0.1:8765/rustyxr/v1/events`
- LSL stream when native LSL is packaged: `rusty_xr_broker_latency`
- OSC latency egress when enabled: `/rusty-xr/broker/latency`
- OSC control ingress when enabled: `/rusty-xr/drive/radius`
- Bio diagnostic stream IDs: `bio:polar_hr_rr`, `bio:polar_ecg`,
  `bio:polar_acc`, `bio:breath`
- XR diagnostic input stream ID: `xr:controller_pose`
- Camera provider stream IDs: `camera_provider.status`,
  `camera_provider.projection_profile`, `camera_provider.visual_acceptance`
- Shell helper status stream ID: `shell_helper.status`
- Rusty Kiosk control-plane stream ID: `kiosk:control_plane`
- Clock stream IDs: `clock:sample`, `clock:health`, `clock:correlation`,
  `clock:openxr_frame`
- Video-lab metric stream ID: `video_lab.metric_sample`
- Video-lab encoded stream manifest ID: `video_lab.encoded_stream_manifest`
- Video-lab encoded sample metadata ID: `video_lab.encoded_sample_metadata`

The broker control socket binds to loopback by default. For LAN control
experiments, start the broker with `rustyxr.brokerLanEnabled=true`; optionally
set `rustyxr.brokerBindHost` to override the default `0.0.0.0` LAN bind.

`xr:controller_pose` is an input stream for samples published by the active
XR app or a plugin/module loaded by that app. It does not mean the broker owns
an OpenXR session or reads controller tracking while another immersive app is
foregrounded. The foreground XR client should sample OpenXR pose/velocity in
its own frame loop and submit the resulting public payloads to the broker. See
[Quest Tracking Access Boundary](../../docs/QUEST_TRACKING_ACCESS_BOUNDARY.md).

The headset console is a normal Horizon OS 2D Android app. Its launch activity
declares a default panel size in the manifest so system panel controls such as
resize, reposition, and focused/theater presentation have explicit starting
dimensions. Those panel controls are owned by Horizon OS; the broker keeps its
foreground service and localhost API running independently of the current panel
presentation.

On supported Horizon OS builds, a useful workflow is to open the broker console
once from the sideloaded/Unknown Sources app view, use the panel's three-dot
system menu, and enable the system anchoring option for that 2D app panel. When
the broker console is anchored, a target XR app can be launched from the
broker's Launcher page and later closed while the broker console remains
available as a headset-local navigation and sensor-control panel. Anchoring is
system-owned UI state; the broker only keeps its service and localhost API
running.

The status payload reports broker uptime, accepted sample counts, active
capabilities, stream descriptors, command counters, LSL availability, OSC
ingress/egress settings, clock state, and a `rustyKiosk` control-plane snapshot.
That snapshot reports whether this is still the normal 2D broker panel
(`BrokerPanel2d` / `BrokerPanelWithShellHelper`) or a future app-owned immersive
home. WebSocket clients can send legacy `status_request`, `hello`, and
`latency_sample` JSON messages. They can also use the first versioned command
envelope for status, capability, stream, subscription, runtime OSC ingress
configuration, generic stream publication, and console requests. Camera-provider
metadata, shell-helper status, and Rusty Kiosk status commands are also
available:

- `clock.status`
- `clock.now`
- `clock.domains`
- `clock.correlations`
- `clock.health`
- `clock.compare_openxr`
- `clock.sync_probe`
- `kiosk.get_status`
- `camera_provider.get_status`
- `camera_provider.get_projection_profile`
- `camera_provider.run_app_camera_probe`
- `camera_provider.start_app_camera_luma_stream`
- `camera_provider.start_app_camera_h264_stream`
- `camera_provider.run_app_camera_h264_decode_probe`
- `media.start_synthetic_h264_stream`
- `media.start_h264_tcp_proxy`
- `media.run_h264_tcp_proxy_probe`
- `camera_provider.set_source_eye_mapping`
- `camera_provider.set_texture_transform`
- `camera_provider.record_visual_acceptance`
- `shell_helper.get_status`
- `shell_helper.report_status`
- `polar_pmd.get_status`
- `polar_pmd.start`
- `polar_pmd.stop`
- `breath_assessment.get_status`
- `breath_assessment.configure`
- `breath_assessment.reset`
- `breath_assessment.submit_controller_pose`
- `set_polar_breath_params`
- `polar_breath_calibrate_begin`
- `polar_breath_calibrate_reset`
- `video_lab.get_status`
- `video_lab.register_encoded_stream_manifest`
- `video_lab.record_encoded_sample_metadata`
- `video_lab.record_metric_sample`

```json
{
  "type": "command",
  "schema": "rusty.xr.broker.command.v1",
  "request_id": "req-001",
  "client_id": "quest-client",
  "command": "subscribe",
  "params": {
    "stream": "osc:/rusty-xr/drive/radius"
  }
}
```

Command replies use `rusty.xr.broker.command_ack.v1`. Subscribed clients
receive generic `stream_event` messages such as:

```json
{
  "type": "stream_event",
  "schema": "rusty.xr.broker.stream_event.v1",
  "stream": "osc:/rusty-xr/drive/radius",
  "sequence_id": 42,
  "clock_stamp": {
    "schema": "rusty.xr.clock.stamp.v1",
    "clock_id": "broker-clock",
    "canonical_domain": "ElapsedRealtime"
  },
  "payload": {
    "value01": 0.75
  }
}
```

The broker clock uses Android elapsed realtime for canonical ordering and wall
clock only for labels. See
[Broker Clock And Timebase](../../docs/BROKER_CLOCK_AND_TIMEBASE.md) for the
HTTP endpoints, WebSocket commands, stream stamp contract, sync-probe shape,
and OpenXR comparison boundary.

The legacy `osc_drive` broadcast remains in place for compatibility while
newer clients move to explicit subscriptions. The broker acknowledges accepted
latency samples and logs transport diagnostics to logcat with the
`RustyXrBroker` tag.

`configure_osc_ingress` can enable or replace the UDP OSC ingress listener
while the broker service is already running. `publish_stream_event` accepts a
stream id, sequence id, and JSON payload, then broadcasts the payload to clients
subscribed to that stream. The Companion `broker bio-simulate` command uses
that generic path to publish Polar-compatible standard Heart Rate Measurement
and Polar PMD ECG/ACC payloads. This broker path carries GATT-shaped diagnostic
payloads; it is not a Bluetooth peripheral advertiser.

The broker also derives diagnostic breath assessment events on `bio:breath`.
When `publish_stream_event` receives `bio:polar_acc`, the broker reads either
the Polar PMD ACC `payload_base64` frame or simpler JSON `samples_mg`,
`acc_mg`, `acc_g`, or x/y/z fields, then emits an assessment payload with
`schema=rusty.xr.bio.breath.v1`, `source=polar_acc`, `volume01`, `state`,
`state01`, `tracking01`, calibration flags, quality, and broker timing fields.
The same output stream is used for controller motion. Thin XR adapters can
run inside the active XR app and either publish controller samples on
`xr:controller_pose` through `publish_stream_event`, or use
`breath_assessment.submit_controller_pose` for a request/ack latency path:

```json
{
  "type": "command",
  "schema": "rusty.xr.broker.command.v1",
  "request_id": "breath-001",
  "command": "breath_assessment.submit_controller_pose",
  "params": {
    "sequence_id": 42,
    "controller": "right",
    "connected": true,
    "tracked": true,
    "sample_time_unix_ns": 1777900000000000000,
    "sample_time_elapsed_ns": 123456789000,
    "position_m": [0.12, 1.08, -0.34],
    "orientation_xyzw": [0.0, 0.0, 0.0, 1.0]
  }
}
```

`breath_assessment.configure` can adjust source-specific calibration frame
counts, movement thresholds, smoothing, quantiles, and `invert_volume`;
`breath_assessment.reset` resets calibration for `polar_acc`, `controller_pose`,
or `all`. The estimates are diagnostic motion-derived values, not medical
measurements.

The broker can also open a direct Android BLE Polar PMD source on the headset.
When enabled, the broker scans for a Polar-compatible BLE advertisement, connects
to the PMD service, enables control-point indications and data notifications,
starts the ACC stream at 200 Hz / 16 bit / 8 g, decodes ACC frames, publishes
them as `bio:polar_acc`, and feeds the same `bio:breath` assessment path used by
adapter-published frames. This is a broker-side diagnostic source rather than a
medical device integration.

Start the direct source through the WebSocket command API:

```json
{
  "type": "command",
  "schema": "rusty.xr.broker.command.v1",
  "request_id": "polar-001",
  "command": "polar_pmd.start",
  "params": {
    "scan_timeout_ms": 60000
  }
}
```

For manual development installs on Android 12 and newer, grant Bluetooth
runtime permissions before starting the source:

```powershell
adb shell pm grant com.example.rustyxr.broker android.permission.BLUETOOTH_SCAN
adb shell pm grant com.example.rustyxr.broker android.permission.BLUETOOTH_CONNECT
```

The camera provider is a P0 metadata/config provider. It returns a public
projection profile with `requires_client_eye_views=true` and
`requires_client_layer_submission=true`. The active Unity or Rusty XR app must
own camera permission, texture import/decode, current eye views/FOV, shaders,
and OpenXR layer submission. The broker can run a bounded app-context Camera2
probe with `camera_provider.run_app_camera_probe`; that command uses the
broker APK's normal Android permission model to enumerate Camera2 metadata and
attempt one short YUV capture per selected camera ID. It records only metadata
and capture success state, not frame payloads. For a bounded payload-transport
diagnostic, `camera_provider.start_app_camera_luma_stream` captures a small
number of app-context `YUV_420_888` frames, copies only the luma plane, and
writes `raw_luma8` packets through the public `RXYRVID1` binary framing over
an ADB-forwarded TCP connection. The broker still sends only manifest, sample
metadata, and metric events through WebSocket JSON, and labels this path as a
probe rather than a production encoded-video or compositor provider. When the
next encoded diagnostic is needed, `camera_provider.start_app_camera_h264_stream`
routes app-context Camera2 frames directly into an Android platform
MediaCodec H.264 input surface, then writes encoded packets through the same
`RXYRVID1` framing with codec ID `1`. By default this remains a bounded
capture-then-write probe for regression stability. When clients pass
`live_stream=true`, the broker accepts the binary stream socket before Camera2
capture starts, drains encoder output directly to the stream, and writes
schema-3 headers with session projection metadata followed by per-packet source
timestamps. That proves Camera2-to-encoder payload transport without bundling a
codec library and lets receivers bootstrap projected rendering from stream
metadata instead of launch-time projection extras. XR clients can consume this
mode with packet-arrival decode so the receiver no longer has to wait for the
whole declared packet count before submitting decoded frames.
For receiver and shader diagnostics that should not depend on a live camera,
`media.start_synthetic_h264_stream` uses the same Android platform MediaCodec
surface encoder and the same `RXYRVID1` schema-3 binary stream writer, but draws
deterministic app-generated frames into the encoder surface. The command accepts
the same `device_port`, `host_port`, `preferred_width`, `preferred_height`,
`capture_ms`, `max_packets`, `bitrate_bps`, `live_stream`, `lan_stream_enabled`,
`bind_host`, `advertised_host`, and `frame_rate_hz` parameters as the
app-camera H.264 stream. `frame_rate_hz` requests the encoder input cadence;
the stream manifest records the requested value, while measured packet and
decode cadence remain the source of truth for device support.
It also accepts `synthetic_pattern` values `diagnostic-grid`, `checkerboard`,
`luma-ramp`, or `motion-bar`. The `diagnostic-grid` frame contains color bars, a
luma ramp, and a lower checkerboard with an intentional 1-pixel white line
overlay anchored to the checker cell centers for high-frequency blur and
projection diagnostics. This path requires no camera permission and is intended
for stream framing, decoder, projection, and downstream processing tests before
switching back to Camera2 input. Synthetic streams include head-anchored
projection metadata with a deterministic estimated profile so projected
receivers can render the diagnostic image through the same stereo projection
path they use for camera-backed streams.
For LAN experiments, non-loopback H.264 payload binds are opt-in. Passing
`lan_stream_enabled=true` allows `camera_provider.start_app_camera_h264_stream`
to use a non-loopback `bind_host` such as `0.0.0.0`; `advertised_host` can
report the peer-reachable device address in the returned binary endpoint. A
receiving broker can then run `media.start_h264_tcp_proxy` with `remote_host`,
`remote_port`, and `local_port` to subscribe to that remote `RXYRVID1` H.264
TCP stream and republish it on local loopback for the existing XR-side
MediaCodec consumer. The TCP proxy accepts schema-1/2 streams and forwards
schema-3 stream-header projection metadata unchanged for projected receiver
paths. This is a broker-to-broker payload proof, not yet discovery,
pairing/authentication, RTP jitter buffering, or an indefinite production
stream. The `media.run_h264_tcp_proxy_probe` command validates the
same TCP proxy plumbing with an in-process synthetic `RXYRVID1` source and
consumer, so the broker-to-broker relay can be smoke-tested without camera
permission, OpenXR, or a second headset. The follow-up
`camera_provider.run_app_camera_h264_decode_probe` command reuses that same
app-camera encoder path and consumes the resulting H.264 packets inside the
broker process with Android platform MediaCodec byte-buffer output. That
isolates decoder compatibility before a client texture path is attempted, but
texture import, eye views, and OpenXR layer submission still belong to the
active XR client.
For one-device remote-source tests, `tools/video/serve_rxyrvid1_h264.py` can
wrap a saved H.264 Annex-B elementary stream in `RXYRVID1` framing from a
laptop or bridge machine. Start the host source, run `media.start_h264_tcp_proxy`
against that host and port, then launch the composite example with
`rustyxr.brokerH264SourceMode=existing-stream` so the XR client consumes the
proxied incoming stream instead of asking the broker to open Camera2.
This remains a live-stream transport and projection validation path. It still
needs performance work around cadence, buffering, and decoder/render overlap
before it should be considered a production Quest-to-Quest media path.
When the optional ADB shell helper reports bounded shell-visible camera
metadata or Camera2 open/capture
feasibility, the broker also summarizes those results in `cameraProvider` and
`projectionProfile` so clients can inspect candidate camera IDs,
pose/intrinsics availability, and the current evidence level without reading
raw helper diagnostics.

The app-context camera probe requires runtime camera permission. For manual
development runs, grant the public example permission before sending the probe:

```powershell
adb shell pm grant com.example.rustyxr.broker android.permission.CAMERA
adb shell pm grant com.example.rustyxr.broker horizonos.permission.HEADSET_CAMERA
```

The shell-helper status path is a control-plane placeholder for a separate
ADB-launched helper. A normal broker APK does not become Android `shell`.
Developer/operator tooling may launch a helper through authorized ADB and have
that helper report its UID, version, capabilities, heartbeat, optional bounded
codec diagnostics, shell-visible camera metadata, and Camera2 open/capture
feasibility through `shell_helper.report_status`. The source-only helper
example lives in `examples/quest-broker-shell-helper`.
For Store-style versus SideQuest/GitHub/lab distribution boundaries, including
why shell-helper UX must stay out of normal app positioning, see
[`docs/QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md`](../../docs/QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md).

The same helper can optionally run long-lived shell-side watchdogs. The
proximity watchdog reads `dumpsys vrpowermanager` and only re-applies
`com.oculus.vrpowermanager.prox_close` when the virtual proximity state is not
already `CLOSE`. The focus guardian reads foreground-window state, polls the
broker's experiment-control state, applies whitelisted `debug.rustyxr.*`
runtime properties, and reactively relaunches either the broker console or a
target app after Meta shell takes focus. This is recovery after focus loss, not
a pre-emptive Home-button intercept, and it should not be used to dismiss
Guardian, permission, package-installer, or safety UI.

For target launches that might strand the headset while projection tuning is
still unstable, use the `launch_target_guard` mode. In that mode the shell
helper applies the current whitelisted runtime properties, launches the target,
and observes foreground state. Package-only target launches prefer a
`MAIN` intent with `com.oculus.intent.category.VR`, then fall back to the
normal launcher path for non-XR targets. Foreground focus is not treated as
full target health: the same bounded guard window also acts as a preview window
after the target reaches foreground. If the target never reaches foreground, or
if the preview window expires without a return transition, the helper
force-stops the target and rolls back to broker. If Meta Home/menu takes focus
while the target is active, the helper foregrounds the broker console. The
helper turns experiment mode back to `off` only after broker focus is
confirmed.

For XR projection tuning, treat foreground focus and successful `am start`
output as launch-routing evidence only. A target-visible witness still needs a
real headset or captured-display visual check. Loading environments, shell
overlays, and unmarked full-screen camera imagery can look plausible while the
app is not actually presenting the custom projection. Validation targets should
render an unmistakable app-owned marker, such as a red projection border, and
should keep native passthrough disabled when testing whether camera pixels come
from the target app.

`video_lab.register_encoded_stream_manifest` and
`video_lab.record_encoded_sample_metadata` define the control-plane shape for a
future encoded stream: session id, codec/MIME, dimensions, frame rate, payload
transport hint, sample sequence, key-frame flag, encoded byte count, and source
timestamps. `video_lab.record_metric_sample` records the timing/drop/queue
metrics that future encoded-video and texture-import experiments need.
High-rate payload bytes must use a binary transport rather than JSON WebSocket
command payloads. The app-camera luma probe uses that split today for bounded
raw-luma frames, and the app-camera H.264 probe uses Android's platform encoder
for bounded or live-bounded encoded packets. The app-camera H.264 decode probe
verifies platform decoder consumption with byte-buffer output only.
Decode-to-texture and XR layer submission remain separate client/provider work.

XR clients can bring the 2D broker console to the foreground by sending the
`open_ui` broker command. The console has Dashboard, Experiment, Polar,
Launcher, Streams, Commands, and Diagnostics pages plus a `Return to XR App`
button. The button and the `close_ui` broker command both finish only the
broker console Activity while leaving the broker foreground service running;
they do not start or relaunch a target app.

The `Experiment` page is a headset-local manual tuning surface. It stores a
target package/activity, a focus-guardian mode, and the public Makepad Q2Q
hotload knobs for horizontal strength, global/left/right/symmetric/vertical UV
offsets, and content scale. The broker APK itself does not have permission to
write Android system properties, so knob changes become active when an
authorized ADB-launched shell helper is running with `--focus-guardian`. In
`toggle_broker_target` mode, the helper treats a Meta shell/Home transition
from the target as a request to foreground the broker, and a Meta shell/Home
transition from the broker as a request to foreground the target.
`Apply + Target` and `Launch Target` use `launch_target_guard` instead of a
direct broker-side launch, so the shell helper must be connected before those
buttons can switch into the target app. While that guarded launch is active, a
Meta Home/menu transition from the target is treated as the return path to the
broker rather than a request to reopen the target; if the target appears
foregrounded but remains unusable, the bounded preview window returns to broker
without requiring headset input.

The `Polar` page is the headset-local control path for the direct Android BLE
Polar PMD source. It can request the broker APK's Bluetooth runtime permission,
start or stop the broker-owned PMD source, and show the current `bio:polar_acc`
and derived `bio:breath` status. This lets a user start the data source from
inside the headset before launching a target XR app through the broker console.
The page also reports scan counts and recent scan candidates. Discovery ignores
unnamed BLE devices unless they advertise a Polar/Heart Rate/PMD signal, so the
broker does not connect to an unrelated anonymous device before the Polar sensor
appears.

The same page exposes Polar accelerometer breath tuning and calibration
controls. `set_polar_breath_params` accepts broker snake_case names and Unity
runtime-config aliases such as `analysisRateHz`,
`calibrationAcceptedFrames`, `minAcceptedDeltaG`,
`minCalibrationTravelG`, `sampleEmaAlpha`, `projectionEmaAlpha`,
`boundsLowerQuantile`, `boundsUpperQuantile`, `boundsEdgeEase`,
`volumeEventMinDelta`, `invertVolume`, and `accBaseMode`. The calibration
commands let a headset user or companion client reset and begin calibration
without restarting the broker or target app.

For local sideloaded/debug installs, the app exposes a normal Android launcher
entrypoint with the label `Rusty XR Broker` and a broker icon. On Quest, that
normally appears in the Apps library under the headset's sideloaded or Unknown
Sources view. System quick-access pins are launcher-owned UI state; this public
example does not try to self-pin from normal app mode.

The console also has a `Launcher` page for headset-local app shortcuts. It uses
normal Android `PackageManager` discovery for visible
`CATEGORY_LAUNCHER` and `CATEGORY_LEANBACK_LAUNCHER` activities, lets the user
create named lists, search visible launchable apps by label/package/activity,
add them to a list, remove them, and launch them from the headset. This is
normal app mode: it does not require the ADB shell helper, does not install or
force-stop packages, and cannot launch apps that do not expose an accessible
front-door Activity. For the public boundary between normal launchers and
ADB-launched shell helpers, see
[`docs/QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md`](../../docs/QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md).

## Build

Build the APK from the public Rusty XR checkout:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1
```

Output:

```text
examples/quest-broker-apk/build/outputs/rusty-xr-quest-broker-debug.apk
```

APK bytes, local debug signing material, native build products, and diagnostic
captures are intentionally ignored by git.

## Optional LSL Packaging

The source includes a small JNI publisher for liblsl. Rusty XR does not vendor
or redistribute `liblsl.so` in this example. To build an LSL-capable broker APK,
provide a compliant Android `liblsl.so` and keep matching license notices with
your downstream distribution:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1 -LslAndroidLibraryPath <path-to-android-arm64-liblsl.so>
```

Without `liblsl.so`, the broker still builds, answers status requests, accepts
WebSocket latency samples, emits logcat diagnostics, and supports OSC ingress
and OSC egress.

If a developer machine has `RUSTY_XR_ANDROID_LIBLSL` set but the current
artifact should not package native LSL, pass `-DisableNativeLsl`:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1 -DisableNativeLsl
```

An LSL-capable validation build should report `enabled=true` and
`publisher=native-lsl` under `/status` -> `lsl`. Accepted latency samples
should acknowledge `lsl_forwarded=true` and
`fallback_transport=native-lsl`, and a host-side LSL resolver on the same LAN
should discover the `rusty_xr_broker_latency` stream and receive the forwarded
JSON sample. Because `liblsl.so` is user supplied, keep generated APK outputs
and native library payloads out of source and include downstream license
notices before distributing APK bytes.

## Launch Profiles

The companion catalog is in:

```text
examples/quest-broker-apk/catalog/rusty-xr-quest-broker.catalog.json
```

Use `broker-latency-websocket-lsl` for the basic localhost API and optional LSL
path. Use `broker-osc-drive-ingress` when a laptop sends OSC control values to
the headset and a Unity-side client consumes the resulting WebSocket
`osc_drive` or subscribed `stream_event` events.

Catalog launches target `.BrokerStartActivity`, a no-display activity that
starts the foreground broker service and exits. This is the intended long-term
automation path, not a crash workaround: on Horizon OS the visible console is a
2D panel, and shell focus may return to a Horizon placeholder or another
foreground XR app while the broker service remains healthy. Catalog validation
should therefore check the broker process/service and localhost API rather than
expecting `.MainActivity` to own foreground focus.

The visible console remains available from the headset launcher through
`.MainActivity`. It also starts the same foreground service path, but it is
for human inspection and control rather than sidecar launch automation.

Runtime OSC ingress can also be configured over the broker WebSocket command
API, which lets comparison tools start the listener without restarting the
broker Activity.

Direct ADB launch for OSC ingress:

```powershell
adb shell am start -n com.example.rustyxr.broker/.BrokerStartActivity `
  --ez rustyxr.oscIngressEnabled true `
  --ei rustyxr.oscIngressPort 9000 `
  --es rustyxr.oscIngressAddress /rusty-xr/drive/radius
```

Direct ADB launch with the broker-side Polar PMD source enabled:

```powershell
adb shell am start -n com.example.rustyxr.broker/.BrokerStartActivity `
  --ez rustyxr.polarPmdEnabled true `
  --el rustyxr.polarScanTimeoutMs 60000
```

Send a probe value from the companion CLI:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- osc send --host <quest-lan-ip> --port 9000 --address /rusty-xr/drive/radius --arg float:0.75
```

After forwarding the broker TCP endpoint with the companion CLI, a Rust-side
probe can exercise the same status, command, stream, subscription, and latency
sample path:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker forward --serial <serial>
cargo run -p rusty-xr-broker-client-probe -- status
cargo run -p rusty-xr-broker-client-probe -- streams
cargo run -p rusty-xr-broker-client-probe -- camera-provider
cargo run -p rusty-xr-broker-client-probe -- projection-profile
cargo run -p rusty-xr-broker-client-probe -- app-camera-probe
cargo run -p rusty-xr-broker-client-probe -- shell-helper-status
cargo run -p rusty-xr-broker-client-probe -- sample --subscribe
cargo run -p rusty-xr-broker-client-probe -- open-ui
cargo run -p rusty-xr-broker-client-probe -- close-ui
```

Companion-side diagnostics can compare direct OSC with broker-routed OSC and
publish Polar-compatible bio payloads through the same broker stream-event
surface:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker compare --quest-host <quest-lan-ip> --serial <serial> --out .\artifacts\broker-compare --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker bio-simulate --serial <serial> --out .\artifacts\broker-bio-sim --json
```

## Validation Status

This proof of concept has been validated with a Unity client running on Quest:
the Unity app connected to the broker over localhost WebSocket, sent latency
samples for broker-side LSL forwarding, and consumed OSC-driven broker events
to drive a live scene parameter. A broker build with a user-supplied Android
`liblsl.so` has also been validated by resolving `rusty_xr_broker_latency` from
Windows and pulling forwarded string samples. The public Rust client probe now
covers the same broker contract from a Rust-native code path.
