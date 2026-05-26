# Makepad Strategy For Rusty XR

Last public review: 2026-05-27.

This note explains what Makepad is, why Rusty XR uses it, and what kinds of
Rusty XR work may be useful to Makepad when generalized. It is a public
strategy document, not a fork audit or downstream app plan.

## Summary

Makepad is a Rust-native application stack: a custom GPU-first UI/runtime
framework, widget layer, Studio tooling, live design language, and
cross-platform build tooling. Its current public site frames Makepad 2.0 around
AI-native Rust UIs and Splash, a readable live UI language intended to connect
native Rust application development with AI-assisted interface generation.

Rusty XR uses Makepad as a promising app-shell and XR-panel substrate, not as
the source of truth for camera-resource semantics. Makepad is strong where XR
apps need rich Rust UI: panels, controls, diagnostic surfaces, live iteration,
text, layout, and tool-like interfaces. Rusty XR still keeps the raw camera,
projection, timing, resource-lifetime, and validation contracts explicit and
framework-neutral.

The maintained Makepad branch has started converging back toward upstream
instead of treating Rusty XR as a long-lived isolated fork. As of the May 2026
alignment pass, it includes selected upstream video lifecycle, Android
rendering, text wrapping, Android lifecycle, manifest/tooling, minimum-SDK, and
Android App Bundle packaging work while preserving the Rusty XR camera-lane
diagnostic contracts.

The intended relationship is:

```text
Rusty XR public contracts
  -> direct HWB / Vulkan examples
  -> direct OES / OpenGL examples
  -> Makepad-first examples and adapters
       -> Makepad runtime and widgets
```

Rusty XR core crates must not depend on Makepad. Makepad-specific code belongs
in examples, optional adapters, or a maintained Makepad branch while the
interfaces are still proving out.

## Current Alignment Status

The Makepad branch used by Rusty XR is no longer only a renderer experiment.
It is also an integration branch where upstream Makepad platform changes are
tested against stricter headset camera and XR requirements.

Recent alignment work includes:

| Area | Integrated direction | Rusty XR reason |
| --- | --- | --- |
| Video widget lifecycle | Upstream video apply-gate behavior plus Rusty XR texture update metadata. | Keep ordinary Makepad video behavior compatible while exposing frame/update facts needed for camera-lane contracts. |
| Android rendering and loading | Upstream Android rendering/loading fixes, including text/render target related surfaces. | Keep Makepad XR panels and Android rendering quality aligned with upstream before attributing issues to Rusty XR camera code. |
| Text wrapping | Upstream text wrapping correction. | Avoid carrying a stale UI/text fork while using Makepad for headset panels. |
| Android lifecycle | Shutdown and activity lifecycle safety work. | XR, camera, and external-resource paths need reliable pause/resume and resource invalidation behavior. |
| Android tooling | Manifest input reconciliation, lower minimum-SDK defaults, upstream-aligned tooling defaults, and AAB packaging path. | Keep Rusty XR's deterministic APK lane close to upstream `cargo-makepad` instead of growing a competing Android packager. |
| Frame-flow diagnostics | Throttled and cadence-tuned XR submit/frame-flow markers. | Provide evidence for stale-frame and resource-lifecycle debugging without making diagnostics the renderer architecture. |

Remaining upstream-diff risk is concentrated in Android tooling, platform
lifecycle, video widgets, text/DPI surfaces, and graphics resource binding.
Rusty XR should re-check upstream before deeper Makepad shader or descriptor
changes.

## What Makepad Is

Makepad is not a webview wrapper and not a thin native-widget binding. Its
public materials describe a Rust-first UI framework with:

- GPU-accelerated 2D/3D rendering.
- A cross-platform runtime and platform abstraction layer.
- A widget library and design language.
- Makepad Studio as an editor and development environment.
- `cargo-makepad` tooling for nonstandard targets such as mobile and web.
- Splash / Live-style UI descriptions for fast iteration and AI-assisted UI
  generation.

Historically, Makepad also had an explicit live-coding and VR/XR ambition. The
current public positioning is broader: native Rust apps, Studio, Splash, and
AI-assisted UI development. For Rusty XR, that means Makepad should be treated
as a general Rust UI/application framework with XR affordances rather than as a
Quest-camera-specific renderer.

## Why Rusty XR Uses Makepad

Rusty XR benefits from Makepad in the app-shell layer:

| Need | Why Makepad is relevant |
| --- | --- |
| Rich XR controls | Makepad widgets can provide tool panels, inspectors, settings, diagnostics, and control surfaces without adopting a web stack. |
| Crisp panel rendering | Makepad's GPU text/vector/widget path is a better fit for headset panels than a low-resolution canvas texture pasted into a scene. |
| Rust-native app structure | Rusty XR can keep app shell, UI state, and diagnostics in Rust rather than splitting core logic from a separate UI language/runtime. |
| Cross-platform iteration | Makepad can support desktop/mobile preview and headset-targeted flows from a shared framework. |
| Live UI direction | Live/Splash-style UI descriptions can make diagnostic panels and operator tools faster to iterate once the runtime contracts are stable. |

Makepad is especially useful for:

- XR operator panels.
- Camera/profile controls.
- Diagnostic dashboards.
- Graph and timeline views.
- In-headset development tools.
- Framework comparison lanes.

## What Rusty XR Does Not Delegate To Makepad

Rusty XR keeps these contracts explicit and framework-neutral:

- Camera frame identity and timestamps.
- Hardware-buffer, OES texture, and CPU-YUV transport identity.
- Acquire, upload/import, reuse, stale-frame, and release lifecycle facts.
- Camera crop, transform, projection, and stereo lane semantics.
- Color model and color-correction status.
- XR frame timing and submission evidence.
- Device-gate summaries and validation artifacts.
- Public schemas and deterministic host-side tests.

This split matters because a normal UI/video texture abstraction is usually too
coarse for headset camera projection. Rusty XR needs to answer which physical
or decoded frame reached which eye, through which resource path, with which
transform and timing. That remains true even when Makepad owns the surrounding
UI.

## What Rusty XR Can Offer Makepad Upstream

Rusty XR should contribute ideas upstream only after stripping downstream
assumptions. The most promising Makepad-level contributions are generic
resource, lifecycle, Android, and XR-panel improvements, not product-specific
markers.

| Rusty XR pressure | Makepad-level contribution candidate | Upstream value |
| --- | --- |
| External camera/video textures | A general external frame/resource descriptor with backend kind, dimensions, format, color space, transform, timestamp, validity, and lifecycle state. | Helps Android camera, video, decoder, XR, and future external-texture integrations without hardcoding a Quest path. |
| Camera frame freshness | Platform video lifecycle events that express acquire, update, reuse, stale, release, and producer timestamp facts. | Makes Makepad video widgets observable enough for real-time media apps and diagnostics. |
| Android and XR lifecycle | Hooks for activity/session state, permissions, surface recreation, pause/resume, shutdown, and resource invalidation. | Reduces fragile app-specific lifecycle work in downstream Android and XR applications. |
| Android packaging | Source-root deterministic builds, manifest input normalization, min/target SDK clarity, AAB support, and transcript ordering. | Makes `cargo-makepad` more predictable for CI, mobile distribution, and external app teams. |
| GPU backend parity | Reflected shader-resource interfaces and descriptor-shape diagnostics that remain independent of any one app. | Gives Makepad a safer path for external resources, YCbCr samplers, and backend-specific descriptor layouts. |
| XR panel quality | DPI, text, scaling, input, focus, and rendering-quality checks for panels used inside XR scenes. | Improves Makepad's XR and high-DPI mobile UI story beyond Rusty XR. |
| Diagnostic vocabulary | Generic overlay hooks for frame/resource status, timing, backend identity, and freshness. | Lets Makepad expose performance/resource facts without every app inventing private log markers. |

The most useful upstream contribution shape is a sequence of small, generic
packets:

1. Video texture update metadata and lifecycle events.
2. External frame/resource descriptors.
3. Android lifecycle and packaging hardening.
4. Shader-resource reflection and descriptor diagnostics.
5. XR panel quality and DPI/text fixes.

Each packet should be expressed in Makepad terms first. Rusty XR can provide a
stress test and validation harness, but the API should still make sense for a
normal Makepad media, Android, desktop, or XR application.

Keep these downstream unless Makepad has a generic abstraction that can carry
the same invariants:

- Quest-specific camera and permission behavior.
- Device-specific stale-frame gates.
- Raw projection and stereo camera-lane naming.
- Passthrough runtime policy.
- App-specific validation profiles.
- Generated device evidence.

## Strategy

Rusty XR should keep Makepad integration modular:

```text
Shared camera texture lane contract
  -> Direct Vulkan/HWB adapter
  -> Direct OpenGL/OES adapter
  -> Makepad CPU-YUV adapter
  -> Makepad hardware-buffer/external-texture adapter
```

The Makepad adapters should translate Makepad events and resources into the
same Rusty XR contract used by the direct renderers. They should not require
Rusty XR core crates to depend on Makepad, and they should not hide camera
resource facts behind path labels or renderer-specific markers.

For upstream collaboration, prefer small packets:

1. Android tooling and lifecycle compatibility.
2. Video/resource metadata and lifecycle events.
3. External texture or hardware-buffer descriptors.
4. Shader-resource reflection and backend descriptor diagnostics.
5. XR panel text, DPI, scaling, focus, and input quality.

Each packet should describe the Makepad-level problem first. Rusty XR can be a
stress case, but the proposed interface should be useful to ordinary Makepad
apps.

Before changing deeper Makepad shader or descriptor behavior, Rusty XR should
first check whether the relevant upstream Android, video, text, and packaging
changes have been integrated. That keeps renderer experiments from compensating
for an old platform baseline.

## Public Evidence

The public Makepad ecosystem appears small but technically serious. The strongest
public signals are Makepad Studio, Robrix / Project Robius, Moly, Moxin Studio,
the Makepad Book, published crates, and the active Makepad repository. These
show a real framework and app ecosystem, while also showing that documentation,
platform maturity, and production adoption are still evolving.

Rusty XR should therefore treat Makepad as a valuable partner stack with active
upstream movement, not as a stable black box. The integration policy is to keep
contracts public and framework-neutral, keep Makepad-specific code adapter-owned,
and regularly re-check upstream changes before deeper renderer work.

## Sources

Sources accessed on 2026-05-26:

- [Makepad official site](https://makepad.nl/)
- [Makepad history](https://makepad.nl/history.html)
- [Makepad GitHub repository](https://github.com/makepad/makepad)
- [Makepad Book architecture](https://makepad.rs/guide/start/makepad-framework-architecture)
- [Makepad Book introduction](https://makepad.rs/guide/start/introduction)
- [Makepad historical repository](https://github.com/makepad/makepad_history)
- [Robrix FOSDEM 2025 session](https://archive.fosdem.org/2025/schedule/event/fosdem-2025-5841-robrix-a-pure-rust-multi-platform-matrix-client-and-more/)
- [Project Robius 2024 retrospective](https://robius.rs/blog/robius-retrospective-2024/)
- [Moly AI repository](https://github.com/moly-ai/moly-ai)
- [Moxin Studio repository](https://github.com/moxin-org/Moxin-Studio)
