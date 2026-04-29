//! Camera metadata and projection helpers for Rusty XR.
//!
//! This crate owns camera math that is public and app-neutral: intrinsics
//! scaling, pinhole projection/back-projection, and timestamp matching.
//!
//! Enable the `serde` feature to serialize helper result types; camera metadata
//! serialization is forwarded through `rusty-xr-contracts/serde`.
//!
//! ```
//! use rusty_xr_camera_model::{project_camera_point, CameraIntrinsics, ImageSize, Vec2, Vec3};
//!
//! let intrinsics = CameraIntrinsics::new(
//!     Vec2::new(500.0, 500.0),
//!     Vec2::new(320.0, 240.0),
//!     ImageSize::new(640, 480),
//! );
//! let pixel = project_camera_point(intrinsics, Vec3::new(0.0, 0.0, 1.0)).unwrap();
//! assert_eq!(pixel, Vec2::new(320.0, 240.0));
//! ```

use core::fmt;

pub use rusty_xr_contracts::{
    CameraExtrinsics, CameraFrameMetadata, CameraIntrinsics, CameraSourceId, ImageSize,
    StereoCameraFrameMetadata, Vec2, Vec3,
};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Camera model helper error.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraModelError {
    InvalidSourceIntrinsics,
    InvalidSourceSize,
    InvalidTargetSize,
    PointBehindCamera,
    ZeroDepth,
}

impl fmt::Display for CameraModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSourceIntrinsics => f.write_str("invalid source intrinsics"),
            Self::InvalidSourceSize => f.write_str("invalid source image size"),
            Self::InvalidTargetSize => f.write_str("invalid target image size"),
            Self::PointBehindCamera => f.write_str("point is behind the camera"),
            Self::ZeroDepth => f.write_str("depth must be non-zero"),
        }
    }
}

impl std::error::Error for CameraModelError {}

/// Scale camera intrinsics from one pixel domain to another.
///
/// This is the core active-array-to-preview-stream operation used by camera
/// adapters before handing projection data to app-neutral consumers.
pub fn scale_intrinsics_to_image(
    intrinsics: CameraIntrinsics,
    source_size: ImageSize,
    target_size: ImageSize,
) -> Result<CameraIntrinsics, CameraModelError> {
    if !intrinsics.is_valid() {
        return Err(CameraModelError::InvalidSourceIntrinsics);
    }
    if !source_size.is_non_empty() {
        return Err(CameraModelError::InvalidSourceSize);
    }
    if !target_size.is_non_empty() {
        return Err(CameraModelError::InvalidTargetSize);
    }

    let scale = Vec2::new(
        target_size.width as f32 / source_size.width as f32,
        target_size.height as f32 / source_size.height as f32,
    );

    Ok(CameraIntrinsics::new(
        Vec2::new(
            intrinsics.focal_length_px.x * scale.x,
            intrinsics.focal_length_px.y * scale.y,
        ),
        Vec2::new(
            intrinsics.principal_point_px.x * scale.x,
            intrinsics.principal_point_px.y * scale.y,
        ),
        target_size,
    ))
}

/// Project a camera-space point into pixel coordinates.
pub fn project_camera_point(
    intrinsics: CameraIntrinsics,
    camera_point: Vec3,
) -> Result<Vec2, CameraModelError> {
    if !intrinsics.is_valid() {
        return Err(CameraModelError::InvalidSourceIntrinsics);
    }
    if camera_point.z <= 0.0 {
        return Err(CameraModelError::PointBehindCamera);
    }

    Ok(Vec2::new(
        (camera_point.x * intrinsics.focal_length_px.x / camera_point.z)
            + intrinsics.principal_point_px.x,
        (camera_point.y * intrinsics.focal_length_px.y / camera_point.z)
            + intrinsics.principal_point_px.y,
    ))
}

/// Back-project a pixel and metric depth into camera-space coordinates.
pub fn back_project_pixel(
    intrinsics: CameraIntrinsics,
    pixel: Vec2,
    depth_meters: f32,
) -> Result<Vec3, CameraModelError> {
    if !intrinsics.is_valid() {
        return Err(CameraModelError::InvalidSourceIntrinsics);
    }
    if !depth_meters.is_finite() || depth_meters.abs() <= f32::EPSILON {
        return Err(CameraModelError::ZeroDepth);
    }

    Ok(Vec3::new(
        (pixel.x - intrinsics.principal_point_px.x) * depth_meters / intrinsics.focal_length_px.x,
        (pixel.y - intrinsics.principal_point_px.y) * depth_meters / intrinsics.focal_length_px.y,
        depth_meters,
    ))
}

/// Result of nearest timestamp matching.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimestampMatch {
    pub candidate_index: usize,
    pub delta_ns: i128,
}

impl TimestampMatch {
    pub const fn absolute_delta_ns(self) -> u128 {
        self.delta_ns.unsigned_abs()
    }
}

/// Find the nearest candidate timestamp to a target timestamp.
pub fn match_nearest_timestamp(
    target_timestamp_ns: u64,
    candidate_timestamps_ns: &[u64],
    max_delta_ns: Option<u64>,
) -> Option<TimestampMatch> {
    let best = candidate_timestamps_ns
        .iter()
        .copied()
        .enumerate()
        .map(|(candidate_index, candidate)| TimestampMatch {
            candidate_index,
            delta_ns: candidate as i128 - target_timestamp_ns as i128,
        })
        .min_by_key(|candidate| candidate.absolute_delta_ns())?;

    if max_delta_ns
        .map(|max_delta_ns| best.absolute_delta_ns() <= max_delta_ns as u128)
        .unwrap_or(true)
    {
        Some(best)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_intrinsics() -> CameraIntrinsics {
        CameraIntrinsics::new(
            Vec2::new(1000.0, 900.0),
            Vec2::new(500.0, 400.0),
            ImageSize::new(1000, 800),
        )
    }

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn scales_intrinsics_between_pixel_domains() {
        let scaled = scale_intrinsics_to_image(
            test_intrinsics(),
            ImageSize::new(1000, 800),
            ImageSize::new(500, 400),
        )
        .expect("intrinsics should scale");

        assert_eq!(scaled.focal_length_px, Vec2::new(500.0, 450.0));
        assert_eq!(scaled.principal_point_px, Vec2::new(250.0, 200.0));
        assert_eq!(scaled.image_size, ImageSize::new(500, 400));
    }

    #[test]
    fn projection_round_trips_camera_point() {
        let intrinsics = test_intrinsics();
        let camera_point = Vec3::new(0.1, -0.2, 2.0);
        let pixel = project_camera_point(intrinsics, camera_point).expect("point should project");
        let round_trip =
            back_project_pixel(intrinsics, pixel, camera_point.z).expect("pixel should unproject");

        assert!((round_trip.x - camera_point.x).abs() < 1.0e-5);
        assert!((round_trip.y - camera_point.y).abs() < 1.0e-5);
        assert!((round_trip.z - camera_point.z).abs() < 1.0e-5);
    }

    #[test]
    fn timestamp_matching_respects_max_delta() {
        let timestamps = [100, 150, 210];
        let matched = match_nearest_timestamp(160, &timestamps, Some(20))
            .expect("nearest timestamp should match");

        assert_eq!(
            matched,
            TimestampMatch {
                candidate_index: 1,
                delta_ns: -10,
            }
        );
        assert_eq!(match_nearest_timestamp(160, &timestamps, Some(5)), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn timestamp_match_round_trips_with_serde() {
        let value = TimestampMatch {
            candidate_index: 1,
            delta_ns: -10,
        };

        let encoded = serde_json::to_string(&value).expect("match should serialize");
        let decoded: TimestampMatch =
            serde_json::from_str(&encoded).expect("match should deserialize");

        assert_eq!(decoded, value);
    }
}
