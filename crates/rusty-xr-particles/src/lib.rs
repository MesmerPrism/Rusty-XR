//! General particle and animation primitives for Rusty XR.
//!
//! This crate intentionally starts with simple, deterministic primitives. It
//! does not include downstream simulation behavior, app scenes, or renderer
//! backend code.
//!
//! Enable the `serde` feature to serialize particle buffers and fixed-step
//! runtime state for fixtures or operator tooling.

use std::collections::{HashMap, HashSet};

pub use rusty_xr_contracts::{
    ColorRgba, RenderCoordinateSpace, RenderPayload, RenderPoint, RuntimeCounters, Vec3,
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
}
