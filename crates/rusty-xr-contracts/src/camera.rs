use crate::{Pose, Vec2};

/// Pixel dimensions for a frame, view, or payload.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

impl ImageSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn is_non_empty(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Stable public identifier for a logical camera source.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CameraSourceId {
    pub label: String,
    pub physical_id: Option<String>,
}

impl CameraSourceId {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            physical_id: None,
        }
    }

    pub fn with_physical_id(mut self, physical_id: impl Into<String>) -> Self {
        self.physical_id = Some(physical_id.into());
        self
    }
}

/// Pinhole camera intrinsics in the delivered frame's pixel domain.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraIntrinsics {
    pub focal_length_px: Vec2,
    pub principal_point_px: Vec2,
    pub image_size: ImageSize,
}

impl CameraIntrinsics {
    pub const fn new(
        focal_length_px: Vec2,
        principal_point_px: Vec2,
        image_size: ImageSize,
    ) -> Self {
        Self {
            focal_length_px,
            principal_point_px,
            image_size,
        }
    }

    pub fn is_valid(self) -> bool {
        self.image_size.is_non_empty()
            && self.focal_length_px.is_finite()
            && self.principal_point_px.is_finite()
            && self.focal_length_px.x > 0.0
            && self.focal_length_px.y > 0.0
    }
}

/// Camera pose relative to a named coordinate frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraExtrinsics {
    pub world_from_camera: Pose,
}

impl CameraExtrinsics {
    pub const fn new(world_from_camera: Pose) -> Self {
        Self { world_from_camera }
    }

    pub fn is_valid(self) -> bool {
        self.world_from_camera.is_finite()
    }
}

/// Metadata for one camera frame without owning the image payload.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CameraFrameMetadata {
    pub source: CameraSourceId,
    pub frame_index: u64,
    pub timestamp_ns: Option<u64>,
    pub intrinsics: CameraIntrinsics,
    pub extrinsics: Option<CameraExtrinsics>,
}

impl CameraFrameMetadata {
    pub fn new(source: CameraSourceId, frame_index: u64, intrinsics: CameraIntrinsics) -> Self {
        Self {
            source,
            frame_index,
            timestamp_ns: None,
            intrinsics,
            extrinsics: None,
        }
    }

    pub fn with_timestamp_ns(mut self, timestamp_ns: u64) -> Self {
        self.timestamp_ns = Some(timestamp_ns);
        self
    }

    pub fn with_extrinsics(mut self, extrinsics: CameraExtrinsics) -> Self {
        self.extrinsics = Some(extrinsics);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.intrinsics.is_valid()
            && self
                .extrinsics
                .map(CameraExtrinsics::is_valid)
                .unwrap_or(true)
    }
}

/// Metadata for synchronized or near-synchronized stereo camera frames.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct StereoCameraFrameMetadata {
    pub frame_index: u64,
    pub left: CameraFrameMetadata,
    pub right: CameraFrameMetadata,
    pub midpoint_timestamp_ns: Option<u64>,
}

impl StereoCameraFrameMetadata {
    pub fn new(frame_index: u64, left: CameraFrameMetadata, right: CameraFrameMetadata) -> Self {
        Self {
            frame_index,
            left,
            right,
            midpoint_timestamp_ns: None,
        }
    }

    pub fn with_midpoint_timestamp_ns(mut self, midpoint_timestamp_ns: u64) -> Self {
        self.midpoint_timestamp_ns = Some(midpoint_timestamp_ns);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.left.is_valid() && self.right.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_intrinsics_require_positive_focal_length() {
        let valid = CameraIntrinsics::new(
            Vec2::new(500.0, 510.0),
            Vec2::new(320.0, 240.0),
            ImageSize::new(640, 480),
        );
        let invalid = CameraIntrinsics::new(
            Vec2::new(0.0, 510.0),
            Vec2::new(320.0, 240.0),
            ImageSize::new(640, 480),
        );

        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn camera_metadata_round_trips_with_serde() {
        let intrinsics = CameraIntrinsics::new(
            Vec2::new(500.0, 510.0),
            Vec2::new(320.0, 240.0),
            ImageSize::new(640, 480),
        );
        let metadata = CameraFrameMetadata::new(CameraSourceId::new("left"), 42, intrinsics)
            .with_timestamp_ns(123);

        let encoded = serde_json::to_string(&metadata).expect("metadata should serialize");
        let decoded: CameraFrameMetadata =
            serde_json::from_str(&encoded).expect("metadata should deserialize");

        assert_eq!(decoded, metadata);
    }
}
