//! General particle and animation primitives for Rusty XR.
//!
//! This crate intentionally starts with simple, deterministic primitives. It
//! does not include downstream simulation behavior, app scenes, or renderer
//! backend code.
//!
//! Enable the `serde` feature to serialize particle buffers and fixed-step
//! runtime state for fixtures or operator tooling.

pub use rusty_xr_contracts::{
    ColorRgba, RenderCoordinateSpace, RenderPayload, RenderPoint, RuntimeCounters, Vec3,
};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        let mut payload = RenderPayload::new(frame_index, coordinate_space);
        payload.points.reserve(self.len());
        for index in 0..self.len() {
            let mut point = RenderPoint::new(
                self.positions[index],
                self.radii_meters[index],
                self.colors[index],
            );
            point.flags = self.flags[index];
            payload.points.push(point);
        }
        payload
            .counters
            .push_count("particle_count", self.len() as u64);
        payload
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
