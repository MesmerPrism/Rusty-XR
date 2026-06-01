# Quest OpenXR Hand Mesh Workflow

Status: public workflow note for downstream Quest/OpenXR examples.

This note captures reusable lessons for recording Meta hand tracking and
provider hand meshes from a foreground Quest OpenXR app. It is intentionally
public-safe: it does not name downstream packages, devices, local paths, or
private captures.

For a fuller explanation of the system, workflow, and artifact formats, see
`docs/QUEST_HAND_MESH_CAPTURE_SYSTEM.md`.

## Boundary

- Rusty XR owns the OpenXR runtime boundary: extension negotiation, tracker
  lifetime, provider bind-mesh acquisition, per-frame hand polling, and
  renderer/example integration.
- Rusty Matter owns the hand geometry and animation model: rigs, topology keys,
  skeleton/skin data, joint clips, validation mesh frames, validation reports,
  and later glTF/GLB export semantics.
- Host tooling owns install, launch, status polling, artifact pull, and evidence
  collection.
- Manifold should become the command/session authority when a recorder route is
  promoted beyond a temporary ADB or local-shell test adapter.

## Launch Readiness

Do not treat Android activity launch success as XR readiness. On Quest, a
target app can return `Status: ok` from `am start -W` and still remain on the
runtime loading screen if it has not entered XR presentation.

Useful readiness witnesses are:

- app-owned frame-loop logs;
- OpenXR extension plan logs;
- successful `XR_FB_hand_tracking_mesh` bind-data acquisition;
- stable status output from the app or host tool;
- screenshots only after the app reports XR-frame readiness.

For Makepad-based Quest examples, keep the app-side XR presentation transition
explicit and logged. If the headset remains on the loading screen, verify that
transition before debugging renderer, passthrough, or hand tracking logic.

## Debug ADB Control

For development-only host control, a debuggable APK can expose command and
status files through:

```text
adb shell run-as <package> sh -c '<command using files/...>'
```

This is a debug adapter, not a production export path. Release builds normally
cannot use `run-as`. Public-sdcard handoff paths can fail because shell-created
directories and app writes have different scoped-storage behavior. Production
recorders should provide an app-owned export route, such as a visible export
command, a content provider, a broker route, or a managed host-tool pull path.

Recorder apps should launch idle, clear stale commands before controlled runs,
write live status, accept bounded start/stop commands, and keep per-session
status inside pulled bundles.

## Hand Mesh Capture Shape

Record the provider bind mesh once per hand:

- topology key;
- handedness and reference space;
- joint bind poses;
- joint parent indices;
- joint radii;
- bind vertices and normals;
- triangle indices;
- UVs when available;
- vertex blend indices and weights.

Record per-frame tracking separately:

- frame index and timestamp;
- tracked joint poses;
- raw location/tracking flags;
- confidence or equivalent quality state;
- tip lengths and pinch strengths when available.

The observed Meta hand-mesh provider topology in one Quest run was 26 bind
joints, 1360 vertices, 6942 indices, and 2314 triangles per hand. Treat these
as observed provider topology values, not portable constants. Every capture
should carry its own topology key and validate its own array counts.

## Validation Mesh Frames

Sparse validation mesh frames are useful before the final exporter exists. They
are baked geometry, so they can be viewed directly in a WebGL/Three.js viewer
to check:

- handedness;
- coordinate-space orientation;
- timing and frame ordering;
- gross skinning deformation;
- left/right topology agreement.

Validation frames are evidence and parity witnesses. They should not replace
the compact rig-plus-joint-clip export path for final animated mesh files.

When a runtime uses a compact hand representation, the rig should carry the
runtime-to-bind mapping. The Makepad compact shape records 21 runtime joints
and stores the five OpenXR tip joints as tip-length values reconstructed from
the parent distal runtime pose. Matter should validate that mapping before it
marks a capture ready for skinned GLB export.

## Promotion Path

1. Start with a foreground OpenXR example that logs extension readiness and
   bind-mesh counts.
2. Add an idle-first recorder controlled by a host tool.
3. Pull artifacts into a Matter validation tool that checks rigs, clips,
   validation frames, counts, finite values, topology keys, timing, and the
   declared required-hand coverage (`both`, `any`, `left`, or `right`).
4. Catalog the run through Manifold once command/session authority is needed.
5. Resolve and validate the runtime-to-bind joint mapping for every present
   hand.
6. Export animated GLB from Matter data and parse it back for validation.
