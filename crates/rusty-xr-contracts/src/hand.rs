use crate::{Pose, Vec3};

/// Left/right hand label independent of any platform API.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handedness {
    Left,
    Right,
}

/// Stable joint names matching common XR hand-tracking skeletons.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandJointName {
    Palm,
    Wrist,
    ThumbMetacarpal,
    ThumbProximal,
    ThumbDistal,
    ThumbTip,
    IndexMetacarpal,
    IndexProximal,
    IndexIntermediate,
    IndexDistal,
    IndexTip,
    MiddleMetacarpal,
    MiddleProximal,
    MiddleIntermediate,
    MiddleDistal,
    MiddleTip,
    RingMetacarpal,
    RingProximal,
    RingIntermediate,
    RingDistal,
    RingTip,
    LittleMetacarpal,
    LittleProximal,
    LittleIntermediate,
    LittleDistal,
    LittleTip,
}

/// Tracking confidence for a pose, joint, or mesh snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackingConfidence {
    None,
    Low,
    High,
}

/// Pose and radius for one tracked hand joint.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandJointPose {
    pub joint: HandJointName,
    pub pose: Pose,
    pub radius_meters: f32,
    pub confidence: TrackingConfidence,
}

impl HandJointPose {
    pub const fn new(joint: HandJointName, pose: Pose, radius_meters: f32) -> Self {
        Self {
            joint,
            pose,
            radius_meters,
            confidence: TrackingConfidence::High,
        }
    }

    pub fn is_valid(self) -> bool {
        self.pose.is_finite() && self.radius_meters.is_finite() && self.radius_meters >= 0.0
    }
}

/// Joint snapshot for one hand at one frame/timestamp.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HandJointSnapshot {
    pub handedness: Handedness,
    pub frame_index: u64,
    pub timestamp_ns: Option<u64>,
    pub joints: Vec<HandJointPose>,
}

impl HandJointSnapshot {
    pub fn new(handedness: Handedness, frame_index: u64, joints: Vec<HandJointPose>) -> Self {
        Self {
            handedness,
            frame_index,
            timestamp_ns: None,
            joints,
        }
    }

    pub fn with_timestamp_ns(mut self, timestamp_ns: u64) -> Self {
        self.timestamp_ns = Some(timestamp_ns);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.joints.iter().copied().all(HandJointPose::is_valid)
    }
}

/// Validation errors for public hand mesh snapshots.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandMeshError {
    EmptyVertices,
    InvalidNormalCount {
        vertex_count: usize,
        normal_count: usize,
    },
    InvalidIndex {
        triangle_index: usize,
        vertex_index: u32,
        vertex_count: usize,
    },
    InvalidSkinningCount {
        vertex_count: usize,
        skinning_count: usize,
    },
}

/// Deformed or bind-pose hand mesh snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HandMeshSnapshot {
    pub handedness: Option<Handedness>,
    pub version: u64,
    pub timestamp_ns: Option<u64>,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
    pub joint_indices: Vec<[u16; 4]>,
    pub joint_weights: Vec<[f32; 4]>,
}

impl HandMeshSnapshot {
    pub fn new(version: u64, vertices: Vec<Vec3>, indices: Vec<[u32; 3]>) -> Self {
        Self {
            handedness: None,
            version,
            timestamp_ns: None,
            vertices,
            normals: Vec::new(),
            indices,
            joint_indices: Vec::new(),
            joint_weights: Vec::new(),
        }
    }

    pub fn with_handedness(mut self, handedness: Handedness) -> Self {
        self.handedness = Some(handedness);
        self
    }

    pub fn with_timestamp_ns(mut self, timestamp_ns: u64) -> Self {
        self.timestamp_ns = Some(timestamp_ns);
        self
    }

    pub fn validate(&self) -> Result<(), HandMeshError> {
        if self.vertices.is_empty() {
            return Err(HandMeshError::EmptyVertices);
        }

        if !self.normals.is_empty() && self.normals.len() != self.vertices.len() {
            return Err(HandMeshError::InvalidNormalCount {
                vertex_count: self.vertices.len(),
                normal_count: self.normals.len(),
            });
        }

        for (triangle_index, triangle) in self.indices.iter().copied().enumerate() {
            for vertex_index in triangle {
                if vertex_index as usize >= self.vertices.len() {
                    return Err(HandMeshError::InvalidIndex {
                        triangle_index,
                        vertex_index,
                        vertex_count: self.vertices.len(),
                    });
                }
            }
        }

        if !self.joint_indices.is_empty() && self.joint_indices.len() != self.vertices.len() {
            return Err(HandMeshError::InvalidSkinningCount {
                vertex_count: self.vertices.len(),
                skinning_count: self.joint_indices.len(),
            });
        }

        if !self.joint_weights.is_empty() && self.joint_weights.len() != self.vertices.len() {
            return Err(HandMeshError::InvalidSkinningCount {
                vertex_count: self.vertices.len(),
                skinning_count: self.joint_weights.len(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_mesh_rejects_out_of_range_indices() {
        let mesh = HandMeshSnapshot::new(1, vec![Vec3::ZERO], vec![[0, 1, 0]]);

        assert_eq!(
            mesh.validate(),
            Err(HandMeshError::InvalidIndex {
                triangle_index: 0,
                vertex_index: 1,
                vertex_count: 1,
            })
        );
    }
}
