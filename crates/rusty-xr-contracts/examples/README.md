# Rusty XR Contracts Examples

These examples are authored for the public repository and use synthetic data.
They do not require headset hardware, APK packaging, native capture APIs, or
downstream application repositories.

## Plain Stereo Feedback Layout

`plain_stereo_feedback_layout.rs` builds a public `PlainStereoLayer` from a
monoscopic 16:9 feedback source, fits it into a square projected surface,
computes the visible border geometry, and prints the result as JSON.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example plain_stereo_feedback_layout --features serde
```

The example demonstrates the contract boundary only. Platform adapters remain
responsible for runtime permissions, frame capture, texture import, renderer
integration, and layer submission.

## Composite Feedback Session

`composite_feedback_session.rs` builds on the layout example by adding
synthetic session diagnostics: display-composite capture source state,
app-render source state, room mesh state, environment-depth state, and a
Companion catalog hint for future public APK metadata.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example composite_feedback_session --features serde
```

This is still a no-hardware example. It does not request permissions, stream
pixels, build an APK, or submit native compositor layers.

## Meta Passthrough Style Catalog

`meta_passthrough_style_catalog.rs` emits a public catalog of native
compositor-passthrough layer descriptors: neutral reconstruction underlay,
overlay with opacity/edge/BCS tuning, mono-to-mono luminance remap,
projected-mesh mono-to-RGBA color map, and a runtime LUT binding.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example meta_passthrough_style_catalog --features serde
```

The example does not create OpenXR handles, submit layers, upload projection
meshes, or allocate LUTs. It shows the contract shape a native adapter can
translate into `XR_FB_passthrough` / `XR_META_passthrough_color_lut` calls.

## Audio-Reactive Passthrough Style

`audio_reactive_passthrough_style.rs` demonstrates a generic public control
pattern where normalized phase and amplitude values drive a mono-to-RGBA
passthrough color map and edge alpha.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example audio_reactive_passthrough_style --features serde
```

The example uses synthetic controls and public gradient stops. It does not open
a microphone, analyze live audio, submit OpenXR layers, or reproduce any
downstream visual-effect tuning.

## Visual Strobe Profiles

`visual_strobe_profiles.rs` emits a public catalog of intentional visual
strobe descriptors: full-field red/black profiles and phase-inverted
passthrough LUT profiles at 10, 40, and 60 Hz, each with a 120 Hz timing plan
and explicit safety warning.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example visual_strobe_profiles --features serde
```

The example does not draw or flash anything. It prints descriptors only.
Applications that actually present these stimuli must require explicit opt-in
and should follow the warnings in
`docs/VISUAL_STROBE_PROFILES.md`.

## Developer Home Manifest

`developer_home_manifest.rs` emits a synthetic public Rusty Kiosk /
developer-home manifest: launcher, system, clock, and diagnostics panels, a
normal launcher entry, settings shortcuts, helper state, bounded supervisor
policy, and a focus-recovery event.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example developer_home_manifest --features serde
```

The example does not build an APK, start ADB, launch another app, intercept
system buttons, or provide kiosk lock-down. It demonstrates the contract shape
documented in `docs/QUEST_DEVELOPER_HOME_MENU.md`.

## Kiosk Command Run Record

`kiosk_command_run_record.rs` emits one public run record for a Rusty Kiosk
surface check. The record ties the command goal, preferred MCP provider,
broker API fallback, ADB fallback note, foreground evidence, clock epoch, and
before/after control-plane status into one JSON shape.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example kiosk_command_run_record --features serde
```

The example does not start `hzdb`, ADB, MCP, a broker, or a headset. It
demonstrates the common evidence envelope that provider adapters and operator
tools should emit.

## Effect Stack Diagnostic Manifest

`effect_stack_diagnostic_manifest.rs` emits a generic multi-pass visual
pipeline descriptor and a layer comparison report for a synthetic
color/edge-detection stack.

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example effect_stack_diagnostic_manifest --features serde
```

The example is data-only. It does not define shader behavior, private visual
tuning, native texture ownership, or a renderer backend. It demonstrates the
public diagnostic shape documented in `docs/EFFECT_STACK_DIAGNOSTICS.md`.

## Depth World-Space Contract

`depth_world_space_contract.rs` emits a synthetic environment-depth contract for
the world-space-first path:

```text
depth UV -> depth view ray -> app reference-space point -> render-eye screen UV
```

Run it with:

```powershell
cargo run -p rusty-xr-contracts --example depth_world_space_contract --features serde
```

The example does not acquire `XR_META_environment_depth` or submit a renderer
layer. It demonstrates the public evidence shape that a Quest adapter can emit
for depth mesh, retained particle, or scene particle-map diagnostics.
