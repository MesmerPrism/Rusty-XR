# Visual Strobe Profiles

Rusty XR includes visual strobe descriptors so public examples can describe and
measure intentionally timed visual stimuli without baking stimulus logic into a
private application shell. The module is `rusty_xr_contracts::strobe`.

These profiles are hazardous by design. They are for explicit, safety-gated
research, diagnostics, and perceptual-tool prototyping. They are not wellness,
therapy, ADHD, imagery-training, entrainment, or medical-treatment claims.

## Safety Warning

Full-field flicker can trigger seizures in people with photosensitive epilepsy
or other visually provoked seizure conditions. It can also cause headache,
nausea, dizziness, migraine, eyestrain, anxiety, or other discomfort in people
who do not have epilepsy.

Do not show the strobing examples by default. Require explicit informed opt-in,
provide a non-strobing exit path, avoid use with children or vulnerable users,
stop immediately if symptoms occur, and keep the headset removable without
obstruction. Do not run these examples around bystanders who have not opted in.

Useful public safety references:

- [Epilepsy Foundation photosensitivity guidance](https://www.epilepsy.com/what-is-epilepsy/seizure-triggers/photosensitivity)
- [W3C WCAG 2.2 Understanding: Three Flashes](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes)

WCAG's general web-content threshold is intentionally conservative: more than
three flashes in any one-second period is outside the normal accessibility
boundary unless specific small-area or low-luminance thresholds apply. The
Rusty XR 10, 40, and 60 Hz examples are deliberate research stimuli and exceed
that general UI boundary.

## Why This Exists

Display-timed flicker is useful for building tools that can test frame cadence,
refresh-rate switching, stimulus scheduling, and subjective visual
phenomenology under controlled conditions. Safety-gated rhythmic audio-visual
tools are a real design space for non-clinical altered-perception exploration,
but they need clear warnings and conservative launch defaults.

Rusty XR keeps this lower-level. It provides descriptors, timing analysis, and
example launch profiles. It does not provide a product experience, private
effect stack, efficacy claim, or medical protocol.

## Public Module

`rusty_xr_contracts::strobe` exports:

- `VisualStrobeProfile`: source mode, full A/B cycle frequency, duty cycle,
  optional display-refresh request, and safety class.
- `VisualStrobeMode`: full-field color alternation or passthrough LUT
  alternation.
- `StrobeFrequencyPlan`: target cycle rate, switch rate, frames per cycle,
  frames per half-cycle, and display-frame feasibility.
- `VisualStrobeSafetyClass`: non-strobing, accessibility-bounded, or
  research-stimulus.
- `XR_FB_DISPLAY_REFRESH_RATE_EXTENSION`: adapter hint for requesting runtime
  display refresh through OpenXR.

The module is data-only. It does not draw frames, create OpenXR handles,
request refresh rates, drive the headset display, or create a safety UI.

## Frequency And Refresh Behavior

The example uses "Hz" as full A/B cycles per second. Each cycle contains both
states. A red/black 40 Hz profile therefore needs 80 state transitions per
second.

| Target | At 120 Hz display refresh | Notes |
| --- | --- | --- |
| 10 Hz | 12 frames per cycle, 6 frames per half-cycle | Clean integer frame timing, but inside the common photosensitive-risk band cited by epilepsy guidance. |
| 40 Hz | 3 frames per cycle, 1.5 frames per half-cycle | The average switch rate is possible only with frame quantization, such as alternating one- and two-frame half-cycles. |
| 60 Hz | 2 frames per cycle, 1 frame per half-cycle | Requires toggling every displayed frame. Any missed frame, runtime fallback, reprojection, or actual refresh below 120 Hz changes the stimulus. |

At 72 Hz, 40 Hz and 60 Hz full cycles are not representable as a clean
full-field A/B square wave because they require 80 and 120 transitions per
second respectively. A 10 Hz profile can run at 72 Hz only with non-integer
3.6-frame half-cycles, so the realized pattern is quantized rather than evenly
timed.

`XR_FB_display_refresh_rate` lets an app ask the runtime for a supported
display refresh rate, but the request is not a guarantee. A validating adapter
should log the enumerated supported rates, the request result, refresh-rate
change events, current active display refresh, observed OpenXR frame cadence,
and stimulus switch statistics.

The Quest composite example logs `full-field flicker stats` and `passthrough
LUT flicker stats` from the OpenXR frame loop. In local 120 Hz validation, the
full-field red/black profiles measured approximately 10, 40, and 60 full cycles
per second after refresh-rate transition. The 60 Hz case is the most fragile
because it consumes one state change per frame.

## Passthrough LUT Flicker

The passthrough version alternates between two native
`XR_META_passthrough_color_lut` handles: a public opponent-color LUT and a
half-phase-inverted LUT. That path changes Meta-provided compositor parameters;
it still does not expose the lower compositor image as a sampleable app texture
or let Rusty XR run arbitrary code inside the Meta compositor.

Use the passthrough LUT profiles when testing compositor-native color style
updates. Use the full-field red/black profiles when testing the app's own
projection-layer timing without passthrough in the path.

## Running The Contract Example

```powershell
cargo run -p rusty-xr-contracts --example visual_strobe_profiles --features serde
```

The example prints JSON descriptors for 10, 40, and 60 Hz full-field and
passthrough-LUT profiles, plus their 120 Hz timing plans. It does not require a
headset.
