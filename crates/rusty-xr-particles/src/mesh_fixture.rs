use super::{
    build_fixture_hand_mesh, generate_icosphere, rotate_positions_x, IcosphereTopology,
    IcosphereTopologyConfig, MeshSurfaceSampleConfig, MeshSurfaceSampleSet, MeshSurfaceTopologyKey,
    TriangleMeshSurface, Vec3, MAX_ICOSPHERE_RECURSION_LEVEL,
};

pub const MESH_FIXTURE_MANIFEST_SCHEMA: &str = "rusty.xr.mesh_fixture_manifest.v1";

/// Public-safe category for a mesh fixture.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureKind {
    HandMesh,
    SyntheticSurface,
    Icosphere,
    DeformingMesh,
    Grid,
    Other,
}

/// Coordinate frame used by fixture vertices and sampled coordinates.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureCoordinateSpace {
    Local,
    Stage,
    World,
    UnitSphere,
}

/// Metric convention used by fixture coordinates.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureUnits {
    Meters,
    UnitRadius,
    Unitless,
}

/// Axis convention used by fixture coordinates.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureCoordinateConvention {
    RightHandedYUpNegativeZForward,
    UnitSphere,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureWindingOrder {
    Clockwise,
    CounterClockwise,
    MixedOrUnspecified,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureIndexFormat {
    U16,
    U32,
    Usize,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureMotionKind {
    Static,
    Animated,
    Deforming,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureValidationExpectation {
    CountsMatchTopology,
    IndicesInRange,
    FiniteCoordinates,
    NonDegenerateSurface,
    StableTopologyHash,
    NeighborTiersMatchSampleCount,
    DeformationFrameRange,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureIntendedUse {
    TopologyTests,
    SamplingTests,
    SdfDepthTests,
    ParticleTests,
    RenderPayloadTests,
    ColliderTests,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureProvenance {
    Synthetic,
    Public,
    Example,
    Generated,
}

/// Min/max neighbor counts expected for one same-surface tier.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshFixtureNeighborTier {
    pub tier: u8,
    pub min_neighbor_count: usize,
    pub max_neighbor_count: usize,
}

impl MeshFixtureNeighborTier {
    pub const fn exact(tier: u8, neighbor_count: usize) -> Self {
        Self {
            tier,
            min_neighbor_count: neighbor_count,
            max_neighbor_count: neighbor_count,
        }
    }
}

/// Inclusive frame-count range accepted for static or deformed fixture playback.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshFixtureFrameRange {
    pub min_frame_count: usize,
    pub max_frame_count: usize,
}

impl MeshFixtureFrameRange {
    pub const fn new(min_frame_count: usize, max_frame_count: usize) -> Self {
        Self {
            min_frame_count,
            max_frame_count,
        }
    }

    pub const fn static_once() -> Self {
        Self::new(1, 1)
    }

    pub const fn contains(self, frame_count: usize) -> bool {
        frame_count >= self.min_frame_count && frame_count <= self.max_frame_count
    }
}

/// Portable manifest for a public synthetic mesh/topology fixture.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MeshFixtureManifest {
    pub schema: String,
    pub fixture_id: String,
    pub fixture_kind: MeshFixtureKind,
    pub topology_key: MeshSurfaceTopologyKey,
    pub topology_hash: u64,
    pub vertex_count: usize,
    pub index_count: usize,
    pub coordinate_sample_count: usize,
    pub coordinate_space: MeshFixtureCoordinateSpace,
    pub coordinate_units: MeshFixtureUnits,
    pub coordinate_convention: MeshFixtureCoordinateConvention,
    pub winding_order: MeshFixtureWindingOrder,
    pub index_format: MeshFixtureIndexFormat,
    pub expected_neighbor_tiers: Vec<MeshFixtureNeighborTier>,
    pub motion: MeshFixtureMotionKind,
    pub allowed_deformation_frames: MeshFixtureFrameRange,
    pub validation_expectations: Vec<MeshFixtureValidationExpectation>,
    pub intended_uses: Vec<MeshFixtureIntendedUse>,
    pub provenance: MeshFixtureProvenance,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureCountField {
    VertexCount,
    IndexCount,
    CoordinateSampleCount,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFixtureManifestError {
    SchemaMismatch,
    EmptyFixtureId,
    EmptyTopology,
    IndexCountOverflow,
    CountMismatch {
        field: MeshFixtureCountField,
        manifest: usize,
        observed: usize,
    },
    TopologyHashMismatch {
        manifest: u64,
        observed: u64,
    },
    EmptyNeighborTiers,
    InvalidNeighborTier {
        tier: u8,
    },
    DuplicateNeighborTier {
        tier: u8,
    },
    InvalidFrameRange,
    StaticFixtureFrameRange,
    FrameCountOutOfRange {
        frame_count: usize,
        min_frame_count: usize,
        max_frame_count: usize,
    },
    EmptyValidationExpectations,
    EmptyIntendedUses,
}

impl MeshFixtureManifest {
    pub fn from_surface(
        fixture_id: impl Into<String>,
        fixture_kind: MeshFixtureKind,
        surface: &TriangleMeshSurface,
        coordinate_sample_count: usize,
    ) -> Self {
        let topology_key = MeshSurfaceTopologyKey::from_mesh(surface);
        Self {
            schema: MESH_FIXTURE_MANIFEST_SCHEMA.to_string(),
            fixture_id: fixture_id.into(),
            fixture_kind,
            topology_key,
            topology_hash: topology_key.index_hash,
            vertex_count: surface.vertex_count(),
            index_count: surface.triangle_count().saturating_mul(3),
            coordinate_sample_count,
            coordinate_space: MeshFixtureCoordinateSpace::Local,
            coordinate_units: MeshFixtureUnits::Meters,
            coordinate_convention: MeshFixtureCoordinateConvention::RightHandedYUpNegativeZForward,
            winding_order: MeshFixtureWindingOrder::CounterClockwise,
            index_format: MeshFixtureIndexFormat::U32,
            expected_neighbor_tiers: Vec::new(),
            motion: MeshFixtureMotionKind::Static,
            allowed_deformation_frames: MeshFixtureFrameRange::static_once(),
            validation_expectations: vec![
                MeshFixtureValidationExpectation::CountsMatchTopology,
                MeshFixtureValidationExpectation::IndicesInRange,
                MeshFixtureValidationExpectation::FiniteCoordinates,
                MeshFixtureValidationExpectation::NonDegenerateSurface,
                MeshFixtureValidationExpectation::StableTopologyHash,
            ],
            intended_uses: Vec::new(),
            provenance: MeshFixtureProvenance::Synthetic,
        }
    }

    pub fn validate(&self) -> Result<(), MeshFixtureManifestError> {
        if self.schema != MESH_FIXTURE_MANIFEST_SCHEMA {
            return Err(MeshFixtureManifestError::SchemaMismatch);
        }
        if self.fixture_id.trim().is_empty() {
            return Err(MeshFixtureManifestError::EmptyFixtureId);
        }
        if self.vertex_count == 0
            || self.index_count == 0
            || self.topology_key.vertex_count == 0
            || self.topology_key.triangle_count == 0
        {
            return Err(MeshFixtureManifestError::EmptyTopology);
        }
        if self.topology_key.vertex_count != self.vertex_count {
            return Err(MeshFixtureManifestError::CountMismatch {
                field: MeshFixtureCountField::VertexCount,
                manifest: self.vertex_count,
                observed: self.topology_key.vertex_count,
            });
        }
        let topology_index_count = self
            .topology_key
            .triangle_count
            .checked_mul(3)
            .ok_or(MeshFixtureManifestError::IndexCountOverflow)?;
        if topology_index_count != self.index_count {
            return Err(MeshFixtureManifestError::CountMismatch {
                field: MeshFixtureCountField::IndexCount,
                manifest: self.index_count,
                observed: topology_index_count,
            });
        }
        if self.coordinate_sample_count == 0 {
            return Err(MeshFixtureManifestError::CountMismatch {
                field: MeshFixtureCountField::CoordinateSampleCount,
                manifest: self.coordinate_sample_count,
                observed: 1,
            });
        }
        if self.topology_hash != self.topology_key.index_hash {
            return Err(MeshFixtureManifestError::TopologyHashMismatch {
                manifest: self.topology_hash,
                observed: self.topology_key.index_hash,
            });
        }
        self.validate_neighbor_tiers()?;
        self.validate_frame_policy()?;
        if self.validation_expectations.is_empty() {
            return Err(MeshFixtureManifestError::EmptyValidationExpectations);
        }
        if self.intended_uses.is_empty() {
            return Err(MeshFixtureManifestError::EmptyIntendedUses);
        }
        Ok(())
    }

    pub fn validate_for_surface(
        &self,
        surface: &TriangleMeshSurface,
        coordinate_sample_count: usize,
    ) -> Result<(), MeshFixtureManifestError> {
        self.validate()?;
        let observed_key = MeshSurfaceTopologyKey::from_mesh(surface);
        if observed_key.vertex_count != self.vertex_count {
            return Err(MeshFixtureManifestError::CountMismatch {
                field: MeshFixtureCountField::VertexCount,
                manifest: self.vertex_count,
                observed: observed_key.vertex_count,
            });
        }
        let observed_index_count = observed_key
            .triangle_count
            .checked_mul(3)
            .ok_or(MeshFixtureManifestError::IndexCountOverflow)?;
        if observed_index_count != self.index_count {
            return Err(MeshFixtureManifestError::CountMismatch {
                field: MeshFixtureCountField::IndexCount,
                manifest: self.index_count,
                observed: observed_index_count,
            });
        }
        if coordinate_sample_count != self.coordinate_sample_count {
            return Err(MeshFixtureManifestError::CountMismatch {
                field: MeshFixtureCountField::CoordinateSampleCount,
                manifest: self.coordinate_sample_count,
                observed: coordinate_sample_count,
            });
        }
        if observed_key.index_hash != self.topology_hash {
            return Err(MeshFixtureManifestError::TopologyHashMismatch {
                manifest: self.topology_hash,
                observed: observed_key.index_hash,
            });
        }
        Ok(())
    }

    pub fn validate_deformation_frame_count(
        &self,
        frame_count: usize,
    ) -> Result<(), MeshFixtureManifestError> {
        self.validate_frame_policy()?;
        if self.allowed_deformation_frames.contains(frame_count) {
            Ok(())
        } else {
            Err(MeshFixtureManifestError::FrameCountOutOfRange {
                frame_count,
                min_frame_count: self.allowed_deformation_frames.min_frame_count,
                max_frame_count: self.allowed_deformation_frames.max_frame_count,
            })
        }
    }

    fn validate_neighbor_tiers(&self) -> Result<(), MeshFixtureManifestError> {
        if self.expected_neighbor_tiers.is_empty() {
            return Err(MeshFixtureManifestError::EmptyNeighborTiers);
        }
        let mut seen = Vec::<u8>::with_capacity(self.expected_neighbor_tiers.len());
        for tier in &self.expected_neighbor_tiers {
            if tier.tier == 0
                || tier.min_neighbor_count > tier.max_neighbor_count
                || tier.max_neighbor_count >= self.coordinate_sample_count
            {
                return Err(MeshFixtureManifestError::InvalidNeighborTier { tier: tier.tier });
            }
            if seen.contains(&tier.tier) {
                return Err(MeshFixtureManifestError::DuplicateNeighborTier { tier: tier.tier });
            }
            seen.push(tier.tier);
        }
        Ok(())
    }

    fn validate_frame_policy(&self) -> Result<(), MeshFixtureManifestError> {
        if self.allowed_deformation_frames.min_frame_count == 0
            || self.allowed_deformation_frames.max_frame_count
                < self.allowed_deformation_frames.min_frame_count
        {
            return Err(MeshFixtureManifestError::InvalidFrameRange);
        }
        if self.motion == MeshFixtureMotionKind::Static
            && self.allowed_deformation_frames != MeshFixtureFrameRange::static_once()
        {
            return Err(MeshFixtureManifestError::StaticFixtureFrameRange);
        }
        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixtureGridMeshConfig {
    pub cells_x: usize,
    pub cells_y: usize,
    pub spacing_meters: f32,
}

impl Default for FixtureGridMeshConfig {
    fn default() -> Self {
        Self {
            cells_x: 2,
            cells_y: 2,
            spacing_meters: 0.05,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixtureIcosphereMeshConfig {
    pub recursion_level: u32,
    pub radius_meters: f32,
    pub x_axis_rotation_degrees: f32,
}

impl Default for FixtureIcosphereMeshConfig {
    fn default() -> Self {
        Self {
            recursion_level: 1,
            radius_meters: 0.1,
            x_axis_rotation_degrees: 0.0,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixtureDeformingGridConfig {
    pub grid: FixtureGridMeshConfig,
    pub frame_count: usize,
    pub amplitude_meters: f32,
}

impl Default for FixtureDeformingGridConfig {
    fn default() -> Self {
        Self {
            grid: FixtureGridMeshConfig {
                cells_x: 3,
                cells_y: 3,
                spacing_meters: 0.04,
            },
            frame_count: 4,
            amplitude_meters: 0.006,
        }
    }
}

pub fn build_fixture_grid_mesh(config: FixtureGridMeshConfig) -> TriangleMeshSurface {
    let cells_x = config.cells_x.max(1);
    let cells_y = config.cells_y.max(1);
    let spacing = if config.spacing_meters.is_finite() {
        config.spacing_meters.max(0.000_1)
    } else {
        FixtureGridMeshConfig::default().spacing_meters
    };
    let side_x = cells_x + 1;
    let side_y = cells_y + 1;
    let half_width = cells_x as f32 * spacing * 0.5;
    let half_height = cells_y as f32 * spacing * 0.5;
    let mut vertices = Vec::with_capacity(side_x * side_y);
    for y in 0..side_y {
        for x in 0..side_x {
            vertices.push(Vec3::new(
                x as f32 * spacing - half_width,
                y as f32 * spacing - half_height,
                0.0,
            ));
        }
    }

    let mut triangles = Vec::with_capacity(cells_x * cells_y * 2);
    for y in 0..cells_y {
        for x in 0..cells_x {
            let a = y * side_x + x;
            let b = a + 1;
            let c = a + side_x;
            let d = c + 1;
            triangles.push([a, b, d]);
            triangles.push([a, d, c]);
        }
    }

    TriangleMeshSurface::new(vertices, triangles)
}

pub fn build_fixture_icosphere_mesh(config: FixtureIcosphereMeshConfig) -> TriangleMeshSurface {
    let level = config.recursion_level.min(MAX_ICOSPHERE_RECURSION_LEVEL);
    let (mut vertices, triangles) = generate_icosphere(level);
    rotate_positions_x(&mut vertices, config.x_axis_rotation_degrees);
    let radius = if config.radius_meters.is_finite() {
        config.radius_meters.max(0.000_1)
    } else {
        FixtureIcosphereMeshConfig::default().radius_meters
    };
    for vertex in &mut vertices {
        *vertex = *vertex * radius;
    }
    TriangleMeshSurface::new(vertices, triangles)
}

pub fn build_fixture_deforming_grid_frames(
    config: FixtureDeformingGridConfig,
) -> Vec<TriangleMeshSurface> {
    let frame_count = config.frame_count.max(1);
    let amplitude = if config.amplitude_meters.is_finite() {
        config.amplitude_meters
    } else {
        0.0
    };
    let mut frames = Vec::with_capacity(frame_count);
    for frame in 0..frame_count {
        let phase = if frame_count == 1 {
            0.0
        } else {
            frame as f32 / (frame_count - 1) as f32
        };
        let mut mesh = build_fixture_grid_mesh(config.grid);
        for vertex in &mut mesh.vertices {
            let wave = ((vertex.x * 31.0) + (phase * core::f32::consts::TAU)).sin()
                * ((vertex.y * 17.0) + (phase * core::f32::consts::TAU)).cos();
            vertex.z += amplitude * wave;
        }
        frames.push(mesh);
    }
    frames
}

pub fn mesh_fixture_manifest_examples() -> Vec<MeshFixtureManifest> {
    vec![
        synthetic_grid_mesh_fixture_manifest(),
        synthetic_icosphere_mesh_fixture_manifest(),
        synthetic_deforming_grid_mesh_fixture_manifest(),
        synthetic_hand_mesh_fixture_manifest(),
    ]
}

pub fn synthetic_grid_mesh_fixture_manifest() -> MeshFixtureManifest {
    let mesh = build_fixture_grid_mesh(FixtureGridMeshConfig::default());
    let samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
        point_count: 16,
        first_tier_neighbor_count: 4,
        second_tier_neighbor_count: 8,
        seed: 2026,
        ..MeshSurfaceSampleConfig::default()
    });
    let mut manifest = MeshFixtureManifest::from_surface(
        "synthetic-grid-surface-2x2-v1",
        MeshFixtureKind::Grid,
        &mesh,
        samples.point_count(),
    );
    manifest.expected_neighbor_tiers = neighbor_tiers_from_samples(&samples);
    manifest.intended_uses = vec![
        MeshFixtureIntendedUse::TopologyTests,
        MeshFixtureIntendedUse::SamplingTests,
        MeshFixtureIntendedUse::ParticleTests,
        MeshFixtureIntendedUse::RenderPayloadTests,
    ];
    manifest.provenance = MeshFixtureProvenance::Synthetic;
    manifest
}

pub fn synthetic_icosphere_mesh_fixture_manifest() -> MeshFixtureManifest {
    let config = FixtureIcosphereMeshConfig::default();
    let mesh = build_fixture_icosphere_mesh(config);
    let topology = IcosphereTopology::generate(IcosphereTopologyConfig {
        recursion_level: config.recursion_level,
        x_axis_rotation_degrees: config.x_axis_rotation_degrees,
        ..IcosphereTopologyConfig::default()
    });
    let mut manifest = MeshFixtureManifest::from_surface(
        "synthetic-icosphere-l1-v1",
        MeshFixtureKind::Icosphere,
        &mesh,
        topology.point_count(),
    );
    manifest.coordinate_space = MeshFixtureCoordinateSpace::UnitSphere;
    manifest.coordinate_units = MeshFixtureUnits::UnitRadius;
    manifest.coordinate_convention = MeshFixtureCoordinateConvention::UnitSphere;
    manifest.expected_neighbor_tiers = vec![
        summarize_neighbor_tier(1, &topology.first_tier_neighbors),
        summarize_neighbor_tier(2, &topology.second_tier_neighbors),
        summarize_neighbor_tier(3, &topology.third_tier_neighbors),
    ];
    manifest.intended_uses = vec![
        MeshFixtureIntendedUse::TopologyTests,
        MeshFixtureIntendedUse::SamplingTests,
        MeshFixtureIntendedUse::ParticleTests,
    ];
    manifest.provenance = MeshFixtureProvenance::Generated;
    manifest
}

pub fn synthetic_deforming_grid_mesh_fixture_manifest() -> MeshFixtureManifest {
    let config = FixtureDeformingGridConfig::default();
    let frames = build_fixture_deforming_grid_frames(config);
    let mesh = frames
        .first()
        .expect("deforming grid builder always returns at least one frame");
    let samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
        point_count: 32,
        first_tier_neighbor_count: 4,
        second_tier_neighbor_count: 8,
        seed: 2027,
        ..MeshSurfaceSampleConfig::default()
    });
    let mut manifest = MeshFixtureManifest::from_surface(
        "synthetic-deforming-grid-3x3-4f-v1",
        MeshFixtureKind::DeformingMesh,
        mesh,
        samples.point_count(),
    );
    manifest.expected_neighbor_tiers = neighbor_tiers_from_samples(&samples);
    manifest.motion = MeshFixtureMotionKind::Deforming;
    manifest.allowed_deformation_frames = MeshFixtureFrameRange::new(2, config.frame_count);
    manifest
        .validation_expectations
        .push(MeshFixtureValidationExpectation::DeformationFrameRange);
    manifest.intended_uses = vec![
        MeshFixtureIntendedUse::TopologyTests,
        MeshFixtureIntendedUse::SamplingTests,
        MeshFixtureIntendedUse::SdfDepthTests,
        MeshFixtureIntendedUse::ParticleTests,
        MeshFixtureIntendedUse::RenderPayloadTests,
        MeshFixtureIntendedUse::ColliderTests,
    ];
    manifest.provenance = MeshFixtureProvenance::Synthetic;
    manifest
}

pub fn synthetic_hand_mesh_fixture_manifest() -> MeshFixtureManifest {
    let mesh = build_fixture_hand_mesh(super::FixtureHandMeshConfig::default());
    let samples = mesh.sample_even_points(MeshSurfaceSampleConfig {
        point_count: 64,
        first_tier_neighbor_count: 5,
        second_tier_neighbor_count: 7,
        seed: 5,
        ..MeshSurfaceSampleConfig::default()
    });
    let mut manifest = MeshFixtureManifest::from_surface(
        "synthetic-hand-mesh-topology-v1",
        MeshFixtureKind::HandMesh,
        &mesh,
        samples.point_count(),
    );
    manifest.expected_neighbor_tiers = neighbor_tiers_from_samples(&samples);
    manifest.motion = MeshFixtureMotionKind::Deforming;
    manifest.allowed_deformation_frames = MeshFixtureFrameRange::new(1, 16);
    manifest
        .validation_expectations
        .push(MeshFixtureValidationExpectation::DeformationFrameRange);
    manifest.intended_uses = vec![
        MeshFixtureIntendedUse::TopologyTests,
        MeshFixtureIntendedUse::SamplingTests,
        MeshFixtureIntendedUse::SdfDepthTests,
        MeshFixtureIntendedUse::ParticleTests,
        MeshFixtureIntendedUse::RenderPayloadTests,
        MeshFixtureIntendedUse::ColliderTests,
    ];
    manifest.provenance = MeshFixtureProvenance::Synthetic;
    manifest
}

pub fn summarize_neighbor_tier(tier: u8, neighbors: &[Vec<usize>]) -> MeshFixtureNeighborTier {
    let min_neighbor_count = neighbors.iter().map(Vec::len).min().unwrap_or(0);
    let max_neighbor_count = neighbors.iter().map(Vec::len).max().unwrap_or(0);
    MeshFixtureNeighborTier {
        tier,
        min_neighbor_count,
        max_neighbor_count,
    }
}

fn neighbor_tiers_from_samples(samples: &MeshSurfaceSampleSet) -> Vec<MeshFixtureNeighborTier> {
    vec![
        summarize_neighbor_tier(1, &samples.first_tier_neighbors),
        summarize_neighbor_tier(2, &samples.second_tier_neighbors),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_mesh_fixture_manifests_validate() {
        for manifest in mesh_fixture_manifest_examples() {
            manifest
                .validate()
                .expect("fixture manifest should validate");
        }
    }

    #[test]
    fn grid_fixture_manifest_matches_surface_and_samples() {
        let mesh = build_fixture_grid_mesh(FixtureGridMeshConfig::default());
        let manifest = synthetic_grid_mesh_fixture_manifest();

        manifest
            .validate_for_surface(&mesh, manifest.coordinate_sample_count)
            .expect("grid manifest should match generated mesh");
        assert_eq!(manifest.index_count, mesh.triangle_count() * 3);
        assert_eq!(
            manifest.topology_hash,
            MeshSurfaceTopologyKey::from_mesh(&mesh).index_hash
        );
    }

    #[test]
    fn manifest_rejects_mismatched_counts() {
        let mut manifest = synthetic_grid_mesh_fixture_manifest();
        manifest.vertex_count += 1;

        assert_eq!(
            manifest.validate(),
            Err(MeshFixtureManifestError::CountMismatch {
                field: MeshFixtureCountField::VertexCount,
                manifest: 10,
                observed: 9,
            })
        );
    }

    #[test]
    fn manifest_neighbor_tiers_are_consistent() {
        let manifest = synthetic_grid_mesh_fixture_manifest();

        assert_eq!(
            manifest.expected_neighbor_tiers,
            vec![
                MeshFixtureNeighborTier::exact(1, 4),
                MeshFixtureNeighborTier::exact(2, 8),
            ]
        );
    }

    #[test]
    fn deforming_fixture_frame_constraints_are_enforced() {
        let manifest = synthetic_deforming_grid_mesh_fixture_manifest();

        manifest
            .validate_deformation_frame_count(4)
            .expect("configured deforming frame count should be allowed");
        assert!(matches!(
            manifest.validate_deformation_frame_count(5),
            Err(MeshFixtureManifestError::FrameCountOutOfRange { .. })
        ));

        let static_manifest = synthetic_grid_mesh_fixture_manifest();
        assert!(matches!(
            static_manifest.validate_deformation_frame_count(2),
            Err(MeshFixtureManifestError::FrameCountOutOfRange { .. })
        ));
    }

    #[test]
    fn deforming_grid_frames_preserve_topology_hash() {
        let frames = build_fixture_deforming_grid_frames(FixtureDeformingGridConfig::default());
        let first_key = MeshSurfaceTopologyKey::from_mesh(&frames[0]);

        assert_eq!(frames.len(), 4);
        assert!(frames
            .iter()
            .all(|frame| MeshSurfaceTopologyKey::from_mesh(frame) == first_key));
        assert!(frames[0]
            .vertices
            .iter()
            .zip(frames[1].vertices.iter())
            .any(|(a, b)| (a.z - b.z).abs() > 1.0e-6));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn mesh_fixture_manifest_round_trips_with_serde() {
        let manifest = synthetic_deforming_grid_mesh_fixture_manifest();

        let encoded = serde_json::to_string(&manifest).expect("manifest should serialize");
        assert!(encoded.contains(MESH_FIXTURE_MANIFEST_SCHEMA));
        let decoded: MeshFixtureManifest =
            serde_json::from_str(&encoded).expect("manifest should deserialize");

        assert_eq!(decoded, manifest);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn committed_mesh_fixture_json_matches_builders() {
        let fixtures = [
            (
                include_str!("../../../fixtures/mesh/synthetic-grid-surface-2x2-v1.manifest.json"),
                synthetic_grid_mesh_fixture_manifest(),
            ),
            (
                include_str!("../../../fixtures/mesh/synthetic-icosphere-l1-v1.manifest.json"),
                synthetic_icosphere_mesh_fixture_manifest(),
            ),
            (
                include_str!(
                    "../../../fixtures/mesh/synthetic-deforming-grid-3x3-4f-v1.manifest.json"
                ),
                synthetic_deforming_grid_mesh_fixture_manifest(),
            ),
            (
                include_str!(
                    "../../../fixtures/mesh/synthetic-hand-mesh-topology-v1.manifest.json"
                ),
                synthetic_hand_mesh_fixture_manifest(),
            ),
        ];

        for (json, expected) in fixtures {
            let decoded: MeshFixtureManifest =
                serde_json::from_str(json).expect("fixture JSON should deserialize");
            decoded.validate().expect("fixture JSON should validate");
            assert_eq!(decoded, expected);
        }
    }
}
