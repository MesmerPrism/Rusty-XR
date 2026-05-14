# OpenGL/OpenXR Multilayer Stack Plan

This is the public Rusty XR implementation plan for an OpenGL ES + OpenXR
multilayer video-rendering lane.

Status: candidate implementation lane. It does not replace the current
OpenXR/Vulkan hardware-buffer path. It gives Rusty XR a reusable way to compare
Android `SurfaceTexture` / `GL_TEXTURE_EXTERNAL_OES` ingestion, OpenGL FBO
pass graphs, projection diagnostics, and public effect-stack examples against
the existing Vulkan and Makepad lanes.

## Boundary

Rusty XR owns the general infrastructure:

- OpenXR + OpenGL ES presentation example;
- Android `MediaCodec` to `SurfaceTexture` / external-OES ingestion;
- OES-to-internal-texture copy pass;
- public pass-graph and layer-diagnostic vocabulary;
- projection-policy diagnostics and scorecard fields;
- public example stacks such as luma, edge detection, masks, simple color maps,
  and final composites.

Downstream apps own product-specific behavior:

- private shader recipes, exact visual target stacks, and tuning constants;
- generated captures, screenshots, logs, and APK identities;
- app-specific launch profiles and validation artifacts;
- final decisions about whether a private renderer uses the public GL lane,
  the Vulkan hardware-buffer lane, or both.

Public docs and examples must use generic terms such as "downstream app",
"public example", "guide texture", "edge detector", "mask", "candidate", and
"reference". Do not copy private pass recipes or local validation evidence into
this repository.

## Architecture

The target shape is one OpenXR app with an OpenGL ES renderer:

```text
broker H.264 or Camera2 frames
  -> MediaCodec or camera output Surface
  -> SurfaceTexture
  -> GL_TEXTURE_EXTERNAL_OES
  -> OES ingest copy with SurfaceTexture transform
  -> internal GL_TEXTURE_2D/FBO pass graph
  -> projection diagnostics and public effect layers
  -> OpenXR left/right swapchain images
```

OpenXR owns eye timing, swapchains, predicted poses, and compositor
submission. OpenGL ES owns video ingestion and the multipass renderer. Do not
try to run two plain Android apps, one per eye.

The external OES texture is ingestion-only. It should be sampled with
`samplerExternalOES`, then copied into normal internal GL textures before
multilayer processing. All guide, edge, mask, displacement, debug, and final
passes should operate on normal GL textures/FBOs.

## Why This Lane Exists

The current public performance target remains the Vulkan path:

```text
MediaCodec / Camera2 producer
  -> ImageReader PRIVATE
  -> AHardwareBuffer
  -> Vulkan external-memory import
  -> OpenXR projection shader
```

That path is still the right baseline for low-level buffer control, native
projection metadata, Camera2 and broker H.264 hardware-buffer parity, and future
compute-heavy work.

The OpenGL lane is worth building because Android video APIs naturally speak
`Surface` / `SurfaceTexture`, and OpenGL FBO ping-pong is a straightforward way
to implement and inspect public multilayer video stacks. It may be simpler for
effect iteration, but it has to prove itself through headset evidence:

- OpenXR + GLES support on target Quest runtime;
- no CPU-YUV upload for broker synthetic H.264;
- correct OES transform handling;
- explicit projection-stage and invalid-fill diagnostics;
- comparable pass-graph output;
- equal or better iteration value, frame time, or thermal behavior.

## Project Touch Map

### Rusty XR

Primary owner.

Expected areas:

- `docs`: this plan, media/projection/effect-stack links, and scorecard
  guidance.
- `rusty-xr-contracts`: public pass descriptors, diagnostic layer taps, and
  comparison report fields if gaps appear.
- `rusty-xr-camera-model`: projection-stage and temporal-projection helpers
  reused by both Vulkan and GL examples.
- `rusty-xr-broker-model`: stream/header/timestamp contracts needed for
  broker synthetic H.264 parity.
- `rusty-xr-quest-diagnostics`: GL/OpenXR runtime status, frame cadence,
  SurfaceTexture, FBO, and pass-budget counters if they become stable.
- `examples/quest-gl-openxr-video-stack-apk`: proposed public Quest example for
  OpenXR/GLES presentation, broker H.264 OES ingestion, projection diagnostics,
  and public-safe multilayer effects.
- `tools`: public scorecard and layer-packet analyzers when they become
  renderer-neutral.

### Rusty XR Companion Apps

Optional public validation surface after the example APK exists.

Expected work:

- catalog metadata for the public GL example;
- install/launch/verify workflows;
- scorecard collection for GL, Vulkan, and Makepad comparison runs.

### Makepad Comparison Lane

Comparison witness, not a dependency for this implementation.

Expected work:

- keep Makepad broker H.264 CPU-YUV evidence separate from GL OES evidence;
- compare projection-stage tokens and pass-budget costs against the new GL
  lane;
- avoid treating framework packaging or hotload behavior as proof that GL
  ingestion is correct.

### Downstream Apps

Consumers and validation targets.

Expected work:

- consume public Rusty XR GL contracts and example patterns;
- keep private pass recipes and tuning downstream;
- feed sanitized findings back into public contracts only when they are general.

## Implementation Iterations

### Iteration 0: Documentation And Boundary

Goal: make the ownership split explicit before code moves.

Tasks:

- add this public Rusty XR plan;
- link it from the implementation plan, README, effect-stack diagnostics, and
  docs index;
- keep private visual-effect plans in downstream repos as consumer notes.

Acceptance:

- public docs describe reusable GL infrastructure without private pass recipes;
- downstream docs point to this plan as the implementation owner;
- public boundary scan passes.

### Iteration 1: Contract Gap Review

Goal: decide whether existing public contracts already cover the GL lane.

Tasks:

- review `EffectStackDescriptor`, `EffectDiagnosticLayer`, and
  `EffectStackComparisonReport` for OES ingest, internal FBO layer taps, and
  public edge/mask examples;
- review camera projection contracts for `screen_to_surface`,
  `surface_to_camera`, and `screen_to_camera` rows;
- review temporal projection contracts for reuse by GL;
- add only data-only contract fields that are demonstrably missing.

Acceptance:

- a public-safe example stack can describe raw, luma, edge, mask, and final
  layers without renderer-specific handles;
- projection-stage and temporal metrics can be shared by Vulkan, GL, and
  Makepad lanes;
- no Android, EGL, OpenXR, or GL handles enter framework-neutral crates.

### Iteration 2: OpenXR/GLES Feasibility Example

Goal: prove Quest OpenXR presentation through OpenGL ES.

Tasks:

- add a new opt-in public example, initially
  `examples/quest-gl-openxr-video-stack-apk`;
- request `XR_KHR_opengl_es_enable`;
- create EGL/GLES context and query
  `xrGetOpenGLESGraphicsRequirementsKHR`;
- create an OpenXR session with `XrGraphicsBindingOpenGLESAndroidKHR`;
- enumerate GL-compatible color/depth swapchain formats;
- render distinct static left/right diagnostic grids;
- log extension list, GLES version, EGL config, swapchain dimensions, view
  count, and frame cadence.

Acceptance:

- distinct left/right eye output is visible;
- OpenXR frame loop is stable;
- no broker, decoder, SurfaceTexture, or effect stack is present yet;
- failure at this stage blocks all later GL work.

### Iteration 3: GL Swapchain Rendering Discipline

Goal: make OpenXR swapchain rendering boring and measurable.

Tasks:

- start with one non-array swapchain per view;
- bind each acquired swapchain GL texture to an FBO;
- use the layer view image rect for `glViewport`;
- report FBO completeness, image index, eye index, viewport, and render status;
- keep depth optional until a public pass needs it.

Acceptance:

- each eye renders through an FBO-complete path;
- screenshots or headset inspection show fresh per-eye output;
- no texture arrays or multiview until the simple path is stable.

### Iteration 4: Broker Synthetic H.264 To OES Textures

Goal: prove decoded-texture parity without CPU-YUV upload.

Tasks:

- request the same public broker synthetic H.264 profile used by Vulkan and
  Makepad comparison lanes;
- parse stream headers, timestamps, and projection metadata;
- create one GL external-OES texture and one `SurfaceTexture` per eye;
- wrap each `SurfaceTexture` in an Android `Surface`;
- configure one `MediaCodec` decoder per eye with that output surface;
- treat frame-available callbacks as dirty flags only;
- call `updateTexImage()` only on the GL render thread with the owning context
  current;
- log frame-available count, `updateTexImage()` count, stream sequence,
  queued PTS, SurfaceTexture timestamp, transform matrix hash, and decoder
  errors.

Acceptance:

- both eyes show decoded broker synthetic frames;
- no CPU-YUV upload is used;
- codec config is consumed correctly;
- source-feed and decoded-texture parity are reported separately;
- SurfaceTexture timestamp is logged as diagnostic data, not treated as the
  sole stereo-pairing authority.

### Iteration 5: OES Ingest Copy To Internal Raw FBO

Goal: convert external video textures into ordinary GL working textures.

Tasks:

- add a `samplerExternalOES` ingest shader;
- apply the `SurfaceTexture` transform matrix explicitly;
- apply public source-eye mapping and texture-orientation state separately;
- write raw-left/raw-right internal textures;
- start with `GL_RGBA8`; introduce `GL_R16F` or `GL_RGBA16F` only when metrics
  show precision matters;
- export a raw diagnostic layer with dimensions, format, timestamp source, and
  frame sequence metadata.

Acceptance:

- raw layer visually matches the direct OES preview;
- raw-left and raw-right checksums differ when the source differs;
- every packet records transform and orientation policy explicitly.

### Iteration 6: Projection-Policy Diagnostic Layers

Goal: make projection policy visible before public effects are interpreted.

Tasks:

- emit `screen_to_surface`, `surface_to_camera`, and `screen_to_camera` rows;
- add diagnostic layers for projection-valid mask, camera-UV X, camera-UV Y,
  invalid-fill regions, and final coverage;
- report valid-mask active fraction, bounding box, row spans, invalid-UV
  percentage, guide-domain label, and invalid-fill policy;
- support at least these public policy lanes:
  visual-continuity fallback, reference-black-invalid, and direct
  surface-camera sampling.

Acceptance:

- public projection-footprint analyzers can run on GL packets;
- effect tuning is blocked until footprint deltas are understood;
- a visually plausible GL result cannot be accepted without projection-policy
  metadata.

### Iteration 7: Public Multilayer Stack Example

Goal: publish a useful stack without publishing downstream private effects.

Start with public-safe layers:

1. raw source;
2. luma guide;
3. edge-detection guide;
4. threshold or confidence mask;
5. optional generic blur or smoothing helper with public parameters only;
6. debug tint/comparison layer;
7. final composite.

Rules:

- every pass can be disabled independently;
- every pass has a diagnostic tap;
- every pass logs input/output texture size and format;
- no private color cycles, displacement recipes, or product-tuned constants;
- neutral defaults produce identity-like output.

Acceptance:

- contact sheet and comparison report are generated;
- public examples demonstrate edge/mask/composite behavior;
- downstream apps can substitute their own private passes without changing the
  public packet shape.

### Iteration 8: Deterministic Cross-Lane Matrix

Goal: compare GL against the existing public lanes using the same source.

Lanes:

- OpenXR/Vulkan hardware-buffer path;
- OpenXR/OpenGL ES SurfaceTexture/OES path;
- Makepad CPU-YUV path;
- downstream reference lane when available, recorded only through sanitized
  scorecards before public publication.

Compare in this order:

1. source-feed parity;
2. decoded-texture parity;
3. projection-stage parity;
4. projection-footprint parity;
5. raw-layer parity;
6. public guide or edge-layer parity;
7. final-composite parity.

Acceptance:

- GL decoded-texture parity passes before projection claims;
- stage rows are present for every lane being compared;
- projection differences are classified before shader constants are changed.

### Iteration 9: Performance And Pass-Budget Matrix

Goal: decide whether the GL lane earns ongoing maintenance.

Metrics:

- OpenXR frame cadence;
- SurfaceTexture update rate and skipped-frame counters;
- decoder input/access-unit rate;
- frame age at submit;
- pass count;
- FBO switches;
- intermediate texture bytes per frame;
- CPU time, GPU time, and thermal state where available;
- capture/diagnostic overhead versus no-capture profile;
- projection motion and invalid-UV scorecard fields.

Acceptance:

- GL wins on performance, iteration speed, or diagnostic clarity enough to
  justify a second renderer;
- if it only simplifies ingestion while losing frame time or thermal behavior,
  keep it as a diagnostic lane rather than a default renderer.

### Iteration 10: Temporal Projection Integration

Goal: reuse existing Rusty XR temporal projection contracts.

Tasks:

- report target and visual `screen_to_camera` states per eye;
- apply the selected visual projection before raw guide/effect generation when
  smoothing is enabled;
- record applied projection motion, residual projection motion, held-frame
  count, and frame-adoption state;
- keep no-smoothing profiles available for comparison.

Acceptance:

- GL and Vulkan lanes use compatible temporal scorecards;
- stereo lockstep remains explicit;
- fast motion produces bounded applied projection motion rather than hidden
  projection jumps.

### Iteration 11: Live Camera2 SurfaceTexture Candidate

Goal: test live camera input only after broker synthetic parity.

Tasks:

- feed Camera2 output surfaces into per-eye `SurfaceTexture`s;
- log camera source capability, selected resolution, frame number/timestamp,
  vendor/source tags when available, and active source-eye mapping;
- do not hard-code one resolution or aspect ratio;
- compare live camera cadence against broker synthetic cadence.

Acceptance:

- left/right live camera feeds are visible;
- resolution and camera metadata are explicit;
- source frame age and cadence are measured;
- failures do not invalidate the broker synthetic regression lane.

### Iteration 12: Publication And Adapter Split

Goal: decide what remains in public Rusty XR.

Keep public:

- stable contracts and scorecard fields;
- GL/OpenXR feasibility example if it is maintainable;
- public edge/mask/composite examples;
- renderer-neutral packet analyzers;
- sanitized comparison guidance.

Keep downstream:

- exact private effect recipes and constants;
- product-specific visual targets;
- device-specific captured artifacts;
- package identities, signing, launch profiles, and release payloads.

Acceptance:

- public examples remain useful without downstream repos;
- downstream apps can consume public infrastructure without copying private
  behavior back into Rusty XR;
- public boundary scan passes before every push.

## Decision Gate

The OpenGL/OpenXR lane can become a preferred public multilayer example only if
it proves at least two of these:

- simpler implementation for video-backed pass graphs;
- faster public effect iteration;
- equal or better projection smoothness;
- equal or better frame time or thermal behavior;
- cleaner layer diagnostics and scorecards;
- lower maintenance risk than the Vulkan hardware-buffer path for the specific
  video-processing use case.

Until then, the Vulkan hardware-buffer path remains the default Rusty XR
performance baseline, and OpenGL remains an isolated comparison lane.

## References

- Android `SurfaceTexture` API:
  <https://developer.android.com/reference/android/graphics/SurfaceTexture>
- Android `MediaCodec` API:
  <https://developer.android.com/reference/android/media/MediaCodec>
- `GL_OES_EGL_image_external`:
  <https://registry.khronos.org/OpenGL/extensions/OES/OES_EGL_image_external.txt>
- Khronos OpenXR SDK `hello_xr` OpenGLES plugin:
  <https://raw.githubusercontent.com/KhronosGroup/OpenXR-SDK-Source/main/src/tests/hello_xr/graphicsplugin_opengles.cpp>
- Meta Passthrough Camera API overview:
  <https://developers.meta.com/horizon/documentation/unity/unity-pca-overview/>
- Rusty XR effect-stack diagnostics:
  [EFFECT_STACK_DIAGNOSTICS.md](EFFECT_STACK_DIAGNOSTICS.md)
- Rusty XR camera projection parity workplan:
  [CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md](CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md)
- Rusty XR temporal projection plan:
  [CUSTOM_STEREO_CAMERA_TEMPORAL_REPROJECTION.md](CUSTOM_STEREO_CAMERA_TEMPORAL_REPROJECTION.md)
