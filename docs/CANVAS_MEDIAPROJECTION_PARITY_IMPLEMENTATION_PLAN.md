# Canvas and MediaProjection Parity Plan

This plan makes the HWB, GL/OES, and Makepad alignment lanes comparable with
the same two projection states and the same two capture sources.

## Target Matrix

Each renderer lane must produce these four captures with explicit geometry:

| Renderer | Canvas mode | Custom projection mode | MediaProjection | HzDB |
| --- | --- | --- | --- | --- |
| HWB/composite | `world-canvas` | `display-screen-homography` | required | required |
| GL/OES | `world-canvas` | `display-screen-homography` | required | required |
| Makepad CPU-YUV | `world-canvas` + `full-frame-diagnostic` content | `display-screen-homography` + `camera-projection` content | required | required |

The reference geometry is:

```text
projectionDepthMeters=1.434085
cameraPreviewFovYDegrees=69.763084
cameraPreviewOffsetYMeters=-0.168832
cameraRawOverlayOverscan=1.0
projectionLayerVisible=true
```

All launches must use the complete renderer profile for camera transport,
stereo layout, frame rate, color mode, and fallback policy. A geometry-only
activity start is not a valid reference launch.

## Implementation

1. Keep the shell-helper proximity watchdog running during long builds and
   captures. The helper must allow a no-broker watchdog mode without disabling
   the watchdog loop or flooding broker heartbeat failures.
2. Reuse the HWB/composite implementation as the reference: it already exposes
   `world-canvas`, custom display-screen homography, and app-side
   MediaProjection streaming.
3. Add GL/OES app-side MediaProjection streaming by giving the GLES
   `NativeActivity` a small Java consent flow and foreground streaming service.
   Expose `rustyquest.cameraProjectionMode=world-canvas` by selecting the existing
   full-frame-to-projection-area shader path for live direct Camera2 OES frames.
4. Add Makepad app-side MediaProjection streaming in the active Makepad Android
   wrapper. The Rusty Quest Makepad evidence app must set
   `debug.rustyquest.camera.projection.mode=world-canvas` for the canvas-equivalent
   pass, size and place its XR panel from the solved projection depth/FOV/Y
   offset/overscan, and set
   `debug.rustyquest.camera.projection.mode=display-screen-homography` plus
   `debug.rustyquest.makepad.camera.projection.geometry.profile=camera-projection`
   for the custom screen-to-camera homography path.
5. Add a parity capture harness that runs the three renderers, the two
   projection states, and the two capture sources. Do not substitute ADB
   screencap for a missing MediaProjection path.

## Validation

For each of the 12 captures, the run manifest must record renderer, projection
state, capture source, launch profile, geometry values, APK path, and whether
the camera transport used the expected live stereo path. The visual evidence is
the captured PNG plus the logcat marker proving the selected content mapping:

- canvas: full camera frame mapped to the solved projection area;
- custom: screen-to-camera homography / camera-footprint sampling;
- MediaProjection: one app-side display-composite frame;
- HzDB: one per-eye compositor screenshot.

The suite is only passing when all three renderers have real MediaProjection
frames and HzDB screenshots for both projection states.
