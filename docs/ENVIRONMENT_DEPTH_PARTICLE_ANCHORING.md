# Environment Depth Particle Anchoring

The Quest composite-layer example now has three useful environment-depth
diagnostic paths:

- a stereo grayscale depth visualizer for validating provider cadence and
  texture orientation
- a generated depth mesh overlay for checking reconstructed surface projection
  against native passthrough
- a retained local-space particle overlay for experimenting with depth-derived
  surface markers

The particle overlay is real progress for live environment-depth mapping: depth
samples are reconstructed into the OpenXR local reference space, retained in a
GPU buffer, and rendered as metric billboards through the current eye poses.
It is not a fullscreen color pass.

## Current Limitation

The current particle overlay is still sourced from a regular view-sampled depth
grid. Even though each sample is written into local scene coordinates, the
active set is continuously refreshed from the headset view. During head motion,
the visible particle pattern can therefore feel view-attached because the
sample lattice itself moves with the headset and replaces older samples before
the scene has an owner-level anchoring policy.

This is different from a scene-owned reconstruction. A scene-owned particle map
should create, update, merge, and retire particles in the environment coordinate
system rather than treating the current view grid as the primary identity of
the particles.

## Follow-Up Design Work

The next implementation should choose an explicit scene map policy before
adding more visual density:

- quantize candidate particles into local-space cells or surface bins
- accumulate confidence across repeated observations of the same cell
- merge overlapping particles instead of drawing every accepted depth sample
- keep particle identity independent from the current headset view
- update only the cells that are currently observable and pass confidence tests
- decay or retire stale particles when observations disagree for long enough
- choose separate scan, display, and confidence resolutions
- keep the distance color ramp, but treat it as particle metadata rather than
  proof that the particle should remain active

The retained particle overlay should remain available as a diagnostic bridge.
It is useful for validating depth cadence, local-space projection, passthrough
composition, and visual encoding. The next production-oriented path should be a
scene-owned particle or sparse surface map that consumes depth observations and
owns particle lifetime explicitly.

## Validation Signals

The live path is considered active when logs report:

- `environmentDepthActive=true`
- `depthMeshProjection=local-space-depth-surface`
- `depthMeshRasterization=retained-local-space-metric-billboard-particles`
- `projection=local-space-retained-particles`
- `rasterization=metric-billboard-particles`
- `passthroughVisible=true`

Headset-motion stability still requires manual in-headset validation. A flat
screen capture can confirm passthrough and surface coverage, but it cannot prove
that particles remain anchored while the headset moves.
