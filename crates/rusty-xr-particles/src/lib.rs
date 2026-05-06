//! General particle and animation primitives for Rusty XR.
//!
//! This crate intentionally starts with simple, deterministic primitives. It
//! does not include downstream simulation behavior, app scenes, or renderer
//! backend code.
//! It also includes stable coordinate sampling for dynamic triangle meshes:
//! providers can update deformed vertices every frame while keeping sampled
//! triangle/barycentric anchors and neighborhood identity stable.
//! Source-only SDF attraction helpers are included as a public example consumer
//! of dynamic mesh fields; higher-level simulation behavior stays downstream.
//!
//! Enable the `serde` feature to serialize particle buffers and fixed-step
//! runtime state for fixtures or operator tooling.

use std::collections::{HashMap, HashSet};

pub use rusty_xr_contracts::{
    ColorRgba, HandMeshError, HandMeshSnapshot, Handedness, RenderCoordinateSpace, RenderPayload,
    RenderPoint, RuntimeCounters, Vec3,
};
pub use rusty_xr_sdf::{
    build_sdf_from_mesh, build_sdf_from_mesh_bounds,
    triangle_mesh_snapshot_from_hand_mesh_snapshot, Bounds3, MeshSdfSignMode, MeshToSdfConfig,
    MeshToSdfConfigError, MeshToSdfError, PackedSdfGrid, PackedSdfSample, SdfSampleMode,
    TriangleMeshSnapshot,
};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_ICOSPHERE_RECURSION_LEVEL: u32 = 4;
pub const DEFAULT_PARTICLE_DISC_SEGMENTS: usize = 12;
pub const DEFAULT_ANIMATED_RING_FRAME_COUNT: usize = 64;
pub const DEFAULT_ANIMATED_RING_FRAME_RESOLUTION: usize = 128;
pub const DEFAULT_ANIMATED_RING_ATLAS_COLUMNS: usize = 8;

/// One particle state record for low-count CPU tests and payload generation.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius_meters: f32,
    pub inverse_mass: f32,
    pub color: ColorRgba,
    pub flags: u32,
}

impl ParticleState {
    pub fn new(position: Vec3, radius_meters: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            radius_meters,
            inverse_mass: 1.0,
            color: ColorRgba::WHITE,
            flags: 0,
        }
    }

    pub fn is_valid(self) -> bool {
        self.position.is_finite()
            && self.velocity.is_finite()
            && self.radius_meters.is_finite()
            && self.radius_meters >= 0.0
            && self.inverse_mass.is_finite()
            && self.inverse_mass >= 0.0
            && self.color.is_finite()
    }
}

/// Renderer-oriented particle record for point sprites, billboards, or custom
/// particle draw paths.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParticleRender {
    pub position: Vec3,
    pub size_meters: f32,
    pub color: ColorRgba,
    pub normal: Vec3,
    pub flags: u32,
    pub rotation_radians: f32,
    pub frame01: f32,
    pub aux0: f32,
    pub aux1: f32,
}

impl ParticleRender {
    pub fn new(position: Vec3, size_meters: f32, color: ColorRgba) -> Self {
        Self {
            position,
            size_meters,
            color,
            normal: Vec3::UP,
            flags: 0,
            rotation_radians: 0.0,
            frame01: 0.0,
            aux0: 1.0,
            aux1: 0.0,
        }
    }

    pub fn is_valid(self) -> bool {
        self.position.is_finite()
            && self.size_meters.is_finite()
            && self.size_meters >= 0.0
            && self.color.is_finite()
            && self.normal.is_finite()
            && self.rotation_radians.is_finite()
            && self.frame01.is_finite()
            && self.aux0.is_finite()
            && self.aux1.is_finite()
    }

    pub fn render_point(self) -> RenderPoint {
        let mut point = RenderPoint::new(self.position, self.size_meters * 0.5, self.color);
        point.normal = self.normal;
        point.flags = self.flags;
        point
    }
}

/// Structure-of-arrays particle storage for public experiments.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParticleSet {
    pub positions: Vec<Vec3>,
    pub velocities: Vec<Vec3>,
    pub radii_meters: Vec<f32>,
    pub inverse_masses: Vec<f32>,
    pub colors: Vec<ColorRgba>,
    pub flags: Vec<u32>,
}

impl ParticleSet {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            velocities: Vec::with_capacity(capacity),
            radii_meters: Vec::with_capacity(capacity),
            inverse_masses: Vec::with_capacity(capacity),
            colors: Vec::with_capacity(capacity),
            flags: Vec::with_capacity(capacity),
        }
    }

    pub fn seed_sphere(
        count: usize,
        center: Vec3,
        radius_meters: f32,
        particle_radius: f32,
    ) -> Self {
        let mut particles = Self::with_capacity(count);
        if count == 0 {
            return particles;
        }

        let radius_meters = radius_meters.max(0.0);
        let golden_angle = 2.399_963_1_f32;
        for index in 0..count {
            let unit_index = (index as f32 + 0.5) / count as f32;
            let z = 1.0 - (2.0 * unit_index);
            let azimuth = index as f32 * golden_angle;
            let planar = (1.0 - (z * z)).max(0.0).sqrt();
            let direction = Vec3::new(azimuth.cos() * planar, z, azimuth.sin() * planar);
            let radial = hash01(index as u32).powf(1.0 / 3.0);
            particles.push_state(ParticleState::new(
                center + (direction * (radius_meters * radial)),
                particle_radius.max(0.0),
            ));
        }

        particles
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn push_state(&mut self, state: ParticleState) {
        self.positions.push(state.position);
        self.velocities.push(state.velocity);
        self.radii_meters.push(state.radius_meters);
        self.inverse_masses.push(state.inverse_mass);
        self.colors.push(state.color);
        self.flags.push(state.flags);
    }

    pub fn render_particles(&self) -> Vec<ParticleRender> {
        let mut payload = Vec::with_capacity(self.len());
        for index in 0..self.len() {
            let mut particle = ParticleRender::new(
                self.positions[index],
                self.radii_meters[index] * 2.0,
                self.colors[index],
            );
            particle.flags = self.flags[index];
            particle.normal = self.positions[index].normalized_or(Vec3::UP);
            payload.push(particle);
        }
        payload
    }

    pub fn validate_layout(&self) -> bool {
        let len = self.positions.len();
        self.velocities.len() == len
            && self.radii_meters.len() == len
            && self.inverse_masses.len() == len
            && self.colors.len() == len
            && self.flags.len() == len
    }

    pub fn render_payload(
        &self,
        frame_index: u64,
        coordinate_space: RenderCoordinateSpace,
    ) -> RenderPayload {
        render_particles_to_payload(frame_index, coordinate_space, &self.render_particles())
    }
}

/// SDF interaction mode for the small public particle attraction stepper.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SdfParticleAttractionMode {
    None,
    AttractToSurface,
    GaussianSurfaceAttract,
}

/// Configuration for attracting simple CPU particles toward an SDF surface.
///
/// This is a dependency-light reference helper for examples and adapters. It
/// does not own threading, GPU kernels, renderer state, or app-specific
/// simulation behavior.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdfParticleAttractionConfig {
    pub mode: SdfParticleAttractionMode,
    pub sample_mode: SdfSampleMode,
    pub strength: f32,
    pub attraction_distance_meters: f32,
    pub gaussian_sigma_meters: f32,
    pub normal_damping: f32,
    pub drag: f32,
    pub max_speed_meters_per_second: f32,
    pub max_extrapolation_meters: f32,
}

impl Default for SdfParticleAttractionConfig {
    fn default() -> Self {
        Self {
            mode: SdfParticleAttractionMode::AttractToSurface,
            sample_mode: SdfSampleMode::Trilinear,
            strength: 5.0,
            attraction_distance_meters: 1.5,
            gaussian_sigma_meters: 0.08,
            normal_damping: 0.35,
            drag: 0.04,
            max_speed_meters_per_second: 1.4,
            max_extrapolation_meters: 0.0,
        }
    }
}

/// Summary from one CPU SDF attraction step.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SdfParticleAttractionStepStats {
    pub sampled_count: usize,
    pub affected_count: usize,
    pub max_speed_observed: f32,
}

/// Step particles with a simple SDF surface attraction force.
pub fn step_particles_toward_sdf(
    particles: &mut ParticleSet,
    sdf: &PackedSdfGrid,
    delta_seconds: f32,
    config: SdfParticleAttractionConfig,
) -> SdfParticleAttractionStepStats {
    if config.mode == SdfParticleAttractionMode::None
        || !delta_seconds.is_finite()
        || delta_seconds <= 0.0
        || !particles.validate_layout()
    {
        return SdfParticleAttractionStepStats::default();
    }

    let mut stats = SdfParticleAttractionStepStats::default();
    for index in 0..particles.len() {
        let position = particles.positions[index];
        let velocity = particles.velocities[index];
        let Some(sample) = sample_sdf_for_attraction(position, sdf, config) else {
            continue;
        };
        stats.sampled_count += 1;

        let normal = sample.normal.normalized_or(Vec3::UP);
        let distance = sample.distance_meters;
        let acceleration = sdf_attraction_acceleration(distance, normal, velocity, config);
        if acceleration.length_squared() > 1.0e-12 {
            stats.affected_count += 1;
        }

        let mut next_velocity = velocity + (acceleration * delta_seconds);
        let drag_multiplier = (1.0 - (config.drag.max(0.0) * delta_seconds)).clamp(0.0, 1.0);
        next_velocity *= drag_multiplier;
        next_velocity = clamp_particle_speed(next_velocity, config.max_speed_meters_per_second);
        particles.positions[index] = position + (next_velocity * delta_seconds);
        particles.velocities[index] = next_velocity;
        stats.max_speed_observed = stats.max_speed_observed.max(next_velocity.length());
    }

    stats
}

/// Camera pose helper for source-only SDF attraction scenarios.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
}

impl CameraPose {
    pub const fn new(position: Vec3, forward: Vec3, up: Vec3) -> Self {
        Self {
            position,
            forward,
            up,
        }
    }

    pub fn forward_for_spawn(self, yaw_only: bool) -> Vec3 {
        if !yaw_only {
            return self.forward.normalized_or(Vec3::FORWARD_NEG_Z);
        }

        let up = self.up.normalized_or(Vec3::UP);
        let flattened = self.forward - (up * self.forward.dot(up));
        flattened.normalized_or(self.forward.normalized_or(Vec3::FORWARD_NEG_Z))
    }
}

/// Places a deterministic particle sphere in front of a camera/head pose.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleSphereSpawnConfig {
    pub count: usize,
    pub distance_meters: f32,
    pub radius_meters: f32,
    pub particle_radius_meters: f32,
    pub vertical_offset_meters: f32,
    pub yaw_only: bool,
}

impl Default for ParticleSphereSpawnConfig {
    fn default() -> Self {
        Self {
            count: 4096,
            distance_meters: 1.25,
            radius_meters: 0.28,
            particle_radius_meters: 0.006,
            vertical_offset_meters: 0.0,
            yaw_only: true,
        }
    }
}

impl ParticleSphereSpawnConfig {
    pub fn center_for_camera(self, camera: CameraPose) -> Vec3 {
        let forward = camera.forward_for_spawn(self.yaw_only);
        camera.position
            + (forward * self.distance_meters.max(0.0))
            + (camera.up.normalized_or(Vec3::UP) * self.vertical_offset_meters)
    }

    pub fn spawn_particles(self, camera: CameraPose) -> ParticleSet {
        ParticleSet::seed_sphere(
            self.count,
            self.center_for_camera(camera),
            self.radius_meters.max(0.0),
            self.particle_radius_meters.max(0.0),
        )
    }
}

/// Scenario builder config for particles attracted to a dynamic mesh SDF.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshSdfParticleAttractionScenarioConfig {
    pub spawn: ParticleSphereSpawnConfig,
    pub mesh_sdf: MeshToSdfConfig,
    pub attraction: SdfParticleAttractionConfig,
    pub bounds_padding_meters: f32,
}

impl Default for MeshSdfParticleAttractionScenarioConfig {
    fn default() -> Self {
        Self {
            spawn: ParticleSphereSpawnConfig::default(),
            mesh_sdf: MeshToSdfConfig::default(),
            attraction: SdfParticleAttractionConfig {
                mode: SdfParticleAttractionMode::AttractToSurface,
                ..SdfParticleAttractionConfig::default()
            },
            bounds_padding_meters: 0.25,
        }
    }
}

/// Initial particle set plus a packed SDF built from a mesh snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshSdfParticleAttractionScenario {
    pub attraction_config: SdfParticleAttractionConfig,
    pub particles: ParticleSet,
    pub sdf: PackedSdfGrid,
    pub particle_spawn_center: Vec3,
    pub sdf_bounds: Bounds3,
    pub simulation_bounds: Bounds3,
}

pub fn spawn_particle_sphere_in_front_of_camera(
    camera: CameraPose,
    spawn: ParticleSphereSpawnConfig,
) -> ParticleSet {
    spawn.spawn_particles(camera)
}

pub fn build_mesh_sdf_particle_attraction_scenario(
    version: u64,
    camera: CameraPose,
    mesh: &TriangleMeshSnapshot,
    config: MeshSdfParticleAttractionScenarioConfig,
) -> Result<MeshSdfParticleAttractionScenario, MeshToSdfError> {
    let spawn_center = config.spawn.center_for_camera(camera);
    let particles = config.spawn.spawn_particles(camera);
    let mesh_bounds = mesh.bounds().ok_or(MeshToSdfError::EmptyMesh)?;
    let sdf_bounds = mesh_bounds.include_sphere(spawn_center, config.spawn.radius_meters);
    let sdf = build_sdf_from_mesh_bounds(version, mesh, config.mesh_sdf, sdf_bounds)?;

    let simulation_bounds = sdf_bounds.expanded(config.bounds_padding_meters);
    let mut attraction_config = config.attraction;
    attraction_config.mode = SdfParticleAttractionMode::AttractToSurface;
    attraction_config.attraction_distance_meters = attraction_config
        .attraction_distance_meters
        .max(config.spawn.radius_meters + config.mesh_sdf.padding_meters);

    Ok(MeshSdfParticleAttractionScenario {
        attraction_config,
        particles,
        sdf,
        particle_spawn_center: spawn_center,
        sdf_bounds,
        simulation_bounds,
    })
}

/// Convert rich particle render records into the public point payload contract.
pub fn render_particles_to_payload(
    frame_index: u64,
    coordinate_space: RenderCoordinateSpace,
    particles: &[ParticleRender],
) -> RenderPayload {
    let mut payload = RenderPayload::new(frame_index, coordinate_space);
    payload.points.reserve(particles.len());
    for particle in particles {
        payload.points.push(particle.render_point());
    }
    payload
        .counters
        .push_count("particle_count", particles.len() as u64);
    payload
}

/// Triangle mesh that can be sampled into surface-locked particle coordinates.
///
/// This is intentionally framework-neutral: app shells can adapt native hand,
/// controller, room, or scanned meshes into this shape without pulling engine
/// code into the public crate.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TriangleMeshSurface {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[usize; 3]>,
}

/// Errors when adapting public hand mesh snapshots into particle surfaces.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriangleMeshSurfaceError {
    InvalidHandMesh(HandMeshError),
    IndexDoesNotFitUsize(u32),
    IndexDoesNotFitU32(usize),
}

impl TriangleMeshSurface {
    pub fn new(vertices: Vec<Vec3>, triangles: Vec<[usize; 3]>) -> Self {
        Self {
            vertices,
            triangles,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    pub fn surface_area(&self) -> f32 {
        self.triangles
            .iter()
            .filter_map(|triangle| triangle_area(self.vertices.as_slice(), *triangle))
            .sum()
    }

    pub fn is_valid(&self) -> bool {
        !self.vertices.is_empty()
            && !self.triangles.is_empty()
            && self.vertices.iter().all(|vertex| vertex.is_finite())
            && self
                .triangles
                .iter()
                .all(|triangle| triangle.iter().all(|index| *index < self.vertices.len()))
            && self.surface_area() > 1.0e-9
    }

    pub fn sample_even_points(&self, config: MeshSurfaceSampleConfig) -> MeshSurfaceSampleSet {
        sample_mesh_surface_points(self, config)
    }

    pub fn from_hand_mesh_snapshot(
        snapshot: &HandMeshSnapshot,
    ) -> Result<Self, TriangleMeshSurfaceError> {
        triangle_mesh_surface_from_hand_mesh_snapshot(snapshot)
    }

    pub fn to_triangle_mesh_snapshot(
        &self,
        version: u64,
    ) -> Result<TriangleMeshSnapshot, TriangleMeshSurfaceError> {
        triangle_mesh_snapshot_from_surface(version, self)
    }
}

/// Convert a framework-neutral hand mesh snapshot into the particle mesh surface
/// used for sampling and live deformed-mesh updates.
pub fn triangle_mesh_surface_from_hand_mesh_snapshot(
    snapshot: &HandMeshSnapshot,
) -> Result<TriangleMeshSurface, TriangleMeshSurfaceError> {
    snapshot
        .validate()
        .map_err(TriangleMeshSurfaceError::InvalidHandMesh)?;

    let mut triangles = Vec::with_capacity(snapshot.indices.len());
    for triangle in snapshot.indices.iter().copied() {
        let a = usize::try_from(triangle[0])
            .map_err(|_| TriangleMeshSurfaceError::IndexDoesNotFitUsize(triangle[0]))?;
        let b = usize::try_from(triangle[1])
            .map_err(|_| TriangleMeshSurfaceError::IndexDoesNotFitUsize(triangle[1]))?;
        let c = usize::try_from(triangle[2])
            .map_err(|_| TriangleMeshSurfaceError::IndexDoesNotFitUsize(triangle[2]))?;
        triangles.push([a, b, c]);
    }

    Ok(TriangleMeshSurface::new(
        snapshot.vertices.clone(),
        triangles,
    ))
}

/// Convert a particle mesh surface into the SDF crate's mesh snapshot shape.
pub fn triangle_mesh_snapshot_from_surface(
    version: u64,
    surface: &TriangleMeshSurface,
) -> Result<TriangleMeshSnapshot, TriangleMeshSurfaceError> {
    let mut indices = Vec::with_capacity(surface.triangles.len());
    for triangle in surface.triangles.iter().copied() {
        let a = u32::try_from(triangle[0])
            .map_err(|_| TriangleMeshSurfaceError::IndexDoesNotFitU32(triangle[0]))?;
        let b = u32::try_from(triangle[1])
            .map_err(|_| TriangleMeshSurfaceError::IndexDoesNotFitU32(triangle[1]))?;
        let c = u32::try_from(triangle[2])
            .map_err(|_| TriangleMeshSurfaceError::IndexDoesNotFitU32(triangle[2]))?;
        indices.push([a, b, c]);
    }

    Ok(TriangleMeshSnapshot::new(
        version,
        surface.vertices.clone(),
        indices,
    ))
}

/// Stable topology identity for any dynamic triangle mesh surface.
///
/// Runtime providers may update vertices every frame, but sampled coordinates
/// can keep their triangle/barycentric anchors while the index topology stays
/// stable. A changed key means the coordinate set should be rebuilt.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshSurfaceTopologyKey {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub index_hash: u64,
}

impl MeshSurfaceTopologyKey {
    pub fn from_mesh(mesh: &TriangleMeshSurface) -> Self {
        Self {
            vertex_count: mesh.vertices.len(),
            triangle_count: mesh.triangles.len(),
            index_hash: mesh_surface_index_hash(&mesh.triangles),
        }
    }
}

/// Thin boundary for platform or engine code that can emit dynamic surfaces.
///
/// Providers own native/engine calls and coordinate-space conversion. This
/// crate only owns deterministic coordinate sampling, anchor updates, and
/// neighbor lists.
pub trait MeshSurfaceProvider {
    fn next_mesh_surface(&mut self) -> Option<TriangleMeshSurface>;
}

/// Outcome of one live dynamic mesh sampler update.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveMeshSurfaceUpdateStatus {
    NoMesh,
    Initialized,
    Updated,
    ResampledTopology,
    InvalidSurface,
}

/// Summary returned after updating sampled coordinates from a dynamic mesh.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveMeshSurfaceUpdate {
    pub status: LiveMeshSurfaceUpdateStatus,
    pub topology_key: Option<MeshSurfaceTopologyKey>,
    pub sample_count: usize,
}

/// Live coordinate sampler for any deformed triangle mesh with stable topology.
///
/// The first valid mesh produces a roughly even coordinate set over the
/// surface. Later meshes with the same topology update only positions and
/// normals by re-evaluating the stored triangle/barycentric anchors. Same-
/// surface neighbor tiers are preserved across these updates so interaction
/// identity stays stable; callers can explicitly rebuild them when they want
/// nearest neighbors in the current deformed pose.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LiveMeshSurfaceSampler {
    pub config: MeshSurfaceSampleConfig,
    samples: MeshSurfaceSampleSet,
    topology_key: Option<MeshSurfaceTopologyKey>,
}

impl LiveMeshSurfaceSampler {
    pub fn new(config: MeshSurfaceSampleConfig) -> Self {
        Self {
            config,
            samples: MeshSurfaceSampleSet::default(),
            topology_key: None,
        }
    }

    pub fn samples(&self) -> &MeshSurfaceSampleSet {
        &self.samples
    }

    pub fn samples_mut(&mut self) -> &mut MeshSurfaceSampleSet {
        &mut self.samples
    }

    pub fn topology_key(&self) -> Option<MeshSurfaceTopologyKey> {
        self.topology_key
    }

    pub fn update_from_provider<P: MeshSurfaceProvider + ?Sized>(
        &mut self,
        provider: &mut P,
    ) -> LiveMeshSurfaceUpdate {
        let Some(mesh) = provider.next_mesh_surface() else {
            return self.update_summary(LiveMeshSurfaceUpdateStatus::NoMesh);
        };
        self.update_from_mesh(&mesh)
    }

    pub fn update_from_mesh(&mut self, mesh: &TriangleMeshSurface) -> LiveMeshSurfaceUpdate {
        if !mesh.is_valid() {
            return self.update_summary(LiveMeshSurfaceUpdateStatus::InvalidSurface);
        }

        let next_key = MeshSurfaceTopologyKey::from_mesh(mesh);
        if self.topology_key != Some(next_key)
            || (self.samples.is_empty() && self.config.point_count > 0)
        {
            let next_samples = mesh.sample_even_points(self.config);
            if self.config.point_count > 0 && next_samples.is_empty() {
                return self.update_summary(LiveMeshSurfaceUpdateStatus::InvalidSurface);
            }

            let status = if self.topology_key.is_some() {
                LiveMeshSurfaceUpdateStatus::ResampledTopology
            } else {
                LiveMeshSurfaceUpdateStatus::Initialized
            };
            self.samples = next_samples;
            self.topology_key = Some(next_key);
            return self.update_summary(status);
        }

        if !self.samples.update_positions_from_mesh(mesh) {
            return self.update_summary(LiveMeshSurfaceUpdateStatus::InvalidSurface);
        }

        self.update_summary(LiveMeshSurfaceUpdateStatus::Updated)
    }

    pub fn rebuild_neighbor_tiers(&mut self) {
        self.samples.rebuild_neighbor_tiers(
            self.config.first_tier_neighbor_count,
            self.config.second_tier_neighbor_count,
        );
    }

    fn update_summary(&self, status: LiveMeshSurfaceUpdateStatus) -> LiveMeshSurfaceUpdate {
        LiveMeshSurfaceUpdate {
            status,
            topology_key: self.topology_key,
            sample_count: self.samples.point_count(),
        }
    }
}

impl Default for LiveMeshSurfaceSampler {
    fn default() -> Self {
        Self::new(MeshSurfaceSampleConfig::default())
    }
}

/// Stable topology identity for live hand-mesh particle anchors.
///
/// Runtime providers may update vertices every frame, but sampled coordinates
/// can keep their triangle/barycentric anchors while the index topology stays
/// stable. A changed key means the sampler should rebuild the coordinate set.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandMeshTopologyKey {
    pub handedness: Option<Handedness>,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub index_hash: u64,
}

impl HandMeshTopologyKey {
    pub fn from_snapshot(snapshot: &HandMeshSnapshot) -> Self {
        Self {
            handedness: snapshot.handedness,
            vertex_count: snapshot.vertices.len(),
            triangle_count: snapshot.indices.len(),
            index_hash: hand_mesh_index_hash(&snapshot.indices),
        }
    }
}

/// Thin boundary for platform-specific hand-mesh providers.
///
/// Native adapters own OpenXR/Meta/engine calls and return public
/// `HandMeshSnapshot` frames. This crate only owns sampling, anchor updates,
/// neighbor lists, and particle payload conversion.
pub trait HandMeshSnapshotProvider {
    fn next_hand_mesh_snapshot(&mut self) -> Option<HandMeshSnapshot>;
}

/// Outcome of one live hand-mesh sampler update.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveHandMeshUpdateStatus {
    NoSnapshot,
    Initialized,
    Updated,
    ResampledTopology,
    InvalidSnapshot,
    InvalidSurface,
}

/// Summary returned after polling a live hand-mesh snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveHandMeshUpdate {
    pub status: LiveHandMeshUpdateStatus,
    pub snapshot_version: Option<u64>,
    pub topology_key: Option<HandMeshTopologyKey>,
    pub sample_count: usize,
}

/// Live particle sampler for deformed hand mesh snapshots.
///
/// The sampler spreads a stable coordinate set over the first valid topology
/// it sees. On later frames with the same topology, it updates coordinates from
/// the deformed vertex positions and preserves neighbor identity. If the
/// topology key changes, it resamples and rebuilds neighbor tiers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LiveHandMeshParticleSampler {
    pub config: MeshSurfaceSampleConfig,
    pub coordinate_space: RenderCoordinateSpace,
    pub particle_size_meters: f32,
    pub particle_color: ColorRgba,
    samples: MeshSurfaceSampleSet,
    topology_key: Option<HandMeshTopologyKey>,
}

impl LiveHandMeshParticleSampler {
    pub fn new(config: MeshSurfaceSampleConfig) -> Self {
        Self {
            config,
            coordinate_space: RenderCoordinateSpace::World,
            particle_size_meters: 0.006,
            particle_color: ColorRgba::new(0.2, 0.8, 1.0, 1.0),
            samples: MeshSurfaceSampleSet::default(),
            topology_key: None,
        }
    }

    pub fn with_render_style(
        mut self,
        coordinate_space: RenderCoordinateSpace,
        particle_size_meters: f32,
        particle_color: ColorRgba,
    ) -> Self {
        self.coordinate_space = coordinate_space;
        self.particle_size_meters = particle_size_meters.max(0.0);
        self.particle_color = particle_color;
        self
    }

    pub fn samples(&self) -> &MeshSurfaceSampleSet {
        &self.samples
    }

    pub fn topology_key(&self) -> Option<HandMeshTopologyKey> {
        self.topology_key
    }

    pub fn update_from_provider<P: HandMeshSnapshotProvider + ?Sized>(
        &mut self,
        provider: &mut P,
    ) -> LiveHandMeshUpdate {
        let Some(snapshot) = provider.next_hand_mesh_snapshot() else {
            return self.update_summary(LiveHandMeshUpdateStatus::NoSnapshot, None);
        };
        self.update_from_snapshot(&snapshot)
    }

    pub fn update_from_snapshot(&mut self, snapshot: &HandMeshSnapshot) -> LiveHandMeshUpdate {
        let Ok(mesh) = triangle_mesh_surface_from_hand_mesh_snapshot(snapshot) else {
            return self.update_summary(
                LiveHandMeshUpdateStatus::InvalidSnapshot,
                Some(snapshot.version),
            );
        };

        let next_key = HandMeshTopologyKey::from_snapshot(snapshot);
        if self.topology_key != Some(next_key) || self.samples.is_empty() {
            let next_samples = mesh.sample_even_points(self.config);
            if self.config.point_count > 0 && next_samples.is_empty() {
                return self.update_summary(
                    LiveHandMeshUpdateStatus::InvalidSurface,
                    Some(snapshot.version),
                );
            }

            let status = if self.topology_key.is_some() {
                LiveHandMeshUpdateStatus::ResampledTopology
            } else {
                LiveHandMeshUpdateStatus::Initialized
            };
            self.samples = next_samples;
            self.topology_key = Some(next_key);
            return self.update_summary(status, Some(snapshot.version));
        }

        if !self.samples.update_positions_from_mesh(&mesh) {
            return self.update_summary(
                LiveHandMeshUpdateStatus::InvalidSurface,
                Some(snapshot.version),
            );
        }

        self.update_summary(LiveHandMeshUpdateStatus::Updated, Some(snapshot.version))
    }

    pub fn render_particles(&self) -> Vec<ParticleRender> {
        self.samples
            .render_particles(self.particle_size_meters, self.particle_color)
    }

    pub fn render_payload(&self, frame_index: u64) -> RenderPayload {
        self.samples.render_payload(
            frame_index,
            self.coordinate_space,
            self.particle_size_meters,
            self.particle_color,
        )
    }

    fn update_summary(
        &self,
        status: LiveHandMeshUpdateStatus,
        snapshot_version: Option<u64>,
    ) -> LiveHandMeshUpdate {
        LiveHandMeshUpdate {
            status,
            snapshot_version,
            topology_key: self.topology_key,
            sample_count: self.samples.point_count(),
        }
    }
}

impl Default for LiveHandMeshParticleSampler {
    fn default() -> Self {
        Self::new(MeshSurfaceSampleConfig::default())
    }
}

/// Configuration for deterministic mesh-surface coordinate sampling.
///
/// `point_count` is the exact requested output count when the mesh has valid
/// area. Neighbor tiers are nearest-sample lists intended for local interaction
/// passes; no oscillator or downstream simulation behavior is included here.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshSurfaceSampleConfig {
    pub point_count: usize,
    pub first_tier_neighbor_count: usize,
    pub second_tier_neighbor_count: usize,
    pub seed: u64,
}

impl Default for MeshSurfaceSampleConfig {
    fn default() -> Self {
        Self {
            point_count: 256,
            first_tier_neighbor_count: 6,
            second_tier_neighbor_count: 12,
            seed: 11_337,
        }
    }
}

/// One sampled coordinate on a triangle mesh surface.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshSurfaceSample {
    pub position: Vec3,
    pub normal: Vec3,
    pub triangle_index: usize,
    pub barycentric: [f32; 3],
}

impl MeshSurfaceSample {
    pub fn is_valid(self) -> bool {
        self.position.is_finite()
            && self.normal.is_finite()
            && self
                .barycentric
                .iter()
                .all(|value| value.is_finite() && *value >= -1.0e-5 && *value <= 1.0 + 1.0e-5)
            && (self.barycentric[0] + self.barycentric[1] + self.barycentric[2] - 1.0).abs()
                <= 1.0e-4
    }
}

/// Sampled mesh coordinates plus nearest-neighbor tiers for interaction passes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshSurfaceSampleSet {
    pub samples: Vec<MeshSurfaceSample>,
    pub first_tier_neighbors: Vec<Vec<usize>>,
    pub second_tier_neighbors: Vec<Vec<usize>>,
}

impl MeshSurfaceSampleSet {
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn point_count(&self) -> usize {
        self.samples.len()
    }

    pub fn positions(&self) -> Vec<Vec3> {
        self.samples.iter().map(|sample| sample.position).collect()
    }

    /// Re-evaluate stable triangle/barycentric anchors against a deformed mesh.
    ///
    /// Use this when a runtime adapter can provide the same mesh topology with
    /// updated vertex positions each frame. Sample identity and existing
    /// neighbor lists are preserved; call `rebuild_neighbor_tiers` afterwards if
    /// interactions need nearest neighbors in the current deformed pose.
    pub fn update_positions_from_mesh(&mut self, mesh: &TriangleMeshSurface) -> bool {
        let mut updates = Vec::with_capacity(self.samples.len());
        for sample in &self.samples {
            let Some((position, normal)) =
                evaluate_surface_anchor(mesh, sample.triangle_index, sample.barycentric)
            else {
                return false;
            };
            updates.push((position, normal));
        }

        for (sample, (position, normal)) in self.samples.iter_mut().zip(updates) {
            sample.position = position;
            sample.normal = normal;
        }
        true
    }

    pub fn update_positions_from_hand_mesh_snapshot(
        &mut self,
        snapshot: &HandMeshSnapshot,
    ) -> bool {
        let Ok(mesh) = triangle_mesh_surface_from_hand_mesh_snapshot(snapshot) else {
            return false;
        };
        self.update_positions_from_mesh(&mesh)
    }

    pub fn rebuild_neighbor_tiers(
        &mut self,
        first_tier_neighbor_count: usize,
        second_tier_neighbor_count: usize,
    ) {
        let (first_tier_neighbors, second_tier_neighbors) = build_nearest_neighbor_tiers(
            &self.positions(),
            first_tier_neighbor_count,
            second_tier_neighbor_count,
        );
        self.first_tier_neighbors = first_tier_neighbors;
        self.second_tier_neighbors = second_tier_neighbors;
    }

    pub fn is_valid(&self) -> bool {
        let count = self.samples.len();
        self.samples.iter().all(|sample| sample.is_valid())
            && self.first_tier_neighbors.len() == count
            && self.second_tier_neighbors.len() == count
            && self
                .first_tier_neighbors
                .iter()
                .enumerate()
                .all(|(origin, neighbors)| neighbor_list_is_valid(origin, count, neighbors))
            && self
                .second_tier_neighbors
                .iter()
                .enumerate()
                .all(|(origin, neighbors)| neighbor_list_is_valid(origin, count, neighbors))
    }

    pub fn render_particles(&self, size_meters: f32, color: ColorRgba) -> Vec<ParticleRender> {
        let mut particles = Vec::with_capacity(self.samples.len());
        for sample in &self.samples {
            let mut particle = ParticleRender::new(sample.position, size_meters.max(0.0), color);
            particle.normal = sample.normal;
            particle.aux0 = sample.triangle_index as f32;
            particle.aux1 = self
                .first_tier_neighbors
                .get(particles.len())
                .map_or(0.0, |neighbors| neighbors.len() as f32);
            particles.push(particle);
        }
        particles
    }

    pub fn render_payload(
        &self,
        frame_index: u64,
        coordinate_space: RenderCoordinateSpace,
        size_meters: f32,
        color: ColorRgba,
    ) -> RenderPayload {
        render_particles_to_payload(
            frame_index,
            coordinate_space,
            &self.render_particles(size_meters, color),
        )
    }

    pub fn cross_neighborhood_with(
        &self,
        other: &Self,
        config: MeshSurfaceCrossNeighborConfig,
    ) -> MeshSurfaceCrossNeighborhood {
        build_mesh_surface_cross_neighborhood(&self.positions(), &other.positions(), config)
    }
}

/// Configuration for nearest-neighbor links between two sampled surfaces.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshSurfaceCrossNeighborConfig {
    pub neighbors_per_point: usize,
    /// Positive values limit links by distance. `0.0` disables the distance gate.
    pub max_distance_meters: f32,
}

impl Default for MeshSurfaceCrossNeighborConfig {
    fn default() -> Self {
        Self {
            neighbors_per_point: 1,
            max_distance_meters: 0.0,
        }
    }
}

/// Bidirectional nearest-neighbor links between two coordinate sets.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshSurfaceCrossNeighborhood {
    pub a_to_b_neighbors: Vec<Vec<usize>>,
    pub b_to_a_neighbors: Vec<Vec<usize>>,
}

impl MeshSurfaceCrossNeighborhood {
    pub fn is_valid(&self, a_count: usize, b_count: usize) -> bool {
        self.a_to_b_neighbors.len() == a_count
            && self.b_to_a_neighbors.len() == b_count
            && self
                .a_to_b_neighbors
                .iter()
                .all(|neighbors| neighbor_targets_are_valid(b_count, neighbors))
            && self
                .b_to_a_neighbors
                .iter()
                .all(|neighbors| neighbor_targets_are_valid(a_count, neighbors))
    }
}

/// Build a deterministic, roughly even set of coordinates over a mesh surface.
///
/// The sampler uses triangle-area stratification plus low-discrepancy
/// barycentric placement. It gives stable, visually even coverage for small and
/// medium public examples; it is not a strict blue-noise optimizer.
pub fn sample_mesh_surface_points(
    mesh: &TriangleMeshSurface,
    config: MeshSurfaceSampleConfig,
) -> MeshSurfaceSampleSet {
    if config.point_count == 0 {
        return MeshSurfaceSampleSet::default();
    }

    let triangles = mesh_surface_triangle_records(mesh);
    if triangles.is_empty() {
        return MeshSurfaceSampleSet::default();
    }

    let total_area = triangles
        .last()
        .map_or(0.0, |triangle| triangle.cumulative_area);
    if !total_area.is_finite() || total_area <= 1.0e-9 {
        return MeshSurfaceSampleSet::default();
    }

    let mut per_triangle_counts = vec![0_usize; mesh.triangles.len()];
    let mut samples = Vec::with_capacity(config.point_count);
    for sample_index in 0..config.point_count {
        let area_target = stratified_area_target(sample_index, config.point_count, total_area);
        let record_index = select_surface_triangle(&triangles, area_target);
        let record = triangles[record_index];
        let local_index = per_triangle_counts[record.triangle_index];
        per_triangle_counts[record.triangle_index] += 1;

        let barycentric = sample_barycentric(local_index, config.seed, record.triangle_index);
        let [a, b, c] = record.indices;
        let position = (mesh.vertices[a] * barycentric[0])
            + (mesh.vertices[b] * barycentric[1])
            + (mesh.vertices[c] * barycentric[2]);
        samples.push(MeshSurfaceSample {
            position,
            normal: record.normal,
            triangle_index: record.triangle_index,
            barycentric,
        });
    }

    let positions: Vec<_> = samples.iter().map(|sample| sample.position).collect();
    let (first_tier_neighbors, second_tier_neighbors) = build_nearest_neighbor_tiers(
        &positions,
        config.first_tier_neighbor_count,
        config.second_tier_neighbor_count,
    );

    MeshSurfaceSampleSet {
        samples,
        first_tier_neighbors,
        second_tier_neighbors,
    }
}

/// Build nearest-neighbor links between two mesh-surface coordinate sets.
pub fn build_mesh_surface_cross_neighborhood(
    a_positions: &[Vec3],
    b_positions: &[Vec3],
    config: MeshSurfaceCrossNeighborConfig,
) -> MeshSurfaceCrossNeighborhood {
    let max_distance_squared =
        if config.max_distance_meters.is_finite() && config.max_distance_meters > 0.0 {
            let max_distance = config.max_distance_meters.max(0.0);
            max_distance * max_distance
        } else {
            f32::INFINITY
        };

    MeshSurfaceCrossNeighborhood {
        a_to_b_neighbors: build_cross_neighbor_lists(
            a_positions,
            b_positions,
            config.neighbors_per_point,
            max_distance_squared,
        ),
        b_to_a_neighbors: build_cross_neighbor_lists(
            b_positions,
            a_positions,
            config.neighbors_per_point,
            max_distance_squared,
        ),
    }
}

/// Dimensions for a simple procedural hand-like mesh used in public examples.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixtureHandMeshConfig {
    pub palm_width_meters: f32,
    pub palm_height_meters: f32,
    pub palm_thickness_meters: f32,
    pub finger_width_meters: f32,
    pub finger_spacing_meters: f32,
    pub finger_thickness_meters: f32,
    pub finger_lengths_meters: [f32; 5],
    pub thumb_angle_degrees: f32,
}

impl Default for FixtureHandMeshConfig {
    fn default() -> Self {
        Self {
            palm_width_meters: 0.085,
            palm_height_meters: 0.095,
            palm_thickness_meters: 0.018,
            finger_width_meters: 0.014,
            finger_spacing_meters: 0.004,
            finger_thickness_meters: 0.014,
            finger_lengths_meters: [0.052, 0.070, 0.080, 0.072, 0.056],
            thumb_angle_degrees: -42.0,
        }
    }
}

/// Build an open-hand fixture mesh for examples and tests.
///
/// The mesh is deliberately procedural and approximate. It demonstrates how a
/// native hand-mesh adapter can feed the sampler without embedding platform or
/// app-specific hand tracking code in this crate.
pub fn build_fixture_hand_mesh(config: FixtureHandMeshConfig) -> TriangleMeshSurface {
    let palm_width = config.palm_width_meters.max(0.001);
    let palm_height = config.palm_height_meters.max(0.001);
    let palm_thickness = config.palm_thickness_meters.max(0.001);
    let finger_width = config.finger_width_meters.max(0.001);
    let finger_spacing = config.finger_spacing_meters.max(0.0);
    let finger_thickness = config.finger_thickness_meters.max(0.001);

    let mut mesh = TriangleMeshSurface::default();
    append_oriented_box(
        &mut mesh,
        Vec3::ZERO,
        Vec3::new(palm_width * 0.5, palm_height * 0.5, palm_thickness * 0.5),
        0.0,
    );

    let upright_lengths = [
        config.finger_lengths_meters[1].max(0.001),
        config.finger_lengths_meters[2].max(0.001),
        config.finger_lengths_meters[3].max(0.001),
        config.finger_lengths_meters[4].max(0.001),
    ];
    let total_finger_width = (finger_width * 4.0) + (finger_spacing * 3.0);
    let first_finger_x = -0.5 * total_finger_width + (finger_width * 0.5);
    let palm_top_y = palm_height * 0.5;
    for (index, length) in upright_lengths.iter().copied().enumerate() {
        let center = Vec3::new(
            first_finger_x + (index as f32 * (finger_width + finger_spacing)),
            palm_top_y + (length * 0.5),
            0.0,
        );
        append_oriented_box(
            &mut mesh,
            center,
            Vec3::new(finger_width * 0.5, length * 0.5, finger_thickness * 0.5),
            0.0,
        );
    }

    let thumb_length = config.finger_lengths_meters[0].max(0.001);
    let thumb_angle = config.thumb_angle_degrees.to_radians();
    let thumb_center = Vec3::new(
        -0.5 * palm_width - (0.32 * thumb_length),
        -0.12 * palm_height + (0.20 * thumb_length),
        0.0,
    );
    append_oriented_box(
        &mut mesh,
        thumb_center,
        Vec3::new(
            finger_width * 0.55,
            thumb_length * 0.5,
            finger_thickness * 0.5,
        ),
        thumb_angle,
    );

    mesh
}

/// Backend-neutral triangle fan geometry for circular particle billboards.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleDiscMeshConfig {
    pub segments: usize,
    pub radius: f32,
}

impl Default for ParticleDiscMeshConfig {
    fn default() -> Self {
        Self {
            segments: DEFAULT_PARTICLE_DISC_SEGMENTS,
            radius: 0.5,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleDiscVertex {
    pub position: Vec3,
    pub uv: [f32; 2],
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParticleDiscMesh {
    pub vertices: Vec<ParticleDiscVertex>,
    pub indices: Vec<u32>,
}

pub fn build_particle_disc_mesh(config: ParticleDiscMeshConfig) -> ParticleDiscMesh {
    let segments = config.segments.max(3);
    let radius = config.radius.max(0.0);
    let mut vertices = Vec::with_capacity(segments + 1);
    let mut indices = Vec::with_capacity(segments * 3);
    vertices.push(ParticleDiscVertex {
        position: Vec3::ZERO,
        uv: [0.5, 0.5],
    });

    for segment in 0..segments {
        let angle = segment as f32 * core::f32::consts::TAU / segments as f32;
        let local_x = angle.cos() * radius;
        let local_y = angle.sin() * radius;
        vertices.push(ParticleDiscVertex {
            position: Vec3::new(local_x, local_y, 0.0),
            uv: [
                local_x / (radius * 2.0).max(1.0e-6) + 0.5,
                local_y / (radius * 2.0).max(1.0e-6) + 0.5,
            ],
        });

        let next = if segment == segments - 1 {
            1
        } else {
            segment as u32 + 2
        };
        indices.extend_from_slice(&[0, segment as u32 + 1, next]);
    }

    ParticleDiscMesh { vertices, indices }
}

/// World-space basis used to place renderer-owned particle buffers into an XR
/// scene without requiring a specific rendering backend.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleSceneBasis {
    pub center: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
    pub scale: f32,
}

impl Default for ParticleSceneBasis {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            right: Vec3::RIGHT,
            up: Vec3::UP,
            forward: Vec3::FORWARD_NEG_Z,
            scale: 1.0,
        }
    }
}

impl ParticleSceneBasis {
    pub const fn new(center: Vec3, right: Vec3, up: Vec3, forward: Vec3, scale: f32) -> Self {
        Self {
            center,
            right,
            up,
            forward,
            scale,
        }
    }

    pub fn normalized(self) -> Self {
        Self {
            center: if self.center.is_finite() {
                self.center
            } else {
                Vec3::ZERO
            },
            right: self.right.normalized_or(Vec3::RIGHT),
            up: self.up.normalized_or(Vec3::UP),
            forward: self.forward.normalized_or(Vec3::FORWARD_NEG_Z),
            scale: if self.scale.is_finite() {
                self.scale
            } else {
                1.0
            },
        }
    }

    pub fn transform_point(self, local: Vec3) -> Vec3 {
        let basis = self.normalized();
        basis.center + (basis.transform_vector(local) * basis.scale)
    }

    pub fn transform_vector(self, local: Vec3) -> Vec3 {
        let basis = self.normalized();
        (basis.right * local.x) + (basis.up * local.y) + (basis.forward * local.z)
    }

    pub fn transform_direction(self, local: Vec3, fallback: Vec3) -> Vec3 {
        self.transform_vector(local).normalized_or(fallback)
    }
}

/// Backend-neutral instance layout for animated particle billboards.
///
/// The shape mirrors common GPU particle records while staying renderer
/// agnostic: position plus size, color, normal plus animation frame, and four
/// auxiliary floats for rotation, effect controls, and compact flags.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParticleBillboardInstance {
    pub position_size: [f32; 4],
    pub color: [f32; 4],
    pub normal_frame: [f32; 4],
    pub aux: [f32; 4],
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleBillboardBuildConfig {
    pub max_instances: usize,
    pub min_size_meters: f32,
    pub min_alpha: f32,
    pub sort_back_to_front: bool,
}

impl Default for ParticleBillboardBuildConfig {
    fn default() -> Self {
        Self {
            max_instances: usize::MAX,
            min_size_meters: 1.0e-6,
            min_alpha: 1.0e-6,
            sort_back_to_front: false,
        }
    }
}

impl ParticleBillboardBuildConfig {
    pub fn sanitized(self) -> Self {
        Self {
            max_instances: self.max_instances,
            min_size_meters: if self.min_size_meters.is_finite() {
                self.min_size_meters.max(0.0)
            } else {
                0.0
            },
            min_alpha: if self.min_alpha.is_finite() {
                self.min_alpha.clamp(0.0, 1.0)
            } else {
                0.0
            },
            sort_back_to_front: self.sort_back_to_front,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParticleBillboardSortCamera {
    pub position: Vec3,
    pub forward: Vec3,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParticleBillboardBuildStats {
    pub source_count: usize,
    pub emitted_count: usize,
    pub skipped_count: usize,
}

pub fn particle_billboard_instance(
    particle: ParticleRender,
    basis: ParticleSceneBasis,
) -> ParticleBillboardInstance {
    let basis = basis.normalized();
    particle_billboard_instance_from_normalized_basis(particle, basis)
}

fn particle_billboard_instance_from_normalized_basis(
    particle: ParticleRender,
    basis: ParticleSceneBasis,
) -> ParticleBillboardInstance {
    let position = transform_point_with_normalized_basis(basis, particle.position);
    let normal =
        transform_vector_with_normalized_basis(basis, particle.normal).normalized_or(Vec3::UP);
    let scale = basis.scale.abs();
    ParticleBillboardInstance {
        position_size: [
            position.x,
            position.y,
            position.z,
            particle.size_meters.max(0.0) * scale,
        ],
        color: [
            particle.color.r,
            particle.color.g,
            particle.color.b,
            particle.color.a.clamp(0.0, 1.0),
        ],
        normal_frame: [
            normal.x,
            normal.y,
            normal.z,
            particle.frame01.clamp(0.0, 1.0),
        ],
        aux: [
            particle.rotation_radians,
            particle.aux0,
            particle.aux1,
            particle.flags as f32,
        ],
    }
}

fn transform_point_with_normalized_basis(basis: ParticleSceneBasis, local: Vec3) -> Vec3 {
    basis.center + (transform_vector_with_normalized_basis(basis, local) * basis.scale)
}

fn transform_vector_with_normalized_basis(basis: ParticleSceneBasis, local: Vec3) -> Vec3 {
    (basis.right * local.x) + (basis.up * local.y) + (basis.forward * local.z)
}

pub fn write_particle_billboard_instances(
    particles: &[ParticleRender],
    basis: ParticleSceneBasis,
    config: ParticleBillboardBuildConfig,
    sort_camera: Option<ParticleBillboardSortCamera>,
    sort_indices: &mut Vec<usize>,
    out: &mut Vec<ParticleBillboardInstance>,
) -> ParticleBillboardBuildStats {
    let basis = basis.normalized();
    let config = config.sanitized();
    let source_count = particles.len();
    let capped_count = source_count.min(config.max_instances);
    out.clear();

    if config.sort_back_to_front {
        sort_indices.clear();
        sort_indices.reserve(capped_count);
        for (index, particle) in particles.iter().copied().take(capped_count).enumerate() {
            if should_emit_billboard_particle(particle, basis, config) {
                sort_indices.push(index);
            }
        }

        if let Some(camera) = sort_camera {
            let forward = camera.forward.normalized_or(Vec3::FORWARD_NEG_Z);
            sort_indices.sort_by(|left, right| {
                let left_depth = particle_billboard_depth_along_forward_normalized_basis(
                    particles[*left],
                    basis,
                    camera.position,
                    forward,
                );
                let right_depth = particle_billboard_depth_along_forward_normalized_basis(
                    particles[*right],
                    basis,
                    camera.position,
                    forward,
                );
                right_depth
                    .partial_cmp(&left_depth)
                    .unwrap_or(core::cmp::Ordering::Equal)
            });
        }

        out.reserve(sort_indices.len());
        for particle_index in sort_indices.iter().copied() {
            out.push(particle_billboard_instance_from_normalized_basis(
                particles[particle_index],
                basis,
            ));
        }
    } else {
        out.reserve(capped_count);
        for particle in particles.iter().copied().take(capped_count) {
            if should_emit_billboard_particle(particle, basis, config) {
                out.push(particle_billboard_instance_from_normalized_basis(
                    particle, basis,
                ));
            }
        }
        sort_indices.clear();
    }

    ParticleBillboardBuildStats {
        source_count,
        emitted_count: out.len(),
        skipped_count: source_count.saturating_sub(out.len()),
    }
}

pub fn particle_billboard_depth_along_forward(
    particle: ParticleRender,
    basis: ParticleSceneBasis,
    camera_position: Vec3,
    camera_forward: Vec3,
) -> f32 {
    particle_billboard_depth_along_forward_normalized_basis(
        particle,
        basis.normalized(),
        camera_position,
        camera_forward.normalized_or(Vec3::FORWARD_NEG_Z),
    )
}

fn particle_billboard_depth_along_forward_normalized_basis(
    particle: ParticleRender,
    basis: ParticleSceneBasis,
    camera_position: Vec3,
    camera_forward: Vec3,
) -> f32 {
    let position = transform_point_with_normalized_basis(basis, particle.position);
    (position - camera_position).dot(camera_forward)
}

pub fn particle_billboard_render_budget(
    source_particles: usize,
    active_trails: usize,
    disc_segments: usize,
) -> ParticleBillboardRenderBudget {
    let visible_instances = source_particles.saturating_add(active_trails);
    let indices_per_instance = disc_segments.max(3).saturating_mul(3);
    ParticleBillboardRenderBudget {
        source_particles,
        active_trails,
        visible_instances,
        indices_per_instance,
        total_indices: visible_instances.saturating_mul(indices_per_instance),
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParticleBillboardRenderBudget {
    pub source_particles: usize,
    pub active_trails: usize,
    pub visible_instances: usize,
    pub indices_per_instance: usize,
    pub total_indices: usize,
}

fn should_emit_billboard_particle(
    particle: ParticleRender,
    basis: ParticleSceneBasis,
    config: ParticleBillboardBuildConfig,
) -> bool {
    if !particle.is_valid() {
        return false;
    }
    let output_size = particle.size_meters.max(0.0) * basis.scale.abs();
    output_size >= config.min_size_meters && particle.color.a >= config.min_alpha
}

/// Configuration for a small morphed-ring billboard animation atlas.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MorphedRingAtlasConfig {
    pub frame_resolution: usize,
    pub frame_count: usize,
    pub atlas_columns: usize,
    pub edge_width: f32,
    pub outer_feather: f32,
    pub ring_radius: f32,
    pub ring_thickness: f32,
    pub dual_offset_degrees: f32,
}

impl Default for MorphedRingAtlasConfig {
    fn default() -> Self {
        Self {
            frame_resolution: DEFAULT_ANIMATED_RING_FRAME_RESOLUTION,
            frame_count: DEFAULT_ANIMATED_RING_FRAME_COUNT,
            atlas_columns: DEFAULT_ANIMATED_RING_ATLAS_COLUMNS,
            edge_width: 0.015,
            outer_feather: 0.06,
            ring_radius: 0.32,
            ring_thickness: 0.03,
            dual_offset_degrees: 180.0,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MorphedRingAtlas {
    pub width: usize,
    pub height: usize,
    pub frame_resolution: usize,
    pub frame_count: usize,
    pub columns: usize,
    pub rows: usize,
    pub rgba: Vec<u8>,
}

pub fn build_morphed_ring_atlas_rgba(config: MorphedRingAtlasConfig) -> MorphedRingAtlas {
    let frame_resolution = config.frame_resolution.max(1);
    let frame_count = config.frame_count.max(1);
    let columns = config.atlas_columns.max(1);
    let rows = frame_count.div_ceil(columns);
    let width = frame_resolution * columns;
    let height = frame_resolution * rows;
    let mut rgba = vec![0_u8; width * height * 4];
    let aa = (1.0 / frame_resolution as f32).max(0.0001);

    for frame in 0..frame_count {
        let phase01 = if frame_count <= 1 {
            0.0
        } else {
            frame as f32 / (frame_count - 1) as f32
        };
        let frame_col = frame % columns;
        let frame_row = frame / columns;
        for y in 0..frame_resolution {
            let uv_y = (y as f32 + 0.5) / frame_resolution as f32;
            for x in 0..frame_resolution {
                let uv = [(x as f32 + 0.5) / frame_resolution as f32, uv_y];
                let d = morphed_ring_distance(
                    uv,
                    phase01,
                    config.ring_radius,
                    config.ring_thickness,
                    config.dual_offset_degrees,
                );
                let core = 1.0 - smoothstep(config.edge_width, config.edge_width + aa, d);
                let feather = 1.0
                    - smoothstep(
                        config.edge_width + aa,
                        config.edge_width + aa + config.outer_feather,
                        d,
                    );
                let value = ((core.max(feather).clamp(0.0, 1.0) * 255.0) + 0.5)
                    .floor()
                    .clamp(0.0, 255.0) as u8;
                let atlas_x = frame_col * frame_resolution + x;
                let atlas_y = frame_row * frame_resolution + y;
                let offset = (atlas_y * width + atlas_x) * 4;
                rgba[offset..offset + 4].copy_from_slice(&[value, value, value, value]);
            }
        }
    }

    MorphedRingAtlas {
        width,
        height,
        frame_resolution,
        frame_count,
        columns,
        rows,
        rgba,
    }
}

pub fn morphed_ring_alpha(uv: [f32; 2], phase01: f32, config: MorphedRingAtlasConfig) -> f32 {
    let resolution = config.frame_resolution.max(1) as f32;
    let aa = (1.0 / resolution).max(0.0001);
    let d = morphed_ring_distance(
        uv,
        phase01,
        config.ring_radius,
        config.ring_thickness,
        config.dual_offset_degrees,
    );
    let core = 1.0 - smoothstep(config.edge_width, config.edge_width + aa, d);
    let feather = 1.0
        - smoothstep(
            config.edge_width + aa,
            config.edge_width + aa + config.outer_feather,
            d,
        );
    core.max(feather).clamp(0.0, 1.0)
}

/// Reusable integrated particle trail state. It copies visible source particles
/// into per-source ring buffers and fades them out over a fixed lifetime.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleTrailConfig {
    pub enabled: bool,
    pub visuals_enabled: bool,
    pub lifetime_seconds: f32,
    pub copies_per_second: f32,
    pub max_spawn_batches_per_frame: usize,
    pub copies_per_particle: usize,
    pub size_multiplier: f32,
}

impl Default for ParticleTrailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            visuals_enabled: true,
            lifetime_seconds: 0.25,
            copies_per_second: 0.0,
            max_spawn_batches_per_frame: 1,
            copies_per_particle: 1,
            size_multiplier: 1.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParticleTrailEmitter {
    config: ParticleTrailConfig,
    particles: Vec<ParticleRender>,
    base_particles: Vec<ParticleRender>,
    ages_seconds: Vec<f32>,
    heads: Vec<usize>,
    emit_accumulator_seconds: f32,
    last_active_count: usize,
    last_emitted_count: usize,
}

impl ParticleTrailEmitter {
    pub fn new(config: ParticleTrailConfig) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    pub fn config(&self) -> ParticleTrailConfig {
        self.config
    }

    pub fn set_config(&mut self, config: ParticleTrailConfig) {
        if self.config.copies_per_particle != config.copies_per_particle
            || self.config.enabled != config.enabled
        {
            self.clear();
        }
        self.config = config;
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.base_particles.clear();
        self.ages_seconds.clear();
        self.heads.clear();
        self.emit_accumulator_seconds = 0.0;
        self.last_active_count = 0;
        self.last_emitted_count = 0;
    }

    pub fn update(
        &mut self,
        delta_seconds: f32,
        source_particles: &[ParticleRender],
    ) -> &[ParticleRender] {
        if !self.config.enabled || source_particles.is_empty() {
            self.clear();
            return &self.particles;
        }

        self.ensure_layout(source_particles.len());
        if self.particles.is_empty() {
            return &self.particles;
        }

        let delta_seconds = delta_seconds.max(0.0);
        let lifetime = self.config.lifetime_seconds.max(0.0);
        let size_multiplier = self.config.size_multiplier.max(0.0);
        let hidden = hidden_render_particle();
        let mut active_count = 0usize;

        for index in 0..self.ages_seconds.len() {
            let age = self.ages_seconds[index];
            if age < 0.0 {
                self.particles[index] = hidden;
                continue;
            }

            let next_age = age + delta_seconds;
            if lifetime <= 0.0 || next_age >= lifetime {
                self.ages_seconds[index] = -1.0;
                self.particles[index] = hidden;
                continue;
            }

            self.ages_seconds[index] = next_age;
            let fade = (1.0 - (next_age / lifetime)).clamp(0.0, 1.0);
            let mut particle = self.base_particles[index];
            if self.config.visuals_enabled {
                particle.size_meters *= size_multiplier;
                particle.color.a = (particle.color.a * fade).clamp(0.0, 1.0);
            } else {
                particle.size_meters = 0.0;
                particle.color.a = 0.0;
            }
            self.particles[index] = particle;
            if particle.size_meters > 1.0e-6 && particle.color.a > 1.0e-6 {
                active_count += 1;
            }
        }

        let copies_per_second = self.config.copies_per_second.max(0.0);
        if copies_per_second <= 0.0 {
            self.last_active_count = active_count;
            self.last_emitted_count = 0;
            return &self.particles;
        }

        let interval = 1.0 / copies_per_second;
        if !interval.is_finite() || interval <= 0.0 {
            self.last_active_count = active_count;
            self.last_emitted_count = 0;
            return &self.particles;
        }

        self.emit_accumulator_seconds += delta_seconds;
        let max_batches = self.config.max_spawn_batches_per_frame.clamp(1, 16);
        let batches =
            ((self.emit_accumulator_seconds / interval).floor() as usize).min(max_batches);
        if batches == 0 {
            self.last_active_count = active_count;
            self.last_emitted_count = 0;
            return &self.particles;
        }
        self.emit_accumulator_seconds -= batches as f32 * interval;

        let copies_per_particle = self.config.copies_per_particle.max(1);
        let mut emitted_count = 0usize;
        for (source_index, source_particle) in source_particles.iter().copied().enumerate() {
            if source_particle.size_meters <= 1.0e-6 || source_particle.color.a <= 1.0e-6 {
                continue;
            }
            let base_flat = source_index.saturating_mul(copies_per_particle);
            if base_flat >= self.particles.len() {
                continue;
            }

            let mut head = self.heads[source_index].min(copies_per_particle - 1);
            for _ in 0..batches {
                let mut slot = None;
                for offset in 0..copies_per_particle {
                    let local = (head + offset) % copies_per_particle;
                    let candidate = base_flat + local;
                    if candidate < self.ages_seconds.len() && self.ages_seconds[candidate] < 0.0 {
                        slot = Some((candidate, local));
                        break;
                    }
                }

                let Some((slot_index, slot_local)) = slot else {
                    break;
                };
                self.base_particles[slot_index] = source_particle;
                self.particles[slot_index] = source_particle;
                self.ages_seconds[slot_index] = 0.0;
                head = (slot_local + 1) % copies_per_particle;
                emitted_count += 1;
            }
            self.heads[source_index] = head;
        }

        self.last_active_count = active_count + emitted_count;
        self.last_emitted_count = emitted_count;
        &self.particles
    }

    pub fn particles(&self) -> &[ParticleRender] {
        &self.particles
    }

    pub fn ages_seconds(&self) -> &[f32] {
        &self.ages_seconds
    }

    pub fn last_active_count(&self) -> usize {
        self.last_active_count
    }

    pub fn last_emitted_count(&self) -> usize {
        self.last_emitted_count
    }

    fn ensure_layout(&mut self, source_count: usize) {
        let copies_per_particle = self.config.copies_per_particle.max(1);
        let total = source_count.saturating_mul(copies_per_particle);
        if self.particles.len() == total && self.heads.len() == source_count {
            return;
        }

        let hidden = hidden_render_particle();
        self.particles = vec![hidden; total];
        self.base_particles = vec![hidden; total];
        self.ages_seconds = vec![-1.0; total];
        self.heads = vec![0; source_count];
        self.emit_accumulator_seconds = 0.0;
        self.last_active_count = 0;
        self.last_emitted_count = 0;
    }
}

pub fn sorted_particle_indices_back_to_front(
    particles: &[ParticleRender],
    camera_position: Vec3,
    camera_forward: Vec3,
) -> Vec<usize> {
    let mut indices = (0..particles.len()).collect::<Vec<_>>();
    sort_particle_indices_back_to_front(&mut indices, particles, camera_position, camera_forward);
    indices
}

pub fn sort_particle_indices_back_to_front(
    indices: &mut [usize],
    particles: &[ParticleRender],
    camera_position: Vec3,
    camera_forward: Vec3,
) {
    let forward = camera_forward.normalized_or(Vec3::FORWARD_NEG_Z);
    indices.sort_by(|left, right| {
        let left_depth = particle_depth_along_forward(particles[*left], camera_position, forward);
        let right_depth = particle_depth_along_forward(particles[*right], camera_position, forward);
        right_depth
            .partial_cmp(&left_depth)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
}

pub fn particle_depth_along_forward(
    particle: ParticleRender,
    camera_position: Vec3,
    camera_forward: Vec3,
) -> f32 {
    (particle.position - camera_position).dot(camera_forward.normalized_or(Vec3::FORWARD_NEG_Z))
}

fn hidden_render_particle() -> ParticleRender {
    ParticleRender {
        color: ColorRgba::new(0.0, 0.0, 0.0, 0.0),
        ..ParticleRender::default()
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let denom = (edge1 - edge0).max(1.0e-6);
    let t = ((x - edge0) / denom).clamp(0.0, 1.0);
    t * t * (3.0 - (2.0 * t))
}

fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [p[0] - a[0], p[1] - a[1]];
    let denom = ((ab[0] * ab[0]) + (ab[1] * ab[1])).max(1.0e-5);
    let t = (((ap[0] * ab[0]) + (ap[1] * ab[1])) / denom).clamp(0.0, 1.0);
    let closest = [a[0] + (ab[0] * t), a[1] + (ab[1] * t)];
    let dx = p[0] - closest[0];
    let dy = p[1] - closest[1];
    ((dx * dx) + (dy * dy)).sqrt()
}

fn rotate_uv_around_center(p: [f32; 2], angle: f32) -> [f32; 2] {
    let local = [p[0] - 0.5, p[1] - 0.5];
    let (sin, cos) = angle.sin_cos();
    [
        0.5 + (local[0] * cos) - (local[1] * sin),
        0.5 + (local[0] * sin) + (local[1] * cos),
    ]
}

fn morph_factor(t: f32) -> f32 {
    let phase = t.clamp(0.0, 1.0).min(0.999_023_44);
    let t4 = phase * 4.0;
    let segment = t4.floor();
    let frac_part = t4 - segment;
    let slope = if segment >= 2.0 { -1.0 } else { 1.0 };
    let offset = if segment < 1.0 {
        0.0
    } else if segment < 2.0 {
        1.0
    } else if segment < 3.0 {
        2.0
    } else {
        1.0
    };
    offset + (slope * frac_part)
}

fn morphed_arc_point(a0: f32, a1: f32, s: f32, m: f32, radius: f32) -> [f32; 2] {
    let theta = a0 + (s * (a1 - a0));
    let circle = [radius * theta.cos(), radius * theta.sin()];
    let a = [radius * a0.cos(), radius * a0.sin()];
    let b = [radius * a1.cos(), radius * a1.sin()];
    let chord = [a[0] + ((b[0] - a[0]) * s), a[1] + ((b[1] - a[1]) * s)];
    [
        circle[0] + (m * (chord[0] - circle[0])),
        circle[1] + (m * (chord[1] - circle[1])),
    ]
}

fn morphed_ring_distance_single(
    p: [f32; 2],
    phase01: f32,
    ring_radius: f32,
    ring_thickness: f32,
) -> f32 {
    let m = morph_factor(phase01);
    let safe_radius = ring_radius.max(1.0e-4);
    let safe_thickness = ring_thickness.max(0.0).min(safe_radius * 0.99);
    let mid_radius = (safe_radius - (0.5 * safe_thickness)).max(1.0e-4);
    let mut min_distance = f32::MAX;
    let center = [0.5, 0.5];
    let arc_segments = 8;

    for arc in 0..3 {
        let a0 = arc as f32 * (core::f32::consts::TAU / 3.0);
        let a1 = (arc as f32 + 1.0) * (core::f32::consts::TAU / 3.0);
        let start = morphed_arc_point(a0, a1, 0.0, m, mid_radius);
        let mut previous = [center[0] + start[0], center[1] + start[1]];
        for index in 1..=arc_segments {
            let s = index as f32 / arc_segments as f32;
            let point = morphed_arc_point(a0, a1, s, m, mid_radius);
            let current = [center[0] + point[0], center[1] + point[1]];
            min_distance = min_distance.min(segment_distance(p, previous, current));
            previous = current;
        }
    }

    min_distance
}

fn morphed_ring_distance(
    p: [f32; 2],
    phase01: f32,
    ring_radius: f32,
    ring_thickness: f32,
    dual_offset_degrees: f32,
) -> f32 {
    let full_offset = dual_offset_degrees.to_radians();
    let dynamic_offset = full_offset * ((phase01 * 2.0) - 1.0).abs();
    let half_offset = 0.5 * dynamic_offset;
    let p_a = rotate_uv_around_center(p, -half_offset);
    let p_b = rotate_uv_around_center(p, half_offset);
    morphed_ring_distance_single(p_a, phase01, ring_radius, ring_thickness).min(
        morphed_ring_distance_single(p_b, phase01, ring_radius, ring_thickness),
    )
}

/// Parameters for a deterministic subdivided icosphere topology.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IcosphereTopologyConfig {
    pub recursion_level: u32,
    pub x_axis_rotation_degrees: f32,
    pub small_world_neighbors_per_point: usize,
    pub small_world_seed: u64,
}

impl Default for IcosphereTopologyConfig {
    fn default() -> Self {
        Self {
            recursion_level: 2,
            x_axis_rotation_degrees: 0.0,
            small_world_neighbors_per_point: 0,
            small_world_seed: 12_345,
        }
    }
}

/// Unit-sphere vertex positions plus deterministic neighbor tiers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct IcosphereTopology {
    pub positions: Vec<Vec3>,
    pub first_tier_neighbors: Vec<Vec<usize>>,
    pub second_tier_neighbors: Vec<Vec<usize>>,
    pub third_tier_neighbors: Vec<Vec<usize>>,
    pub small_world_neighbors: Vec<Vec<usize>>,
}

impl IcosphereTopology {
    pub fn generate(config: IcosphereTopologyConfig) -> Self {
        let recursion_level = config.recursion_level.min(MAX_ICOSPHERE_RECURSION_LEVEL);
        let (mut positions, triangles) = generate_icosphere(recursion_level);
        rotate_positions_x(&mut positions, config.x_axis_rotation_degrees);
        let first_tier_neighbors = build_first_tier_from_triangles(positions.len(), &triangles);
        let (second_tier_neighbors, third_tier_neighbors) = build_tier_rings(&first_tier_neighbors);
        let small_world_neighbors = build_small_world_topology(
            positions.len(),
            config.small_world_neighbors_per_point,
            config.small_world_seed,
        );

        Self {
            positions,
            first_tier_neighbors,
            second_tier_neighbors,
            third_tier_neighbors,
            small_world_neighbors,
        }
    }

    pub fn oscillator_count(&self) -> usize {
        self.positions.len()
    }

    pub fn point_count(&self) -> usize {
        self.positions.len()
    }
}

/// Fixed-step accumulator for deterministic simulation loops.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedStepClock {
    pub step_seconds: f32,
    pub max_steps_per_frame: u32,
    accumulator_seconds: f32,
}

impl FixedStepClock {
    pub const fn new(step_seconds: f32, max_steps_per_frame: u32) -> Self {
        Self {
            step_seconds,
            max_steps_per_frame,
            accumulator_seconds: 0.0,
        }
    }

    pub fn advance(&mut self, delta_seconds: f32) -> u32 {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 || self.step_seconds <= 0.0 {
            return 0;
        }

        self.accumulator_seconds += delta_seconds;
        let mut steps = 0;
        while self.accumulator_seconds >= self.step_seconds && steps < self.max_steps_per_frame {
            self.accumulator_seconds -= self.step_seconds;
            steps += 1;
        }

        if steps == self.max_steps_per_frame && self.accumulator_seconds >= self.step_seconds {
            self.accumulator_seconds = self.accumulator_seconds.min(self.step_seconds);
        }

        steps
    }

    pub const fn accumulator_seconds(self) -> f32 {
        self.accumulator_seconds
    }
}

fn sample_sdf_for_attraction(
    position: Vec3,
    sdf: &PackedSdfGrid,
    config: SdfParticleAttractionConfig,
) -> Option<PackedSdfSample> {
    if config.max_extrapolation_meters > 0.0 {
        return sdf.sample_extrapolated(
            position,
            config.sample_mode,
            config.max_extrapolation_meters,
        );
    }
    sdf.sample(position, config.sample_mode)
}

fn sdf_attraction_acceleration(
    distance_meters: f32,
    normal: Vec3,
    velocity: Vec3,
    config: SdfParticleAttractionConfig,
) -> Vec3 {
    match config.mode {
        SdfParticleAttractionMode::None => Vec3::ZERO,
        SdfParticleAttractionMode::AttractToSurface => {
            let attraction_distance = config.attraction_distance_meters.max(0.000_1);
            let abs_distance = distance_meters.abs();
            if abs_distance > attraction_distance {
                return Vec3::ZERO;
            }

            let to_surface = if distance_meters >= 0.0 {
                -normal
            } else {
                normal
            };
            let falloff = 1.0 - (abs_distance / attraction_distance).clamp(0.0, 1.0);
            to_surface * (config.strength * falloff * falloff)
        }
        SdfParticleAttractionMode::GaussianSurfaceAttract => {
            let attraction_distance = config.attraction_distance_meters.max(0.000_1);
            let abs_distance = distance_meters.abs();
            let sigma = config.gaussian_sigma_meters.max(0.000_1);
            let range_fade = soft_sdf_range_fade(abs_distance, attraction_distance, sigma);
            if range_fade <= 0.0 {
                return Vec3::ZERO;
            }

            let normalized_distance = distance_meters / sigma;
            let gaussian = (-0.5 * normalized_distance * normalized_distance).exp() * range_fade;
            let spring = -normal * (distance_meters * config.strength.max(0.0) * gaussian);
            let normal_speed = velocity.dot(normal);
            let damping = -normal * (normal_speed * config.normal_damping.max(0.0) * gaussian);
            spring + damping
        }
    }
}

fn soft_sdf_range_fade(
    abs_distance_meters: f32,
    attraction_distance_meters: f32,
    sigma_meters: f32,
) -> f32 {
    if abs_distance_meters <= attraction_distance_meters {
        return 1.0;
    }

    let edge_width = (attraction_distance_meters * 0.35)
        .max(sigma_meters)
        .max(0.000_1);
    let t = ((abs_distance_meters - attraction_distance_meters) / edge_width).clamp(0.0, 1.0);
    1.0 - (t * t * (3.0 - 2.0 * t))
}

fn clamp_particle_speed(velocity: Vec3, max_speed_meters_per_second: f32) -> Vec3 {
    let max_speed = max_speed_meters_per_second.max(0.0);
    if max_speed <= 1.0e-5 {
        return velocity;
    }
    velocity.clamped_length(max_speed)
}

#[derive(Clone, Copy, Debug)]
struct SurfaceTriangleRecord {
    indices: [usize; 3],
    triangle_index: usize,
    normal: Vec3,
    cumulative_area: f32,
}

fn triangle_area(vertices: &[Vec3], triangle: [usize; 3]) -> Option<f32> {
    let [a, b, c] = triangle;
    if a >= vertices.len() || b >= vertices.len() || c >= vertices.len() {
        return None;
    }

    let area = (vertices[b] - vertices[a])
        .cross(vertices[c] - vertices[a])
        .length()
        * 0.5;
    if area.is_finite() && area > 1.0e-9 {
        Some(area)
    } else {
        None
    }
}

fn mesh_surface_triangle_records(mesh: &TriangleMeshSurface) -> Vec<SurfaceTriangleRecord> {
    let mut records = Vec::new();
    let mut cumulative_area = 0.0_f32;
    for (triangle_index, indices) in mesh.triangles.iter().copied().enumerate() {
        let Some(area) = triangle_area(mesh.vertices.as_slice(), indices) else {
            continue;
        };

        let [a, b, c] = indices;
        if !mesh.vertices[a].is_finite()
            || !mesh.vertices[b].is_finite()
            || !mesh.vertices[c].is_finite()
        {
            continue;
        }

        let normal = (mesh.vertices[b] - mesh.vertices[a])
            .cross(mesh.vertices[c] - mesh.vertices[a])
            .normalized_or(Vec3::UP);
        cumulative_area += area;
        records.push(SurfaceTriangleRecord {
            indices,
            triangle_index,
            normal,
            cumulative_area,
        });
    }
    records
}

fn evaluate_surface_anchor(
    mesh: &TriangleMeshSurface,
    triangle_index: usize,
    barycentric: [f32; 3],
) -> Option<(Vec3, Vec3)> {
    if !barycentric.iter().all(|value| value.is_finite()) {
        return None;
    }
    let indices = *mesh.triangles.get(triangle_index)?;
    let [a, b, c] = indices;
    if a >= mesh.vertices.len() || b >= mesh.vertices.len() || c >= mesh.vertices.len() {
        return None;
    }

    let v0 = mesh.vertices[a];
    let v1 = mesh.vertices[b];
    let v2 = mesh.vertices[c];
    if !v0.is_finite() || !v1.is_finite() || !v2.is_finite() {
        return None;
    }

    let normal = (v1 - v0).cross(v2 - v0).normalized_or(Vec3::ZERO);
    if normal == Vec3::ZERO {
        return None;
    }

    Some((
        (v0 * barycentric[0]) + (v1 * barycentric[1]) + (v2 * barycentric[2]),
        normal,
    ))
}

fn stratified_area_target(sample_index: usize, point_count: usize, total_area: f32) -> f32 {
    let unit = (sample_index as f32 + 0.5) / point_count.max(1) as f32;
    (unit * total_area).min(total_area)
}

fn select_surface_triangle(records: &[SurfaceTriangleRecord], area_target: f32) -> usize {
    let mut low = 0_usize;
    let mut high = records.len();
    while low < high {
        let mid = low + ((high - low) / 2);
        if area_target <= records[mid].cumulative_area {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low.min(records.len().saturating_sub(1))
}

fn sample_barycentric(local_index: usize, seed: u64, triangle_index: usize) -> [f32; 3] {
    let u = quasirandom01(local_index, seed, triangle_index, 0);
    let v = quasirandom01(local_index, seed, triangle_index, 1);
    let sqrt_u = u.sqrt();
    [1.0 - sqrt_u, sqrt_u * (1.0 - v), sqrt_u * v]
}

fn quasirandom01(local_index: usize, seed: u64, triangle_index: usize, axis: u32) -> f32 {
    let seed_mix = (seed as u32)
        ^ ((seed >> 32) as u32)
        ^ (triangle_index as u32).wrapping_mul(0x9E37_79B9)
        ^ axis.wrapping_mul(0x85EB_CA6B);
    let offset = hash01(seed_mix);
    let step = if axis == 0 { 0.618_034 } else { 0.754_877_7 };
    ((local_index as f32 + 0.5) * step + offset)
        .fract()
        .clamp(1.0e-6, 0.999_999)
}

fn build_nearest_neighbor_tiers(
    positions: &[Vec3],
    first_tier_count: usize,
    second_tier_count: usize,
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let point_count = positions.len();
    if point_count == 0 {
        return (Vec::new(), Vec::new());
    }

    let first_tier_count = first_tier_count.min(point_count.saturating_sub(1));
    let second_tier_count = second_tier_count.min(
        point_count
            .saturating_sub(1)
            .saturating_sub(first_tier_count),
    );
    let mut first_tier = Vec::with_capacity(point_count);
    let mut second_tier = Vec::with_capacity(point_count);

    for origin in 0..point_count {
        let mut distances = Vec::with_capacity(point_count.saturating_sub(1));
        for candidate in 0..point_count {
            if candidate == origin {
                continue;
            }
            let distance = (positions[origin] - positions[candidate]).length_squared();
            let distance = if distance.is_finite() {
                distance
            } else {
                f32::INFINITY
            };
            distances.push((distance, candidate));
        }
        distances.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));

        first_tier.push(
            distances
                .iter()
                .take(first_tier_count)
                .map(|(_, index)| *index)
                .collect(),
        );
        second_tier.push(
            distances
                .iter()
                .skip(first_tier_count)
                .take(second_tier_count)
                .map(|(_, index)| *index)
                .collect(),
        );
    }

    (first_tier, second_tier)
}

fn build_cross_neighbor_lists(
    source_positions: &[Vec3],
    target_positions: &[Vec3],
    neighbors_per_point: usize,
    max_distance_squared: f32,
) -> Vec<Vec<usize>> {
    if source_positions.is_empty() {
        return Vec::new();
    }
    if target_positions.is_empty() || neighbors_per_point == 0 {
        return vec![Vec::new(); source_positions.len()];
    }

    let mut all_neighbors = Vec::with_capacity(source_positions.len());
    for source in source_positions {
        let mut distances = Vec::with_capacity(target_positions.len());
        for (target_index, target) in target_positions.iter().copied().enumerate() {
            let distance = (*source - target).length_squared();
            if distance.is_finite() && distance <= max_distance_squared {
                distances.push((distance, target_index));
            }
        }
        distances.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));
        all_neighbors.push(
            distances
                .iter()
                .take(neighbors_per_point)
                .map(|(_, index)| *index)
                .collect(),
        );
    }

    all_neighbors
}

fn neighbor_list_is_valid(origin: usize, count: usize, neighbors: &[usize]) -> bool {
    let mut seen = HashSet::<usize>::new();
    neighbors
        .iter()
        .all(|neighbor| *neighbor < count && *neighbor != origin && seen.insert(*neighbor))
}

fn neighbor_targets_are_valid(count: usize, neighbors: &[usize]) -> bool {
    let mut seen = HashSet::<usize>::new();
    neighbors
        .iter()
        .all(|neighbor| *neighbor < count && seen.insert(*neighbor))
}

fn mesh_surface_index_hash(indices: &[[usize; 3]]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for triangle in indices {
        for index in triangle {
            for byte in index.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    hash
}

fn hand_mesh_index_hash(indices: &[[u32; 3]]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for triangle in indices {
        for index in triangle {
            for byte in index.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
    }
    hash
}

fn append_oriented_box(
    mesh: &mut TriangleMeshSurface,
    center: Vec3,
    half_extents: Vec3,
    z_rotation_radians: f32,
) {
    let base = mesh.vertices.len();
    let local_corners = [
        Vec3::new(-half_extents.x, -half_extents.y, -half_extents.z),
        Vec3::new(half_extents.x, -half_extents.y, -half_extents.z),
        Vec3::new(half_extents.x, half_extents.y, -half_extents.z),
        Vec3::new(-half_extents.x, half_extents.y, -half_extents.z),
        Vec3::new(-half_extents.x, -half_extents.y, half_extents.z),
        Vec3::new(half_extents.x, -half_extents.y, half_extents.z),
        Vec3::new(half_extents.x, half_extents.y, half_extents.z),
        Vec3::new(-half_extents.x, half_extents.y, half_extents.z),
    ];
    let (sin, cos) = z_rotation_radians.sin_cos();
    for corner in local_corners {
        let rotated = Vec3::new(
            (corner.x * cos) - (corner.y * sin),
            (corner.x * sin) + (corner.y * cos),
            corner.z,
        );
        mesh.vertices.push(center + rotated);
    }

    const BOX_TRIANGLES: [[usize; 3]; 12] = [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [3, 6, 2],
        [3, 7, 6],
        [0, 4, 7],
        [0, 7, 3],
        [1, 2, 6],
        [1, 6, 5],
    ];
    mesh.triangles.extend(
        BOX_TRIANGLES
            .iter()
            .map(|triangle| [base + triangle[0], base + triangle[1], base + triangle[2]]),
    );
}

fn hash01(value: u32) -> f32 {
    let mut x = value;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x & 0x00FF_FFFF) as f32 / 16_777_215.0
}

fn generate_icosphere(recursion_level: u32) -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let t = (1.0 + 5.0_f32.sqrt()) * 0.5;
    let mut vertices = vec![
        Vec3::new(-1.0, t, 0.0),
        Vec3::new(1.0, t, 0.0),
        Vec3::new(-1.0, -t, 0.0),
        Vec3::new(1.0, -t, 0.0),
        Vec3::new(0.0, -1.0, t),
        Vec3::new(0.0, 1.0, t),
        Vec3::new(0.0, -1.0, -t),
        Vec3::new(0.0, 1.0, -t),
        Vec3::new(t, 0.0, -1.0),
        Vec3::new(t, 0.0, 1.0),
        Vec3::new(-t, 0.0, -1.0),
        Vec3::new(-t, 0.0, 1.0),
    ];
    for vertex in &mut vertices {
        *vertex = vertex.normalized_or(Vec3::UP);
    }

    let mut triangles = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    for _ in 0..recursion_level {
        let mut midpoint_cache = HashMap::<(usize, usize), usize>::new();
        let mut next_triangles = Vec::with_capacity(triangles.len() * 4);
        for [a, b, c] in triangles {
            let ab = midpoint_index(&mut vertices, &mut midpoint_cache, a, b);
            let bc = midpoint_index(&mut vertices, &mut midpoint_cache, b, c);
            let ca = midpoint_index(&mut vertices, &mut midpoint_cache, c, a);
            next_triangles.push([a, ab, ca]);
            next_triangles.push([b, bc, ab]);
            next_triangles.push([c, ca, bc]);
            next_triangles.push([ab, bc, ca]);
        }
        triangles = next_triangles;
    }

    (vertices, triangles)
}

fn midpoint_index(
    vertices: &mut Vec<Vec3>,
    cache: &mut HashMap<(usize, usize), usize>,
    a: usize,
    b: usize,
) -> usize {
    let key = if a < b { (a, b) } else { (b, a) };
    if let Some(index) = cache.get(&key) {
        return *index;
    }
    let vertex = ((vertices[a] + vertices[b]) * 0.5).normalized_or(Vec3::UP);
    let index = vertices.len();
    vertices.push(vertex);
    cache.insert(key, index);
    index
}

fn rotate_positions_x(positions: &mut [Vec3], degrees: f32) {
    if degrees.abs() <= f32::EPSILON {
        return;
    }
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    for position in positions {
        let y = position.y * cos - position.z * sin;
        let z = position.y * sin + position.z * cos;
        *position = Vec3::new(position.x, y, z).normalized_or(Vec3::UP);
    }
}

fn build_first_tier_from_triangles(count: usize, triangles: &[[usize; 3]]) -> Vec<Vec<usize>> {
    let mut sets = vec![HashSet::<usize>::new(); count];
    for [a, b, c] in triangles {
        sets[*a].insert(*b);
        sets[*a].insert(*c);
        sets[*b].insert(*a);
        sets[*b].insert(*c);
        sets[*c].insert(*a);
        sets[*c].insert(*b);
    }
    sets.into_iter()
        .map(|set| {
            let mut neighbors: Vec<_> = set.into_iter().collect();
            neighbors.sort_unstable();
            neighbors
        })
        .collect()
}

fn build_tier_rings(first_tier_neighbors: &[Vec<usize>]) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let count = first_tier_neighbors.len();
    let mut second_tier = Vec::with_capacity(count);
    let mut third_tier = Vec::with_capacity(count);

    for origin in 0..count {
        let first_set: HashSet<_> = first_tier_neighbors[origin].iter().copied().collect();
        let mut second_set = HashSet::<usize>::new();
        for first_neighbor in &first_tier_neighbors[origin] {
            for candidate in &first_tier_neighbors[*first_neighbor] {
                if *candidate != origin && !first_set.contains(candidate) {
                    second_set.insert(*candidate);
                }
            }
        }
        let mut second: Vec<_> = second_set.iter().copied().collect();
        second.sort_unstable();

        let mut third_set = HashSet::<usize>::new();
        for second_neighbor in &second {
            for candidate in &first_tier_neighbors[*second_neighbor] {
                if *candidate != origin
                    && !first_set.contains(candidate)
                    && !second_set.contains(candidate)
                {
                    third_set.insert(*candidate);
                }
            }
        }
        let mut third: Vec<_> = third_set.into_iter().collect();
        third.sort_unstable();

        second_tier.push(second);
        third_tier.push(third);
    }

    (second_tier, third_tier)
}

fn build_small_world_topology(
    count: usize,
    neighbors_per_point: usize,
    seed: u64,
) -> Vec<Vec<usize>> {
    if count == 0 || neighbors_per_point == 0 {
        return vec![Vec::new(); count];
    }

    let mut rng = DeterministicRng::new(seed);
    let mut neighbors = vec![Vec::<usize>::new(); count];
    for (origin, point_neighbors) in neighbors.iter_mut().enumerate() {
        let mut used = HashSet::<usize>::new();
        while used.len() < neighbors_per_point.min(count.saturating_sub(1)) {
            let candidate = rng.next_usize(count);
            if candidate != origin && used.insert(candidate) {
                point_neighbors.push(candidate);
            }
        }
        point_neighbors.sort_unstable();
    }

    neighbors
}

#[derive(Clone, Copy, Debug)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    const fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    fn next_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive <= 1 {
            return 0;
        }
        (self.next_u64() as usize) % upper_exclusive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn seeded_sphere_has_valid_layout() {
        let particles = ParticleSet::seed_sphere(32, Vec3::ZERO, 0.5, 0.01);

        assert_eq!(particles.len(), 32);
        assert!(particles.validate_layout());
        assert!(particles.positions.iter().all(|p| p.length() <= 0.5));
    }

    #[test]
    fn render_payload_contains_particle_count_counter() {
        let particles = ParticleSet::seed_sphere(4, Vec3::ZERO, 0.5, 0.01);
        let payload = particles.render_payload(9, RenderCoordinateSpace::World);

        assert_eq!(payload.frame_index, 9);
        assert_eq!(payload.points.len(), 4);
        assert!(payload.is_valid());
    }

    #[test]
    fn render_particles_convert_to_point_payload() {
        let mut particle = ParticleRender::new(Vec3::UP, 0.08, ColorRgba::WHITE);
        particle.flags = 7;
        particle.frame01 = 0.25;

        let payload = render_particles_to_payload(3, RenderCoordinateSpace::World, &[particle]);

        assert_eq!(payload.frame_index, 3);
        assert_eq!(payload.points.len(), 1);
        assert_eq!(payload.points[0].radius_meters, 0.04);
        assert_eq!(payload.points[0].flags, 7);
        assert!(payload.is_valid());
    }

    #[test]
    fn sdf_attraction_moves_particle_toward_surface() {
        let sdf = PackedSdfGrid::sphere(
            1,
            Vec3::ZERO,
            0.2,
            Vec3::new(-1.0, -1.0, -1.0),
            0.05,
            [40, 40, 40],
        )
        .expect("sphere SDF should build");
        let mut particles = ParticleSet::with_capacity(1);
        particles.push_state(ParticleState::new(Vec3::new(0.55, 0.0, 0.0), 0.01));

        let stats = step_particles_toward_sdf(
            &mut particles,
            &sdf,
            1.0 / 60.0,
            SdfParticleAttractionConfig {
                strength: 12.0,
                attraction_distance_meters: 0.6,
                max_speed_meters_per_second: 2.0,
                ..SdfParticleAttractionConfig::default()
            },
        );

        assert_eq!(stats.sampled_count, 1);
        assert_eq!(stats.affected_count, 1);
        assert!(particles.positions[0].x < 0.55);
    }

    #[test]
    fn builds_mesh_sdf_particle_attraction_scenario() {
        let camera = CameraPose::new(Vec3::ZERO, Vec3::FORWARD_NEG_Z, Vec3::UP);
        let surface = build_fixture_hand_mesh(FixtureHandMeshConfig::default());
        let mesh = surface
            .to_triangle_mesh_snapshot(3)
            .expect("fixture hand should convert to SDF mesh");
        let scenario = build_mesh_sdf_particle_attraction_scenario(
            3,
            camera,
            &mesh,
            MeshSdfParticleAttractionScenarioConfig {
                spawn: ParticleSphereSpawnConfig {
                    count: 32,
                    distance_meters: 0.35,
                    radius_meters: 0.08,
                    particle_radius_meters: 0.005,
                    vertical_offset_meters: 0.0,
                    yaw_only: true,
                },
                mesh_sdf: MeshToSdfConfig {
                    voxel_size_meters: 0.04,
                    padding_meters: 0.1,
                    max_voxels: 64 * 64 * 64,
                    sign_mode: MeshSdfSignMode::TriangleNormal,
                    ..MeshToSdfConfig::default()
                },
                ..MeshSdfParticleAttractionScenarioConfig::default()
            },
        )
        .expect("scenario should build");

        assert_eq!(scenario.particles.len(), 32);
        assert!(scenario.sdf.voxel_count() > 0);
        assert!(scenario.simulation_bounds.is_valid());
    }

    #[test]
    fn mesh_surface_sampler_returns_exact_count_and_neighbors() {
        let mesh = TriangleMeshSurface::new(
            vec![
                Vec3::new(-1.0, -1.0, 0.0),
                Vec3::new(1.0, -1.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(-1.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        );

        let samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
            point_count: 16,
            first_tier_neighbor_count: 3,
            second_tier_neighbor_count: 4,
            seed: 19,
        });

        assert_eq!(samples.point_count(), 16);
        assert!(samples.is_valid());
        assert!(samples
            .samples
            .iter()
            .all(|sample| sample.position.z.abs() < 1.0e-6));
        assert!(samples
            .first_tier_neighbors
            .iter()
            .all(|neighbors| neighbors.len() == 3));
        assert!(samples
            .second_tier_neighbors
            .iter()
            .all(|neighbors| neighbors.len() == 4));
    }

    #[test]
    fn mesh_surface_sampler_is_deterministic() {
        let mesh = TriangleMeshSurface::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let config = MeshSurfaceSampleConfig {
            point_count: 8,
            first_tier_neighbor_count: 2,
            second_tier_neighbor_count: 2,
            seed: 77,
        };

        let first = sample_mesh_surface_points(&mesh, config);
        let second = sample_mesh_surface_points(&mesh, config);

        assert_eq!(first, second);
    }

    #[test]
    fn live_mesh_surface_sampler_updates_deformed_mesh() {
        let mesh = TriangleMeshSurface::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let mut sampler = LiveMeshSurfaceSampler::new(MeshSurfaceSampleConfig {
            point_count: 6,
            first_tier_neighbor_count: 2,
            second_tier_neighbor_count: 2,
            seed: 101,
        });

        let first = sampler.update_from_mesh(&mesh);
        let first_key = sampler.topology_key();
        let first_positions = sampler.samples().positions();
        let first_neighbors = sampler.samples().first_tier_neighbors.clone();

        assert_eq!(first.status, LiveMeshSurfaceUpdateStatus::Initialized);
        assert_eq!(first.sample_count, 6);
        assert_eq!(first.topology_key, first_key);

        let offset = Vec3::new(0.25, -0.5, 0.75);
        let deformed = TriangleMeshSurface::new(
            mesh.vertices
                .iter()
                .copied()
                .map(|vertex| vertex + offset)
                .collect(),
            mesh.triangles.clone(),
        );
        let second = sampler.update_from_mesh(&deformed);

        assert_eq!(second.status, LiveMeshSurfaceUpdateStatus::Updated);
        assert_eq!(second.topology_key, first_key);
        assert_eq!(sampler.samples().first_tier_neighbors, first_neighbors);
        for (old_position, sample) in first_positions
            .iter()
            .copied()
            .zip(sampler.samples().samples.iter())
        {
            assert!((sample.position - (old_position + offset)).length() < 1.0e-5);
        }
    }

    #[test]
    fn live_mesh_surface_sampler_resamples_changed_topology() {
        let initial = TriangleMeshSurface::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let changed = TriangleMeshSurface::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        );
        let mut sampler = LiveMeshSurfaceSampler::new(MeshSurfaceSampleConfig {
            point_count: 8,
            first_tier_neighbor_count: 2,
            second_tier_neighbor_count: 3,
            seed: 202,
        });

        let first = sampler.update_from_mesh(&initial);
        let first_key = sampler.topology_key();
        let second = sampler.update_from_mesh(&changed);

        assert_eq!(first.status, LiveMeshSurfaceUpdateStatus::Initialized);
        assert_eq!(
            second.status,
            LiveMeshSurfaceUpdateStatus::ResampledTopology
        );
        assert_ne!(sampler.topology_key(), first_key);
        assert_eq!(sampler.samples().point_count(), 8);
        assert!(sampler.samples().is_valid());
    }

    #[test]
    fn live_mesh_surface_sampler_updates_from_provider_frames() {
        struct SequenceProvider {
            meshes: Vec<TriangleMeshSurface>,
            cursor: usize,
        }

        impl MeshSurfaceProvider for SequenceProvider {
            fn next_mesh_surface(&mut self) -> Option<TriangleMeshSurface> {
                let mesh = self.meshes.get(self.cursor).cloned();
                self.cursor += usize::from(mesh.is_some());
                mesh
            }
        }

        let initial = TriangleMeshSurface::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let deformed = TriangleMeshSurface::new(
            initial
                .vertices
                .iter()
                .copied()
                .map(|vertex| vertex + Vec3::new(0.0, 0.1, 0.2))
                .collect(),
            initial.triangles.clone(),
        );
        let mut provider = SequenceProvider {
            meshes: vec![initial, deformed],
            cursor: 0,
        };
        let mut sampler = LiveMeshSurfaceSampler::new(MeshSurfaceSampleConfig {
            point_count: 5,
            ..MeshSurfaceSampleConfig::default()
        });

        let first = sampler.update_from_provider(&mut provider);
        let second = sampler.update_from_provider(&mut provider);
        let third = sampler.update_from_provider(&mut provider);

        assert_eq!(first.status, LiveMeshSurfaceUpdateStatus::Initialized);
        assert_eq!(second.status, LiveMeshSurfaceUpdateStatus::Updated);
        assert_eq!(third.status, LiveMeshSurfaceUpdateStatus::NoMesh);
        assert_eq!(third.sample_count, 5);
    }

    #[test]
    fn mesh_surface_samples_update_from_deformed_mesh() {
        let mesh = TriangleMeshSurface::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let mut samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
            point_count: 8,
            first_tier_neighbor_count: 2,
            second_tier_neighbor_count: 2,
            seed: 17,
        });
        let before = samples.positions();
        let offset = Vec3::new(0.5, -0.25, 1.25);
        let deformed = TriangleMeshSurface::new(
            mesh.vertices
                .iter()
                .copied()
                .map(|vertex| vertex + offset)
                .collect(),
            mesh.triangles.clone(),
        );

        assert!(samples.update_positions_from_mesh(&deformed));
        for (old_position, sample) in before.iter().copied().zip(samples.samples.iter()) {
            let expected = old_position + offset;
            assert!((sample.position - expected).length() < 1.0e-5);
            assert_eq!(sample.normal, Vec3::FORWARD_NEG_Z * -1.0);
        }
    }

    #[test]
    fn mesh_surface_sample_update_rejects_changed_topology() {
        let mesh = TriangleMeshSurface::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let mut samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
            point_count: 4,
            ..MeshSurfaceSampleConfig::default()
        });
        let before = samples.clone();
        let changed_topology = TriangleMeshSurface::new(mesh.vertices.clone(), Vec::new());

        assert!(!samples.update_positions_from_mesh(&changed_topology));
        assert_eq!(samples, before);
    }

    #[test]
    fn hand_mesh_snapshot_converts_to_sample_surface() {
        let snapshot = HandMeshSnapshot::new(
            7,
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );

        let mesh = triangle_mesh_surface_from_hand_mesh_snapshot(&snapshot)
            .expect("valid snapshot should convert");
        let samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
            point_count: 5,
            first_tier_neighbor_count: 2,
            second_tier_neighbor_count: 1,
            seed: 31,
        });

        assert_eq!(mesh.triangle_count(), 1);
        assert_eq!(samples.point_count(), 5);
        assert!(samples.is_valid());
    }

    #[test]
    fn hand_mesh_snapshot_updates_live_samples() {
        let initial = HandMeshSnapshot::new(
            1,
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let mesh = TriangleMeshSurface::from_hand_mesh_snapshot(&initial)
            .expect("valid snapshot should convert");
        let mut samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
            point_count: 5,
            ..MeshSurfaceSampleConfig::default()
        });
        let before = samples.positions();
        let offset = Vec3::new(0.2, 0.3, 0.4);
        let deformed = HandMeshSnapshot::new(
            2,
            initial
                .vertices
                .iter()
                .copied()
                .map(|vertex| vertex + offset)
                .collect(),
            initial.indices.clone(),
        );

        assert!(samples.update_positions_from_hand_mesh_snapshot(&deformed));
        for (old_position, sample) in before.iter().copied().zip(samples.samples.iter()) {
            assert!((sample.position - (old_position + offset)).length() < 1.0e-5);
        }
    }

    #[test]
    fn live_hand_mesh_sampler_updates_from_provider_frames() {
        struct SequenceProvider {
            snapshots: Vec<HandMeshSnapshot>,
            cursor: usize,
        }

        impl HandMeshSnapshotProvider for SequenceProvider {
            fn next_hand_mesh_snapshot(&mut self) -> Option<HandMeshSnapshot> {
                let snapshot = self.snapshots.get(self.cursor).cloned();
                self.cursor += usize::from(snapshot.is_some());
                snapshot
            }
        }

        let initial = HandMeshSnapshot::new(
            10,
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        )
        .with_handedness(Handedness::Left);
        let offset = Vec3::new(0.1, 0.2, 0.3);
        let deformed = HandMeshSnapshot::new(
            11,
            initial
                .vertices
                .iter()
                .copied()
                .map(|vertex| vertex + offset)
                .collect(),
            initial.indices.clone(),
        )
        .with_handedness(Handedness::Left);
        let mut provider = SequenceProvider {
            snapshots: vec![initial, deformed],
            cursor: 0,
        };
        let mut sampler = LiveHandMeshParticleSampler::new(MeshSurfaceSampleConfig {
            point_count: 6,
            first_tier_neighbor_count: 2,
            second_tier_neighbor_count: 2,
            seed: 99,
        })
        .with_render_style(
            RenderCoordinateSpace::World,
            0.01,
            ColorRgba::new(0.1, 0.7, 1.0, 1.0),
        );

        let first = sampler.update_from_provider(&mut provider);
        let first_positions = sampler.samples().positions();
        let topology_key = first.topology_key;

        assert_eq!(first.status, LiveHandMeshUpdateStatus::Initialized);
        assert_eq!(first.snapshot_version, Some(10));
        assert_eq!(first.sample_count, 6);
        assert_eq!(topology_key, sampler.topology_key());

        let second = sampler.update_from_provider(&mut provider);
        assert_eq!(second.status, LiveHandMeshUpdateStatus::Updated);
        assert_eq!(second.snapshot_version, Some(11));
        assert_eq!(second.topology_key, topology_key);
        for (old_position, sample) in first_positions
            .iter()
            .copied()
            .zip(sampler.samples().samples.iter())
        {
            assert!((sample.position - (old_position + offset)).length() < 1.0e-5);
        }

        let payload = sampler.render_payload(12);
        assert_eq!(payload.points.len(), 6);
        assert!(payload.is_valid());

        let third = sampler.update_from_provider(&mut provider);
        assert_eq!(third.status, LiveHandMeshUpdateStatus::NoSnapshot);
        assert_eq!(third.sample_count, 6);
    }

    #[test]
    fn live_hand_mesh_sampler_resamples_when_topology_changes() {
        let initial = HandMeshSnapshot::new(
            1,
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let changed_topology = HandMeshSnapshot::new(
            2,
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        );
        let mut sampler = LiveHandMeshParticleSampler::new(MeshSurfaceSampleConfig {
            point_count: 8,
            first_tier_neighbor_count: 2,
            second_tier_neighbor_count: 3,
            seed: 7,
        });

        let first = sampler.update_from_snapshot(&initial);
        let first_key = sampler.topology_key();
        let second = sampler.update_from_snapshot(&changed_topology);

        assert_eq!(first.status, LiveHandMeshUpdateStatus::Initialized);
        assert_eq!(second.status, LiveHandMeshUpdateStatus::ResampledTopology);
        assert_ne!(sampler.topology_key(), first_key);
        assert_eq!(sampler.samples().point_count(), 8);
        assert!(sampler.samples().is_valid());
    }

    #[test]
    fn fixture_hand_mesh_samples_convert_to_render_payload() {
        let mesh = build_fixture_hand_mesh(FixtureHandMeshConfig::default());
        assert!(mesh.is_valid());
        assert_eq!(mesh.vertex_count(), 48);
        assert_eq!(mesh.triangle_count(), 72);

        let samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
            point_count: 64,
            first_tier_neighbor_count: 5,
            second_tier_neighbor_count: 7,
            seed: 5,
        });
        let particles = samples.render_particles(0.006, ColorRgba::new(0.2, 0.8, 1.0, 1.0));
        let payload = samples.render_payload(
            12,
            RenderCoordinateSpace::World,
            0.006,
            ColorRgba::new(0.2, 0.8, 1.0, 1.0),
        );

        assert!(samples.is_valid());
        assert_eq!(particles.len(), 64);
        assert_eq!(payload.points.len(), 64);
        assert!(payload.is_valid());
    }

    #[test]
    fn cross_surface_neighborhood_links_two_coordinate_sets() {
        let a_positions = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let b_positions = [
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::new(1.2, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
        ];

        let cross = build_mesh_surface_cross_neighborhood(
            &a_positions,
            &b_positions,
            MeshSurfaceCrossNeighborConfig {
                neighbors_per_point: 1,
                max_distance_meters: 0.35,
            },
        );

        assert!(cross.is_valid(a_positions.len(), b_positions.len()));
        assert_eq!(cross.a_to_b_neighbors, vec![vec![0], vec![1], Vec::new()]);
        assert_eq!(cross.b_to_a_neighbors, vec![vec![0], vec![1], Vec::new()]);
    }

    #[test]
    fn particle_disc_mesh_builds_triangle_fan() {
        let mesh = build_particle_disc_mesh(ParticleDiscMeshConfig::default());

        assert_eq!(mesh.vertices.len(), DEFAULT_PARTICLE_DISC_SEGMENTS + 1);
        assert_eq!(mesh.indices.len(), DEFAULT_PARTICLE_DISC_SEGMENTS * 3);
        assert_eq!(mesh.vertices[0].uv, [0.5, 0.5]);
        assert_eq!(mesh.indices[0..3], [0, 1, 2]);
    }

    #[test]
    fn billboard_instances_apply_scene_basis() {
        let basis = ParticleSceneBasis::new(
            Vec3::new(10.0, 20.0, 30.0),
            Vec3::UP,
            Vec3::RIGHT,
            Vec3::new(0.0, 0.0, -1.0),
            2.0,
        );
        let mut particle = ParticleRender::new(
            Vec3::new(1.0, 2.0, 3.0),
            0.25,
            ColorRgba::new(0.1, 0.2, 0.3, 0.4),
        );
        particle.normal = Vec3::RIGHT;
        particle.frame01 = 1.5;
        particle.rotation_radians = 0.75;
        particle.aux0 = 0.5;
        particle.aux1 = 0.25;
        particle.flags = 9;

        let instance = particle_billboard_instance(particle, basis);

        assert_eq!(instance.position_size, [14.0, 22.0, 24.0, 0.5]);
        assert_eq!(instance.color, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(instance.normal_frame, [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(instance.aux, [0.75, 0.5, 0.25, 9.0]);
    }

    #[test]
    fn billboard_instance_writer_filters_and_sorts() {
        let particles = [
            ParticleRender::new(Vec3::new(0.0, 0.0, 1.0), 0.1, ColorRgba::WHITE),
            ParticleRender::new(Vec3::new(0.0, 0.0, 4.0), 0.1, ColorRgba::WHITE),
            ParticleRender::new(
                Vec3::new(0.0, 0.0, 2.0),
                0.1,
                ColorRgba::new(1.0, 1.0, 1.0, 0.0),
            ),
            ParticleRender::new(Vec3::new(0.0, 0.0, 3.0), 0.1, ColorRgba::WHITE),
        ];
        let mut indices = Vec::new();
        let mut instances = Vec::new();

        let stats = write_particle_billboard_instances(
            &particles,
            ParticleSceneBasis::default(),
            ParticleBillboardBuildConfig {
                sort_back_to_front: true,
                ..ParticleBillboardBuildConfig::default()
            },
            Some(ParticleBillboardSortCamera {
                position: Vec3::ZERO,
                forward: Vec3::FORWARD_NEG_Z,
            }),
            &mut indices,
            &mut instances,
        );

        assert_eq!(stats.source_count, 4);
        assert_eq!(stats.emitted_count, 3);
        assert_eq!(stats.skipped_count, 1);
        assert_eq!(indices, vec![1, 3, 0]);
        assert_eq!(instances[0].position_size[2], -4.0);
        assert_eq!(instances[1].position_size[2], -3.0);
        assert_eq!(instances[2].position_size[2], -1.0);
    }

    #[test]
    fn billboard_budget_accounts_for_disc_indices() {
        let budget = particle_billboard_render_budget(2562, 5124, DEFAULT_PARTICLE_DISC_SEGMENTS);

        assert_eq!(budget.visible_instances, 7686);
        assert_eq!(
            budget.indices_per_instance,
            DEFAULT_PARTICLE_DISC_SEGMENTS * 3
        );
        assert_eq!(
            budget.total_indices,
            budget.visible_instances * budget.indices_per_instance
        );
    }

    #[test]
    fn morphed_ring_atlas_has_expected_layout_and_mask() {
        let config = MorphedRingAtlasConfig {
            frame_resolution: 16,
            frame_count: 8,
            atlas_columns: 4,
            ..MorphedRingAtlasConfig::default()
        };
        let atlas = build_morphed_ring_atlas_rgba(config);

        assert_eq!(atlas.width, 64);
        assert_eq!(atlas.height, 32);
        assert_eq!(atlas.rgba.len(), atlas.width * atlas.height * 4);
        assert!(atlas.rgba.iter().any(|value| *value > 0));
        assert_eq!(morphed_ring_alpha([0.5, 0.5], 0.0, config), 0.0);
    }

    #[test]
    fn particle_trail_emitter_spawns_and_fades() {
        let mut emitter = ParticleTrailEmitter::new(ParticleTrailConfig {
            enabled: true,
            visuals_enabled: true,
            lifetime_seconds: 1.0,
            copies_per_second: 10.0,
            max_spawn_batches_per_frame: 2,
            copies_per_particle: 2,
            size_multiplier: 0.5,
        });
        let source = [ParticleRender::new(Vec3::ZERO, 0.1, ColorRgba::WHITE)];

        let first = emitter.update(0.11, &source);
        assert_eq!(first.len(), 2);
        assert_eq!(emitter.last_emitted_count(), 1);

        let second = emitter.update(0.5, &source);
        let has_faded_particle = second.iter().any(|particle| {
            particle.color.a > 0.0 && particle.color.a < 1.0 && particle.size_meters <= 0.05
        });
        assert_eq!(emitter.last_active_count(), 2);
        assert!(has_faded_particle);
    }

    #[test]
    fn particle_depth_sort_orders_back_to_front() {
        let particles = [
            ParticleRender::new(Vec3::new(0.0, 0.0, -1.0), 0.1, ColorRgba::WHITE),
            ParticleRender::new(Vec3::new(0.0, 0.0, -4.0), 0.1, ColorRgba::WHITE),
            ParticleRender::new(Vec3::new(0.0, 0.0, -2.0), 0.1, ColorRgba::WHITE),
        ];

        let indices =
            sorted_particle_indices_back_to_front(&particles, Vec3::ZERO, Vec3::FORWARD_NEG_Z);

        assert_eq!(indices, vec![1, 2, 0]);
    }

    #[test]
    fn generated_icosphere_matches_expected_counts() {
        for (level, expected) in [(0, 12), (1, 42), (2, 162), (3, 642), (4, 2562)] {
            let topology = IcosphereTopology::generate(IcosphereTopologyConfig {
                recursion_level: level,
                ..IcosphereTopologyConfig::default()
            });
            assert_eq!(topology.point_count(), expected);
        }
    }

    #[test]
    fn icosphere_neighbor_tiers_are_deterministic() {
        let config = IcosphereTopologyConfig {
            recursion_level: 2,
            small_world_neighbors_per_point: 2,
            small_world_seed: 9,
            ..IcosphereTopologyConfig::default()
        };
        let first = IcosphereTopology::generate(config);
        let second = IcosphereTopology::generate(config);

        assert_eq!(first, second);
        assert!(first
            .positions
            .iter()
            .all(|p| (p.length() - 1.0).abs() < 1.0e-5));
        assert!(first.first_tier_neighbors.iter().all(|n| !n.is_empty()));
        assert!(first.small_world_neighbors.iter().all(|n| n.len() == 2));
    }

    #[test]
    fn fixed_step_clock_caps_steps() {
        let mut clock = FixedStepClock::new(0.01, 3);
        let steps = clock.advance(0.10);

        assert_eq!(steps, 3);
        assert!(clock.accumulator_seconds() <= 0.01);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn particle_set_round_trips_with_serde() {
        let particles = ParticleSet::seed_sphere(4, Vec3::ZERO, 0.5, 0.01);

        let encoded = serde_json::to_string(&particles).expect("particles should serialize");
        let decoded: ParticleSet =
            serde_json::from_str(&encoded).expect("particles should deserialize");

        assert_eq!(decoded, particles);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn mesh_surface_samples_round_trip_with_serde() {
        let mesh = build_fixture_hand_mesh(FixtureHandMeshConfig::default());
        let samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
            point_count: 12,
            first_tier_neighbor_count: 3,
            second_tier_neighbor_count: 2,
            seed: 91,
        });

        let encoded = serde_json::to_string(&samples).expect("samples should serialize");
        let decoded: MeshSurfaceSampleSet =
            serde_json::from_str(&encoded).expect("samples should deserialize");

        assert_eq!(decoded, samples);
    }
}
