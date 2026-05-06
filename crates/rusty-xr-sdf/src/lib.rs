//! Signed-distance-field contracts and utilities for Rusty XR.
//!
//! This crate owns public SDF, sparse TSDF, scan-surface, mesh snapshot, and
//! dynamic mesh-to-SDF reference utilities. Native depth acquisition, meshing
//! workers, physics backends, and captured room datasets stay in adapters or
//! downstream repos.
//!
//! Enable the `serde` feature to serialize public scan and SDF snapshots.

use core::fmt;
use std::collections::{HashMap, HashSet};

pub use rusty_xr_contracts::Vec3;
use rusty_xr_contracts::{HandMeshError, HandMeshSnapshot};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Packed SDF sample at a voxel center.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PackedSdfSample {
    pub distance_meters: f32,
    pub normal: Vec3,
}

impl PackedSdfSample {
    pub const fn new(distance_meters: f32, normal: Vec3) -> Self {
        Self {
            distance_meters,
            normal,
        }
    }
}

/// Integer voxel coordinate for sparse scan and TSDF data.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct VoxelCoord3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl VoxelCoord3 {
    pub const ZERO: Self = Self::new(0, 0, 0);

    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Sparse TSDF sample suitable for scan-fusion snapshots.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SparseTsdfSample {
    pub coord: VoxelCoord3,
    pub normalized_distance: f32,
    pub confidence: u8,
    pub last_seen_time_ns: Option<u64>,
}

impl SparseTsdfSample {
    pub const fn new(coord: VoxelCoord3, normalized_distance: f32, confidence: u8) -> Self {
        Self {
            coord,
            normalized_distance,
            confidence,
            last_seen_time_ns: None,
        }
    }

    pub const fn with_last_seen_time_ns(mut self, last_seen_time_ns: u64) -> Self {
        self.last_seen_time_ns = Some(last_seen_time_ns);
        self
    }

    pub fn signed_distance_meters(self, truncation_distance_meters: f32) -> f32 {
        self.normalized_distance.clamp(-1.0, 1.0) * truncation_distance_meters
    }

    pub fn is_surface_candidate(self, surface_band_normalized: f32) -> bool {
        self.confidence > 0
            && self.normalized_distance.is_finite()
            && self.normalized_distance.abs() <= surface_band_normalized.max(0.0)
    }
}

/// Sparse TSDF snapshot exported by a scanner or environment-depth adapter.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SparseTsdfSnapshot {
    pub version: u64,
    pub origin: Vec3,
    pub voxel_size_meters: f32,
    pub truncation_distance_meters: f32,
    pub samples: Vec<SparseTsdfSample>,
}

impl SparseTsdfSnapshot {
    pub fn new(
        version: u64,
        origin: Vec3,
        voxel_size_meters: f32,
        truncation_distance_meters: f32,
        samples: Vec<SparseTsdfSample>,
    ) -> Self {
        Self {
            version,
            origin,
            voxel_size_meters,
            truncation_distance_meters,
            samples,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.origin.is_finite()
            && self.voxel_size_meters.is_finite()
            && self.voxel_size_meters > 0.0
            && self.truncation_distance_meters.is_finite()
            && self.truncation_distance_meters > 0.0
            && self
                .samples
                .iter()
                .all(|sample| sample.normalized_distance.is_finite())
    }

    pub fn surface_candidate_count(&self, surface_band_normalized: f32) -> usize {
        self.samples
            .iter()
            .copied()
            .filter(|sample| sample.is_surface_candidate(surface_band_normalized))
            .count()
    }

    pub fn voxel_center_world(&self, coord: VoxelCoord3) -> Vec3 {
        self.origin
            + Vec3::new(
                (coord.x as f32 + 0.5) * self.voxel_size_meters,
                (coord.y as f32 + 0.5) * self.voxel_size_meters,
                (coord.z as f32 + 0.5) * self.voxel_size_meters,
            )
    }
}

/// Surface sample extracted from a scan-fusion or TSDF volume.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScanSurfaceSample {
    pub coord: VoxelCoord3,
    pub world_position: Vec3,
    pub world_normal: Vec3,
    pub confidence: u8,
    pub signed_distance_meters: f32,
    pub last_seen_time_ns: Option<u64>,
}

impl ScanSurfaceSample {
    pub fn is_valid(self) -> bool {
        self.world_position.is_finite()
            && self.world_normal.is_finite()
            && self.signed_distance_meters.is_finite()
    }
}

/// Role a depth-derived surface can play for an interaction or physics adapter.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DepthQuerySurfaceRole {
    Support,
    Impact,
}

/// Stable key for a retained depth query surface.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DepthQuerySurfaceKey {
    pub request_key: u64,
    pub role: DepthQuerySurfaceRole,
}

impl DepthQuerySurfaceKey {
    pub const fn new(request_key: u64, role: DepthQuerySurfaceRole) -> Self {
        Self { request_key, role }
    }
}

/// A finite support or impact plane derived from depth/TSDF analysis.
///
/// This is a data contract. It does not require a physics engine and does not
/// prescribe how a TSDF or depth image should be queried.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthSupportPlane {
    pub point: Vec3,
    pub normal: Vec3,
    pub tangent: Vec3,
    pub bitangent: Vec3,
    pub half_extent_tangent_meters: f32,
    pub half_extent_bitangent_meters: f32,
}

impl DepthSupportPlane {
    pub const fn new(
        point: Vec3,
        normal: Vec3,
        tangent: Vec3,
        bitangent: Vec3,
        half_extent_tangent_meters: f32,
        half_extent_bitangent_meters: f32,
    ) -> Self {
        Self {
            point,
            normal,
            tangent,
            bitangent,
            half_extent_tangent_meters,
            half_extent_bitangent_meters,
        }
    }

    pub fn is_valid(self) -> bool {
        let min_len_sq = 1.0e-8;
        self.point.is_finite()
            && self.normal.is_finite()
            && self.tangent.is_finite()
            && self.bitangent.is_finite()
            && self.normal.length_squared() > min_len_sq
            && self.tangent.length_squared() > min_len_sq
            && self.bitangent.length_squared() > min_len_sq
            && self.half_extent_tangent_meters.is_finite()
            && self.half_extent_tangent_meters > 0.0
            && self.half_extent_bitangent_meters.is_finite()
            && self.half_extent_bitangent_meters > 0.0
    }

    pub fn quad_vertices(self) -> [Vec3; 4] {
        let tangent = self.tangent.normalized_or(Vec3::RIGHT) * self.half_extent_tangent_meters;
        let bitangent =
            self.bitangent.normalized_or(Vec3::FORWARD_NEG_Z) * self.half_extent_bitangent_meters;
        [
            self.point - tangent - bitangent,
            self.point + tangent - bitangent,
            self.point + tangent + bitangent,
            self.point - tangent + bitangent,
        ]
    }

    pub fn supports_point(self, point: Vec3, radius_meters: f32, edge_margin_meters: f32) -> bool {
        if !self.is_valid() || !point.is_finite() {
            return false;
        }
        let radius_meters = radius_meters.max(0.0);
        let edge_margin_meters = edge_margin_meters.max(0.0);
        let normal = self.normal.normalized_or(Vec3::UP);
        let tangent = self.tangent.normalized_or(Vec3::RIGHT);
        let bitangent = self.bitangent.normalized_or(Vec3::FORWARD_NEG_Z);
        let offset = point - self.point;
        let signed_distance = offset.dot(normal);
        let tangent_slack =
            self.half_extent_tangent_meters - offset.dot(tangent).abs() - edge_margin_meters;
        let bitangent_slack =
            self.half_extent_bitangent_meters - offset.dot(bitangent).abs() - edge_margin_meters;

        signed_distance.abs() <= radius_meters
            && tangent_slack >= -radius_meters
            && bitangent_slack >= -radius_meters
    }
}

/// Public summary of a depth query surface.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthQuerySurfaceSummary {
    pub role: DepthQuerySurfaceRole,
    pub plane: DepthSupportPlane,
    pub confidence: u8,
    pub restitution: f32,
}

impl DepthQuerySurfaceSummary {
    pub const fn new(
        role: DepthQuerySurfaceRole,
        plane: DepthSupportPlane,
        confidence: u8,
        restitution: f32,
    ) -> Self {
        Self {
            role,
            plane,
            confidence,
            restitution,
        }
    }

    pub fn is_valid(self) -> bool {
        self.plane.is_valid() && self.confidence > 0 && self.restitution.is_finite()
    }
}

/// Request shape for a depth-backed support or impact query.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthQueryRequest {
    pub key: u64,
    pub center: Vec3,
    pub predicted_center: Vec3,
    pub velocity: Vec3,
    pub radius_meters: f32,
    pub max_distance_meters: f32,
}

impl DepthQueryRequest {
    pub const fn new(
        key: u64,
        center: Vec3,
        predicted_center: Vec3,
        velocity: Vec3,
        radius_meters: f32,
        max_distance_meters: f32,
    ) -> Self {
        Self {
            key,
            center,
            predicted_center,
            velocity,
            radius_meters,
            max_distance_meters,
        }
    }

    pub fn is_valid(self) -> bool {
        self.center.is_finite()
            && self.predicted_center.is_finite()
            && self.velocity.is_finite()
            && self.radius_meters.is_finite()
            && self.radius_meters > 0.0
            && self.max_distance_meters.is_finite()
            && self.max_distance_meters > 0.0
    }

    pub fn travel_distance_meters(self) -> f32 {
        (self.predicted_center - self.center).length()
    }

    pub fn might_need_impact_refresh(
        self,
        min_speed_mps: f32,
        min_horizontal_speed_mps: f32,
        min_upward_speed_mps: f32,
        min_travel_meters: f32,
    ) -> bool {
        if !self.is_valid() {
            return false;
        }
        let velocity_length = self.velocity.length();
        let travel_distance = self.travel_distance_meters();
        if velocity_length < min_speed_mps.max(0.0) && travel_distance < min_travel_meters.max(0.0)
        {
            return false;
        }
        let horizontal_speed =
            ((self.velocity.x * self.velocity.x) + (self.velocity.z * self.velocity.z)).sqrt();
        let upward_speed = self.velocity.y.max(0.0);
        horizontal_speed >= min_horizontal_speed_mps.max(0.0)
            || upward_speed >= min_upward_speed_mps.max(0.0)
    }
}

/// Settings for evaluating support planes from a sparse TSDF snapshot.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TsdfSupportQuerySettings {
    /// Maximum absolute signed distance for samples used to fit a support
    /// surface.
    pub surface_band_meters: f32,
    /// Extra horizontal radius around the query sphere used when collecting
    /// candidate support samples.
    pub lateral_padding_meters: f32,
    /// Minimum upward-facing normal component for candidate support samples.
    pub min_upward_normal_dot: f32,
    /// Minimum number of candidate samples needed to report a plane.
    pub min_sample_count: usize,
    /// Minimum half extents for the resulting finite support plane.
    pub min_half_extent_meters: f32,
    /// Restitution value copied into the public surface summary.
    pub restitution: f32,
}

impl Default for TsdfSupportQuerySettings {
    fn default() -> Self {
        Self {
            surface_band_meters: 0.08,
            lateral_padding_meters: 0.05,
            min_upward_normal_dot: 0.35,
            min_sample_count: 4,
            min_half_extent_meters: 0.05,
            restitution: 0.0,
        }
    }
}

impl TsdfSupportQuerySettings {
    pub fn normalized(self) -> Self {
        let defaults = Self::default();
        Self {
            surface_band_meters: if self.surface_band_meters.is_finite()
                && self.surface_band_meters > 0.0
            {
                self.surface_band_meters
            } else {
                defaults.surface_band_meters
            },
            lateral_padding_meters: if self.lateral_padding_meters.is_finite() {
                self.lateral_padding_meters.max(0.0)
            } else {
                defaults.lateral_padding_meters
            },
            min_upward_normal_dot: if self.min_upward_normal_dot.is_finite() {
                self.min_upward_normal_dot.clamp(-1.0, 1.0)
            } else {
                defaults.min_upward_normal_dot
            },
            min_sample_count: self.min_sample_count.max(3),
            min_half_extent_meters: if self.min_half_extent_meters.is_finite()
                && self.min_half_extent_meters > 0.0
            {
                self.min_half_extent_meters
            } else {
                defaults.min_half_extent_meters
            },
            restitution: if self.restitution.is_finite() {
                self.restitution
            } else {
                defaults.restitution
            },
        }
    }
}

/// Result of evaluating one TSDF-backed support query.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TsdfSupportQueryResult {
    pub request_key: u64,
    pub snapshot_version: u64,
    pub surface: DepthQuerySurfaceSummary,
    pub sample_count: usize,
    pub average_signed_distance_meters: f32,
    pub max_abs_signed_distance_meters: f32,
}

impl TsdfSupportQueryResult {
    pub fn is_valid(self) -> bool {
        self.surface.is_valid()
            && self.sample_count > 0
            && self.average_signed_distance_meters.is_finite()
            && self.max_abs_signed_distance_meters.is_finite()
    }
}

/// Evaluate a finite support plane under one depth query request.
///
/// This is a deterministic CPU reference over public sparse TSDF snapshots.
/// Physics engines can convert the returned plane into their own collider
/// representation while preserving the request key.
pub fn evaluate_tsdf_support_query(
    snapshot: &SparseTsdfSnapshot,
    request: DepthQueryRequest,
    settings: TsdfSupportQuerySettings,
) -> Option<TsdfSupportQueryResult> {
    if !snapshot.is_valid() || !request.is_valid() {
        return None;
    }
    let settings = settings.normalized();
    let lookup = sparse_tsdf_lookup(snapshot);
    if lookup.is_empty() {
        return None;
    }

    let up = Vec3::UP;
    let query_center = request.predicted_center;
    let max_lateral_meters = request.radius_meters + settings.lateral_padding_meters;
    let max_above_meters = request.radius_meters;
    let max_below_meters = request.max_distance_meters + request.radius_meters;
    let mut candidates = Vec::<TsdfSupportCandidate>::new();

    for sample in snapshot.samples.iter().copied() {
        if sample.confidence == 0 || !sample.normalized_distance.is_finite() {
            continue;
        }
        let signed_distance = sample.signed_distance_meters(snapshot.truncation_distance_meters);
        if signed_distance.abs() > settings.surface_band_meters {
            continue;
        }
        let Some(normal) = estimate_sparse_tsdf_normal(snapshot, &lookup, sample.coord) else {
            continue;
        };
        if normal.dot(up) < settings.min_upward_normal_dot {
            continue;
        }
        let surface_point = snapshot.voxel_center_world(sample.coord) - (normal * signed_distance);
        let offset = surface_point - query_center;
        let vertical_meters = offset.dot(up);
        if vertical_meters > max_above_meters || vertical_meters < -max_below_meters {
            continue;
        }
        let lateral = offset - (up * vertical_meters);
        if lateral.length() > max_lateral_meters {
            continue;
        }
        candidates.push(TsdfSupportCandidate {
            surface_point,
            normal,
            signed_distance,
            confidence: sample.confidence,
        });
    }

    if candidates.len() < settings.min_sample_count {
        return None;
    }

    let mut point_sum = Vec3::ZERO;
    let mut normal_sum = Vec3::ZERO;
    let mut signed_sum = 0.0f32;
    let mut max_abs_signed = 0.0f32;
    let mut confidence_sum = 0u32;
    for candidate in &candidates {
        point_sum += candidate.surface_point;
        normal_sum += candidate.normal;
        signed_sum += candidate.signed_distance;
        max_abs_signed = max_abs_signed.max(candidate.signed_distance.abs());
        confidence_sum += candidate.confidence as u32;
    }

    let sample_count = candidates.len();
    let average_point = point_sum / sample_count as f32;
    let normal = normal_sum.normalized_or(Vec3::UP);
    let (tangent, bitangent) = support_plane_basis(normal);
    let center_to_average = query_center - average_point;
    let plane_point = query_center - (normal * center_to_average.dot(normal));

    let mut half_extent_tangent = 0.0f32;
    let mut half_extent_bitangent = 0.0f32;
    for candidate in &candidates {
        let offset = candidate.surface_point - plane_point;
        half_extent_tangent = half_extent_tangent.max(offset.dot(tangent).abs());
        half_extent_bitangent = half_extent_bitangent.max(offset.dot(bitangent).abs());
    }
    half_extent_tangent = half_extent_tangent
        .max(request.radius_meters)
        .max(settings.min_half_extent_meters);
    half_extent_bitangent = half_extent_bitangent
        .max(request.radius_meters)
        .max(settings.min_half_extent_meters);

    let plane = DepthSupportPlane::new(
        plane_point,
        normal,
        tangent,
        bitangent,
        half_extent_tangent,
        half_extent_bitangent,
    );
    if !plane.is_valid() {
        return None;
    }

    let confidence = (confidence_sum / sample_count as u32).clamp(1, u8::MAX as u32) as u8;
    Some(TsdfSupportQueryResult {
        request_key: request.key,
        snapshot_version: snapshot.version,
        surface: DepthQuerySurfaceSummary::new(
            DepthQuerySurfaceRole::Support,
            plane,
            confidence,
            settings.restitution,
        ),
        sample_count,
        average_signed_distance_meters: signed_sum / sample_count as f32,
        max_abs_signed_distance_meters: max_abs_signed,
    })
}

/// Evaluate support planes for a batch of depth query requests.
pub fn evaluate_tsdf_support_queries(
    snapshot: &SparseTsdfSnapshot,
    requests: &[DepthQueryRequest],
    settings: TsdfSupportQuerySettings,
) -> Vec<TsdfSupportQueryResult> {
    requests
        .iter()
        .copied()
        .filter_map(|request| evaluate_tsdf_support_query(snapshot, request, settings))
        .collect()
}

/// Settings for evaluating swept impact planes from a sparse TSDF snapshot.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TsdfImpactQuerySettings {
    /// Maximum absolute signed distance for samples used as impact surface
    /// candidates.
    pub surface_band_meters: f32,
    /// Extra radius around the swept query sphere for noisy TSDF samples.
    pub lateral_padding_meters: f32,
    /// Minimum absolute alignment between motion direction and surface normal.
    pub min_motion_normal_dot: f32,
    /// Minimum half extents for the resulting finite impact plane.
    pub min_half_extent_meters: f32,
    /// Restitution value copied into the public surface summary.
    pub restitution: f32,
}

impl Default for TsdfImpactQuerySettings {
    fn default() -> Self {
        Self {
            surface_band_meters: 0.08,
            lateral_padding_meters: 0.03,
            min_motion_normal_dot: 0.15,
            min_half_extent_meters: 0.05,
            restitution: 0.0,
        }
    }
}

impl TsdfImpactQuerySettings {
    pub fn normalized(self) -> Self {
        let defaults = Self::default();
        Self {
            surface_band_meters: if self.surface_band_meters.is_finite()
                && self.surface_band_meters > 0.0
            {
                self.surface_band_meters
            } else {
                defaults.surface_band_meters
            },
            lateral_padding_meters: if self.lateral_padding_meters.is_finite() {
                self.lateral_padding_meters.max(0.0)
            } else {
                defaults.lateral_padding_meters
            },
            min_motion_normal_dot: if self.min_motion_normal_dot.is_finite() {
                self.min_motion_normal_dot.clamp(0.0, 1.0)
            } else {
                defaults.min_motion_normal_dot
            },
            min_half_extent_meters: if self.min_half_extent_meters.is_finite()
                && self.min_half_extent_meters > 0.0
            {
                self.min_half_extent_meters
            } else {
                defaults.min_half_extent_meters
            },
            restitution: if self.restitution.is_finite() {
                self.restitution
            } else {
                defaults.restitution
            },
        }
    }
}

/// Result of evaluating one swept TSDF-backed impact query.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TsdfImpactQueryResult {
    pub request_key: u64,
    pub snapshot_version: u64,
    pub surface: DepthQuerySurfaceSummary,
    pub impact_center: Vec3,
    pub travel_fraction: f32,
    pub candidate_count: usize,
    pub signed_clearance_at_impact_meters: f32,
}

impl TsdfImpactQueryResult {
    pub fn is_valid(self) -> bool {
        self.surface.is_valid()
            && self.impact_center.is_finite()
            && self.travel_fraction.is_finite()
            && (0.0..=1.0).contains(&self.travel_fraction)
            && self.candidate_count > 0
            && self.signed_clearance_at_impact_meters.is_finite()
    }
}

/// Evaluate the earliest swept-sphere impact plane for one depth query request.
///
/// This CPU reference finds TSDF surface candidates near the swept sphere,
/// estimates their normals, orients the returned plane against motion, and
/// reports the earliest contact. Physics engines remain adapter-owned.
pub fn evaluate_tsdf_impact_query(
    snapshot: &SparseTsdfSnapshot,
    request: DepthQueryRequest,
    settings: TsdfImpactQuerySettings,
) -> Option<TsdfImpactQueryResult> {
    if !snapshot.is_valid() || !request.is_valid() {
        return None;
    }
    let settings = settings.normalized();
    let lookup = sparse_tsdf_lookup(snapshot);
    if lookup.is_empty() {
        return None;
    }

    let motion = request.predicted_center - request.center;
    let motion_len_sq = motion.length_squared();
    if motion_len_sq <= 1.0e-10 || !motion_len_sq.is_finite() {
        return None;
    }
    let travel_dir = motion.normalized_or(Vec3::ZERO);
    if travel_dir.length_squared() == 0.0 {
        return None;
    }

    let mut best: Option<TsdfImpactCandidate> = None;
    let mut candidate_count = 0usize;
    for sample in snapshot.samples.iter().copied() {
        if sample.confidence == 0 || !sample.normalized_distance.is_finite() {
            continue;
        }
        let signed_distance = sample.signed_distance_meters(snapshot.truncation_distance_meters);
        if signed_distance.abs() > settings.surface_band_meters {
            continue;
        }
        let Some(normal) = estimate_sparse_tsdf_normal(snapshot, &lookup, sample.coord) else {
            continue;
        };
        let normal_alignment = normal.dot(travel_dir).abs();
        if normal_alignment < settings.min_motion_normal_dot {
            continue;
        }

        let mut impact_normal = normal;
        if impact_normal.dot(travel_dir) > 0.0 {
            impact_normal *= -1.0;
        }
        let surface_point = snapshot.voxel_center_world(sample.coord) - (normal * signed_distance);
        let start_clearance =
            (request.center - surface_point).dot(impact_normal) - request.radius_meters;
        let end_clearance =
            (request.predicted_center - surface_point).dot(impact_normal) - request.radius_meters;
        if start_clearance > 0.0 && end_clearance > 0.0 {
            continue;
        }
        let denom = motion.dot(impact_normal);
        if start_clearance > 0.0 && denom >= -1.0e-6 {
            continue;
        }
        let travel_fraction = if start_clearance <= 0.0 {
            0.0
        } else {
            (-start_clearance / denom).clamp(0.0, 1.0)
        };
        let impact_center = request.center + (motion * travel_fraction);
        let impact_offset = surface_point - impact_center;
        let tangential_offset = impact_offset - (impact_normal * impact_offset.dot(impact_normal));
        let lateral_distance = tangential_offset.length();
        if lateral_distance > request.radius_meters + settings.lateral_padding_meters {
            continue;
        }

        candidate_count += 1;
        let candidate = TsdfImpactCandidate {
            surface_point,
            normal: impact_normal,
            impact_center,
            travel_fraction,
            lateral_distance,
            signed_clearance: (impact_center - surface_point).dot(impact_normal)
                - request.radius_meters,
            confidence: sample.confidence,
        };
        if best
            .as_ref()
            .is_none_or(|current| impact_candidate_is_better(candidate, *current))
        {
            best = Some(candidate);
        }
    }

    let best = best?;
    let (tangent, bitangent) = support_plane_basis(best.normal);
    let half_extent = request
        .radius_meters
        .max(settings.min_half_extent_meters)
        .max(best.lateral_distance);
    let plane = DepthSupportPlane::new(
        best.surface_point,
        best.normal,
        tangent,
        bitangent,
        half_extent,
        half_extent,
    );
    if !plane.is_valid() {
        return None;
    }

    Some(TsdfImpactQueryResult {
        request_key: request.key,
        snapshot_version: snapshot.version,
        surface: DepthQuerySurfaceSummary::new(
            DepthQuerySurfaceRole::Impact,
            plane,
            best.confidence.max(1),
            settings.restitution,
        ),
        impact_center: best.impact_center,
        travel_fraction: best.travel_fraction,
        candidate_count,
        signed_clearance_at_impact_meters: best.signed_clearance,
    })
}

/// Evaluate swept impact planes for a batch of depth query requests.
pub fn evaluate_tsdf_impact_queries(
    snapshot: &SparseTsdfSnapshot,
    requests: &[DepthQueryRequest],
    settings: TsdfImpactQuerySettings,
) -> Vec<TsdfImpactQueryResult> {
    requests
        .iter()
        .copied()
        .filter_map(|request| evaluate_tsdf_impact_query(snapshot, request, settings))
        .collect()
}

/// Retained surface entry produced by a TSDF-backed query frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthQuerySurfaceEntry {
    pub key: DepthQuerySurfaceKey,
    pub snapshot_version: u64,
    pub surface: DepthQuerySurfaceSummary,
    pub sample_count: usize,
    pub last_seen_frame_index: u64,
}

impl DepthQuerySurfaceEntry {
    pub fn from_support_result(result: TsdfSupportQueryResult, frame_index: u64) -> Self {
        Self {
            key: DepthQuerySurfaceKey::new(result.request_key, DepthQuerySurfaceRole::Support),
            snapshot_version: result.snapshot_version,
            surface: result.surface,
            sample_count: result.sample_count,
            last_seen_frame_index: frame_index,
        }
    }

    pub fn from_impact_result(result: TsdfImpactQueryResult, frame_index: u64) -> Self {
        Self {
            key: DepthQuerySurfaceKey::new(result.request_key, DepthQuerySurfaceRole::Impact),
            snapshot_version: result.snapshot_version,
            surface: result.surface,
            sample_count: result.candidate_count,
            last_seen_frame_index: frame_index,
        }
    }

    pub fn is_valid(self) -> bool {
        self.surface.is_valid() && self.surface.role == self.key.role && self.sample_count > 0
    }

    pub fn age_frames(self, frame_index: u64) -> u64 {
        frame_index.saturating_sub(self.last_seen_frame_index)
    }
}

/// Settings for a retained TSDF query-surface update frame.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TsdfQuerySurfaceFrameSettings {
    pub support_settings: TsdfSupportQuerySettings,
    pub impact_settings: TsdfImpactQuerySettings,
    pub enable_support_queries: bool,
    pub enable_impact_queries: bool,
    /// Number of missed frames before a retained surface is removed.
    pub miss_retention_frames: u32,
    /// Clear retained surfaces immediately if no usable snapshot is available.
    pub clear_when_snapshot_unavailable: bool,
    pub min_impact_speed_mps: f32,
    pub min_impact_horizontal_speed_mps: f32,
    pub min_impact_upward_speed_mps: f32,
    pub min_impact_travel_meters: f32,
}

impl Default for TsdfQuerySurfaceFrameSettings {
    fn default() -> Self {
        Self {
            support_settings: TsdfSupportQuerySettings::default(),
            impact_settings: TsdfImpactQuerySettings::default(),
            enable_support_queries: true,
            enable_impact_queries: true,
            miss_retention_frames: 1,
            clear_when_snapshot_unavailable: false,
            min_impact_speed_mps: 0.05,
            min_impact_horizontal_speed_mps: 0.01,
            min_impact_upward_speed_mps: 0.05,
            min_impact_travel_meters: 0.01,
        }
    }
}

impl TsdfQuerySurfaceFrameSettings {
    pub fn normalized(self) -> Self {
        let defaults = Self::default();
        Self {
            support_settings: self.support_settings.normalized(),
            impact_settings: self.impact_settings.normalized(),
            enable_support_queries: self.enable_support_queries,
            enable_impact_queries: self.enable_impact_queries,
            miss_retention_frames: self.miss_retention_frames,
            clear_when_snapshot_unavailable: self.clear_when_snapshot_unavailable,
            min_impact_speed_mps: normalize_nonnegative_f32(
                self.min_impact_speed_mps,
                defaults.min_impact_speed_mps,
            ),
            min_impact_horizontal_speed_mps: normalize_nonnegative_f32(
                self.min_impact_horizontal_speed_mps,
                defaults.min_impact_horizontal_speed_mps,
            ),
            min_impact_upward_speed_mps: normalize_nonnegative_f32(
                self.min_impact_upward_speed_mps,
                defaults.min_impact_upward_speed_mps,
            ),
            min_impact_travel_meters: normalize_nonnegative_f32(
                self.min_impact_travel_meters,
                defaults.min_impact_travel_meters,
            ),
        }
    }

    pub fn disables_queries(self) -> bool {
        !self.enable_support_queries && !self.enable_impact_queries
    }
}

/// Incremental changes needed to keep TSDF query surfaces in sync with a
/// downstream physics, particle, or debug adapter.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TsdfQuerySurfaceUpdate {
    pub snapshot_version: u64,
    /// Surface keys that were hit by the latest TSDF query pass.
    pub visible_keys: Vec<DepthQuerySurfaceKey>,
    /// Surface keys retained without a downstream upsert.
    pub retained_keys: Vec<DepthQuerySurfaceKey>,
    /// New or changed surfaces that should replace adapter-side planes.
    pub upserts: Vec<DepthQuerySurfaceEntry>,
    /// Retained surfaces that should be removed adapter-side.
    pub removals: Vec<DepthQuerySurfaceKey>,
    pub support_request_count: usize,
    pub impact_request_count: usize,
    pub support_hit_count: usize,
    pub impact_hit_count: usize,
    pub reused_surface_count: usize,
}

impl TsdfQuerySurfaceUpdate {
    pub fn is_empty(&self) -> bool {
        self.upserts.is_empty() && self.removals.is_empty()
    }

    pub fn changed_surface_count(&self) -> usize {
        self.upserts.len() + self.removals.len()
    }
}

/// Outcome classification for one retained TSDF query-surface frame update.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TsdfQuerySurfaceFrameStatus {
    Updated,
    Disabled,
    WaitingForSnapshot,
    InvalidSnapshot,
}

/// Result of advancing a retained TSDF query-surface driver by one frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TsdfQuerySurfaceFrameOutput {
    pub frame_index: u64,
    pub status: TsdfQuerySurfaceFrameStatus,
    pub update: TsdfQuerySurfaceUpdate,
    pub cached_surface_count: usize,
    pub last_snapshot_version: Option<u64>,
    pub total_upsert_count: u64,
    pub total_removal_count: u64,
}

impl TsdfQuerySurfaceFrameOutput {
    pub fn is_physics_noop(&self) -> bool {
        self.update.is_empty()
    }
}

/// In-memory cache for retained TSDF support and impact planes.
///
/// This mirrors the useful public shape of a query-plane physics path: stable
/// keys, sticky surfaces across short misses, and explicit upsert/removal
/// events. Physics engines, particle systems, and renderer uploads remain
/// adapter-owned.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TsdfQuerySurfaceCache {
    surfaces: HashMap<DepthQuerySurfaceKey, DepthQuerySurfaceEntry>,
}

impl TsdfQuerySurfaceCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.surfaces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }

    pub fn contains_key(&self, key: DepthQuerySurfaceKey) -> bool {
        self.surfaces.contains_key(&key)
    }

    pub fn get(&self, key: DepthQuerySurfaceKey) -> Option<&DepthQuerySurfaceEntry> {
        self.surfaces.get(&key)
    }

    pub fn surfaces(&self) -> impl Iterator<Item = &DepthQuerySurfaceEntry> {
        self.surfaces.values()
    }

    pub fn clear(&mut self) -> Vec<DepthQuerySurfaceKey> {
        let mut removed = self.surfaces.keys().copied().collect::<Vec<_>>();
        removed.sort_by_key(depth_query_surface_key_tuple);
        self.surfaces.clear();
        removed
    }

    /// Update retained query surfaces from a sparse TSDF snapshot.
    pub fn update_from_tsdf(
        &mut self,
        frame_index: u64,
        snapshot: &SparseTsdfSnapshot,
        requests: &[DepthQueryRequest],
        settings: TsdfQuerySurfaceFrameSettings,
    ) -> TsdfQuerySurfaceUpdate {
        let settings = settings.normalized();
        let mut update = TsdfQuerySurfaceUpdate {
            snapshot_version: snapshot.version,
            ..TsdfQuerySurfaceUpdate::default()
        };
        if !snapshot.is_valid() || settings.disables_queries() {
            return update;
        }

        let mut hit_keys = HashSet::with_capacity(requests.len().saturating_mul(2));
        for request in requests
            .iter()
            .copied()
            .filter(|request| request.is_valid())
        {
            if settings.enable_support_queries {
                update.support_request_count += 1;
                if let Some(result) =
                    evaluate_tsdf_support_query(snapshot, request, settings.support_settings)
                {
                    update.support_hit_count += 1;
                    let entry = DepthQuerySurfaceEntry::from_support_result(result, frame_index);
                    self.record_surface_hit(entry, &mut hit_keys, &mut update);
                }
            }

            if settings.enable_impact_queries
                && request.might_need_impact_refresh(
                    settings.min_impact_speed_mps,
                    settings.min_impact_horizontal_speed_mps,
                    settings.min_impact_upward_speed_mps,
                    settings.min_impact_travel_meters,
                )
            {
                update.impact_request_count += 1;
                if let Some(result) =
                    evaluate_tsdf_impact_query(snapshot, request, settings.impact_settings)
                {
                    update.impact_hit_count += 1;
                    let entry = DepthQuerySurfaceEntry::from_impact_result(result, frame_index);
                    self.record_surface_hit(entry, &mut hit_keys, &mut update);
                }
            }
        }

        let mut stale_keys = self
            .surfaces
            .keys()
            .copied()
            .filter(|key| !hit_keys.contains(key))
            .collect::<Vec<_>>();
        stale_keys.sort_by_key(depth_query_surface_key_tuple);
        for key in stale_keys {
            let retain = self.surfaces.get(&key).is_some_and(|entry| {
                entry.age_frames(frame_index) <= settings.miss_retention_frames as u64
            });
            if retain {
                update.retained_keys.push(key);
            } else if self.surfaces.remove(&key).is_some() {
                update.removals.push(key);
            }
        }
        update.removals.sort_by_key(depth_query_surface_key_tuple);
        update
    }

    fn record_surface_hit(
        &mut self,
        entry: DepthQuerySurfaceEntry,
        hit_keys: &mut HashSet<DepthQuerySurfaceKey>,
        update: &mut TsdfQuerySurfaceUpdate,
    ) {
        if !entry.is_valid() || !hit_keys.insert(entry.key) {
            return;
        }
        update.visible_keys.push(entry.key);

        if self
            .surfaces
            .get(&entry.key)
            .is_some_and(|previous| surface_entries_match_without_frame(*previous, entry))
        {
            self.surfaces.insert(entry.key, entry);
            update.retained_keys.push(entry.key);
            update.reused_surface_count += 1;
            return;
        }

        self.surfaces.insert(entry.key, entry);
        update.upserts.push(entry);
    }
}

/// Frame-by-frame driver for retained TSDF support and impact surfaces.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TsdfQuerySurfaceFrameDriver {
    cache: TsdfQuerySurfaceCache,
    last_frame_index: Option<u64>,
    last_snapshot_version: Option<u64>,
    total_upsert_count: u64,
    total_removal_count: u64,
}

impl TsdfQuerySurfaceFrameDriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cache(&self) -> &TsdfQuerySurfaceCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut TsdfQuerySurfaceCache {
        &mut self.cache
    }

    pub fn cached_surface_count(&self) -> usize {
        self.cache.len()
    }

    pub fn last_frame_index(&self) -> Option<u64> {
        self.last_frame_index
    }

    pub fn last_snapshot_version(&self) -> Option<u64> {
        self.last_snapshot_version
    }

    pub fn total_upsert_count(&self) -> u64 {
        self.total_upsert_count
    }

    pub fn total_removal_count(&self) -> u64 {
        self.total_removal_count
    }

    pub fn clear(&mut self, frame_index: u64) -> TsdfQuerySurfaceFrameOutput {
        self.last_frame_index = Some(frame_index);
        let removals = self.cache.clear();
        self.frame_output(
            frame_index,
            TsdfQuerySurfaceFrameStatus::Disabled,
            clear_query_surface_update(self.last_snapshot_version.unwrap_or_default(), removals),
        )
    }

    pub fn advance_frame(
        &mut self,
        frame_index: u64,
        snapshot: Option<&SparseTsdfSnapshot>,
        requests: &[DepthQueryRequest],
        settings: TsdfQuerySurfaceFrameSettings,
    ) -> TsdfQuerySurfaceFrameOutput {
        self.last_frame_index = Some(frame_index);
        let settings = settings.normalized();

        if settings.disables_queries() {
            return self.clear(frame_index);
        }

        let Some(snapshot) = snapshot else {
            let removals = if settings.clear_when_snapshot_unavailable {
                self.cache.clear()
            } else {
                Vec::new()
            };
            return self.frame_output(
                frame_index,
                TsdfQuerySurfaceFrameStatus::WaitingForSnapshot,
                clear_query_surface_update(
                    self.last_snapshot_version.unwrap_or_default(),
                    removals,
                ),
            );
        };

        if !snapshot.is_valid() {
            let removals = if settings.clear_when_snapshot_unavailable {
                self.cache.clear()
            } else {
                Vec::new()
            };
            return self.frame_output(
                frame_index,
                TsdfQuerySurfaceFrameStatus::InvalidSnapshot,
                clear_query_surface_update(snapshot.version, removals),
            );
        }

        let update = self
            .cache
            .update_from_tsdf(frame_index, snapshot, requests, settings);
        self.last_snapshot_version = Some(snapshot.version);
        self.frame_output(frame_index, TsdfQuerySurfaceFrameStatus::Updated, update)
    }

    fn frame_output(
        &mut self,
        frame_index: u64,
        status: TsdfQuerySurfaceFrameStatus,
        update: TsdfQuerySurfaceUpdate,
    ) -> TsdfQuerySurfaceFrameOutput {
        self.total_upsert_count += update.upserts.len() as u64;
        self.total_removal_count += update.removals.len() as u64;
        TsdfQuerySurfaceFrameOutput {
            frame_index,
            status,
            update,
            cached_surface_count: self.cache.len(),
            last_snapshot_version: self.last_snapshot_version,
            total_upsert_count: self.total_upsert_count,
            total_removal_count: self.total_removal_count,
        }
    }
}

/// Runtime scan-fusion status for diagnostics and UI.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScanFusionStats {
    pub active_voxel_count: usize,
    pub surface_sample_count: usize,
    pub integrated_ray_count: u64,
    pub rejected_ray_count: u64,
    pub dropped_new_voxel_count: u64,
    pub pruned_voxel_count: u64,
    pub voxel_size_meters: f32,
    pub truncation_distance_meters: f32,
}

impl ScanFusionStats {
    pub fn acceptance_ratio(self) -> Option<f32> {
        let total = self.integrated_ray_count + self.rejected_ray_count;
        if total == 0 {
            None
        } else {
            Some(self.integrated_ray_count as f32 / total as f32)
        }
    }
}

/// Stable key for a TSDF-derived mesh chunk.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TsdfMeshChunkKey {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl TsdfMeshChunkKey {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Settings for focused TSDF mesh chunk planning and extraction.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TsdfMeshChunkSettings {
    /// Non-overlap edge length of a chunk in TSDF sample cells.
    pub chunk_edge_voxels: i32,
    /// Extra sample cells included around each chunk for stable surface nets.
    pub overlap_voxels: i32,
    /// Sampling stride for later lower-detail extraction. The reference
    /// extractor currently supports stride 1.
    pub stride_voxels: i32,
    /// Maximum number of planned chunks returned for a focus request.
    pub max_chunk_count: usize,
}

impl Default for TsdfMeshChunkSettings {
    fn default() -> Self {
        Self {
            chunk_edge_voxels: 8,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 32,
        }
    }
}

impl TsdfMeshChunkSettings {
    pub fn normalized(self) -> Self {
        Self {
            chunk_edge_voxels: self.chunk_edge_voxels.max(2),
            overlap_voxels: self.overlap_voxels.max(0),
            stride_voxels: self.stride_voxels.max(1),
            max_chunk_count: if self.max_chunk_count == 0 {
                Self::default().max_chunk_count
            } else {
                self.max_chunk_count
            },
        }
    }
}

/// Work plan for extracting one focused TSDF mesh chunk.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TsdfMeshChunkPlan {
    pub snapshot_version: u64,
    pub key: TsdfMeshChunkKey,
    pub start_coord: VoxelCoord3,
    pub extent_voxels: [i32; 3],
    pub bounds: Bounds3,
    pub fingerprint: u64,
}

impl TsdfMeshChunkPlan {
    pub fn is_valid(&self) -> bool {
        self.bounds.min.is_finite()
            && self.bounds.max.is_finite()
            && self.extent_voxels.iter().all(|extent| *extent >= 2)
            && self.fingerprint != 0
    }
}

/// Extracted TSDF surface mesh for one chunk.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TsdfMeshChunkSnapshot {
    pub snapshot_version: u64,
    pub key: TsdfMeshChunkKey,
    pub fingerprint: u64,
    pub bounds: Bounds3,
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
}

impl TsdfMeshChunkSnapshot {
    pub fn is_valid(&self) -> bool {
        self.fingerprint != 0
            && self.bounds.min.is_finite()
            && self.bounds.max.is_finite()
            && !self.vertices.is_empty()
            && !self.indices.is_empty()
            && self.vertices.iter().copied().all(Vec3::is_finite)
            && self.indices.iter().copied().all(|triangle| {
                triangle
                    .iter()
                    .copied()
                    .all(|index| (index as usize) < self.vertices.len())
            })
    }
}

/// Incremental changes needed to keep a renderer or physics debug view in sync
/// with a focused TSDF mesh cache.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TsdfMeshChunkUpdate {
    pub snapshot_version: u64,
    /// Planned keys in focus priority order for the latest request.
    pub visible_keys: Vec<TsdfMeshChunkKey>,
    /// Planned keys whose cached mesh already matched the latest fingerprint.
    pub retained_keys: Vec<TsdfMeshChunkKey>,
    /// New or changed meshes that should replace renderer-side chunks.
    pub upserts: Vec<TsdfMeshChunkSnapshot>,
    /// Cached meshes that should be removed renderer-side.
    pub removals: Vec<TsdfMeshChunkKey>,
    pub planned_chunk_count: usize,
    pub reused_chunk_count: usize,
    pub extracted_chunk_count: usize,
    pub skipped_empty_chunk_count: usize,
}

impl TsdfMeshChunkUpdate {
    pub fn is_empty(&self) -> bool {
        self.upserts.is_empty() && self.removals.is_empty()
    }

    pub fn changed_chunk_count(&self) -> usize {
        self.upserts.len() + self.removals.len()
    }
}

/// Per-frame input for a focused TSDF mesh update loop.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TsdfMeshChunkFrameRequest {
    pub frame_index: u64,
    pub focus_center: Vec3,
    pub focus_cube_size_meters: f32,
    pub settings: TsdfMeshChunkSettings,
    /// Keep the last usable mesh visible when no current TSDF snapshot is
    /// available. Set this to `true` when the adapter wants provider loss or
    /// invalid snapshots to clear renderer-side chunks immediately.
    pub clear_when_snapshot_unavailable: bool,
}

impl TsdfMeshChunkFrameRequest {
    pub const fn new(
        frame_index: u64,
        focus_center: Vec3,
        focus_cube_size_meters: f32,
        settings: TsdfMeshChunkSettings,
    ) -> Self {
        Self {
            frame_index,
            focus_center,
            focus_cube_size_meters,
            settings,
            clear_when_snapshot_unavailable: false,
        }
    }

    pub const fn with_clear_when_snapshot_unavailable(mut self, clear: bool) -> Self {
        self.clear_when_snapshot_unavailable = clear;
        self
    }

    pub fn is_valid(self) -> bool {
        self.focus_center.is_finite()
            && self.focus_cube_size_meters.is_finite()
            && self.focus_cube_size_meters >= 0.0
    }

    pub fn disables_mesh(self) -> bool {
        self.is_valid() && self.focus_cube_size_meters == 0.0
    }
}

/// Outcome classification for one focused TSDF mesh frame update.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TsdfMeshChunkFrameStatus {
    Updated,
    Disabled,
    WaitingForSnapshot,
    InvalidRequest,
    InvalidSnapshot,
}

/// Result of advancing a focused TSDF mesh driver by one frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TsdfMeshChunkFrameOutput {
    pub frame_index: u64,
    pub status: TsdfMeshChunkFrameStatus,
    pub update: TsdfMeshChunkUpdate,
    pub cached_chunk_count: usize,
    pub last_snapshot_version: Option<u64>,
    pub total_upsert_count: u64,
    pub total_removal_count: u64,
}

impl TsdfMeshChunkFrameOutput {
    pub fn is_render_noop(&self) -> bool {
        self.update.is_empty()
    }
}

/// In-memory cache for focused TSDF mesh chunks.
///
/// This is a CPU/reference utility for apps and adapters that want chunk
/// upsert/removal behavior without depending on a renderer or worker runtime.
/// Platform adapters can keep their own worker threads and still use the same
/// keys, fingerprints, snapshots, and update semantics.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TsdfMeshChunkCache {
    chunks: HashMap<TsdfMeshChunkKey, TsdfMeshChunkSnapshot>,
}

impl TsdfMeshChunkCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn contains_key(&self, key: TsdfMeshChunkKey) -> bool {
        self.chunks.contains_key(&key)
    }

    pub fn get(&self, key: TsdfMeshChunkKey) -> Option<&TsdfMeshChunkSnapshot> {
        self.chunks.get(&key)
    }

    pub fn chunks(&self) -> impl Iterator<Item = &TsdfMeshChunkSnapshot> {
        self.chunks.values()
    }

    pub fn clear(&mut self) -> Vec<TsdfMeshChunkKey> {
        let mut removed = self.chunks.keys().copied().collect::<Vec<_>>();
        removed.sort_by_key(tsdf_mesh_chunk_key_tuple);
        self.chunks.clear();
        removed
    }

    /// Update the cache for a focused world-space cube and return the chunk
    /// changes needed by a renderer or debug view.
    pub fn update_focused(
        &mut self,
        snapshot: &SparseTsdfSnapshot,
        focus_center: Vec3,
        cube_size_meters: f32,
        settings: TsdfMeshChunkSettings,
    ) -> TsdfMeshChunkUpdate {
        let plans =
            plan_focused_tsdf_mesh_chunks(snapshot, focus_center, cube_size_meters, settings);
        let mut update = TsdfMeshChunkUpdate {
            snapshot_version: snapshot.version,
            planned_chunk_count: plans.len(),
            ..TsdfMeshChunkUpdate::default()
        };
        let mut visible = HashSet::with_capacity(plans.len());

        for plan in plans {
            visible.insert(plan.key);
            update.visible_keys.push(plan.key);
            let cached_is_current = self.chunks.get(&plan.key).is_some_and(|chunk| {
                chunk.snapshot_version == plan.snapshot_version
                    && chunk.fingerprint == plan.fingerprint
            });
            if cached_is_current {
                update.retained_keys.push(plan.key);
                update.reused_chunk_count += 1;
                continue;
            }

            match extract_tsdf_mesh_chunk(snapshot, &plan) {
                Some(chunk) => {
                    self.chunks.insert(plan.key, chunk.clone());
                    update.upserts.push(chunk);
                    update.extracted_chunk_count += 1;
                }
                None => {
                    update.skipped_empty_chunk_count += 1;
                    if self.chunks.remove(&plan.key).is_some() {
                        update.removals.push(plan.key);
                    }
                }
            }
        }

        let mut stale_keys = self
            .chunks
            .keys()
            .copied()
            .filter(|key| !visible.contains(key))
            .collect::<Vec<_>>();
        stale_keys.sort_by_key(tsdf_mesh_chunk_key_tuple);
        for key in stale_keys {
            self.chunks.remove(&key);
            update.removals.push(key);
        }
        update.removals.sort_by_key(tsdf_mesh_chunk_key_tuple);
        update
    }
}

/// Frame-by-frame driver for focused realtime TSDF mesh chunks.
///
/// Adapters can call this from their render or worker polling loop. The driver
/// owns only the public cache/update state; TSDF acquisition, renderer upload,
/// worker threads, and physics backend integration stay outside this crate.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TsdfMeshChunkFrameDriver {
    cache: TsdfMeshChunkCache,
    last_frame_index: Option<u64>,
    last_snapshot_version: Option<u64>,
    total_upsert_count: u64,
    total_removal_count: u64,
}

impl TsdfMeshChunkFrameDriver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cache(&self) -> &TsdfMeshChunkCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut TsdfMeshChunkCache {
        &mut self.cache
    }

    pub fn cached_chunk_count(&self) -> usize {
        self.cache.len()
    }

    pub fn last_frame_index(&self) -> Option<u64> {
        self.last_frame_index
    }

    pub fn last_snapshot_version(&self) -> Option<u64> {
        self.last_snapshot_version
    }

    pub fn total_upsert_count(&self) -> u64 {
        self.total_upsert_count
    }

    pub fn total_removal_count(&self) -> u64 {
        self.total_removal_count
    }

    pub fn clear(&mut self, frame_index: u64) -> TsdfMeshChunkFrameOutput {
        self.last_frame_index = Some(frame_index);
        let removals = self.cache.clear();
        self.frame_output(
            frame_index,
            TsdfMeshChunkFrameStatus::Disabled,
            clear_update(self.last_snapshot_version.unwrap_or_default(), removals),
        )
    }

    pub fn advance_frame(
        &mut self,
        request: TsdfMeshChunkFrameRequest,
        snapshot: Option<&SparseTsdfSnapshot>,
    ) -> TsdfMeshChunkFrameOutput {
        self.last_frame_index = Some(request.frame_index);
        if !request.is_valid() {
            return self.frame_output(
                request.frame_index,
                TsdfMeshChunkFrameStatus::InvalidRequest,
                TsdfMeshChunkUpdate {
                    snapshot_version: self.last_snapshot_version.unwrap_or_default(),
                    ..TsdfMeshChunkUpdate::default()
                },
            );
        }

        if request.disables_mesh() {
            return self.clear(request.frame_index);
        }

        let Some(snapshot) = snapshot else {
            let removals = if request.clear_when_snapshot_unavailable {
                self.cache.clear()
            } else {
                Vec::new()
            };
            return self.frame_output(
                request.frame_index,
                TsdfMeshChunkFrameStatus::WaitingForSnapshot,
                clear_update(self.last_snapshot_version.unwrap_or_default(), removals),
            );
        };

        if !snapshot.is_valid() {
            let removals = if request.clear_when_snapshot_unavailable {
                self.cache.clear()
            } else {
                Vec::new()
            };
            return self.frame_output(
                request.frame_index,
                TsdfMeshChunkFrameStatus::InvalidSnapshot,
                clear_update(snapshot.version, removals),
            );
        }

        let update = self.cache.update_focused(
            snapshot,
            request.focus_center,
            request.focus_cube_size_meters,
            request.settings,
        );
        self.last_snapshot_version = Some(snapshot.version);
        self.frame_output(
            request.frame_index,
            TsdfMeshChunkFrameStatus::Updated,
            update,
        )
    }

    fn frame_output(
        &mut self,
        frame_index: u64,
        status: TsdfMeshChunkFrameStatus,
        update: TsdfMeshChunkUpdate,
    ) -> TsdfMeshChunkFrameOutput {
        self.total_upsert_count += update.upserts.len() as u64;
        self.total_removal_count += update.removals.len() as u64;
        TsdfMeshChunkFrameOutput {
            frame_index,
            status,
            update,
            cached_chunk_count: self.cache.len(),
            last_snapshot_version: self.last_snapshot_version,
            total_upsert_count: self.total_upsert_count,
            total_removal_count: self.total_removal_count,
        }
    }
}

/// Plan focused TSDF mesh chunks around a world-space cube.
///
/// This mirrors the useful shape of an object-following debug mesh request:
/// callers choose a moving focus point, and the core helper limits work to
/// chunks intersecting that focus volume.
pub fn plan_focused_tsdf_mesh_chunks(
    snapshot: &SparseTsdfSnapshot,
    focus_center: Vec3,
    cube_size_meters: f32,
    settings: TsdfMeshChunkSettings,
) -> Vec<TsdfMeshChunkPlan> {
    let settings = settings.normalized();
    if !snapshot.is_valid() || !focus_center.is_finite() || cube_size_meters <= 0.0 {
        return Vec::new();
    }
    let Some((active_min, active_max)) = sparse_tsdf_active_coord_bounds(snapshot) else {
        return Vec::new();
    };

    let half = Vec3::splat(cube_size_meters * 0.5);
    let focus_min = world_to_voxel_floor(snapshot, focus_center - half);
    let focus_max = world_to_voxel_floor(snapshot, focus_center + half);
    let coord_min = coord_max(focus_min, active_min);
    let coord_max = coord_min_inclusive(focus_max, active_max);
    if coord_min.x > coord_max.x || coord_min.y > coord_max.y || coord_min.z > coord_max.z {
        return Vec::new();
    }

    let min_key = chunk_key_for_coord(coord_min, settings.chunk_edge_voxels);
    let max_key = chunk_key_for_coord(coord_max, settings.chunk_edge_voxels);
    let mut plans = Vec::new();
    let extent = settings.chunk_edge_voxels + (settings.overlap_voxels * 2) + 1;

    for z in min_key.z..=max_key.z {
        for y in min_key.y..=max_key.y {
            for x in min_key.x..=max_key.x {
                let key = TsdfMeshChunkKey::new(x, y, z);
                let start_coord = VoxelCoord3::new(
                    x * settings.chunk_edge_voxels - settings.overlap_voxels,
                    y * settings.chunk_edge_voxels - settings.overlap_voxels,
                    z * settings.chunk_edge_voxels - settings.overlap_voxels,
                );
                let extent_voxels = [extent, extent, extent];
                let fingerprint = fingerprint_tsdf_region(snapshot, start_coord, extent_voxels);
                if fingerprint == 0 {
                    continue;
                }
                plans.push(TsdfMeshChunkPlan {
                    snapshot_version: snapshot.version,
                    key,
                    start_coord,
                    extent_voxels,
                    bounds: tsdf_region_bounds(snapshot, start_coord, extent_voxels),
                    fingerprint,
                });
            }
        }
    }

    plans.sort_by(|left, right| {
        let left_distance = (left.bounds.center() - focus_center).length_squared();
        let right_distance = (right.bounds.center() - focus_center).length_squared();
        left_distance.total_cmp(&right_distance).then_with(|| {
            (left.key.x, left.key.y, left.key.z).cmp(&(right.key.x, right.key.y, right.key.z))
        })
    });
    plans.truncate(settings.max_chunk_count);
    plans
}

/// Extract a surface-net-style triangle mesh for one TSDF chunk plan.
///
/// This CPU reference favors deterministic output and simple contracts over
/// peak performance. Adapters can replace it with worker or GPU-backed
/// extraction while preserving the same chunk snapshot shape.
pub fn extract_tsdf_mesh_chunk(
    snapshot: &SparseTsdfSnapshot,
    plan: &TsdfMeshChunkPlan,
) -> Option<TsdfMeshChunkSnapshot> {
    if !snapshot.is_valid() || plan.snapshot_version != snapshot.version || !plan.is_valid() {
        return None;
    }

    let lookup = sparse_tsdf_lookup(snapshot);
    let start = plan.start_coord;
    let end = coord_add_extent(start, plan.extent_voxels)?;
    let mut vertices = Vec::<Vec3>::new();
    let mut cell_vertices = HashMap::<VoxelCoord3, u32>::new();

    for z in start.z..(end.z - 1) {
        for y in start.y..(end.y - 1) {
            for x in start.x..(end.x - 1) {
                let cell = VoxelCoord3::new(x, y, z);
                if let Some(vertex) = surface_net_cell_vertex(snapshot, &lookup, cell) {
                    let index = vertices.len() as u32;
                    vertices.push(vertex);
                    cell_vertices.insert(cell, index);
                }
            }
        }
    }
    if vertices.is_empty() {
        return None;
    }

    let mut indices = Vec::<[u32; 3]>::new();
    emit_surface_net_quads(snapshot, &lookup, start, end, &cell_vertices, &mut indices);
    if indices.is_empty() {
        return None;
    }

    Some(TsdfMeshChunkSnapshot {
        snapshot_version: snapshot.version,
        key: plan.key,
        fingerprint: plan.fingerprint,
        bounds: plan.bounds,
        vertices,
        indices,
    })
}

/// Plan and extract focused TSDF mesh chunks in one CPU reference pass.
pub fn extract_focused_tsdf_mesh_chunks(
    snapshot: &SparseTsdfSnapshot,
    focus_center: Vec3,
    cube_size_meters: f32,
    settings: TsdfMeshChunkSettings,
) -> Vec<TsdfMeshChunkSnapshot> {
    plan_focused_tsdf_mesh_chunks(snapshot, focus_center, cube_size_meters, settings)
        .into_iter()
        .filter_map(|plan| extract_tsdf_mesh_chunk(snapshot, &plan))
        .collect()
}

/// SDF sampling mode.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdfSampleMode {
    Nearest,
    Trilinear,
}

/// Axis-aligned mesh/grid bounds.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds3 {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(self) -> Vec3 {
        self.max - self.min
    }

    pub fn expanded(self, padding_meters: f32) -> Self {
        let padding = Vec3::splat(padding_meters.max(0.0));
        Self {
            min: self.min - padding,
            max: self.max + padding,
        }
    }

    pub fn include_sphere(self, center: Vec3, radius_meters: f32) -> Self {
        let extents = Vec3::splat(radius_meters.max(0.0));
        Self {
            min: self.min.min(center - extents),
            max: self.max.max(center + extents),
        }
    }

    pub fn is_valid(self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && self.max.x > self.min.x
            && self.max.y > self.min.y
            && self.max.z > self.min.z
    }
}

/// Packed dense SDF grid with samples at voxel centers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PackedSdfGrid {
    pub version: u64,
    pub origin: Vec3,
    pub voxel_size_meters: f32,
    pub resolution: [usize; 3],
    pub samples: Vec<PackedSdfSample>,
}

impl PackedSdfGrid {
    pub fn from_samples(
        version: u64,
        origin: Vec3,
        voxel_size_meters: f32,
        resolution: [usize; 3],
        samples: Vec<PackedSdfSample>,
    ) -> Result<Self, SdfGridError> {
        let expected_len = voxel_count_for_resolution(resolution)?;
        if samples.len() != expected_len {
            return Err(SdfGridError::SampleCountMismatch {
                expected: expected_len,
                actual: samples.len(),
            });
        }
        if !voxel_size_meters.is_finite() || voxel_size_meters <= 0.0 {
            return Err(SdfGridError::InvalidVoxelSize);
        }

        Ok(Self {
            version,
            origin,
            voxel_size_meters,
            resolution,
            samples,
        })
    }

    pub fn sphere(
        version: u64,
        center: Vec3,
        radius_meters: f32,
        origin: Vec3,
        voxel_size_meters: f32,
        resolution: [usize; 3],
    ) -> Result<Self, SdfGridError> {
        let sample_count = voxel_count_for_resolution(resolution)?;
        let mut samples = Vec::with_capacity(sample_count);
        for z in 0..resolution[2] {
            for y in 0..resolution[1] {
                for x in 0..resolution[0] {
                    let world = origin
                        + Vec3::new(
                            (x as f32 + 0.5) * voxel_size_meters,
                            (y as f32 + 0.5) * voxel_size_meters,
                            (z as f32 + 0.5) * voxel_size_meters,
                        );
                    let from_center = world - center;
                    samples.push(PackedSdfSample::new(
                        from_center.length() - radius_meters,
                        from_center.normalized_or(Vec3::UP),
                    ));
                }
            }
        }

        Self::from_samples(version, origin, voxel_size_meters, resolution, samples)
    }

    pub fn voxel_count(&self) -> usize {
        self.samples.len()
    }

    pub fn bounds(&self) -> Bounds3 {
        Bounds3::new(
            self.origin,
            self.origin
                + Vec3::new(
                    self.resolution[0] as f32 * self.voxel_size_meters,
                    self.resolution[1] as f32 * self.voxel_size_meters,
                    self.resolution[2] as f32 * self.voxel_size_meters,
                ),
        )
    }

    pub fn sample(&self, world: Vec3, mode: SdfSampleMode) -> Option<PackedSdfSample> {
        match mode {
            SdfSampleMode::Nearest => self.sample_nearest(world),
            SdfSampleMode::Trilinear => self.sample_trilinear(world),
        }
    }

    pub fn sample_extrapolated(
        &self,
        world: Vec3,
        mode: SdfSampleMode,
        max_extrapolation_meters: f32,
    ) -> Option<PackedSdfSample> {
        if let Some(sample) = self.sample(world, mode) {
            return Some(sample);
        }
        if !world.is_finite()
            || !max_extrapolation_meters.is_finite()
            || max_extrapolation_meters <= 0.0
        {
            return None;
        }

        let (min, max) = self.sample_center_bounds()?;
        let clamped_world = world.clamp(min, max);
        let outside = world - clamped_world;
        let outside_distance = outside.length();
        if outside_distance > max_extrapolation_meters {
            return None;
        }

        let edge_sample = self.sample(clamped_world, mode)?;
        let edge_normal = edge_sample.normal.normalized_or(Vec3::UP);
        let normal = if outside_distance > 1.0e-5 {
            outside / outside_distance
        } else {
            edge_normal
        };

        Some(PackedSdfSample::new(
            edge_sample.distance_meters.max(0.0) + outside_distance,
            normal,
        ))
    }

    pub fn sample_nearest(&self, world: Vec3) -> Option<PackedSdfSample> {
        let coord = self.grid_coord(world)?;
        let x = coord.x.round() as isize;
        let y = coord.y.round() as isize;
        let z = coord.z.round() as isize;
        self.sample_at(x, y, z)
    }

    pub fn sample_trilinear(&self, world: Vec3) -> Option<PackedSdfSample> {
        let coord = self.grid_coord(world)?;
        let x0 = coord.x.floor() as isize;
        let y0 = coord.y.floor() as isize;
        let z0 = coord.z.floor() as isize;
        let tx = coord.x - x0 as f32;
        let ty = coord.y - y0 as f32;
        let tz = coord.z - z0 as f32;

        let c000 = self.sample_at_clamped(x0, y0, z0)?;
        let c100 = self.sample_at_clamped(x0 + 1, y0, z0)?;
        let c010 = self.sample_at_clamped(x0, y0 + 1, z0)?;
        let c110 = self.sample_at_clamped(x0 + 1, y0 + 1, z0)?;
        let c001 = self.sample_at_clamped(x0, y0, z0 + 1)?;
        let c101 = self.sample_at_clamped(x0 + 1, y0, z0 + 1)?;
        let c011 = self.sample_at_clamped(x0, y0 + 1, z0 + 1)?;
        let c111 = self.sample_at_clamped(x0 + 1, y0 + 1, z0 + 1)?;

        let x00 = lerp_sample(c000, c100, tx);
        let x10 = lerp_sample(c010, c110, tx);
        let x01 = lerp_sample(c001, c101, tx);
        let x11 = lerp_sample(c011, c111, tx);
        let y0_sample = lerp_sample(x00, x10, ty);
        let y1_sample = lerp_sample(x01, x11, ty);
        Some(lerp_sample(y0_sample, y1_sample, tz))
    }

    pub fn sample_at(&self, x: isize, y: isize, z: isize) -> Option<PackedSdfSample> {
        let index = self.index(x, y, z)?;
        self.samples.get(index).copied()
    }

    fn sample_center_bounds(&self) -> Option<(Vec3, Vec3)> {
        if self.samples.is_empty()
            || self.resolution[0] == 0
            || self.resolution[1] == 0
            || self.resolution[2] == 0
        {
            return None;
        }

        let half_voxel = Vec3::splat(self.voxel_size_meters * 0.5);
        let min = self.origin + half_voxel;
        let max = self.origin
            + Vec3::new(
                (self.resolution[0] as f32 - 0.5) * self.voxel_size_meters,
                (self.resolution[1] as f32 - 0.5) * self.voxel_size_meters,
                (self.resolution[2] as f32 - 0.5) * self.voxel_size_meters,
            );
        Some((min, max))
    }

    fn sample_at_clamped(&self, x: isize, y: isize, z: isize) -> Option<PackedSdfSample> {
        if self.resolution[0] == 0 || self.resolution[1] == 0 || self.resolution[2] == 0 {
            return None;
        }
        let x = x.clamp(0, self.resolution[0].saturating_sub(1) as isize);
        let y = y.clamp(0, self.resolution[1].saturating_sub(1) as isize);
        let z = z.clamp(0, self.resolution[2].saturating_sub(1) as isize);
        self.sample_at(x, y, z)
    }

    fn grid_coord(&self, world: Vec3) -> Option<Vec3> {
        if !world.is_finite() || !self.origin.is_finite() || self.voxel_size_meters <= 0.0 {
            return None;
        }
        let local = (world - self.origin) / self.voxel_size_meters;
        let coord = local - Vec3::splat(0.5);
        let max = Vec3::new(
            self.resolution[0].saturating_sub(1) as f32,
            self.resolution[1].saturating_sub(1) as f32,
            self.resolution[2].saturating_sub(1) as f32,
        );
        if coord.x < 0.0 || coord.y < 0.0 || coord.z < 0.0 {
            return None;
        }
        if coord.x > max.x || coord.y > max.y || coord.z > max.z {
            return None;
        }
        Some(coord)
    }

    fn index(&self, x: isize, y: isize, z: isize) -> Option<usize> {
        if x < 0 || y < 0 || z < 0 {
            return None;
        }
        let x = x as usize;
        let y = y as usize;
        let z = z as usize;
        if x >= self.resolution[0] || y >= self.resolution[1] || z >= self.resolution[2] {
            return None;
        }
        Some(x + (y * self.resolution[0]) + (z * self.resolution[0] * self.resolution[1]))
    }
}

/// Triangle mesh snapshot suitable for SDF conversion adapters.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TriangleMeshSnapshot {
    pub version: u64,
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
}

impl TriangleMeshSnapshot {
    pub fn new(version: u64, vertices: Vec<Vec3>, indices: Vec<[u32; 3]>) -> Self {
        Self {
            version,
            vertices,
            indices,
        }
    }

    pub fn validate(&self) -> Result<(), SdfGridError> {
        if self.vertices.is_empty() || self.indices.is_empty() {
            return Err(SdfGridError::EmptyMesh);
        }
        for (triangle_index, triangle) in self.indices.iter().copied().enumerate() {
            for vertex_index in triangle {
                if vertex_index as usize >= self.vertices.len() {
                    return Err(SdfGridError::InvalidMeshIndex {
                        triangle_index,
                        vertex_index,
                        vertex_count: self.vertices.len(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn bounds(&self) -> Option<Bounds3> {
        let mut vertices = self.vertices.iter().copied();
        let first = vertices.next()?;
        let mut min = first;
        let mut max = first;
        for vertex in vertices {
            min = min.min(vertex);
            max = max.max(vertex);
        }
        Some(Bounds3::new(min, max))
    }
}

/// Convert a public hand mesh snapshot into the SDF crate's mesh snapshot.
///
/// Native adapters should perform platform coordinate-space conversion before
/// constructing `HandMeshSnapshot`; this helper only validates and copies the
/// already-public mesh data.
pub fn triangle_mesh_snapshot_from_hand_mesh_snapshot(
    snapshot: &HandMeshSnapshot,
) -> Result<TriangleMeshSnapshot, HandMeshError> {
    snapshot.validate()?;
    Ok(TriangleMeshSnapshot::new(
        snapshot.version,
        snapshot.vertices.clone(),
        snapshot.indices.clone(),
    ))
}

/// Sign convention used when converting a triangle mesh into a packed SDF.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeshSdfSignMode {
    /// Uses ray parity. This expects a closed, consistently wound mesh.
    ClosedMeshRaycast,
    /// Uses the closest triangle normal. Useful for open or thin dynamic meshes.
    TriangleNormal,
}

/// Settings for the dependency-light CPU mesh-to-SDF reference builder.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshToSdfConfig {
    pub voxel_size_meters: f32,
    pub padding_meters: f32,
    pub max_voxels: usize,
    pub distance_offset_meters: f32,
    pub sign_mode: MeshSdfSignMode,
}

impl Default for MeshToSdfConfig {
    fn default() -> Self {
        Self {
            voxel_size_meters: 0.015,
            padding_meters: 0.08,
            max_voxels: 512 * 512,
            distance_offset_meters: 0.0,
            sign_mode: MeshSdfSignMode::ClosedMeshRaycast,
        }
    }
}

/// Specific config validation failure for mesh-to-SDF conversion.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshToSdfConfigError {
    InvalidVoxelSize,
    InvalidPadding,
    InvalidMaxVoxels,
    InvalidDistanceOffset,
    InvalidBounds,
    NonPositiveBounds,
    VoxelCountOverflow,
}

/// Errors from the mesh-to-SDF reference builder.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshToSdfError {
    EmptyMesh,
    DegenerateMesh,
    InvalidVertex {
        vertex_index: usize,
    },
    InvalidIndex {
        triangle_index: usize,
        vertex_index: u32,
        vertex_count: usize,
    },
    InvalidConfig(MeshToSdfConfigError),
    VoxelLimitExceeded {
        requested: usize,
        limit: usize,
    },
    GridBuild(SdfGridError),
}

impl fmt::Display for MeshToSdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMesh => f.write_str("mesh has no vertices or triangles"),
            Self::DegenerateMesh => f.write_str("mesh has no non-degenerate triangles"),
            Self::InvalidVertex { vertex_index } => {
                write!(f, "mesh vertex {vertex_index} is not finite")
            }
            Self::InvalidIndex {
                triangle_index,
                vertex_index,
                vertex_count,
            } => write!(
                f,
                "triangle {triangle_index} references vertex {vertex_index}, but mesh has {vertex_count} vertices"
            ),
            Self::InvalidConfig(reason) => write!(f, "invalid mesh-to-SDF config: {reason}"),
            Self::VoxelLimitExceeded { requested, limit } => write!(
                f,
                "mesh-to-SDF volume requests {requested} voxels, above limit {limit}"
            ),
            Self::GridBuild(error) => write!(f, "failed to build packed SDF grid: {error}"),
        }
    }
}

impl std::error::Error for MeshToSdfError {}

impl fmt::Display for MeshToSdfConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVoxelSize => f.write_str("voxel_size_meters must be finite and positive"),
            Self::InvalidPadding => f.write_str("padding_meters must be finite and non-negative"),
            Self::InvalidMaxVoxels => f.write_str("max_voxels must be positive"),
            Self::InvalidDistanceOffset => f.write_str("distance_offset_meters must be finite"),
            Self::InvalidBounds => f.write_str("bounds must be finite"),
            Self::NonPositiveBounds => f.write_str("bounds must have positive size"),
            Self::VoxelCountOverflow => f.write_str("voxel count overflow"),
        }
    }
}

/// Build a packed SDF around the mesh's own bounds plus config padding.
pub fn build_sdf_from_mesh(
    version: u64,
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
) -> Result<PackedSdfGrid, MeshToSdfError> {
    let bounds = mesh.bounds().ok_or(MeshToSdfError::EmptyMesh)?;
    build_sdf_from_mesh_bounds(version, mesh, config, bounds)
}

/// Build a packed SDF around explicit bounds plus config padding.
///
/// Explicit bounds are useful when particles or other consumers start outside
/// the mesh bounds but still need a valid SDF sample region.
pub fn build_sdf_from_mesh_bounds(
    version: u64,
    mesh: &TriangleMeshSnapshot,
    config: MeshToSdfConfig,
    bounds: Bounds3,
) -> Result<PackedSdfGrid, MeshToSdfError> {
    validate_mesh_for_sdf(mesh)?;
    validate_mesh_to_sdf_config(config)?;

    let bounds = bounds.expanded(config.padding_meters);
    if !bounds.min.is_finite() || !bounds.max.is_finite() {
        return Err(MeshToSdfError::InvalidConfig(
            MeshToSdfConfigError::InvalidBounds,
        ));
    }

    let size = bounds.size();
    if size.x <= 0.0 || size.y <= 0.0 || size.z <= 0.0 {
        return Err(MeshToSdfError::InvalidConfig(
            MeshToSdfConfigError::NonPositiveBounds,
        ));
    }

    let resolution = [
        mesh_sdf_resolution_axis(size.x, config.voxel_size_meters),
        mesh_sdf_resolution_axis(size.y, config.voxel_size_meters),
        mesh_sdf_resolution_axis(size.z, config.voxel_size_meters),
    ];
    let voxel_count = voxel_count_for_resolution(resolution)
        .map_err(|_| MeshToSdfError::InvalidConfig(MeshToSdfConfigError::VoxelCountOverflow))?;
    if voxel_count > config.max_voxels {
        return Err(MeshToSdfError::VoxelLimitExceeded {
            requested: voxel_count,
            limit: config.max_voxels,
        });
    }

    let triangles = PreparedMeshSdfTriangle::from_mesh(mesh)?;
    let mut samples = Vec::with_capacity(voxel_count);
    let mut ray_hits = Vec::with_capacity(triangles.len().min(128));
    for z in 0..resolution[2] {
        for y in 0..resolution[1] {
            for x in 0..resolution[0] {
                let world = bounds.min
                    + Vec3::new(
                        (x as f32 + 0.5) * config.voxel_size_meters,
                        (y as f32 + 0.5) * config.voxel_size_meters,
                        (z as f32 + 0.5) * config.voxel_size_meters,
                    );
                samples.push(sample_mesh_sdf(
                    &triangles,
                    world,
                    config.sign_mode,
                    config.distance_offset_meters,
                    &mut ray_hits,
                ));
            }
        }
    }

    PackedSdfGrid::from_samples(
        version,
        bounds.min,
        config.voxel_size_meters,
        resolution,
        samples,
    )
    .map_err(MeshToSdfError::GridBuild)
}

/// Errors for public SDF contracts and validation helpers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdfGridError {
    ResolutionOverflow,
    InvalidVoxelSize,
    SampleCountMismatch {
        expected: usize,
        actual: usize,
    },
    EmptyMesh,
    InvalidMeshIndex {
        triangle_index: usize,
        vertex_index: u32,
        vertex_count: usize,
    },
}

impl fmt::Display for SdfGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResolutionOverflow => f.write_str("SDF resolution overflows usize"),
            Self::InvalidVoxelSize => f.write_str("SDF voxel size must be finite and positive"),
            Self::SampleCountMismatch { expected, actual } => {
                write!(f, "SDF sample count mismatch: got {actual}, expected {expected}")
            }
            Self::EmptyMesh => f.write_str("triangle mesh is empty"),
            Self::InvalidMeshIndex {
                triangle_index,
                vertex_index,
                vertex_count,
            } => write!(
                f,
                "triangle {triangle_index} references vertex {vertex_index}, but mesh has {vertex_count} vertices"
            ),
        }
    }
}

impl std::error::Error for SdfGridError {}

fn voxel_count_for_resolution(resolution: [usize; 3]) -> Result<usize, SdfGridError> {
    resolution[0]
        .checked_mul(resolution[1])
        .and_then(|value| value.checked_mul(resolution[2]))
        .ok_or(SdfGridError::ResolutionOverflow)
}

fn lerp_sample(left: PackedSdfSample, right: PackedSdfSample, t: f32) -> PackedSdfSample {
    PackedSdfSample::new(
        left.distance_meters + ((right.distance_meters - left.distance_meters) * t),
        (left.normal + ((right.normal - left.normal) * t)).normalized_or(Vec3::UP),
    )
}

fn validate_mesh_for_sdf(mesh: &TriangleMeshSnapshot) -> Result<(), MeshToSdfError> {
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return Err(MeshToSdfError::EmptyMesh);
    }

    for (vertex_index, vertex) in mesh.vertices.iter().copied().enumerate() {
        if !vertex.is_finite() {
            return Err(MeshToSdfError::InvalidVertex { vertex_index });
        }
    }

    let vertex_count = mesh.vertices.len();
    for (triangle_index, triangle) in mesh.indices.iter().copied().enumerate() {
        for vertex_index in triangle {
            if vertex_index as usize >= vertex_count {
                return Err(MeshToSdfError::InvalidIndex {
                    triangle_index,
                    vertex_index,
                    vertex_count,
                });
            }
        }
    }

    Ok(())
}

fn validate_mesh_to_sdf_config(config: MeshToSdfConfig) -> Result<(), MeshToSdfError> {
    if !config.voxel_size_meters.is_finite() || config.voxel_size_meters <= 0.0 {
        return Err(MeshToSdfError::InvalidConfig(
            MeshToSdfConfigError::InvalidVoxelSize,
        ));
    }
    if !config.padding_meters.is_finite() || config.padding_meters < 0.0 {
        return Err(MeshToSdfError::InvalidConfig(
            MeshToSdfConfigError::InvalidPadding,
        ));
    }
    if config.max_voxels == 0 {
        return Err(MeshToSdfError::InvalidConfig(
            MeshToSdfConfigError::InvalidMaxVoxels,
        ));
    }
    if !config.distance_offset_meters.is_finite() {
        return Err(MeshToSdfError::InvalidConfig(
            MeshToSdfConfigError::InvalidDistanceOffset,
        ));
    }
    Ok(())
}

fn mesh_sdf_resolution_axis(size_meters: f32, voxel_size_meters: f32) -> usize {
    (size_meters / voxel_size_meters).ceil().max(2.0) as usize
}

#[derive(Clone, Copy, Debug)]
struct PreparedMeshSdfTriangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    normal: Vec3,
}

impl PreparedMeshSdfTriangle {
    fn from_mesh(mesh: &TriangleMeshSnapshot) -> Result<Vec<Self>, MeshToSdfError> {
        let mut triangles = Vec::with_capacity(mesh.indices.len());
        for indices in &mesh.indices {
            let a = mesh.vertices[indices[0] as usize];
            let b = mesh.vertices[indices[1] as usize];
            let c = mesh.vertices[indices[2] as usize];
            let normal = (b - a).cross(c - a);
            if normal.length_squared() <= 1.0e-14 {
                continue;
            }
            triangles.push(Self {
                a,
                b,
                c,
                normal: normal.normalized_or(Vec3::UP),
            });
        }

        if triangles.is_empty() {
            return Err(MeshToSdfError::DegenerateMesh);
        }
        Ok(triangles)
    }
}

fn sample_mesh_sdf(
    triangles: &[PreparedMeshSdfTriangle],
    point: Vec3,
    sign_mode: MeshSdfSignMode,
    distance_offset_meters: f32,
    ray_hits: &mut Vec<f32>,
) -> PackedSdfSample {
    let mut best_distance_sq = f32::INFINITY;
    let mut best_closest = point;
    let mut best_triangle = triangles[0];

    for triangle in triangles {
        let closest = closest_point_on_triangle(point, triangle.a, triangle.b, triangle.c);
        let delta = point - closest;
        let distance_sq = delta.length_squared();
        if distance_sq < best_distance_sq {
            best_distance_sq = distance_sq;
            best_closest = closest;
            best_triangle = *triangle;
        }
    }

    let distance = best_distance_sq.sqrt();
    let sign = match sign_mode {
        MeshSdfSignMode::ClosedMeshRaycast => {
            if point_inside_closed_mesh(point, triangles, ray_hits) {
                -1.0
            } else {
                1.0
            }
        }
        MeshSdfSignMode::TriangleNormal => {
            if (point - best_triangle.a).dot(best_triangle.normal) < 0.0 {
                -1.0
            } else {
                1.0
            }
        }
    };

    let from_surface = (point - best_closest).normalized_or(best_triangle.normal);
    let normal = if sign < 0.0 {
        -from_surface
    } else {
        from_surface
    }
    .normalized_or(best_triangle.normal);

    PackedSdfSample::new((distance * sign) + distance_offset_meters, normal)
}

fn closest_point_on_triangle(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = (d1 * d4) - (d3 * d2);
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + (ab * v);
    }

    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = (d5 * d2) - (d1 * d6);
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + (ac * w);
    }

    let va = (d3 * d6) - (d5 * d4);
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + ((c - b) * w);
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    a + (ab * v) + (ac * w)
}

fn point_inside_closed_mesh(
    point: Vec3,
    triangles: &[PreparedMeshSdfTriangle],
    ray_hits: &mut Vec<f32>,
) -> bool {
    let ray_dir = Vec3::new(1.0, 0.000_131, 0.000_217).normalized_or(Vec3::RIGHT);
    ray_hits.clear();
    for triangle in triangles {
        if let Some(t) = ray_triangle_intersection(point, ray_dir, triangle) {
            ray_hits.push(t);
        }
    }

    ray_hits.sort_by(|left, right| left.total_cmp(right));
    let mut unique_hits = 0usize;
    let mut last_t = f32::NEG_INFINITY;
    for t in ray_hits.iter().copied() {
        if (t - last_t).abs() > 1.0e-4 {
            unique_hits += 1;
            last_t = t;
        }
    }
    unique_hits % 2 == 1
}

fn ray_triangle_intersection(
    origin: Vec3,
    direction: Vec3,
    triangle: &PreparedMeshSdfTriangle,
) -> Option<f32> {
    let edge1 = triangle.b - triangle.a;
    let edge2 = triangle.c - triangle.a;
    let p = direction.cross(edge2);
    let determinant = edge1.dot(p);
    if determinant.abs() <= 1.0e-7 {
        return None;
    }

    let inv_determinant = 1.0 / determinant;
    let tvec = origin - triangle.a;
    let u = tvec.dot(p) * inv_determinant;
    if !(-1.0e-6..=1.0 + 1.0e-6).contains(&u) {
        return None;
    }

    let q = tvec.cross(edge1);
    let v = direction.dot(q) * inv_determinant;
    if v < -1.0e-6 || u + v > 1.0 + 1.0e-6 {
        return None;
    }

    let t = edge2.dot(q) * inv_determinant;
    if t > 1.0e-6 {
        Some(t)
    } else {
        None
    }
}

fn sparse_tsdf_active_coord_bounds(
    snapshot: &SparseTsdfSnapshot,
) -> Option<(VoxelCoord3, VoxelCoord3)> {
    let mut samples = snapshot
        .samples
        .iter()
        .filter(|sample| sample.confidence > 0);
    let first = samples.next()?.coord;
    let mut min = first;
    let mut max = first;
    for sample in samples {
        min = coord_min_inclusive(min, sample.coord);
        max = coord_max(max, sample.coord);
    }
    Some((min, max))
}

fn coord_min_inclusive(left: VoxelCoord3, right: VoxelCoord3) -> VoxelCoord3 {
    VoxelCoord3::new(
        left.x.min(right.x),
        left.y.min(right.y),
        left.z.min(right.z),
    )
}

fn coord_max(left: VoxelCoord3, right: VoxelCoord3) -> VoxelCoord3 {
    VoxelCoord3::new(
        left.x.max(right.x),
        left.y.max(right.y),
        left.z.max(right.z),
    )
}

fn coord_add_extent(coord: VoxelCoord3, extent: [i32; 3]) -> Option<VoxelCoord3> {
    Some(VoxelCoord3::new(
        coord.x.checked_add(extent[0])?,
        coord.y.checked_add(extent[1])?,
        coord.z.checked_add(extent[2])?,
    ))
}

fn world_to_voxel_floor(snapshot: &SparseTsdfSnapshot, world: Vec3) -> VoxelCoord3 {
    let local = (world - snapshot.origin) / snapshot.voxel_size_meters;
    VoxelCoord3::new(
        local.x.floor() as i32,
        local.y.floor() as i32,
        local.z.floor() as i32,
    )
}

fn chunk_key_for_coord(coord: VoxelCoord3, chunk_edge_voxels: i32) -> TsdfMeshChunkKey {
    let edge = chunk_edge_voxels.max(1);
    TsdfMeshChunkKey::new(
        coord.x.div_euclid(edge),
        coord.y.div_euclid(edge),
        coord.z.div_euclid(edge),
    )
}

fn tsdf_mesh_chunk_key_tuple(key: &TsdfMeshChunkKey) -> (i32, i32, i32) {
    (key.x, key.y, key.z)
}

fn depth_query_surface_key_tuple(key: &DepthQuerySurfaceKey) -> (u64, u8) {
    (
        key.request_key,
        match key.role {
            DepthQuerySurfaceRole::Support => 0,
            DepthQuerySurfaceRole::Impact => 1,
        },
    )
}

fn normalize_nonnegative_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}

fn clear_query_surface_update(
    snapshot_version: u64,
    removals: Vec<DepthQuerySurfaceKey>,
) -> TsdfQuerySurfaceUpdate {
    TsdfQuerySurfaceUpdate {
        snapshot_version,
        removals,
        ..TsdfQuerySurfaceUpdate::default()
    }
}

fn surface_entries_match_without_frame(
    left: DepthQuerySurfaceEntry,
    right: DepthQuerySurfaceEntry,
) -> bool {
    left.key == right.key
        && left.snapshot_version == right.snapshot_version
        && left.surface == right.surface
        && left.sample_count == right.sample_count
}

fn clear_update(snapshot_version: u64, removals: Vec<TsdfMeshChunkKey>) -> TsdfMeshChunkUpdate {
    TsdfMeshChunkUpdate {
        snapshot_version,
        removals,
        ..TsdfMeshChunkUpdate::default()
    }
}

fn tsdf_region_bounds(
    snapshot: &SparseTsdfSnapshot,
    start: VoxelCoord3,
    extent: [i32; 3],
) -> Bounds3 {
    let min = snapshot.origin
        + Vec3::new(
            start.x as f32 * snapshot.voxel_size_meters,
            start.y as f32 * snapshot.voxel_size_meters,
            start.z as f32 * snapshot.voxel_size_meters,
        );
    let max = snapshot.origin
        + Vec3::new(
            (start.x + extent[0]) as f32 * snapshot.voxel_size_meters,
            (start.y + extent[1]) as f32 * snapshot.voxel_size_meters,
            (start.z + extent[2]) as f32 * snapshot.voxel_size_meters,
        );
    Bounds3::new(min, max)
}

fn coord_in_region(coord: VoxelCoord3, start: VoxelCoord3, extent: [i32; 3]) -> bool {
    coord.x >= start.x
        && coord.y >= start.y
        && coord.z >= start.z
        && coord.x < start.x + extent[0]
        && coord.y < start.y + extent[1]
        && coord.z < start.z + extent[2]
}

fn fingerprint_tsdf_region(
    snapshot: &SparseTsdfSnapshot,
    start: VoxelCoord3,
    extent: [i32; 3],
) -> u64 {
    let mut samples = snapshot
        .samples
        .iter()
        .copied()
        .filter(|sample| sample.confidence > 0 && coord_in_region(sample.coord, start, extent))
        .collect::<Vec<_>>();
    if samples.is_empty() {
        return 0;
    }
    samples.sort_by_key(|sample| (sample.coord.x, sample.coord.y, sample.coord.z));

    let mut hash = FNV_OFFSET_BASIS;
    hash_u64(&mut hash, snapshot.version);
    hash_u64(&mut hash, snapshot.origin.x.to_bits() as u64);
    hash_u64(&mut hash, snapshot.origin.y.to_bits() as u64);
    hash_u64(&mut hash, snapshot.origin.z.to_bits() as u64);
    hash_u64(&mut hash, snapshot.voxel_size_meters.to_bits() as u64);
    hash_u64(
        &mut hash,
        snapshot.truncation_distance_meters.to_bits() as u64,
    );
    for sample in samples {
        hash_i32(&mut hash, sample.coord.x);
        hash_i32(&mut hash, sample.coord.y);
        hash_i32(&mut hash, sample.coord.z);
        hash_u64(&mut hash, sample.normalized_distance.to_bits() as u64);
        hash_u64(&mut hash, sample.confidence as u64);
        hash_u64(&mut hash, sample.last_seen_time_ns.unwrap_or(0));
    }
    if hash == 0 {
        FNV_OFFSET_BASIS
    } else {
        hash
    }
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn hash_i32(hash: &mut u64, value: i32) {
    hash_u64(hash, value as u32 as u64);
}

fn hash_u64(hash: &mut u64, value: u64) {
    *hash ^= value;
    *hash = hash.wrapping_mul(FNV_PRIME);
}

fn sparse_tsdf_lookup(snapshot: &SparseTsdfSnapshot) -> HashMap<VoxelCoord3, SparseTsdfSample> {
    snapshot
        .samples
        .iter()
        .copied()
        .filter(|sample| sample.confidence > 0 && sample.normalized_distance.is_finite())
        .map(|sample| (sample.coord, sample))
        .collect()
}

fn sample_signed_distance(
    snapshot: &SparseTsdfSnapshot,
    lookup: &HashMap<VoxelCoord3, SparseTsdfSample>,
    coord: VoxelCoord3,
) -> Option<f32> {
    lookup
        .get(&coord)
        .copied()
        .map(|sample| sample.signed_distance_meters(snapshot.truncation_distance_meters))
        .filter(|distance| distance.is_finite())
}

#[derive(Clone, Copy, Debug)]
struct TsdfSupportCandidate {
    surface_point: Vec3,
    normal: Vec3,
    signed_distance: f32,
    confidence: u8,
}

#[derive(Clone, Copy, Debug)]
struct TsdfImpactCandidate {
    surface_point: Vec3,
    normal: Vec3,
    impact_center: Vec3,
    travel_fraction: f32,
    lateral_distance: f32,
    signed_clearance: f32,
    confidence: u8,
}

fn impact_candidate_is_better(left: TsdfImpactCandidate, right: TsdfImpactCandidate) -> bool {
    left.travel_fraction
        .total_cmp(&right.travel_fraction)
        .then_with(|| left.lateral_distance.total_cmp(&right.lateral_distance))
        .then_with(|| right.confidence.cmp(&left.confidence))
        .is_lt()
}

fn estimate_sparse_tsdf_normal(
    snapshot: &SparseTsdfSnapshot,
    lookup: &HashMap<VoxelCoord3, SparseTsdfSample>,
    coord: VoxelCoord3,
) -> Option<Vec3> {
    let dx = estimate_sparse_tsdf_axis_gradient(snapshot, lookup, coord, VoxelCoord3::new(1, 0, 0));
    let dy = estimate_sparse_tsdf_axis_gradient(snapshot, lookup, coord, VoxelCoord3::new(0, 1, 0));
    let dz = estimate_sparse_tsdf_axis_gradient(snapshot, lookup, coord, VoxelCoord3::new(0, 0, 1));
    let gradient = Vec3::new(dx?, dy?, dz?);
    let normal = gradient.normalized_or(Vec3::ZERO);
    (normal.length_squared() > 0.0).then_some(normal)
}

fn estimate_sparse_tsdf_axis_gradient(
    snapshot: &SparseTsdfSnapshot,
    lookup: &HashMap<VoxelCoord3, SparseTsdfSample>,
    coord: VoxelCoord3,
    axis: VoxelCoord3,
) -> Option<f32> {
    let forward = offset_coord(coord, axis)?;
    let backward = offset_coord(coord, VoxelCoord3::new(-axis.x, -axis.y, -axis.z))?;
    let forward_distance = sample_signed_distance(snapshot, lookup, forward);
    let center_distance = sample_signed_distance(snapshot, lookup, coord);
    let backward_distance = sample_signed_distance(snapshot, lookup, backward);
    let voxel_size = snapshot.voxel_size_meters;

    match (backward_distance, center_distance, forward_distance) {
        (Some(backward), _, Some(forward)) => Some((forward - backward) / (2.0 * voxel_size)),
        (_, Some(center), Some(forward)) => Some((forward - center) / voxel_size),
        (Some(backward), Some(center), _) => Some((center - backward) / voxel_size),
        _ => None,
    }
}

fn offset_coord(coord: VoxelCoord3, delta: VoxelCoord3) -> Option<VoxelCoord3> {
    Some(VoxelCoord3::new(
        coord.x.checked_add(delta.x)?,
        coord.y.checked_add(delta.y)?,
        coord.z.checked_add(delta.z)?,
    ))
}

fn support_plane_basis(normal: Vec3) -> (Vec3, Vec3) {
    let normal = normal.normalized_or(Vec3::UP);
    let tangent_seed = if normal.dot(Vec3::RIGHT).abs() < 0.8 {
        Vec3::RIGHT
    } else {
        Vec3::FORWARD_NEG_Z
    };
    let tangent = (tangent_seed - (normal * tangent_seed.dot(normal))).normalized_or(Vec3::RIGHT);
    let bitangent = normal.cross(tangent).normalized_or(Vec3::FORWARD_NEG_Z);
    (tangent, bitangent)
}

fn voxel_center(snapshot: &SparseTsdfSnapshot, coord: VoxelCoord3) -> Vec3 {
    snapshot.voxel_center_world(coord)
}

fn surface_net_cell_vertex(
    snapshot: &SparseTsdfSnapshot,
    lookup: &HashMap<VoxelCoord3, SparseTsdfSample>,
    cell: VoxelCoord3,
) -> Option<Vec3> {
    const CORNERS: [(i32, i32, i32); 8] = [
        (0, 0, 0),
        (1, 0, 0),
        (0, 1, 0),
        (1, 1, 0),
        (0, 0, 1),
        (1, 0, 1),
        (0, 1, 1),
        (1, 1, 1),
    ];
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    let mut distances = [0.0f32; 8];
    let mut positions = [Vec3::ZERO; 8];
    let mut has_negative = false;
    let mut has_positive = false;

    for (index, (dx, dy, dz)) in CORNERS.iter().copied().enumerate() {
        let coord = VoxelCoord3::new(cell.x + dx, cell.y + dy, cell.z + dz);
        let distance = sample_signed_distance(snapshot, lookup, coord)?;
        distances[index] = distance;
        positions[index] = voxel_center(snapshot, coord);
        has_negative |= distance < 0.0;
        has_positive |= distance > 0.0;
    }

    if !has_negative || !has_positive {
        return None;
    }

    let mut sum = Vec3::ZERO;
    let mut count = 0u32;
    for (a, b) in EDGES {
        let da = distances[a];
        let db = distances[b];
        if !signed_distances_cross(da, db) {
            continue;
        }
        let t = if (da - db).abs() <= f32::EPSILON {
            0.5
        } else {
            (da / (da - db)).clamp(0.0, 1.0)
        };
        sum += positions[a] + ((positions[b] - positions[a]) * t);
        count += 1;
    }

    if count == 0 {
        Some(
            positions
                .iter()
                .copied()
                .fold(Vec3::ZERO, |acc, position| acc + position)
                / positions.len() as f32,
        )
    } else {
        Some(sum / count as f32)
    }
}

fn signed_distances_cross(left: f32, right: f32) -> bool {
    left.is_finite()
        && right.is_finite()
        && left != right
        && ((left <= 0.0 && right >= 0.0) || (left >= 0.0 && right <= 0.0))
}

fn emit_surface_net_quads(
    snapshot: &SparseTsdfSnapshot,
    lookup: &HashMap<VoxelCoord3, SparseTsdfSample>,
    start: VoxelCoord3,
    end: VoxelCoord3,
    cell_vertices: &HashMap<VoxelCoord3, u32>,
    indices: &mut Vec<[u32; 3]>,
) {
    for z in start.z..end.z {
        for y in start.y..end.y {
            for x in start.x..(end.x - 1) {
                let coord = VoxelCoord3::new(x, y, z);
                let Some(flip) = edge_crosses(snapshot, lookup, coord, VoxelCoord3::new(1, 0, 0))
                else {
                    continue;
                };
                push_cell_quad(
                    cell_vertices,
                    indices,
                    [
                        VoxelCoord3::new(x, y - 1, z - 1),
                        VoxelCoord3::new(x, y, z - 1),
                        VoxelCoord3::new(x, y, z),
                        VoxelCoord3::new(x, y - 1, z),
                    ],
                    flip,
                );
            }
        }
    }

    for z in start.z..end.z {
        for y in start.y..(end.y - 1) {
            for x in start.x..end.x {
                let coord = VoxelCoord3::new(x, y, z);
                let Some(flip) = edge_crosses(snapshot, lookup, coord, VoxelCoord3::new(0, 1, 0))
                else {
                    continue;
                };
                push_cell_quad(
                    cell_vertices,
                    indices,
                    [
                        VoxelCoord3::new(x - 1, y, z - 1),
                        VoxelCoord3::new(x, y, z - 1),
                        VoxelCoord3::new(x, y, z),
                        VoxelCoord3::new(x - 1, y, z),
                    ],
                    flip,
                );
            }
        }
    }

    for z in start.z..(end.z - 1) {
        for y in start.y..end.y {
            for x in start.x..end.x {
                let coord = VoxelCoord3::new(x, y, z);
                let Some(flip) = edge_crosses(snapshot, lookup, coord, VoxelCoord3::new(0, 0, 1))
                else {
                    continue;
                };
                push_cell_quad(
                    cell_vertices,
                    indices,
                    [
                        VoxelCoord3::new(x - 1, y - 1, z),
                        VoxelCoord3::new(x, y - 1, z),
                        VoxelCoord3::new(x, y, z),
                        VoxelCoord3::new(x - 1, y, z),
                    ],
                    flip,
                );
            }
        }
    }
}

fn edge_crosses(
    snapshot: &SparseTsdfSnapshot,
    lookup: &HashMap<VoxelCoord3, SparseTsdfSample>,
    coord: VoxelCoord3,
    delta: VoxelCoord3,
) -> Option<bool> {
    let right = VoxelCoord3::new(coord.x + delta.x, coord.y + delta.y, coord.z + delta.z);
    let left_distance = sample_signed_distance(snapshot, lookup, coord)?;
    let right_distance = sample_signed_distance(snapshot, lookup, right)?;
    signed_distances_cross(left_distance, right_distance).then_some(left_distance > right_distance)
}

fn push_cell_quad(
    cell_vertices: &HashMap<VoxelCoord3, u32>,
    indices: &mut Vec<[u32; 3]>,
    cells: [VoxelCoord3; 4],
    flip: bool,
) {
    let Some(&a) = cell_vertices.get(&cells[0]) else {
        return;
    };
    let Some(&b) = cell_vertices.get(&cells[1]) else {
        return;
    };
    let Some(&c) = cell_vertices.get(&cells[2]) else {
        return;
    };
    let Some(&d) = cell_vertices.get(&cells[3]) else {
        return;
    };
    if flip {
        indices.push([a, c, b]);
        indices.push([a, d, c]);
    } else {
        indices.push([a, b, c]);
        indices.push([a, c, d]);
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
    fn builds_and_samples_sphere_grid() {
        let grid = PackedSdfGrid::sphere(
            1,
            Vec3::ZERO,
            0.5,
            Vec3::new(-1.0, -1.0, -1.0),
            0.5,
            [4, 4, 4],
        )
        .expect("sphere SDF should build");

        let centerish = grid
            .sample_nearest(Vec3::new(0.0, 0.0, 0.0))
            .expect("center-ish point should sample");
        let outside = grid
            .sample_nearest(Vec3::new(0.75, 0.0, 0.0))
            .expect("outside point should sample");

        assert_eq!(grid.voxel_count(), 64);
        assert!(centerish.distance_meters < 0.0);
        assert!(outside.distance_meters > centerish.distance_meters);
    }

    #[test]
    fn validates_triangle_mesh_indices() {
        let mesh = TriangleMeshSnapshot::new(1, vec![Vec3::ZERO], vec![[0, 1, 0]]);

        assert_eq!(
            mesh.validate(),
            Err(SdfGridError::InvalidMeshIndex {
                triangle_index: 0,
                vertex_index: 1,
                vertex_count: 1,
            })
        );
    }

    #[test]
    fn mesh_sdf_marks_closed_mesh_inside_negative() {
        let mesh = cube_mesh_snapshot(Vec3::ZERO, Vec3::splat(0.5));
        let sdf = build_sdf_from_mesh(
            1,
            &mesh,
            MeshToSdfConfig {
                voxel_size_meters: 0.1,
                padding_meters: 0.2,
                max_voxels: 64 * 64 * 64,
                ..MeshToSdfConfig::default()
            },
        )
        .expect("cube mesh should build");

        let center = sdf
            .sample(Vec3::ZERO, SdfSampleMode::Nearest)
            .expect("center should be inside grid");
        let outside = sdf
            .sample(Vec3::new(0.62, 0.0, 0.0), SdfSampleMode::Nearest)
            .expect("outside point should be inside grid");

        assert!(center.distance_meters < 0.0);
        assert!(outside.distance_meters > 0.0);
    }

    #[test]
    fn mesh_sdf_rejects_bad_indices() {
        let mesh = TriangleMeshSnapshot::new(1, vec![Vec3::ZERO], vec![[0, 1, 2]]);
        let err = build_sdf_from_mesh(1, &mesh, MeshToSdfConfig::default())
            .expect_err("invalid indices should fail");

        assert!(matches!(err, MeshToSdfError::InvalidIndex { .. }));
    }

    #[test]
    fn sdf_sample_extrapolates_near_grid_edge() {
        let sdf = PackedSdfGrid::sphere(
            1,
            Vec3::ZERO,
            0.5,
            Vec3::new(-1.0, -1.0, -1.0),
            0.1,
            [20, 20, 20],
        )
        .expect("sphere SDF should build");

        let sample = sdf
            .sample_extrapolated(Vec3::new(1.25, 0.0, 0.0), SdfSampleMode::Trilinear, 0.5)
            .expect("nearby positions outside the grid should extrapolate");

        assert!(sample.distance_meters > 0.0);
        assert!(sample.normal.x > 0.0);
        assert!(sdf
            .sample_extrapolated(Vec3::new(2.0, 0.0, 0.0), SdfSampleMode::Trilinear, 0.2)
            .is_none());
    }

    #[test]
    fn sparse_tsdf_reports_surface_candidates() {
        let snapshot = SparseTsdfSnapshot::new(
            1,
            Vec3::ZERO,
            0.05,
            0.15,
            vec![
                SparseTsdfSample::new(VoxelCoord3::new(0, 0, 0), 0.02, 4),
                SparseTsdfSample::new(VoxelCoord3::new(1, 0, 0), 0.75, 4),
                SparseTsdfSample::new(VoxelCoord3::new(2, 0, 0), 0.01, 0),
            ],
        );

        assert!(snapshot.is_valid());
        assert_eq!(snapshot.surface_candidate_count(0.05), 1);
        assert_eq!(
            snapshot.voxel_center_world(VoxelCoord3::ZERO),
            Vec3::new(0.025, 0.025, 0.025)
        );
    }

    #[test]
    fn scan_fusion_stats_report_acceptance_ratio() {
        let stats = ScanFusionStats {
            integrated_ray_count: 8,
            rejected_ray_count: 2,
            ..ScanFusionStats::default()
        };

        assert_eq!(stats.acceptance_ratio(), Some(0.8));
        assert_eq!(ScanFusionStats::default().acceptance_ratio(), None);
    }

    fn flat_floor_tsdf_snapshot(version: u64) -> SparseTsdfSnapshot {
        let voxel_size_meters = 0.1;
        let truncation_distance_meters = 0.2;
        let mut samples = Vec::new();
        for z in -4..=4 {
            for y in -4..=4 {
                for x in -4..=4 {
                    let coord = VoxelCoord3::new(x, y, z);
                    let world_y = (y as f32 + 0.5) * voxel_size_meters;
                    samples.push(SparseTsdfSample::new(
                        coord,
                        (world_y / truncation_distance_meters).clamp(-1.0, 1.0),
                        8,
                    ));
                }
            }
        }
        SparseTsdfSnapshot::new(
            version,
            Vec3::ZERO,
            voxel_size_meters,
            truncation_distance_meters,
            samples,
        )
    }

    fn ramp_tsdf_snapshot(version: u64, slope_x: f32) -> SparseTsdfSnapshot {
        let voxel_size_meters = 0.1;
        let truncation_distance_meters = 0.25;
        let normal_scale = Vec3::new(-slope_x, 1.0, 0.0).length();
        let mut samples = Vec::new();
        for z in -4..=4 {
            for y in -4..=4 {
                for x in -4..=4 {
                    let coord = VoxelCoord3::new(x, y, z);
                    let world = Vec3::new(
                        (x as f32 + 0.5) * voxel_size_meters,
                        (y as f32 + 0.5) * voxel_size_meters,
                        (z as f32 + 0.5) * voxel_size_meters,
                    );
                    let signed_distance = (world.y - (slope_x * world.x)) / normal_scale;
                    samples.push(SparseTsdfSample::new(
                        coord,
                        (signed_distance / truncation_distance_meters).clamp(-1.0, 1.0),
                        12,
                    ));
                }
            }
        }
        SparseTsdfSnapshot::new(
            version,
            Vec3::ZERO,
            voxel_size_meters,
            truncation_distance_meters,
            samples,
        )
    }

    fn cube_mesh_snapshot(center: Vec3, half_extents: Vec3) -> TriangleMeshSnapshot {
        let min = center - half_extents;
        let max = center + half_extents;
        let vertices = vec![
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ];
        let indices = vec![
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
        TriangleMeshSnapshot::new(1, vertices, indices)
    }

    fn vertical_wall_tsdf_snapshot(version: u64) -> SparseTsdfSnapshot {
        let voxel_size_meters = 0.1;
        let truncation_distance_meters = 0.2;
        let mut samples = Vec::new();
        for z in -4..=4 {
            for y in -4..=4 {
                for x in -4..=4 {
                    let coord = VoxelCoord3::new(x, y, z);
                    let world_x = (x as f32 + 0.5) * voxel_size_meters;
                    samples.push(SparseTsdfSample::new(
                        coord,
                        (world_x / truncation_distance_meters).clamp(-1.0, 1.0),
                        10,
                    ));
                }
            }
        }
        SparseTsdfSnapshot::new(
            version,
            Vec3::ZERO,
            voxel_size_meters,
            truncation_distance_meters,
            samples,
        )
    }

    fn assert_close(left: f32, right: f32, epsilon: f32) {
        assert!(
            (left - right).abs() <= epsilon,
            "expected {left} to be within {epsilon} of {right}"
        );
    }

    #[test]
    fn tsdf_support_query_extracts_flat_floor_plane() {
        let snapshot = flat_floor_tsdf_snapshot(20);
        let request = DepthQueryRequest::new(
            77,
            Vec3::new(0.0, 0.2, 0.0),
            Vec3::new(0.0, 0.16, 0.0),
            Vec3::new(0.0, -0.2, 0.0),
            0.2,
            0.5,
        );

        let result =
            evaluate_tsdf_support_query(&snapshot, request, TsdfSupportQuerySettings::default())
                .expect("flat floor should provide a support plane");

        assert!(result.is_valid());
        assert_eq!(result.request_key, request.key);
        assert_eq!(result.snapshot_version, snapshot.version);
        assert_eq!(result.surface.role, DepthQuerySurfaceRole::Support);
        assert!(result.sample_count >= 4);
        assert!(result.surface.plane.normal.dot(Vec3::UP) > 0.99);
        assert_close(result.surface.plane.point.y, 0.0, 1.0e-5);
        assert!(result.surface.plane.supports_point(
            request.predicted_center,
            request.radius_meters,
            0.0
        ));
    }

    #[test]
    fn tsdf_support_query_tracks_ramped_surface_normal() {
        let slope_x = 0.25;
        let snapshot = ramp_tsdf_snapshot(21, slope_x);
        let expected_normal = Vec3::new(-slope_x, 1.0, 0.0).normalized_or(Vec3::UP);
        let request = DepthQueryRequest::new(
            91,
            Vec3::new(0.2, 0.25, 0.0),
            Vec3::new(0.2, (slope_x * 0.2) + 0.16, 0.0),
            Vec3::new(0.0, -0.1, 0.0),
            0.22,
            0.5,
        );

        let result = evaluate_tsdf_support_query(
            &snapshot,
            request,
            TsdfSupportQuerySettings {
                surface_band_meters: 0.09,
                min_upward_normal_dot: 0.5,
                ..TsdfSupportQuerySettings::default()
            },
        )
        .expect("ramp should provide a support plane");

        let plane = result.surface.plane;
        assert!(plane.normal.dot(expected_normal) > 0.98);
        assert_close(plane.point.y - (slope_x * plane.point.x), 0.0, 0.02);
        assert!(plane.supports_point(request.predicted_center, request.radius_meters, 0.0));
    }

    #[test]
    fn tsdf_support_query_batch_preserves_request_keys_and_skips_misses() {
        let snapshot = flat_floor_tsdf_snapshot(22);
        let valid = DepthQueryRequest::new(
            1,
            Vec3::new(0.0, 0.2, 0.0),
            Vec3::new(0.0, 0.16, 0.0),
            Vec3::new(0.0, -0.2, 0.0),
            0.2,
            0.5,
        );
        let too_high = DepthQueryRequest::new(
            2,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::ZERO,
            0.1,
            0.2,
        );

        let results = evaluate_tsdf_support_queries(
            &snapshot,
            &[valid, too_high],
            TsdfSupportQuerySettings::default(),
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].request_key, valid.key);
        assert!(results[0].surface.is_valid());
    }

    #[test]
    fn tsdf_impact_query_hits_vertical_wall_along_motion() {
        let snapshot = vertical_wall_tsdf_snapshot(23);
        let request = DepthQueryRequest::new(
            501,
            Vec3::new(-0.35, 0.0, 0.0),
            Vec3::new(0.25, 0.0, 0.0),
            Vec3::new(0.6, 0.0, 0.0),
            0.12,
            1.0,
        );

        let result =
            evaluate_tsdf_impact_query(&snapshot, request, TsdfImpactQuerySettings::default())
                .expect("swept query should hit the wall");

        assert!(result.is_valid());
        assert_eq!(result.request_key, request.key);
        assert_eq!(result.snapshot_version, snapshot.version);
        assert_eq!(result.surface.role, DepthQuerySurfaceRole::Impact);
        assert!(result.surface.plane.normal.dot(Vec3::RIGHT) < -0.99);
        assert_close(result.surface.plane.point.x, 0.0, 1.0e-5);
        assert_close(result.impact_center.x, -request.radius_meters, 1.0e-5);
        assert_close(result.signed_clearance_at_impact_meters, 0.0, 1.0e-5);
        assert!(result.travel_fraction > 0.0 && result.travel_fraction < 1.0);
        assert!(result.surface.plane.supports_point(
            result.impact_center,
            request.radius_meters + 1.0e-4,
            0.0
        ));
    }

    #[test]
    fn tsdf_impact_query_ignores_parallel_wall_motion() {
        let snapshot = vertical_wall_tsdf_snapshot(24);
        let request = DepthQueryRequest::new(
            502,
            Vec3::new(-0.25, 0.0, -0.3),
            Vec3::new(-0.25, 0.0, 0.3),
            Vec3::new(0.0, 0.0, 0.6),
            0.12,
            1.0,
        );

        let result =
            evaluate_tsdf_impact_query(&snapshot, request, TsdfImpactQuerySettings::default());

        assert_eq!(result, None);
    }

    #[test]
    fn tsdf_impact_query_batch_preserves_request_keys_and_skips_misses() {
        let snapshot = vertical_wall_tsdf_snapshot(25);
        let hit = DepthQueryRequest::new(
            601,
            Vec3::new(-0.35, 0.0, 0.0),
            Vec3::new(0.25, 0.0, 0.0),
            Vec3::new(0.6, 0.0, 0.0),
            0.12,
            1.0,
        );
        let miss = DepthQueryRequest::new(
            602,
            Vec3::new(-0.35, 0.0, 0.0),
            Vec3::new(-0.25, 0.0, 0.0),
            Vec3::new(0.1, 0.0, 0.0),
            0.05,
            1.0,
        );

        let results = evaluate_tsdf_impact_queries(
            &snapshot,
            &[hit, miss],
            TsdfImpactQuerySettings::default(),
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].request_key, hit.key);
        assert_eq!(results[0].surface.role, DepthQuerySurfaceRole::Impact);
    }

    #[test]
    fn tsdf_query_surface_frame_driver_upserts_support_planes() {
        let snapshot = flat_floor_tsdf_snapshot(26);
        let request = DepthQueryRequest::new(
            701,
            Vec3::new(0.0, 0.2, 0.0),
            Vec3::new(0.0, 0.16, 0.0),
            Vec3::new(0.0, -0.2, 0.0),
            0.2,
            0.5,
        );
        let settings = TsdfQuerySurfaceFrameSettings {
            enable_impact_queries: false,
            ..TsdfQuerySurfaceFrameSettings::default()
        };
        let mut driver = TsdfQuerySurfaceFrameDriver::new();

        let output = driver.advance_frame(0, Some(&snapshot), &[request], settings);

        let key = DepthQuerySurfaceKey::new(request.key, DepthQuerySurfaceRole::Support);
        assert_eq!(output.status, TsdfQuerySurfaceFrameStatus::Updated);
        assert_eq!(output.update.support_request_count, 1);
        assert_eq!(output.update.support_hit_count, 1);
        assert_eq!(output.update.impact_request_count, 0);
        assert_eq!(output.update.upserts.len(), 1);
        assert_eq!(output.update.upserts[0].key, key);
        assert_eq!(output.update.visible_keys, vec![key]);
        assert_eq!(driver.cached_surface_count(), 1);
        assert!(driver.cache().contains_key(key));
        assert_eq!(driver.last_snapshot_version(), Some(snapshot.version));
    }

    #[test]
    fn tsdf_query_surface_frame_driver_upserts_impact_planes() {
        let snapshot = vertical_wall_tsdf_snapshot(27);
        let request = DepthQueryRequest::new(
            702,
            Vec3::new(-0.35, 0.0, 0.0),
            Vec3::new(0.25, 0.0, 0.0),
            Vec3::new(0.6, 0.0, 0.0),
            0.12,
            1.0,
        );
        let settings = TsdfQuerySurfaceFrameSettings {
            enable_support_queries: false,
            ..TsdfQuerySurfaceFrameSettings::default()
        };
        let mut driver = TsdfQuerySurfaceFrameDriver::new();

        let output = driver.advance_frame(0, Some(&snapshot), &[request], settings);

        let key = DepthQuerySurfaceKey::new(request.key, DepthQuerySurfaceRole::Impact);
        assert_eq!(output.status, TsdfQuerySurfaceFrameStatus::Updated);
        assert_eq!(output.update.support_request_count, 0);
        assert_eq!(output.update.impact_request_count, 1);
        assert_eq!(output.update.impact_hit_count, 1);
        assert_eq!(output.update.upserts.len(), 1);
        assert_eq!(output.update.upserts[0].key, key);
        assert_eq!(
            output.update.upserts[0].surface.role,
            DepthQuerySurfaceRole::Impact
        );
        assert!(driver.cache().contains_key(key));
    }

    #[test]
    fn tsdf_query_surface_frame_driver_reuses_unchanged_surfaces() {
        let snapshot = flat_floor_tsdf_snapshot(28);
        let request = DepthQueryRequest::new(
            703,
            Vec3::new(0.0, 0.2, 0.0),
            Vec3::new(0.0, 0.16, 0.0),
            Vec3::new(0.0, -0.2, 0.0),
            0.2,
            0.5,
        );
        let settings = TsdfQuerySurfaceFrameSettings {
            enable_impact_queries: false,
            ..TsdfQuerySurfaceFrameSettings::default()
        };
        let mut driver = TsdfQuerySurfaceFrameDriver::new();
        let first = driver.advance_frame(0, Some(&snapshot), &[request], settings);
        let second = driver.advance_frame(1, Some(&snapshot), &[request], settings);

        let key = DepthQuerySurfaceKey::new(request.key, DepthQuerySurfaceRole::Support);
        assert_eq!(first.update.upserts.len(), 1);
        assert!(second.is_physics_noop());
        assert_eq!(second.update.retained_keys, vec![key]);
        assert_eq!(second.update.reused_surface_count, 1);
        assert_eq!(driver.total_upsert_count(), 1);
        assert_eq!(driver.total_removal_count(), 0);
        assert_eq!(
            driver
                .cache()
                .get(key)
                .map(|entry| entry.last_seen_frame_index),
            Some(1)
        );
    }

    #[test]
    fn tsdf_query_surface_frame_driver_retains_short_misses_then_removes() {
        let snapshot = flat_floor_tsdf_snapshot(29);
        let hit = DepthQueryRequest::new(
            704,
            Vec3::new(0.0, 0.2, 0.0),
            Vec3::new(0.0, 0.16, 0.0),
            Vec3::new(0.0, -0.2, 0.0),
            0.2,
            0.5,
        );
        let miss = DepthQueryRequest::new(
            hit.key,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::ZERO,
            0.1,
            0.1,
        );
        let settings = TsdfQuerySurfaceFrameSettings {
            enable_impact_queries: false,
            miss_retention_frames: 1,
            ..TsdfQuerySurfaceFrameSettings::default()
        };
        let mut driver = TsdfQuerySurfaceFrameDriver::new();
        driver.advance_frame(0, Some(&snapshot), &[hit], settings);

        let retained = driver.advance_frame(1, Some(&snapshot), &[miss], settings);
        let expired = driver.advance_frame(2, Some(&snapshot), &[miss], settings);

        let key = DepthQuerySurfaceKey::new(hit.key, DepthQuerySurfaceRole::Support);
        assert!(retained.is_physics_noop());
        assert_eq!(retained.update.retained_keys, vec![key]);
        assert_eq!(retained.cached_surface_count, 1);
        assert_eq!(expired.update.removals, vec![key]);
        assert_eq!(expired.cached_surface_count, 0);
        assert_eq!(driver.total_removal_count(), 1);
    }

    #[test]
    fn tsdf_query_surface_frame_driver_can_clear_when_snapshot_is_unavailable() {
        let snapshot = flat_floor_tsdf_snapshot(30);
        let request = DepthQueryRequest::new(
            705,
            Vec3::new(0.0, 0.2, 0.0),
            Vec3::new(0.0, 0.16, 0.0),
            Vec3::new(0.0, -0.2, 0.0),
            0.2,
            0.5,
        );
        let settings = TsdfQuerySurfaceFrameSettings {
            enable_impact_queries: false,
            clear_when_snapshot_unavailable: true,
            ..TsdfQuerySurfaceFrameSettings::default()
        };
        let mut driver = TsdfQuerySurfaceFrameDriver::new();
        driver.advance_frame(0, Some(&snapshot), &[request], settings);
        let cached_count = driver.cached_surface_count();

        let waiting = driver.advance_frame(1, None, &[request], settings);

        assert_eq!(cached_count, 1);
        assert_eq!(
            waiting.status,
            TsdfQuerySurfaceFrameStatus::WaitingForSnapshot
        );
        assert_eq!(waiting.update.removals.len(), cached_count);
        assert_eq!(waiting.cached_surface_count, 0);
        assert_eq!(waiting.last_snapshot_version, Some(snapshot.version));
    }

    #[test]
    fn tsdf_query_surface_frame_driver_disables_queries_and_removes_surfaces() {
        let snapshot = flat_floor_tsdf_snapshot(31);
        let request = DepthQueryRequest::new(
            706,
            Vec3::new(0.0, 0.2, 0.0),
            Vec3::new(0.0, 0.16, 0.0),
            Vec3::new(0.0, -0.2, 0.0),
            0.2,
            0.5,
        );
        let mut driver = TsdfQuerySurfaceFrameDriver::new();
        driver.advance_frame(
            0,
            Some(&snapshot),
            &[request],
            TsdfQuerySurfaceFrameSettings {
                enable_impact_queries: false,
                ..TsdfQuerySurfaceFrameSettings::default()
            },
        );

        let disabled = driver.advance_frame(
            1,
            Some(&snapshot),
            &[request],
            TsdfQuerySurfaceFrameSettings {
                enable_support_queries: false,
                enable_impact_queries: false,
                ..TsdfQuerySurfaceFrameSettings::default()
            },
        );

        assert_eq!(disabled.status, TsdfQuerySurfaceFrameStatus::Disabled);
        assert_eq!(disabled.update.removals.len(), 1);
        assert_eq!(disabled.cached_surface_count, 0);
        assert_eq!(driver.cached_surface_count(), 0);
    }

    #[test]
    fn focused_tsdf_mesh_planning_returns_stable_chunks() {
        let snapshot = flat_floor_tsdf_snapshot(42);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 8,
        };

        let plans = plan_focused_tsdf_mesh_chunks(&snapshot, Vec3::ZERO, 0.7, settings);

        assert!(!plans.is_empty());
        assert!(plans.len() <= settings.max_chunk_count);
        assert!(plans.iter().all(TsdfMeshChunkPlan::is_valid));
        assert!(plans
            .iter()
            .any(|plan| plan.key == TsdfMeshChunkKey::new(0, 0, 0)));
    }

    #[test]
    fn focused_tsdf_mesh_chunks_extract_surface_net_geometry() {
        let snapshot = flat_floor_tsdf_snapshot(7);
        let chunks = extract_focused_tsdf_mesh_chunks(
            &snapshot,
            Vec3::ZERO,
            0.7,
            TsdfMeshChunkSettings {
                chunk_edge_voxels: 4,
                overlap_voxels: 1,
                stride_voxels: 1,
                max_chunk_count: 8,
            },
        );

        let chunk = chunks
            .iter()
            .find(|chunk| chunk.key == TsdfMeshChunkKey::new(0, 0, 0))
            .expect("focused floor should produce the central chunk");

        assert!(chunk.is_valid());
        assert!(chunk.vertices.iter().all(|vertex| vertex.y.abs() <= 1.0e-5));
        assert!(chunk.indices.len() >= 2);
    }

    #[test]
    fn tsdf_mesh_chunk_fingerprint_tracks_region_changes() {
        let snapshot = flat_floor_tsdf_snapshot(9);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 8,
        };
        let baseline = plan_focused_tsdf_mesh_chunks(&snapshot, Vec3::ZERO, 0.7, settings);
        let baseline_central = baseline
            .iter()
            .find(|plan| plan.key == TsdfMeshChunkKey::new(0, 0, 0))
            .expect("baseline central chunk should be planned");

        let mut changed = snapshot.clone();
        let sample = changed
            .samples
            .iter_mut()
            .find(|sample| sample.coord == VoxelCoord3::new(0, 0, 0))
            .expect("fixture should include central sample");
        sample.normalized_distance = 0.5;
        let changed_plans = plan_focused_tsdf_mesh_chunks(&changed, Vec3::ZERO, 0.7, settings);
        let changed_central = changed_plans
            .iter()
            .find(|plan| plan.key == TsdfMeshChunkKey::new(0, 0, 0))
            .expect("changed central chunk should be planned");

        assert_ne!(baseline_central.fingerprint, changed_central.fingerprint);
    }

    #[test]
    fn tsdf_mesh_chunk_fingerprint_tracks_origin_changes() {
        let snapshot = flat_floor_tsdf_snapshot(9);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 8,
        };
        let baseline = plan_focused_tsdf_mesh_chunks(&snapshot, Vec3::ZERO, 0.7, settings);
        let baseline_central = baseline
            .iter()
            .find(|plan| plan.key == TsdfMeshChunkKey::new(0, 0, 0))
            .expect("baseline central chunk should be planned");

        let mut moved = snapshot.clone();
        moved.origin = Vec3::new(0.01, 0.0, 0.0);
        let moved_plans = plan_focused_tsdf_mesh_chunks(&moved, Vec3::ZERO, 0.7, settings);
        let moved_central = moved_plans
            .iter()
            .find(|plan| plan.key == TsdfMeshChunkKey::new(0, 0, 0))
            .expect("moved central chunk should be planned");

        assert_ne!(baseline_central.fingerprint, moved_central.fingerprint);
    }

    #[test]
    fn tsdf_mesh_chunk_cache_reuses_unchanged_chunks() {
        let snapshot = flat_floor_tsdf_snapshot(11);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 8,
        };
        let mut cache = TsdfMeshChunkCache::new();

        let first = cache.update_focused(&snapshot, Vec3::ZERO, 0.7, settings);
        let cached_count = cache.len();
        let second = cache.update_focused(&snapshot, Vec3::ZERO, 0.7, settings);

        assert!(!first.upserts.is_empty());
        assert_eq!(first.upserts.len(), cached_count);
        assert!(first.removals.is_empty());
        assert!(second.upserts.is_empty());
        assert!(second.removals.is_empty());
        assert_eq!(second.retained_keys.len(), cached_count);
        assert_eq!(second.reused_chunk_count, cached_count);
        assert!(second.is_empty());
    }

    #[test]
    fn tsdf_mesh_chunk_cache_upserts_changed_chunks() {
        let snapshot = flat_floor_tsdf_snapshot(12);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 8,
        };
        let mut cache = TsdfMeshChunkCache::new();
        cache.update_focused(&snapshot, Vec3::ZERO, 0.7, settings);
        let baseline_fingerprint = cache
            .get(TsdfMeshChunkKey::new(0, 0, 0))
            .expect("central chunk should be cached")
            .fingerprint;

        let mut changed = snapshot.clone();
        let sample = changed
            .samples
            .iter_mut()
            .find(|sample| sample.coord == VoxelCoord3::new(0, 0, 0))
            .expect("fixture should include central sample");
        sample.normalized_distance = 0.5;
        let update = cache.update_focused(&changed, Vec3::ZERO, 0.7, settings);
        let changed_fingerprint = cache
            .get(TsdfMeshChunkKey::new(0, 0, 0))
            .expect("central chunk should stay cached")
            .fingerprint;

        assert!(update
            .upserts
            .iter()
            .any(|chunk| chunk.key == TsdfMeshChunkKey::new(0, 0, 0)));
        assert_ne!(baseline_fingerprint, changed_fingerprint);
    }

    #[test]
    fn tsdf_mesh_chunk_cache_removes_out_of_focus_chunks() {
        let snapshot = flat_floor_tsdf_snapshot(13);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 8,
        };
        let mut cache = TsdfMeshChunkCache::new();
        cache.update_focused(&snapshot, Vec3::ZERO, 0.7, settings);
        let cached_count = cache.len();

        let update = cache.update_focused(&snapshot, Vec3::ZERO, 0.0, settings);

        assert!(cached_count > 0);
        assert!(cache.is_empty());
        assert_eq!(update.removals.len(), cached_count);
        assert_eq!(update.changed_chunk_count(), cached_count);
    }

    #[test]
    fn tsdf_mesh_chunk_frame_driver_moves_focus_and_emits_chunk_changes() {
        let snapshot = flat_floor_tsdf_snapshot(14);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 1,
        };
        let mut driver = TsdfMeshChunkFrameDriver::new();

        let first = driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(0, Vec3::new(-0.35, 0.0, 0.0), 0.2, settings),
            Some(&snapshot),
        );
        let second = driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(1, Vec3::new(0.35, 0.0, 0.0), 0.2, settings),
            Some(&snapshot),
        );

        assert_eq!(first.status, TsdfMeshChunkFrameStatus::Updated);
        assert_eq!(second.status, TsdfMeshChunkFrameStatus::Updated);
        assert_eq!(first.update.upserts.len(), 1);
        assert_eq!(second.update.upserts.len(), 1);
        assert_eq!(second.update.removals.len(), 1);
        assert_ne!(first.update.visible_keys, second.update.visible_keys);
        assert_eq!(driver.cached_chunk_count(), 1);
        assert_eq!(driver.last_frame_index(), Some(1));
        assert_eq!(driver.last_snapshot_version(), Some(snapshot.version));
        assert_eq!(driver.total_upsert_count(), 2);
        assert_eq!(driver.total_removal_count(), 1);
    }

    #[test]
    fn tsdf_mesh_chunk_frame_driver_keeps_mesh_while_waiting_for_snapshot() {
        let snapshot = flat_floor_tsdf_snapshot(15);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 4,
        };
        let mut driver = TsdfMeshChunkFrameDriver::new();
        let first = driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(0, Vec3::ZERO, 0.5, settings),
            Some(&snapshot),
        );
        let cached_count = driver.cached_chunk_count();

        let waiting = driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(1, Vec3::new(0.1, 0.0, 0.0), 0.5, settings),
            None,
        );

        assert!(!first.update.upserts.is_empty());
        assert_eq!(waiting.status, TsdfMeshChunkFrameStatus::WaitingForSnapshot);
        assert!(waiting.is_render_noop());
        assert_eq!(waiting.cached_chunk_count, cached_count);
        assert_eq!(driver.cached_chunk_count(), cached_count);
        assert_eq!(waiting.last_snapshot_version, Some(snapshot.version));
    }

    #[test]
    fn tsdf_mesh_chunk_frame_driver_can_clear_when_snapshot_is_unavailable() {
        let snapshot = flat_floor_tsdf_snapshot(16);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 4,
        };
        let mut driver = TsdfMeshChunkFrameDriver::new();
        driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(0, Vec3::ZERO, 0.5, settings),
            Some(&snapshot),
        );
        let cached_count = driver.cached_chunk_count();

        let cleared = driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(1, Vec3::ZERO, 0.5, settings)
                .with_clear_when_snapshot_unavailable(true),
            None,
        );

        assert!(cached_count > 0);
        assert_eq!(cleared.status, TsdfMeshChunkFrameStatus::WaitingForSnapshot);
        assert_eq!(cleared.update.removals.len(), cached_count);
        assert_eq!(cleared.cached_chunk_count, 0);
        assert_eq!(driver.cached_chunk_count(), 0);
    }

    #[test]
    fn tsdf_mesh_chunk_frame_driver_disables_mesh_without_snapshot() {
        let snapshot = flat_floor_tsdf_snapshot(17);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 4,
        };
        let mut driver = TsdfMeshChunkFrameDriver::new();
        driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(0, Vec3::ZERO, 0.5, settings),
            Some(&snapshot),
        );
        let cached_count = driver.cached_chunk_count();

        let disabled = driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(1, Vec3::ZERO, 0.0, settings),
            None,
        );

        assert!(cached_count > 0);
        assert_eq!(disabled.status, TsdfMeshChunkFrameStatus::Disabled);
        assert_eq!(disabled.update.removals.len(), cached_count);
        assert_eq!(disabled.cached_chunk_count, 0);
    }

    #[test]
    fn tsdf_mesh_chunk_frame_driver_rejects_invalid_focus_without_clearing() {
        let snapshot = flat_floor_tsdf_snapshot(18);
        let settings = TsdfMeshChunkSettings {
            chunk_edge_voxels: 4,
            overlap_voxels: 1,
            stride_voxels: 1,
            max_chunk_count: 4,
        };
        let mut driver = TsdfMeshChunkFrameDriver::new();
        driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(0, Vec3::ZERO, 0.5, settings),
            Some(&snapshot),
        );
        let cached_count = driver.cached_chunk_count();

        let invalid = driver.advance_frame(
            TsdfMeshChunkFrameRequest::new(1, Vec3::new(f32::NAN, 0.0, 0.0), 0.5, settings),
            Some(&snapshot),
        );

        assert_eq!(invalid.status, TsdfMeshChunkFrameStatus::InvalidRequest);
        assert!(invalid.is_render_noop());
        assert_eq!(driver.cached_chunk_count(), cached_count);
    }

    #[test]
    fn depth_support_plane_reports_quad_and_support() {
        let plane = DepthSupportPlane::new(
            Vec3::ZERO,
            Vec3::UP,
            Vec3::RIGHT,
            Vec3::FORWARD_NEG_Z,
            0.5,
            0.25,
        );
        let quad = plane.quad_vertices();

        assert!(plane.is_valid());
        assert_eq!(quad[0], Vec3::new(-0.5, 0.0, 0.25));
        assert!(plane.supports_point(Vec3::new(0.1, 0.02, -0.1), 0.05, 0.0));
        assert!(!plane.supports_point(Vec3::new(0.8, 0.02, -0.1), 0.05, 0.0));
    }

    #[test]
    fn depth_query_request_classifies_impact_refresh_need() {
        let slow = DepthQueryRequest::new(
            1,
            Vec3::ZERO,
            Vec3::new(0.01, 0.0, 0.0),
            Vec3::new(0.05, 0.0, 0.0),
            0.05,
            1.0,
        );
        let fast = DepthQueryRequest::new(
            1,
            Vec3::ZERO,
            Vec3::new(0.2, 0.0, 0.0),
            Vec3::new(0.7, 0.0, 0.0),
            0.05,
            1.0,
        );

        assert!(!slow.might_need_impact_refresh(0.55, 0.4, 0.55, 0.03));
        assert!(fast.might_need_impact_refresh(0.55, 0.4, 0.55, 0.03));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn sparse_tsdf_snapshot_round_trips_with_serde() {
        let snapshot = SparseTsdfSnapshot::new(
            1,
            Vec3::ZERO,
            0.05,
            0.15,
            vec![SparseTsdfSample::new(VoxelCoord3::new(0, 0, 0), 0.02, 4)],
        );

        let encoded = serde_json::to_string(&snapshot).expect("snapshot should serialize");
        let decoded: SparseTsdfSnapshot =
            serde_json::from_str(&encoded).expect("snapshot should deserialize");

        assert_eq!(decoded, snapshot);
    }
}
