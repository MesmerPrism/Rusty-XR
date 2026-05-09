//! Engine-neutral eye-data contracts and synthetic samples for Rusty XR.
//!
//! This crate models screen-space gaze samples, headset-local/world-space gaze
//! rays, AOI hits, and derived processor events without depending on a tracker
//! SDK, Unity, OpenXR, Android, LSL, OSC, or a broker runtime.
//!
//! Enable the `serde` feature when samples need to cross process boundaries.
//!
//! ```
//! use rusty_xr_eye_model::{
//!     EyeCoordinateSpace, EyeSampleBase, EyeScreenGazePoint, Vec2,
//! };
//!
//! let base = EyeSampleBase::new(
//!     "synthetic",
//!     "desktop",
//!     1,
//!     1_000,
//!     EyeCoordinateSpace::ScreenNormalized,
//! );
//! let sample = EyeScreenGazePoint::new_screen_normalized(base, Vec2::new(0.5, 0.5));
//!
//! assert!(sample.is_valid());
//! ```

use core::fmt;

pub use rusty_xr_contracts::{Vec2, Vec3};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Versioned JSON schema id for screen-space gaze samples.
pub const EYE_SCREEN_GAZE_POINT_SCHEMA: &str = "rusty.xr.eye.screen.gaze_point.v1";

/// Versioned JSON schema id for XR gaze rays.
pub const EYE_XR_GAZE_RAY_SCHEMA: &str = "rusty.xr.eye.xr.gaze_ray.v1";

/// Versioned JSON schema id for screen-space AOI hit samples.
pub const EYE_SCREEN_AOI_HIT_SCHEMA: &str = "rusty.xr.eye.screen.aoi_hit.v1";

/// Versioned JSON schema id for derived eye processor events.
pub const EYE_PROCESSOR_EVENT_SCHEMA: &str = "rusty.xr.eye.processor.event.v1";

/// Public stream id for screen-space gaze points.
pub const STREAM_EYE_SCREEN_GAZE_POINT: &str = "eye.screen.gaze_point";

/// Public stream id for headset-local gaze rays.
pub const STREAM_EYE_XR_LOCAL_RAY: &str = "eye.xr.local_ray";

/// Public stream id for world-space gaze rays.
pub const STREAM_EYE_XR_WORLD_RAY: &str = "eye.xr.world_ray";

/// Public stream id for screen-space AOI hits.
pub const STREAM_EYE_SCREEN_AOI_HIT: &str = "eye.screen.aoi_hit";

/// Public stream id for fixation processor output.
pub const STREAM_EYE_PROCESSOR_FIXATION: &str = "eye.processor.fixation";

/// Public stream id for dwell processor output.
pub const STREAM_EYE_PROCESSOR_DWELL: &str = "eye.processor.dwell";

/// Public stream id for blink/dropout processor output.
pub const STREAM_EYE_PROCESSOR_BLINK: &str = "eye.processor.blink";

/// Coordinate space used by an eye-data sample.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeCoordinateSpace {
    ScreenNormalized,
    ScreenPixels,
    XrLocal,
    XrWorld,
    SceneObject,
}

/// Eye identity carried when a provider can distinguish eyes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeIdentity {
    Left,
    Right,
    Combined,
}

/// Derived eye processor event kind.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeDerivedKind {
    Fixation,
    Dwell,
    Blink,
}

/// Provider-neutral validity flags for an eye-data sample.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EyeValidityFlags {
    pub sample_valid: bool,
    pub left_valid: bool,
    pub right_valid: bool,
    pub blink: bool,
    pub tracking_lost: bool,
}

impl EyeValidityFlags {
    pub const fn valid() -> Self {
        Self {
            sample_valid: true,
            left_valid: true,
            right_valid: true,
            blink: false,
            tracking_lost: false,
        }
    }

    pub const fn invalid() -> Self {
        Self {
            sample_valid: false,
            left_valid: false,
            right_valid: false,
            blink: false,
            tracking_lost: true,
        }
    }

    pub const fn blink_dropout() -> Self {
        Self {
            sample_valid: false,
            left_valid: false,
            right_valid: false,
            blink: true,
            tracking_lost: true,
        }
    }

    pub const fn is_sample_usable(self) -> bool {
        self.sample_valid && !self.tracking_lost
    }
}

impl Default for EyeValidityFlags {
    fn default() -> Self {
        Self::valid()
    }
}

/// Shared metadata carried by raw and derived eye samples.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EyeSampleBase {
    pub provider_id: String,
    pub source_device_id: String,
    pub sequence_number: u64,
    pub sample_time_ns: u64,
    pub broker_receive_time_ns: Option<u64>,
    pub validity: EyeValidityFlags,
    pub confidence: Option<f32>,
    pub eye: Option<EyeIdentity>,
    pub coordinate_space: EyeCoordinateSpace,
}

impl EyeSampleBase {
    pub fn new(
        provider_id: impl Into<String>,
        source_device_id: impl Into<String>,
        sequence_number: u64,
        sample_time_ns: u64,
        coordinate_space: EyeCoordinateSpace,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            source_device_id: source_device_id.into(),
            sequence_number,
            sample_time_ns,
            broker_receive_time_ns: None,
            validity: EyeValidityFlags::valid(),
            confidence: None,
            eye: None,
            coordinate_space,
        }
    }

    pub const fn with_broker_receive_time_ns(mut self, broker_receive_time_ns: u64) -> Self {
        self.broker_receive_time_ns = Some(broker_receive_time_ns);
        self
    }

    pub const fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub const fn with_eye(mut self, eye: EyeIdentity) -> Self {
        self.eye = Some(eye);
        self
    }

    pub const fn with_validity(mut self, validity: EyeValidityFlags) -> Self {
        self.validity = validity;
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.provider_id.trim().is_empty()
            && !self.source_device_id.trim().is_empty()
            && self.confidence.map(valid_unit_interval).unwrap_or(true)
    }
}

/// Screen-space gaze point from a desktop tracker or app-derived screen mapper.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EyeScreenGazePoint {
    pub schema: String,
    pub base: EyeSampleBase,
    pub display_id: Option<String>,
    pub normalized_point: Vec2,
    pub screen_pixel: Option<Vec2>,
    pub pupil_diameter_mm: Option<f32>,
}

impl EyeScreenGazePoint {
    pub fn new_screen_normalized(base: EyeSampleBase, normalized_point: Vec2) -> Self {
        Self {
            schema: EYE_SCREEN_GAZE_POINT_SCHEMA.to_string(),
            base,
            display_id: None,
            normalized_point,
            screen_pixel: None,
            pupil_diameter_mm: None,
        }
    }

    pub fn with_display_id(mut self, display_id: impl Into<String>) -> Self {
        self.display_id = Some(display_id.into());
        self
    }

    pub const fn with_screen_pixel(mut self, screen_pixel: Vec2) -> Self {
        self.screen_pixel = Some(screen_pixel);
        self
    }

    pub const fn with_pupil_diameter_mm(mut self, pupil_diameter_mm: f32) -> Self {
        self.pupil_diameter_mm = Some(pupil_diameter_mm);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.base.is_valid()
            && matches!(
                self.base.coordinate_space,
                EyeCoordinateSpace::ScreenNormalized | EyeCoordinateSpace::ScreenPixels
            )
            && valid_normalized_point(self.normalized_point)
            && self.screen_pixel.map(valid_pixel_point).unwrap_or(true)
            && self
                .pupil_diameter_mm
                .map(|value| value.is_finite() && value > 0.0)
                .unwrap_or(true)
    }
}

/// Provenance for fields derived by a portable or engine-local processor.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EyeDerivedProvenance {
    pub source_stream_id: String,
    pub processor_id: String,
    pub source_sequence_start: u64,
    pub source_sequence_end: u64,
}

impl EyeDerivedProvenance {
    pub fn new(
        source_stream_id: impl Into<String>,
        processor_id: impl Into<String>,
        source_sequence_start: u64,
        source_sequence_end: u64,
    ) -> Self {
        Self {
            source_stream_id: source_stream_id.into(),
            processor_id: processor_id.into(),
            source_sequence_start,
            source_sequence_end,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.source_stream_id.trim().is_empty()
            && !self.processor_id.trim().is_empty()
            && self.source_sequence_end >= self.source_sequence_start
    }
}

/// Optional scene hit associated with a gaze ray.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EyeSceneHit {
    pub target_id: String,
    pub position_m: Vec3,
    pub normal: Option<Vec3>,
    pub distance_m: Option<f32>,
    pub derived_from: EyeDerivedProvenance,
}

impl EyeSceneHit {
    pub fn new(
        target_id: impl Into<String>,
        position_m: Vec3,
        derived_from: EyeDerivedProvenance,
    ) -> Self {
        Self {
            target_id: target_id.into(),
            position_m,
            normal: None,
            distance_m: None,
            derived_from,
        }
    }

    pub const fn with_normal(mut self, normal: Vec3) -> Self {
        self.normal = Some(normal);
        self
    }

    pub const fn with_distance_m(mut self, distance_m: f32) -> Self {
        self.distance_m = Some(distance_m);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.target_id.trim().is_empty()
            && self.position_m.is_finite()
            && self
                .normal
                .map(|normal| normal.is_finite() && normal.length_squared() > 1.0e-12)
                .unwrap_or(true)
            && self
                .distance_m
                .map(|distance| distance.is_finite() && distance >= 0.0)
                .unwrap_or(true)
            && self.derived_from.is_valid()
    }
}

/// XR-space gaze ray from a headset bridge or engine-local mapper.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EyeXrGazeRay {
    pub schema: String,
    pub base: EyeSampleBase,
    pub origin_m: Vec3,
    pub direction: Vec3,
    pub scene_hit: Option<EyeSceneHit>,
}

impl EyeXrGazeRay {
    pub fn new(base: EyeSampleBase, origin_m: Vec3, direction: Vec3) -> Self {
        Self {
            schema: EYE_XR_GAZE_RAY_SCHEMA.to_string(),
            base,
            origin_m,
            direction,
            scene_hit: None,
        }
    }

    pub fn with_scene_hit(mut self, scene_hit: EyeSceneHit) -> Self {
        self.scene_hit = Some(scene_hit);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.base.is_valid()
            && matches!(
                self.base.coordinate_space,
                EyeCoordinateSpace::XrLocal | EyeCoordinateSpace::XrWorld
            )
            && self.origin_m.is_finite()
            && self.direction.is_finite()
            && self.direction.length_squared() > 1.0e-12
            && self
                .scene_hit
                .as_ref()
                .map(EyeSceneHit::is_valid)
                .unwrap_or(true)
    }
}

/// Screen-space area-of-interest hit derived from gaze points.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EyeScreenAoiHit {
    pub schema: String,
    pub base: EyeSampleBase,
    pub aoi_id: String,
    pub hit: bool,
    pub dwell_time_ns: Option<u64>,
    pub derived_from: EyeDerivedProvenance,
}

impl EyeScreenAoiHit {
    pub fn new(
        base: EyeSampleBase,
        aoi_id: impl Into<String>,
        hit: bool,
        derived_from: EyeDerivedProvenance,
    ) -> Self {
        Self {
            schema: EYE_SCREEN_AOI_HIT_SCHEMA.to_string(),
            base,
            aoi_id: aoi_id.into(),
            hit,
            dwell_time_ns: None,
            derived_from,
        }
    }

    pub const fn with_dwell_time_ns(mut self, dwell_time_ns: u64) -> Self {
        self.dwell_time_ns = Some(dwell_time_ns);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.base.is_valid()
            && matches!(
                self.base.coordinate_space,
                EyeCoordinateSpace::ScreenNormalized | EyeCoordinateSpace::ScreenPixels
            )
            && !self.aoi_id.trim().is_empty()
            && self.derived_from.is_valid()
    }
}

/// Normalized rectangular area of interest for screen-space gaze processors.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenAoiBounds {
    pub min: Vec2,
    pub max: Vec2,
}

impl ScreenAoiBounds {
    pub const fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn is_valid(self) -> bool {
        valid_normalized_point(self.min)
            && valid_normalized_point(self.max)
            && self.min.x <= self.max.x
            && self.min.y <= self.max.y
    }

    pub fn contains_point(self, point: Vec2) -> bool {
        self.is_valid()
            && valid_normalized_point(point)
            && point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn contains_sample(self, sample: &EyeScreenGazePoint) -> bool {
        sample.is_valid()
            && sample.base.validity.is_sample_usable()
            && self.contains_point(sample.normalized_point)
    }
}

/// Derive one AOI hit sample from one screen-gaze sample.
pub fn derive_screen_aoi_hit(
    sample: &EyeScreenGazePoint,
    aoi_id: impl Into<String>,
    bounds: ScreenAoiBounds,
    processor_id: impl Into<String>,
    dwell_start_time_ns: Option<u64>,
) -> EyeScreenAoiHit {
    let hit = bounds.contains_sample(sample);
    let provenance = EyeDerivedProvenance::new(
        STREAM_EYE_SCREEN_GAZE_POINT,
        processor_id,
        sample.base.sequence_number,
        sample.base.sequence_number,
    );
    let derived = EyeScreenAoiHit::new(sample.base.clone(), aoi_id, hit, provenance);

    if hit {
        if let Some(start_time) = dwell_start_time_ns {
            if let Some(dwell_time) = sample.base.sample_time_ns.checked_sub(start_time) {
                return derived.with_dwell_time_ns(dwell_time);
            }
        }
    }

    derived
}

/// Derive a dwell processor event when all supplied samples stay inside an AOI.
pub fn derive_screen_dwell_event(
    samples: &[EyeScreenGazePoint],
    bounds: ScreenAoiBounds,
    processor_id: impl Into<String>,
    min_duration_ns: u64,
) -> Option<EyeProcessorEvent> {
    let first = samples.first()?;
    let last = samples.last()?;
    if samples
        .iter()
        .any(|sample| !bounds.contains_sample(sample) || !sample.is_valid())
    {
        return None;
    }

    let duration_ns = last
        .base
        .sample_time_ns
        .checked_sub(first.base.sample_time_ns)?;
    if duration_ns < min_duration_ns {
        return None;
    }

    let provenance = EyeDerivedProvenance::new(
        STREAM_EYE_SCREEN_GAZE_POINT,
        processor_id,
        first.base.sequence_number,
        last.base.sequence_number,
    );
    Some(
        EyeProcessorEvent::new(EyeDerivedKind::Dwell, last.base.clone(), provenance)
            .with_duration_ns(duration_ns),
    )
}

/// Derive a fixation event when usable screen-gaze samples stay tightly clustered.
pub fn derive_screen_fixation_event(
    samples: &[EyeScreenGazePoint],
    processor_id: impl Into<String>,
    max_radius01: f32,
) -> Option<EyeProcessorEvent> {
    let first = samples.first()?;
    let last = samples.last()?;
    if samples.len() < 2 || !max_radius01.is_finite() || max_radius01 < 0.0 {
        return None;
    }

    let center = samples.iter().try_fold(Vec2::ZERO, |sum, sample| {
        if sample.is_valid() && sample.base.validity.is_sample_usable() {
            Some(sum + sample.normalized_point)
        } else {
            None
        }
    })? / samples.len() as f32;

    let max_radius_sq = max_radius01 * max_radius01;
    if samples
        .iter()
        .any(|sample| distance_squared(sample.normalized_point, center) > max_radius_sq)
    {
        return None;
    }

    let duration_ns = last
        .base
        .sample_time_ns
        .checked_sub(first.base.sample_time_ns)?;
    let provenance = EyeDerivedProvenance::new(
        STREAM_EYE_SCREEN_GAZE_POINT,
        processor_id,
        first.base.sequence_number,
        last.base.sequence_number,
    );
    Some(
        EyeProcessorEvent::new(EyeDerivedKind::Fixation, last.base.clone(), provenance)
            .with_duration_ns(duration_ns),
    )
}

/// Derive a blink/dropout processor event from an unusable eye sample.
pub fn derive_blink_dropout_event(
    sample: &EyeScreenGazePoint,
    processor_id: impl Into<String>,
) -> Option<EyeProcessorEvent> {
    if sample.base.validity.is_sample_usable()
        || !(sample.base.validity.blink || sample.base.validity.tracking_lost)
    {
        return None;
    }

    let provenance = EyeDerivedProvenance::new(
        STREAM_EYE_SCREEN_GAZE_POINT,
        processor_id,
        sample.base.sequence_number,
        sample.base.sequence_number,
    );
    Some(EyeProcessorEvent::new(
        EyeDerivedKind::Blink,
        sample.base.clone(),
        provenance,
    ))
}

/// Derived eye processor event, such as fixation, dwell, or blink/dropout.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EyeProcessorEvent {
    pub schema: String,
    pub kind: EyeDerivedKind,
    pub base: EyeSampleBase,
    pub duration_ns: Option<u64>,
    pub derived_from: EyeDerivedProvenance,
}

impl EyeProcessorEvent {
    pub fn new(
        kind: EyeDerivedKind,
        base: EyeSampleBase,
        derived_from: EyeDerivedProvenance,
    ) -> Self {
        Self {
            schema: EYE_PROCESSOR_EVENT_SCHEMA.to_string(),
            kind,
            base,
            duration_ns: None,
            derived_from,
        }
    }

    pub const fn with_duration_ns(mut self, duration_ns: u64) -> Self {
        self.duration_ns = Some(duration_ns);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.base.is_valid() && self.derived_from.is_valid()
    }
}

/// Built-in synthetic eye stream scenarios for processor and schema tests.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntheticEyeScenario {
    StableFixation,
    Saccade,
    BlinkDropout,
}

/// Validation error for deterministic synthetic eye stream generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeSyntheticError {
    InvalidRateHz,
}

impl fmt::Display for EyeSyntheticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRateHz => f.write_str("sample rate must be finite and greater than zero"),
        }
    }
}

impl std::error::Error for EyeSyntheticError {}

/// Source-only deterministic screen-gaze generator for broker and processor tests.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntheticScreenGazeGenerator {
    pub provider_id: String,
    pub source_device_id: String,
    pub scenario: SyntheticEyeScenario,
    pub sample_period_ns: u64,
    pub next_sequence_number: u64,
    pub next_time_ns: u64,
}

impl SyntheticScreenGazeGenerator {
    pub fn new(
        provider_id: impl Into<String>,
        source_device_id: impl Into<String>,
        scenario: SyntheticEyeScenario,
        sample_rate_hz: f32,
    ) -> Result<Self, EyeSyntheticError> {
        if !sample_rate_hz.is_finite() || sample_rate_hz <= 0.0 {
            return Err(EyeSyntheticError::InvalidRateHz);
        }

        Ok(Self {
            provider_id: provider_id.into(),
            source_device_id: source_device_id.into(),
            scenario,
            sample_period_ns: (1_000_000_000.0 / sample_rate_hz).round().max(1.0) as u64,
            next_sequence_number: 0,
            next_time_ns: 0,
        })
    }

    pub fn next_sample(&mut self) -> EyeScreenGazePoint {
        let sequence_number = self.next_sequence_number;
        let sample_time_ns = self.next_time_ns;
        let mut base = EyeSampleBase::new(
            self.provider_id.clone(),
            self.source_device_id.clone(),
            sequence_number,
            sample_time_ns,
            EyeCoordinateSpace::ScreenNormalized,
        )
        .with_confidence(0.95);

        let point = match self.scenario {
            SyntheticEyeScenario::StableFixation => {
                let phase = (sequence_number % 16) as f32 / 16.0;
                Vec2::new(0.5 + (phase * core::f32::consts::TAU).sin() * 0.01, 0.5)
            }
            SyntheticEyeScenario::Saccade => {
                if (sequence_number / 8).is_multiple_of(2) {
                    Vec2::new(0.25, 0.45)
                } else {
                    Vec2::new(0.75, 0.55)
                }
            }
            SyntheticEyeScenario::BlinkDropout => {
                if sequence_number % 10 == 5 {
                    base = base
                        .with_validity(EyeValidityFlags::blink_dropout())
                        .with_confidence(0.0);
                }
                Vec2::new(0.5, 0.5)
            }
        };

        self.next_sequence_number = self.next_sequence_number.saturating_add(1);
        self.next_time_ns = self.next_time_ns.saturating_add(self.sample_period_ns);

        EyeScreenGazePoint::new_screen_normalized(base, point)
    }
}

fn valid_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn valid_normalized_point(point: Vec2) -> bool {
    point.is_finite() && valid_unit_interval(point.x) && valid_unit_interval(point.y)
}

fn valid_pixel_point(point: Vec2) -> bool {
    point.is_finite() && point.x >= 0.0 && point.y >= 0.0
}

fn distance_squared(a: Vec2, b: Vec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx) + (dy * dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen_base(sequence_number: u64) -> EyeSampleBase {
        EyeSampleBase::new(
            "synthetic",
            "desktop-tracker",
            sequence_number,
            sequence_number * 1_000,
            EyeCoordinateSpace::ScreenNormalized,
        )
    }

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn screen_gaze_point_validates_normalized_domain() {
        let sample = EyeScreenGazePoint::new_screen_normalized(
            screen_base(1).with_confidence(0.9),
            Vec2::new(0.25, 0.75),
        )
        .with_screen_pixel(Vec2::new(320.0, 240.0))
        .with_pupil_diameter_mm(4.0);

        assert!(sample.is_valid());
        assert_eq!(sample.schema, EYE_SCREEN_GAZE_POINT_SCHEMA);

        let invalid =
            EyeScreenGazePoint::new_screen_normalized(screen_base(2), Vec2::new(1.25, 0.5));
        assert!(!invalid.is_valid());
    }

    #[test]
    fn xr_ray_rejects_screen_space_base() {
        let ray = EyeXrGazeRay::new(screen_base(1), Vec3::ZERO, Vec3::FORWARD_NEG_Z);

        assert!(!ray.is_valid());

        let xr_base = EyeSampleBase::new(
            "synthetic",
            "headset",
            1,
            1_000,
            EyeCoordinateSpace::XrLocal,
        );
        let valid_ray = EyeXrGazeRay::new(xr_base, Vec3::ZERO, Vec3::FORWARD_NEG_Z);

        assert!(valid_ray.is_valid());
    }

    #[test]
    fn derived_aoi_hit_carries_provenance() {
        let provenance =
            EyeDerivedProvenance::new(STREAM_EYE_SCREEN_GAZE_POINT, "synthetic-aoi", 10, 20);
        let hit = EyeScreenAoiHit::new(screen_base(20), "button-1", true, provenance)
            .with_dwell_time_ns(250_000_000);

        assert!(hit.is_valid());
        assert_eq!(hit.schema, EYE_SCREEN_AOI_HIT_SCHEMA);
    }

    #[test]
    fn screen_aoi_processor_counts_hits_and_dwell() {
        let mut generator = SyntheticScreenGazeGenerator::new(
            "synthetic",
            "desktop",
            SyntheticEyeScenario::Saccade,
            60.0,
        )
        .expect("valid generator");
        let samples: Vec<_> = (0..9).map(|_| generator.next_sample()).collect();
        let left_button_bounds = ScreenAoiBounds::new(Vec2::new(0.20, 0.40), Vec2::new(0.30, 0.50));

        let hits: Vec<_> = samples
            .iter()
            .map(|sample| {
                derive_screen_aoi_hit(
                    sample,
                    "left-button",
                    left_button_bounds,
                    "test-aoi",
                    Some(samples[0].base.sample_time_ns),
                )
            })
            .collect();

        assert_eq!(hits.iter().filter(|hit| hit.hit).count(), 8);
        assert!(!hits[8].hit);
        assert_eq!(
            hits[7].dwell_time_ns,
            Some(samples[7].base.sample_time_ns - samples[0].base.sample_time_ns)
        );

        let dwell = derive_screen_dwell_event(&samples[..8], left_button_bounds, "test-dwell", 1)
            .expect("contiguous AOI samples should dwell");
        assert!(dwell.is_valid());
        assert_eq!(dwell.kind, EyeDerivedKind::Dwell);
        assert_eq!(dwell.derived_from.source_sequence_start, 0);
        assert_eq!(dwell.derived_from.source_sequence_end, 7);
        assert!(derive_screen_dwell_event(&samples, left_button_bounds, "test-dwell", 1).is_none());
    }

    #[test]
    fn fixation_processor_requires_clustered_usable_samples() {
        let mut stable = SyntheticScreenGazeGenerator::new(
            "synthetic",
            "desktop",
            SyntheticEyeScenario::StableFixation,
            120.0,
        )
        .expect("valid generator");
        let stable_samples: Vec<_> = (0..8).map(|_| stable.next_sample()).collect();

        let fixation = derive_screen_fixation_event(&stable_samples, "test-fixation", 0.02)
            .expect("stable gaze should produce fixation");
        assert!(fixation.is_valid());
        assert_eq!(fixation.kind, EyeDerivedKind::Fixation);
        assert_eq!(fixation.derived_from.source_sequence_start, 0);
        assert_eq!(fixation.derived_from.source_sequence_end, 7);
        assert_eq!(
            fixation.duration_ns,
            Some(stable_samples[7].base.sample_time_ns - stable_samples[0].base.sample_time_ns)
        );

        let mut saccade = SyntheticScreenGazeGenerator::new(
            "synthetic",
            "desktop",
            SyntheticEyeScenario::Saccade,
            120.0,
        )
        .expect("valid generator");
        let saccade_samples: Vec<_> = (0..10).map(|_| saccade.next_sample()).collect();

        assert!(derive_screen_fixation_event(&saccade_samples, "test-fixation", 0.02).is_none());
    }

    #[test]
    fn blink_dropout_processor_marks_invalid_sample() {
        let mut generator = SyntheticScreenGazeGenerator::new(
            "synthetic",
            "desktop",
            SyntheticEyeScenario::BlinkDropout,
            90.0,
        )
        .expect("valid generator");

        for _ in 0..5 {
            let sample = generator.next_sample();
            assert!(derive_blink_dropout_event(&sample, "test-blink").is_none());
        }
        let blink_sample = generator.next_sample();
        let event = derive_blink_dropout_event(&blink_sample, "test-blink")
            .expect("blink dropout should produce processor event");

        assert!(event.is_valid());
        assert_eq!(event.kind, EyeDerivedKind::Blink);
        assert_eq!(event.base.sequence_number, 5);
        assert_eq!(event.derived_from.source_sequence_start, 5);
        assert_eq!(event.derived_from.source_sequence_end, 5);
    }

    #[test]
    fn stable_synthetic_fixation_is_deterministic() {
        let mut generator = SyntheticScreenGazeGenerator::new(
            "synthetic",
            "desktop",
            SyntheticEyeScenario::StableFixation,
            120.0,
        )
        .expect("valid generator");

        let first = generator.next_sample();
        let second = generator.next_sample();

        assert!(first.is_valid());
        assert!(second.is_valid());
        assert_eq!(generator.sample_period_ns, 8_333_334);
        assert_eq!(first.base.sequence_number, 0);
        assert_eq!(second.base.sequence_number, 1);
        assert_eq!(second.base.sample_time_ns, 8_333_334);
    }

    #[test]
    fn synthetic_saccade_changes_screen_target() {
        let mut generator = SyntheticScreenGazeGenerator::new(
            "synthetic",
            "desktop",
            SyntheticEyeScenario::Saccade,
            60.0,
        )
        .expect("valid generator");

        let first = generator.next_sample();
        for _ in 0..7 {
            generator.next_sample();
        }
        let ninth = generator.next_sample();

        assert!(first.is_valid());
        assert!(ninth.is_valid());
        assert!(first.normalized_point.x < ninth.normalized_point.x);
    }

    #[test]
    fn synthetic_blink_dropout_marks_invalid_sample() {
        let mut generator = SyntheticScreenGazeGenerator::new(
            "synthetic",
            "desktop",
            SyntheticEyeScenario::BlinkDropout,
            90.0,
        )
        .expect("valid generator");

        for _ in 0..5 {
            let sample = generator.next_sample();
            assert!(sample.base.validity.is_sample_usable());
        }
        let blink = generator.next_sample();

        assert!(blink.is_valid());
        assert!(!blink.base.validity.is_sample_usable());
        assert!(blink.base.validity.blink);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn screen_gaze_point_round_trips_with_serde() {
        let sample = EyeScreenGazePoint::new_screen_normalized(
            screen_base(3).with_confidence(0.8),
            Vec2::new(0.5, 0.5),
        );

        let encoded = serde_json::to_string(&sample).expect("sample should serialize");
        let decoded: EyeScreenGazePoint =
            serde_json::from_str(&encoded).expect("sample should deserialize");

        assert_eq!(decoded, sample);
    }
}
