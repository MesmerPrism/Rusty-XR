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
pub mod effect_stack;
pub mod hand;
pub mod home;
pub mod interaction;
pub mod layer;
pub mod math;
pub mod passthrough;
pub mod projection_matrix;
pub mod render;
pub mod room;
pub mod strobe;
pub mod time;
pub mod view;

pub use camera::{
    CameraCompositeTier, CameraExtrinsics, CameraFrameAdoptionMode, CameraFrameMetadata,
    CameraFrameMetadataFlags, CameraFrameTiming, CameraGpuBufferDescriptor, CameraImageRotation,
    CameraIntrinsics, CameraPixelDomain, CameraPixelDomainKind, CameraPoseSource,
    CameraProjectionState, CameraProjectionStatus, CameraSourceDiagnostic,
    CameraSourceDiagnosticsReport, CameraSourceId, CameraTextureColorStatus,
    CameraTextureDescriptorShape, CameraTextureLaneColor, CameraTextureLaneContract,
    CameraTextureLaneKind, CameraTextureLaneLifecycle, CameraTextureLaneProjection,
    CameraTextureLaneResource, CameraTextureLaneSource, CameraTextureLaneTiming,
    CameraTextureLaneTransform, CameraTextureResourceKind, CameraTextureSourceKind,
    CameraTextureTransform, ImageSize, ProjectionTargetState, SourceSamplerYAxis,
    SourceSamplingContract, SourceSamplingTransformStage, SourceUvRect,
    StereoCameraCalibrationProfile, StereoCameraCandidateDiagnostic, StereoCameraFrameMetadata,
    StereoCameraFramePair, StereoSourceEyeMapping, TemporalProjectionEdgeMode,
    TemporalProjectionMetrics, TemporalProjectionMode, TemporalProjectionPolicy,
    VisualProjectionState, CAMERA_TEXTURE_LANE_CONTRACT_SCHEMA, SOURCE_SAMPLING_CONTRACT_SCHEMA,
};
pub use depth::{
    depth_view_position_from_uv, reference_space_point_from_depth_uv,
    render_eye_screen_uv_from_reference_point, ConfidenceFormat, DepthConfidenceSource,
    DepthFormat, DepthFrameDescriptor, DepthMetricRange, DepthPayloadDescriptor,
    DepthSampleIdentityPolicy, DepthViewDescriptor, DepthWorldSpaceContract,
    DepthWorldSpaceMetricRange, DepthWorldSpaceRenderPath, DepthWorldSpaceSourceKind,
    DepthWorldSpaceStageEvidence, DepthWorldSpaceStageKind, EnvironmentDepthState,
    DEPTH_WORLD_SPACE_CONTRACT_SCHEMA,
};
pub use effect_stack::{
    EffectBufferDescriptor, EffectBufferFormat, EffectDiagnosticLayer, EffectLayerComparison,
    EffectLayerComparisonMetrics, EffectLayerMetrics, EffectPassDescriptor, EffectPassInput,
    EffectPassInputRole, EffectPassKind, EffectStackComparisonReport, EffectStackDescriptor,
    EFFECT_STACK_COMPARISON_REPORT_SCHEMA, EFFECT_STACK_DESCRIPTOR_SCHEMA,
};
pub use hand::{
    HandJointName, HandJointPose, HandJointSnapshot, HandMeshError, HandMeshSnapshot, Handedness,
    TrackingConfidence,
};
pub use home::{
    ExternalLaunchState, FocusRecoveryAction, FocusRecoveryEvent, FocusRecoveryResult,
    HomeHelperState, HomeMode, HomePanelDescriptor, HomePanelKind, HomePanelPlacement,
    HomeSessionState, HomeSupervisorPolicy, HomeSupervisorState, KioskCommandEvidence,
    KioskCommandOutcome, KioskCommandProvider, KioskCommandRunRecord, KioskControlPlanePhase,
    KioskControlPlaneStatus, KioskSurfaceIntent, LauncherEntry, LauncherEntrySource,
    SettingsShortcutCategory, SettingsShortcutDescriptor, HOME_FOCUS_RECOVERY_EVENT_SCHEMA,
    HOME_LAUNCHER_ENTRY_SCHEMA, HOME_PANEL_DESCRIPTOR_SCHEMA, HOME_SESSION_STATE_SCHEMA,
    HOME_SETTINGS_SHORTCUT_SCHEMA, KIOSK_COMMAND_EVIDENCE_SCHEMA, KIOSK_COMMAND_RUN_RECORD_SCHEMA,
    KIOSK_CONTROL_PLANE_STATUS_SCHEMA,
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
pub use passthrough::{
    audio_reactive_mono_to_rgba_style, PassthroughColorAdjustment, PassthroughColorLutBinding,
    PassthroughColorLutChannels, PassthroughColorLutSpec, PassthroughColorReproduction,
    PassthroughExtensionRequirements, PassthroughGradientStop,
    PassthroughInterpolatedColorLutBinding, PassthroughLayerPlacement, PassthroughLayerPurpose,
    PassthroughMonoToMonoMap, PassthroughMonoToRgbaMap, PassthroughStyle, PlatformPassthroughLayer,
    PASSTHROUGH_COLOR_MAP_SIZE, XR_FB_PASSTHROUGH_EXTENSION, XR_FB_TRIANGLE_MESH_EXTENSION,
    XR_META_PASSTHROUGH_COLOR_LUT_EXTENSION,
};
pub use projection_matrix::{
    InvalidProjectionFillPolicy, MatrixStepStatus, MatrixSyntheticVideoSource,
    ProjectionFootprintRowSpan, ProjectionFootprintSummary, ProjectionGuideDomain,
    ProjectionMatrixLaneKind, ProjectionMatrixLaneReport, ProjectionPerformanceMatrixPacket,
    ProjectionPerformanceScorecard, ProjectionStageKind, ProjectionStageTokenRow,
    PROJECTION_PERFORMANCE_MATRIX_SCHEMA,
};
pub use render::{
    ColorRgba, CounterSample, CounterValue, RenderCoordinateSpace, RenderPayload, RenderPoint,
    RuntimeCounters,
};
pub use room::{
    CaptureLifecycleState, CapturePermissionState, CaptureSourceKind, CaptureSourceState,
    RoomMeshCoordinateSpace, RoomMeshError, RoomMeshSemanticLabel, RoomMeshSnapshot,
    RoomMeshSourceKind, RoomMeshSourceState, RoomMeshSurface,
};
pub use strobe::{
    safety_class_for_cycle_hz, StrobeFrameFeasibility, StrobeFrequencyPlan, VisualStrobeMode,
    VisualStrobeProfile, VisualStrobeSafetyClass, FULL_FIELD_STROBE_WARNING,
    PHOTOSENSITIVE_RISK_BAND_MAX_HZ, PHOTOSENSITIVE_RISK_BAND_MIN_HZ, WCAG_GENERAL_FLASH_LIMIT_HZ,
    XR_FB_DISPLAY_REFRESH_RATE_EXTENSION,
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
