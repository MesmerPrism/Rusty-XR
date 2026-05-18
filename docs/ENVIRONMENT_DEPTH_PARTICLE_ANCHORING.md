# Environment Depth Particle Anchoring

The Quest composite-layer example now has four useful environment-depth
diagnostic paths:

- a stereo grayscale depth visualizer for validating provider cadence and
  texture orientation
- a generated depth mesh overlay for checking reconstructed surface projection
  against native passthrough
- a retained local-space particle overlay for experimenting with depth-derived
  surface markers
- a scene-owned particle map for testing persistent local-space scan markers

The particle overlay is real progress for live environment-depth mapping: depth
samples are reconstructed into the OpenXR local reference space, retained in a
GPU buffer, and rendered as metric billboards through the current eye poses.
It is not a fullscreen color pass.

For the shared coordinate vocabulary that relates this world-space-first path
to direct per-eye camera projection and later blur diagnostics, see
[PROJECTION_COORDINATE_SPACE_LEDGER.md](PROJECTION_COORDINATE_SPACE_LEDGER.md).

## Retained Overlay Limitation

The `particle-overlay` mode is still sourced from a regular view-sampled depth
grid. Even though each sample is written into local scene coordinates, the
active set is continuously refreshed from the headset view. During head motion,
the visible particle pattern can therefore feel view-attached because the sample
lattice itself moves with the headset and replaces older samples before the
scene has an owner-level anchoring policy.

This is different from a scene-owned reconstruction. A scene-owned particle map
should create, update, merge, and retire particles in the environment coordinate
system rather than treating the current view grid as the primary identity of
the particles.

## Scene Particle Map

The `scene-particle-map` mode is the first scene-owned version. It still takes
candidate observations from the live depth texture, but particle identity is
based on quantized OpenXR local-space cells rather than screen-raster slots.
Each accepted depth sample reconstructs a local-space position, hashes its
metric cell, probes a small neighborhood of particle slots, and then either
creates a new particle, confidence-blends an existing particle in the same
cell, or replaces a stale particle.
Invalid candidate samples do not clear arbitrary particle slots in this mode;
they leave existing cells alone so lifetime is owned by local-space age/fade
rather than by the current headset raster. High-confidence current samples also
actively correct visible free space: cells on the local-space ray in front of
the observed surface are retired, while cells at or behind the current surface
are preserved because they may be occluded rather than wrong.

The headset-motion fix has two separate parts:

- render and depth poses are composed from `VIEW` space into the stable app
  reference space before they are used for mapping or drawing
- the environment-depth mesh and particle shaders fold the Vulkan positive
  viewport Y convention into their manual OpenXR FOV projection, matching the
  known-good particle renderer path

This is intentionally a visual map, not a CPU point-cloud export, TSDF volume,
or mesh reconstruction. The goal is to make headset-motion validation possible:
previously observed particles should stay attached to the room while new
observations fill or refresh nearby cells. Stale cells fade and retire, and
high-confidence visible free-space evidence can clear stale foreground cells so
the map can recover when the runtime depth surface changes without becoming
headset-raster-owned again.

## Particle Rendering

The depth particle renderer uses metric billboards in the current render eye
view, but the particle centers stay in the OpenXR local reference space. For
the scene particle map, the visible particles are intentionally small
alpha-clipped opaque default discs:

- half-size range: `0.002..0.004` meters
- mask: `default-disc`
- opacity policy: `alpha-clipped-opaque`

The smaller opaque discs reduce the low-alpha cloud look while keeping the
map readable as a real-time scan surface over native passthrough. The distance
color ramp remains metadata from the current depth sample, not evidence that a
particle should stay active forever.

## Follow-Up Design Work

The next implementation should tune the explicit scene map policy before adding
more visual density:

- tune active cell correction thresholds for live Quest depth: the current
  policy clears only visible free-space cells in front of a confident surface
  and preserves cells behind that surface
- tune local-space cell size and probe count for Quest depth noise
- make merge confidence account for observation angle and depth confidence when
  the runtime exposes confidence payloads
- update only the cells that are currently observable and pass confidence tests
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
- `projection=local-space-scene-particle-map`
- `mapPolicy=spatial-hash-local-cells`
- `invalidSamplePolicy=preserve-existing-cells`
- `activeCorrectionPolicy=visible-free-space-ray-clear`
- `occlusionPolicy=preserve-behind-current-depth`
- `particleHalfSizeMeters=0.002..0.004`
- `particleMask=default-disc`
- `particleOpacity=alpha-clipped-opaque`
- `depthPoseSource=view-space-composed`
- `projectionYConvention=vulkan-positive-viewport-y-flipped-in-shader`
- `rasterization=metric-billboard-particles`
- `passthroughVisible=true`

Headset-motion stability still requires manual in-headset validation. A flat
screen capture can confirm passthrough and surface coverage, but it cannot prove
that particles remain anchored while the headset moves.
