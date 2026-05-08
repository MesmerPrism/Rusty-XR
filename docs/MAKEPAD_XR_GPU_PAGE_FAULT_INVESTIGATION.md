# Makepad XR GPU Page Fault Investigation

This document tracks sanitized investigation results for the Makepad XR Quest
page-fault blocker. It intentionally records only public-safe observations:
variant shape, validation command class, and outcome. Raw logcat dumps, local
paths, device serials, generated APKs, and private app/package details stay out
of the repository.

## Current Hypothesis

The fault has been narrowed below Rusty XR app widgets and below Makepad's
explicit OpenXR path. A control run of Makepad's upstream XR example reproduced
the same GPU page-fault symptom after the Windows Makepad Android tooling fixes.

The fault now survives removal of the obvious XR feature surfaces and also
survives removal of Makepad's explicit OpenXR lifecycle. In Quest/Vulkan-shaped
builds it still appears when environment depth is not created, passthrough is
not created, no composition layers are submitted, the OpenXR frame loop is
skipped, no OpenXR session is begun or created, Makepad OpenXR instance
creation is skipped, and the normal Android activity is launched instead of the
generated XR activity.

Fresh default Android/GLES-only controls did not reproduce the page-fault or
small hardware-buffer warning class over the same 90s window. A plain upstream
Makepad counter app then reproduced the page-fault class when built in the
Quest/Vulkan package shape and stayed clean when the same Quest-shaped control
was forced through GLES. A follow-up Quest/Vulkan control stayed clean when the
Vulkan window draw/present path was skipped after activity and surface startup.
Further splits inside `draw_pass_and_present` showed that suppressing
suboptimal-triggered swapchain recreation stayed clean, while a same-toolchain
baseline still faulted. Waiting the device idle immediately before that
suboptimal-triggered recreation stayed clean, and waiting Makepad's current
window-frame fence before the same recreation also stayed clean. The same
frame-fence wait has now been promoted from a temporary diagnostic into the
local Makepad fork state and stayed clean in a no-diagnostic Quest/Vulkan
counter run. The strongest current lead is therefore a synchronization/lifetime
issue in Makepad's Android Vulkan window-swapchain recreation after
acquire/present reports suboptimal, not generic Makepad Android activity
startup, the Quest manifest shape alone, Vulkan backend/surface setup by
itself, `makepad-xr` scene content, environment depth, passthrough, OpenXR
composition layers, or OpenXR session ownership.

## Investigation Rules

- Change one variable per attempt.
- Clear old logs before each run and stop unrelated XR probes when possible.
- Treat a running process and emitted marker as startup evidence, not as GPU
  stability.
- Do not add camera transport, broker integration, or projected camera rendering
  until the Makepad XR smoke path is clean or the fault is narrowly bracketed.
- Record whether the run used the Makepad launcher activity or the generated XR
  activity directly.

## Attempt Log

| Attempt | Variant | Launch | Result | Interpretation |
| --- | --- | --- | --- | --- |
| 0 | Rusty XR Makepad smoke, `makepad-xr` `XrRoot`, status panel, cube marker, `XrPermissionsFlow` | Makepad launcher and direct generated XR activity | Marker emitted; app process remained alive; GPU page faults observed | Confirms the blocker exists in the current Rusty XR Makepad lane. |
| 1 | Makepad upstream XR example | Direct generated XR activity | Generated XR activity focused; GPU page faults observed | Fault is likely not specific to the Rusty XR smoke panel. |
| 2 | Rusty XR Makepad smoke with `XrPermissionsFlow` removed | Direct generated XR activity | Marker emitted; process remained alive; GPU page faults persisted | Fault does not require the visible permission-flow widget. |
| 3 | Rusty XR Makepad smoke with status panel / `XrView` removed, keeping a simple 3D marker | Direct generated XR activity | Marker emitted; GPU page faults persisted | Fault does not require Makepad UI surfaces. |
| 4 | Empty `XrRoot`, environment cube enabled | Direct generated XR activity | GPU page faults persisted; small hardware-buffer allocation warnings appeared | Fault does not require explicit scene content. |
| 5 | Empty `XrRoot`, environment cube disabled | Direct generated XR activity | GPU page faults persisted; small hardware-buffer allocation warnings persisted | Fault is not only passthrough environment-cube drawing. |
| 6 | Empty `XrRoot` after clearing headset-camera permission | Direct generated XR activity | Permission prompt path appeared; GPU page faults persisted | A persistent headset-camera grant is not required. |
| 7 | Upstream Makepad XR with local diagnostic no-op camera sync | Direct generated XR activity | GPU page faults persisted | Fault is not explained solely by passthrough camera synchronization. |
| 8 | Upstream Makepad XR with fixed foveation disabled | Direct generated XR activity | GPU page faults persisted | Fixed foveation is not the blocker by itself. |
| 9 | Upstream Makepad XR with XR MSAA set to 1 | Direct generated XR activity | One short run was clean; longer repeat faulted | MSAA changes timing/frequency but does not clear the fault. |
| 10 | Upstream Makepad XR with lower XR render scale | Direct generated XR activity | GPU page faults reduced in one run but persisted | Render pressure affects the signal but is not decisive. |
| 11 | Upstream Makepad XR with lower XR render scale and MSAA 1 | Direct generated XR activity | GPU page faults persisted | Combined pressure knobs did not eliminate the issue. |
| 12 | Upstream Makepad XR with queue idle after each OpenXR submit | Direct generated XR activity | GPU page faults persisted | Not simply app-side queue work continuing after resource recycle. |
| 13 | Upstream Makepad XR with app-side environment-depth acquire/readback disabled | Direct generated XR activity | One 60s run had no GPU page faults, but warnings remained | Environment-depth acquire/readback affects timing and remains useful for comparison. |
| 13b | Same as attempt 13 with a longer run | Direct generated XR activity | GPU page faults returned in the longer run | Depth acquire/readback is not a complete fix; runtime buffer churn or image ownership remains suspect. |
| 14 | Environment-depth provider not created, with the depth extension still requested and passthrough still created | Direct generated XR activity | Immersive activity appeared, then the app crashed natively before a stable sample; no GPU faults were observed in that invalid run | Skipping provider creation is not a valid Makepad smoke state without more lifecycle work. |
| 15 | Provider and depth swapchain created, provider start disabled, and per-frame acquire disabled | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Provider start, acquire, and readback are not required for the fault. |
| 16 | Provider and depth swapchain created, depth swapchain image enumeration/view creation disabled, provider start disabled, and per-frame acquire disabled | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Depth swapchain images/views are not required; later attempts move the lead below passthrough, composition, and OpenXR ownership. |
| 17 | Passthrough not created; depth provider/swapchain still created but provider start, acquire, and depth images disabled; projection layer submitted | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Passthrough creation and passthrough layer submission are not required. |
| 18 | Passthrough created/resumed but not submitted as a composition layer; depth start/acquire/images disabled; projection layer submitted | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Passthrough composition-layer submission is not required. |
| 19 | No passthrough, no environment-depth provider/swapchain, projection layer submitted | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Environment-depth provider/swapchain creation is not required. |
| 20 | No passthrough, no environment-depth provider/swapchain, no composition layers submitted | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Runtime compositor sampling of submitted layers is not required; later attempts push the lead below OpenXR render-target ownership. |
| 21 | Same as 20, but Makepad's Vulkan OpenXR draw path returned before command-buffer recording or queue submit | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | App-side Vulkan draw submission into the OpenXR color target is not required. |
| 22 | Same as 21, plus OpenXR color swapchain acquire/wait/release disabled | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Per-frame color swapchain acquire/release is not required. |
| 23 | Same stripped setup, plus OpenXR color swapchain creation and color image/view/framebuffer setup disabled | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Makepad-created OpenXR color swapchain resources are not required. |
| 24 | Same stripped setup, plus OpenXR wait/begin/end frame loop skipped | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | The active OpenXR frame loop is not required. |
| 25 | Same stripped setup, plus `xrBeginSession` disabled on the READY state | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | A begun OpenXR session is not required. |
| 26 | Makepad Android startup created the Vulkan backend but skipped `xrCreateSession` | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | OpenXR session creation is not required. |
| 27 | Explicit Makepad Vulkan-backend creation for the XR surface was skipped, while earlier activity OpenXR setup still ran | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | The explicit XR-surface backend handoff is not required; an earlier activity/runtime graphics path still reaches the driver. |
| 28 | Makepad OpenXR instance creation, Vulkan backend creation, and session creation were skipped | Direct generated XR activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | Makepad OpenXR instance creation is not required. |
| 29 | Same no-OpenXR-instance diagnostic build, but launched through the normal Makepad Android activity instead of the generated XR activity | Normal launcher activity | App stayed alive; GPU page faults and hardware-buffer warnings returned | The generated XR activity is not required; the lead moves to Makepad's base Android graphics/activity path on Quest / Horizon OS. |
| 30 | Rusty XR Makepad Q2Q source rebuilt as Makepad default Android/GLES-only and launched through the normal activity after runtime permissions were granted | Normal launcher activity | App process stayed alive/visible for a 90s sample; no page-fault-like, small hardware-buffer, or fatal-signature lines were observed; the Q2Q marker did not emit in this default-mode control | Default Android/GLES-only backend selection does not reproduce the fault in this sample, although the missing marker makes this a process/surface-liveness control rather than a full Q2Q startup-marker run. |
| 31 | Upstream Makepad counter example rebuilt as default Android/GLES-only and launched through the normal activity after runtime permissions were granted | Normal launcher activity | App process stayed alive/visible for a 90s sample; no page-fault-like, small hardware-buffer, or fatal-signature lines were observed | A plain non-XR Makepad GLES app also stays clean, strengthening the Quest/Vulkan-backend split. |
| 32 | Upstream Makepad counter example rebuilt with the Quest/Vulkan package shape and launched through the normal activity | Normal launcher activity | App process stayed alive/visible for a 90s sample; GPU page-fault lines were observed against the app process; no fatal process crash was observed | The fault does not require Rusty XR code, `makepad-xr`, generated XR activity launch, or OpenXR session setup; a plain Makepad app with the Quest/Vulkan backend is enough. |
| 33 | Same upstream Makepad counter Quest package shape, but with a temporary local diagnostic build gate suppressing `use_vulkan` so the normal activity used GLES | Normal launcher activity | App process stayed alive/visible for a 90s sample; no page-fault-like, small hardware-buffer, or fatal-signature lines were observed | The Quest manifest/package shape is not sufficient by itself; the active trigger is the Makepad Android Vulkan backend or its surface lifecycle. |
| 34 | Same upstream Makepad counter Quest/Vulkan package shape, but with a temporary local diagnostic build gate returning before Vulkan window draw/present | Normal launcher activity | App process stayed alive/visible for a 90s sample; no page-fault-like, small hardware-buffer, or fatal-signature lines were observed | Vulkan backend and surface setup alone did not reproduce the fault in this sample; the next split is inside the Makepad Vulkan window draw/present path. |
| 35 | Same upstream Makepad counter Quest/Vulkan package shape, but with a temporary local diagnostic gate suppressing swapchain recreation when acquire or present reported suboptimal | Normal launcher activity | App process stayed alive/visible for a 90s sample; no page-fault-like, small hardware-buffer, Vulkan-fault, or fatal-signature lines were observed | Suboptimal-triggered swapchain recreation/destruction is now the strongest bracket inside `draw_pass_and_present`. |
| 36 | Same upstream Makepad counter Quest/Vulkan package shape rebuilt through the same toolchain with the diagnostic suboptimal-recreate suppression absent | Normal launcher activity | App process stayed alive/visible for a 90s sample; GPU page-fault lines returned against the app process; no fatal process crash was observed | The clean attempt 35 result is not explained by broad toolchain, package, or launch drift. The fault class includes both image and pipeline/descriptor/queue-adjacent premature-free signatures. |
| 37 | Same upstream Makepad counter Quest/Vulkan package shape, but with a temporary local diagnostic gate calling device idle immediately before suboptimal-triggered swapchain recreation | Normal launcher activity | App process stayed alive/visible for a 90s sample; no page-fault-like, small hardware-buffer, Vulkan-fault, or fatal-signature lines were observed | Synchronizing before suboptimal-triggered swapchain recreation changes the outcome, pointing at a resource-lifetime race rather than normal Vulkan use in general. |
| 38 | Same upstream Makepad counter Quest/Vulkan package shape, but with a temporary local diagnostic gate waiting Makepad's current window-frame fence before suboptimal-triggered swapchain recreation | Normal launcher activity | App process stayed alive/visible for a 90s sample; no page-fault-like, small hardware-buffer, Vulkan-fault, or fatal-signature lines were observed | A targeted frame-fence wait is enough in this sample; the likely Makepad-side fix candidate is to wait the submitted window frame before destroying/recreating swapchain-owned resources on the suboptimal path. |
| 39 | Same upstream Makepad counter Quest/Vulkan package shape rebuilt from the maintained local Makepad fork state, with the frame-fence wait applied as a persistent source patch and no diagnostic environment gates | Normal launcher activity | App process stayed alive/visible for a 90s sample; no page-fault-like, small hardware-buffer, Vulkan-fault, or fatal-signature lines were observed | The maintained fork state preserves the clean result without relying on temporary diagnostic gates. Next validation should repeat for longer and then move back to the Makepad XR smoke path. |

## Depth Path Comparison

The Makepad path and the non-Makepad Rusty XR depth path differ in several
affordances that matter for this blocker:

- Makepad requests `XR_META_environment_depth` as part of the XR instance setup
  and creates passthrough plus environment-depth provider/swapchain during
  session creation. The public Rusty XR composite example enables the extension
  when available, but creates a provider only when
  `environmentDepthMode != off`.
- Makepad starts the depth provider for the session and attempts
  `xrAcquireEnvironmentDepthImageMETA` every rendered frame. The non-Makepad
  path routes acquisition through `OpenXrEnvironmentDepthProbe`, so the default
  app path keeps depth off and the probe can be switched at runtime.
- Makepad treats non-success depth acquisition as "no image this frame" without
  separating `ENVIRONMENT_DEPTH_NOT_AVAILABLE_META` from harder errors. The
  non-Makepad path tracks unavailable frames, error frames, acquire timing,
  swapchain index, near/far range, and capture timestamps separately.
- Makepad's depth image is made available to XR shader descriptors whenever an
  acquired frame has a swapchain index, and the depth readback/mesh job is a
  framework-level feature. The non-Makepad visualizer only prepares/draws depth
  resources when the selected depth mode visualizes or maps depth.
- Both paths create Vulkan views over Meta environment-depth swapchain images,
  but they use different descriptor assumptions: Makepad binds depth sampled
  images with `DEPTH_STENCIL_READ_ONLY_OPTIMAL`; the non-Makepad visualizer
  currently records its descriptor layout as `SHADER_READ_ONLY_OPTIMAL`.
- The non-Makepad composite path uses a fixed small in-flight frame pipeline and
  waits per-slot fences before reusing render resources. Makepad selects a ready
  in-flight XR frame and can grow that pool. That was a useful earlier suspect,
  but attempts 21-29 show the current page-fault class does not require the
  OpenXR color swapchain or frame loop. Attempts 30-33 then isolate the
  remaining split to Makepad's Android Vulkan backend: default and Quest-shaped
  GLES controls stayed clean, while the same plain app faulted when Quest/Vulkan
  was enabled. Attempt 34 shows backend/surface setup is not sufficient in the
  short sample if Makepad skips Vulkan window draw/present.

The depth diff explains why Makepad was a reasonable suspect: it eagerly turns
on more runtime surfaces than the non-Makepad path. Attempts 14-31 narrow that
further. Depth acquire/readback can change timing, but depth provider creation,
depth swapchain creation, passthrough creation, passthrough layer submission,
projection layer submission, color swapchain creation, OpenXR frame-loop work,
OpenXR session creation, OpenXR instance creation, and the generated XR activity
are not necessary for the page faults. The new split is that Quest/Vulkan-shaped
Makepad Android runs fault, while default and Quest-shaped GLES controls do not.
A Quest/Vulkan no-window-draw control also stayed clean, pushing the next
isolation point into `draw_pass_and_present`.

## Vulkan Window Swapchain Comparison

The non-Makepad Rusty XR Quest examples do use Vulkan, but their working path is
not equivalent to Makepad's Android window WSI path now implicated by attempts
35-38.

- Makepad's normal Android Vulkan path renders into an Android
  `VkSurfaceKHR` swapchain. The frame flow acquires an image, records a window
  render pass, submits to the shared graphics/present queue, calls
  `vkQueuePresentKHR`, and then recreates swapchain-owned resources when acquire
  or present reports suboptimal.
- The non-Makepad Rusty XR Vulkan path renders into OpenXR-owned swapchain
  images. It acquires and waits an OpenXR swapchain image, waits a per-slot
  fence before reusing that slot, submits the frame, releases the image back to
  OpenXR, and lets OpenXR frame submission own presentation to the headset.
- That means "Rusty XR Vulkan works on Quest" remains true, but it does not
  exercise Android `VkSurfaceKHR` WSI presentation or Makepad's
  suboptimal-triggered Android swapchain teardown/recreation path.
- Attempts 35-38 point specifically at Makepad destroying or rebuilding
  swapchain/pipeline/framebuffer-related resources too soon after a submitted
  Android-window frame when the current acquire/present path reports suboptimal.

The next useful Makepad isolation is therefore fork-state validation rather than
another broad renderer split:

- repeat the Quest/Vulkan counter run for a longer sample and a few launch
  cycles
- keep the same wait on the post-submit `ERROR_OUT_OF_DATE_KHR` path and
  validate whether surface update/suspend need additional coverage
- only after that, move the patch back into the Rusty XR Makepad Q2Q shell and
  re-run the XR smoke path

## Open Isolation Questions

- Does a targeted frame-fence wait before suboptimal-triggered
  `recreate_swapchain` stay clean across longer samples and repeated
  launch/stop cycles?
- Does the same synchronization need to be applied to the out-of-date and
  surface-lost paths, or is the Quest/Horizon symptom specific to suboptimal
  acquire/present returns?
- Which exact Makepad resource destruction inside `recreate_swapchain` is racing
  the still-running frame: swapchain image views/framebuffers, depth targets,
  pipelines, descriptor pools, or the old swapchain handle?
- Why did the plain counter Quest/Vulkan fault path stop showing the earlier
  small hardware-buffer warning class while still showing app-process GPU page
  faults?
- What renderer/backend lifecycle difference separates the non-Makepad Rusty XR
  Quest examples, which use custom Android/OpenXR/Vulkan setup, from Makepad's
  generated Quest/Vulkan Android path?
