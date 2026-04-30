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
    Vec2, Vec3, VisualFeedbackBorder, VisualFeedbackBorderLayout,
};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    sync::{Mutex, OnceLock},
};

#[cfg(target_os = "android")]
mod openxr_vulkan;

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
pub(crate) enum StereoSourceEyeMapping {
    #[default]
    DisplayLeftFromLeftSource,
    DisplayLeftFromRightSource,
}

impl StereoSourceEyeMapping {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "left-right"
            | "display-left-from-left"
            | "displayLeftFromLeft"
            | "natural"
            | "camera50-left" => Some(Self::DisplayLeftFromLeftSource),
            "right-left"
            | "display-left-from-right"
            | "displayLeftFromRight"
            | "swapped"
            | "swap"
            | "camera51-left" => Some(Self::DisplayLeftFromRightSource),
            _ => None,
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::DisplayLeftFromLeftSource => "display-left-from-left-source",
            Self::DisplayLeftFromRightSource => "display-left-from-right-source",
        }
    }

    pub(crate) const fn shader_swap_bit(self) -> u32 {
        match self {
            Self::DisplayLeftFromLeftSource => 0,
            Self::DisplayLeftFromRightSource => 1,
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
        left | (right << 5) | (self.source_eye_mapping.shader_swap_bit() << 10)
    }

    pub(crate) fn left_label(&self) -> String {
        self.left_texture_transform.label()
    }

    pub(crate) fn right_label(&self) -> String {
        self.right_texture_transform.label()
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct RuntimeConfig {
    pub(crate) camera_tier: CameraCompositeTier,
    pub(crate) camera_enabled: bool,
    pub(crate) media_projection_enabled: bool,
    pub(crate) allow_cpu_fallback: bool,
    pub(crate) cpu_upload_hz: u32,
    pub(crate) stereo_layout: StereoMediaLayout,
    pub(crate) camera_projection_fov_y_degrees: f32,
    pub(crate) camera_preview_fov_y_degrees: f32,
    pub(crate) camera_projection_scale: f32,
    pub(crate) camera_raw_overlay_overscan: f32,
    pub(crate) camera_full_view_overlay_overscan: f32,
    pub(crate) camera_edge_fade: f32,
    pub(crate) camera_texture_transform: CameraTextureTransform,
    pub(crate) left_camera_texture_transform: CameraTextureTransform,
    pub(crate) right_camera_texture_transform: CameraTextureTransform,
    pub(crate) source_eye_mapping: StereoSourceEyeMapping,
    pub(crate) orientation_diagnostic_mode: CameraOrientationDiagnosticMode,
    pub(crate) visual_release_accepted: bool,
    pub(crate) xr_render_scale: f32,
    pub(crate) xr_fixed_foveation_level: u8,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            camera_tier: CameraCompositeTier::CpuDiagnosticFlatCopy,
            camera_enabled: true,
            media_projection_enabled: false,
            allow_cpu_fallback: true,
            cpu_upload_hz: 4,
            stereo_layout: StereoMediaLayout::Mono,
            camera_projection_fov_y_degrees: 92.0,
            camera_preview_fov_y_degrees: 60.0,
            camera_projection_scale: 0.75,
            camera_raw_overlay_overscan: 1.06,
            camera_full_view_overlay_overscan: 2.10,
            camera_edge_fade: 0.12,
            camera_texture_transform: CameraTextureTransform::default(),
            left_camera_texture_transform: CameraTextureTransform::default(),
            right_camera_texture_transform: CameraTextureTransform::default(),
            source_eye_mapping: StereoSourceEyeMapping::default(),
            orientation_diagnostic_mode: CameraOrientationDiagnosticMode::Off,
            visual_release_accepted: false,
            xr_render_scale: 0.75,
            xr_fixed_foveation_level: 0,
        }
    }
}

impl RuntimeConfig {
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

fn headset_camera_state() -> &'static Mutex<HeadsetCameraState> {
    HEADSET_CAMERA_STATE.get_or_init(|| Mutex::new(HeadsetCameraState::default()))
}

fn runtime_config_state() -> &'static Mutex<RuntimeConfig> {
    RUNTIME_CONFIG.get_or_init(|| Mutex::new(RuntimeConfig::default()))
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

    #[cfg(target_os = "android")]
    log_info(format!(
        "Rusty XR camera path config requestedTier={} cameraEnabled={} mediaProjection={} allowCpuFallback={} cpuUploadHz={} stereoLayout={:?} projectionFovY={} previewFovY={} projectionScale={} rawOverscan={} fullViewOverscan={} edgeFade={} cameraTextureTransform={} leftCameraTextureTransform={} rightCameraTextureTransform={} sourceEyeMapping={} orientationDiagnosticMode={} cameraTextureTransformSource={} cameraTextureTransformReason={} orientationCheck={} visualReleaseAccepted={} xrRenderScale={} fixedFoveationLevel={}",
        config.camera_tier.stable_id(),
        config.camera_enabled,
        config.media_projection_enabled,
        config.allow_cpu_fallback,
        config.cpu_upload_hz,
        config.stereo_layout,
        config.camera_projection_fov_y_degrees,
        config.camera_preview_fov_y_degrees,
        config.camera_projection_scale,
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
        config.xr_fixed_foveation_level
    ));
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
        if index == 0 || index % 30 == 0 {
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
        if index == 0 || index % 120 == 0 {
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

        if index == 0 || index % 120 == 0 {
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
    camera_enabled: Option<bool>,
    media_projection_enabled: Option<bool>,
    allow_cpu_fallback: Option<bool>,
    cpu_upload_hz: Option<u32>,
    stereo_layout: Option<String>,
    camera_projection_fov_y_degrees: Option<f32>,
    camera_preview_fov_y_degrees: Option<f32>,
    camera_projection_scale: Option<f32>,
    camera_raw_overlay_overscan: Option<f32>,
    camera_full_view_overlay_overscan: Option<f32>,
    camera_edge_fade: Option<f32>,
    camera_texture_rotation: Option<String>,
    camera_texture_flip_x: Option<bool>,
    camera_texture_flip_y: Option<bool>,
    camera_texture_mirror: Option<bool>,
    camera_texture_transform_source: Option<String>,
    camera_texture_transform_reason: Option<String>,
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
    visual_release_accepted: Option<bool>,
    visual_acceptance_token: Option<String>,
    xr_render_scale: Option<f32>,
    xr_fixed_foveation_level: Option<u8>,
}

fn public_runtime_config(bridge: &JavaRuntimeConfig) -> RuntimeConfig {
    let camera_tier = bridge
        .camera_tier
        .as_deref()
        .and_then(CameraCompositeTier::parse)
        .unwrap_or(CameraCompositeTier::CpuDiagnosticFlatCopy);
    let defaults = RuntimeConfig::default();
    RuntimeConfig {
        camera_tier,
        camera_enabled: bridge.camera_enabled.unwrap_or(true),
        media_projection_enabled: bridge.media_projection_enabled.unwrap_or(false),
        allow_cpu_fallback: bridge.allow_cpu_fallback.unwrap_or(true),
        cpu_upload_hz: bridge.cpu_upload_hz.unwrap_or(4),
        stereo_layout: bridge
            .stereo_layout
            .as_deref()
            .and_then(parse_stereo_layout)
            .unwrap_or(StereoMediaLayout::Mono),
        camera_projection_fov_y_degrees: finite_positive_or(
            bridge.camera_projection_fov_y_degrees,
            defaults.camera_projection_fov_y_degrees,
        ),
        camera_preview_fov_y_degrees: finite_positive_or(
            bridge.camera_preview_fov_y_degrees,
            defaults.camera_preview_fov_y_degrees,
        ),
        camera_projection_scale: finite_positive_or(
            bridge.camera_projection_scale,
            defaults.camera_projection_scale,
        ),
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
        xr_fixed_foveation_level: bridge
            .xr_fixed_foveation_level
            .unwrap_or(defaults.xr_fixed_foveation_level),
    }
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaCameraFrameMetadata {
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
        "Camera2 PRIVATE AHardwareBuffer",
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
    let mut descriptor = CameraGpuBufferDescriptor::new(
        "Camera2 PRIVATE AHardwareBuffer",
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
    use super::{
        contract_json, public_camera_metadata, public_runtime_config,
        CameraOrientationDiagnosticMode, JavaCameraExtrinsics, JavaCameraFrameMetadata,
        JavaCameraIntrinsics, JavaPixelDomain, JavaPixelDomainKind, JavaRuntimeConfig,
        StereoSourceEyeMapping,
    };
    use rusty_xr_contracts::{
        CameraCompositeTier, CameraImageRotation, CameraPixelDomainKind, ImageSize,
    };

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
    fn java_camera_metadata_marks_mono_missing_pose_fallback() {
        let bridge = JavaCameraFrameMetadata {
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
            extrinsics: None,
            missing_intrinsics: Some(false),
            missing_pose: Some(true),
            mono_fallback: Some(true),
            fallback_reason: Some("missing camera pose; diagnostic flat camera copy".to_string()),
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
            camera_enabled: Some(true),
            media_projection_enabled: Some(false),
            allow_cpu_fallback: Some(false),
            cpu_upload_hz: Some(0),
            stereo_layout: Some("separate".to_string()),
            camera_projection_fov_y_degrees: Some(92.0),
            camera_preview_fov_y_degrees: Some(60.0),
            camera_projection_scale: Some(0.75),
            camera_raw_overlay_overscan: Some(1.06),
            camera_full_view_overlay_overscan: Some(2.15),
            camera_edge_fade: Some(0.06),
            camera_texture_rotation: Some("rotate180".to_string()),
            camera_texture_flip_x: Some(false),
            camera_texture_flip_y: Some(false),
            camera_texture_mirror: Some(false),
            camera_texture_transform_source: Some("public-live-check".to_string()),
            camera_texture_transform_reason: Some("upright texture validation".to_string()),
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
            visual_release_accepted: Some(true),
            visual_acceptance_token: Some("manual-visual-accepted".to_string()),
            xr_render_scale: Some(0.75),
            xr_fixed_foveation_level: Some(0),
        });

        assert_eq!(config.camera_tier, CameraCompositeTier::GpuProjected);
        assert_eq!(config.cpu_upload_hz, 0);
        assert!(!config.allow_cpu_fallback);
        assert_eq!(config.camera_projection_fov_y_degrees, 92.0);
        assert_eq!(config.camera_preview_fov_y_degrees, 60.0);
        assert_eq!(config.camera_projection_scale, 0.75);
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
            config.orientation_diagnostic_mode,
            CameraOrientationDiagnosticMode::CycleSourceEyeMapping
        );
        assert!(config.visual_release_accepted);
        assert!(config.camera_texture_transform.is_explicit_visual_check());
        assert_eq!(config.xr_render_scale, 0.75);
        assert_eq!(config.xr_fixed_foveation_level, 0);
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
