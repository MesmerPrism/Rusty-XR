//! Framework-neutral XR data contracts for Rusty XR.
//!
//! This crate deliberately contains plain Rust data and small validation
//! helpers only. It does not depend on Android, OpenXR, Vulkan, Makepad, Unity,
//! Meta SDKs, LSL, or downstream application repositories.
//!
//! Enable the `serde` feature to derive serialization for stable public data
//! contracts without making serialization mandatory for plain Rust consumers.
//!
//! ```
//! use rusty_xr_contracts::{CameraIntrinsics, ImageSize, Vec2};
//!
//! let intrinsics = CameraIntrinsics::new(
//!     Vec2::new(500.0, 510.0),
//!     Vec2::new(320.0, 240.0),
//!     ImageSize::new(640, 480),
//! );
//! assert!(intrinsics.is_valid());
//! ```

pub mod camera;
pub mod depth;
pub mod hand;
pub mod interaction;
pub mod layer;
pub mod math;
pub mod render;
pub mod room;
pub mod time;
pub mod view;

pub use camera::{
    CameraExtrinsics, CameraFrameMetadata, CameraIntrinsics, CameraSourceId, ImageSize,
    StereoCameraFrameMetadata,
};
pub use depth::{
    ConfidenceFormat, DepthFormat, DepthFrameDescriptor, DepthPayloadDescriptor,
    EnvironmentDepthState,
};
pub use hand::{
    HandJointName, HandJointPose, HandJointSnapshot, HandMeshError, HandMeshSnapshot, Handedness,
    TrackingConfidence,
};
pub use interaction::{
    HandInfluencePoint, HandMenuActivation, HandMenuAnchor, InteractionRay, XrCanvasHit,
    XrCanvasSurface,
};
pub use layer::{
    FeedbackBorderTuning, PlainStereoLayer, Rect2, StereoLayerCameraPath, StereoLayerContentMode,
    StereoLayerDepthPolicy, StereoLayerPerformanceHints, StereoMediaLayout, VisualFeedbackBorder,
    VisualFeedbackBorderLayout, VisualFeedbackLayerTuning,
};
pub use math::{Pose, Quat, Vec2, Vec3};
pub use render::{
    ColorRgba, CounterSample, CounterValue, RenderCoordinateSpace, RenderPayload, RenderPoint,
    RuntimeCounters,
};
pub use room::{
    CaptureLifecycleState, CapturePermissionState, CaptureSourceKind, CaptureSourceState,
    RoomMeshCoordinateSpace, RoomMeshError, RoomMeshSemanticLabel, RoomMeshSnapshot,
    RoomMeshSourceKind, RoomMeshSourceState, RoomMeshSurface,
};
pub use time::FrameTiming;
pub use view::{Eye, EyeView, FieldOfView, StereoViews};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }
}
