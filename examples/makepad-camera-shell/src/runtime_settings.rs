#[cfg(target_os = "android")]
use rusty_xr_runtime_config::{AndroidPropertyPrefix, RuntimeKey};
use rusty_xr_runtime_config::{RuntimeConfig, RuntimeConfigSource, RuntimeValue};

pub(crate) const DEFAULT_PROFILE: &str = "makepad-stereo-projection-pair-probe";
pub(crate) const DEFAULT_TRANSPORT: &str = "makepad-direct-hwb-target-inner-band-stretch";
pub(crate) const DEFAULT_CAMERA_TIER: &str = "native-camera2-makepad-stereo-vulkan-import-probe";
pub(crate) const DEFAULT_CAMERA_PROJECTION_MODE: &str = "display-screen-homography";
pub(crate) const DEFAULT_COMPARISON_BASELINE: &str = "custom-apk-camera-stereo-gpu-composite";
pub(crate) const DEFAULT_SYNTHETIC_SCENE: &str = "camera-panel-target-inner-band-stretch-border";
pub(crate) const DEFAULT_ACQUISITION_PROFILE: &str =
    "bounded-camera2-private-plus-makepad-paired-import-probe";
pub(crate) const DEFAULT_PROJECTION_SCALE: f64 = 1.0;
pub(crate) const DEFAULT_PROJECTION_DEPTH_METERS: f64 = 1.434085;
pub(crate) const DEFAULT_XR_RENDER_SCALE: f64 = 1.0;
pub(crate) const DEFAULT_BROKER_H264_ENABLED: bool = false;
pub(crate) const DEFAULT_BROKER_H264_HOST: &str = "127.0.0.1";
pub(crate) const DEFAULT_BROKER_H264_BROKER_PORT: u16 = 8765;
pub(crate) const DEFAULT_BROKER_H264_STREAM_PORT: u16 = 8879;
pub(crate) const DEFAULT_BROKER_H264_RIGHT_STREAM_PORT: u16 = 8880;
pub(crate) const DEFAULT_BROKER_H264_SOURCE_MODE: &str = "broker-synthetic";
pub(crate) const DEFAULT_BROKER_H264_DECODE_OUTPUT_MODE: &str = "cpu-yuv";
pub(crate) const DEFAULT_BROKER_H264_SYNTHETIC_PATTERN: &str = "diagnostic-grid";
pub(crate) const DEFAULT_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE: &str =
    "head-anchored-virtual-camera";
pub(crate) const DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE: &str = "camera-projection";
pub(crate) const DEFAULT_CAMERA_SOURCE_SAMPLING_MODE: &str = "target-local-raster";
pub(crate) const DEFAULT_BROKER_H264_LEFT_CAMERA_ID: &str = "";
pub(crate) const DEFAULT_BROKER_H264_RIGHT_CAMERA_ID: &str = "";
pub(crate) const DEFAULT_BROKER_H264_WIDTH: u32 = 1280;
pub(crate) const DEFAULT_BROKER_H264_HEIGHT: u32 = 1280;
pub(crate) const DEFAULT_BROKER_H264_CAPTURE_MS: u32 = 45_000;
pub(crate) const DEFAULT_BROKER_H264_MAX_PACKETS: u32 = 0;
pub(crate) const DEFAULT_BROKER_H264_BITRATE_BPS: u32 = 6_000_000;
pub(crate) const DEFAULT_BROKER_H264_FRAME_RATE_HZ: u32 = 30;
pub(crate) const DEFAULT_BROKER_H264_COMMAND_TIMEOUT_MS: u32 = 10_000;
pub(crate) const DEFAULT_BROKER_H264_STREAM_TIMEOUT_MS: u32 = 30_000;
pub(crate) const DEFAULT_BROKER_H264_DECODE_TIMEOUT_MS: u32 = 20_000;
pub(crate) const DEFAULT_BROKER_H264_LIVE_STREAM: bool = true;
pub(crate) const DEFAULT_BROKER_H264_STEREO_PAIR_ID: &str =
    "makepad-broker-h264-stereo-camera";
pub(crate) const DEFAULT_BROKER_H264_STEREO_PAIR_MAX_DELTA_NS: u32 = 25_000_000;
pub(crate) const DEFAULT_MAKEPAD_DIRECT_CAMERA_HARDWARE_BUFFER_EXTERNAL: bool = true;
pub(crate) const DEFAULT_MANIFOLD_POSE_PUBLISH_ENABLED: bool = false;
pub(crate) const DEFAULT_MANIFOLD_POSE_STREAM: &str = "stream.motion.object_pose";
pub(crate) const DEFAULT_MANIFOLD_POSE_SOURCE: &str = "provider.makepad_xr.controller_pose";
pub(crate) const DEFAULT_MANIFOLD_POSE_CONTROLLER: &str = "right";
pub(crate) const DEFAULT_MANIFOLD_POSE_KIND: &str = "grip";
pub(crate) const DEFAULT_MANIFOLD_BROKER_HOST: &str = "127.0.0.1";
pub(crate) const DEFAULT_MANIFOLD_BROKER_PORT: u16 = 8765;
pub(crate) const DEFAULT_MANIFOLD_POSE_SAMPLE_HZ: f32 = 20.0;
pub(crate) const DEFAULT_MANIFOLD_POSE_CONNECT_TIMEOUT_MS: u32 = 250;
pub(crate) const DEFAULT_MANIFOLD_BREATH_FEEDBACK_ENABLED: bool = false;
pub(crate) const DEFAULT_MANIFOLD_BREATH_FEEDBACK_STREAM: &str = "stream.breath.feedback_state";
pub(crate) const DEFAULT_MANIFOLD_BREATH_FEEDBACK_RECEIVER: &str =
    "app.makepad_camera_shell.breath_feedback";
pub(crate) const DEFAULT_MANIFOLD_BREATH_FEEDBACK_CONNECT_TIMEOUT_MS: u32 = 250;
pub(crate) const DEFAULT_MAKEPAD_PROJECTION_TARGET_JOYSTICK_CONTROLS: &str = "offset-scale";
pub(crate) const SUPPRESS_LIVE_CAMERA_SAMPLING: bool = false;
pub(crate) const FORCE_FULL_SURFACE_LIVE_CAMERA_UV: bool = false;
pub(crate) const FORCE_IN_SURFACE_CAMERA_WINDOW: bool = true;
pub(crate) const TARGET_HORIZONTAL_ALIGNMENT_STRENGTH: f32 = 0.0;
pub(crate) const TARGET_MANUAL_HORIZONTAL_OFFSET_LEFT_UV: f32 = 0.0;
pub(crate) const TARGET_MANUAL_HORIZONTAL_OFFSET_RIGHT_UV: f32 = 0.0;
pub(crate) const TARGET_MANUAL_VERTICAL_OFFSET_UV: f32 = 0.0;
pub(crate) const TARGET_FULL_VIEW_CONTENT_UV_SCALE: f32 = 1.60;
pub(crate) const TARGET_PROJECTION_BORDER_OPACITY: f32 = 1.0;
pub(crate) const TARGET_PROJECTION_AREA_DIAGNOSTIC: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_AREA_OFFSET_LEFT_UV: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_AREA_SCALE_X: f32 = 1.0;
pub(crate) const TARGET_PROJECTION_AREA_SCALE_Y: f32 = 1.0;
pub(crate) const TARGET_PROJECTION_TARGET_OFFSET_X_UV: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_TARGET_OFFSET_Y_UV: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_TARGET_SCALE: f32 = 1.0;
pub(crate) const DEFAULT_CAMERA_TARGET_SCREEN_UV_RECT: &str = "";
pub(crate) const DEFAULT_CAMERA_LEFT_TARGET_SCREEN_UV_RECT: &str = "0.171875;0.21875;0.75;0.65625";
pub(crate) const DEFAULT_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT: &str =
    "0.078125;0.21875;0.75;0.671875";
pub(crate) const TARGET_PROJECTION_AREA_RADIUS_X_UV: f32 = 0.5;
pub(crate) const TARGET_PROJECTION_AREA_RADIUS_Y_UV: f32 = 0.5;
pub(crate) const TARGET_PROJECTION_AREA_CORNER_RADIUS_UV: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_AREA_KEYSTONE_X: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_AREA_BOW_X: f32 = 0.0;
pub(crate) const TARGET_PROJECTION_AREA_OPACITY: f32 = 1.0;
pub(crate) const SOURCE_VALID_FOOTPRINT_GRID: usize = 64;
pub(crate) const TARGET_DISPLAY_EYE_OFFSET_METERS: f32 = 0.032;
pub(crate) const TARGET_DISPLAY_FOV_Y_DEGREES: f32 = 92.0;
pub(crate) const TARGET_DISPLAY_ASPECT: f32 = 1.0;
pub(crate) const TARGET_PROJECTION_DEPTH_METERS: f32 = DEFAULT_PROJECTION_DEPTH_METERS as f32;
pub(crate) const TARGET_PROJECTION_PREVIEW_FOV_Y_DEGREES: f32 = 69.763084;
pub(crate) const TARGET_PROJECTION_PREVIEW_OFFSET_Y_METERS: f32 = -0.168832;
pub(crate) const TARGET_PROJECTION_RAW_OVERSCAN: f32 = 1.0;
pub(crate) const FRAME_RASTER_TOP_LEFT_Y_DOWN: &str = "top-left-origin-y-down";
pub(crate) const FRAME_RASTER_BOTTOM_LEFT_Y_UP: &str = "bottom-left-origin-y-up";
pub(crate) const IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY: [[f32; 3]; 3] =
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
pub(crate) const MAKEPAD_BRANCH: &str = "rusty-xr/android-libstd-packaging";
pub(crate) const MAKEPAD_REV: &str = "22aba400f";
pub(crate) const DEFAULT_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING: &str = "display-left-from-left-source";
pub(crate) const PAIRED_IMPORT_DELAY_SECONDS: f64 = 6.0;
pub(crate) const PAIRED_IMPORT_RETRY_SECONDS: f64 = 1.0;
pub(crate) const PAIRED_IMPORT_MAX_WAITS: usize = 10;
pub(crate) const CADENCE_SAMPLE_SECONDS: f64 = 5.0;
pub(crate) const PROJECTION_TARGET_JOYSTICK_DEADZONE: f32 = 0.18;
pub(crate) const PROJECTION_TARGET_OFFSET_RATE_UV_PER_SECOND: f32 = 0.28;
pub(crate) const PROJECTION_TARGET_SCALE_RATE_PER_SECOND: f32 = 0.45;
pub(crate) const PROJECTION_AREA_MIN_SCALE: f32 = 0.01;
pub(crate) const PROJECTION_AREA_MAX_SCALE: f32 = 10.00;
pub(crate) const PROJECTION_TARGET_MIN_SCALE: f32 = 0.05;
pub(crate) const PROJECTION_TARGET_MAX_SCALE: f32 = 1.50;
// S25 showed this diagnostic can reintroduce app-process GPU page faults on Quest.
pub(crate) const NATIVE_VIDEO_WIDGET_SURFACE_DIAGNOSTIC: bool = false;
pub(crate) const NATIVE_VIDEO_WIDGET_RETRY_SECONDS: f64 = 0.5;
pub(crate) const NATIVE_VIDEO_WIDGET_MAX_RESETS: usize = 3;
pub(crate) const RAW_VIDEO_EVENT_MARKER_LIMIT: usize = 48;
pub(crate) const TEXTURE_UPDATE_MARKER_LIMIT: usize = 32;
pub(crate) const TEXTURE_UPDATE_MARKER_PERIOD: usize = 121;
pub(crate) const TEXTURE_CONTENT_PROBE_MARKER_LIMIT: usize = 8;
pub(crate) const SYNTHETIC_LUMA_SLOT_PROOF: bool = false;
pub(crate) const SYNTHETIC_LUMA_ALL_SLOT_PROOF: bool = false;
pub(crate) const SYNTHETIC_LUMA_PROBE_SIZE: usize = 128;
pub(crate) const KEY_RUNTIME_PROFILE: &str = "runtime_profile";
pub(crate) const KEY_TRANSPORT_PROFILE: &str = "transport_profile";
pub(crate) const KEY_CAMERA_TIER: &str = "camera_tier";
pub(crate) const KEY_CAMERA_PROJECTION_MODE: &str = "camera_projection_mode";
pub(crate) const KEY_COMPARISON_BASELINE: &str = "comparison_baseline";
pub(crate) const KEY_SYNTHETIC_SCENE: &str = "synthetic_scene";
pub(crate) const KEY_ACQUISITION_PROFILE: &str = "acquisition_profile";
pub(crate) const KEY_PROJECTION_SCALE: &str = "projection_scale";
pub(crate) const KEY_PROJECTION_DEPTH_METERS: &str = "projection_depth_meters";
pub(crate) const KEY_CAMERA_PREVIEW_FOV_Y_DEGREES: &str = "camera_preview_fov_y_degrees";
pub(crate) const KEY_CAMERA_PREVIEW_OFFSET_Y_METERS: &str = "camera_preview_offset_y_meters";
pub(crate) const KEY_CAMERA_RAW_OVERLAY_OVERSCAN: &str = "camera_raw_overlay_overscan";
pub(crate) const KEY_XR_RENDER_SCALE: &str = "xr_render_scale";
pub(crate) const KEY_RENDERER: &str = "renderer";
pub(crate) const KEY_ANDROID_PACKAGER: &str = "android_packager";
pub(crate) const KEY_MAKEPAD_REVISION: &str = "makepad_revision";
pub(crate) const KEY_MAKEPAD_BRANCH: &str = "makepad_branch";
pub(crate) const KEY_STUDIO_HOST: &str = "studio_host";
pub(crate) const KEY_MAKEPAD_HORIZONTAL_ALIGNMENT_STRENGTH: &str =
    "makepad_horizontal_alignment_strength";
pub(crate) const KEY_MAKEPAD_HORIZONTAL_OFFSET_UV: &str = "makepad_horizontal_offset_uv";
pub(crate) const KEY_MAKEPAD_HORIZONTAL_OFFSET_LEFT_UV: &str = "makepad_horizontal_offset_left_uv";
pub(crate) const KEY_MAKEPAD_HORIZONTAL_OFFSET_RIGHT_UV: &str =
    "makepad_horizontal_offset_right_uv";
pub(crate) const KEY_MAKEPAD_VERTICAL_OFFSET_UV: &str = "makepad_vertical_offset_uv";
pub(crate) const KEY_MAKEPAD_CONTENT_UV_SCALE: &str = "makepad_content_uv_scale";
pub(crate) const KEY_MAKEPAD_PROJECTION_BORDER_OPACITY: &str = "makepad_projection_border_opacity";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_DIAGNOSTIC: &str =
    "makepad_projection_area_diagnostic";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_OFFSET_LEFT_UV: &str =
    "makepad_projection_area_offset_left_uv";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_OFFSET_RIGHT_UV: &str =
    "makepad_projection_area_offset_right_uv";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_OFFSET_VERTICAL_UV: &str =
    "makepad_projection_area_offset_vertical_uv";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_SCALE_X: &str = "makepad_projection_area_scale_x";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_SCALE_Y: &str = "makepad_projection_area_scale_y";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_RADIUS_X_UV: &str =
    "makepad_projection_area_radius_x_uv";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_RADIUS_Y_UV: &str =
    "makepad_projection_area_radius_y_uv";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_CORNER_RADIUS_UV: &str =
    "makepad_projection_area_corner_radius_uv";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_KEYSTONE_X: &str =
    "makepad_projection_area_keystone_x";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_BOW_X: &str = "makepad_projection_area_bow_x";
pub(crate) const KEY_MAKEPAD_PROJECTION_AREA_OPACITY: &str = "makepad_projection_area_opacity";
pub(crate) const KEY_MAKEPAD_PROJECTION_ALPHA_MODE: &str = "makepad_projection_alpha_mode";
pub(crate) const KEY_MAKEPAD_PROJECTION_ALPHA_SCALE: &str = "makepad_projection_alpha_scale";
pub(crate) const KEY_MAKEPAD_PROJECTION_ALPHA_BIAS: &str = "makepad_projection_alpha_bias";
pub(crate) const KEY_MAKEPAD_PROJECTION_BORDER_POLICY: &str = "makepad_projection_border_policy";
pub(crate) const KEY_MAKEPAD_PROJECTION_SAMPLE_MODE: &str = "makepad_projection_sample_mode";
pub(crate) const KEY_MAKEPAD_PROCESSING_LAYER: &str = "makepad_processing_layer";
pub(crate) const KEY_MAKEPAD_BLUR_RADIUS_PX: &str = "makepad_blur_radius_px";
pub(crate) const KEY_MAKEPAD_PROJECTION_RUNTIME_RESOLUTION_ENABLED: &str =
    "makepad_projection_runtime_resolution_enabled";
pub(crate) const KEY_MAKEPAD_NATIVE_PASSTHROUGH_ENABLED: &str =
    "makepad_native_passthrough_enabled";
pub(crate) const KEY_MAKEPAD_DIRECT_CAMERA_HARDWARE_BUFFER_EXTERNAL: &str =
    "makepad_direct_camera_hardware_buffer_external";
pub(crate) const KEY_MAKEPAD_BROKER_H264_ENABLED: &str = "makepad_broker_h264_enabled";
pub(crate) const KEY_MAKEPAD_BROKER_H264_HOST: &str = "makepad_broker_h264_host";
pub(crate) const KEY_MAKEPAD_BROKER_H264_BROKER_PORT: &str = "makepad_broker_h264_broker_port";
pub(crate) const KEY_MAKEPAD_BROKER_H264_STREAM_PORT: &str = "makepad_broker_h264_stream_port";
pub(crate) const KEY_MAKEPAD_BROKER_H264_RIGHT_STREAM_PORT: &str =
    "makepad_broker_h264_right_stream_port";
pub(crate) const KEY_MAKEPAD_BROKER_H264_SOURCE_MODE: &str = "makepad_broker_h264_source_mode";
pub(crate) const KEY_MAKEPAD_BROKER_H264_DECODE_OUTPUT_MODE: &str =
    "makepad_broker_h264_decode_output_mode";
pub(crate) const KEY_MAKEPAD_BROKER_H264_SYNTHETIC_PATTERN: &str =
    "makepad_broker_h264_synthetic_pattern";
pub(crate) const KEY_MAKEPAD_BROKER_H264_PROJECTION_GEOMETRY_PROFILE: &str =
    "makepad_broker_h264_projection_geometry_profile";
pub(crate) const KEY_MAKEPAD_BROKER_H264_SOURCE_SAMPLING_MODE: &str =
    "makepad_broker_h264_source_sampling_mode";
pub(crate) const KEY_MAKEPAD_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE: &str =
    "makepad_broker_h264_synthetic_projection_profile";
pub(crate) const KEY_MAKEPAD_BROKER_H264_TARGET_SCREEN_UV_RECT: &str =
    "makepad_broker_h264_target_screen_uv_rect";
pub(crate) const KEY_MAKEPAD_BROKER_H264_LEFT_TARGET_SCREEN_UV_RECT: &str =
    "makepad_broker_h264_left_target_screen_uv_rect";
pub(crate) const KEY_MAKEPAD_BROKER_H264_RIGHT_TARGET_SCREEN_UV_RECT: &str =
    "makepad_broker_h264_right_target_screen_uv_rect";
pub(crate) const KEY_CAMERA_PROJECTION_GEOMETRY_PROFILE: &str =
    "camera_projection_geometry_profile";
pub(crate) const KEY_MAKEPAD_CAMERA_PROJECTION_GEOMETRY_PROFILE: &str =
    "makepad_camera_projection_geometry_profile";
pub(crate) const KEY_CAMERA_SOURCE_SAMPLING_MODE: &str = "camera_source_sampling_mode";
pub(crate) const KEY_MAKEPAD_CAMERA_SOURCE_SAMPLING_MODE: &str =
    "makepad_camera_source_sampling_mode";
pub(crate) const KEY_CAMERA_TARGET_SCREEN_UV_RECT: &str = "camera_target_screen_uv_rect";
pub(crate) const KEY_CAMERA_LEFT_TARGET_SCREEN_UV_RECT: &str = "camera_left_target_screen_uv_rect";
pub(crate) const KEY_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT: &str =
    "camera_right_target_screen_uv_rect";
pub(crate) const KEY_MAKEPAD_CAMERA_TARGET_SCREEN_UV_RECT: &str =
    "makepad_camera_target_screen_uv_rect";
pub(crate) const KEY_MAKEPAD_CAMERA_LEFT_TARGET_SCREEN_UV_RECT: &str =
    "makepad_camera_left_target_screen_uv_rect";
pub(crate) const KEY_MAKEPAD_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT: &str =
    "makepad_camera_right_target_screen_uv_rect";
pub(crate) const KEY_MAKEPAD_BROKER_H264_LEFT_CAMERA_ID: &str =
    "makepad_broker_h264_left_camera_id";
pub(crate) const KEY_MAKEPAD_BROKER_H264_RIGHT_CAMERA_ID: &str =
    "makepad_broker_h264_right_camera_id";
pub(crate) const KEY_MAKEPAD_BROKER_H264_WIDTH: &str = "makepad_broker_h264_width";
pub(crate) const KEY_MAKEPAD_BROKER_H264_HEIGHT: &str = "makepad_broker_h264_height";
pub(crate) const KEY_MAKEPAD_BROKER_H264_CAPTURE_MS: &str = "makepad_broker_h264_capture_ms";
pub(crate) const KEY_MAKEPAD_BROKER_H264_MAX_PACKETS: &str = "makepad_broker_h264_max_packets";
pub(crate) const KEY_MAKEPAD_BROKER_H264_BITRATE_BPS: &str = "makepad_broker_h264_bitrate_bps";
pub(crate) const KEY_MAKEPAD_BROKER_H264_FRAME_RATE_HZ: &str = "makepad_broker_h264_frame_rate_hz";
pub(crate) const KEY_MAKEPAD_BROKER_H264_COMMAND_TIMEOUT_MS: &str =
    "makepad_broker_h264_command_timeout_ms";
pub(crate) const KEY_MAKEPAD_BROKER_H264_STREAM_TIMEOUT_MS: &str =
    "makepad_broker_h264_stream_timeout_ms";
pub(crate) const KEY_MAKEPAD_BROKER_H264_DECODE_TIMEOUT_MS: &str =
    "makepad_broker_h264_decode_timeout_ms";
pub(crate) const KEY_MAKEPAD_BROKER_H264_LIVE_STREAM: &str = "makepad_broker_h264_live_stream";
pub(crate) const KEY_MAKEPAD_BROKER_H264_STEREO_PAIR_ID: &str =
    "makepad_broker_h264_stereo_pair_id";
pub(crate) const KEY_MAKEPAD_BROKER_H264_STEREO_PAIR_MAX_DELTA_NS: &str =
    "makepad_broker_h264_stereo_pair_max_delta_ns";
pub(crate) const KEY_MANIFOLD_POSE_PUBLISH_ENABLED: &str = "manifold_pose_publish_enabled";
pub(crate) const KEY_MANIFOLD_POSE_STREAM: &str = "manifold_pose_stream";
pub(crate) const KEY_MANIFOLD_POSE_SOURCE: &str = "manifold_pose_source";
pub(crate) const KEY_MANIFOLD_POSE_CONTROLLER: &str = "manifold_pose_controller";
pub(crate) const KEY_MANIFOLD_POSE_KIND: &str = "manifold_pose_kind";
pub(crate) const KEY_MANIFOLD_BROKER_HOST: &str = "manifold_broker_host";
pub(crate) const KEY_MANIFOLD_BROKER_PORT: &str = "manifold_broker_port";
pub(crate) const KEY_MANIFOLD_POSE_SAMPLE_HZ: &str = "manifold_pose_sample_hz";
pub(crate) const KEY_MANIFOLD_POSE_CONNECT_TIMEOUT_MS: &str = "manifold_pose_connect_timeout_ms";
pub(crate) const KEY_MANIFOLD_BREATH_FEEDBACK_ENABLED: &str = "manifold_breath_feedback_enabled";
pub(crate) const KEY_MANIFOLD_BREATH_FEEDBACK_STREAM: &str = "manifold_breath_feedback_stream";
pub(crate) const KEY_MANIFOLD_BREATH_FEEDBACK_RECEIVER: &str = "manifold_breath_feedback_receiver";
pub(crate) const KEY_MANIFOLD_BREATH_FEEDBACK_CONNECT_TIMEOUT_MS: &str =
    "manifold_breath_feedback_connect_timeout_ms";
pub(crate) const KEY_MAKEPAD_PROJECTION_TARGET_JOYSTICK_CONTROLS: &str =
    "makepad_projection_target_joystick_controls";

pub(crate) fn makepad_runtime_config() -> RuntimeConfig {
    let mut config = RuntimeConfig::new();
    set_runtime_text(
        &mut config,
        KEY_RUNTIME_PROFILE,
        std::env::var("RUSTY_XR_RUNTIME_PROFILE").unwrap_or_else(|_| DEFAULT_PROFILE.to_string()),
        RuntimeConfigSource::Environment,
    );
    set_runtime_text(
        &mut config,
        KEY_TRANSPORT_PROFILE,
        std::env::var("RUSTY_XR_TRANSPORT_PROFILE")
            .unwrap_or_else(|_| DEFAULT_TRANSPORT.to_string()),
        RuntimeConfigSource::Environment,
    );
    set_runtime_text(
        &mut config,
        KEY_CAMERA_TIER,
        std::env::var("RUSTY_XR_CAMERA_TIER").unwrap_or_else(|_| DEFAULT_CAMERA_TIER.to_string()),
        RuntimeConfigSource::Environment,
    );
    set_runtime_text(
        &mut config,
        KEY_CAMERA_PROJECTION_MODE,
        runtime_property_value(KEY_CAMERA_PROJECTION_MODE)
            .or_else(|| std::env::var("RUSTY_XR_CAMERA_PROJECTION_MODE").ok())
            .unwrap_or_else(|| DEFAULT_CAMERA_PROJECTION_MODE.to_string()),
        RuntimeConfigSource::Environment,
    );
    set_runtime_text(
        &mut config,
        KEY_COMPARISON_BASELINE,
        std::env::var("RUSTY_XR_COMPARISON_BASELINE")
            .unwrap_or_else(|_| DEFAULT_COMPARISON_BASELINE.to_string()),
        RuntimeConfigSource::Environment,
    );
    set_runtime_text(
        &mut config,
        KEY_SYNTHETIC_SCENE,
        std::env::var("RUSTY_XR_SYNTHETIC_SCENE")
            .unwrap_or_else(|_| DEFAULT_SYNTHETIC_SCENE.to_string()),
        RuntimeConfigSource::Environment,
    );
    set_runtime_text(
        &mut config,
        KEY_ACQUISITION_PROFILE,
        std::env::var("RUSTY_XR_ACQUISITION_PROFILE")
            .unwrap_or_else(|_| DEFAULT_ACQUISITION_PROFILE.to_string()),
        RuntimeConfigSource::Environment,
    );
    set_runtime_float(
        &mut config,
        KEY_PROJECTION_SCALE,
        startup_f64(
            KEY_PROJECTION_SCALE,
            "RUSTY_XR_PROJECTION_SCALE",
            DEFAULT_PROJECTION_SCALE,
        ),
        RuntimeConfigSource::Environment,
    );
    set_runtime_float(
        &mut config,
        KEY_PROJECTION_DEPTH_METERS,
        startup_f64(
            KEY_PROJECTION_DEPTH_METERS,
            "RUSTY_XR_PROJECTION_DEPTH_METERS",
            DEFAULT_PROJECTION_DEPTH_METERS,
        ),
        RuntimeConfigSource::Environment,
    );
    set_runtime_float(
        &mut config,
        KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        startup_f64(
            KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
            "RUSTY_XR_CAMERA_PREVIEW_FOV_Y_DEGREES",
            TARGET_PROJECTION_PREVIEW_FOV_Y_DEGREES as f64,
        ),
        RuntimeConfigSource::Environment,
    );
    set_runtime_float(
        &mut config,
        KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
        startup_signed_f64(
            KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
            "RUSTY_XR_CAMERA_PREVIEW_OFFSET_Y_METERS",
            TARGET_PROJECTION_PREVIEW_OFFSET_Y_METERS as f64,
        ),
        RuntimeConfigSource::Environment,
    );
    set_runtime_float(
        &mut config,
        KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        startup_f64(
            KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
            "RUSTY_XR_CAMERA_RAW_OVERLAY_OVERSCAN",
            TARGET_PROJECTION_RAW_OVERSCAN as f64,
        ),
        RuntimeConfigSource::Environment,
    );
    set_runtime_float(
        &mut config,
        KEY_XR_RENDER_SCALE,
        startup_f64(
            KEY_XR_RENDER_SCALE,
            "RUSTY_XR_RENDER_SCALE",
            DEFAULT_XR_RENDER_SCALE,
        ),
        RuntimeConfigSource::Environment,
    );
    set_runtime_text(
        &mut config,
        KEY_RENDERER,
        "makepad".to_string(),
        RuntimeConfigSource::Synthetic,
    );
    set_runtime_text(
        &mut config,
        KEY_ANDROID_PACKAGER,
        "cargo-makepad".to_string(),
        RuntimeConfigSource::Synthetic,
    );
    set_runtime_text(
        &mut config,
        KEY_MAKEPAD_REVISION,
        MAKEPAD_REV.to_string(),
        RuntimeConfigSource::Synthetic,
    );
    set_runtime_text(
        &mut config,
        KEY_MAKEPAD_BRANCH,
        MAKEPAD_BRANCH.to_string(),
        RuntimeConfigSource::Synthetic,
    );
    set_runtime_text(
        &mut config,
        KEY_STUDIO_HOST,
        std::env::var("STUDIO_HOST").unwrap_or_else(|_| "unset".to_string()),
        RuntimeConfigSource::Environment,
    );
    config
}

fn set_runtime_text(
    config: &mut RuntimeConfig,
    key: &'static str,
    value: String,
    source: RuntimeConfigSource,
) {
    config
        .set(key, RuntimeValue::Text(value), source)
        .expect("runtime config keys should be public-safe constants");
}

fn set_runtime_float(
    config: &mut RuntimeConfig,
    key: &'static str,
    value: f64,
    source: RuntimeConfigSource,
) {
    config
        .set(key, RuntimeValue::Float(value), source)
        .expect("runtime config keys should be public-safe constants");
}

pub(crate) fn runtime_text(config: &RuntimeConfig, key: &str) -> String {
    config
        .get(key)
        .and_then(RuntimeValue::as_text)
        .unwrap_or("")
        .to_string()
}

pub(crate) fn runtime_float(config: &RuntimeConfig, key: &str) -> f64 {
    config
        .get(key)
        .and_then(RuntimeValue::as_float)
        .unwrap_or(0.0)
}

fn startup_f64(runtime_key: &'static str, env_key: &str, default: f64) -> f64 {
    runtime_property_value(runtime_key)
        .or_else(|| std::env::var(env_key).ok())
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
}

fn startup_signed_f64(runtime_key: &'static str, env_key: &str, default: f64) -> f64 {
    runtime_property_value(runtime_key)
        .or_else(|| std::env::var(env_key).ok())
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

pub(crate) fn hotload_f32(key: &'static str, default: f32, min: f32, max: f32) -> f32 {
    runtime_property_value(key)
        .or_else(|| std::env::var(runtime_env_key(key)).ok())
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

pub(crate) fn hotload_f32_any(keys: &[&'static str], default: f32, min: f32, max: f32) -> f32 {
    keys.iter()
        .find_map(|key| {
            runtime_property_value(key)
                .or_else(|| std::env::var(runtime_env_key(key)).ok())
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite())
        })
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

pub(crate) fn hotload_bool(key: &'static str, default: bool) -> bool {
    runtime_property_value(key)
        .or_else(|| std::env::var(runtime_env_key(key)).ok())
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on" | "enabled"
            )
        })
        .unwrap_or(default)
}

pub(crate) fn hotload_text(key: &'static str, default: &str) -> String {
    runtime_property_value(key)
        .or_else(|| std::env::var(runtime_env_key(key)).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

pub(crate) fn hotload_text_any(keys: &[&'static str], default: &str) -> String {
    keys.iter()
        .find_map(|key| {
            runtime_property_value(key)
                .or_else(|| std::env::var(runtime_env_key(key)).ok())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| default.to_string())
}

pub(crate) fn hotload_u32(key: &'static str, default: u32, min: u32, max: u32) -> u32 {
    runtime_property_value(key)
        .or_else(|| std::env::var(runtime_env_key(key)).ok())
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

pub(crate) fn hotload_u16(key: &'static str, default: u16, min: u16, max: u16) -> u16 {
    hotload_u32(key, default as u32, min as u32, max as u32) as u16
}

fn runtime_env_key(key: &str) -> String {
    format!(
        "RUSTY_XR_{}",
        key.replace(['-', '.'], "_").to_ascii_uppercase()
    )
}

#[cfg(target_os = "android")]
fn runtime_property_name(key: &'static str) -> String {
    RuntimeKey::new(key)
        .expect("runtime config key should be valid")
        .android_property(&AndroidPropertyPrefix::default())
}

#[cfg(target_os = "android")]
pub(crate) fn runtime_property_value(key: &'static str) -> Option<String> {
    android_system_property_value(&runtime_property_name(key))
}

#[cfg(target_os = "android")]
pub(crate) fn android_system_property_value(name: &str) -> Option<String> {
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_int};

    #[link(name = "c")]
    unsafe extern "C" {
        fn __system_property_get(name: *const c_char, value: *mut c_char) -> c_int;
    }

    let name = CString::new(name).ok()?;
    let mut value = [0 as c_char; 128];
    let len = unsafe { __system_property_get(name.as_ptr(), value.as_mut_ptr()) };
    if len <= 0 {
        return None;
    }
    let value = unsafe { CStr::from_ptr(value.as_ptr()) }
        .to_string_lossy()
        .trim()
        .to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(not(target_os = "android"))]
pub(crate) fn runtime_property_value(_key: &'static str) -> Option<String> {
    None
}

#[cfg(not(target_os = "android"))]
pub(crate) fn android_system_property_value(_name: &str) -> Option<String> {
    None
}

pub(crate) fn marker_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}
