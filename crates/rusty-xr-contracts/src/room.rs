use crate::{Pose, Vec3};
use core::fmt;

/// General source category for public capture and scan lifecycle reporting.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaptureSourceKind {
    #[default]
    Unknown,
    PassthroughCamera,
    EnvironmentDepth,
    MediaProjection,
    RoomMesh,
    AppRender,
    ImportedFile,
    Synthetic,
}

/// Permission/consent state for a runtime-owned source.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CapturePermissionState {
    #[default]
    Unknown,
    NotRequired,
    Required,
    Requesting,
    Granted,
    Denied,
    Blocked,
}

impl CapturePermissionState {
    pub fn allows_capture(self) -> bool {
        matches!(self, Self::NotRequired | Self::Granted)
    }

    pub fn needs_user_prompt(self) -> bool {
        matches!(self, Self::Required | Self::Denied)
    }
}

/// Runtime lifecycle state for a capture, scan, or mesh source.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaptureLifecycleState {
    #[default]
    Unavailable,
    PermissionRequired,
    Idle,
    Starting,
    Running,
    Paused,
    Stopping,
    Failed,
}

impl CaptureLifecycleState {
    pub fn is_live(self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }

    pub fn needs_user_action(self) -> bool {
        matches!(self, Self::PermissionRequired | Self::Failed)
    }
}

/// Public runtime snapshot for a capture source.
///
/// This is status metadata only. It does not own Android permission prompts,
/// OpenXR provider calls, texture import, file export, or transport.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaptureSourceState {
    pub source_kind: CaptureSourceKind,
    pub lifecycle: CaptureLifecycleState,
    pub permission: CapturePermissionState,
    pub frame_count: u64,
    pub dropped_frame_count: u64,
    pub last_frame_time_ns: Option<u64>,
}

impl CaptureSourceState {
    pub const fn new(source_kind: CaptureSourceKind) -> Self {
        Self {
            source_kind,
            lifecycle: CaptureLifecycleState::Idle,
            permission: CapturePermissionState::Unknown,
            frame_count: 0,
            dropped_frame_count: 0,
            last_frame_time_ns: None,
        }
    }

    pub const fn with_lifecycle(mut self, lifecycle: CaptureLifecycleState) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    pub const fn with_permission(mut self, permission: CapturePermissionState) -> Self {
        self.permission = permission;
        self
    }

    pub const fn with_counts(mut self, frame_count: u64, dropped_frame_count: u64) -> Self {
        self.frame_count = frame_count;
        self.dropped_frame_count = dropped_frame_count;
        self
    }

    pub const fn with_last_frame_time_ns(mut self, last_frame_time_ns: u64) -> Self {
        self.last_frame_time_ns = Some(last_frame_time_ns);
        self
    }

    pub fn is_capturing(self) -> bool {
        self.lifecycle.is_live() && self.permission.allows_capture()
    }

    pub fn dropped_frame_ratio(self) -> Option<f32> {
        let total = self.frame_count.checked_add(self.dropped_frame_count)?;
        if total == 0 {
            None
        } else {
            Some(self.dropped_frame_count as f32 / total as f32)
        }
    }
}

/// Source kind for room or environment mesh snapshots.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoomMeshSourceKind {
    #[default]
    Unknown,
    RuntimeRoomMesh,
    SemanticSceneModel,
    DepthFusion,
    ImportedMesh,
    Synthetic,
}

/// Coordinate space used by a room mesh snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoomMeshCoordinateSpace {
    #[default]
    Local,
    Stage,
    World,
}

/// Public semantic label for a room mesh surface.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoomMeshSemanticLabel {
    #[default]
    Unknown,
    Floor,
    Ceiling,
    Wall,
    Door,
    Window,
    Table,
    Seat,
    Platform,
    Other,
}

/// Lightweight status for a room mesh source.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoomMeshSourceState {
    pub source_kind: RoomMeshSourceKind,
    pub lifecycle: CaptureLifecycleState,
    pub permission: CapturePermissionState,
    pub mesh_version: u64,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub surface_count: u32,
    pub last_update_time_ns: Option<u64>,
}

impl RoomMeshSourceState {
    pub const fn new(source_kind: RoomMeshSourceKind) -> Self {
        Self {
            source_kind,
            lifecycle: CaptureLifecycleState::Idle,
            permission: CapturePermissionState::Unknown,
            mesh_version: 0,
            vertex_count: 0,
            triangle_count: 0,
            surface_count: 0,
            last_update_time_ns: None,
        }
    }

    pub const fn with_lifecycle(mut self, lifecycle: CaptureLifecycleState) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    pub const fn with_permission(mut self, permission: CapturePermissionState) -> Self {
        self.permission = permission;
        self
    }

    pub const fn with_mesh_counts(
        mut self,
        mesh_version: u64,
        vertex_count: u32,
        triangle_count: u32,
        surface_count: u32,
    ) -> Self {
        self.mesh_version = mesh_version;
        self.vertex_count = vertex_count;
        self.triangle_count = triangle_count;
        self.surface_count = surface_count;
        self
    }

    pub const fn with_last_update_time_ns(mut self, last_update_time_ns: u64) -> Self {
        self.last_update_time_ns = Some(last_update_time_ns);
        self
    }

    pub fn has_mesh(self) -> bool {
        self.mesh_version > 0 && self.vertex_count > 0 && self.triangle_count > 0
    }

    pub fn is_live(self) -> bool {
        self.lifecycle.is_live() && self.permission.allows_capture()
    }
}

/// Semantic range over triangles in a room mesh snapshot.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoomMeshSurface {
    pub label: RoomMeshSemanticLabel,
    pub first_triangle_index: u32,
    pub triangle_count: u32,
    pub confidence: u8,
    pub last_seen_time_ns: Option<u64>,
}

impl RoomMeshSurface {
    pub const fn new(
        label: RoomMeshSemanticLabel,
        first_triangle_index: u32,
        triangle_count: u32,
        confidence: u8,
    ) -> Self {
        Self {
            label,
            first_triangle_index,
            triangle_count,
            confidence,
            last_seen_time_ns: None,
        }
    }

    pub const fn with_last_seen_time_ns(mut self, last_seen_time_ns: u64) -> Self {
        self.last_seen_time_ns = Some(last_seen_time_ns);
        self
    }

    pub fn end_triangle_index(self) -> Option<u32> {
        self.first_triangle_index.checked_add(self.triangle_count)
    }

    pub fn is_valid_for_triangle_count(self, total_triangle_count: u32) -> bool {
        self.triangle_count > 0
            && self
                .end_triangle_index()
                .map(|end| end <= total_triangle_count)
                .unwrap_or(false)
    }
}

/// Semantic room mesh snapshot.
///
/// The snapshot is intentionally plain geometry and metadata. Runtime room mesh
/// providers, scene-model APIs, depth fusion, mesh cleanup, and renderer upload
/// remain adapter or downstream responsibilities.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct RoomMeshSnapshot {
    pub version: u64,
    pub source_kind: RoomMeshSourceKind,
    pub coordinate_space: RoomMeshCoordinateSpace,
    pub root_pose: Pose,
    pub captured_time_ns: Option<u64>,
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
    pub surfaces: Vec<RoomMeshSurface>,
}

impl RoomMeshSnapshot {
    pub fn new(
        version: u64,
        source_kind: RoomMeshSourceKind,
        vertices: Vec<Vec3>,
        indices: Vec<[u32; 3]>,
    ) -> Self {
        Self {
            version,
            source_kind,
            coordinate_space: RoomMeshCoordinateSpace::Local,
            root_pose: Pose::IDENTITY,
            captured_time_ns: None,
            vertices,
            indices,
            surfaces: Vec::new(),
        }
    }

    pub fn with_coordinate_space(mut self, coordinate_space: RoomMeshCoordinateSpace) -> Self {
        self.coordinate_space = coordinate_space;
        self
    }

    pub fn with_root_pose(mut self, root_pose: Pose) -> Self {
        self.root_pose = root_pose;
        self
    }

    pub fn with_captured_time_ns(mut self, captured_time_ns: u64) -> Self {
        self.captured_time_ns = Some(captured_time_ns);
        self
    }

    pub fn with_surfaces(mut self, surfaces: Vec<RoomMeshSurface>) -> Self {
        self.surfaces = surfaces;
        self
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    pub fn validate(&self) -> Result<(), RoomMeshError> {
        if self.vertices.is_empty() || self.indices.is_empty() {
            return Err(RoomMeshError::EmptyMesh);
        }
        if !self.root_pose.is_finite() {
            return Err(RoomMeshError::InvalidRootPose);
        }
        for (vertex_index, vertex) in self.vertices.iter().copied().enumerate() {
            if !vertex.is_finite() {
                return Err(RoomMeshError::InvalidVertex { vertex_index });
            }
        }
        for (triangle_index, triangle) in self.indices.iter().copied().enumerate() {
            for vertex_index in triangle {
                if vertex_index as usize >= self.vertices.len() {
                    return Err(RoomMeshError::InvalidTriangleIndex {
                        triangle_index,
                        vertex_index,
                        vertex_count: self.vertices.len(),
                    });
                }
            }
        }

        let total_triangle_count = self.indices.len().min(u32::MAX as usize) as u32;
        for (surface_index, surface) in self.surfaces.iter().copied().enumerate() {
            if !surface.is_valid_for_triangle_count(total_triangle_count) {
                return Err(RoomMeshError::InvalidSurfaceRange { surface_index });
            }
        }

        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Validation errors for room mesh contracts.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomMeshError {
    EmptyMesh,
    InvalidRootPose,
    InvalidVertex {
        vertex_index: usize,
    },
    InvalidTriangleIndex {
        triangle_index: usize,
        vertex_index: u32,
        vertex_count: usize,
    },
    InvalidSurfaceRange {
        surface_index: usize,
    },
}

impl fmt::Display for RoomMeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMesh => f.write_str("room mesh is empty"),
            Self::InvalidRootPose => f.write_str("room mesh root pose is invalid"),
            Self::InvalidVertex { vertex_index } => {
                write!(f, "room mesh vertex {vertex_index} is invalid")
            }
            Self::InvalidTriangleIndex {
                triangle_index,
                vertex_index,
                vertex_count,
            } => write!(
                f,
                "room mesh triangle {triangle_index} references vertex {vertex_index}, but mesh has {vertex_count} vertices"
            ),
            Self::InvalidSurfaceRange { surface_index } => {
                write!(f, "room mesh surface {surface_index} has an invalid triangle range")
            }
        }
    }
}

impl std::error::Error for RoomMeshError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_state_reports_live_and_drop_ratio() {
        let state = CaptureSourceState::new(CaptureSourceKind::EnvironmentDepth)
            .with_lifecycle(CaptureLifecycleState::Running)
            .with_permission(CapturePermissionState::Granted)
            .with_counts(9, 1);

        assert!(state.is_capturing());
        assert_eq!(state.dropped_frame_ratio(), Some(0.1));
    }

    #[test]
    fn room_mesh_source_state_reports_mesh_availability() {
        let state = RoomMeshSourceState::new(RoomMeshSourceKind::RuntimeRoomMesh)
            .with_lifecycle(CaptureLifecycleState::Running)
            .with_permission(CapturePermissionState::Granted)
            .with_mesh_counts(7, 4, 2, 1);

        assert!(state.is_live());
        assert!(state.has_mesh());
    }

    #[test]
    fn room_mesh_snapshot_validates_semantic_surface_ranges() {
        let snapshot = RoomMeshSnapshot::new(
            1,
            RoomMeshSourceKind::Synthetic,
            vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        )
        .with_surfaces(vec![RoomMeshSurface::new(
            RoomMeshSemanticLabel::Floor,
            0,
            1,
            255,
        )]);

        assert_eq!(snapshot.triangle_count(), 1);
        assert!(snapshot.is_valid());
    }

    #[test]
    fn room_mesh_snapshot_rejects_invalid_indices() {
        let snapshot = RoomMeshSnapshot::new(
            1,
            RoomMeshSourceKind::Synthetic,
            vec![Vec3::ZERO],
            vec![[0, 1, 0]],
        );

        assert_eq!(
            snapshot.validate(),
            Err(RoomMeshError::InvalidTriangleIndex {
                triangle_index: 0,
                vertex_index: 1,
                vertex_count: 1,
            })
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn room_mesh_snapshot_round_trips_with_serde() {
        let snapshot = RoomMeshSnapshot::new(
            1,
            RoomMeshSourceKind::Synthetic,
            vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );

        let encoded = serde_json::to_string(&snapshot).expect("snapshot should serialize");
        let decoded: RoomMeshSnapshot =
            serde_json::from_str(&encoded).expect("snapshot should deserialize");

        assert_eq!(decoded, snapshot);
    }
}
