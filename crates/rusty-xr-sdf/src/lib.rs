//! Signed-distance-field contracts and utilities for Rusty XR.
//!
//! This crate owns public SDF, sparse TSDF, scan-surface, and mesh snapshot
//! contracts. Native depth acquisition, meshing workers, physics backends, and
//! captured room datasets stay in adapters or downstream repos.
//!
//! Enable the `serde` feature to serialize public scan and SDF snapshots.

use core::fmt;

pub use rusty_xr_contracts::Vec3;

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

        let c000 = self.sample_at(x0, y0, z0)?;
        let c100 = self.sample_at(x0 + 1, y0, z0)?;
        let c010 = self.sample_at(x0, y0 + 1, z0)?;
        let c110 = self.sample_at(x0 + 1, y0 + 1, z0)?;
        let c001 = self.sample_at(x0, y0, z0 + 1)?;
        let c101 = self.sample_at(x0 + 1, y0, z0 + 1)?;
        let c011 = self.sample_at(x0, y0 + 1, z0 + 1)?;
        let c111 = self.sample_at(x0 + 1, y0 + 1, z0 + 1)?;

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
