//! Native payload for the public Quest composite-layer APK example.
//!
//! The Android build uses this library both as the `NativeActivity` entrypoint
//! and as a small JNI bridge for contract/status logging. The OpenXR renderer is
//! intentionally minimal: it supports a diagnostic flat camera copy and a
//! public Android hardware-buffer probe, plus the public stereo GPU projection
//! path when paired buffers and per-eye metadata are available.

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use jni::{
    objects::{JByteArray, JClass, JObject, JString},
    sys::{jboolean, jint, jlong, jstring},
    JNIEnv,
};
use rusty_xr_camera_model::camera2_lens_pose_to_extrinsics;
use rusty_xr_contracts::{
    CameraCompositeTier, CameraExtrinsics, CameraFrameMetadata, CameraFrameMetadataFlags,
    CameraGpuBufferDescriptor, CameraImageRotation, CameraIntrinsics, CameraPixelDomain,
    CameraPixelDomainKind, CameraProjectionStatus, CameraSourceId, CameraTextureTransform,
    CaptureLifecycleState, CapturePermissionState, CaptureSourceKind, CaptureSourceState,
    ColorRgba, EnvironmentDepthState, ImageSize, PlainStereoLayer, Pose, Quat, Rect2,
    StereoLayerCameraPath, StereoLayerContentMode, StereoLayerPerformanceHints, StereoMediaLayout,
    StereoSourceEyeMapping, TemporalProjectionEdgeMode, TemporalProjectionMode, Vec2, Vec3,
    VisualFeedbackBorder, VisualFeedbackBorderLayout,
};
use rusty_xr_debug_canvas::{
    DiagnosticHudCommand, DiagnosticHudInputSource, DiagnosticHudState, DiagnosticHudUpdate,
};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    sync::{Mutex, OnceLock},
};

pub(crate) const CAMERA_IMPORT_CACHE_LIMIT_DEFAULT: usize = 16;
pub(crate) const CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = 16;

mod camera_color_pipeline;
pub(crate) use camera_color_pipeline::{
    CameraFeedPipelineMode, CameraPeripheralStretchBlendMode, CameraPeripheralStretchConfig,
    CameraPeripheralStretchCornerMode, CameraPeripheralStretchDebug, CameraPeripheralStretchMode,
    CameraProcessingLayer, CameraProjectionBorderPolicy, CameraProjectionEffectMode,
    OpenXrColorFormatMode,
};

mod projection_runtime;
#[path = "openxr_vulkan/projection_target_footprint.rs"]
mod projection_target_footprint;
mod source_sampling;
#[cfg(target_os = "android")]
use projection_runtime::log_projection_runtime_manifest;
use projection_runtime::{
    apply_hwb_projection_runtime_resolution, hwb_projection_runtime_resolution,
};

#[cfg(target_os = "android")]
mod acamera_sys;

#[cfg(target_os = "android")]
mod native_camera;

#[cfg(target_os = "android")]
mod openxr_vulkan;

#[cfg(target_os = "android")]
mod osc_ingress;

#[cfg(target_os = "android")]
pub(crate) fn log_info(message: impl AsRef<str>) {
    android_log(
        ndk_sys::android_LogPriority::ANDROID_LOG_INFO,
        message.as_ref(),
    );
}

#[cfg(target_os = "android")]
pub(crate) fn log_error(message: impl AsRef<str>) {
    android_log(
        ndk_sys::android_LogPriority::ANDROID_LOG_ERROR,
        message.as_ref(),
    );
}

#[cfg(not(target_os = "android"))]
pub(crate) fn log_error(message: impl AsRef<str>) {
    eprintln!("{}", message.as_ref());
}

#[cfg(target_os = "android")]
fn android_log(priority: ndk_sys::android_LogPriority, message: &str) {
    use std::{ffi::CString, os::raw::c_int};

    let tag = CString::new("RustyXrComposite").expect("static Android log tag is valid");
    let safe_message = message.replace('\0', "\\0");
    if let Ok(message) = CString::new(safe_message) {
        unsafe {
            ndk_sys::__android_log_write(priority.0 as c_int, tag.as_ptr(), message.as_ptr());
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CompositeLayerSession {
    schema_version: &'static str,
    app_id: &'static str,
    package_name: &'static str,
    activity_name: &'static str,
    layer_kind: &'static str,
    render_path: &'static str,
    alignment_mode: &'static str,
    stereo_layout: StereoMediaLayout,
    feedback_layer: PlainStereoLayer,
    content_rect: Rect2,
    border_layout: VisualFeedbackBorderLayout,
    capture_sources: [CaptureSourceState; 4],
    environment_depth: EnvironmentDepthState,
    notes: [&'static str; 7],
}

#[derive(Clone)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct HeadsetCameraFrame {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) timestamp_ns: i64,
    pub(crate) index: u64,
    pub(crate) metadata: CameraFrameMetadata,
    pub(crate) diagnostics: HeadsetCameraFrameDiagnostics,
    pub(crate) rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct HeadsetCameraFrameDiagnostics {
    pub(crate) source: Option<String>,
    pub(crate) camera_id: Option<String>,
    pub(crate) lens_facing: Option<String>,
    pub(crate) lens_facing_rank: Option<i32>,
    pub(crate) selection_score: Option<i64>,
    pub(crate) requested_tier: CameraCompositeTier,
    pub(crate) active_tier_label: String,
    pub(crate) transport: String,
    pub(crate) pose_source: Option<String>,
    pub(crate) pose_coordinate_convention: Option<String>,
    pub(crate) lens_pose_reference_label: Option<String>,
    #[expect(
        dead_code,
        reason = "retained in the camera contract for analyzer-side source metadata"
    )]
    pub(crate) diagnostic_source: Option<bool>,
    pub(crate) synthetic_projection_profile: Option<String>,
    pub(crate) projection_geometry_profile: Option<String>,
    pub(crate) synthetic_pattern: Option<String>,
    pub(crate) orientation_kind: Option<String>,
    pub(crate) raster_orientation: Option<String>,
    pub(crate) upright_marker: Option<String>,
    pub(crate) orientation_metadata_source: Option<String>,
    pub(crate) orientation_default: Option<bool>,
    pub(crate) stimulus_raster_orientation: Option<String>,
    pub(crate) stimulus_upright_marker: Option<String>,
    pub(crate) stimulus_orientation_default: Option<bool>,
    pub(crate) content_kind: Option<String>,
    pub(crate) content_width: Option<u32>,
    pub(crate) content_height: Option<u32>,
    pub(crate) content_aspect_ratio: Option<f32>,
    pub(crate) desired_display_aspect_ratio: Option<f32>,
    pub(crate) desired_projection_aspect_ratio: Option<f32>,
    pub(crate) content_coordinate_space: Option<String>,
    pub(crate) content_origin: Option<String>,
    pub(crate) content_x_axis: Option<String>,
    pub(crate) content_y_axis: Option<String>,
    pub(crate) content_uv_rect: Option<[f32; 4]>,
    pub(crate) source_visible_uv_rect: Option<[f32; 4]>,
    pub(crate) source_crop_rect_px: Option<[u32; 4]>,
    pub(crate) source_crop_rect_state: Option<String>,
    pub(crate) source_crop_rect_owner: Option<String>,
    pub(crate) source_sampling_mode: Option<String>,
    pub(crate) content_mapping_intent: Option<String>,
    pub(crate) content_geometry_metadata_source: Option<String>,
    pub(crate) content_geometry_default: Option<bool>,
    pub(crate) target_footprint_schema: Option<String>,
    pub(crate) target_coordinate_space: Option<String>,
    pub(crate) target_screen_uv_rect: Option<[f32; 4]>,
    pub(crate) target_clip_policy: Option<String>,
    pub(crate) target_footprint_metadata_source: Option<String>,
    pub(crate) target_footprint_default: Option<bool>,
    pub(crate) requested_stereo_layout: Option<String>,
    pub(crate) stereo_layout: StereoMediaLayout,
    pub(crate) mono_fallback: bool,
    pub(crate) fallback_reason: String,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct HeadsetCameraGpuFrame {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) timestamp_ns: i64,
    pub(crate) index: u64,
    pub(crate) metadata: CameraFrameMetadata,
    pub(crate) diagnostics: HeadsetCameraFrameDiagnostics,
    pub(crate) descriptor: CameraGpuBufferDescriptor,
    #[cfg(target_os = "android")]
    pub(crate) hardware_buffer: AndroidHardwareBufferHandle,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct StereoGpuCameraFrame {
    pub(crate) index: u64,
    pub(crate) left: HeadsetCameraGpuFrame,
    pub(crate) right: HeadsetCameraGpuFrame,
    pub(crate) pair_delta_ns: u64,
    pub(crate) midpoint_timestamp_ns: i64,
}

#[cfg(target_os = "android")]
#[derive(Debug)]
pub(crate) struct AndroidHardwareBufferHandle {
    ptr: *mut ndk_sys::AHardwareBuffer,
}

#[cfg(target_os = "android")]
unsafe impl Send for AndroidHardwareBufferHandle {}

#[cfg(target_os = "android")]
unsafe impl Sync for AndroidHardwareBufferHandle {}

#[cfg(target_os = "android")]
impl AndroidHardwareBufferHandle {
    fn acquire(ptr: *mut ndk_sys::AHardwareBuffer) -> Result<Self, String> {
        if ptr.is_null() {
            return Err("AHardwareBuffer pointer is null".to_string());
        }
        unsafe {
            ndk_sys::AHardwareBuffer_acquire(ptr);
        }
        Ok(Self { ptr })
    }

    pub(crate) fn as_ptr(&self) -> *mut ndk_sys::AHardwareBuffer {
        self.ptr
    }
}

#[cfg(target_os = "android")]
impl Clone for AndroidHardwareBufferHandle {
    fn clone(&self) -> Self {
        unsafe {
            ndk_sys::AHardwareBuffer_acquire(self.ptr);
        }
        Self { ptr: self.ptr }
    }
}

#[cfg(target_os = "android")]
impl Drop for AndroidHardwareBufferHandle {
    fn drop(&mut self) {
        unsafe {
            ndk_sys::AHardwareBuffer_release(self.ptr);
        }
    }
}

#[derive(Clone, Debug)]
struct HeadsetCameraGpuBufferImport {
    descriptor: CameraGpuBufferDescriptor,
    #[cfg(target_os = "android")]
    hardware_buffer: AndroidHardwareBufferHandle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraProjectionMode {
    #[default]
    DisplayScreenHomography,
    QuadSurface,
    WorldCanvas,
}

impl CameraProjectionMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "display-screen-homography"
            | "screen-homography"
            | "display-eye-homography"
            | "fullscreen"
            | "default" => Some(Self::DisplayScreenHomography),
            "quad-surface" | "quadSurface" | "content-surface" | "surface" | "quad-style" => {
                Some(Self::QuadSurface)
            }
            "world-canvas" | "worldCanvas" | "world-space-canvas" | "world-space-quad"
            | "mesh-quad" | "actual-quad" => Some(Self::WorldCanvas),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::DisplayScreenHomography => "display-screen-homography",
            Self::QuadSurface => "quad-surface",
            Self::WorldCanvas => "world-canvas",
        }
    }

    pub(crate) const fn projection_surface_label(self) -> &'static str {
        match self {
            Self::DisplayScreenHomography => "head-anchored-content-surface-via-openxr-eye-view",
            Self::QuadSurface => "head-anchored-content-surface-quad-emulated",
            Self::WorldCanvas => "head-anchored-content-surface-world-canvas",
        }
    }

    pub(crate) const fn uses_world_canvas(self) -> bool {
        matches!(self, Self::WorldCanvas)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraColorMode {
    #[default]
    ExternalRgb,
    ExternalCrYCbBt601Narrow,
    DebugRedOnly,
}

impl CameraColorMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "external-rgb" | "rgb" | "default" => Some(Self::ExternalRgb),
            "external-cr-y-cb-bt601-narrow"
            | "external-ycbcr-bt601-narrow"
            | "cr-y-cb-bt601-narrow"
            | "bt601-narrow"
            | "quest-external-bt601" => Some(Self::ExternalCrYCbBt601Narrow),
            "debug-red-only" | "debug-r-only" | "red-only" => Some(Self::DebugRedOnly),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::ExternalRgb => "external-rgb",
            Self::ExternalCrYCbBt601Narrow => "external-cr-y-cb-bt601-narrow",
            Self::DebugRedOnly => "debug-red-only",
        }
    }

    pub(crate) const fn shader_bit(self) -> u32 {
        match self {
            Self::ExternalRgb => 0,
            Self::ExternalCrYCbBt601Narrow => 1 << 11,
            Self::DebugRedOnly => 1 << 12,
        }
    }

    pub(crate) const fn source_color_input_encoding(self) -> &'static str {
        match self {
            Self::ExternalRgb => "hardware-buffer-external-rgb",
            Self::ExternalCrYCbBt601Narrow => "hardware-buffer-external-cr-y-cb-bt601-narrow",
            Self::DebugRedOnly => "hardware-buffer-external-rgb",
        }
    }

    pub(crate) const fn source_color_transform(self) -> &'static str {
        match self {
            Self::ExternalRgb => "identity",
            Self::ExternalCrYCbBt601Narrow => "bt601-narrow-ycbcr-to-rgb",
            Self::DebugRedOnly => "debug-red-channel-only",
        }
    }

    pub(crate) const fn source_color_transform_applied(self) -> bool {
        !matches!(self, Self::ExternalRgb)
    }

    pub(crate) const fn source_color_output_encoding(self) -> &'static str {
        match self {
            Self::ExternalRgb => "renderer-native-rgb",
            Self::ExternalCrYCbBt601Narrow => "renderer-native-rgb",
            Self::DebugRedOnly => "debug-red-only-rgb",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraSamplerBindingMode {
    #[default]
    CombinedImmutableSampler,
    SeparateImageSampler,
    SeparateImmutableSampler,
}

impl CameraSamplerBindingMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "combined-immutable-sampler" | "combined-sampler" | "combined" | "default" => {
                Some(Self::CombinedImmutableSampler)
            }
            "separate-image-sampler"
            | "separate-sampler"
            | "separate"
            | "sampled-image-plus-sampler" => Some(Self::SeparateImageSampler),
            "separate-immutable-sampler"
            | "immutable-separate-sampler"
            | "sampled-image-plus-immutable-sampler" => Some(Self::SeparateImmutableSampler),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::CombinedImmutableSampler => "combined-immutable-sampler",
            Self::SeparateImageSampler => "separate-image-sampler",
            Self::SeparateImmutableSampler => "separate-immutable-sampler",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraImportImageLayoutMode {
    #[default]
    ShaderReadOnlyTransition,
    GeneralNoTransition,
}

impl CameraImportImageLayoutMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "shader-read-transition"
            | "shader-read"
            | "shader-read-only"
            | "transition"
            | "default" => Some(Self::ShaderReadOnlyTransition),
            "general-no-transition" | "general" | "no-transition" | "external-general" => {
                Some(Self::GeneralNoTransition)
            }
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::ShaderReadOnlyTransition => "shader-read-transition",
            Self::GeneralNoTransition => "general-no-transition",
        }
    }

    pub(crate) const fn needs_transition(self) -> bool {
        matches!(self, Self::ShaderReadOnlyTransition)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraPipelinePreset {
    #[default]
    Manual,
    ProjectedSrgb,
    RawFeedUnorm,
    ProjectedUnorm,
    RawFeedSrgb,
    ShaderDecodeUnorm,
    SeparateDecodeUnorm,
    RawProjectionUnorm,
    ProjectionAreaDiagnosticUnorm,
    DisplayEyeUvFiducialUnorm,
    ProjectionContentUvFiducialUnorm,
    SourceSamplingWitnessUnorm,
}

impl CameraPipelinePreset {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "manual" => Some(Self::Manual),
            "projected-srgb" => Some(Self::ProjectedSrgb),
            "raw-feed-unorm" => Some(Self::RawFeedUnorm),
            "projected-unorm" => Some(Self::ProjectedUnorm),
            "raw-feed-srgb" => Some(Self::RawFeedSrgb),
            "shader-decode-unorm" => Some(Self::ShaderDecodeUnorm),
            "separate-decode-unorm" => Some(Self::SeparateDecodeUnorm),
            "raw-projection-unorm"
            | "raw-projection-fast-unorm"
            | "direct-raw-projection-unorm"
            | "fast-raw-unorm"
            | "raw-projection-solid-red-unorm"
            | "raw-projection-red-border-unorm"
            | "direct-raw-projection-solid-red-unorm"
            | "fast-raw-solid-red-unorm"
            | "raw-projection-invalid-fill-unorm"
            | "raw-projection-invalid-only-fill-unorm"
            | "direct-raw-projection-invalid-fill-unorm"
            | "fast-raw-invalid-fill-unorm"
            | "raw-projection-fill-unorm"
            | "raw-projection-coverage-fill-unorm"
            | "raw-projection-fast-fill-unorm"
            | "direct-raw-projection-fill-unorm"
            | "fast-raw-fill-unorm"
            | "raw-projection-perimeter-fill-unorm"
            | "raw-projection-rim-fill-unorm"
            | "direct-raw-projection-perimeter-fill-unorm"
            | "fast-raw-perimeter-fill-unorm"
            | "raw-projection-soft-border-unorm"
            | "raw-projection-cheap-border-unorm"
            | "direct-raw-projection-soft-border-unorm"
            | "fast-raw-soft-border-unorm"
            | "raw-projection-strong-border-unorm"
            | "raw-projection-strong-cheap-border-unorm"
            | "direct-raw-projection-strong-border-unorm"
            | "fast-raw-strong-border-unorm"
            | "raw-projection-dynamic-border-unorm"
            | "raw-projection-feedback-border-unorm"
            | "direct-raw-projection-dynamic-border-unorm"
            | "fast-raw-dynamic-border-unorm"
            | "raw-projection-warm-border-unorm"
            | "raw-projection-warm-feedback-border-unorm"
            | "direct-raw-projection-warm-border-unorm"
            | "fast-raw-warm-border-unorm"
            | "raw-projection-cycling-border-unorm"
            | "raw-projection-cycle-border-unorm"
            | "raw-projection-spectral-border-unorm"
            | "direct-raw-projection-cycling-border-unorm"
            | "fast-raw-cycling-border-unorm"
            | "raw-projection-underlay-unorm"
            | "raw-projection-alpha-underlay-unorm"
            | "direct-raw-projection-underlay-unorm"
            | "fast-raw-underlay-unorm"
            | "raw-projection-camera-footprint-underlay-unorm"
            | "raw-projection-projection-area-bounded-underlay-unorm"
            | "raw-projection-bounded-footprint-underlay-unorm"
            | "camera-footprint-underlay-unorm"
            | "projection-area-bounded-underlay-unorm" => Some(Self::RawProjectionUnorm),
            "projection-area-diagnostic-unorm" => Some(Self::ProjectionAreaDiagnosticUnorm),
            "display-eye-uv-fiducial-unorm" => Some(Self::DisplayEyeUvFiducialUnorm),
            "projection-content-uv-fiducial-unorm" => Some(Self::ProjectionContentUvFiducialUnorm),
            "source-sampling-witness-unorm" => Some(Self::SourceSamplingWitnessUnorm),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::ProjectedSrgb => "projected-srgb",
            Self::RawFeedUnorm => "raw-feed-unorm",
            Self::ProjectedUnorm => "projected-unorm",
            Self::RawFeedSrgb => "raw-feed-srgb",
            Self::ShaderDecodeUnorm => "shader-decode-unorm",
            Self::SeparateDecodeUnorm => "separate-decode-unorm",
            Self::RawProjectionUnorm => "raw-projection-unorm",
            Self::ProjectionAreaDiagnosticUnorm => "projection-area-diagnostic-unorm",
            Self::DisplayEyeUvFiducialUnorm => "display-eye-uv-fiducial-unorm",
            Self::ProjectionContentUvFiducialUnorm => "projection-content-uv-fiducial-unorm",
            Self::SourceSamplingWitnessUnorm => "source-sampling-witness-unorm",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraOrientationDiagnosticMode {
    #[default]
    Off,
    CycleSourceEyeMapping,
    CycleLeftTextureTransform,
    CycleRightTextureTransform,
    CycleAll,
}

impl CameraOrientationDiagnosticMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "off" | "none" => Some(Self::Off),
            "cycle-source-eye-mapping" | "cycleSourceEyeMapping" | "cycle-source" => {
                Some(Self::CycleSourceEyeMapping)
            }
            "cycle-left-texture-transform" | "cycleLeftTextureTransform" | "cycle-left" => {
                Some(Self::CycleLeftTextureTransform)
            }
            "cycle-right-texture-transform" | "cycleRightTextureTransform" | "cycle-right" => {
                Some(Self::CycleRightTextureTransform)
            }
            "cycle-all" | "cycleAll" | "cycle" => Some(Self::CycleAll),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::CycleSourceEyeMapping => "cycle-source-eye-mapping",
            Self::CycleLeftTextureTransform => "cycle-left-texture-transform",
            Self::CycleRightTextureTransform => "cycle-right-texture-transform",
            Self::CycleAll => "cycle-all",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StereoProjectionControls {
    pub(crate) source_eye_mapping: StereoSourceEyeMapping,
    pub(crate) left_texture_transform: CameraTextureTransform,
    pub(crate) right_texture_transform: CameraTextureTransform,
    pub(crate) diagnostic_mode: CameraOrientationDiagnosticMode,
    pub(crate) diagnostic_step: u32,
}

impl StereoProjectionControls {
    pub(crate) fn packed_shader_flags(&self) -> u32 {
        let left = self.left_texture_transform.shader_flags() & 0x1f;
        let right = self.right_texture_transform.shader_flags() & 0x1f;
        left | (right << 5) | ((self.source_eye_mapping.swaps_display_source_eyes() as u32) << 10)
    }

    pub(crate) fn left_label(&self) -> String {
        self.left_texture_transform.label()
    }

    pub(crate) fn right_label(&self) -> String {
        self.right_texture_transform.label()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CameraFrameAdoptionMode {
    #[default]
    Off,
    HoldUntilSmooth,
}

impl CameraFrameAdoptionMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "disabled" => Some(Self::Off),
            "hold-until-smooth" | "hold_until_smooth" | "hold" => Some(Self::HoldUntilSmooth),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::HoldUntilSmooth => "hold-until-smooth",
        }
    }

    pub(crate) const fn is_active(self) -> bool {
        matches!(self, Self::HoldUntilSmooth)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ProjectionTargetJoystickControls {
    #[default]
    Off,
    OffsetScale,
}

impl ProjectionTargetJoystickControls {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "" | "off" | "false" | "0" | "disabled" | "none" => Some(Self::Off),
            "offset-scale"
            | "target-offset-scale"
            | "projection-target"
            | "target-footprint"
            | "joystick-offset-scale"
            | "on"
            | "true"
            | "1"
            | "enabled" => Some(Self::OffsetScale),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::OffsetScale => "offset-scale",
        }
    }

    pub(crate) const fn enabled(self) -> bool {
        matches!(self, Self::OffsetScale)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct RuntimeConfig {
    pub(crate) camera_tier: CameraCompositeTier,
    pub(crate) camera_acquisition: String,
    pub(crate) camera_enabled: bool,
    pub(crate) media_projection_enabled: bool,
    pub(crate) allow_cpu_fallback: bool,
    pub(crate) cpu_upload_hz: u32,
    pub(crate) stereo_layout: StereoMediaLayout,
    pub(crate) projection_runtime_resolution_enabled: bool,
    pub(crate) camera_projection_fov_y_degrees: f32,
    pub(crate) camera_preview_fov_y_degrees: f32,
    pub(crate) camera_preview_offset_y_meters: f32,
    pub(crate) camera_projection_scale: f32,
    pub(crate) camera_projection_depth_meters: f32,
    pub(crate) camera_projection_area_scale_uv: f32,
    pub(crate) camera_projection_area_offset_x_uv: f32,
    pub(crate) camera_projection_area_offset_y_uv: f32,
    pub(crate) camera_projection_area_left_offset_x_uv: f32,
    pub(crate) camera_projection_area_left_offset_y_uv: f32,
    pub(crate) camera_projection_area_right_offset_x_uv: f32,
    pub(crate) camera_projection_area_right_offset_y_uv: f32,
    pub(crate) camera_projection_area_radius_x_uv: f32,
    pub(crate) camera_projection_area_radius_y_uv: f32,
    pub(crate) camera_projection_area_corner_radius_uv: f32,
    pub(crate) camera_projection_area_opacity: f32,
    pub(crate) camera_projection_border_opacity: f32,
    pub(crate) projection_target_offset_x_uv: f32,
    pub(crate) projection_target_offset_y_uv: f32,
    pub(crate) projection_target_scale: f32,
    pub(crate) projection_target_joystick_controls: ProjectionTargetJoystickControls,
    pub(crate) camera_projection_alpha_mode: CameraProjectionAlphaMode,
    pub(crate) camera_projection_alpha_scale: f32,
    pub(crate) camera_projection_alpha_bias: f32,
    pub(crate) camera_raw_overlay_overscan: f32,
    pub(crate) camera_full_view_overlay_overscan: f32,
    pub(crate) camera_edge_fade: f32,
    pub(crate) camera_texture_transform: CameraTextureTransform,
    pub(crate) left_camera_texture_transform: CameraTextureTransform,
    pub(crate) right_camera_texture_transform: CameraTextureTransform,
    pub(crate) source_eye_mapping: StereoSourceEyeMapping,
    pub(crate) camera_projection_mode: CameraProjectionMode,
    pub(crate) camera_pipeline_preset: CameraPipelinePreset,
    pub(crate) camera_projection_effect_mode: CameraProjectionEffectMode,
    pub(crate) camera_projection_border_policy: CameraProjectionBorderPolicy,
    pub(crate) camera_processing_layer: CameraProcessingLayer,
    pub(crate) camera_peripheral_stretch: CameraPeripheralStretchConfig,
    pub(crate) camera_feed_pipeline_mode: CameraFeedPipelineMode,
    pub(crate) camera_color_mode: CameraColorMode,
    pub(crate) camera_sampler_binding_mode: CameraSamplerBindingMode,
    pub(crate) camera_import_image_layout_mode: CameraImportImageLayoutMode,
    pub(crate) camera_import_cache_limit: usize,
    pub(crate) camera_color_matrix: [[f32; 3]; 3],
    pub(crate) camera_color_offset: [f32; 3],
    pub(crate) camera_color_contrast: f32,
    pub(crate) camera_color_brightness: f32,
    pub(crate) camera_color_saturation: f32,
    pub(crate) camera_blur_radius_px: f32,
    pub(crate) camera_temporal_projection_enabled: bool,
    pub(crate) camera_temporal_mode: TemporalProjectionMode,
    pub(crate) camera_temporal_max_pixels_per_frame: f32,
    pub(crate) camera_temporal_max_angular_degrees_per_frame: f32,
    pub(crate) camera_temporal_max_linear_meters_per_frame: f32,
    pub(crate) camera_temporal_catchup_half_life_ms: f32,
    pub(crate) camera_temporal_max_visual_lag_ms: f32,
    pub(crate) camera_temporal_stereo_lockstep: bool,
    pub(crate) camera_temporal_edge_mode: TemporalProjectionEdgeMode,
    pub(crate) camera_frame_adoption_mode: CameraFrameAdoptionMode,
    pub(crate) camera_frame_adoption_max_jump_px: f32,
    pub(crate) camera_frame_adoption_max_hold_ms: f32,
    pub(crate) orientation_diagnostic_mode: CameraOrientationDiagnosticMode,
    pub(crate) visual_release_accepted: bool,
    pub(crate) xr_render_scale: f32,
    pub(crate) xr_display_refresh_hz: f32,
    pub(crate) xr_fixed_foveation_level: u8,
    pub(crate) xr_color_format_mode: OpenXrColorFormatMode,
    pub(crate) environment_depth_mode: EnvironmentDepthMode,
    pub(crate) environment_depth_hand_removal: bool,
    pub(crate) hand_particle_mode: HandParticleMode,
    pub(crate) openxr_passthrough_probe: OpenXrPassthroughProbeMode,
    pub(crate) passthrough_style_mode: OpenXrPassthroughStyleMode,
    pub(crate) passthrough_opacity: f32,
    pub(crate) passthrough_edge_color: [f32; 4],
    pub(crate) passthrough_brightness: f32,
    pub(crate) passthrough_contrast: f32,
    pub(crate) passthrough_saturation: f32,
    pub(crate) passthrough_color_phase: f32,
    pub(crate) passthrough_color_amplitude: f32,
    pub(crate) passthrough_lut_resolution: u32,
    pub(crate) passthrough_lut_weight: f32,
    pub(crate) passthrough_lut_flicker_hz: f32,
    pub(crate) full_field_flicker_hz: f32,
    pub(crate) projection_layer_visible: bool,
    pub(crate) diagnostic_hud_visible: bool,
    pub(crate) osc_enabled: bool,
    pub(crate) osc_overlay_enabled: bool,
    pub(crate) osc_listen_addr: String,
    pub(crate) osc_max_packet_bytes: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            camera_tier: CameraCompositeTier::CpuDiagnosticFlatCopy,
            camera_acquisition: "java-camera2".to_string(),
            camera_enabled: true,
            media_projection_enabled: false,
            allow_cpu_fallback: true,
            cpu_upload_hz: 4,
            stereo_layout: StereoMediaLayout::Mono,
            projection_runtime_resolution_enabled: false,
            camera_projection_fov_y_degrees: 92.0,
            camera_preview_fov_y_degrees: 60.0,
            camera_preview_offset_y_meters: 0.0,
            camera_projection_scale: 1.0,
            camera_projection_depth_meters: 1.0,
            camera_projection_area_scale_uv: 1.0,
            camera_projection_area_offset_x_uv: 0.0,
            camera_projection_area_offset_y_uv: 0.0,
            camera_projection_area_left_offset_x_uv: 0.0,
            camera_projection_area_left_offset_y_uv: 0.0,
            camera_projection_area_right_offset_x_uv: 0.0,
            camera_projection_area_right_offset_y_uv: 0.0,
            camera_projection_area_radius_x_uv: 0.5,
            camera_projection_area_radius_y_uv: 0.5,
            camera_projection_area_corner_radius_uv: 0.0,
            camera_projection_area_opacity: 1.0,
            camera_projection_border_opacity: 1.0,
            projection_target_offset_x_uv: 0.0,
            projection_target_offset_y_uv: 0.0,
            projection_target_scale: 1.0,
            projection_target_joystick_controls: ProjectionTargetJoystickControls::Off,
            camera_projection_alpha_mode: CameraProjectionAlphaMode::default(),
            camera_projection_alpha_scale: 1.0,
            camera_projection_alpha_bias: 0.0,
            camera_raw_overlay_overscan: 1.06,
            camera_full_view_overlay_overscan: 2.10,
            camera_edge_fade: 0.12,
            camera_texture_transform: CameraTextureTransform::default(),
            left_camera_texture_transform: CameraTextureTransform::default(),
            right_camera_texture_transform: CameraTextureTransform::default(),
            source_eye_mapping: StereoSourceEyeMapping::default(),
            camera_projection_mode: CameraProjectionMode::default(),
            camera_pipeline_preset: CameraPipelinePreset::default(),
            camera_projection_effect_mode: CameraProjectionEffectMode::default(),
            camera_projection_border_policy: CameraProjectionBorderPolicy::default(),
            camera_processing_layer: CameraProcessingLayer::default(),
            camera_peripheral_stretch: CameraPeripheralStretchConfig::default(),
            camera_feed_pipeline_mode: CameraFeedPipelineMode::default(),
            camera_color_mode: CameraColorMode::default(),
            camera_sampler_binding_mode: CameraSamplerBindingMode::default(),
            camera_import_image_layout_mode: CameraImportImageLayoutMode::default(),
            camera_import_cache_limit: CAMERA_IMPORT_CACHE_LIMIT_DEFAULT,
            camera_color_matrix: identity_color_matrix(),
            camera_color_offset: [0.0, 0.0, 0.0],
            camera_color_contrast: 1.0,
            camera_color_brightness: 0.0,
            camera_color_saturation: 1.0,
            camera_blur_radius_px: 2.0,
            camera_temporal_projection_enabled: false,
            camera_temporal_mode: TemporalProjectionMode::Off,
            camera_temporal_max_pixels_per_frame: 18.0,
            camera_temporal_max_angular_degrees_per_frame: 1.25,
            camera_temporal_max_linear_meters_per_frame: 0.012,
            camera_temporal_catchup_half_life_ms: 50.0,
            camera_temporal_max_visual_lag_ms: 120.0,
            camera_temporal_stereo_lockstep: true,
            camera_temporal_edge_mode: TemporalProjectionEdgeMode::None,
            camera_frame_adoption_mode: CameraFrameAdoptionMode::Off,
            camera_frame_adoption_max_jump_px: 24.0,
            camera_frame_adoption_max_hold_ms: 80.0,
            orientation_diagnostic_mode: CameraOrientationDiagnosticMode::Off,
            visual_release_accepted: false,
            xr_render_scale: 1.0,
            xr_display_refresh_hz: 72.0,
            xr_fixed_foveation_level: 0,
            xr_color_format_mode: OpenXrColorFormatMode::default(),
            environment_depth_mode: EnvironmentDepthMode::default(),
            environment_depth_hand_removal: false,
            hand_particle_mode: HandParticleMode::default(),
            openxr_passthrough_probe: OpenXrPassthroughProbeMode::Off,
            passthrough_style_mode: OpenXrPassthroughStyleMode::default(),
            passthrough_opacity: 1.0,
            passthrough_edge_color: [0.0, 0.0, 0.0, 0.0],
            passthrough_brightness: 0.0,
            passthrough_contrast: 1.0,
            passthrough_saturation: 1.0,
            passthrough_color_phase: 0.0,
            passthrough_color_amplitude: 0.0,
            passthrough_lut_resolution: 32,
            passthrough_lut_weight: 1.0,
            passthrough_lut_flicker_hz: 0.0,
            full_field_flicker_hz: 0.0,
            projection_layer_visible: true,
            diagnostic_hud_visible: false,
            osc_enabled: false,
            osc_overlay_enabled: true,
            osc_listen_addr: "0.0.0.0:9000".to_string(),
            osc_max_packet_bytes: 8192,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum EnvironmentDepthMode {
    #[default]
    Off,
    Status,
    Visualize,
    MeshOverlay,
    ParticleOverlay,
    SceneParticleMap,
}

impl EnvironmentDepthMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "false" | "0" | "disabled" | "none" => Some(Self::Off),
            "status" | "diagnostic" | "diagnostics" | "on" | "true" | "1" => Some(Self::Status),
            "visualize" | "visualise" | "visual" | "debug-visual" => Some(Self::Visualize),
            "mesh" | "mesh-overlay" | "depth-mesh" | "debug-mesh" | "wire-mesh" => {
                Some(Self::MeshOverlay)
            }
            "particles" | "particle-overlay" | "depth-particles" | "surface-particles" => {
                Some(Self::ParticleOverlay)
            }
            "scene-particle-map"
            | "scene-particles"
            | "spatial-particles"
            | "surface-particle-map"
            | "depth-scene-particles" => Some(Self::SceneParticleMap),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Status => "status",
            Self::Visualize => "visualize",
            Self::MeshOverlay => "mesh-overlay",
            Self::ParticleOverlay => "particle-overlay",
            Self::SceneParticleMap => "scene-particle-map",
        }
    }

    pub(crate) const fn enabled(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub(crate) const fn visualizes(self) -> bool {
        matches!(
            self,
            Self::Visualize | Self::MeshOverlay | Self::ParticleOverlay | Self::SceneParticleMap
        )
    }

    pub(crate) const fn mesh_overlay(self) -> bool {
        matches!(self, Self::MeshOverlay)
    }

    pub(crate) const fn particle_overlay(self) -> bool {
        matches!(self, Self::ParticleOverlay | Self::SceneParticleMap)
    }

    pub(crate) const fn scene_particle_map(self) -> bool {
        matches!(self, Self::SceneParticleMap)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum HandParticleMode {
    #[default]
    Off,
    Meta,
}

impl HandParticleMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "false" | "0" | "disabled" | "none" => Some(Self::Off),
            "hand-mesh" | "on" | "true" | "1" | "meta" | "meta-hand" | "meta-hand-mesh"
            | "openxr" | "openxr-hand" | "openxr-hand-mesh" | "runtime" | "runtime-hand"
            | "runtime-hand-mesh" | "real" | "real-hand" | "real-hand-mesh" => Some(Self::Meta),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Meta => "meta",
        }
    }

    pub(crate) const fn enabled(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub(crate) const fn uses_openxr_hand_mesh(self) -> bool {
        matches!(self, Self::Meta)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum OpenXrPassthroughProbeMode {
    #[default]
    Off,
    Client,
    Warmup,
    Underlay,
}

impl OpenXrPassthroughProbeMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "false" | "0" | "disabled" | "none" => Some(Self::Off),
            "client" | "true" | "1" | "enabled" | "probe" => Some(Self::Client),
            "warmup" | "pulse" | "brief" => Some(Self::Warmup),
            "underlay" | "visible-underlay" | "composition-underlay" | "background" => {
                Some(Self::Underlay)
            }
            _ => None,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Client => "client",
            Self::Warmup => "warmup",
            Self::Underlay => "underlay",
        }
    }

    pub(crate) fn enabled(self) -> bool {
        self != Self::Off
    }

    pub(crate) fn submits_composition_layer(self) -> bool {
        self == Self::Underlay
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum OpenXrPassthroughStyleMode {
    #[default]
    None,
    BrightnessContrastSaturation,
    MonoToRgba,
    ColorLut,
}

impl OpenXrPassthroughStyleMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" | "native" | "off" | "false" | "0" => Some(Self::None),
            "bcs" | "brightness-contrast-saturation" | "brightness" => {
                Some(Self::BrightnessContrastSaturation)
            }
            "mono-to-rgba" | "mono-rgba" | "rgba-map" | "gradient" | "audio-gradient" => {
                Some(Self::MonoToRgba)
            }
            "color-lut" | "rgb-lut" | "lut" | "opponent-lut" | "lut-flicker" => {
                Some(Self::ColorLut)
            }
            _ => None,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::BrightnessContrastSaturation => "brightness-contrast-saturation",
            Self::MonoToRgba => "mono-to-rgba",
            Self::ColorLut => "color-lut",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum CameraProjectionAlphaMode {
    #[default]
    Fixed,
    Red,
    Green,
    Blue,
    Luma,
    InverseRed,
    InverseGreen,
    InverseBlue,
    InverseLuma,
    RedDominance,
    GreenDominance,
    BlueDominance,
    Saturation,
    InverseSaturation,
}

impl CameraProjectionAlphaMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "fixed" | "none" | "constant" | "area-opacity" | "opacity" => Some(Self::Fixed),
            "red" | "r" | "channel-r" => Some(Self::Red),
            "green" | "g" | "channel-g" => Some(Self::Green),
            "blue" | "b" | "channel-b" => Some(Self::Blue),
            "luma" | "luminance" | "brightness" | "value" => Some(Self::Luma),
            "inverse-red" | "red-inverse" | "inv-red" | "one-minus-red" | "1-red" | "1-r" => {
                Some(Self::InverseRed)
            }
            "inverse-green" | "green-inverse" | "inv-green" | "one-minus-green" | "1-green"
            | "1-g" => Some(Self::InverseGreen),
            "inverse-blue" | "blue-inverse" | "inv-blue" | "one-minus-blue" | "1-blue" | "1-b" => {
                Some(Self::InverseBlue)
            }
            "inverse-luma" | "luma-inverse" | "inv-luma" | "inverse-brightness"
            | "one-minus-luma" | "1-luma" | "1-brightness" => Some(Self::InverseLuma),
            "red-dominance" | "dominant-red" | "red-key" | "red-chroma" | "red-minus-max" => {
                Some(Self::RedDominance)
            }
            "green-dominance" | "dominant-green" | "green-key" | "green-chroma"
            | "green-minus-max" | "screen-green" => Some(Self::GreenDominance),
            "blue-dominance" | "dominant-blue" | "blue-key" | "blue-chroma" | "blue-minus-max" => {
                Some(Self::BlueDominance)
            }
            "saturation" | "chroma" | "max-min" | "colorfulness" => Some(Self::Saturation),
            "inverse-saturation"
            | "saturation-inverse"
            | "inverse-chroma"
            | "inv-chroma"
            | "one-minus-saturation"
            | "1-saturation" => Some(Self::InverseSaturation),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Luma => "luma",
            Self::InverseRed => "inverse-red",
            Self::InverseGreen => "inverse-green",
            Self::InverseBlue => "inverse-blue",
            Self::InverseLuma => "inverse-luma",
            Self::RedDominance => "red-dominance",
            Self::GreenDominance => "green-dominance",
            Self::BlueDominance => "blue-dominance",
            Self::Saturation => "saturation",
            Self::InverseSaturation => "inverse-saturation",
        }
    }

    pub(crate) const fn shader_code(self) -> f32 {
        match self {
            Self::Fixed => 0.0,
            Self::Red => 1.0,
            Self::Green => 2.0,
            Self::Blue => 3.0,
            Self::Luma => 4.0,
            Self::InverseRed => 5.0,
            Self::InverseGreen => 6.0,
            Self::InverseBlue => 7.0,
            Self::InverseLuma => 8.0,
            Self::RedDominance => 9.0,
            Self::GreenDominance => 10.0,
            Self::BlueDominance => 11.0,
            Self::Saturation => 12.0,
            Self::InverseSaturation => 13.0,
        }
    }

    pub(crate) const fn uses_dynamic_alpha(self) -> bool {
        !matches!(self, Self::Fixed)
    }
}

impl RuntimeConfig {
    pub(crate) fn camera_color_adjust_push(&self) -> [f32; 4] {
        [
            self.camera_color_contrast.max(0.0),
            self.camera_color_brightness,
            self.camera_color_saturation.max(0.0),
            if self.camera_projection_mode.uses_world_canvas() {
                2.0
            } else {
                1.0
            },
        ]
    }

    pub(crate) fn hwb_source_color_contract_fields(&self) -> String {
        let swapchain_encoding = match self.xr_color_format_mode {
            OpenXrColorFormatMode::Rgba8Srgb => "srgb",
            OpenXrColorFormatMode::Rgba8Unorm => "linear-or-runtime-default",
        };
        format!(
            "sourceColorInputEncoding={} sourceColorTransformStage=post_hardware_buffer_sample_pre_camera_color_controls sourceColorTransform={} sourceColorTransformOwner=vulkan-hwb-camera_projection_shader sourceColorTransformApplied={} sourceColorOutputEncoding={} cameraColorControlStage=post_source_color_transfer swapchainColorFormat={} swapchainColorEncoding={}",
            self.camera_color_mode.source_color_input_encoding(),
            self.camera_color_mode.source_color_transform(),
            self.camera_color_mode.source_color_transform_applied(),
            self.camera_color_mode.source_color_output_encoding(),
            self.xr_color_format_mode.stable_id(),
            swapchain_encoding
        )
    }

    pub(crate) fn hwb_peripheral_stretch_contract_fields(&self) -> String {
        let peripheral_stretch = self.camera_peripheral_stretch.sanitized();
        format!(
            "peripheralStretchMode={} peripheralStretchCoreScale={:.3} peripheralStretchEdgeInsetUv={:.3} peripheralStretchMaxInsetUv={:.3} peripheralStretchCurve={:.3} peripheralStretchInnerBlendUv={:.3} peripheralStretchBlendCurve={:.3} peripheralStretchBlendMode={} peripheralStretchCornerMode={} peripheralStretchDebug={} peripheralStretchConsumesProjectionExterior={} peripheralStretchCoreRegion=target-footprint-minus-inner-transition-band peripheralStretchTransitionRegion=target-footprint-inner-edge-band peripheralStretchExteriorRegion=visible-render-surface-minus-target-footprint peripheralStretchTransitionSpace=target-local-raster-uv peripheralStretchTransitionSemantics=canonical-sample-to-stretch-sample-remap peripheralStretchBorderSource=projection-edge-sample peripheralStretchExteriorSource=target-edge-sample",
            peripheral_stretch.mode.stable_id(),
            peripheral_stretch.core_scale,
            peripheral_stretch.edge_inset_uv,
            peripheral_stretch.max_inset_uv,
            peripheral_stretch.curve,
            peripheral_stretch.inner_blend_uv,
            peripheral_stretch.blend_curve,
            peripheral_stretch.blend_mode.stable_id(),
            peripheral_stretch.corner_mode.stable_id(),
            peripheral_stretch.debug.stable_id(),
            self.camera_processing_layer.consumes_projection_exterior()
        )
    }

    pub(crate) fn hwb_projection_target_contract_fields(&self) -> String {
        format!(
            "projectionTargetOffsetXUv={:.4} projectionTargetOffsetYUv={:.4} projectionTargetScale={:.4} projectionTargetJoystickControls={} projectionTargetControlCoordinateSpace=display-eye-screen-uv projectionTargetControlSemantics=runtime_adjustment_applied_after_source_metadata",
            self.projection_target_offset_x_uv,
            self.projection_target_offset_y_uv,
            self.projection_target_scale,
            self.projection_target_joystick_controls.stable_id()
        )
    }

    pub(crate) fn camera_effect_params_push(&self) -> [f32; 4] {
        let processing_diagnostic = self.camera_processing_layer.diagnostic_shader_code();
        [
            self.camera_blur_radius_px.clamp(0.0, 16.0),
            self.camera_projection_area_opacity.clamp(0.0, 1.0),
            self.camera_projection_border_opacity.clamp(0.0, 1.0),
            if processing_diagnostic > 0.0 {
                processing_diagnostic
            } else {
                self.camera_projection_effect_mode.diagnostic_shader_code()
            },
        ]
    }

    pub(crate) fn camera_peripheral_stretch_params_push(&self) -> [f32; 4] {
        let peripheral_stretch = self.camera_peripheral_stretch.sanitized();
        [
            peripheral_stretch.core_scale,
            peripheral_stretch.edge_inset_uv,
            peripheral_stretch.max_inset_uv,
            peripheral_stretch.curve,
        ]
    }

    pub(crate) fn camera_peripheral_stretch_blend_params_push(&self) -> [f32; 4] {
        let peripheral_stretch = self.camera_peripheral_stretch.sanitized();
        [
            peripheral_stretch.inner_blend_uv,
            peripheral_stretch.blend_curve,
            peripheral_stretch.blend_mode.shader_code(),
            0.0,
        ]
    }

    pub(crate) fn camera_alpha_params_push(&self) -> [f32; 4] {
        [
            self.camera_projection_alpha_mode.shader_code(),
            self.camera_projection_alpha_scale.clamp(0.0, 16.0),
            self.camera_projection_alpha_bias.clamp(-1.0, 1.0),
            self.camera_peripheral_stretch
                .sanitized()
                .debug
                .shader_code(),
        ]
    }

    pub(crate) fn camera_area_offset_params_push(&self) -> [f32; 4] {
        [
            self.camera_projection_area_left_offset_x_uv
                .clamp(-0.5, 0.5),
            self.camera_projection_area_left_offset_y_uv
                .clamp(-0.5, 0.5),
            self.camera_projection_area_right_offset_x_uv
                .clamp(-0.5, 0.5),
            self.camera_projection_area_right_offset_y_uv
                .clamp(-0.5, 0.5),
        ]
    }

    pub(crate) fn camera_projection_area_offset_for_eye(&self, eye_index: usize) -> [f32; 2] {
        if eye_index == 0 {
            [
                self.camera_projection_area_left_offset_x_uv
                    .clamp(-0.5, 0.5),
                self.camera_projection_area_left_offset_y_uv
                    .clamp(-0.5, 0.5),
            ]
        } else {
            [
                self.camera_projection_area_right_offset_x_uv
                    .clamp(-0.5, 0.5),
                self.camera_projection_area_right_offset_y_uv
                    .clamp(-0.5, 0.5),
            ]
        }
    }

    pub(crate) fn camera_area_params_push(&self) -> [f32; 4] {
        [
            self.camera_projection_area_radius_x_uv.clamp(0.05, 0.5),
            self.camera_projection_area_radius_y_uv.clamp(0.05, 0.5),
            self.camera_projection_area_corner_radius_uv.clamp(0.0, 0.5),
            self.camera_projection_area_scale_uv.clamp(0.05, 4.0),
        ]
    }

    pub(crate) fn camera_projection_border_policy_active(&self) -> bool {
        self.camera_projection_effect_mode
            .uses_projection_border_policy()
    }

    pub(crate) fn camera_projection_border_policy_shader_bit(&self) -> u32 {
        if self.camera_projection_border_policy_active() {
            self.camera_projection_border_policy.shader_bit()
        } else {
            0
        }
    }

    pub(crate) fn camera_projection_border_policy_requires_full_pipeline(&self) -> bool {
        self.camera_projection_border_policy_active()
            && self
                .camera_projection_border_policy
                .requires_full_projection_pipeline()
    }

    #[cfg(target_os = "android")]
    pub(crate) fn projection_layer_needs_source_alpha(&self) -> bool {
        self.openxr_passthrough_probe.submits_composition_layer()
            || (self.camera_projection_border_policy_active()
                && self
                    .camera_projection_border_policy
                    .uses_passthrough_underlay_alpha())
            || self.camera_projection_area_opacity < 0.999
            || self.camera_projection_border_opacity < 0.999
            || self.camera_projection_alpha_mode.uses_dynamic_alpha()
    }

    pub(crate) fn stereo_projection_controls(&self, frame_count: u64) -> StereoProjectionControls {
        let mut controls = StereoProjectionControls {
            source_eye_mapping: self.source_eye_mapping,
            left_texture_transform: self.left_camera_texture_transform.clone(),
            right_texture_transform: self.right_camera_texture_transform.clone(),
            diagnostic_mode: self.orientation_diagnostic_mode,
            diagnostic_step: 0,
        };
        if controls.left_texture_transform == CameraTextureTransform::default() {
            controls.left_texture_transform = self.camera_texture_transform.clone();
        }
        if controls.right_texture_transform == CameraTextureTransform::default() {
            controls.right_texture_transform = self.camera_texture_transform.clone();
        }

        if self.orientation_diagnostic_mode == CameraOrientationDiagnosticMode::Off {
            return controls;
        }

        let step = ((frame_count / 180) % 32) as u32;
        controls.diagnostic_step = step;
        match self.orientation_diagnostic_mode {
            CameraOrientationDiagnosticMode::Off => {}
            CameraOrientationDiagnosticMode::CycleSourceEyeMapping => {
                controls.source_eye_mapping = if step.is_multiple_of(2) {
                    StereoSourceEyeMapping::DisplayLeftFromLeftSource
                } else {
                    StereoSourceEyeMapping::DisplayLeftFromRightSource
                };
            }
            CameraOrientationDiagnosticMode::CycleLeftTextureTransform => {
                controls.left_texture_transform = diagnostic_texture_transform(step);
            }
            CameraOrientationDiagnosticMode::CycleRightTextureTransform => {
                controls.right_texture_transform = diagnostic_texture_transform(step);
            }
            CameraOrientationDiagnosticMode::CycleAll => {
                controls.source_eye_mapping = if step.is_multiple_of(2) {
                    StereoSourceEyeMapping::DisplayLeftFromLeftSource
                } else {
                    StereoSourceEyeMapping::DisplayLeftFromRightSource
                };
                controls.left_texture_transform = diagnostic_texture_transform(step / 2);
                controls.right_texture_transform = diagnostic_texture_transform((step / 2) + 8);
            }
        }
        controls
    }
}

pub(crate) const fn identity_color_matrix() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

fn diagnostic_texture_transform(step: u32) -> CameraTextureTransform {
    let rotation = match step & 3 {
        1 => CameraImageRotation::Rotate90,
        2 => CameraImageRotation::Rotate180,
        3 => CameraImageRotation::Rotate270,
        _ => CameraImageRotation::Rotate0,
    };
    CameraTextureTransform::new(
        "runtime-orientation-diagnostic-cycle",
        format!("diagnostic orientation cycle step {step}"),
    )
    .with_rotation(rotation)
    .with_flip_x((step & 4) != 0)
    .with_flip_y((step & 8) != 0)
    .with_mirror((step & 16) != 0)
}

#[derive(Default)]
struct HeadsetCameraState {
    latest: Option<HeadsetCameraFrame>,
    latest_gpu: Option<HeadsetCameraGpuFrame>,
    latest_stereo_gpu: Option<StereoGpuCameraFrame>,
    next_index: u64,
    next_gpu_index: u64,
    next_stereo_gpu_index: u64,
    gpu_probe_success_count: u64,
    gpu_probe_failure_count: u64,
    gpu_descriptor_cache_keys: BTreeSet<String>,
    stereo_left_received_count: u64,
    stereo_right_received_count: u64,
    stereo_paired_count: u64,
    stereo_dropped_count: u64,
    stereo_pair_delta_total_ns: u64,
    stereo_pair_delta_max_ns: u64,
}

static HEADSET_CAMERA_STATE: OnceLock<Mutex<HeadsetCameraState>> = OnceLock::new();
static RUNTIME_CONFIG: OnceLock<Mutex<RuntimeConfig>> = OnceLock::new();
static DIAGNOSTIC_HUD_STATE: OnceLock<Mutex<DiagnosticHudState>> = OnceLock::new();

fn headset_camera_state() -> &'static Mutex<HeadsetCameraState> {
    HEADSET_CAMERA_STATE.get_or_init(|| Mutex::new(HeadsetCameraState::default()))
}

fn runtime_config_state() -> &'static Mutex<RuntimeConfig> {
    RUNTIME_CONFIG.get_or_init(|| Mutex::new(RuntimeConfig::default()))
}

fn diagnostic_hud_state() -> &'static Mutex<DiagnosticHudState> {
    DIAGNOSTIC_HUD_STATE.get_or_init(|| {
        Mutex::new(DiagnosticHudState::new(
            RuntimeConfig::default().diagnostic_hud_visible,
        ))
    })
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn latest_headset_camera_frame() -> Option<HeadsetCameraFrame> {
    headset_camera_state()
        .lock()
        .ok()
        .and_then(|state| state.latest.clone())
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn latest_headset_camera_gpu_frame() -> Option<HeadsetCameraGpuFrame> {
    headset_camera_state()
        .lock()
        .ok()
        .and_then(|state| state.latest_gpu.clone())
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn latest_headset_stereo_camera_gpu_frame() -> Option<StereoGpuCameraFrame> {
    headset_camera_state()
        .lock()
        .ok()
        .and_then(|state| state.latest_stereo_gpu.clone())
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn runtime_config() -> RuntimeConfig {
    runtime_config_state()
        .lock()
        .map(|state| state.clone())
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct CameraAlignmentTuningUpdate {
    pub(crate) projection_depth_meters: f32,
    pub(crate) camera_preview_fov_y_degrees: f32,
    pub(crate) camera_preview_offset_y_meters: f32,
    pub(crate) camera_raw_overlay_overscan: f32,
    pub(crate) projection_layer_visible: bool,
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn apply_camera_alignment_tuning(update: CameraAlignmentTuningUpdate) -> RuntimeConfig {
    runtime_config_state()
        .lock()
        .map(|mut state| {
            state.camera_projection_depth_meters = update.projection_depth_meters.clamp(0.05, 10.0);
            state.camera_preview_fov_y_degrees =
                update.camera_preview_fov_y_degrees.clamp(1.0, 175.0);
            state.camera_preview_offset_y_meters =
                update.camera_preview_offset_y_meters.clamp(-2.0, 2.0);
            state.camera_raw_overlay_overscan = update.camera_raw_overlay_overscan.max(1.0);
            state.projection_layer_visible = update.projection_layer_visible;
            state.clone()
        })
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct ProjectionTargetControlUpdate {
    pub(crate) offset_x_uv: f32,
    pub(crate) offset_y_uv: f32,
    pub(crate) scale: f32,
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn apply_projection_target_control_update(
    update: ProjectionTargetControlUpdate,
) -> RuntimeConfig {
    runtime_config_state()
        .lock()
        .map(|mut state| {
            state.projection_target_offset_x_uv = update.offset_x_uv.clamp(-0.5, 0.5);
            state.projection_target_offset_y_uv = update.offset_y_uv.clamp(-0.5, 0.5);
            state.projection_target_scale = update.scale.clamp(0.05, 1.5);
            state.clone()
        })
        .unwrap_or_default()
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn apply_projection_target_scale_update(scale: f32) -> RuntimeConfig {
    runtime_config_state()
        .lock()
        .map(|mut state| {
            if scale.is_finite() {
                state.projection_target_scale = scale.clamp(0.05, 1.5);
            }
            state.clone()
        })
        .unwrap_or_default()
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn diagnostic_hud_snapshot() -> DiagnosticHudUpdate {
    diagnostic_hud_state()
        .lock()
        .map(|state| state.snapshot())
        .unwrap_or_else(|_| {
            DiagnosticHudState::new(RuntimeConfig::default().diagnostic_hud_visible).snapshot()
        })
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn apply_diagnostic_hud_command(
    command: DiagnosticHudCommand,
    source: DiagnosticHudInputSource,
) -> DiagnosticHudUpdate {
    diagnostic_hud_state()
        .lock()
        .map(|mut state| state.apply(command, source))
        .unwrap_or_else(|_| {
            DiagnosticHudState::new(RuntimeConfig::default().diagnostic_hud_visible).snapshot()
        })
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn gpu_probe_counters() -> (u64, u64, usize) {
    headset_camera_state()
        .lock()
        .map(|state| {
            (
                state.gpu_probe_success_count,
                state.gpu_probe_failure_count,
                state.gpu_descriptor_cache_keys.len(),
            )
        })
        .unwrap_or((0, 0, 0))
}

#[cfg(target_os = "android")]
fn parse_runtime_config_json(json: &str) -> Option<JavaRuntimeConfig> {
    match serde_json::from_str::<JavaRuntimeConfig>(json) {
        Ok(parsed) => Some(parsed),
        Err(error) => {
            log_error(format!(
                "Rusty XR could not parse runtime config JSON: {error}"
            ));
            None
        }
    }
}

#[cfg(not(target_os = "android"))]
fn parse_runtime_config_json(json: &str) -> Option<JavaRuntimeConfig> {
    serde_json::from_str::<JavaRuntimeConfig>(json).ok()
}

#[cfg(target_os = "android")]
fn parse_camera_frame_metadata_json(
    json: &str,
    description: &str,
) -> Option<JavaCameraFrameMetadata> {
    match serde_json::from_str::<JavaCameraFrameMetadata>(json) {
        Ok(parsed) => Some(parsed),
        Err(error) => {
            log_error(format!(
                "Rusty XR could not parse {description} metadata JSON: {error}"
            ));
            None
        }
    }
}

#[cfg(not(target_os = "android"))]
fn parse_camera_frame_metadata_json(
    json: &str,
    _description: &str,
) -> Option<JavaCameraFrameMetadata> {
    serde_json::from_str::<JavaCameraFrameMetadata>(json).ok()
}

fn store_runtime_config(config_json: Option<String>) {
    let parsed = config_json.as_deref().and_then(parse_runtime_config_json);
    let config = parsed
        .as_ref()
        .map(public_runtime_config)
        .unwrap_or_default();

    if let Ok(mut state) = runtime_config_state().lock() {
        *state = config.clone();
    }
    sync_diagnostic_hud_state(parsed.as_ref(), &config);

    #[cfg(target_os = "android")]
    log_info(format!(
        "Rusty XR camera path config requestedTier={} cameraAcquisition={} cameraEnabled={} mediaProjection={} allowCpuFallback={} cpuUploadHz={} stereoLayout={:?} projectionMode={} cameraPipelinePreset={} cameraProjectionEffectMode={} projectionBorderPolicy={} processingLayer={} cameraFeedMode={} cameraColorMode={} cameraColorShaderBit={} {} cameraSamplerBindingMode={} cameraImportImageLayout={} cameraImportCacheLimit={} cameraColorMatrix={:?} cameraColorOffset={:?} cameraColorContrast={} cameraColorBrightness={} cameraColorSaturation={} cameraBlurRadiusPx={} {} {} temporalProjectionEnabled={} temporalProjectionMode={} temporalProjectionMaxPixelsPerFrame={} temporalProjectionMaxAngularDegreesPerFrame={} temporalProjectionMaxLinearMetersPerFrame={} temporalProjectionCatchupHalfLifeMs={} temporalProjectionMaxVisualLagMs={} temporalProjectionStereoLockstep={} temporalProjectionEdgeMode={} cameraFrameAdoptionMode={} cameraFrameAdoptionMaxJumpPx={} cameraFrameAdoptionMaxHoldMs={} projectionFovY={} previewFovY={} previewOffsetYMeters={} projectionScale={} projectionDepthMeters={} projectionAreaScaleUv={} projectionAreaOffsetXUv={} projectionAreaOffsetYUv={} projectionAreaLeftOffsetXUv={} projectionAreaLeftOffsetYUv={} projectionAreaRightOffsetXUv={} projectionAreaRightOffsetYUv={} projectionAreaRadiusXUv={} projectionAreaRadiusYUv={} projectionAreaCornerRadiusUv={} projectionAreaOpacity={} projectionBorderOpacity={} projectionAlphaMode={} projectionAlphaScale={} projectionAlphaBias={} rawOverscan={} fullViewOverscan={} edgeFade={} cameraTextureTransform={} leftCameraTextureTransform={} rightCameraTextureTransform={} sourceEyeMapping={} orientationDiagnosticMode={} cameraTextureTransformSource={} cameraTextureTransformReason={} orientationCheck={} visualReleaseAccepted={} xrRenderScale={} xrDisplayRefreshHz={} fixedFoveationLevel={} xrColorFormat={} environmentDepthMode={} environmentDepthHandRemoval={} openxrPassthroughProbe={} passthroughStyleMode={} passthroughOpacity={} passthroughEdgeColor={:?} passthroughBrightness={} passthroughContrast={} passthroughSaturation={} passthroughColorPhase={} passthroughColorAmplitude={} passthroughLutResolution={} passthroughLutWeight={} passthroughLutFlickerHz={} fullFieldFlickerHz={} projectionLayerVisible={} diagnosticHudVisible={}",
        config.camera_tier.stable_id(),
        config.camera_acquisition.as_str(),
        config.camera_enabled,
        config.media_projection_enabled,
        config.allow_cpu_fallback,
        config.cpu_upload_hz,
        config.stereo_layout,
        config.camera_projection_mode.stable_id(),
        config.camera_pipeline_preset.stable_id(),
        config.camera_projection_effect_mode.stable_id(),
        config.camera_projection_border_policy.stable_id(),
        config.camera_processing_layer.stable_id(),
        config.camera_feed_pipeline_mode.stable_id(),
        config.camera_color_mode.stable_id(),
        config.camera_color_mode.shader_bit(),
        config.hwb_source_color_contract_fields(),
        config.camera_sampler_binding_mode.stable_id(),
        config.camera_import_image_layout_mode.stable_id(),
        config.camera_import_cache_limit,
        config.camera_color_matrix,
        config.camera_color_offset,
        config.camera_color_contrast,
        config.camera_color_brightness,
        config.camera_color_saturation,
        config.camera_blur_radius_px,
        config.hwb_peripheral_stretch_contract_fields(),
        config.hwb_projection_target_contract_fields(),
        config.camera_temporal_projection_enabled,
        config.camera_temporal_mode.stable_id(),
        config.camera_temporal_max_pixels_per_frame,
        config.camera_temporal_max_angular_degrees_per_frame,
        config.camera_temporal_max_linear_meters_per_frame,
        config.camera_temporal_catchup_half_life_ms,
        config.camera_temporal_max_visual_lag_ms,
        config.camera_temporal_stereo_lockstep,
        config.camera_temporal_edge_mode.stable_id(),
        config.camera_frame_adoption_mode.stable_id(),
        config.camera_frame_adoption_max_jump_px,
        config.camera_frame_adoption_max_hold_ms,
        config.camera_projection_fov_y_degrees,
        config.camera_preview_fov_y_degrees,
        config.camera_preview_offset_y_meters,
        config.camera_projection_scale,
        config.camera_projection_depth_meters,
        config.camera_projection_area_scale_uv,
        config.camera_projection_area_offset_x_uv,
        config.camera_projection_area_offset_y_uv,
        config.camera_projection_area_left_offset_x_uv,
        config.camera_projection_area_left_offset_y_uv,
        config.camera_projection_area_right_offset_x_uv,
        config.camera_projection_area_right_offset_y_uv,
        config.camera_projection_area_radius_x_uv,
        config.camera_projection_area_radius_y_uv,
        config.camera_projection_area_corner_radius_uv,
        config.camera_projection_area_opacity,
        config.camera_projection_border_opacity,
        config.camera_projection_alpha_mode.stable_id(),
        config.camera_projection_alpha_scale,
        config.camera_projection_alpha_bias,
        config.camera_raw_overlay_overscan,
        config.camera_full_view_overlay_overscan,
        config.camera_edge_fade,
        config.camera_texture_transform.label(),
        config.left_camera_texture_transform.label(),
        config.right_camera_texture_transform.label(),
        config.source_eye_mapping.stable_id(),
        config.orientation_diagnostic_mode.stable_id(),
        config.camera_texture_transform.source_label.as_str(),
        config.camera_texture_transform.reason.as_str(),
        config.camera_texture_transform.is_explicit_visual_check(),
        config.visual_release_accepted,
        config.xr_render_scale,
        config.xr_display_refresh_hz,
        config.xr_fixed_foveation_level,
        config.xr_color_format_mode.stable_id(),
        config.environment_depth_mode.stable_id(),
        config.environment_depth_hand_removal,
        config.openxr_passthrough_probe.stable_id(),
        config.passthrough_style_mode.stable_id(),
        config.passthrough_opacity,
        config.passthrough_edge_color,
        config.passthrough_brightness,
        config.passthrough_contrast,
        config.passthrough_saturation,
        config.passthrough_color_phase,
        config.passthrough_color_amplitude,
        config.passthrough_lut_resolution,
        config.passthrough_lut_weight,
        config.passthrough_lut_flicker_hz,
        config.full_field_flicker_hz,
        config.projection_layer_visible,
        diagnostic_hud_snapshot().visible
    ));
    #[cfg(target_os = "android")]
    log_projection_runtime_manifest("runtime-config", &config, parsed.is_some());

    #[cfg(target_os = "android")]
    {
        log_info(format!(
            "Rusty XR OSC config enabled={} listenAddr={} maxPacketBytes={}",
            config.osc_enabled, config.osc_listen_addr, config.osc_max_packet_bytes
        ));
        osc_ingress::ensure_listener(&config);
    }
}

fn sync_diagnostic_hud_state(parsed: Option<&JavaRuntimeConfig>, config: &RuntimeConfig) {
    let parsed_command = parsed
        .and_then(|bridge| bridge.diagnostic_hud_command.as_deref())
        .and_then(parse_diagnostic_hud_command);
    let command = parsed_command.unwrap_or(DiagnosticHudCommand::SetVisible(
        config.diagnostic_hud_visible,
    ));
    let source = if parsed_command.is_some() {
        DiagnosticHudInputSource::AdbIntent
    } else {
        DiagnosticHudInputSource::RuntimeConfig
    };
    let update = apply_diagnostic_hud_command(command, source);

    #[cfg(not(target_os = "android"))]
    let _ = update;

    #[cfg(target_os = "android")]
    if update.changed {
        log_info(format!(
            "Rusty XR diagnostic HUD state visible={} page={}/{} revision={} source={}",
            update.visible,
            update.page_index.saturating_add(1),
            update.page_count,
            update.revision,
            update
                .last_input_source
                .map(DiagnosticHudInputSource::stable_id)
                .unwrap_or("unknown")
        ));
    }
}

fn parse_diagnostic_hud_command(value: &str) -> Option<DiagnosticHudCommand> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "none" | "noop" => None,
        "show" | "on" | "true" | "1" => Some(DiagnosticHudCommand::Show),
        "hide" | "off" | "false" | "0" => Some(DiagnosticHudCommand::Hide),
        "toggle" => Some(DiagnosticHudCommand::Toggle),
        "next" | "next-page" | "page-next" => Some(DiagnosticHudCommand::NextPage),
        "previous" | "prev" | "previous-page" | "page-prev" => {
            Some(DiagnosticHudCommand::PreviousPage)
        }
        _ => normalized
            .strip_prefix("page:")
            .and_then(|page| page.parse::<usize>().ok())
            .map(DiagnosticHudCommand::SetPage),
    }
}

fn store_headset_camera_frame(
    width: u32,
    height: u32,
    timestamp_ns: i64,
    metadata_json: Option<String>,
    rgba: Vec<u8>,
) {
    let expected_len = width as usize * height as usize * 4;
    if width == 0 || height == 0 || rgba.len() != expected_len {
        #[cfg(target_os = "android")]
        log_error(format!(
            "Rusty XR rejected headset camera frame {}x{} bytes={} expected={}",
            width,
            height,
            rgba.len(),
            expected_len
        ));
        return;
    }

    let parsed_metadata = metadata_json
        .as_deref()
        .and_then(|json| parse_camera_frame_metadata_json(json, "camera"));

    if let Ok(mut state) = headset_camera_state().lock() {
        let index = state.next_index;
        state.next_index = state.next_index.saturating_add(1);
        let (metadata, diagnostics) =
            public_camera_metadata(parsed_metadata.as_ref(), index, width, height, timestamp_ns);
        state.latest = Some(HeadsetCameraFrame {
            width,
            height,
            timestamp_ns,
            index,
            metadata,
            diagnostics: diagnostics.clone(),
            rgba,
        });
        if index.is_multiple_of(30) {
            #[cfg(target_os = "android")]
            log_info(format!(
                "Rusty XR received headset camera frame {} source={} cameraId={} size={}x{} ts={} lensFacing={} lensRank={} score={} transport={} requestedTier={} activeTier={} stereoLayout={:?} requestedStereoLayout={} intrinsics={} pose={} poseSource={} fallbackReason={}",
                index,
                diagnostics_source_label(parsed_metadata.as_ref()),
                diagnostics.camera_id.as_deref().unwrap_or("unknown"),
                width,
                height,
                timestamp_ns,
                diagnostics.lens_facing.as_deref().unwrap_or("unknown"),
                diagnostics
                    .lens_facing_rank
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                diagnostics
                    .selection_score
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                diagnostics.transport,
                diagnostics.requested_tier.stable_id(),
                diagnostics.active_tier_label,
                diagnostics.stereo_layout,
                diagnostics
                    .requested_stereo_layout
                    .as_deref()
                    .unwrap_or("unknown"),
                if state
                    .latest
                    .as_ref()
                    .map(|frame| frame.metadata.has_intrinsics())
                    .unwrap_or(false)
                {
                    "available"
                } else {
                    "missing"
                },
                if state
                    .latest
                    .as_ref()
                    .map(|frame| frame.metadata.has_pose())
                    .unwrap_or(false)
                {
                    "available"
                } else {
                    "missing"
                },
                diagnostics.pose_source.as_deref().unwrap_or("missing"),
                diagnostics.fallback_reason
            ));
        }
    }
}

fn store_headset_camera_gpu_frame(
    width: u32,
    height: u32,
    timestamp_ns: i64,
    metadata_json: Option<String>,
    gpu_buffer: HeadsetCameraGpuBufferImport,
) -> bool {
    let descriptor = gpu_buffer.descriptor.clone();
    if width == 0 || height == 0 || !descriptor.is_valid() {
        #[cfg(target_os = "android")]
        log_error(format!(
            "Rusty XR rejected headset camera GPU buffer frame {}x{} descriptorValid={}",
            width,
            height,
            descriptor.is_valid()
        ));
        if let Ok(mut state) = headset_camera_state().lock() {
            state.gpu_probe_failure_count = state.gpu_probe_failure_count.saturating_add(1);
        }
        return false;
    }

    let parsed_metadata = metadata_json
        .as_deref()
        .and_then(|json| parse_camera_frame_metadata_json(json, "camera GPU"));

    if let Ok(mut state) = headset_camera_state().lock() {
        let index = state.next_gpu_index;
        state.next_gpu_index = state.next_gpu_index.saturating_add(1);
        state.gpu_probe_success_count = state.gpu_probe_success_count.saturating_add(1);
        state
            .gpu_descriptor_cache_keys
            .insert(gpu_descriptor_cache_key(&descriptor));
        let (metadata, diagnostics) =
            public_camera_metadata(parsed_metadata.as_ref(), index, width, height, timestamp_ns);
        state.latest_gpu = Some(HeadsetCameraGpuFrame {
            width,
            height,
            timestamp_ns,
            index,
            metadata,
            diagnostics: diagnostics.clone(),
            descriptor: descriptor.clone(),
            #[cfg(target_os = "android")]
            hardware_buffer: gpu_buffer.hardware_buffer,
        });
        if index.is_multiple_of(120) {
            let _status = CameraProjectionStatus::fallback(
                diagnostics.requested_tier,
                CameraCompositeTier::GpuBufferProbe,
                diagnostics.fallback_reason.clone(),
            );
            #[cfg(target_os = "android")]
            log_info(format!(
                "Rusty XR received headset camera GPU buffer frame {} source={} cameraId={} size={}x{} ts={} format={} nativeFormat={} usage={} stride={} layers={} bufferId={} requestedTier={} activeTier={} stereoLayout={:?} requestedStereoLayout={} intrinsics={} pose={} poseSource={} gpuImportProbe=success descriptorProbeCacheSize={} fallbackReason={} alignedProjection={}",
                index,
                diagnostics_source_label(parsed_metadata.as_ref()),
                diagnostics.camera_id.as_deref().unwrap_or("unknown"),
                width,
                height,
                timestamp_ns,
                descriptor.format_label.as_str(),
                descriptor
                    .native_format
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                descriptor
                    .usage_flags
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                descriptor
                    .stride_px
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                descriptor
                    .layer_count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                descriptor
                    .buffer_id
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                diagnostics.requested_tier.stable_id(),
                diagnostics.active_tier_label,
                diagnostics.stereo_layout,
                diagnostics
                    .requested_stereo_layout
                    .as_deref()
                    .unwrap_or("unknown"),
                if state
                    .latest_gpu
                    .as_ref()
                    .map(|frame| frame.metadata.has_intrinsics())
                    .unwrap_or(false)
                {
                    "available"
                } else {
                    "missing"
                },
                if state
                    .latest_gpu
                    .as_ref()
                    .map(|frame| frame.metadata.has_pose())
                    .unwrap_or(false)
                {
                    "available"
                } else {
                    "missing"
                },
                diagnostics.pose_source.as_deref().unwrap_or("missing"),
                state.gpu_descriptor_cache_keys.len(),
                diagnostics.fallback_reason,
                _status.is_aligned_projection()
            ));
        }
        true
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn store_headset_stereo_camera_gpu_frame(
    left_width: u32,
    left_height: u32,
    left_timestamp_ns: i64,
    left_metadata_json: Option<String>,
    left_gpu_buffer: HeadsetCameraGpuBufferImport,
    right_width: u32,
    right_height: u32,
    right_timestamp_ns: i64,
    right_metadata_json: Option<String>,
    right_gpu_buffer: HeadsetCameraGpuBufferImport,
    pair_delta_ns: u64,
    pair_index: u64,
) -> bool {
    let left_descriptor = left_gpu_buffer.descriptor.clone();
    let right_descriptor = right_gpu_buffer.descriptor.clone();
    if left_width == 0
        || left_height == 0
        || right_width == 0
        || right_height == 0
        || !left_descriptor.is_valid()
        || !right_descriptor.is_valid()
    {
        #[cfg(target_os = "android")]
        log_error(format!(
            "Rusty XR rejected stereo camera GPU pair left={}x{} valid={} right={}x{} valid={}",
            left_width,
            left_height,
            left_descriptor.is_valid(),
            right_width,
            right_height,
            right_descriptor.is_valid()
        ));
        if let Ok(mut state) = headset_camera_state().lock() {
            state.stereo_dropped_count = state.stereo_dropped_count.saturating_add(1);
        }
        return false;
    }

    let left_parsed = left_metadata_json
        .as_deref()
        .and_then(|json| parse_camera_frame_metadata_json(json, "left stereo camera"));
    let right_parsed = right_metadata_json
        .as_deref()
        .and_then(|json| parse_camera_frame_metadata_json(json, "right stereo camera"));

    if let Ok(mut state) = headset_camera_state().lock() {
        let index = state.next_stereo_gpu_index.max(pair_index);
        state.next_stereo_gpu_index = index.saturating_add(1);
        state.stereo_left_received_count = state.stereo_left_received_count.saturating_add(1);
        state.stereo_right_received_count = state.stereo_right_received_count.saturating_add(1);
        state.stereo_paired_count = state.stereo_paired_count.saturating_add(1);
        state.stereo_pair_delta_total_ns = state
            .stereo_pair_delta_total_ns
            .saturating_add(pair_delta_ns);
        state.stereo_pair_delta_max_ns = state.stereo_pair_delta_max_ns.max(pair_delta_ns);
        state
            .gpu_descriptor_cache_keys
            .insert(gpu_descriptor_cache_key(&left_descriptor));
        state
            .gpu_descriptor_cache_keys
            .insert(gpu_descriptor_cache_key(&right_descriptor));

        let (left_metadata, left_diagnostics) = public_camera_metadata(
            left_parsed.as_ref(),
            index,
            left_width,
            left_height,
            left_timestamp_ns,
        );
        let (right_metadata, right_diagnostics) = public_camera_metadata(
            right_parsed.as_ref(),
            index,
            right_width,
            right_height,
            right_timestamp_ns,
        );
        let left_frame = HeadsetCameraGpuFrame {
            width: left_width,
            height: left_height,
            timestamp_ns: left_timestamp_ns,
            index,
            metadata: left_metadata,
            diagnostics: left_diagnostics.clone(),
            descriptor: left_descriptor.clone(),
            #[cfg(target_os = "android")]
            hardware_buffer: left_gpu_buffer.hardware_buffer,
        };
        let right_frame = HeadsetCameraGpuFrame {
            width: right_width,
            height: right_height,
            timestamp_ns: right_timestamp_ns,
            index,
            metadata: right_metadata,
            diagnostics: right_diagnostics.clone(),
            descriptor: right_descriptor.clone(),
            #[cfg(target_os = "android")]
            hardware_buffer: right_gpu_buffer.hardware_buffer,
        };
        let midpoint_timestamp_ns = left_timestamp_ns / 2 + right_timestamp_ns / 2;
        state.latest_stereo_gpu = Some(StereoGpuCameraFrame {
            index,
            left: left_frame,
            right: right_frame,
            pair_delta_ns,
            midpoint_timestamp_ns,
        });

        if index.is_multiple_of(120) {
            let _avg_delta = if state.stereo_paired_count > 0 {
                state.stereo_pair_delta_total_ns / state.stereo_paired_count
            } else {
                0
            };
            let _pose_source = left_diagnostics.pose_source.as_deref().unwrap_or("missing");
            let _pose_reference = left_diagnostics
                .lens_pose_reference_label
                .as_deref()
                .unwrap_or("unknown");
            let _projection_metadata_ready = left_diagnostics.requested_tier
                == CameraCompositeTier::GpuProjected
                && left_parsed
                    .as_ref()
                    .map(|_| {
                        state
                            .latest_stereo_gpu
                            .as_ref()
                            .unwrap()
                            .left
                            .metadata
                            .has_projection_metadata()
                    })
                    .unwrap_or(false)
                && right_parsed
                    .as_ref()
                    .map(|_| {
                        state
                            .latest_stereo_gpu
                            .as_ref()
                            .unwrap()
                            .right
                            .metadata
                            .has_projection_metadata()
                    })
                    .unwrap_or(false);
            #[cfg(target_os = "android")]
            log_info(format!(
                "Rusty XR received stereo GPU camera pair {} activeTier={} alignedProjection=false projectionMetadataReady={} stereoLayout=Separate poseSource={} poseReference={} leftCameraId={} rightCameraId={} left={}x{} right={}x{} leftTs={} rightTs={} pairDeltaNs={} avgPairDeltaNs={} maxPairDeltaNs={} pairedLeftRightGpuBuffers=true cpuUploadCount=0 droppedStereoFrames={} descriptorProbeCacheSize={}",
                index,
                left_diagnostics.active_tier_label,
                _projection_metadata_ready,
                _pose_source,
                _pose_reference,
                left_diagnostics.camera_id.as_deref().unwrap_or("unknown"),
                right_diagnostics.camera_id.as_deref().unwrap_or("unknown"),
                left_width,
                left_height,
                right_width,
                right_height,
                left_timestamp_ns,
                right_timestamp_ns,
                pair_delta_ns,
                _avg_delta,
                state.stereo_pair_delta_max_ns,
                state.stereo_dropped_count,
                state.gpu_descriptor_cache_keys.len()
            ));
        }
        true
    } else {
        false
    }
}

fn gpu_descriptor_cache_key(descriptor: &CameraGpuBufferDescriptor) -> String {
    if let Some(buffer_id) = descriptor.buffer_id {
        return format!("id:{buffer_id}");
    }

    format!(
        "{}:{}x{}:{}:{}",
        descriptor.format_label,
        descriptor.size.width,
        descriptor.size.height,
        descriptor.native_format.unwrap_or_default(),
        descriptor.stride_px.unwrap_or_default()
    )
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaRuntimeConfig {
    camera_tier: Option<String>,
    camera_acquisition: Option<String>,
    camera_enabled: Option<bool>,
    media_projection_enabled: Option<bool>,
    allow_cpu_fallback: Option<bool>,
    cpu_upload_hz: Option<u32>,
    stereo_layout: Option<String>,
    projection_runtime_resolution_enabled: Option<bool>,
    camera_projection_fov_y_degrees: Option<f32>,
    camera_preview_fov_y_degrees: Option<f32>,
    camera_preview_offset_y_meters: Option<f32>,
    camera_projection_scale: Option<f32>,
    projection_depth_meters: Option<f32>,
    projection_area_scale_uv: Option<f32>,
    projection_area_offset_x_uv: Option<f32>,
    projection_area_offset_y_uv: Option<f32>,
    projection_area_left_offset_x_uv: Option<f32>,
    projection_area_left_offset_y_uv: Option<f32>,
    projection_area_right_offset_x_uv: Option<f32>,
    projection_area_right_offset_y_uv: Option<f32>,
    projection_area_radius_x_uv: Option<f32>,
    projection_area_radius_y_uv: Option<f32>,
    projection_area_corner_radius_uv: Option<f32>,
    projection_area_opacity: Option<f32>,
    projection_border_opacity: Option<f32>,
    projection_target_offset_x_uv: Option<f32>,
    projection_target_offset_y_uv: Option<f32>,
    projection_target_scale: Option<f32>,
    projection_target_joystick_controls: Option<String>,
    projection_alpha_mode: Option<String>,
    projection_alpha_scale: Option<f32>,
    projection_alpha_bias: Option<f32>,
    camera_raw_overlay_overscan: Option<f32>,
    camera_full_view_overlay_overscan: Option<f32>,
    camera_edge_fade: Option<f32>,
    camera_texture_rotation: Option<String>,
    camera_texture_flip_x: Option<bool>,
    camera_texture_flip_y: Option<bool>,
    camera_texture_mirror: Option<bool>,
    camera_texture_transform_source: Option<String>,
    camera_texture_transform_reason: Option<String>,
    camera_projection_mode: Option<String>,
    #[serde(rename = "cameraPipelinePreset")]
    camera_pipeline_preset: Option<String>,
    #[serde(rename = "cameraProjectionEffectMode")]
    camera_projection_effect_mode: Option<String>,
    projection_border_policy: Option<String>,
    processing_layer: Option<String>,
    peripheral_stretch_mode: Option<String>,
    peripheral_stretch_core_scale: Option<f32>,
    peripheral_stretch_edge_inset_uv: Option<f32>,
    peripheral_stretch_max_inset_uv: Option<f32>,
    peripheral_stretch_curve: Option<f32>,
    peripheral_stretch_inner_blend_uv: Option<f32>,
    peripheral_stretch_blend_curve: Option<f32>,
    peripheral_stretch_blend_mode: Option<String>,
    peripheral_stretch_corner_mode: Option<String>,
    peripheral_stretch_debug: Option<String>,
    camera_color_mode: Option<String>,
    camera_sampler_binding_mode: Option<String>,
    #[serde(rename = "cameraImportImageLayout")]
    camera_import_image_layout_mode: Option<String>,
    camera_import_cache_limit: Option<usize>,
    camera_color_matrix: Option<String>,
    camera_color_offset: Option<String>,
    camera_color_contrast: Option<f32>,
    camera_color_brightness: Option<f32>,
    camera_color_saturation: Option<f32>,
    #[serde(rename = "cameraBlurRadiusPx")]
    camera_blur_radius_px: Option<f32>,
    camera_temporal_projection_enabled: Option<bool>,
    camera_temporal_mode: Option<String>,
    camera_temporal_max_pixels_per_frame: Option<f32>,
    camera_temporal_max_angular_degrees_per_frame: Option<f32>,
    camera_temporal_max_linear_meters_per_frame: Option<f32>,
    camera_temporal_catchup_half_life_ms: Option<f32>,
    camera_temporal_max_visual_lag_ms: Option<f32>,
    camera_temporal_stereo_lockstep: Option<bool>,
    camera_temporal_edge_mode: Option<String>,
    camera_frame_adoption_mode: Option<String>,
    camera_frame_adoption_max_jump_px: Option<f32>,
    camera_frame_adoption_max_hold_ms: Option<f32>,
    left_camera_texture_rotation: Option<String>,
    left_camera_texture_flip_x: Option<bool>,
    left_camera_texture_flip_y: Option<bool>,
    left_camera_texture_mirror: Option<bool>,
    left_camera_texture_transform_source: Option<String>,
    left_camera_texture_transform_reason: Option<String>,
    right_camera_texture_rotation: Option<String>,
    right_camera_texture_flip_x: Option<bool>,
    right_camera_texture_flip_y: Option<bool>,
    right_camera_texture_mirror: Option<bool>,
    right_camera_texture_transform_source: Option<String>,
    right_camera_texture_transform_reason: Option<String>,
    camera_source_eye_mapping: Option<String>,
    camera_orientation_diagnostic_mode: Option<String>,
    #[serde(rename = "cameraFeedMode", alias = "cameraFeedPipelineMode")]
    camera_feed_pipeline_mode: Option<String>,
    visual_release_accepted: Option<bool>,
    visual_acceptance_token: Option<String>,
    xr_render_scale: Option<f32>,
    #[serde(rename = "xrDisplayRefreshHz")]
    xr_display_refresh_hz: Option<f32>,
    xr_fixed_foveation_level: Option<u8>,
    #[serde(rename = "xrColorFormat")]
    xr_color_format_mode: Option<String>,
    openxr_passthrough_probe: Option<String>,
    environment_depth_mode: Option<String>,
    environment_depth_hand_removal: Option<bool>,
    hand_particle_mode: Option<String>,
    passthrough_style_mode: Option<String>,
    passthrough_opacity: Option<f32>,
    passthrough_edge_r: Option<f32>,
    passthrough_edge_g: Option<f32>,
    passthrough_edge_b: Option<f32>,
    passthrough_edge_a: Option<f32>,
    passthrough_brightness: Option<f32>,
    passthrough_contrast: Option<f32>,
    passthrough_saturation: Option<f32>,
    passthrough_color_phase: Option<f32>,
    passthrough_color_amplitude: Option<f32>,
    passthrough_lut_resolution: Option<u32>,
    passthrough_lut_weight: Option<f32>,
    passthrough_lut_flicker_hz: Option<f32>,
    #[serde(rename = "fullFieldFlickerHz")]
    full_field_flicker_hz: Option<f32>,
    projection_layer_visible: Option<bool>,
    #[serde(alias = "diagnosticsHudVisible")]
    diagnostic_hud_visible: Option<bool>,
    #[serde(alias = "diagnosticsHudCommand")]
    diagnostic_hud_command: Option<String>,
    osc_enabled: Option<bool>,
    osc_overlay_enabled: Option<bool>,
    osc_listen_addr: Option<String>,
    osc_max_packet_bytes: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaProjectionTargetControl {
    #[serde(rename = "projectionTargetScale", alias = "scale")]
    projection_target_scale: Option<f32>,
}

fn public_runtime_config(bridge: &JavaRuntimeConfig) -> RuntimeConfig {
    let camera_tier = bridge
        .camera_tier
        .as_deref()
        .and_then(CameraCompositeTier::parse)
        .unwrap_or(CameraCompositeTier::CpuDiagnosticFlatCopy);
    let defaults = RuntimeConfig::default();
    let mut config = RuntimeConfig {
        camera_tier,
        camera_acquisition: bridge
            .camera_acquisition
            .clone()
            .unwrap_or(defaults.camera_acquisition),
        camera_enabled: bridge.camera_enabled.unwrap_or(true),
        media_projection_enabled: bridge.media_projection_enabled.unwrap_or(false),
        allow_cpu_fallback: bridge.allow_cpu_fallback.unwrap_or(true),
        cpu_upload_hz: bridge.cpu_upload_hz.unwrap_or(4),
        stereo_layout: bridge
            .stereo_layout
            .as_deref()
            .and_then(parse_stereo_layout)
            .unwrap_or(StereoMediaLayout::Mono),
        projection_runtime_resolution_enabled: bridge
            .projection_runtime_resolution_enabled
            .unwrap_or(defaults.projection_runtime_resolution_enabled),
        camera_projection_fov_y_degrees: finite_positive_or(
            bridge.camera_projection_fov_y_degrees,
            defaults.camera_projection_fov_y_degrees,
        ),
        camera_preview_fov_y_degrees: finite_positive_or(
            bridge.camera_preview_fov_y_degrees,
            defaults.camera_preview_fov_y_degrees,
        ),
        camera_preview_offset_y_meters: finite_or(
            bridge.camera_preview_offset_y_meters,
            defaults.camera_preview_offset_y_meters,
        )
        .clamp(-2.0, 2.0),
        camera_projection_scale: finite_positive_or(
            bridge.camera_projection_scale,
            defaults.camera_projection_scale,
        ),
        camera_projection_depth_meters: finite_positive_or(
            bridge.projection_depth_meters,
            defaults.camera_projection_depth_meters,
        )
        .clamp(0.05, 10.0),
        camera_projection_area_scale_uv: finite_positive_or(
            bridge.projection_area_scale_uv,
            defaults.camera_projection_area_scale_uv,
        )
        .clamp(0.05, 4.0),
        camera_projection_area_offset_x_uv: bridge
            .projection_area_offset_x_uv
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_area_offset_x_uv)
            .clamp(-0.5, 0.5),
        camera_projection_area_offset_y_uv: bridge
            .projection_area_offset_y_uv
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_area_offset_y_uv)
            .clamp(-0.5, 0.5),
        camera_projection_area_left_offset_x_uv: bridge
            .projection_area_left_offset_x_uv
            .filter(|value| value.is_finite())
            .unwrap_or_else(|| {
                bridge
                    .projection_area_offset_x_uv
                    .filter(|value| value.is_finite())
                    .unwrap_or(defaults.camera_projection_area_offset_x_uv)
            })
            .clamp(-0.5, 0.5),
        camera_projection_area_left_offset_y_uv: bridge
            .projection_area_left_offset_y_uv
            .filter(|value| value.is_finite())
            .unwrap_or_else(|| {
                bridge
                    .projection_area_offset_y_uv
                    .filter(|value| value.is_finite())
                    .unwrap_or(defaults.camera_projection_area_offset_y_uv)
            })
            .clamp(-0.5, 0.5),
        camera_projection_area_right_offset_x_uv: bridge
            .projection_area_right_offset_x_uv
            .filter(|value| value.is_finite())
            .unwrap_or_else(|| {
                bridge
                    .projection_area_offset_x_uv
                    .filter(|value| value.is_finite())
                    .unwrap_or(defaults.camera_projection_area_offset_x_uv)
            })
            .clamp(-0.5, 0.5),
        camera_projection_area_right_offset_y_uv: bridge
            .projection_area_right_offset_y_uv
            .filter(|value| value.is_finite())
            .unwrap_or_else(|| {
                bridge
                    .projection_area_offset_y_uv
                    .filter(|value| value.is_finite())
                    .unwrap_or(defaults.camera_projection_area_offset_y_uv)
            })
            .clamp(-0.5, 0.5),
        camera_projection_area_radius_x_uv: bridge
            .projection_area_radius_x_uv
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_area_radius_x_uv)
            .clamp(0.05, 0.5),
        camera_projection_area_radius_y_uv: bridge
            .projection_area_radius_y_uv
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_area_radius_y_uv)
            .clamp(0.05, 0.5),
        camera_projection_area_corner_radius_uv: bridge
            .projection_area_corner_radius_uv
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_area_corner_radius_uv)
            .clamp(0.0, 0.5),
        camera_projection_area_opacity: bridge
            .projection_area_opacity
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_area_opacity)
            .clamp(0.0, 1.0),
        camera_projection_border_opacity: bridge
            .projection_border_opacity
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_border_opacity)
            .clamp(0.0, 1.0),
        projection_target_offset_x_uv: bridge
            .projection_target_offset_x_uv
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.projection_target_offset_x_uv)
            .clamp(-0.5, 0.5),
        projection_target_offset_y_uv: bridge
            .projection_target_offset_y_uv
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.projection_target_offset_y_uv)
            .clamp(-0.5, 0.5),
        projection_target_scale: finite_positive_or(
            bridge.projection_target_scale,
            defaults.projection_target_scale,
        )
        .clamp(0.05, 1.5),
        projection_target_joystick_controls: bridge
            .projection_target_joystick_controls
            .as_deref()
            .and_then(ProjectionTargetJoystickControls::parse)
            .unwrap_or(defaults.projection_target_joystick_controls),
        camera_projection_alpha_mode: bridge
            .projection_alpha_mode
            .as_deref()
            .and_then(CameraProjectionAlphaMode::parse)
            .unwrap_or(defaults.camera_projection_alpha_mode),
        camera_projection_alpha_scale: bridge
            .projection_alpha_scale
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_alpha_scale)
            .clamp(0.0, 4.0),
        camera_projection_alpha_bias: bridge
            .projection_alpha_bias
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.camera_projection_alpha_bias)
            .clamp(-1.0, 1.0),
        camera_raw_overlay_overscan: finite_positive_or(
            bridge.camera_raw_overlay_overscan,
            defaults.camera_raw_overlay_overscan,
        )
        .max(1.0),
        camera_full_view_overlay_overscan: finite_positive_or(
            bridge.camera_full_view_overlay_overscan,
            defaults.camera_full_view_overlay_overscan,
        )
        .max(1.0),
        camera_edge_fade: finite_positive_or(bridge.camera_edge_fade, defaults.camera_edge_fade)
            .clamp(0.0, 0.5),
        camera_texture_transform: public_camera_texture_transform(bridge),
        left_camera_texture_transform: public_eye_camera_texture_transform(bridge, true),
        right_camera_texture_transform: public_eye_camera_texture_transform(bridge, false),
        source_eye_mapping: bridge
            .camera_source_eye_mapping
            .as_deref()
            .and_then(StereoSourceEyeMapping::parse)
            .unwrap_or_default(),
        camera_projection_mode: bridge
            .camera_projection_mode
            .as_deref()
            .and_then(CameraProjectionMode::parse)
            .unwrap_or_default(),
        camera_pipeline_preset: bridge
            .camera_pipeline_preset
            .as_deref()
            .and_then(CameraPipelinePreset::parse)
            .unwrap_or_default(),
        camera_projection_effect_mode: bridge
            .camera_projection_effect_mode
            .as_deref()
            .and_then(CameraProjectionEffectMode::parse)
            .unwrap_or_default(),
        camera_projection_border_policy: bridge
            .projection_border_policy
            .as_deref()
            .and_then(CameraProjectionBorderPolicy::parse)
            .or_else(|| {
                bridge
                    .camera_projection_effect_mode
                    .as_deref()
                    .and_then(CameraProjectionBorderPolicy::from_legacy_projection_value)
            })
            .or_else(|| {
                bridge
                    .camera_pipeline_preset
                    .as_deref()
                    .and_then(CameraProjectionBorderPolicy::from_legacy_projection_value)
            })
            .unwrap_or_default(),
        camera_processing_layer: bridge
            .processing_layer
            .as_deref()
            .and_then(CameraProcessingLayer::parse)
            .unwrap_or_default(),
        camera_peripheral_stretch: public_peripheral_stretch_config(bridge),
        camera_feed_pipeline_mode: bridge
            .camera_feed_pipeline_mode
            .as_deref()
            .and_then(CameraFeedPipelineMode::parse)
            .unwrap_or_default(),
        camera_color_mode: bridge
            .camera_color_mode
            .as_deref()
            .and_then(CameraColorMode::parse)
            .unwrap_or_default(),
        camera_sampler_binding_mode: bridge
            .camera_sampler_binding_mode
            .as_deref()
            .and_then(CameraSamplerBindingMode::parse)
            .unwrap_or_default(),
        camera_import_image_layout_mode: bridge
            .camera_import_image_layout_mode
            .as_deref()
            .and_then(CameraImportImageLayoutMode::parse)
            .unwrap_or_default(),
        camera_import_cache_limit: bridge
            .camera_import_cache_limit
            .unwrap_or(defaults.camera_import_cache_limit)
            .clamp(2, CAMERA_IMPORT_CACHE_LIMIT_MAX),
        camera_color_matrix: parse_camera_color_matrix(
            bridge.camera_color_matrix.as_deref(),
            defaults.camera_color_matrix,
        ),
        camera_color_offset: parse_camera_color_offset(
            bridge.camera_color_offset.as_deref(),
            defaults.camera_color_offset,
        ),
        camera_color_contrast: finite_positive_or(
            bridge.camera_color_contrast,
            defaults.camera_color_contrast,
        )
        .clamp(0.0, 4.0),
        camera_color_brightness: finite_or(
            bridge.camera_color_brightness,
            defaults.camera_color_brightness,
        )
        .clamp(-1.0, 1.0),
        camera_color_saturation: finite_positive_or(
            bridge.camera_color_saturation,
            defaults.camera_color_saturation,
        )
        .clamp(0.0, 4.0),
        camera_blur_radius_px: finite_or(
            bridge.camera_blur_radius_px,
            defaults.camera_blur_radius_px,
        )
        .clamp(0.0, 16.0),
        camera_temporal_projection_enabled: bridge
            .camera_temporal_projection_enabled
            .unwrap_or(defaults.camera_temporal_projection_enabled),
        camera_temporal_mode: bridge
            .camera_temporal_mode
            .as_deref()
            .and_then(TemporalProjectionMode::parse)
            .unwrap_or(defaults.camera_temporal_mode),
        camera_temporal_max_pixels_per_frame: finite_positive_or(
            bridge.camera_temporal_max_pixels_per_frame,
            defaults.camera_temporal_max_pixels_per_frame,
        )
        .clamp(1.0, 512.0),
        camera_temporal_max_angular_degrees_per_frame: finite_positive_or(
            bridge.camera_temporal_max_angular_degrees_per_frame,
            defaults.camera_temporal_max_angular_degrees_per_frame,
        )
        .clamp(0.01, 90.0),
        camera_temporal_max_linear_meters_per_frame: finite_positive_or(
            bridge.camera_temporal_max_linear_meters_per_frame,
            defaults.camera_temporal_max_linear_meters_per_frame,
        )
        .clamp(0.001, 2.0),
        camera_temporal_catchup_half_life_ms: finite_positive_or(
            bridge.camera_temporal_catchup_half_life_ms,
            defaults.camera_temporal_catchup_half_life_ms,
        )
        .clamp(1.0, 1000.0),
        camera_temporal_max_visual_lag_ms: finite_positive_or(
            bridge.camera_temporal_max_visual_lag_ms,
            defaults.camera_temporal_max_visual_lag_ms,
        )
        .clamp(0.0, 1000.0),
        camera_temporal_stereo_lockstep: bridge
            .camera_temporal_stereo_lockstep
            .unwrap_or(defaults.camera_temporal_stereo_lockstep),
        camera_temporal_edge_mode: bridge
            .camera_temporal_edge_mode
            .as_deref()
            .and_then(TemporalProjectionEdgeMode::parse)
            .unwrap_or(defaults.camera_temporal_edge_mode),
        camera_frame_adoption_mode: bridge
            .camera_frame_adoption_mode
            .as_deref()
            .and_then(CameraFrameAdoptionMode::parse)
            .unwrap_or(defaults.camera_frame_adoption_mode),
        camera_frame_adoption_max_jump_px: finite_positive_or(
            bridge.camera_frame_adoption_max_jump_px,
            defaults.camera_frame_adoption_max_jump_px,
        )
        .clamp(1.0, 512.0),
        camera_frame_adoption_max_hold_ms: finite_positive_or(
            bridge.camera_frame_adoption_max_hold_ms,
            defaults.camera_frame_adoption_max_hold_ms,
        )
        .clamp(0.0, 1000.0),
        orientation_diagnostic_mode: bridge
            .camera_orientation_diagnostic_mode
            .as_deref()
            .and_then(CameraOrientationDiagnosticMode::parse)
            .unwrap_or_default(),
        visual_release_accepted: bridge.visual_release_accepted.unwrap_or(false)
            && matches!(
                bridge.visual_acceptance_token.as_deref(),
                Some("manual-visual-accepted")
            ),
        xr_render_scale: finite_positive_or(bridge.xr_render_scale, defaults.xr_render_scale)
            .clamp(0.25, 1.5),
        xr_display_refresh_hz: finite_positive_or(
            bridge.xr_display_refresh_hz,
            defaults.xr_display_refresh_hz,
        )
        .clamp(60.0, 144.0),
        xr_fixed_foveation_level: bridge
            .xr_fixed_foveation_level
            .unwrap_or(defaults.xr_fixed_foveation_level),
        xr_color_format_mode: bridge
            .xr_color_format_mode
            .as_deref()
            .and_then(OpenXrColorFormatMode::parse)
            .unwrap_or(defaults.xr_color_format_mode),
        environment_depth_mode: bridge
            .environment_depth_mode
            .as_deref()
            .and_then(EnvironmentDepthMode::parse)
            .unwrap_or(defaults.environment_depth_mode),
        environment_depth_hand_removal: bridge
            .environment_depth_hand_removal
            .unwrap_or(defaults.environment_depth_hand_removal),
        hand_particle_mode: bridge
            .hand_particle_mode
            .as_deref()
            .and_then(HandParticleMode::parse)
            .unwrap_or(defaults.hand_particle_mode),
        openxr_passthrough_probe: bridge
            .openxr_passthrough_probe
            .as_deref()
            .and_then(OpenXrPassthroughProbeMode::parse)
            .unwrap_or(defaults.openxr_passthrough_probe),
        passthrough_style_mode: bridge
            .passthrough_style_mode
            .as_deref()
            .and_then(OpenXrPassthroughStyleMode::parse)
            .unwrap_or(defaults.passthrough_style_mode),
        passthrough_opacity: finite_positive_or(
            bridge.passthrough_opacity,
            defaults.passthrough_opacity,
        )
        .clamp(0.0, 1.0),
        passthrough_edge_color: [
            finite_or(
                bridge.passthrough_edge_r,
                defaults.passthrough_edge_color[0],
            )
            .clamp(0.0, 1.0),
            finite_or(
                bridge.passthrough_edge_g,
                defaults.passthrough_edge_color[1],
            )
            .clamp(0.0, 1.0),
            finite_or(
                bridge.passthrough_edge_b,
                defaults.passthrough_edge_color[2],
            )
            .clamp(0.0, 1.0),
            finite_or(
                bridge.passthrough_edge_a,
                defaults.passthrough_edge_color[3],
            )
            .clamp(0.0, 1.0),
        ],
        passthrough_brightness: finite_or(
            bridge.passthrough_brightness,
            defaults.passthrough_brightness,
        )
        .clamp(-100.0, 100.0),
        passthrough_contrast: finite_positive_or(
            bridge.passthrough_contrast,
            defaults.passthrough_contrast,
        )
        .clamp(0.0, 10.0),
        passthrough_saturation: finite_positive_or(
            bridge.passthrough_saturation,
            defaults.passthrough_saturation,
        )
        .clamp(0.0, 10.0),
        passthrough_color_phase: finite_or(
            bridge.passthrough_color_phase,
            defaults.passthrough_color_phase,
        ),
        passthrough_color_amplitude: finite_positive_or(
            bridge.passthrough_color_amplitude,
            defaults.passthrough_color_amplitude,
        )
        .clamp(0.0, 1.0),
        passthrough_lut_resolution: bridge
            .passthrough_lut_resolution
            .unwrap_or(defaults.passthrough_lut_resolution)
            .clamp(2, 64),
        passthrough_lut_weight: finite_or(
            bridge.passthrough_lut_weight,
            defaults.passthrough_lut_weight,
        )
        .clamp(0.0, 1.0),
        passthrough_lut_flicker_hz: finite_or(
            bridge.passthrough_lut_flicker_hz,
            defaults.passthrough_lut_flicker_hz,
        )
        .clamp(0.0, 120.0),
        full_field_flicker_hz: finite_or(
            bridge.full_field_flicker_hz,
            defaults.full_field_flicker_hz,
        )
        .clamp(0.0, 120.0),
        projection_layer_visible: bridge
            .projection_layer_visible
            .unwrap_or(defaults.projection_layer_visible),
        diagnostic_hud_visible: bridge
            .diagnostic_hud_visible
            .or(bridge.osc_overlay_enabled)
            .unwrap_or_else(|| {
                if bridge.osc_enabled.unwrap_or(false) {
                    defaults.osc_overlay_enabled
                } else {
                    defaults.diagnostic_hud_visible
                }
            }),
        osc_enabled: bridge.osc_enabled.unwrap_or(defaults.osc_enabled),
        osc_overlay_enabled: bridge
            .osc_overlay_enabled
            .or(bridge.diagnostic_hud_visible)
            .unwrap_or(defaults.osc_overlay_enabled),
        osc_listen_addr: public_osc_listen_addr(
            bridge.osc_listen_addr.as_deref(),
            &defaults.osc_listen_addr,
        ),
        osc_max_packet_bytes: bridge
            .osc_max_packet_bytes
            .unwrap_or(defaults.osc_max_packet_bytes)
            .clamp(256, rusty_xr_osc::DEFAULT_MAX_PACKET_BYTES),
    };
    apply_camera_pipeline_preset(&mut config);
    apply_projection_border_policy(&mut config);
    if config.projection_runtime_resolution_enabled {
        let runtime = hwb_projection_runtime_resolution(&config, true);
        apply_hwb_projection_runtime_resolution(&mut config, &runtime.resolution);
        apply_projection_border_policy(&mut config);
    }
    config
}

fn apply_camera_pipeline_preset(config: &mut RuntimeConfig) {
    let (
        feed_mode,
        color_mode,
        sampler_binding_mode,
        import_image_layout_mode,
        projection_effect_mode,
        xr_color_format_mode,
        openxr_passthrough_probe,
    ) = match config.camera_pipeline_preset {
        CameraPipelinePreset::Manual => return,
        CameraPipelinePreset::ProjectedSrgb => (
            CameraFeedPipelineMode::ProjectedFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::BorderComposite,
            OpenXrColorFormatMode::Rgba8Srgb,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::RawFeedUnorm => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::BorderComposite,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::ProjectedUnorm => (
            CameraFeedPipelineMode::ProjectedFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::BorderComposite,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::RawFeedSrgb => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::BorderComposite,
            OpenXrColorFormatMode::Rgba8Srgb,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::ShaderDecodeUnorm => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalCrYCbBt601Narrow,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::BorderComposite,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::SeparateDecodeUnorm => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalCrYCbBt601Narrow,
            CameraSamplerBindingMode::SeparateImageSampler,
            CameraImportImageLayoutMode::GeneralNoTransition,
            CameraProjectionEffectMode::BorderComposite,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::RawProjectionUnorm => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::RawProjection,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::ProjectionAreaDiagnosticUnorm => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::ProjectionAreaDiagnostic,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::DisplayEyeUvFiducialUnorm => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::DisplayEyeUvFiducial,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::ProjectionContentUvFiducialUnorm => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::ProjectionContentUvFiducial,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
        CameraPipelinePreset::SourceSamplingWitnessUnorm => (
            CameraFeedPipelineMode::RawFeed,
            CameraColorMode::ExternalRgb,
            CameraSamplerBindingMode::CombinedImmutableSampler,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition,
            CameraProjectionEffectMode::SourceSamplingWitness,
            OpenXrColorFormatMode::Rgba8Unorm,
            config.openxr_passthrough_probe,
        ),
    };
    config.camera_feed_pipeline_mode = feed_mode;
    config.camera_color_mode = color_mode;
    config.camera_sampler_binding_mode = sampler_binding_mode;
    config.camera_import_image_layout_mode = import_image_layout_mode;
    config.camera_projection_effect_mode = projection_effect_mode;
    config.xr_color_format_mode = xr_color_format_mode;
    config.openxr_passthrough_probe = openxr_passthrough_probe;
    config.camera_color_matrix = identity_color_matrix();
    config.camera_color_offset = [0.0, 0.0, 0.0];
    config.camera_color_contrast = 1.0;
    config.camera_color_brightness = 0.0;
    config.camera_color_saturation = 1.0;
}

fn apply_projection_border_policy(config: &mut RuntimeConfig) {
    if config
        .camera_projection_effect_mode
        .uses_projection_border_policy()
        && config
            .camera_projection_border_policy
            .uses_passthrough_underlay_alpha()
    {
        config.openxr_passthrough_probe = OpenXrPassthroughProbeMode::Underlay;
    }
}

fn public_peripheral_stretch_config(bridge: &JavaRuntimeConfig) -> CameraPeripheralStretchConfig {
    let defaults = CameraPeripheralStretchConfig::default();
    CameraPeripheralStretchConfig {
        mode: bridge
            .peripheral_stretch_mode
            .as_deref()
            .and_then(CameraPeripheralStretchMode::parse)
            .unwrap_or(defaults.mode),
        core_scale: finite_positive_or(bridge.peripheral_stretch_core_scale, defaults.core_scale),
        edge_inset_uv: finite_or(
            bridge.peripheral_stretch_edge_inset_uv,
            defaults.edge_inset_uv,
        ),
        max_inset_uv: finite_or(
            bridge.peripheral_stretch_max_inset_uv,
            defaults.max_inset_uv,
        ),
        curve: finite_positive_or(bridge.peripheral_stretch_curve, defaults.curve),
        inner_blend_uv: finite_or(
            bridge.peripheral_stretch_inner_blend_uv,
            defaults.inner_blend_uv,
        ),
        blend_curve: finite_positive_or(
            bridge.peripheral_stretch_blend_curve,
            defaults.blend_curve,
        ),
        blend_mode: bridge
            .peripheral_stretch_blend_mode
            .as_deref()
            .and_then(CameraPeripheralStretchBlendMode::parse)
            .unwrap_or(defaults.blend_mode),
        corner_mode: bridge
            .peripheral_stretch_corner_mode
            .as_deref()
            .and_then(CameraPeripheralStretchCornerMode::parse)
            .unwrap_or(defaults.corner_mode),
        debug: bridge
            .peripheral_stretch_debug
            .as_deref()
            .and_then(CameraPeripheralStretchDebug::parse)
            .unwrap_or(defaults.debug),
    }
    .sanitized()
}

fn public_camera_texture_transform(bridge: &JavaRuntimeConfig) -> CameraTextureTransform {
    let rotation = bridge
        .camera_texture_rotation
        .as_deref()
        .and_then(CameraImageRotation::parse)
        .unwrap_or(CameraImageRotation::Rotate0);
    CameraTextureTransform::new(
        bridge
            .camera_texture_transform_source
            .clone()
            .unwrap_or_else(|| "default".to_string()),
        bridge
            .camera_texture_transform_reason
            .clone()
            .unwrap_or_else(|| "unspecified".to_string()),
    )
    .with_rotation(rotation)
    .with_flip_x(bridge.camera_texture_flip_x.unwrap_or(false))
    .with_flip_y(bridge.camera_texture_flip_y.unwrap_or(false))
    .with_mirror(bridge.camera_texture_mirror.unwrap_or(false))
}

fn public_eye_camera_texture_transform(
    bridge: &JavaRuntimeConfig,
    left: bool,
) -> CameraTextureTransform {
    let global = public_camera_texture_transform(bridge);
    let rotation = if left {
        bridge.left_camera_texture_rotation.as_deref()
    } else {
        bridge.right_camera_texture_rotation.as_deref()
    }
    .and_then(CameraImageRotation::parse)
    .unwrap_or(global.rotation);
    let source = if left {
        bridge.left_camera_texture_transform_source.as_ref()
    } else {
        bridge.right_camera_texture_transform_source.as_ref()
    }
    .cloned()
    .unwrap_or_else(|| global.source_label.clone());
    let reason = if left {
        bridge.left_camera_texture_transform_reason.as_ref()
    } else {
        bridge.right_camera_texture_transform_reason.as_ref()
    }
    .cloned()
    .unwrap_or_else(|| global.reason.clone());

    CameraTextureTransform::new(source, reason)
        .with_rotation(rotation)
        .with_flip_x(
            if left {
                bridge.left_camera_texture_flip_x
            } else {
                bridge.right_camera_texture_flip_x
            }
            .unwrap_or(global.flip_x),
        )
        .with_flip_y(
            if left {
                bridge.left_camera_texture_flip_y
            } else {
                bridge.right_camera_texture_flip_y
            }
            .unwrap_or(global.flip_y),
        )
        .with_mirror(
            if left {
                bridge.left_camera_texture_mirror
            } else {
                bridge.right_camera_texture_mirror
            }
            .unwrap_or(global.mirror),
        )
}

fn finite_positive_or(value: Option<f32>, fallback: f32) -> f32 {
    value
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(fallback)
}

fn finite_or(value: Option<f32>, fallback: f32) -> f32 {
    value.filter(|value| value.is_finite()).unwrap_or(fallback)
}

fn public_osc_listen_addr(value: Option<&str>, fallback: &str) -> String {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return fallback.to_string();
    };
    if value.contains('\0') || value.len() > 128 {
        return fallback.to_string();
    }
    value.to_string()
}

fn parse_camera_color_matrix(value: Option<&str>, fallback: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let Some(value) = value else {
        return fallback;
    };
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("identity")
        || trimmed.eq_ignore_ascii_case("default")
        || trimmed.eq_ignore_ascii_case("none")
        || trimmed.eq_ignore_ascii_case("off")
    {
        return fallback;
    }
    let Some(values) = parse_f32_list(trimmed, 9) else {
        return fallback;
    };
    [
        [
            values[0].clamp(-8.0, 8.0),
            values[1].clamp(-8.0, 8.0),
            values[2].clamp(-8.0, 8.0),
        ],
        [
            values[3].clamp(-8.0, 8.0),
            values[4].clamp(-8.0, 8.0),
            values[5].clamp(-8.0, 8.0),
        ],
        [
            values[6].clamp(-8.0, 8.0),
            values[7].clamp(-8.0, 8.0),
            values[8].clamp(-8.0, 8.0),
        ],
    ]
}

fn parse_camera_color_offset(value: Option<&str>, fallback: [f32; 3]) -> [f32; 3] {
    let Some(value) = value else {
        return fallback;
    };
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("zero")
        || trimmed.eq_ignore_ascii_case("default")
        || trimmed.eq_ignore_ascii_case("none")
        || trimmed.eq_ignore_ascii_case("off")
    {
        return fallback;
    }
    let Some(values) = parse_f32_list(trimmed, 3) else {
        return fallback;
    };
    [
        values[0].clamp(-4.0, 4.0),
        values[1].clamp(-4.0, 4.0),
        values[2].clamp(-4.0, 4.0),
    ]
}

fn parse_f32_list(value: &str, expected_len: usize) -> Option<Vec<f32>> {
    let normalized = value
        .trim_matches(|c| matches!(c, '[' | ']' | '(' | ')'))
        .replace([';', '|'], ",");
    let values = normalized
        .split([',', ' ', '\t', '\n', '\r'])
        .filter(|part| !part.trim().is_empty())
        .map(|part| part.trim().parse::<f32>().ok())
        .collect::<Option<Vec<_>>>()?;
    if values.len() == expected_len && values.iter().all(|value| value.is_finite()) {
        Some(values)
    } else {
        None
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct JavaCameraFrameMetadata {
    source: Option<String>,
    source_label: Option<String>,
    camera_id: Option<String>,
    lens_facing: Option<String>,
    lens_facing_rank: Option<i32>,
    selection_score: Option<i64>,
    delivered_width: Option<u32>,
    delivered_height: Option<u32>,
    timestamp_ns: Option<i64>,
    sensor_orientation_degrees: Option<i32>,
    intrinsics: Option<JavaCameraIntrinsics>,
    intrinsics_domain: Option<JavaPixelDomain>,
    active_array_domain: Option<JavaPixelDomain>,
    sensor_pixel_domain: Option<JavaPixelDomain>,
    stereo_layout: Option<String>,
    requested_stereo_layout: Option<String>,
    transport: Option<String>,
    requested_tier: Option<String>,
    active_tier: Option<String>,
    gpu_import_requested: Option<bool>,
    pose_source: Option<String>,
    pose_coordinate_convention: Option<String>,
    lens_pose_reference_label: Option<String>,
    diagnostic_source: Option<bool>,
    synthetic_projection_profile: Option<String>,
    projection_geometry_profile: Option<String>,
    synthetic_pattern: Option<String>,
    raster_orientation_schema: Option<String>,
    orientation_kind: Option<String>,
    raster_orientation: Option<String>,
    raster_origin: Option<String>,
    raster_y_axis: Option<String>,
    upright_marker: Option<String>,
    orientation_metadata_source: Option<String>,
    orientation_default: Option<bool>,
    stimulus_orientation_schema: Option<String>,
    stimulus_raster_orientation: Option<String>,
    stimulus_upright_marker: Option<String>,
    stimulus_orientation_metadata_source: Option<String>,
    stimulus_orientation_default: Option<bool>,
    content_geometry_schema: Option<String>,
    content_kind: Option<String>,
    content_width: Option<u32>,
    content_height: Option<u32>,
    content_aspect_ratio: Option<f32>,
    desired_display_aspect_ratio: Option<f32>,
    desired_projection_aspect_ratio: Option<f32>,
    content_coordinate_space: Option<String>,
    content_origin: Option<String>,
    content_x_axis: Option<String>,
    content_y_axis: Option<String>,
    content_uv_rect: Option<JavaUvRect>,
    source_visible_uv_rect: Option<JavaUvRect>,
    source_crop_rect_px: Option<JavaPixelRect>,
    source_crop_rect_state: Option<String>,
    source_crop_rect_owner: Option<String>,
    source_sampling_mode: Option<String>,
    content_mapping_intent: Option<String>,
    content_geometry_metadata_source: Option<String>,
    content_geometry_default: Option<bool>,
    target_footprint_schema: Option<String>,
    target_coordinate_space: Option<String>,
    target_screen_uv_rect: Option<JavaTargetScreenUvRect>,
    target_clip_policy: Option<String>,
    target_footprint_metadata_source: Option<String>,
    target_footprint_default: Option<bool>,
    extrinsics: Option<JavaCameraExtrinsics>,
    missing_intrinsics: Option<bool>,
    missing_pose: Option<bool>,
    mono_fallback: Option<bool>,
    fallback_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaCameraIntrinsics {
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
    #[serde(default)]
    skew: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaCameraExtrinsics {
    px: f32,
    py: f32,
    pz: f32,
    qx: f32,
    qy: f32,
    qz: f32,
    qw: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaPixelDomain {
    kind: JavaPixelDomainKind,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaUvRect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaTargetScreenUvRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaPixelRect {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum JavaPixelDomainKind {
    DeliveredImage,
    ActiveArray,
    SensorPixelArray,
    Other,
}

fn public_camera_metadata(
    bridge: Option<&JavaCameraFrameMetadata>,
    frame_index: u64,
    width: u32,
    height: u32,
    timestamp_ns: i64,
) -> (CameraFrameMetadata, HeadsetCameraFrameDiagnostics) {
    let delivered_size = ImageSize::new(
        bridge
            .and_then(|value| value.delivered_width)
            .unwrap_or(width),
        bridge
            .and_then(|value| value.delivered_height)
            .unwrap_or(height),
    );
    let source_label = bridge
        .and_then(|value| value.source_label.clone())
        .unwrap_or_else(|| "Camera2 unknown".to_string());
    let mut source = CameraSourceId::new(source_label);
    if let Some(camera_id) = bridge.and_then(|value| value.camera_id.clone()) {
        source = source.with_physical_id(camera_id);
    }

    let intrinsics_domain = bridge
        .and_then(|value| value.intrinsics_domain)
        .or_else(|| bridge.and_then(|value| value.active_array_domain))
        .or_else(|| bridge.and_then(|value| value.sensor_pixel_domain))
        .and_then(public_pixel_domain);
    let intrinsics = bridge
        .and_then(|value| value.intrinsics)
        .zip(intrinsics_domain)
        .map(|(intrinsics, domain)| {
            CameraIntrinsics::new(
                Vec2::new(intrinsics.fx, intrinsics.fy),
                Vec2::new(intrinsics.cx, intrinsics.cy),
                domain.size,
            )
            .with_skew_px(intrinsics.skew)
        })
        .filter(|intrinsics| intrinsics.is_valid());
    let missing_intrinsics = bridge
        .and_then(|value| value.missing_intrinsics)
        .unwrap_or(false)
        || intrinsics.is_none();
    let pose_source = bridge.and_then(|value| value.pose_source.as_deref());
    let extrinsics = bridge
        .and_then(|value| value.extrinsics)
        .and_then(|extrinsics| {
            if pose_source == Some("platform") {
                camera2_lens_pose_to_extrinsics(
                    [extrinsics.px, extrinsics.py, extrinsics.pz],
                    [extrinsics.qx, extrinsics.qy, extrinsics.qz, extrinsics.qw],
                )
                .ok()
            } else {
                Some(CameraExtrinsics::new(Pose::new(
                    Vec3::new(extrinsics.px, extrinsics.py, extrinsics.pz),
                    Quat::new(extrinsics.qx, extrinsics.qy, extrinsics.qz, extrinsics.qw),
                )))
            }
        })
        .filter(|extrinsics| extrinsics.is_valid());
    let missing_pose =
        bridge.and_then(|value| value.missing_pose).unwrap_or(true) || extrinsics.is_none();

    let mut metadata = if let Some(intrinsics) = intrinsics {
        CameraFrameMetadata::new(source, frame_index, intrinsics)
    } else {
        CameraFrameMetadata::without_intrinsics(source, frame_index, delivered_size)
    };
    metadata.delivered_size = delivered_size;
    metadata.timestamp_ns = u64::try_from(
        bridge
            .and_then(|value| value.timestamp_ns)
            .unwrap_or(timestamp_ns),
    )
    .ok();
    metadata.sensor_orientation_degrees = bridge.and_then(|value| value.sensor_orientation_degrees);
    metadata.intrinsics_domain = intrinsics_domain;
    metadata.sensor_pixel_domain = bridge
        .and_then(|value| value.sensor_pixel_domain)
        .and_then(public_pixel_domain);
    metadata.flags = CameraFrameMetadataFlags::new(missing_intrinsics, missing_pose);
    if let Some(extrinsics) = extrinsics {
        metadata = metadata.with_extrinsics(extrinsics);
    }

    let diagnostics = HeadsetCameraFrameDiagnostics {
        source: bridge.and_then(|value| value.source.clone()),
        camera_id: bridge.and_then(|value| value.camera_id.clone()),
        lens_facing: bridge.and_then(|value| value.lens_facing.clone()),
        lens_facing_rank: bridge.and_then(|value| value.lens_facing_rank),
        selection_score: bridge.and_then(|value| value.selection_score),
        requested_tier: bridge
            .and_then(|value| value.requested_tier.as_deref())
            .and_then(CameraCompositeTier::parse)
            .unwrap_or(CameraCompositeTier::CpuDiagnosticFlatCopy),
        active_tier_label: bridge
            .and_then(|value| value.active_tier.clone())
            .unwrap_or_else(|| "cpu-diagnostic-flat-copy".to_string()),
        transport: bridge
            .and_then(|value| value.transport.clone())
            .unwrap_or_else(|| "cpu-yuv-rgba".to_string()),
        pose_source: bridge.and_then(|value| value.pose_source.clone()),
        pose_coordinate_convention: bridge
            .and_then(|value| value.pose_coordinate_convention.clone()),
        lens_pose_reference_label: bridge.and_then(|value| value.lens_pose_reference_label.clone()),
        diagnostic_source: bridge.and_then(|value| value.diagnostic_source),
        synthetic_projection_profile: bridge
            .and_then(|value| value.synthetic_projection_profile.clone()),
        projection_geometry_profile: bridge
            .and_then(|value| value.projection_geometry_profile.clone()),
        synthetic_pattern: bridge.and_then(|value| value.synthetic_pattern.clone()),
        orientation_kind: bridge.and_then(|value| value.orientation_kind.clone()),
        raster_orientation: bridge.and_then(|value| value.raster_orientation.clone()),
        upright_marker: bridge.and_then(|value| value.upright_marker.clone()),
        orientation_metadata_source: bridge
            .and_then(|value| value.orientation_metadata_source.clone()),
        orientation_default: bridge.and_then(|value| value.orientation_default),
        stimulus_raster_orientation: bridge
            .and_then(|value| value.stimulus_raster_orientation.clone()),
        stimulus_upright_marker: bridge.and_then(|value| value.stimulus_upright_marker.clone()),
        stimulus_orientation_default: bridge.and_then(|value| value.stimulus_orientation_default),
        content_kind: bridge.and_then(|value| value.content_kind.clone()),
        content_width: bridge.and_then(|value| value.content_width),
        content_height: bridge.and_then(|value| value.content_height),
        content_aspect_ratio: bridge.and_then(|value| value.content_aspect_ratio),
        desired_display_aspect_ratio: bridge.and_then(|value| value.desired_display_aspect_ratio),
        desired_projection_aspect_ratio: bridge
            .and_then(|value| value.desired_projection_aspect_ratio),
        content_coordinate_space: bridge.and_then(|value| value.content_coordinate_space.clone()),
        content_origin: bridge.and_then(|value| value.content_origin.clone()),
        content_x_axis: bridge.and_then(|value| value.content_x_axis.clone()),
        content_y_axis: bridge.and_then(|value| value.content_y_axis.clone()),
        content_uv_rect: bridge
            .and_then(|value| value.content_uv_rect)
            .and_then(public_uv_rect),
        source_visible_uv_rect: bridge
            .and_then(|value| value.source_visible_uv_rect)
            .and_then(public_uv_rect),
        source_crop_rect_px: bridge
            .and_then(|value| value.source_crop_rect_px)
            .and_then(public_pixel_rect),
        source_crop_rect_state: bridge.and_then(|value| value.source_crop_rect_state.clone()),
        source_crop_rect_owner: bridge.and_then(|value| value.source_crop_rect_owner.clone()),
        source_sampling_mode: bridge.and_then(|value| value.source_sampling_mode.clone()),
        content_mapping_intent: bridge.and_then(|value| value.content_mapping_intent.clone()),
        content_geometry_metadata_source: bridge
            .and_then(|value| value.content_geometry_metadata_source.clone()),
        content_geometry_default: bridge.and_then(|value| value.content_geometry_default),
        target_footprint_schema: bridge.and_then(|value| value.target_footprint_schema.clone()),
        target_coordinate_space: bridge.and_then(|value| value.target_coordinate_space.clone()),
        target_screen_uv_rect: bridge
            .and_then(|value| value.target_screen_uv_rect)
            .and_then(public_target_screen_uv_rect),
        target_clip_policy: bridge.and_then(|value| value.target_clip_policy.clone()),
        target_footprint_metadata_source: bridge
            .and_then(|value| value.target_footprint_metadata_source.clone()),
        target_footprint_default: bridge.and_then(|value| value.target_footprint_default),
        requested_stereo_layout: bridge.and_then(|value| value.requested_stereo_layout.clone()),
        stereo_layout: bridge
            .and_then(|value| value.stereo_layout.as_deref())
            .and_then(parse_stereo_layout)
            .unwrap_or(StereoMediaLayout::Mono),
        mono_fallback: bridge.and_then(|value| value.mono_fallback).unwrap_or(true),
        fallback_reason: bridge
            .and_then(|value| value.fallback_reason.clone())
            .unwrap_or_else(|| {
                if missing_intrinsics {
                    "missing intrinsics; diagnostic flat camera copy".to_string()
                } else if missing_pose {
                    "missing camera pose; diagnostic flat camera copy".to_string()
                } else {
                    "metadata-backed projection not active in CPU copy path".to_string()
                }
            }),
    };
    let _gpu_import_requested = bridge.and_then(|value| value.gpu_import_requested);

    (metadata, diagnostics)
}

fn public_pixel_domain(domain: JavaPixelDomain) -> Option<CameraPixelDomain> {
    let size = ImageSize::new(domain.width, domain.height);
    if !size.is_non_empty() {
        return None;
    }

    let kind = match domain.kind {
        JavaPixelDomainKind::DeliveredImage => CameraPixelDomainKind::DeliveredImage,
        JavaPixelDomainKind::ActiveArray => CameraPixelDomainKind::ActiveArray,
        JavaPixelDomainKind::SensorPixelArray => CameraPixelDomainKind::SensorPixelArray,
        JavaPixelDomainKind::Other => CameraPixelDomainKind::Other,
    };
    Some(CameraPixelDomain::new(kind, size))
}

fn public_uv_rect(rect: JavaUvRect) -> Option<[f32; 4]> {
    let values = [rect.left, rect.top, rect.right, rect.bottom];
    if values.iter().all(|value| value.is_finite())
        && rect.right >= rect.left
        && rect.bottom >= rect.top
    {
        Some([
            rect.left.clamp(0.0, 1.0),
            rect.top.clamp(0.0, 1.0),
            rect.right.clamp(0.0, 1.0),
            rect.bottom.clamp(0.0, 1.0),
        ])
    } else {
        None
    }
}

fn public_target_screen_uv_rect(rect: JavaTargetScreenUvRect) -> Option<[f32; 4]> {
    let values = [rect.x, rect.y, rect.width, rect.height];
    (values.iter().all(|value| value.is_finite()) && rect.width > 0.0 && rect.height > 0.0)
        .then_some(values)
}

fn public_pixel_rect(rect: JavaPixelRect) -> Option<[u32; 4]> {
    if rect.right >= rect.left && rect.bottom >= rect.top {
        Some([rect.left, rect.top, rect.right, rect.bottom])
    } else {
        None
    }
}

fn parse_stereo_layout(value: &str) -> Option<StereoMediaLayout> {
    match value {
        "mono" => Some(StereoMediaLayout::Mono),
        "side-by-side" => Some(StereoMediaLayout::SIDE_BY_SIDE_LEFT_FIRST),
        "top-bottom" => Some(StereoMediaLayout::TOP_BOTTOM_LEFT_FIRST),
        "separate" => Some(StereoMediaLayout::Separate),
        _ => None,
    }
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn diagnostics_source_label(bridge: Option<&JavaCameraFrameMetadata>) -> String {
    bridge
        .and_then(|value| value.source_label.clone())
        .unwrap_or_else(|| "Camera2 unknown".to_string())
}

pub fn contract_json() -> String {
    let diagnostic_hints = StereoLayerPerformanceHints {
        preferred_camera_path: StereoLayerCameraPath::CpuYuv,
        allow_cpu_visible_path: true,
        ..StereoLayerPerformanceHints::QUEST_STEREO_CAMERA_BASELINE
    };
    let layer = PlainStereoLayer::new(ImageSize::new(1280, 1280), Vec2::new(1.0, 1.0))
        .with_source_layout(StereoMediaLayout::Mono)
        .with_content_mode(StereoLayerContentMode::Fit)
        .with_pose(Pose::new(Vec3::new(0.0, 0.0, -1.35), Quat::IDENTITY))
        .with_border(VisualFeedbackBorder::new(0.018, ColorRgba::WHITE).with_opacity(0.86))
        .with_performance_hints(diagnostic_hints);

    let session = CompositeLayerSession {
        schema_version: "rusty.xr.quest-app-catalog.v1",
        app_id: "rusty-xr-quest-composite-layer",
        package_name: "com.example.rustyxr.composite",
        activity_name: ".CompositeLayerActivity",
        layer_kind: "openxr-vulkan-diagnostic-flat-camera-copy",
        render_path: "diagnostic-flat-camera-copy",
        alignment_mode: "metadata-backed projection inactive; falls back when intrinsics or pose are missing",
        stereo_layout: StereoMediaLayout::Mono,
        feedback_layer: layer,
        content_rect: layer
            .content_rect()
            .expect("public composite layer should have a valid content rect"),
        border_layout: layer
            .border_layout()
            .expect("public composite layer should have a valid border layout"),
        capture_sources: [
            CaptureSourceState::new(CaptureSourceKind::AppRender)
                .with_lifecycle(CaptureLifecycleState::Running)
                .with_permission(CapturePermissionState::NotRequired),
            CaptureSourceState::new(CaptureSourceKind::PassthroughCamera)
                .with_lifecycle(CaptureLifecycleState::PermissionRequired)
                .with_permission(CapturePermissionState::Required),
            CaptureSourceState::new(CaptureSourceKind::MediaProjection)
                .with_lifecycle(CaptureLifecycleState::PermissionRequired)
                .with_permission(CapturePermissionState::Required),
            CaptureSourceState::new(CaptureSourceKind::Synthetic)
                .with_lifecycle(CaptureLifecycleState::Running)
                .with_permission(CapturePermissionState::NotRequired),
        ],
        environment_depth: EnvironmentDepthState {
            supported: false,
            permission_granted: false,
            provider_created: false,
            provider_running: false,
            frame_available: false,
        },
        notes: [
            "This APK is the public immersive camera-composite scaffold.",
            "Tier 0 is a synthetic OpenXR/Vulkan smoke test; Tier 1 is a diagnostic flat CPU camera copy; Tier 2 is the intended GPU-projected stereo camera path.",
            "The Java bridge passes public camera metadata to Rust, including source label, delivered size, timestamp, optional sensor orientation, optional pixel domains, optional intrinsics, and missing-metadata flags.",
            "The current visible path is still Tier 1 unless a GPU-imported camera buffer and metadata-backed projection renderer are active.",
            "Tier 2 hardware-buffer frames are probed through a public Android hardware-buffer bridge, but the diagnostic copy path is not renamed as aligned projection.",
            "MediaProjection consent is only used for Windows/operator screen streaming.",
            "True camera/view alignment requires metadata-backed per-eye projection and is not claimed by the CPU copy path.",
        ],
    };

    serde_json::to_string_pretty(&session).expect("composite session should serialize")
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_CompositeLayerActivity_nativeRuntimeConfig(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    config_json: JString<'_>,
) {
    let config_json = env
        .get_string(&config_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok();
    store_runtime_config(config_json);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_CompositeLayerActivity_nativeProjectionTargetControl(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    control_json: JString<'_>,
) {
    let Some(control_json) = env
        .get_string(&control_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok()
    else {
        log_error("Rusty XR projection-target control bridge received invalid JNI string");
        return;
    };
    let Ok(control) = serde_json::from_str::<JavaProjectionTargetControl>(&control_json) else {
        log_error("Rusty XR projection-target control bridge could not parse control JSON");
        return;
    };
    let Some(scale) = control
        .projection_target_scale
        .filter(|value| value.is_finite())
    else {
        log_error("Rusty XR projection-target control bridge missing finite projectionTargetScale");
        return;
    };
    apply_projection_target_scale_update(scale);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_CompositeLayerActivity_contractJson(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    match env.new_string(contract_json()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_CompositeLayerActivity_nativeActivityEvent(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    event_json: JString<'_>,
) {
    log_jni_event(&mut env, "activity", event_json);
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_CompositeLayerActivity_nativeStartNativeCamera(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    config_json: JString<'_>,
) -> jboolean {
    let config_json = env
        .get_string(&config_json)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "{}".to_string());
    match native_camera::start_from_json(&config_json) {
        Ok(()) => 1,
        Err(error) => {
            log_error(format!(
                "Rusty XR native camera acquisition failed: {error}"
            ));
            0
        }
    }
}

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_CompositeLayerActivity_nativeStopNativeCamera(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
) {
    native_camera::stop();
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_MediaProjectionStreamService_nativeMediaProjectionEvent(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    event_json: JString<'_>,
) {
    log_jni_event(&mut env, "media", event_json);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_HeadsetCameraService_nativeHeadsetCameraEvent(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    event_json: JString<'_>,
) {
    log_jni_event(&mut env, "headset_camera", event_json);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_HeadsetCameraService_nativeHeadsetCameraFrame(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    width: jint,
    height: jint,
    timestamp_ns: jlong,
    metadata_json: JString<'_>,
    rgba: JByteArray<'_>,
) {
    if width <= 0 || height <= 0 {
        #[cfg(target_os = "android")]
        log_error("Rusty XR received headset camera frame with invalid dimensions");
        return;
    }

    let metadata_json = env
        .get_string(&metadata_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok();

    match env.convert_byte_array(&rgba) {
        Ok(bytes) => {
            store_headset_camera_frame(
                width as u32,
                height as u32,
                timestamp_ns,
                metadata_json,
                bytes,
            );
        }
        Err(_error) => {
            #[cfg(target_os = "android")]
            log_error(format!(
                "Rusty XR could not copy headset camera frame: {_error}"
            ));
        }
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_HeadsetCameraService_nativeHeadsetCameraHardwareBufferFrame(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    width: jint,
    height: jint,
    timestamp_ns: jlong,
    metadata_json: JString<'_>,
    hardware_buffer: JObject<'_>,
    hardware_buffer_format: jint,
    hardware_buffer_usage: jlong,
    hardware_buffer_layers: jint,
    hardware_buffer_id: jlong,
) -> jboolean {
    if width <= 0 || height <= 0 {
        #[cfg(target_os = "android")]
        log_error("Rusty XR received headset camera hardware buffer with invalid dimensions");
        return 0;
    }

    let metadata_json = env
        .get_string(&metadata_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok();

    let gpu_buffer = match probe_android_hardware_buffer_descriptor(
        &mut env,
        &hardware_buffer,
        width as u32,
        height as u32,
        hardware_buffer_format,
        hardware_buffer_usage,
        hardware_buffer_layers,
        hardware_buffer_id,
    ) {
        Ok(descriptor) => descriptor,
        Err(_error) => {
            #[cfg(target_os = "android")]
            log_error(format!("Rusty XR hardware-buffer probe failed: {_error}"));
            if let Ok(mut state) = headset_camera_state().lock() {
                state.gpu_probe_failure_count = state.gpu_probe_failure_count.saturating_add(1);
            }
            return 0;
        }
    };

    if store_headset_camera_gpu_frame(
        width as u32,
        height as u32,
        timestamp_ns,
        metadata_json,
        gpu_buffer,
    ) {
        1
    } else {
        0
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_BrokerH264ConsumerProbe_nativeBrokerH264DecodedHardwareBufferFrame(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    width: jint,
    height: jint,
    timestamp_ns: jlong,
    metadata_json: JString<'_>,
    hardware_buffer: JObject<'_>,
    hardware_buffer_format: jint,
    hardware_buffer_usage: jlong,
    hardware_buffer_layers: jint,
    hardware_buffer_id: jlong,
) -> jboolean {
    if width <= 0 || height <= 0 {
        #[cfg(target_os = "android")]
        log_error("Rusty XR received broker H.264 decoded hardware buffer with invalid dimensions");
        return 0;
    }

    let metadata_json = env
        .get_string(&metadata_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok();

    let gpu_buffer = match probe_android_hardware_buffer_descriptor_with_label(
        &mut env,
        &hardware_buffer,
        width as u32,
        height as u32,
        hardware_buffer_format,
        hardware_buffer_usage,
        hardware_buffer_layers,
        hardware_buffer_id,
        "Broker H.264 decoded AHardwareBuffer",
    ) {
        Ok(descriptor) => descriptor,
        Err(_error) => {
            #[cfg(target_os = "android")]
            log_error(format!(
                "Rusty XR broker H.264 decoded hardware-buffer probe failed: {_error}"
            ));
            if let Ok(mut state) = headset_camera_state().lock() {
                state.gpu_probe_failure_count = state.gpu_probe_failure_count.saturating_add(1);
            }
            return 0;
        }
    };

    if store_headset_camera_gpu_frame(
        width as u32,
        height as u32,
        timestamp_ns,
        metadata_json,
        gpu_buffer,
    ) {
        1
    } else {
        0
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_BrokerH264ConsumerProbe_nativeBrokerH264DecodedStereoHardwareBufferFrame(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    left_width: jint,
    left_height: jint,
    left_timestamp_ns: jlong,
    left_metadata_json: JString<'_>,
    left_hardware_buffer: JObject<'_>,
    left_hardware_buffer_format: jint,
    left_hardware_buffer_usage: jlong,
    left_hardware_buffer_layers: jint,
    left_hardware_buffer_id: jlong,
    right_width: jint,
    right_height: jint,
    right_timestamp_ns: jlong,
    right_metadata_json: JString<'_>,
    right_hardware_buffer: JObject<'_>,
    right_hardware_buffer_format: jint,
    right_hardware_buffer_usage: jlong,
    right_hardware_buffer_layers: jint,
    right_hardware_buffer_id: jlong,
    pair_delta_ns: jlong,
    pair_index: jlong,
) -> jboolean {
    if left_width <= 0 || left_height <= 0 || right_width <= 0 || right_height <= 0 {
        #[cfg(target_os = "android")]
        log_error(
            "Rusty XR received broker H.264 decoded stereo hardware buffer with invalid dimensions",
        );
        return 0;
    }

    let left_metadata_json = env
        .get_string(&left_metadata_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok();
    let right_metadata_json = env
        .get_string(&right_metadata_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok();

    let left_gpu_buffer = match probe_android_hardware_buffer_descriptor_with_label(
        &mut env,
        &left_hardware_buffer,
        left_width as u32,
        left_height as u32,
        left_hardware_buffer_format,
        left_hardware_buffer_usage,
        left_hardware_buffer_layers,
        left_hardware_buffer_id,
        "Broker H.264 decoded left AHardwareBuffer",
    ) {
        Ok(descriptor) => descriptor,
        Err(_error) => {
            #[cfg(target_os = "android")]
            log_error(format!(
                "Rusty XR broker H.264 decoded left stereo hardware-buffer probe failed: {_error}"
            ));
            if let Ok(mut state) = headset_camera_state().lock() {
                state.gpu_probe_failure_count = state.gpu_probe_failure_count.saturating_add(1);
            }
            return 0;
        }
    };
    let right_gpu_buffer = match probe_android_hardware_buffer_descriptor_with_label(
        &mut env,
        &right_hardware_buffer,
        right_width as u32,
        right_height as u32,
        right_hardware_buffer_format,
        right_hardware_buffer_usage,
        right_hardware_buffer_layers,
        right_hardware_buffer_id,
        "Broker H.264 decoded right AHardwareBuffer",
    ) {
        Ok(descriptor) => descriptor,
        Err(_error) => {
            #[cfg(target_os = "android")]
            log_error(format!(
                "Rusty XR broker H.264 decoded right stereo hardware-buffer probe failed: {_error}"
            ));
            if let Ok(mut state) = headset_camera_state().lock() {
                state.gpu_probe_failure_count = state.gpu_probe_failure_count.saturating_add(1);
            }
            return 0;
        }
    };

    if store_headset_stereo_camera_gpu_frame(
        left_width as u32,
        left_height as u32,
        left_timestamp_ns,
        left_metadata_json,
        left_gpu_buffer,
        right_width as u32,
        right_height as u32,
        right_timestamp_ns,
        right_metadata_json,
        right_gpu_buffer,
        pair_delta_ns.max(0) as u64,
        pair_index.max(0) as u64,
    ) {
        1
    } else {
        0
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_composite_HeadsetCameraService_nativeHeadsetStereoCameraHardwareBufferFrame(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    left_width: jint,
    left_height: jint,
    left_timestamp_ns: jlong,
    left_metadata_json: JString<'_>,
    left_hardware_buffer: JObject<'_>,
    left_hardware_buffer_format: jint,
    left_hardware_buffer_usage: jlong,
    left_hardware_buffer_layers: jint,
    left_hardware_buffer_id: jlong,
    right_width: jint,
    right_height: jint,
    right_timestamp_ns: jlong,
    right_metadata_json: JString<'_>,
    right_hardware_buffer: JObject<'_>,
    right_hardware_buffer_format: jint,
    right_hardware_buffer_usage: jlong,
    right_hardware_buffer_layers: jint,
    right_hardware_buffer_id: jlong,
    pair_delta_ns: jlong,
    pair_index: jlong,
) -> jboolean {
    if left_width <= 0 || left_height <= 0 || right_width <= 0 || right_height <= 0 {
        #[cfg(target_os = "android")]
        log_error(
            "Rusty XR received stereo headset camera hardware buffer with invalid dimensions",
        );
        return 0;
    }

    let left_metadata_json = env
        .get_string(&left_metadata_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok();
    let right_metadata_json = env
        .get_string(&right_metadata_json)
        .map(|value| value.to_string_lossy().into_owned())
        .ok();

    let left_gpu_buffer = match probe_android_hardware_buffer_descriptor(
        &mut env,
        &left_hardware_buffer,
        left_width as u32,
        left_height as u32,
        left_hardware_buffer_format,
        left_hardware_buffer_usage,
        left_hardware_buffer_layers,
        left_hardware_buffer_id,
    ) {
        Ok(descriptor) => descriptor,
        Err(_error) => {
            #[cfg(target_os = "android")]
            log_error(format!(
                "Rusty XR left stereo hardware-buffer probe failed: {_error}"
            ));
            return 0;
        }
    };
    let right_gpu_buffer = match probe_android_hardware_buffer_descriptor(
        &mut env,
        &right_hardware_buffer,
        right_width as u32,
        right_height as u32,
        right_hardware_buffer_format,
        right_hardware_buffer_usage,
        right_hardware_buffer_layers,
        right_hardware_buffer_id,
    ) {
        Ok(descriptor) => descriptor,
        Err(_error) => {
            #[cfg(target_os = "android")]
            log_error(format!(
                "Rusty XR right stereo hardware-buffer probe failed: {_error}"
            ));
            return 0;
        }
    };

    if store_headset_stereo_camera_gpu_frame(
        left_width as u32,
        left_height as u32,
        left_timestamp_ns,
        left_metadata_json,
        left_gpu_buffer,
        right_width as u32,
        right_height as u32,
        right_timestamp_ns,
        right_metadata_json,
        right_gpu_buffer,
        pair_delta_ns.max(0) as u64,
        pair_index.max(0) as u64,
    ) {
        1
    } else {
        0
    }
}

#[cfg(target_os = "android")]
#[allow(clippy::too_many_arguments)]
fn probe_android_hardware_buffer_descriptor(
    env: &mut JNIEnv<'_>,
    hardware_buffer: &JObject<'_>,
    width: u32,
    height: u32,
    hardware_buffer_format: jint,
    hardware_buffer_usage: jlong,
    hardware_buffer_layers: jint,
    hardware_buffer_id: jlong,
) -> Result<HeadsetCameraGpuBufferImport, String> {
    probe_android_hardware_buffer_descriptor_with_label(
        env,
        hardware_buffer,
        width,
        height,
        hardware_buffer_format,
        hardware_buffer_usage,
        hardware_buffer_layers,
        hardware_buffer_id,
        "Camera2 PRIVATE AHardwareBuffer",
    )
}

#[cfg(target_os = "android")]
#[allow(clippy::too_many_arguments)]
fn probe_android_hardware_buffer_descriptor_with_label(
    env: &mut JNIEnv<'_>,
    hardware_buffer: &JObject<'_>,
    width: u32,
    height: u32,
    hardware_buffer_format: jint,
    hardware_buffer_usage: jlong,
    hardware_buffer_layers: jint,
    hardware_buffer_id: jlong,
    descriptor_label: &'static str,
) -> Result<HeadsetCameraGpuBufferImport, String> {
    let buffer = unsafe {
        ndk_sys::AHardwareBuffer_fromHardwareBuffer(
            env.get_native_interface().cast(),
            hardware_buffer.as_raw().cast(),
        )
    };
    if buffer.is_null() {
        return Err("AHardwareBuffer_fromHardwareBuffer returned null".to_string());
    }
    let hardware_buffer = AndroidHardwareBufferHandle::acquire(buffer)?;

    let mut desc = std::mem::MaybeUninit::<ndk_sys::AHardwareBuffer_Desc>::zeroed();
    unsafe {
        ndk_sys::AHardwareBuffer_describe(buffer, desc.as_mut_ptr());
    }
    let desc = unsafe { desc.assume_init() };
    let mut descriptor = CameraGpuBufferDescriptor::new(
        descriptor_label,
        ImageSize::new(width, height),
        "AHardwareBuffer",
    )
    .with_native_format(desc.format as u64)
    .with_usage_flags(desc.usage)
    .with_layer_count(desc.layers)
    .with_stride_px(desc.stride);

    let mut native_id = 0_u64;
    let id_result = unsafe { ndk_sys::AHardwareBuffer_getId(buffer, &mut native_id) };
    if id_result == 0 && native_id != 0 {
        descriptor = descriptor.with_buffer_id(native_id);
    } else if hardware_buffer_id > 0 {
        descriptor = descriptor.with_buffer_id(hardware_buffer_id as u64);
    }

    if descriptor.native_format == Some(0) && hardware_buffer_format > 0 {
        descriptor.native_format = Some(hardware_buffer_format as u64);
    }
    if descriptor.usage_flags == Some(0) && hardware_buffer_usage > 0 {
        descriptor.usage_flags = Some(hardware_buffer_usage as u64);
    }
    if descriptor.layer_count == Some(0) && hardware_buffer_layers > 0 {
        descriptor.layer_count = Some(hardware_buffer_layers as u32);
    }

    Ok(HeadsetCameraGpuBufferImport {
        descriptor,
        hardware_buffer,
    })
}

#[cfg(not(target_os = "android"))]
#[allow(clippy::too_many_arguments)]
fn probe_android_hardware_buffer_descriptor(
    _env: &mut JNIEnv<'_>,
    _hardware_buffer: &JObject<'_>,
    width: u32,
    height: u32,
    hardware_buffer_format: jint,
    hardware_buffer_usage: jlong,
    hardware_buffer_layers: jint,
    hardware_buffer_id: jlong,
) -> Result<HeadsetCameraGpuBufferImport, String> {
    probe_android_hardware_buffer_descriptor_with_label(
        _env,
        _hardware_buffer,
        width,
        height,
        hardware_buffer_format,
        hardware_buffer_usage,
        hardware_buffer_layers,
        hardware_buffer_id,
        "Camera2 PRIVATE AHardwareBuffer",
    )
}

#[cfg(not(target_os = "android"))]
#[allow(clippy::too_many_arguments)]
fn probe_android_hardware_buffer_descriptor_with_label(
    _env: &mut JNIEnv<'_>,
    _hardware_buffer: &JObject<'_>,
    width: u32,
    height: u32,
    hardware_buffer_format: jint,
    hardware_buffer_usage: jlong,
    hardware_buffer_layers: jint,
    hardware_buffer_id: jlong,
    descriptor_label: &'static str,
) -> Result<HeadsetCameraGpuBufferImport, String> {
    let mut descriptor = CameraGpuBufferDescriptor::new(
        descriptor_label,
        ImageSize::new(width, height),
        "AHardwareBuffer",
    );
    if hardware_buffer_format > 0 {
        descriptor = descriptor.with_native_format(hardware_buffer_format as u64);
    }
    if hardware_buffer_usage > 0 {
        descriptor = descriptor.with_usage_flags(hardware_buffer_usage as u64);
    }
    if hardware_buffer_layers > 0 {
        descriptor = descriptor.with_layer_count(hardware_buffer_layers as u32);
    }
    if hardware_buffer_id > 0 {
        descriptor = descriptor.with_buffer_id(hardware_buffer_id as u64);
    }
    Ok(HeadsetCameraGpuBufferImport { descriptor })
}

fn log_jni_event(env: &mut JNIEnv<'_>, channel: &str, event_json: JString<'_>) {
    let event = env
        .get_string(&event_json)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "{\"event\":\"invalidJniString\"}".to_string());
    let message = format!("Rusty XR composite {channel} event: {event}");
    #[cfg(target_os = "android")]
    log_info(message);
    #[cfg(not(target_os = "android"))]
    println!("{message}");
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_on_create(_state: &android_activity::OnCreateState) {
    log_info("Rusty XR composite native activity created");
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    log_info(format!(
        "Rusty XR composite layer contract: {}",
        contract_json()
    ));
    let app_for_error = app.clone();
    if let Err(error) = openxr_vulkan::run(app) {
        log_error(format!("Rusty XR OpenXR loop failed: {error}"));
        keep_activity_alive_after_error(app_for_error);
    }
}

#[cfg(target_os = "android")]
fn keep_activity_alive_after_error(app: android_activity::AndroidApp) {
    use std::time::Duration;

    use android_activity::{MainEvent, PollEvent};

    log_info("Rusty XR keeping activity alive after OpenXR setup failure");
    let mut running = true;
    while running {
        app.poll_events(Some(Duration::from_millis(250)), |event| {
            if let PollEvent::Main(MainEvent::Destroy) = event {
                running = false;
            }
        });
    }
    log_info("Rusty XR post-error keepalive exited");
}

#[cfg(test)]
mod tests {
    use super::projection_runtime::public_projection_runtime_config;
    use super::{
        apply_hwb_projection_runtime_resolution, contract_json, parse_diagnostic_hud_command,
        public_camera_metadata, public_runtime_config, CameraColorMode, CameraFeedPipelineMode,
        CameraFrameAdoptionMode, CameraImportImageLayoutMode, CameraOrientationDiagnosticMode,
        CameraPeripheralStretchBlendMode, CameraPipelinePreset, CameraProcessingLayer,
        CameraProjectionAlphaMode, CameraProjectionBorderPolicy, CameraProjectionEffectMode,
        CameraProjectionMode, CameraSamplerBindingMode, EnvironmentDepthMode, HandParticleMode,
        JavaCameraExtrinsics, JavaCameraFrameMetadata, JavaCameraIntrinsics, JavaPixelDomain,
        JavaPixelDomainKind, JavaRuntimeConfig, OpenXrColorFormatMode, OpenXrPassthroughProbeMode,
        OpenXrPassthroughStyleMode, ProjectionTargetJoystickControls, RuntimeConfig,
        StereoSourceEyeMapping,
    };
    use rusty_xr_contracts::{
        CameraCompositeTier, CameraImageRotation, CameraPixelDomainKind, ImageSize,
        TemporalProjectionEdgeMode, TemporalProjectionMode,
    };
    use rusty_xr_debug_canvas::DiagnosticHudCommand;
    use rusty_xr_runtime_config as rxrc;

    #[test]
    fn contract_json_contains_public_identity_and_media_projection() {
        let json = contract_json();

        assert!(json.contains("\"appId\": \"rusty-xr-quest-composite-layer\""));
        assert!(json.contains("\"packageName\": \"com.example.rustyxr.composite\""));
        assert!(json.contains("\"renderPath\": \"diagnostic-flat-camera-copy\""));
        assert!(json.contains("\"layerKind\": \"openxr-vulkan-diagnostic-flat-camera-copy\""));
        assert!(json.contains("\"source_kind\": \"MediaProjection\""));
        assert!(json.contains("\"preferred_camera_path\": \"CpuYuv\""));
    }

    #[test]
    fn runtime_config_defaults_to_clean_projection_border() {
        let config = public_runtime_config(&JavaRuntimeConfig::default());

        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::SolidRed
        );
        assert!(config.camera_projection_border_policy_active());
        assert_eq!(
            config.camera_projection_border_policy_shader_bit(),
            super::camera_color_pipeline::CAMERA_SHADER_FLAG_PROJECTION_BORDER_SOLID_RED
        );
    }

    #[test]
    fn runtime_config_keeps_feedback_border_effect_explicit() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_projection_effect_mode: Some("border-composite".to_string()),
            projection_border_policy: Some("solid-red".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::BorderComposite
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::SolidRed
        );
        assert!(!config.camera_projection_border_policy_active());
    }

    #[test]
    fn runtime_config_parses_separate_immutable_sampler_probe() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_sampler_binding_mode: Some("separate-immutable-sampler".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::SeparateImmutableSampler
        );
    }

    #[test]
    fn java_camera_metadata_marks_mono_missing_pose_fallback() {
        let bridge = JavaCameraFrameMetadata {
            source: None,
            source_label: Some("Camera2 7 back".to_string()),
            camera_id: Some("7".to_string()),
            lens_facing: Some("back".to_string()),
            lens_facing_rank: Some(2),
            selection_score: Some(42),
            delivered_width: Some(1280),
            delivered_height: Some(1280),
            timestamp_ns: Some(123),
            sensor_orientation_degrees: Some(90),
            intrinsics: Some(JavaCameraIntrinsics {
                fx: 2000.0,
                fy: 2000.0,
                cx: 1000.0,
                cy: 1000.0,
                skew: 0.0,
            }),
            intrinsics_domain: Some(JavaPixelDomain {
                kind: JavaPixelDomainKind::ActiveArray,
                width: 2000,
                height: 2000,
            }),
            active_array_domain: None,
            sensor_pixel_domain: Some(JavaPixelDomain {
                kind: JavaPixelDomainKind::SensorPixelArray,
                width: 2200,
                height: 2200,
            }),
            stereo_layout: Some("mono".to_string()),
            requested_stereo_layout: Some("mono".to_string()),
            transport: Some("cpu-yuv-rgba".to_string()),
            requested_tier: Some("cpu-diagnostic-flat-copy".to_string()),
            active_tier: Some("cpu-diagnostic-flat-copy".to_string()),
            gpu_import_requested: Some(false),
            pose_source: Some("missing".to_string()),
            pose_coordinate_convention: None,
            lens_pose_reference_label: None,
            diagnostic_source: None,
            synthetic_projection_profile: None,
            projection_geometry_profile: None,
            synthetic_pattern: None,
            extrinsics: None,
            missing_intrinsics: Some(false),
            missing_pose: Some(true),
            mono_fallback: Some(true),
            fallback_reason: Some("missing camera pose; diagnostic flat camera copy".to_string()),
            ..Default::default()
        };

        let (metadata, diagnostics) = public_camera_metadata(Some(&bridge), 5, 640, 480, 123);

        assert_eq!(metadata.delivered_size, ImageSize::new(1280, 1280));
        assert_eq!(
            metadata.intrinsics_domain.unwrap().kind,
            CameraPixelDomainKind::ActiveArray
        );
        assert!(metadata.has_intrinsics());
        assert!(!metadata.has_pose());
        assert!(metadata.flags.missing_pose);
        assert!(diagnostics.mono_fallback);
    }

    #[test]
    fn java_camera_metadata_accepts_explicit_estimated_pose() {
        let bridge = JavaCameraFrameMetadata {
            source_label: Some("Camera2 estimated".to_string()),
            delivered_width: Some(1280),
            delivered_height: Some(1280),
            intrinsics: None,
            intrinsics_domain: None,
            active_array_domain: None,
            sensor_pixel_domain: None,
            pose_source: Some("estimated-profile".to_string()),
            extrinsics: Some(JavaCameraExtrinsics {
                px: 0.01,
                py: 0.0,
                pz: -0.02,
                qx: 0.0,
                qy: 0.0,
                qz: 0.0,
                qw: 1.0,
            }),
            missing_pose: Some(false),
            ..Default::default()
        };

        let (metadata, diagnostics) = public_camera_metadata(Some(&bridge), 6, 1280, 1280, 456);

        assert!(metadata.has_pose());
        assert_eq!(
            diagnostics.pose_source.as_deref(),
            Some("estimated-profile")
        );
    }

    #[test]
    fn java_platform_camera2_pose_is_converted_to_world_from_camera() {
        let s = core::f32::consts::FRAC_1_SQRT_2;
        let bridge = JavaCameraFrameMetadata {
            source_label: Some("Camera2 platform".to_string()),
            delivered_width: Some(1280),
            delivered_height: Some(1280),
            pose_source: Some("platform".to_string()),
            pose_coordinate_convention: Some(
                "android-camera2-lens-pose-reference-from-camera".to_string(),
            ),
            lens_pose_reference_label: Some("GYROSCOPE".to_string()),
            extrinsics: Some(JavaCameraExtrinsics {
                px: 0.03,
                py: 0.0,
                pz: -0.07,
                qx: 0.0,
                qy: 0.0,
                qz: s,
                qw: s,
            }),
            missing_pose: Some(false),
            ..Default::default()
        };

        let (metadata, diagnostics) = public_camera_metadata(Some(&bridge), 7, 1280, 1280, 789);
        let extrinsics = metadata.extrinsics.expect("platform pose should convert");

        assert!(metadata.has_pose());
        assert!((extrinsics.world_from_camera.orientation.z + s).abs() < 1.0e-6);
        assert_eq!(
            diagnostics.lens_pose_reference_label.as_deref(),
            Some("GYROSCOPE")
        );
    }

    #[test]
    fn runtime_config_parses_public_projection_and_render_knobs() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_tier: Some("gpu-projected".to_string()),
            camera_acquisition: Some("native-ndk".to_string()),
            camera_enabled: Some(true),
            media_projection_enabled: Some(false),
            allow_cpu_fallback: Some(false),
            cpu_upload_hz: Some(0),
            stereo_layout: Some("separate".to_string()),
            projection_runtime_resolution_enabled: Some(true),
            camera_projection_fov_y_degrees: Some(92.0),
            camera_preview_fov_y_degrees: Some(60.0),
            camera_preview_offset_y_meters: Some(0.125),
            camera_projection_scale: Some(0.75),
            projection_depth_meters: Some(1.25),
            projection_area_scale_uv: Some(0.815),
            projection_area_offset_x_uv: Some(0.125),
            projection_area_offset_y_uv: Some(-0.25),
            projection_area_left_offset_x_uv: Some(-0.03125),
            projection_area_left_offset_y_uv: Some(0.0625),
            projection_area_right_offset_x_uv: Some(0.09375),
            projection_area_right_offset_y_uv: None,
            projection_area_radius_x_uv: Some(0.5),
            projection_area_radius_y_uv: Some(0.5),
            projection_area_corner_radius_uv: Some(0.0),
            projection_area_opacity: Some(0.42),
            projection_border_opacity: Some(0.85),
            projection_target_offset_x_uv: Some(0.0625),
            projection_target_offset_y_uv: Some(-0.03125),
            projection_target_scale: Some(0.75),
            projection_target_joystick_controls: Some("offset-scale".to_string()),
            projection_alpha_mode: Some("green".to_string()),
            projection_alpha_scale: Some(1.5),
            projection_alpha_bias: Some(-0.1),
            camera_raw_overlay_overscan: Some(1.06),
            camera_full_view_overlay_overscan: Some(2.15),
            camera_edge_fade: Some(0.06),
            camera_texture_rotation: Some("rotate180".to_string()),
            camera_texture_flip_x: Some(false),
            camera_texture_flip_y: Some(false),
            camera_texture_mirror: Some(false),
            camera_texture_transform_source: Some("public-live-check".to_string()),
            camera_texture_transform_reason: Some("upright texture validation".to_string()),
            camera_projection_mode: Some("quad-surface".to_string()),
            camera_pipeline_preset: None,
            camera_projection_effect_mode: Some("raw-projection".to_string()),
            projection_border_policy: Some("solid-red".to_string()),
            processing_layer: Some("blur".to_string()),
            peripheral_stretch_mode: Some("edge-stretch".to_string()),
            peripheral_stretch_core_scale: Some(1.0),
            peripheral_stretch_edge_inset_uv: Some(0.015),
            peripheral_stretch_max_inset_uv: Some(0.14),
            peripheral_stretch_curve: Some(1.6),
            peripheral_stretch_inner_blend_uv: Some(0.04),
            peripheral_stretch_blend_curve: Some(1.6),
            peripheral_stretch_blend_mode: Some("target-inner-band".to_string()),
            peripheral_stretch_corner_mode: Some("target-footprint".to_string()),
            peripheral_stretch_debug: Some("off".to_string()),
            camera_color_mode: Some("external-rgb".to_string()),
            camera_sampler_binding_mode: Some("separate-image-sampler".to_string()),
            camera_import_image_layout_mode: Some("general-no-transition".to_string()),
            camera_import_cache_limit: Some(2),
            camera_color_matrix: Some("0.9;0.1;0;0;1;0;0;0.2;0.8".to_string()),
            camera_color_offset: Some("-0.01;0.02;0.03".to_string()),
            camera_color_contrast: Some(1.1),
            camera_color_brightness: Some(0.04),
            camera_color_saturation: Some(1.0),
            camera_blur_radius_px: Some(2.5),
            camera_temporal_projection_enabled: Some(true),
            camera_temporal_mode: Some("screen-motion-clamp".to_string()),
            camera_temporal_max_pixels_per_frame: Some(18.0),
            camera_temporal_max_angular_degrees_per_frame: Some(1.25),
            camera_temporal_max_linear_meters_per_frame: Some(0.012),
            camera_temporal_catchup_half_life_ms: Some(50.0),
            camera_temporal_max_visual_lag_ms: Some(120.0),
            camera_temporal_stereo_lockstep: Some(true),
            camera_temporal_edge_mode: Some("clamp-soft".to_string()),
            camera_frame_adoption_mode: Some("hold-until-smooth".to_string()),
            camera_frame_adoption_max_jump_px: Some(24.0),
            camera_frame_adoption_max_hold_ms: Some(80.0),
            left_camera_texture_rotation: Some("rotate180".to_string()),
            left_camera_texture_flip_x: Some(true),
            left_camera_texture_flip_y: Some(false),
            left_camera_texture_mirror: Some(false),
            left_camera_texture_transform_source: Some("left-public-live-check".to_string()),
            left_camera_texture_transform_reason: Some(
                "left upright texture validation".to_string(),
            ),
            right_camera_texture_rotation: Some("rotate90".to_string()),
            right_camera_texture_flip_x: Some(false),
            right_camera_texture_flip_y: Some(true),
            right_camera_texture_mirror: Some(false),
            right_camera_texture_transform_source: Some("right-public-live-check".to_string()),
            right_camera_texture_transform_reason: Some(
                "right upright texture validation".to_string(),
            ),
            camera_source_eye_mapping: Some("right-left".to_string()),
            camera_orientation_diagnostic_mode: Some("cycle-source-eye-mapping".to_string()),
            camera_feed_pipeline_mode: Some("raw-feed".to_string()),
            visual_release_accepted: Some(true),
            visual_acceptance_token: Some("manual-visual-accepted".to_string()),
            xr_render_scale: Some(0.75),
            xr_display_refresh_hz: Some(90.0),
            xr_fixed_foveation_level: Some(0),
            xr_color_format_mode: Some("rgba8-unorm".to_string()),
            environment_depth_mode: Some("mesh-overlay".to_string()),
            environment_depth_hand_removal: Some(true),
            hand_particle_mode: Some("meta".to_string()),
            openxr_passthrough_probe: Some("client".to_string()),
            passthrough_style_mode: Some("color-lut".to_string()),
            passthrough_opacity: Some(0.68),
            passthrough_edge_r: Some(0.2),
            passthrough_edge_g: Some(0.7),
            passthrough_edge_b: Some(1.0),
            passthrough_edge_a: Some(0.45),
            passthrough_brightness: Some(8.0),
            passthrough_contrast: Some(1.2),
            passthrough_saturation: Some(0.85),
            passthrough_color_phase: Some(0.25),
            passthrough_color_amplitude: Some(0.9),
            passthrough_lut_resolution: Some(16),
            passthrough_lut_weight: Some(0.75),
            passthrough_lut_flicker_hz: Some(10.0),
            full_field_flicker_hz: Some(40.0),
            projection_layer_visible: Some(false),
            diagnostic_hud_visible: None,
            diagnostic_hud_command: None,
            osc_enabled: Some(true),
            osc_overlay_enabled: Some(false),
            osc_listen_addr: Some("127.0.0.1:9100".to_string()),
            osc_max_packet_bytes: Some(4096),
        });

        assert_eq!(config.camera_tier, CameraCompositeTier::GpuProjected);
        assert_eq!(config.camera_acquisition, "native-ndk");
        assert_eq!(config.cpu_upload_hz, 0);
        assert!(!config.allow_cpu_fallback);
        assert!(config.projection_runtime_resolution_enabled);
        assert_eq!(config.camera_projection_fov_y_degrees, 92.0);
        assert_eq!(config.camera_preview_fov_y_degrees, 60.0);
        assert_eq!(config.camera_preview_offset_y_meters, 0.125);
        assert_eq!(config.camera_projection_scale, 0.75);
        assert_eq!(config.camera_projection_depth_meters, 1.25);
        assert_eq!(config.camera_projection_area_scale_uv, 0.815);
        assert_eq!(config.camera_projection_area_offset_x_uv, 0.125);
        assert_eq!(config.camera_projection_area_offset_y_uv, -0.25);
        assert_eq!(
            config.camera_projection_area_offset_for_eye(0),
            [-0.03125, 0.0625]
        );
        assert_eq!(
            config.camera_projection_area_offset_for_eye(1),
            [0.09375, -0.25]
        );
        assert_eq!(
            config.camera_area_offset_params_push(),
            [-0.03125, 0.0625, 0.09375, -0.25]
        );
        assert_eq!(config.camera_projection_area_radius_x_uv, 0.5);
        assert_eq!(config.camera_projection_area_radius_y_uv, 0.5);
        assert_eq!(config.camera_projection_area_corner_radius_uv, 0.0);
        assert_eq!(config.camera_projection_area_opacity, 0.42);
        assert_eq!(config.camera_projection_border_opacity, 0.85);
        assert_eq!(config.projection_target_offset_x_uv, 0.0625);
        assert_eq!(config.projection_target_offset_y_uv, -0.03125);
        assert_eq!(config.projection_target_scale, 0.75);
        assert_eq!(
            config.projection_target_joystick_controls,
            ProjectionTargetJoystickControls::OffsetScale
        );
        assert_eq!(
            config.camera_projection_alpha_mode,
            CameraProjectionAlphaMode::Green
        );
        assert_eq!(config.camera_projection_alpha_scale, 1.5);
        assert_eq!(config.camera_projection_alpha_bias, -0.1);
        assert_eq!(config.camera_raw_overlay_overscan, 1.06);
        assert_eq!(config.camera_full_view_overlay_overscan, 2.15);
        assert_eq!(config.camera_edge_fade, 0.06);
        assert_eq!(
            config.camera_texture_transform.rotation,
            CameraImageRotation::Rotate180
        );
        assert_eq!(
            config.left_camera_texture_transform.rotation,
            CameraImageRotation::Rotate180
        );
        assert!(config.left_camera_texture_transform.flip_x);
        assert_eq!(
            config.right_camera_texture_transform.rotation,
            CameraImageRotation::Rotate90
        );
        assert!(config.right_camera_texture_transform.flip_y);
        assert_eq!(
            config.source_eye_mapping,
            StereoSourceEyeMapping::DisplayLeftFromRightSource
        );
        assert_eq!(
            config.camera_projection_mode,
            CameraProjectionMode::QuadSurface
        );
        assert_eq!(config.camera_pipeline_preset, CameraPipelinePreset::Manual);
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::SolidRed
        );
        assert_eq!(config.camera_processing_layer, CameraProcessingLayer::Blur);
        assert_eq!(config.camera_peripheral_stretch.inner_blend_uv, 0.04);
        assert_eq!(config.camera_peripheral_stretch.blend_curve, 1.6);
        assert_eq!(
            config.camera_peripheral_stretch.blend_mode,
            CameraPeripheralStretchBlendMode::TargetInnerBand
        );
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::SeparateImageSampler
        );
        assert_eq!(
            config.camera_import_image_layout_mode,
            CameraImportImageLayoutMode::GeneralNoTransition
        );
        assert_eq!(config.camera_import_cache_limit, 2);
        assert_eq!(
            config.camera_color_matrix,
            [[0.9, 0.1, 0.0], [0.0, 1.0, 0.0], [0.0, 0.2, 0.8]]
        );
        assert_eq!(config.camera_color_offset, [-0.01, 0.02, 0.03]);
        assert_eq!(config.camera_color_contrast, 1.1);
        assert_eq!(config.camera_color_brightness, 0.04);
        assert_eq!(config.camera_color_saturation, 1.0);
        assert_eq!(config.camera_blur_radius_px, 2.5);
        assert!(config.camera_temporal_projection_enabled);
        assert_eq!(
            config.camera_temporal_mode,
            TemporalProjectionMode::ScreenMotionClamp
        );
        assert_eq!(config.camera_temporal_max_pixels_per_frame, 18.0);
        assert_eq!(config.camera_temporal_max_angular_degrees_per_frame, 1.25);
        assert_eq!(config.camera_temporal_max_linear_meters_per_frame, 0.012);
        assert_eq!(config.camera_temporal_catchup_half_life_ms, 50.0);
        assert_eq!(config.camera_temporal_max_visual_lag_ms, 120.0);
        assert!(config.camera_temporal_stereo_lockstep);
        assert_eq!(
            config.camera_temporal_edge_mode,
            TemporalProjectionEdgeMode::ClampSoft
        );
        assert_eq!(
            config.camera_frame_adoption_mode,
            CameraFrameAdoptionMode::HoldUntilSmooth
        );
        assert_eq!(config.camera_frame_adoption_max_jump_px, 24.0);
        assert_eq!(config.camera_frame_adoption_max_hold_ms, 80.0);
        assert_eq!(
            config.orientation_diagnostic_mode,
            CameraOrientationDiagnosticMode::CycleSourceEyeMapping
        );
        assert!(config.visual_release_accepted);
        assert!(config.camera_texture_transform.is_explicit_visual_check());
        assert_eq!(config.xr_render_scale, 0.75);
        assert_eq!(config.xr_display_refresh_hz, 90.0);
        assert_eq!(config.xr_fixed_foveation_level, 0);
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
        assert_eq!(
            config.environment_depth_mode,
            EnvironmentDepthMode::MeshOverlay
        );
        assert!(config.environment_depth_mode.visualizes());
        assert!(config.environment_depth_mode.mesh_overlay());
        assert!(config.environment_depth_hand_removal);
        assert_eq!(config.hand_particle_mode, HandParticleMode::Meta);
        assert!(config.hand_particle_mode.enabled());
        assert_eq!(
            config.openxr_passthrough_probe,
            OpenXrPassthroughProbeMode::Client
        );
        assert_eq!(
            config.passthrough_style_mode,
            OpenXrPassthroughStyleMode::ColorLut
        );
        assert_eq!(config.passthrough_opacity, 0.68);
        assert_eq!(config.passthrough_edge_color, [0.2, 0.7, 1.0, 0.45]);
        assert_eq!(config.passthrough_brightness, 8.0);
        assert_eq!(config.passthrough_contrast, 1.2);
        assert_eq!(config.passthrough_saturation, 0.85);
        assert_eq!(config.passthrough_color_phase, 0.25);
        assert_eq!(config.passthrough_color_amplitude, 0.9);
        assert_eq!(config.passthrough_lut_resolution, 16);
        assert_eq!(config.passthrough_lut_weight, 0.75);
        assert_eq!(config.passthrough_lut_flicker_hz, 10.0);
        assert_eq!(config.full_field_flicker_hz, 40.0);
        assert!(!config.projection_layer_visible);
        assert!(!config.diagnostic_hud_visible);
        assert!(config.osc_enabled);
        assert!(!config.osc_overlay_enabled);
        assert_eq!(config.osc_listen_addr, "127.0.0.1:9100");
        assert_eq!(config.osc_max_packet_bytes, 4096);
    }

    #[test]
    fn runtime_config_parses_shared_projection_area_runtime_keys() {
        let bridge: JavaRuntimeConfig = serde_json::from_str(
            r#"{
                "projectionDepthMeters": 1.75,
                "projectionAreaScaleUv": 0.625,
                "projectionAreaOffsetXUv": 0.03125,
                "projectionAreaOffsetYUv": -0.0625,
                "projectionAreaLeftOffsetXUv": -0.125,
                "projectionAreaLeftOffsetYUv": 0.25,
                "projectionAreaRightOffsetXUv": 0.1875,
                "projectionAreaRightOffsetYUv": -0.3125,
                "projectionAreaRadiusXUv": 0.45,
                "projectionAreaRadiusYUv": 0.35,
                "projectionAreaCornerRadiusUv": 0.08,
                "projectionAreaOpacity": 0.7,
                "projectionBorderOpacity": 0.6,
                "projectionAlphaMode": "green",
                "projectionAlphaScale": 1.25,
                "projectionAlphaBias": -0.2
            }"#,
        )
        .expect("shared projection runtime keys should parse");
        let config = public_runtime_config(&bridge);

        assert_eq!(config.camera_projection_depth_meters, 1.75);
        assert_eq!(config.camera_projection_area_scale_uv, 0.625);
        assert_eq!(config.camera_projection_area_offset_x_uv, 0.03125);
        assert_eq!(config.camera_projection_area_offset_y_uv, -0.0625);
        assert_eq!(
            config.camera_projection_area_offset_for_eye(0),
            [-0.125, 0.25]
        );
        assert_eq!(
            config.camera_projection_area_offset_for_eye(1),
            [0.1875, -0.3125]
        );
        assert_eq!(config.camera_projection_area_radius_x_uv, 0.45);
        assert_eq!(config.camera_projection_area_radius_y_uv, 0.35);
        assert_eq!(config.camera_projection_area_corner_radius_uv, 0.08);
        assert_eq!(config.camera_projection_area_opacity, 0.7);
        assert_eq!(config.camera_projection_border_opacity, 0.6);
        assert_eq!(
            config.camera_projection_alpha_mode,
            CameraProjectionAlphaMode::Green
        );
        assert_eq!(config.camera_projection_alpha_scale, 1.25);
        assert_eq!(config.camera_projection_alpha_bias, -0.2);
    }

    #[test]
    fn hwb_projection_runtime_resolution_consumes_higher_precedence_projection_layer() {
        let mut config = RuntimeConfig {
            camera_projection_depth_meters: 1.0,
            camera_projection_area_offset_y_uv: 0.0,
            camera_projection_area_left_offset_y_uv: 0.0,
            camera_projection_area_right_offset_y_uv: 0.0,
            camera_projection_border_policy: CameraProjectionBorderPolicy::SolidRed,
            ..Default::default()
        };
        let mut property_config = rxrc::RuntimeConfig::new();
        property_config
            .set(
                rxrc::KEY_PROJECTION_DEPTH_METERS,
                rxrc::RuntimeValue::Float(2.0),
                rxrc::RuntimeConfigSource::AndroidProperty,
            )
            .expect("projection key should be valid");
        property_config
            .set(
                rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
                rxrc::RuntimeValue::Float(0.25),
                rxrc::RuntimeConfigSource::AndroidProperty,
            )
            .expect("projection key should be valid");
        property_config
            .set(
                rxrc::KEY_PROJECTION_BORDER_POLICY,
                rxrc::RuntimeValue::Text("passthrough-underlay".to_string()),
                rxrc::RuntimeConfigSource::AndroidProperty,
            )
            .expect("projection key should be valid");
        let runtime = rxrc::ProjectionRuntimeConfigBuilder::new()
            .with_layer(
                "hwb-launch-effective",
                10,
                public_projection_runtime_config(&config, rxrc::RuntimeConfigSource::CommandLine),
            )
            .expect("manifest owner should be valid")
            .with_layer("hwb-android-properties", 20, property_config)
            .expect("manifest owner should be valid")
            .resolve();

        apply_hwb_projection_runtime_resolution(&mut config, &runtime.resolution);

        assert_eq!(config.camera_projection_depth_meters, 2.0);
        assert_eq!(config.camera_projection_area_offset_y_uv, 0.25);
        assert_eq!(config.camera_projection_area_left_offset_y_uv, 0.25);
        assert_eq!(config.camera_projection_area_right_offset_y_uv, 0.25);
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::PassthroughUnderlay
        );
    }

    #[test]
    fn hand_particle_mode_parses_openxr_hand_mesh_aliases() {
        assert_eq!(
            HandParticleMode::parse("meta-hand-mesh"),
            Some(HandParticleMode::Meta)
        );
        assert_eq!(
            HandParticleMode::parse("openxr"),
            Some(HandParticleMode::Meta)
        );
        assert!(HandParticleMode::Meta.uses_openxr_hand_mesh());
        assert_eq!(HandParticleMode::Meta.stable_id(), "meta");
    }

    #[test]
    fn runtime_config_supports_generic_diagnostic_hud_toggle_path() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            diagnostic_hud_visible: Some(true),
            osc_overlay_enabled: Some(false),
            ..Default::default()
        });
        assert!(config.diagnostic_hud_visible);
        assert!(!config.osc_overlay_enabled);

        let osc_legacy_config = public_runtime_config(&JavaRuntimeConfig {
            osc_enabled: Some(true),
            ..Default::default()
        });
        assert!(osc_legacy_config.diagnostic_hud_visible);

        assert_eq!(
            parse_diagnostic_hud_command("toggle"),
            Some(DiagnosticHudCommand::Toggle)
        );
        assert_eq!(
            parse_diagnostic_hud_command("page:2"),
            Some(DiagnosticHudCommand::SetPage(2))
        );
    }

    #[test]
    fn runtime_config_camera_pipeline_preset_overrides_module_axes() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("separate-decode-unorm".to_string()),
            camera_feed_pipeline_mode: Some("projected-feed".to_string()),
            camera_color_mode: Some("external-rgb".to_string()),
            camera_sampler_binding_mode: Some("combined-immutable-sampler".to_string()),
            camera_import_image_layout_mode: Some("shader-read-transition".to_string()),
            camera_color_matrix: Some("0.9;0.1;0;0;1;0;0;0.2;0.8".to_string()),
            camera_color_offset: Some("-0.01;0.02;0.03".to_string()),
            camera_color_contrast: Some(1.3),
            camera_color_brightness: Some(0.2),
            camera_color_saturation: Some(1.8),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::SeparateDecodeUnorm
        );
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(
            config.camera_color_mode,
            CameraColorMode::ExternalCrYCbBt601Narrow
        );
        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::SeparateImageSampler
        );
        assert_eq!(
            config.camera_import_image_layout_mode,
            CameraImportImageLayoutMode::GeneralNoTransition
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::BorderComposite
        );
        assert_eq!(config.camera_color_matrix, super::identity_color_matrix());
        assert_eq!(config.camera_color_offset, [0.0, 0.0, 0.0]);
        assert_eq!(config.camera_color_contrast, 1.0);
        assert_eq!(config.camera_color_brightness, 0.0);
        assert_eq!(config.camera_color_saturation, 1.0);
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
    }

    #[test]
    fn runtime_config_raw_projection_preset_selects_raw_unorm_path() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("raw-projection-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            camera_feed_pipeline_mode: Some("projected-feed".to_string()),
            camera_color_mode: Some("debug-red-only".to_string()),
            camera_sampler_binding_mode: Some("separate-image-sampler".to_string()),
            camera_import_image_layout_mode: Some("general-no-transition".to_string()),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::RawProjectionUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::CombinedImmutableSampler
        );
        assert_eq!(
            config.camera_import_image_layout_mode,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition
        );
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
    }

    #[test]
    fn runtime_config_projection_border_policy_solid_red_uses_raw_path() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("raw-projection-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            projection_border_policy: Some("solid-red".to_string()),
            camera_feed_pipeline_mode: Some("projected-feed".to_string()),
            camera_color_mode: Some("debug-red-only".to_string()),
            camera_sampler_binding_mode: Some("separate-image-sampler".to_string()),
            camera_import_image_layout_mode: Some("general-no-transition".to_string()),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::RawProjectionUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::SolidRed
        );
        assert!(config.camera_projection_border_policy_active());
        assert!(config.camera_projection_border_policy_requires_full_pipeline());
        assert_eq!(
            config.camera_projection_border_policy_shader_bit(),
            super::camera_color_pipeline::CAMERA_SHADER_FLAG_PROJECTION_BORDER_SOLID_RED
        );
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::CombinedImmutableSampler
        );
        assert_eq!(
            config.camera_import_image_layout_mode,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition
        );
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
    }

    #[test]
    fn runtime_config_legacy_solid_red_projection_aliases_resolve_to_current_policy() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("raw-projection-solid-red-unorm".to_string()),
            camera_projection_effect_mode: Some("raw-projection-solid-red".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::RawProjectionUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::SolidRed
        );
        assert!(config.camera_projection_border_policy_active());
        assert!(config.camera_projection_border_policy_requires_full_pipeline());
    }

    #[test]
    fn runtime_config_legacy_underlay_projection_aliases_resolve_to_current_policy() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("raw-projection-underlay-unorm".to_string()),
            camera_projection_effect_mode: Some("raw-projection-underlay".to_string()),
            openxr_passthrough_probe: Some("off".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::RawProjectionUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::PassthroughUnderlay
        );
        assert_eq!(
            config.openxr_passthrough_probe,
            OpenXrPassthroughProbeMode::Underlay
        );
    }

    #[test]
    fn runtime_config_processing_layer_blur_solid_red_policy_selects_blur_path() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("raw-projection-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            projection_border_policy: Some("solid-red".to_string()),
            processing_layer: Some("blur".to_string()),
            camera_blur_radius_px: Some(3.5),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::RawProjectionUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::SolidRed
        );
        assert_eq!(config.camera_processing_layer, CameraProcessingLayer::Blur);
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
        assert_eq!(
            config.openxr_passthrough_probe,
            OpenXrPassthroughProbeMode::Off
        );
        assert_eq!(config.camera_blur_radius_px, 3.5);
        assert_eq!(config.camera_effect_params_push()[3], 5.0);
    }

    #[test]
    fn runtime_config_processing_layer_blur_underlay_policy_selects_passthrough_underlay() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("raw-projection-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            projection_border_policy: Some("passthrough-underlay".to_string()),
            processing_layer: Some("blur".to_string()),
            openxr_passthrough_probe: Some("off".to_string()),
            camera_blur_radius_px: Some(32.0),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::RawProjectionUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::PassthroughUnderlay
        );
        assert_eq!(config.camera_processing_layer, CameraProcessingLayer::Blur);
        assert_eq!(
            config.openxr_passthrough_probe,
            OpenXrPassthroughProbeMode::Underlay
        );
        assert_eq!(config.camera_blur_radius_px, 16.0);
        assert_eq!(config.camera_effect_params_push()[3], 5.0);
    }

    #[test]
    fn runtime_config_processing_layer_peripheral_stretch_selects_effect_path() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("raw-projection-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            projection_border_policy: Some("solid-red".to_string()),
            processing_layer: Some("peripheral-stretch".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::SolidRed
        );
        assert_eq!(
            config.camera_processing_layer,
            CameraProcessingLayer::PeripheralStretch
        );
        assert_eq!(config.camera_effect_params_push()[3], 6.0);
        assert_eq!(
            config.camera_peripheral_stretch_blend_params_push(),
            [0.0, 1.5, 1.0, 0.0]
        );
    }

    #[test]
    fn runtime_config_projection_area_diagnostic_preset_keeps_raw_geometry() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("projection-area-diagnostic-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            camera_feed_pipeline_mode: Some("projected-feed".to_string()),
            camera_color_mode: Some("debug-red-only".to_string()),
            camera_sampler_binding_mode: Some("separate-image-sampler".to_string()),
            camera_import_image_layout_mode: Some("general-no-transition".to_string()),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::ProjectionAreaDiagnosticUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::ProjectionAreaDiagnostic
        );
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::CombinedImmutableSampler
        );
        assert_eq!(
            config.camera_import_image_layout_mode,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition
        );
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
    }

    #[test]
    fn runtime_config_display_eye_uv_fiducial_preset_selects_mapping_probe() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("display-eye-uv-fiducial-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            camera_feed_pipeline_mode: Some("projected-feed".to_string()),
            camera_color_mode: Some("debug-red-only".to_string()),
            camera_sampler_binding_mode: Some("separate-image-sampler".to_string()),
            camera_import_image_layout_mode: Some("general-no-transition".to_string()),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::DisplayEyeUvFiducialUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::DisplayEyeUvFiducial
        );
        assert_eq!(config.camera_effect_params_push()[3], 1.0);
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::CombinedImmutableSampler
        );
        assert_eq!(
            config.camera_import_image_layout_mode,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition
        );
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
    }

    #[test]
    fn runtime_config_projection_content_uv_fiducial_preset_selects_post_offset_probe() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("projection-content-uv-fiducial-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            camera_feed_pipeline_mode: Some("projected-feed".to_string()),
            camera_color_mode: Some("debug-red-only".to_string()),
            camera_sampler_binding_mode: Some("separate-image-sampler".to_string()),
            camera_import_image_layout_mode: Some("general-no-transition".to_string()),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::ProjectionContentUvFiducialUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::ProjectionContentUvFiducial
        );
        assert_eq!(config.camera_effect_params_push()[3], 2.0);
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::CombinedImmutableSampler
        );
        assert_eq!(
            config.camera_import_image_layout_mode,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition
        );
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
    }

    #[test]
    fn runtime_config_source_sampling_witness_preset_selects_source_overlay_probe() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("source-sampling-witness-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            camera_feed_pipeline_mode: Some("projected-feed".to_string()),
            camera_color_mode: Some("debug-red-only".to_string()),
            camera_sampler_binding_mode: Some("separate-image-sampler".to_string()),
            camera_import_image_layout_mode: Some("general-no-transition".to_string()),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::SourceSamplingWitnessUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::SourceSamplingWitness
        );
        assert_eq!(config.camera_effect_params_push()[3], 3.0);
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.camera_sampler_binding_mode,
            CameraSamplerBindingMode::CombinedImmutableSampler
        );
        assert_eq!(
            config.camera_import_image_layout_mode,
            CameraImportImageLayoutMode::ShaderReadOnlyTransition
        );
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
    }

    #[test]
    fn runtime_config_projection_border_policy_underlay_selects_passthrough_underlay() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_pipeline_preset: Some("raw-projection-unorm".to_string()),
            camera_projection_effect_mode: Some("border-composite".to_string()),
            projection_border_policy: Some("passthrough-underlay".to_string()),
            openxr_passthrough_probe: Some("off".to_string()),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(
            config.camera_pipeline_preset,
            CameraPipelinePreset::RawProjectionUnorm
        );
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::RawProjection
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::PassthroughUnderlay
        );
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::RawFeed
        );
        assert_eq!(config.camera_color_mode, CameraColorMode::ExternalRgb);
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Unorm
        );
        assert_eq!(
            config.openxr_passthrough_probe,
            OpenXrPassthroughProbeMode::Underlay
        );
    }

    #[test]
    fn runtime_config_full_frame_stimulus_surface_mapping_effect_is_explicit() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_projection_effect_mode: Some("full-frame-stimulus-surface-mapping".to_string()),
            projection_border_policy: Some("passthrough-underlay".to_string()),
            openxr_passthrough_probe: Some("off".to_string()),
            xr_color_format_mode: Some("rgba8-srgb".to_string()),
            ..Default::default()
        });

        assert_eq!(config.camera_pipeline_preset, CameraPipelinePreset::Manual);
        assert_eq!(
            config.camera_projection_effect_mode,
            CameraProjectionEffectMode::FullFrameStimulusSurfaceMapping
        );
        assert_eq!(
            config.camera_projection_border_policy,
            CameraProjectionBorderPolicy::PassthroughUnderlay
        );
        assert_eq!(config.camera_effect_params_push()[3], 4.0);
        assert_eq!(
            config.camera_feed_pipeline_mode,
            CameraFeedPipelineMode::ProjectedFeed
        );
        assert_eq!(
            config.xr_color_format_mode,
            OpenXrColorFormatMode::Rgba8Srgb
        );
        assert_eq!(
            config.openxr_passthrough_probe,
            OpenXrPassthroughProbeMode::Underlay
        );
    }

    #[test]
    fn environment_depth_scene_particle_map_parses_aliases() {
        assert_eq!(
            EnvironmentDepthMode::parse("scene-particle-map"),
            Some(EnvironmentDepthMode::SceneParticleMap)
        );
        assert_eq!(
            EnvironmentDepthMode::parse("spatial-particles"),
            Some(EnvironmentDepthMode::SceneParticleMap)
        );
        assert!(EnvironmentDepthMode::SceneParticleMap.visualizes());
        assert!(EnvironmentDepthMode::SceneParticleMap.particle_overlay());
        assert!(EnvironmentDepthMode::SceneParticleMap.scene_particle_map());
        assert!(!EnvironmentDepthMode::SceneParticleMap.mesh_overlay());
    }

    #[test]
    fn visual_release_acceptance_requires_manual_token() {
        let config = public_runtime_config(&JavaRuntimeConfig {
            camera_tier: Some("gpu-projected".to_string()),
            visual_release_accepted: Some(true),
            visual_acceptance_token: Some("wrong-token".to_string()),
            ..Default::default()
        });

        assert!(!config.visual_release_accepted);
    }
}
