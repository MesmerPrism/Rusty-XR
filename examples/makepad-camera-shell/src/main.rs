pub use makepad_xr::makepad_widgets;

#[cfg(target_os = "android")]
mod acamera_sys;
#[cfg(target_os = "android")]
mod android_camera_probe;
mod source_sampling;
mod projection_runtime;
mod projection_geometry;
mod source_metadata;
use projection_geometry::{
    makepad_draw_vars_bound_marker_fields, makepad_projection_target_marker_fields,
    makepad_visible_panel_bound_marker_fields, projection_homography_marker_fields,
    MakepadOpenXrProjectionContract,
};
#[cfg(target_os = "android")]
use projection_geometry::broker_projection_plan_marker_fields;
use projection_runtime::{
    makepad_current_projection_runtime_float, makepad_horizontal_alignment_tuning_from_resolution,
    makepad_projection_runtime_manifest_lines, makepad_projection_runtime_resolution,
    makepad_projection_runtime_resolution_enabled,
};
use source_metadata::{
    broker_pair_content_geometry_marker_fields, direct_camera2_content_geometry_marker_fields,
    missing_broker_content_geometry_marker_fields, normalize_direct_camera_projection_geometry_profile,
    stream_header_metadata_marker_fields,
    BrokerH264ProjectionMetadata,
};

use makepad_widgets::makepad_platform::{
    event::video_playback::{
        BrokerH264VideoSource, CameraPreviewMode, TextureHandleReadyEvent, VideoSource,
        VideoYuvMetadata,
    },
    permission::Permission,
    thread::SignalToUI,
    video::{VideoFormat, VideoInputsEvent, VideoPixelFormat},
    TextureFormat, TextureId, TextureUpdated,
};
use makepad_widgets::*;
use makepad_xr::scene::{xr_widget_world_transform, XrNode};
use rusty_xr_camera_model::Rect2;
use rusty_xr_runtime_config as rxrc;
#[cfg(target_os = "android")]
use rusty_xr_runtime_config::{AndroidPropertyPrefix, RuntimeKey};
use rusty_xr_runtime_config::{RuntimeConfig, RuntimeConfigSource, RuntimeValue};
use source_sampling::MakepadSourceSamplingHandoff;
use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
    time::Duration,
};

app_main!(App);

#[cfg(target_os = "android")]
fn main() {
    // Makepad Android launches through the JNI entrypoint emitted by app_main!.
    // Plain Cargo target checks still compile this source as a binary crate.
}

static STARTUP_MARKERS_EMITTED: AtomicBool = AtomicBool::new(false);
static PAIRED_IMPORT_SIGNAL_READY: AtomicBool = AtomicBool::new(false);
static CAMERA_PANEL_DRAW_MARKER_EMITTED: AtomicBool = AtomicBool::new(false);
static VIDEO_EVENT_RAW_MARKERS_EMITTED: AtomicUsize = AtomicUsize::new(0);
static TEXTURE_UPDATE_MARKERS_EMITTED: AtomicUsize = AtomicUsize::new(0);
static TEXTURE_CONTENT_PROBE_MARKERS_EMITTED: AtomicUsize = AtomicUsize::new(0);

const DEFAULT_PROFILE: &str = "makepad-stereo-projection-pair-probe";
const DEFAULT_TRANSPORT: &str = "makepad-s109-red-projection-border-passthrough-off";
const DEFAULT_CAMERA_TIER: &str = "native-camera2-makepad-stereo-vulkan-import-probe";
const DEFAULT_CAMERA_PROJECTION_MODE: &str = "display-screen-homography";
const DEFAULT_COMPARISON_BASELINE: &str = "custom-apk-camera-stereo-gpu-composite";
const DEFAULT_SYNTHETIC_SCENE: &str =
    "camera-panel-s118-projected-footprint-red-border-passthrough-off";
const DEFAULT_ACQUISITION_PROFILE: &str =
    "bounded-camera2-private-plus-makepad-paired-import-probe";
const DEFAULT_PROJECTION_SCALE: f64 = 1.0;
const DEFAULT_PROJECTION_DEPTH_METERS: f64 = 1.0;
const DEFAULT_XR_RENDER_SCALE: f64 = 1.0;
const DEFAULT_BROKER_H264_ENABLED: bool = false;
const DEFAULT_BROKER_H264_HOST: &str = "127.0.0.1";
const DEFAULT_BROKER_H264_BROKER_PORT: u16 = 8765;
const DEFAULT_BROKER_H264_STREAM_PORT: u16 = 8879;
const DEFAULT_BROKER_H264_RIGHT_STREAM_PORT: u16 = 8880;
const DEFAULT_BROKER_H264_SOURCE_MODE: &str = "broker-synthetic";
const DEFAULT_BROKER_H264_SYNTHETIC_PATTERN: &str = "diagnostic-grid";
const DEFAULT_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE: &str = "head-anchored-virtual-camera";
const DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE: &str = "full-frame-diagnostic";
const DEFAULT_BROKER_H264_LEFT_CAMERA_ID: &str = "";
const DEFAULT_BROKER_H264_RIGHT_CAMERA_ID: &str = "";
const DEFAULT_BROKER_H264_WIDTH: u32 = 1280;
const DEFAULT_BROKER_H264_HEIGHT: u32 = 1280;
const DEFAULT_BROKER_H264_CAPTURE_MS: u32 = 45_000;
const DEFAULT_BROKER_H264_MAX_PACKETS: u32 = 0;
const DEFAULT_BROKER_H264_BITRATE_BPS: u32 = 6_000_000;
const DEFAULT_BROKER_H264_FRAME_RATE_HZ: u32 = 30;
const DEFAULT_BROKER_H264_COMMAND_TIMEOUT_MS: u32 = 10_000;
const DEFAULT_BROKER_H264_STREAM_TIMEOUT_MS: u32 = 30_000;
const DEFAULT_BROKER_H264_DECODE_TIMEOUT_MS: u32 = 20_000;
const DEFAULT_BROKER_H264_LIVE_STREAM: bool = true;
const SUPPRESS_LIVE_CAMERA_SAMPLING: bool = false;
const FORCE_FULL_SURFACE_LIVE_CAMERA_UV: bool = false;
const FORCE_IN_SURFACE_CAMERA_WINDOW: bool = true;
const TARGET_HORIZONTAL_ALIGNMENT_STRENGTH: f32 = 0.0;
const TARGET_MANUAL_HORIZONTAL_OFFSET_LEFT_UV: f32 = 0.0;
const TARGET_MANUAL_HORIZONTAL_OFFSET_RIGHT_UV: f32 = 0.0;
const TARGET_MANUAL_VERTICAL_OFFSET_UV: f32 = 0.0;
const TARGET_FULL_VIEW_CONTENT_UV_SCALE: f32 = 1.60;
const TARGET_PROJECTION_BORDER_OPACITY: f32 = 1.0;
const TARGET_PROJECTION_AREA_DIAGNOSTIC: f32 = 0.0;
const TARGET_PROJECTION_AREA_OFFSET_LEFT_UV: f32 = 0.0;
const TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV: f32 = 0.0;
const TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV: f32 = 0.0;
const TARGET_PROJECTION_AREA_SCALE_X: f32 = 1.0;
const TARGET_PROJECTION_AREA_SCALE_Y: f32 = 1.0;
const TARGET_PROJECTION_AREA_RADIUS_X_UV: f32 = 0.5;
const TARGET_PROJECTION_AREA_RADIUS_Y_UV: f32 = 0.5;
const TARGET_PROJECTION_AREA_CORNER_RADIUS_UV: f32 = 0.0;
const TARGET_PROJECTION_AREA_KEYSTONE_X: f32 = 0.0;
const TARGET_PROJECTION_AREA_BOW_X: f32 = 0.0;
const TARGET_PROJECTION_AREA_OPACITY: f32 = 1.0;
const SOURCE_VALID_FOOTPRINT_GRID: usize = 64;
const TARGET_DISPLAY_EYE_OFFSET_METERS: f32 = 0.032;
const TARGET_DISPLAY_FOV_Y_DEGREES: f32 = 92.0;
const TARGET_DISPLAY_ASPECT: f32 = 1.0;
const TARGET_PROJECTION_DEPTH_METERS: f32 = DEFAULT_PROJECTION_DEPTH_METERS as f32;
const TARGET_PROJECTION_PREVIEW_FOV_Y_DEGREES: f32 = 60.0;
const TARGET_PROJECTION_RAW_OVERSCAN: f32 = 1.06;
const FRAME_RASTER_TOP_LEFT_Y_DOWN: &str = "top-left-origin-y-down";
const FRAME_RASTER_BOTTOM_LEFT_Y_UP: &str = "bottom-left-origin-y-up";
const IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY: [[f32; 3]; 3] =
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
const MAKEPAD_BRANCH: &str = "rusty-xr/android-libstd-packaging";
const MAKEPAD_REV: &str = "2952b07c";
const DEFAULT_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING: &str = "display-left-from-left-source";
const PAIRED_IMPORT_DELAY_SECONDS: f64 = 6.0;
const PAIRED_IMPORT_RETRY_SECONDS: f64 = 1.0;
const PAIRED_IMPORT_MAX_WAITS: usize = 10;
const CADENCE_SAMPLE_SECONDS: f64 = 5.0;
// S25 showed this diagnostic can reintroduce app-process GPU page faults on Quest.
const NATIVE_VIDEO_WIDGET_SURFACE_DIAGNOSTIC: bool = false;
const NATIVE_VIDEO_WIDGET_RETRY_SECONDS: f64 = 0.5;
const NATIVE_VIDEO_WIDGET_MAX_RESETS: usize = 3;
const RAW_VIDEO_EVENT_MARKER_LIMIT: usize = 48;
const TEXTURE_UPDATE_MARKER_LIMIT: usize = 32;
const TEXTURE_CONTENT_PROBE_MARKER_LIMIT: usize = 8;
const SYNTHETIC_LUMA_SLOT_PROOF: bool = false;
const SYNTHETIC_LUMA_ALL_SLOT_PROOF: bool = false;
const SYNTHETIC_LUMA_PROBE_SIZE: usize = 128;
const KEY_RUNTIME_PROFILE: &str = "runtime_profile";
const KEY_TRANSPORT_PROFILE: &str = "transport_profile";
const KEY_CAMERA_TIER: &str = "camera_tier";
const KEY_CAMERA_PROJECTION_MODE: &str = "camera_projection_mode";
const KEY_COMPARISON_BASELINE: &str = "comparison_baseline";
const KEY_SYNTHETIC_SCENE: &str = "synthetic_scene";
const KEY_ACQUISITION_PROFILE: &str = "acquisition_profile";
const KEY_PROJECTION_SCALE: &str = "projection_scale";
const KEY_PROJECTION_DEPTH_METERS: &str = "projection_depth_meters";
const KEY_CAMERA_PREVIEW_FOV_Y_DEGREES: &str = "camera_preview_fov_y_degrees";
const KEY_CAMERA_PREVIEW_OFFSET_Y_METERS: &str = "camera_preview_offset_y_meters";
const KEY_CAMERA_RAW_OVERLAY_OVERSCAN: &str = "camera_raw_overlay_overscan";
const KEY_XR_RENDER_SCALE: &str = "xr_render_scale";
const KEY_RENDERER: &str = "renderer";
const KEY_ANDROID_PACKAGER: &str = "android_packager";
const KEY_MAKEPAD_REVISION: &str = "makepad_revision";
const KEY_MAKEPAD_BRANCH: &str = "makepad_branch";
const KEY_STUDIO_HOST: &str = "studio_host";
const KEY_MAKEPAD_HORIZONTAL_ALIGNMENT_STRENGTH: &str = "makepad_horizontal_alignment_strength";
const KEY_MAKEPAD_HORIZONTAL_OFFSET_UV: &str = "makepad_horizontal_offset_uv";
const KEY_MAKEPAD_HORIZONTAL_OFFSET_LEFT_UV: &str = "makepad_horizontal_offset_left_uv";
const KEY_MAKEPAD_HORIZONTAL_OFFSET_RIGHT_UV: &str = "makepad_horizontal_offset_right_uv";
const KEY_MAKEPAD_VERTICAL_OFFSET_UV: &str = "makepad_vertical_offset_uv";
const KEY_MAKEPAD_CONTENT_UV_SCALE: &str = "makepad_content_uv_scale";
const KEY_MAKEPAD_PROJECTION_BORDER_OPACITY: &str = "makepad_projection_border_opacity";
const KEY_MAKEPAD_PROJECTION_AREA_DIAGNOSTIC: &str = "makepad_projection_area_diagnostic";
const KEY_MAKEPAD_PROJECTION_AREA_OFFSET_LEFT_UV: &str = "makepad_projection_area_offset_left_uv";
const KEY_MAKEPAD_PROJECTION_AREA_OFFSET_RIGHT_UV: &str = "makepad_projection_area_offset_right_uv";
const KEY_MAKEPAD_PROJECTION_AREA_OFFSET_VERTICAL_UV: &str =
    "makepad_projection_area_offset_vertical_uv";
const KEY_MAKEPAD_PROJECTION_AREA_SCALE_X: &str = "makepad_projection_area_scale_x";
const KEY_MAKEPAD_PROJECTION_AREA_SCALE_Y: &str = "makepad_projection_area_scale_y";
const KEY_MAKEPAD_PROJECTION_AREA_RADIUS_X_UV: &str = "makepad_projection_area_radius_x_uv";
const KEY_MAKEPAD_PROJECTION_AREA_RADIUS_Y_UV: &str = "makepad_projection_area_radius_y_uv";
const KEY_MAKEPAD_PROJECTION_AREA_CORNER_RADIUS_UV: &str =
    "makepad_projection_area_corner_radius_uv";
const KEY_MAKEPAD_PROJECTION_AREA_KEYSTONE_X: &str = "makepad_projection_area_keystone_x";
const KEY_MAKEPAD_PROJECTION_AREA_BOW_X: &str = "makepad_projection_area_bow_x";
const KEY_MAKEPAD_PROJECTION_AREA_OPACITY: &str = "makepad_projection_area_opacity";
const KEY_MAKEPAD_PROJECTION_ALPHA_MODE: &str = "makepad_projection_alpha_mode";
const KEY_MAKEPAD_PROJECTION_ALPHA_SCALE: &str = "makepad_projection_alpha_scale";
const KEY_MAKEPAD_PROJECTION_ALPHA_BIAS: &str = "makepad_projection_alpha_bias";
const KEY_MAKEPAD_PROJECTION_BORDER_POLICY: &str = "makepad_projection_border_policy";
const KEY_MAKEPAD_PROCESSING_LAYER: &str = "makepad_processing_layer";
const KEY_MAKEPAD_BLUR_RADIUS_PX: &str = "makepad_blur_radius_px";
const KEY_MAKEPAD_PROJECTION_RUNTIME_RESOLUTION_ENABLED: &str =
    "makepad_projection_runtime_resolution_enabled";
const KEY_MAKEPAD_NATIVE_PASSTHROUGH_ENABLED: &str = "makepad_native_passthrough_enabled";
const KEY_MAKEPAD_BROKER_H264_ENABLED: &str = "makepad_broker_h264_enabled";
const KEY_MAKEPAD_BROKER_H264_HOST: &str = "makepad_broker_h264_host";
const KEY_MAKEPAD_BROKER_H264_BROKER_PORT: &str = "makepad_broker_h264_broker_port";
const KEY_MAKEPAD_BROKER_H264_STREAM_PORT: &str = "makepad_broker_h264_stream_port";
const KEY_MAKEPAD_BROKER_H264_RIGHT_STREAM_PORT: &str = "makepad_broker_h264_right_stream_port";
const KEY_MAKEPAD_BROKER_H264_SOURCE_MODE: &str = "makepad_broker_h264_source_mode";
const KEY_MAKEPAD_BROKER_H264_SYNTHETIC_PATTERN: &str = "makepad_broker_h264_synthetic_pattern";
const KEY_MAKEPAD_BROKER_H264_PROJECTION_GEOMETRY_PROFILE: &str =
    "makepad_broker_h264_projection_geometry_profile";
const KEY_MAKEPAD_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE: &str =
    "makepad_broker_h264_synthetic_projection_profile";
const KEY_MAKEPAD_CAMERA_PROJECTION_GEOMETRY_PROFILE: &str =
    "makepad_camera_projection_geometry_profile";
const KEY_MAKEPAD_BROKER_H264_LEFT_CAMERA_ID: &str = "makepad_broker_h264_left_camera_id";
const KEY_MAKEPAD_BROKER_H264_RIGHT_CAMERA_ID: &str = "makepad_broker_h264_right_camera_id";
const KEY_MAKEPAD_BROKER_H264_WIDTH: &str = "makepad_broker_h264_width";
const KEY_MAKEPAD_BROKER_H264_HEIGHT: &str = "makepad_broker_h264_height";
const KEY_MAKEPAD_BROKER_H264_CAPTURE_MS: &str = "makepad_broker_h264_capture_ms";
const KEY_MAKEPAD_BROKER_H264_MAX_PACKETS: &str = "makepad_broker_h264_max_packets";
const KEY_MAKEPAD_BROKER_H264_BITRATE_BPS: &str = "makepad_broker_h264_bitrate_bps";
const KEY_MAKEPAD_BROKER_H264_FRAME_RATE_HZ: &str = "makepad_broker_h264_frame_rate_hz";
const KEY_MAKEPAD_BROKER_H264_COMMAND_TIMEOUT_MS: &str = "makepad_broker_h264_command_timeout_ms";
const KEY_MAKEPAD_BROKER_H264_STREAM_TIMEOUT_MS: &str = "makepad_broker_h264_stream_timeout_ms";
const KEY_MAKEPAD_BROKER_H264_DECODE_TIMEOUT_MS: &str = "makepad_broker_h264_decode_timeout_ms";
const KEY_MAKEPAD_BROKER_H264_LIVE_STREAM: &str = "makepad_broker_h264_live_stream";

script_mod! {
    use mod.pod.*
    use mod.math.*
    use mod.shader.*
    use mod.draw
    use mod.geom
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.draw.DrawMakepadStereoCameraPanel = mod.std.set_type_default() do #(DrawMakepadStereoCameraPanel::script_shader(vm)){
        alpha_blend: true
        backface_culling: false
        vertex_pos: vertex_position(vec4f)
        fb0: fragment_output(0, vec4f)
        draw_call: uniform_buffer(draw.DrawCallUniforms)
        draw_pass: uniform_buffer(draw.DrawPassUniforms)
        draw_list: uniform_buffer(draw.DrawListUniforms)
        geom: vertex_buffer(geom.QuadVertex, geom.QuadGeom)

        world: varying(vec4f)

        left_camera_texture: texture_video()
        right_camera_texture: texture_video()
        left_tex_y: texture_2d(float)
        left_tex_u: texture_2d(float)
        left_tex_v: texture_2d(float)
        right_tex_y: texture_2d(float)
        right_tex_u: texture_2d(float)
        right_tex_v: texture_2d(float)
        left_projection_h00: 1.0
        left_projection_h01: 0.0
        left_projection_h02: 0.0
        left_projection_h10: 0.0
        left_projection_h11: 1.0
        left_projection_h12: 0.0
        left_projection_h20: 0.0
        left_projection_h21: 0.0
        left_projection_h22: 1.0
        right_projection_h00: 1.0
        right_projection_h01: 0.0
        right_projection_h02: 0.0
        right_projection_h10: 0.0
        right_projection_h11: 1.0
        right_projection_h12: 0.0
        right_projection_h20: 0.0
        right_projection_h21: 0.0
        right_projection_h22: 1.0
        left_screen_to_camera_h00: 1.0
        left_screen_to_camera_h01: 0.0
        left_screen_to_camera_h02: 0.0
        left_screen_to_camera_h10: 0.0
        left_screen_to_camera_h11: 1.0
        left_screen_to_camera_h12: 0.0
        left_screen_to_camera_h20: 0.0
        left_screen_to_camera_h21: 0.0
        left_screen_to_camera_h22: 1.0
        right_screen_to_camera_h00: 1.0
        right_screen_to_camera_h01: 0.0
        right_screen_to_camera_h02: 0.0
        right_screen_to_camera_h10: 0.0
        right_screen_to_camera_h11: 1.0
        right_screen_to_camera_h12: 0.0
        right_screen_to_camera_h20: 0.0
        right_screen_to_camera_h21: 0.0
        right_screen_to_camera_h22: 1.0
        left_screen_to_surface_h00: 1.0
        left_screen_to_surface_h01: 0.0
        left_screen_to_surface_h02: 0.0
        left_screen_to_surface_h10: 0.0
        left_screen_to_surface_h11: 1.0
        left_screen_to_surface_h12: 0.0
        left_screen_to_surface_h20: 0.0
        left_screen_to_surface_h21: 0.0
        left_screen_to_surface_h22: 1.0
        right_screen_to_surface_h00: 1.0
        right_screen_to_surface_h01: 0.0
        right_screen_to_surface_h02: 0.0
        right_screen_to_surface_h10: 0.0
        right_screen_to_surface_h11: 1.0
        right_screen_to_surface_h12: 0.0
        right_screen_to_surface_h20: 0.0
        right_screen_to_surface_h21: 0.0
        right_screen_to_surface_h22: 1.0
        content_uv_scale: 1.60
        display_eye_offset_meters: 0.032
        display_fov_y_degrees: 92.0
        display_aspect: 1.0
        projection_depth_meters: 1.0
        projection_preview_offset_y_meters: 0.0
        projection_preview_fov_y_degrees: 60.0
        projection_raw_overscan: 1.06
        suppress_live_camera_sampling: 1.0
        force_full_surface_live_camera_uv: 1.0
        force_in_surface_camera_window: 1.0
        projection_border_opacity: 1.0
        projection_border_policy: 0.0
        processing_layer: 0.0
        blur_radius_px: 2.0
        projection_area_diagnostic: 0.0
        projection_area_offset_left_uv: 0.0
        projection_area_offset_right_uv: 0.0
        projection_area_offset_vertical_uv: 0.0
        projection_area_scale_x: 1.0
        projection_area_scale_y: 1.0
        projection_area_radius_x_uv: 0.5
        projection_area_radius_y_uv: 0.5
        projection_area_corner_radius_uv: 0.0
        projection_area_keystone_x: 0.0
        projection_area_bow_x: 0.0
        projection_area_opacity: 1.0
        projection_alpha_mode: 0.0
        projection_alpha_scale: 1.0
        projection_alpha_bias: 0.0
        source_sample_y_flip: 0.0
        projection_content_mapping_mode: 0.0
        display_source_eye_swap: 0.0
        manual_vertical_offset_uv: 0.0
        v_uv: varying(vec2f)

        cube_size: vec3(1.0, 1.0, 1.0)
        cube_pos: vec3(0.0, 0.0, 0.0)
        depth_clip: 0.0

        get_size: fn() {
            return self.cube_size
        }

        get_pos: fn() {
            return self.cube_pos
        }

        vertex: fn() {
            let screen_uv = clamp(self.geom.pos, vec2(0.0, 0.0), vec2(1.0, 1.0));
            self.world = vec4(screen_uv.x, screen_uv.y, 0.0, 1.0);
            self.v_uv = screen_uv;
            self.vertex_pos = vec4(screen_uv.x * 2.0 - 1.0, screen_uv.y * 2.0 - 1.0, 0.0, 1.0);
        }

        active_eye_is_right: fn() -> float {
            return clamp(xr_view_id(), 0.0, 1.0);
        }

        source_eye_selector: fn() -> float {
            let display_eye = self.active_eye_is_right();
            return mix(display_eye, 1.0 - display_eye, self.display_source_eye_swap);
        }

        apply_projection_homography: fn(
            coord: vec2f,
            h00: float,
            h01: float,
            h02: float,
            h10: float,
            h11: float,
            h12: float,
            h20: float,
            h21: float,
            h22: float
        ) -> vec2f {
            let x = h00 * coord.x + h01 * coord.y + h02;
            let y = h10 * coord.x + h11 * coord.y + h12;
            let w = h20 * coord.x + h21 * coord.y + h22;
            let safe_w = mix(1.0, w, step(0.00001, abs(w)));
            return vec2(x, y) / safe_w;
        }

        source_camera_uv: fn(coord: vec2f, selector: float) -> vec2f {
            let left_uv = self.apply_projection_homography(
                coord,
                self.left_projection_h00,
                self.left_projection_h01,
                self.left_projection_h02,
                self.left_projection_h10,
                self.left_projection_h11,
                self.left_projection_h12,
                self.left_projection_h20,
                self.left_projection_h21,
                self.left_projection_h22
            );
            let right_uv = self.apply_projection_homography(
                coord,
                self.right_projection_h00,
                self.right_projection_h01,
                self.right_projection_h02,
                self.right_projection_h10,
                self.right_projection_h11,
                self.right_projection_h12,
                self.right_projection_h20,
                self.right_projection_h21,
                self.right_projection_h22
            );
            return mix(left_uv, right_uv, selector);
        }

        source_screen_camera_uv: fn(coord: vec2f, selector: float) -> vec2f {
            let left_uv = self.apply_projection_homography(
                coord,
                self.left_screen_to_camera_h00,
                self.left_screen_to_camera_h01,
                self.left_screen_to_camera_h02,
                self.left_screen_to_camera_h10,
                self.left_screen_to_camera_h11,
                self.left_screen_to_camera_h12,
                self.left_screen_to_camera_h20,
                self.left_screen_to_camera_h21,
                self.left_screen_to_camera_h22
            );
            let right_uv = self.apply_projection_homography(
                coord,
                self.right_screen_to_camera_h00,
                self.right_screen_to_camera_h01,
                self.right_screen_to_camera_h02,
                self.right_screen_to_camera_h10,
                self.right_screen_to_camera_h11,
                self.right_screen_to_camera_h12,
                self.right_screen_to_camera_h20,
                self.right_screen_to_camera_h21,
                self.right_screen_to_camera_h22
            );
            return mix(left_uv, right_uv, selector);
        }

        screen_surface_uv: fn(coord: vec2f, display_eye_selector: float) -> vec2f {
            let left_uv = self.apply_projection_homography(
                coord,
                self.left_screen_to_surface_h00,
                self.left_screen_to_surface_h01,
                self.left_screen_to_surface_h02,
                self.left_screen_to_surface_h10,
                self.left_screen_to_surface_h11,
                self.left_screen_to_surface_h12,
                self.left_screen_to_surface_h20,
                self.left_screen_to_surface_h21,
                self.left_screen_to_surface_h22
            );
            let right_uv = self.apply_projection_homography(
                coord,
                self.right_screen_to_surface_h00,
                self.right_screen_to_surface_h01,
                self.right_screen_to_surface_h02,
                self.right_screen_to_surface_h10,
                self.right_screen_to_surface_h11,
                self.right_screen_to_surface_h12,
                self.right_screen_to_surface_h20,
                self.right_screen_to_surface_h21,
                self.right_screen_to_surface_h22
            );
            return mix(left_uv, right_uv, display_eye_selector);
        }

        projection_area_screen_uv: fn(coord: vec2f, display_eye_selector: float) -> vec2f {
            let offset_x = mix(
                self.projection_area_offset_left_uv,
                self.projection_area_offset_right_uv,
                display_eye_selector
            );
            let scale = max(
                vec2(self.projection_area_scale_x, self.projection_area_scale_y),
                vec2(0.05, 0.05)
            );
            let scaled = (coord - vec2(0.5, 0.5)) * scale + vec2(0.5, 0.5);
            let y = clamp(scaled.y, 0.0, 1.0);
            let keystone_x = clamp(self.projection_area_keystone_x, -0.45, 0.45);
            let bow_x = clamp(self.projection_area_bow_x, -0.25, 0.25);
            let midpoint_bow = 4.0 * y * (1.0 - y);
            let x_scale = max(0.05, 1.0 + keystone_x * (1.0 - 2.0 * y) + bow_x * midpoint_bow);
            let keystoned = vec2((scaled.x - 0.5) * x_scale + 0.5, scaled.y);
            return keystoned + vec2(offset_x, self.projection_area_offset_vertical_uv);
        }

        clamp_border_seed_uv: fn(seed_uv: vec2f) -> vec2f {
            let center = vec2(0.5, 0.5);
            let radius = vec2(0.31, 0.28);
            let p = (seed_uv - center) / radius;
            let len = max(length(p), 1.0);
            return center + (p / len) * radius;
        }

        screen_to_head_surface_uv: fn(screen_uv: vec2f) -> vec2f {
            let eye_selector = self.active_eye_is_right();
            let eye_sign = mix(-1.0, 1.0, eye_selector);
            let eye_origin4 = self.draw_pass.camera_inv * vec4(0.0, 0.0, 0.0, 1.0);
            let right4 = self.draw_pass.camera_inv * vec4(1.0, 0.0, 0.0, 0.0);
            let up4 = self.draw_pass.camera_inv * vec4(0.0, 1.0, 0.0, 0.0);
            let forward4 = self.draw_pass.camera_inv * vec4(0.0, 0.0, -1.0, 0.0);
            let eye_origin = eye_origin4.xyz;
            let right = normalize(right4.xyz);
            let up = normalize(up4.xyz);
            let forward = normalize(forward4.xyz);
            let head_origin = eye_origin - right * (eye_sign * self.display_eye_offset_meters);

            let ndc = vec2(screen_uv.x * 2.0 - 1.0, 1.0 - screen_uv.y * 2.0);
            let projection_inv = inverse(self.draw_pass.camera_projection);
            let near4 = projection_inv * vec4(ndc.x, ndc.y, -1.0, 1.0);
            let far4 = projection_inv * vec4(ndc.x, ndc.y, 1.0, 1.0);
            let near_w = mix(1.0, near4.w, step(0.00001, abs(near4.w)));
            let far_w = mix(1.0, far4.w, step(0.00001, abs(far4.w)));
            let near_eye = near4.xyz / near_w;
            let far_eye = far4.xyz / far_w;
            let ray_eye_raw = normalize(far_eye - near_eye);
            let ray_eye = ray_eye_raw * mix(1.0, -1.0, step(0.0, ray_eye_raw.z));
            let ray4 = self.draw_pass.camera_inv * vec4(ray_eye.x, ray_eye.y, ray_eye.z, 0.0);
            let ray = normalize(ray4.xyz);

            let depth = max(self.projection_depth_meters, 0.05);
            let surface_center =
                head_origin +
                forward * depth +
                up * self.projection_preview_offset_y_meters;
            let denom = dot(ray, forward);
            let safe_denom = mix(0.0001, denom, step(0.0001, abs(denom)));
            let t = dot(surface_center - eye_origin, forward) / safe_denom;
            let surface_point = eye_origin + ray * t;
            let half_height =
                tan(self.projection_preview_fov_y_degrees * 0.5 * 0.01745329251) *
                depth *
                max(self.projection_raw_overscan, 1.0);
            let half_width = half_height * max(self.display_aspect, 0.1);
            let delta = surface_point - surface_center;
            return vec2(
                0.5 + dot(delta, right) / max(half_width * 2.0, 0.0001),
                0.5 - dot(delta, up) / max(half_height * 2.0, 0.0001)
            );
        }

        uv_valid: fn(coord: vec2f) -> float {
            return
                step(0.0, coord.x) *
                step(coord.x, 1.0) *
                step(0.0, coord.y) *
                step(coord.y, 1.0);
        }

        rotate_uv: fn(coord: vec2f, rotation_steps: float) -> vec2f {
            let coord_90 = vec2(1.0 - coord.y, coord.x);
            let coord_180 = vec2(1.0 - coord.x, 1.0 - coord.y);
            let coord_270 = vec2(coord.y, 1.0 - coord.x);
            let is_90 = step(0.5, rotation_steps) * step(rotation_steps, 1.5);
            let is_180 = step(1.5, rotation_steps) * step(rotation_steps, 2.5);
            let is_270 = step(2.5, rotation_steps);
            let is_0 = 1.0 - is_90 - is_180 - is_270;
            return coord * is_0 + coord_90 * is_90 + coord_180 * is_180 + coord_270 * is_270;
        }

        yuv_to_rgb: fn(y_val: float, u_val: float, v_val: float) -> vec3f {
            let y = (y_val * 255.0 - 16.0) / 219.0;
            let u = (u_val * 255.0 - 128.0) / 224.0;
            let v = (v_val * 255.0 - 128.0) / 224.0;

            let r709 = y + 1.5748 * v;
            let g709 = y - 0.1873 * u - 0.4681 * v;
            let b709 = y + 1.8556 * u;

            let r601 = y + 1.402 * v;
            let g601 = y - 0.3441 * u - 0.7141 * v;
            let b601 = y + 1.772 * u;

            let r2020 = y + 1.4746 * v;
            let g2020 = y - 0.1646 * u - 0.5714 * v;
            let b2020 = y + 1.8814 * u;

            let is_601 = step(0.5, self.yuv_matrix) * step(self.yuv_matrix, 1.5);
            let is_2020 = step(1.5, self.yuv_matrix);
            let is_709 = 1.0 - is_601 - is_2020;

            return vec3(
                clamp(is_709 * r709 + is_601 * r601 + is_2020 * r2020, 0.0, 1.0),
                clamp(is_709 * g709 + is_601 * g601 + is_2020 * g2020, 0.0, 1.0),
                clamp(is_709 * b709 + is_601 * b601 + is_2020 * b2020, 0.0, 1.0)
            );
        }

        yuv_to_rgb_limited_601: fn(y_val: float, u_val: float, v_val: float) -> vec3f {
            let y = (y_val * 255.0 - 16.0) / 219.0;
            let u = (u_val * 255.0 - 128.0) / 224.0;
            let v = (v_val * 255.0 - 128.0) / 224.0;
            return vec3(
                clamp(y + 1.402 * v, 0.0, 1.0),
                clamp(y - 0.3441 * u - 0.7141 * v, 0.0, 1.0),
                clamp(y + 1.772 * u, 0.0, 1.0)
            );
        }

        yuv_to_rgb_limited_709: fn(y_val: float, u_val: float, v_val: float) -> vec3f {
            let y = (y_val * 255.0 - 16.0) / 219.0;
            let u = (u_val * 255.0 - 128.0) / 224.0;
            let v = (v_val * 255.0 - 128.0) / 224.0;
            return vec3(
                clamp(y + 1.5748 * v, 0.0, 1.0),
                clamp(y - 0.1873 * u - 0.4681 * v, 0.0, 1.0),
                clamp(y + 1.8556 * u, 0.0, 1.0)
            );
        }

        yuv_to_rgb_full_601: fn(y_val: float, u_val: float, v_val: float) -> vec3f {
            let y = y_val;
            let u = u_val - 0.5;
            let v = v_val - 0.5;
            return vec3(
                clamp(y + 1.402 * v, 0.0, 1.0),
                clamp(y - 0.3441 * u - 0.7141 * v, 0.0, 1.0),
                clamp(y + 1.772 * u, 0.0, 1.0)
            );
        }

        yuv_to_rgb_full_709: fn(y_val: float, u_val: float, v_val: float) -> vec3f {
            let y = y_val;
            let u = u_val - 0.5;
            let v = v_val - 0.5;
            return vec3(
                clamp(y + 1.5748 * v, 0.0, 1.0),
                clamp(y - 0.1873 * u - 0.4681 * v, 0.0, 1.0),
                clamp(y + 1.8556 * u, 0.0, 1.0)
            );
        }

        sample_left_yuv: fn(coord: vec2f) -> vec3f {
            let y_val = self.left_tex_y.sample(coord).x;
            let uv_sample = self.left_tex_u.sample(coord);
            let u_val = uv_sample.x;
            let v_val = mix(self.left_tex_v.sample(coord).x, uv_sample.y, step(0.5, self.yuv_biplanar));
            return self.yuv_to_rgb(y_val, u_val, v_val);
        }

        sample_right_yuv: fn(coord: vec2f) -> vec3f {
            let y_val = self.right_tex_y.sample(coord).x;
            let uv_sample = self.right_tex_u.sample(coord);
            let u_val = uv_sample.x;
            let v_val = mix(self.right_tex_v.sample(coord).x, uv_sample.y, step(0.5, self.yuv_biplanar));
            return self.yuv_to_rgb(y_val, u_val, v_val);
        }

        sample_camera_rgb: fn(coord: vec2f, eye_selector: float) -> vec3f {
            let sample_uv = clamp(coord, vec2(0.0, 0.0), vec2(1.0, 1.0));
            let left_yuv = self.sample_left_yuv(sample_uv);
            let right_yuv = self.sample_right_yuv(sample_uv);
            let yuv_rgb = mix(left_yuv, right_yuv, eye_selector);
            let left_external_rgb = self.left_camera_texture.sample_video(sample_uv).xyz;
            let right_external_rgb = self.right_camera_texture.sample_video(sample_uv).xyz;
            let external_rgb = mix(left_external_rgb, right_external_rgb, eye_selector);
            return mix(external_rgb, yuv_rgb, self.yuv_mode);
        }

        sample_camera_blur_rgb: fn(coord: vec2f, eye_selector: float) -> vec3f {
            let blur_source_texel = vec2(1.0 / 1280.0, 1.0 / 1280.0);
            let sample_step = blur_source_texel * clamp(self.blur_radius_px, 0.0, 16.0) * 4.0;
            let sample_uv = clamp(coord, vec2(0.0, 0.0), vec2(1.0, 1.0));
            let x0 = -2.0 * sample_step.x;
            let x1 = -1.0 * sample_step.x;
            let x2 = 0.0;
            let x3 = 1.0 * sample_step.x;
            let x4 = 2.0 * sample_step.x;
            let y0 = -2.0 * sample_step.y;
            let y1 = -1.0 * sample_step.y;
            let y2 = 0.0;
            let y3 = 1.0 * sample_step.y;
            let y4 = 2.0 * sample_step.y;
            let row0 =
                self.sample_camera_rgb(sample_uv + vec2(x0, y0), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x1, y0), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x2, y0), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x3, y0), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x4, y0), eye_selector);
            let row1 =
                self.sample_camera_rgb(sample_uv + vec2(x0, y1), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x1, y1), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x2, y1), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x3, y1), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x4, y1), eye_selector);
            let row2 =
                self.sample_camera_rgb(sample_uv + vec2(x0, y2), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x1, y2), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x2, y2), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x3, y2), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x4, y2), eye_selector);
            let row3 =
                self.sample_camera_rgb(sample_uv + vec2(x0, y3), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x1, y3), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x2, y3), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x3, y3), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x4, y3), eye_selector);
            let row4 =
                self.sample_camera_rgb(sample_uv + vec2(x0, y4), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x1, y4), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x2, y4), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x3, y4), eye_selector) +
                self.sample_camera_rgb(sample_uv + vec2(x4, y4), eye_selector);
            let color = (row0 + row1 + row2 + row3 + row4) / 25.0;
            return vec3(
                clamp(color.x, 0.0, 1.0),
                clamp(color.y, 0.0, 1.0),
                clamp(color.z, 0.0, 1.0)
            );
        }

        sample_processed_camera_rgb: fn(coord: vec2f, eye_selector: float) -> vec3f {
            let raw_rgb = self.sample_camera_rgb(coord, eye_selector);
            let blur_rgb = self.sample_camera_blur_rgb(coord, eye_selector);
            return mix(raw_rgb, blur_rgb, step(0.5, self.processing_layer));
        }

        projection_alpha_transform: fn(mask: float) -> float {
            return clamp(
                mask * max(self.projection_alpha_scale, 0.0) + self.projection_alpha_bias,
                0.0,
                1.0
            );
        }

        projection_color_alpha: fn(rgb: vec3f) -> float {
            let color = clamp(rgb, vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0));
            let luma = color.x * 0.2126 + color.y * 0.7152 + color.z * 0.0722;
            let max_channel = max(max(color.x, color.y), color.z);
            let min_channel = min(min(color.x, color.y), color.z);
            let saturation = max_channel - min_channel;
            let mode = self.projection_alpha_mode;
            if mode > 0.5 && mode < 1.5 {
                return self.projection_alpha_transform(color.x);
            }
            if mode > 1.5 && mode < 2.5 {
                return self.projection_alpha_transform(color.y);
            }
            if mode > 2.5 && mode < 3.5 {
                return self.projection_alpha_transform(color.z);
            }
            if mode > 3.5 && mode < 4.5 {
                return self.projection_alpha_transform(luma);
            }
            if mode > 4.5 && mode < 5.5 {
                return self.projection_alpha_transform(1.0 - color.x);
            }
            if mode > 5.5 && mode < 6.5 {
                return self.projection_alpha_transform(1.0 - color.y);
            }
            if mode > 6.5 && mode < 7.5 {
                return self.projection_alpha_transform(1.0 - color.z);
            }
            if mode > 7.5 && mode < 8.5 {
                return self.projection_alpha_transform(1.0 - luma);
            }
            if mode > 8.5 && mode < 9.5 {
                return self.projection_alpha_transform(max(color.x - max(color.y, color.z), 0.0));
            }
            if mode > 9.5 && mode < 10.5 {
                return self.projection_alpha_transform(max(color.y - max(color.x, color.z), 0.0));
            }
            if mode > 10.5 && mode < 11.5 {
                return self.projection_alpha_transform(max(color.z - max(color.x, color.y), 0.0));
            }
            if mode > 11.5 && mode < 12.5 {
                return self.projection_alpha_transform(saturation);
            }
            if mode > 12.5 && mode < 13.5 {
                return self.projection_alpha_transform(1.0 - saturation);
            }
            return self.projection_alpha_transform(1.0);
        }

        source_sample_uv: fn(coord: vec2f) -> vec2f {
            let sample_uv = clamp(coord, vec2(0.0, 0.0), vec2(1.0, 1.0));
            let flip_y = step(0.5, self.source_sample_y_flip);
            return vec2(sample_uv.x, mix(sample_uv.y, 1.0 - sample_uv.y, flip_y));
        }

        guide_mask: fn(coord: vec2f) -> float {
            let edge_x = min(coord.x, 1.0 - coord.x);
            let edge_y = min(coord.y, 1.0 - coord.y);
            let border = 1.0 - step(0.015, min(edge_x, edge_y));
            return clamp(border, 0.0, 1.0);
        }

        projection_border_mask: fn(coord: vec2f) -> float {
            let inside = self.uv_valid(coord);
            let edge_x = min(coord.x, 1.0 - coord.x);
            let edge_y = min(coord.y, 1.0 - coord.y);
            let border = 1.0 - step(0.025, min(edge_x, edge_y));
            return clamp(border * inside, 0.0, 1.0);
        }

        projection_area_mask: fn(area_uv: vec2f) -> float {
            let half_size = max(
                vec2(self.projection_area_radius_x_uv, self.projection_area_radius_y_uv),
                vec2(0.05, 0.05)
            );
            let corner_radius = clamp(
                self.projection_area_corner_radius_uv,
                0.0,
                min(half_size.x, half_size.y) - 0.001
            );
            let q = abs(area_uv - vec2(0.5, 0.5)) - (half_size - vec2(corner_radius, corner_radius));
            let outside = length(max(q, vec2(0.0, 0.0)));
            let inside = min(max(q.x, q.y), 0.0);
            let signed_distance = outside + inside - corner_radius;
            return 1.0 - step(0.0001, signed_distance);
        }

        projection_area_edge_mask: fn(area_uv: vec2f) -> float {
            let half_size = max(
                vec2(self.projection_area_radius_x_uv, self.projection_area_radius_y_uv),
                vec2(0.05, 0.05)
            );
            let corner_radius = clamp(
                self.projection_area_corner_radius_uv,
                0.0,
                min(half_size.x, half_size.y) - 0.001
            );
            let q = abs(area_uv - vec2(0.5, 0.5)) - (half_size - vec2(corner_radius, corner_radius));
            let outside = length(max(q, vec2(0.0, 0.0)));
            let inside = min(max(q.x, q.y), 0.0);
            let signed_distance = outside + inside - corner_radius;
            return 1.0 - step(0.012, abs(signed_distance));
        }

        projection_area_content_uv: fn(area_uv: vec2f) -> vec2f {
            let half_size = max(
                vec2(self.projection_area_radius_x_uv, self.projection_area_radius_y_uv),
                vec2(0.05, 0.05)
            );
            return (area_uv - (vec2(0.5, 0.5) - half_size)) /
                max(half_size * 2.0, vec2(0.001, 0.001));
        }

        diagnostic_domain_edge_mask: fn(coord: vec2f, width: float, pad: float) -> float {
            let near_domain =
                step(-pad, coord.x) *
                step(coord.x, 1.0 + pad) *
                step(-pad, coord.y) *
                step(coord.y, 1.0 + pad);
            let edge_x = min(abs(coord.x), abs(coord.x - 1.0));
            let edge_y = min(abs(coord.y), abs(coord.y - 1.0));
            return (1.0 - step(width, min(edge_x, edge_y))) * near_domain;
        }

        diagnostic_axis_mask: fn(coord: vec2f, axis: float, width: float) -> float {
            return max(
                1.0 - step(width, abs(coord.x - axis)),
                1.0 - step(width, abs(coord.y - axis))
            );
        }

        projection_area_diagnostic_color: fn(
            surface_uv: vec2f,
            camera_uv: vec2f,
            display_eye_selector: float,
            projection_valid: float
        ) -> vec3f {
            let diagnostic_uv = clamp(camera_uv, vec2(0.0, 0.0), vec2(1.0, 1.0));
            let border = self.diagnostic_domain_edge_mask(camera_uv, 0.018, 0.060);
            let surface_guide_strength =
                1.0 - step(1.5, self.projection_area_diagnostic);
            let surface_border =
                self.diagnostic_domain_edge_mask(surface_uv, 0.010, 0.035) *
                projection_valid *
                surface_guide_strength;
            let major_axes = self.diagnostic_axis_mask(diagnostic_uv, 0.5, 0.010);
            let quarter_axes = max(
                self.diagnostic_axis_mask(diagnostic_uv, 0.25, 0.006),
                self.diagnostic_axis_mask(diagnostic_uv, 0.75, 0.006)
            );
            let diagonal = clamp(
                (1.0 - step(0.010, abs(diagnostic_uv.x - diagnostic_uv.y))) +
                (1.0 - step(0.010, abs((diagnostic_uv.x + diagnostic_uv.y) - 1.0))),
                0.0,
                1.0
            );
            let left_color = vec3(0.08, 0.30, 0.98);
            let right_color = vec3(0.98, 0.06, 0.48);
            let base = mix(left_color, right_color, display_eye_selector);
            let ramp = vec3(
                0.18 + diagnostic_uv.x * 0.62,
                0.12 + diagnostic_uv.y * 0.76,
                0.90 - diagnostic_uv.x * 0.22
            );
            let with_grid = mix(base, ramp, 0.42);
            let with_major = mix(with_grid, vec3(1.0, 1.0, 1.0), clamp(major_axes * 0.82, 0.0, 1.0));
            let with_quarters = mix(with_major, vec3(0.05, 1.0, 0.72), clamp(quarter_axes * 0.52, 0.0, 1.0));
            let with_diagonal = mix(with_quarters, vec3(1.0, 0.86, 0.04), clamp(diagonal * 0.44, 0.0, 1.0));
            let inside = mix(vec3(0.0, 0.0, 0.0), with_diagonal, projection_valid);
            let with_border = mix(inside, vec3(1.0, 0.0, 1.0), clamp(border, 0.0, 1.0));
            return mix(with_border, vec3(1.0, 1.0, 1.0), clamp(surface_border * 0.70, 0.0, 1.0));
        }

        pixel: fn() {
            let renderer_surface_uv = clamp(self.v_uv, vec2(0.0, 0.0), vec2(1.0, 1.0));
            let full_view_uv = vec2(renderer_surface_uv.x, 1.0 - renderer_surface_uv.y);
            let proof_guide = 0.0;
            let eye_selector = self.source_eye_selector();
            let display_eye_selector = self.active_eye_is_right();
            let projection_screen_uv =
                self.projection_area_screen_uv(full_view_uv, display_eye_selector);
            let projected_uv = self.source_screen_camera_uv(
                projection_screen_uv,
                display_eye_selector
            );
            let full_frame_projection_area_mapping =
                step(0.5, self.projection_content_mapping_mode);
            let projection_area_content_uv =
                self.projection_area_content_uv(projection_screen_uv);
            let mapped_source_uv =
                mix(projected_uv, projection_area_content_uv, full_frame_projection_area_mapping);
            let projection_area_mask = self.projection_area_mask(projection_screen_uv);
            let projection_valid = self.uv_valid(mapped_source_uv) * projection_area_mask;
            let surface_uv = mix(
                self.screen_surface_uv(projection_screen_uv, display_eye_selector),
                projection_area_content_uv,
                full_frame_projection_area_mapping
            );
            let fallback_seed_uv =
                self.clamp_border_seed_uv(clamp(surface_uv, vec2(0.0, 0.0), vec2(1.0, 1.0)));
            let projected_sample_uv = self.source_sample_uv(mapped_source_uv);
            let fallback_sample_uv = self.source_sample_uv(fallback_seed_uv);
            let sample_uv = mix(fallback_sample_uv, projected_sample_uv, projection_valid);
            let full_surface_sample_uv = self.source_sample_uv(full_view_uv);
            let live_sample_uv = mix(sample_uv, full_surface_sample_uv, self.force_full_surface_live_camera_uv);
            let live_projection_valid = mix(projection_valid, 1.0, self.force_full_surface_live_camera_uv);
            if self.camera_ready <= 0.5 {
                let waiting = vec3(0.015, 0.020, 0.024);
                let guided_waiting = mix(waiting, vec3(1.0, 0.98, 0.84), proof_guide);
                return vec4(guided_waiting.x, guided_waiting.y, guided_waiting.z, 1.0);
            }
            if self.suppress_live_camera_sampling > 0.5 {
                let armed = vec3(0.015, 0.18, 0.08);
                let guided_armed = mix(armed, vec3(1.0, 0.98, 0.84), proof_guide);
                return vec4(guided_armed.x, guided_armed.y, guided_armed.z, 1.0);
            }
            if self.projection_area_diagnostic > 0.5 {
                let diagnostic_rgb = self.projection_area_diagnostic_color(
                    surface_uv,
                    mapped_source_uv,
                    display_eye_selector,
                    projection_valid
                );
                let guided_diagnostic = mix(diagnostic_rgb, vec3(1.0, 0.98, 0.84), proof_guide);
                return vec4(guided_diagnostic.x, guided_diagnostic.y, guided_diagnostic.z, 1.0);
            }
            if self.force_in_surface_camera_window > 0.5 {
                let camera_window_uv = clamp(mapped_source_uv, vec2(0.0, 0.0), vec2(1.0, 1.0));
                let window_sample_uv = self.source_sample_uv(camera_window_uv);
                let camera_rgb = self.sample_processed_camera_rgb(window_sample_uv, eye_selector);
                let passthrough_border_policy =
                    step(0.5, self.projection_border_policy);
                let projection_area_opacity = clamp(self.projection_area_opacity, 0.0, 1.0);
                let projection_border_opacity = clamp(self.projection_border_opacity, 0.0, 1.0);
                let source_uv_valid = self.uv_valid(mapped_source_uv);
                let diagnostic_fill_rgb = vec3(1.0, 0.0, 0.0);
                let matte = mix(diagnostic_fill_rgb, vec3(0.0, 0.0, 0.0), passthrough_border_policy);
                let camera_window_valid = source_uv_valid * projection_area_mask;
                let window_rgb = mix(matte, camera_rgb, camera_window_valid);
                let guided_window = mix(window_rgb, vec3(1.0, 0.98, 0.84), proof_guide);
                let border_alpha = projection_border_opacity * (1.0 - passthrough_border_policy);
                let area_alpha = projection_area_opacity * self.projection_color_alpha(camera_rgb);
                let alpha = mix(border_alpha, area_alpha, camera_window_valid);
                let premultiplied_window = guided_window * alpha;
                return vec4(
                    premultiplied_window.x,
                    premultiplied_window.y,
                    premultiplied_window.z,
                    alpha
                );
            }
            let direct_rgb =
                self.sample_processed_camera_rgb(live_sample_uv, eye_selector) * mix(0.12, 1.0, live_projection_valid);
            let guided_direct = mix(direct_rgb, vec3(1.0, 0.98, 0.84), proof_guide);
            return vec4(guided_direct.x, guided_direct.y, guided_direct.z, 1.0);

            let half_selector = step(0.5, panel_uv.x);
            let source_uv = vec2(mix(panel_uv.x * 2.0, (panel_uv.x - 0.5) * 2.0, half_selector), panel_uv.y);
            let left_uv = clamp(self.rotate_uv(source_uv, self.left_rotation_steps), vec2(0.0, 0.0), vec2(1.0, 1.0));
            let right_uv = clamp(self.rotate_uv(source_uv, self.right_rotation_steps), vec2(0.0, 0.0), vec2(1.0, 1.0));
            let left_proof_tint = vec3(0.02, 0.42, 1.0);
            let right_proof_tint = vec3(1.0, 0.06, 0.06);
            let proof_tint = mix(left_proof_tint, right_proof_tint, half_selector);

            if self.camera_ready <= 0.5 {
                let waiting = vec3(0.015, 0.020, 0.024);
                let guided_waiting = mix(waiting, vec3(1.0, 1.0, 1.0), proof_guide);
                return vec4(guided_waiting.x, guided_waiting.y, guided_waiting.z, 1.0);
            }

            let guide = proof_guide * self.alignment_guide;
            if self.diagnostic_solid > 0.5 {
                let left_color = vec3(0.02 + 0.28 * source_uv.x, 0.72 + 0.20 * source_uv.y, 0.95);
                let right_color = vec3(0.95, 0.10 + 0.24 * source_uv.y, 0.70 + 0.18 * source_uv.x);
                let diagnostic = mix(left_color, right_color, half_selector);
                let guided_diagnostic = mix(diagnostic, vec3(1.0, 1.0, 1.0), guide);
                return vec4(guided_diagnostic.x, guided_diagnostic.y, guided_diagnostic.z, 1.0);
            }

            let left_external_sample = self.left_camera_texture.sample_video(left_uv).xyz;
            let right_external_sample = self.right_camera_texture.sample_video(right_uv).xyz;
            let left_luma_sample = self.left_tex_y.sample(left_uv).x;
            let right_luma_sample = self.right_tex_y.sample(right_uv).x;
            let left_yuv_sample = self.sample_left_yuv(left_uv);
            let right_yuv_sample = self.sample_right_yuv(right_uv);
            if self.texture_probe_mode > 1.5 {
                let bypass_rgb = vec3(0.42, 0.42, 0.42);
                let guided_bypass = mix(bypass_rgb, vec3(0.98, 0.98, 0.92), guide);
                return vec4(guided_bypass.x, guided_bypass.y, guided_bypass.z, 1.0);
            }
            if self.texture_probe_mode > 0.5 {
                let luma = mix(left_luma_sample, right_luma_sample, half_selector);
                let luma_visual = clamp(luma * 8.0, 0.0, 1.0);
                let luma_rgb = vec3(luma_visual, luma_visual, luma_visual);
                let guided_luma = mix(luma_rgb, vec3(0.98, 0.98, 0.92), guide);
                return vec4(guided_luma.x, guided_luma.y, guided_luma.z, 1.0);
            }
            let left_sample = mix(left_external_sample, left_yuv_sample, self.yuv_mode);
            let right_sample = mix(right_external_sample, right_yuv_sample, self.yuv_mode);
            let sample = mix(left_sample, right_sample, half_selector);
            let rgb = vec3(
                clamp(sample.x * self.exposure, 0.0, 1.0),
                clamp(sample.y * self.exposure, 0.0, 1.0),
                clamp(sample.z * self.exposure, 0.0, 1.0)
            );
            let tinted = mix(rgb, proof_tint, self.proof_tint_strength);
            let guided = mix(tinted, vec3(0.98, 0.98, 0.92), guide);
            return vec4(guided.x, guided.y, guided.z, 1.0);
        }

        fragment: fn() {
            self.fb0 = depth_clip(self.world, self.pixel(), self.depth_clip);
        }
    }

    mod.widgets.MakepadStereoCameraPanelBase = #(MakepadStereoCameraPanel::register_widget(vm))
    mod.widgets.MakepadStereoCameraPanel = set_type_default() do mod.widgets.MakepadStereoCameraPanelBase{
        body: mod.widgets.XrBodyKind.Fixed
        shared_object_policy: mod.widgets.XrSharedObjectPolicy.None
        size: vec3(0.92, 0.92, 0.010)
        draw_panel +: {
            exposure: 1.06
            camera_ready: 1.0
            diagnostic_solid: 0.0
            alignment_guide: 1.0
            yuv_mode: 1.0
            yuv_matrix: 1.0
            yuv_biplanar: 0.0
            texture_probe_mode: 2.0
            proof_tint_strength: 0.0
            depth_clip: 0.0
        }
    }

    startup() do #(App::script_component(vm)){
        ui: XrRoot{
            window.inner_size: vec2(760, 480)
            pass.clear_color: #x203040
            camera.fov_y: 36.0
            camera.desktop_target: vec3(0.0, -0.05, -0.72)
            camera.distance: 1.65
            env.gravity: 0.0
            env.env_cube: false
            env.depth_mesh: false

            camera_projection_scene := XrNode{
                pos: vec3(0.0, 0.0, 0.0)

                camera_projection_panel := mod.widgets.MakepadStereoCameraPanel{
                    body: mod.widgets.XrBodyKind.Fixed
                    size: vec3(1.0, 1.0, 0.010)
                    pos: vec3(0.0, 0.0, -1.0)
                }
            }

            camera_video_view := XrView{
                visible: false
                pos: vec3(0.0, -0.04, -0.764)
                logical_size: vec2(960, 540)
                pixel_scale: 0.00096
                dpi_factor: 1.0

                SolidView{
                    width: Fill
                    height: Fill
                    flow: Right
                    spacing: 0
                    draw_bg.color: #x05090dff

                    left_camera_video := Video{
                        width: 480
                        height: Fill
                        autoplay: false
                        show_controls: false
                    }

                    right_camera_video := Video{
                        width: 480
                        height: Fill
                        autoplay: false
                        show_controls: false
                    }
                }
            }

            xr_permissions := XrPermissionsFlow{}
        }
    }
}

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    paired_import_timer: Timer,
    #[rust]
    paired_import_wait_count: usize,
    #[rust]
    paired_import_choice: Option<MakepadCameraPair>,
    #[rust]
    paired_import_selection_logged: bool,
    #[rust]
    paired_import_started: bool,
    #[rust]
    broker_h264_left_playback_requested: bool,
    #[rust]
    broker_h264_right_playback_requested: bool,
    #[rust]
    broker_h264_left_projection_metadata: Option<BrokerH264ProjectionMetadata>,
    #[rust]
    broker_h264_right_projection_metadata: Option<BrokerH264ProjectionMetadata>,
    #[rust]
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    broker_h264_projection_plan_logged: bool,
    #[rust]
    native_video_widget_started: bool,
    #[rust]
    native_video_widget_retry_timer: Timer,
    #[rust]
    native_video_widget_retry_pair: Option<MakepadCameraPair>,
    #[rust]
    native_video_widget_retry_count: usize,
    #[rust]
    paired_import_finished: bool,
    #[rust]
    paired_import_left_texture: Option<Texture>,
    #[rust]
    paired_import_right_texture: Option<Texture>,
    #[rust]
    paired_import_left_yuv_textures: Option<MakepadCameraYuvTextures>,
    #[rust]
    paired_import_right_yuv_textures: Option<MakepadCameraYuvTextures>,
    #[rust]
    paired_import_left_prepared: bool,
    #[rust]
    paired_import_right_prepared: bool,
    #[rust]
    paired_import_left_updated: bool,
    #[rust]
    paired_import_right_updated: bool,
    #[rust]
    paired_import_left_rotation_steps: f32,
    #[rust]
    paired_import_right_rotation_steps: f32,
    #[rust]
    camera_projection_textures_bound: bool,
    #[rust]
    camera_projection_paired_textures_bound: bool,
    #[rust]
    camera_projection_single_stream_logged: bool,
    #[rust]
    camera_projection_bind_error_logged: bool,
    #[rust]
    synthetic_scene_hidden_for_camera: bool,
    #[rust]
    horizontal_alignment_tuning_ready: bool,
    #[rust]
    horizontal_alignment_strength: f32,
    #[rust]
    manual_horizontal_offset_left_uv: f32,
    #[rust]
    manual_horizontal_offset_right_uv: f32,
    #[rust]
    manual_vertical_offset_uv: f32,
    #[rust]
    content_uv_scale: f32,
    #[rust]
    projection_border_opacity: f32,
    #[rust]
    projection_border_policy: f32,
    #[rust]
    processing_layer: f32,
    #[rust]
    blur_radius_px: f32,
    #[rust]
    projection_area_diagnostic: f32,
    #[rust]
    projection_area_offset_left_uv: f32,
    #[rust]
    projection_area_offset_right_uv: f32,
    #[rust]
    projection_area_offset_vertical_uv: f32,
    #[rust]
    projection_area_scale_x: f32,
    #[rust]
    projection_area_scale_y: f32,
    #[rust]
    projection_area_radius_x_uv: f32,
    #[rust]
    projection_area_radius_y_uv: f32,
    #[rust]
    projection_area_corner_radius_uv: f32,
    #[rust]
    projection_area_keystone_x: f32,
    #[rust]
    projection_area_bow_x: f32,
    #[rust]
    projection_area_opacity: f32,
    #[rust]
    projection_alpha_mode: f32,
    #[rust]
    projection_alpha_scale: f32,
    #[rust]
    projection_alpha_bias: f32,
    #[rust]
    #[allow(dead_code)]
    projection_content_mapping_mode: f32,
    #[rust]
    cadence_next_frame: Option<NextFrame>,
    #[rust]
    cadence_started: bool,
    #[rust]
    cadence_start_time: f64,
    #[rust]
    cadence_last_sample_time: f64,
    #[rust]
    cadence_frame_count: u64,
    #[rust]
    cadence_frame_count_at_last_sample: u64,
    #[rust]
    cadence_xr_update_count: u64,
    #[rust]
    cadence_xr_update_count_at_last_sample: u64,
    #[rust]
    cadence_draw_event_count: u64,
    #[rust]
    cadence_draw_event_count_at_last_sample: u64,
    #[rust]
    cadence_left_texture_update_count: u64,
    #[rust]
    cadence_right_texture_update_count: u64,
    #[rust]
    cadence_left_texture_update_count_at_last_sample: u64,
    #[rust]
    cadence_right_texture_update_count_at_last_sample: u64,
    #[rust]
    cadence_left_last_position_ms: u128,
    #[rust]
    cadence_right_last_position_ms: u128,
}

#[derive(Script, ScriptHook, Debug)]
#[repr(C)]
pub struct DrawMakepadStereoCameraPanel {
    #[deref]
    pub draw_vars: DrawVars,
    #[live(vec3(1.0, 1.0, 1.0))]
    pub cube_size: Vec3f,
    #[live(vec3(0.0, 0.0, 0.0))]
    pub cube_pos: Vec3f,
    #[live(0.0_f32)]
    pub depth_clip: f32,
    #[live(1.0_f32)]
    pub camera_ready: f32,
    #[live(0.0_f32)]
    pub left_rotation_steps: f32,
    #[live(0.0_f32)]
    pub right_rotation_steps: f32,
    #[live(1.0_f32)]
    pub exposure: f32,
    #[live(0.0_f32)]
    pub diagnostic_solid: f32,
    #[live(0.0_f32)]
    pub alignment_guide: f32,
    #[live(1.0_f32)]
    pub yuv_mode: f32,
    #[live(1.0_f32)]
    pub yuv_matrix: f32,
    #[live(0.0_f32)]
    pub yuv_biplanar: f32,
    #[live(2.0_f32)]
    pub texture_probe_mode: f32,
    #[live(0.0_f32)]
    pub proof_tint_strength: f32,
    #[live(1.9811321_f32)]
    pub content_uv_scale: f32,
    #[live(0.032_f32)]
    pub display_eye_offset_meters: f32,
    #[live(92.0_f32)]
    pub display_fov_y_degrees: f32,
    #[live(1.0_f32)]
    pub display_aspect: f32,
    #[live(0.75_f32)]
    pub projection_depth_meters: f32,
    #[live(0.0_f32)]
    pub projection_preview_offset_y_meters: f32,
    #[live(60.0_f32)]
    pub projection_preview_fov_y_degrees: f32,
    #[live(1.06_f32)]
    pub projection_raw_overscan: f32,
    #[live(1.0_f32)]
    pub suppress_live_camera_sampling: f32,
    #[live(1.0_f32)]
    pub force_full_surface_live_camera_uv: f32,
    #[live(1.0_f32)]
    pub force_in_surface_camera_window: f32,
    #[live(1.0_f32)]
    pub projection_border_opacity: f32,
    #[live(0.0_f32)]
    pub projection_border_policy: f32,
    #[live(0.0_f32)]
    pub processing_layer: f32,
    #[live(2.0_f32)]
    pub blur_radius_px: f32,
    #[live(0.0_f32)]
    pub projection_area_diagnostic: f32,
    #[live(0.0_f32)]
    pub projection_area_offset_left_uv: f32,
    #[live(0.0_f32)]
    pub projection_area_offset_right_uv: f32,
    #[live(0.0_f32)]
    pub projection_area_offset_vertical_uv: f32,
    #[live(1.0_f32)]
    pub projection_area_scale_x: f32,
    #[live(1.0_f32)]
    pub projection_area_scale_y: f32,
    #[live(0.5_f32)]
    pub projection_area_radius_x_uv: f32,
    #[live(0.5_f32)]
    pub projection_area_radius_y_uv: f32,
    #[live(0.0_f32)]
    pub projection_area_corner_radius_uv: f32,
    #[live(0.0_f32)]
    pub projection_area_keystone_x: f32,
    #[live(0.0_f32)]
    pub projection_area_bow_x: f32,
    #[live(1.0_f32)]
    pub projection_area_opacity: f32,
    #[live(0.0_f32)]
    pub projection_alpha_mode: f32,
    #[live(1.0_f32)]
    pub projection_alpha_scale: f32,
    #[live(0.0_f32)]
    pub projection_alpha_bias: f32,
    #[live(1.0_f32)]
    pub source_sample_y_flip: f32,
    #[live(0.0_f32)]
    pub projection_content_mapping_mode: f32,
    #[live(1.0_f32)]
    pub display_source_eye_swap: f32,
    #[live(1.0_f32)]
    pub horizontal_alignment_strength: f32,
    #[live(0.0_f32)]
    pub manual_horizontal_offset_left_uv: f32,
    #[live(0.0_f32)]
    pub manual_horizontal_offset_right_uv: f32,
    #[live(0.0_f32)]
    pub manual_vertical_offset_uv: f32,
    #[live(1.0_f32)]
    pub left_projection_h00: f32,
    #[live(0.0_f32)]
    pub left_projection_h01: f32,
    #[live(0.0_f32)]
    pub left_projection_h02: f32,
    #[live(0.0_f32)]
    pub left_projection_h10: f32,
    #[live(1.0_f32)]
    pub left_projection_h11: f32,
    #[live(0.0_f32)]
    pub left_projection_h12: f32,
    #[live(0.0_f32)]
    pub left_projection_h20: f32,
    #[live(0.0_f32)]
    pub left_projection_h21: f32,
    #[live(1.0_f32)]
    pub left_projection_h22: f32,
    #[live(1.0_f32)]
    pub right_projection_h00: f32,
    #[live(0.0_f32)]
    pub right_projection_h01: f32,
    #[live(0.0_f32)]
    pub right_projection_h02: f32,
    #[live(0.0_f32)]
    pub right_projection_h10: f32,
    #[live(1.0_f32)]
    pub right_projection_h11: f32,
    #[live(0.0_f32)]
    pub right_projection_h12: f32,
    #[live(0.0_f32)]
    pub right_projection_h20: f32,
    #[live(0.0_f32)]
    pub right_projection_h21: f32,
    #[live(1.0_f32)]
    pub right_projection_h22: f32,
    #[live(1.0_f32)]
    pub left_screen_to_camera_h00: f32,
    #[live(0.0_f32)]
    pub left_screen_to_camera_h01: f32,
    #[live(0.0_f32)]
    pub left_screen_to_camera_h02: f32,
    #[live(0.0_f32)]
    pub left_screen_to_camera_h10: f32,
    #[live(1.0_f32)]
    pub left_screen_to_camera_h11: f32,
    #[live(0.0_f32)]
    pub left_screen_to_camera_h12: f32,
    #[live(0.0_f32)]
    pub left_screen_to_camera_h20: f32,
    #[live(0.0_f32)]
    pub left_screen_to_camera_h21: f32,
    #[live(1.0_f32)]
    pub left_screen_to_camera_h22: f32,
    #[live(1.0_f32)]
    pub right_screen_to_camera_h00: f32,
    #[live(0.0_f32)]
    pub right_screen_to_camera_h01: f32,
    #[live(0.0_f32)]
    pub right_screen_to_camera_h02: f32,
    #[live(0.0_f32)]
    pub right_screen_to_camera_h10: f32,
    #[live(1.0_f32)]
    pub right_screen_to_camera_h11: f32,
    #[live(0.0_f32)]
    pub right_screen_to_camera_h12: f32,
    #[live(0.0_f32)]
    pub right_screen_to_camera_h20: f32,
    #[live(0.0_f32)]
    pub right_screen_to_camera_h21: f32,
    #[live(1.0_f32)]
    pub right_screen_to_camera_h22: f32,
    #[live(1.0_f32)]
    pub left_screen_to_surface_h00: f32,
    #[live(0.0_f32)]
    pub left_screen_to_surface_h01: f32,
    #[live(0.0_f32)]
    pub left_screen_to_surface_h02: f32,
    #[live(0.0_f32)]
    pub left_screen_to_surface_h10: f32,
    #[live(1.0_f32)]
    pub left_screen_to_surface_h11: f32,
    #[live(0.0_f32)]
    pub left_screen_to_surface_h12: f32,
    #[live(0.0_f32)]
    pub left_screen_to_surface_h20: f32,
    #[live(0.0_f32)]
    pub left_screen_to_surface_h21: f32,
    #[live(1.0_f32)]
    pub left_screen_to_surface_h22: f32,
    #[live(1.0_f32)]
    pub right_screen_to_surface_h00: f32,
    #[live(0.0_f32)]
    pub right_screen_to_surface_h01: f32,
    #[live(0.0_f32)]
    pub right_screen_to_surface_h02: f32,
    #[live(0.0_f32)]
    pub right_screen_to_surface_h10: f32,
    #[live(1.0_f32)]
    pub right_screen_to_surface_h11: f32,
    #[live(0.0_f32)]
    pub right_screen_to_surface_h12: f32,
    #[live(0.0_f32)]
    pub right_screen_to_surface_h20: f32,
    #[live(0.0_f32)]
    pub right_screen_to_surface_h21: f32,
    #[live(1.0_f32)]
    pub right_screen_to_surface_h22: f32,
}

impl DrawMakepadStereoCameraPanel {
    fn assign_texture_slot(&mut self, slot: usize, texture: Option<Texture>) {
        match texture {
            Some(texture) => self.draw_vars.set_texture(slot, &texture),
            None => self.draw_vars.empty_texture(slot),
        }
    }

    fn set_camera_textures(&mut self, cx: &mut Cx, left: Option<Texture>, right: Option<Texture>) {
        self.assign_texture_slot(0, left);
        self.assign_texture_slot(1, right);
        self.draw_vars.redraw(cx);
    }

    fn set_camera_yuv_textures(
        &mut self,
        cx: &mut Cx,
        left: Option<MakepadCameraYuvTextures>,
        right: Option<MakepadCameraYuvTextures>,
    ) {
        let left_effective = left.clone().or_else(|| right.clone());
        let right_effective = right.clone().or_else(|| left_effective.clone());
        self.assign_texture_slot(
            2,
            left_effective.as_ref().map(|textures| textures.y.clone()),
        );
        self.assign_texture_slot(
            3,
            left_effective.as_ref().map(|textures| textures.u.clone()),
        );
        self.assign_texture_slot(
            4,
            left_effective.as_ref().map(|textures| textures.v.clone()),
        );
        self.assign_texture_slot(
            5,
            right_effective.as_ref().map(|textures| textures.y.clone()),
        );
        self.assign_texture_slot(
            6,
            right_effective.as_ref().map(|textures| textures.u.clone()),
        );
        self.assign_texture_slot(
            7,
            right_effective.as_ref().map(|textures| textures.v.clone()),
        );
        self.yuv_mode = if left_effective.is_some() && right_effective.is_some() {
            1.0
        } else {
            0.0
        };
        self.draw_vars.redraw(cx);
    }

    fn draw(&mut self, cx: &mut CxDraw) {
        if self.draw_vars.can_instance() {
            let new_area = cx.add_instance(&self.draw_vars);
            self.draw_vars.area = cx.update_area_refs(self.draw_vars.area, new_area);
        }
    }
}

#[derive(Script, Widget)]
pub struct MakepadStereoCameraPanel {
    #[redraw]
    #[live]
    draw_panel: DrawMakepadStereoCameraPanel,
    #[live(vec3(0.92, 0.52, 0.010))]
    size: Vec3f,
    #[rust(false)]
    camera_ready: bool,
    #[cast]
    #[deref]
    node: XrNode,
    #[rust]
    synthetic_luma_probe_texture: Option<Texture>,
}

#[derive(Clone, Copy, Debug)]
struct HorizontalAlignmentTuning {
    strength: f32,
    left_offset_uv: f32,
    right_offset_uv: f32,
    vertical_offset_uv: f32,
    content_uv_scale: f32,
    projection_border_opacity: f32,
    projection_border_policy: f32,
    processing_layer: f32,
    blur_radius_px: f32,
    projection_area_diagnostic: f32,
    projection_area_offset_left_uv: f32,
    projection_area_offset_right_uv: f32,
    projection_area_offset_vertical_uv: f32,
    projection_area_scale_x: f32,
    projection_area_scale_y: f32,
    projection_area_radius_x_uv: f32,
    projection_area_radius_y_uv: f32,
    projection_area_corner_radius_uv: f32,
    projection_area_keystone_x: f32,
    projection_area_bow_x: f32,
    projection_area_opacity: f32,
    projection_alpha_mode: f32,
    projection_alpha_scale: f32,
    projection_alpha_bias: f32,
}

#[derive(Clone, Copy, Debug)]
struct ProjectionPanelGeometry {
    width_meters: f32,
    height_meters: f32,
    depth_meters: f32,
    offset_y_meters: f32,
    z_meters: f32,
}

impl ProjectionPanelGeometry {
    fn size(self) -> Vec3f {
        vec3f(self.width_meters, self.height_meters, 0.010)
    }

    fn pos(self) -> Vec3f {
        vec3f(0.0, self.offset_y_meters, self.z_meters)
    }
}

impl Default for HorizontalAlignmentTuning {
    fn default() -> Self {
        Self {
            strength: TARGET_HORIZONTAL_ALIGNMENT_STRENGTH,
            left_offset_uv: TARGET_MANUAL_HORIZONTAL_OFFSET_LEFT_UV,
            right_offset_uv: TARGET_MANUAL_HORIZONTAL_OFFSET_RIGHT_UV,
            vertical_offset_uv: TARGET_MANUAL_VERTICAL_OFFSET_UV,
            content_uv_scale: TARGET_FULL_VIEW_CONTENT_UV_SCALE,
            projection_border_opacity: TARGET_PROJECTION_BORDER_OPACITY,
            projection_border_policy: MakepadProjectionBorderPolicy::current().shader_code(),
            processing_layer: MakepadProcessingLayer::current().shader_code(),
            blur_radius_px: makepad_blur_radius_px(),
            projection_area_diagnostic: TARGET_PROJECTION_AREA_DIAGNOSTIC,
            projection_area_offset_left_uv: TARGET_PROJECTION_AREA_OFFSET_LEFT_UV,
            projection_area_offset_right_uv: TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV,
            projection_area_offset_vertical_uv: TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV,
            projection_area_scale_x: TARGET_PROJECTION_AREA_SCALE_X,
            projection_area_scale_y: TARGET_PROJECTION_AREA_SCALE_Y,
            projection_area_radius_x_uv: TARGET_PROJECTION_AREA_RADIUS_X_UV,
            projection_area_radius_y_uv: TARGET_PROJECTION_AREA_RADIUS_Y_UV,
            projection_area_corner_radius_uv: TARGET_PROJECTION_AREA_CORNER_RADIUS_UV,
            projection_area_keystone_x: TARGET_PROJECTION_AREA_KEYSTONE_X,
            projection_area_bow_x: TARGET_PROJECTION_AREA_BOW_X,
            projection_area_opacity: TARGET_PROJECTION_AREA_OPACITY,
            projection_alpha_mode: MakepadProjectionAlphaMode::current().shader_code(),
            projection_alpha_scale: makepad_projection_alpha_scale(),
            projection_alpha_bias: makepad_projection_alpha_bias(),
        }
    }
}

impl MakepadStereoCameraPanel {
    fn apply_projection_panel_geometry(&mut self, cx: &mut Cx) {
        let geometry = makepad_projection_panel_geometry();
        self.size = geometry.size();
        self.node.set_implicit_physics_size(self.size);
        self.node.set_pos(cx, geometry.pos());
    }

    fn synthetic_luma_probe_texture(&mut self, cx: &mut Cx) -> Texture {
        if let Some(texture) = &self.synthetic_luma_probe_texture {
            return texture.clone();
        }

        let size = SYNTHETIC_LUMA_PROBE_SIZE;
        let mut data = Vec::with_capacity(size * size);
        for y in 0..size {
            for x in 0..size {
                let band = ((x / 16) + (y / 16)) % 4;
                data.push(8 + (band as u8) * 8);
            }
        }

        let texture = Texture::new_with_format(
            cx,
            TextureFormat::VecRu8 {
                width: size,
                height: size,
                data: Some(data),
                unpack_row_length: None,
                updated: TextureUpdated::Full,
            },
        );
        self.synthetic_luma_probe_texture = Some(texture.clone());
        texture
    }

    #[allow(clippy::too_many_arguments)]
    fn set_camera_textures(
        &mut self,
        cx: &mut Cx,
        left: Option<Texture>,
        right: Option<Texture>,
        left_yuv: Option<MakepadCameraYuvTextures>,
        right_yuv: Option<MakepadCameraYuvTextures>,
        left_rotation_steps: f32,
        right_rotation_steps: f32,
        left_surface_to_camera_h: [[f32; 3]; 3],
        right_surface_to_camera_h: [[f32; 3]; 3],
        left_screen_to_camera_h: [[f32; 3]; 3],
        right_screen_to_camera_h: [[f32; 3]; 3],
        left_screen_to_surface_h: [[f32; 3]; 3],
        right_screen_to_surface_h: [[f32; 3]; 3],
        source_sample_y_flip: f32,
        projection_content_mapping_mode: f32,
    ) {
        self.draw_panel.set_camera_textures(cx, left, right);
        let (left_yuv, right_yuv) = if SYNTHETIC_LUMA_SLOT_PROOF {
            let probe = self.synthetic_luma_probe_texture(cx);
            if SYNTHETIC_LUMA_ALL_SLOT_PROOF {
                for slot in 0..8 {
                    self.draw_panel
                        .assign_texture_slot(slot, Some(probe.clone()));
                }
            }
            let textures = MakepadCameraYuvTextures::new(probe.clone(), probe.clone(), probe);
            (Some(textures.clone()), Some(textures))
        } else {
            (left_yuv, right_yuv)
        };
        self.draw_panel
            .set_camera_yuv_textures(cx, left_yuv, right_yuv);
        self.draw_panel.left_rotation_steps = left_rotation_steps;
        self.draw_panel.right_rotation_steps = right_rotation_steps;
        self.draw_panel.left_projection_h00 = left_surface_to_camera_h[0][0];
        self.draw_panel.left_projection_h01 = left_surface_to_camera_h[0][1];
        self.draw_panel.left_projection_h02 = left_surface_to_camera_h[0][2];
        self.draw_panel.left_projection_h10 = left_surface_to_camera_h[1][0];
        self.draw_panel.left_projection_h11 = left_surface_to_camera_h[1][1];
        self.draw_panel.left_projection_h12 = left_surface_to_camera_h[1][2];
        self.draw_panel.left_projection_h20 = left_surface_to_camera_h[2][0];
        self.draw_panel.left_projection_h21 = left_surface_to_camera_h[2][1];
        self.draw_panel.left_projection_h22 = left_surface_to_camera_h[2][2];
        self.draw_panel.right_projection_h00 = right_surface_to_camera_h[0][0];
        self.draw_panel.right_projection_h01 = right_surface_to_camera_h[0][1];
        self.draw_panel.right_projection_h02 = right_surface_to_camera_h[0][2];
        self.draw_panel.right_projection_h10 = right_surface_to_camera_h[1][0];
        self.draw_panel.right_projection_h11 = right_surface_to_camera_h[1][1];
        self.draw_panel.right_projection_h12 = right_surface_to_camera_h[1][2];
        self.draw_panel.right_projection_h20 = right_surface_to_camera_h[2][0];
        self.draw_panel.right_projection_h21 = right_surface_to_camera_h[2][1];
        self.draw_panel.right_projection_h22 = right_surface_to_camera_h[2][2];
        self.draw_panel.left_screen_to_camera_h00 = left_screen_to_camera_h[0][0];
        self.draw_panel.left_screen_to_camera_h01 = left_screen_to_camera_h[0][1];
        self.draw_panel.left_screen_to_camera_h02 = left_screen_to_camera_h[0][2];
        self.draw_panel.left_screen_to_camera_h10 = left_screen_to_camera_h[1][0];
        self.draw_panel.left_screen_to_camera_h11 = left_screen_to_camera_h[1][1];
        self.draw_panel.left_screen_to_camera_h12 = left_screen_to_camera_h[1][2];
        self.draw_panel.left_screen_to_camera_h20 = left_screen_to_camera_h[2][0];
        self.draw_panel.left_screen_to_camera_h21 = left_screen_to_camera_h[2][1];
        self.draw_panel.left_screen_to_camera_h22 = left_screen_to_camera_h[2][2];
        self.draw_panel.right_screen_to_camera_h00 = right_screen_to_camera_h[0][0];
        self.draw_panel.right_screen_to_camera_h01 = right_screen_to_camera_h[0][1];
        self.draw_panel.right_screen_to_camera_h02 = right_screen_to_camera_h[0][2];
        self.draw_panel.right_screen_to_camera_h10 = right_screen_to_camera_h[1][0];
        self.draw_panel.right_screen_to_camera_h11 = right_screen_to_camera_h[1][1];
        self.draw_panel.right_screen_to_camera_h12 = right_screen_to_camera_h[1][2];
        self.draw_panel.right_screen_to_camera_h20 = right_screen_to_camera_h[2][0];
        self.draw_panel.right_screen_to_camera_h21 = right_screen_to_camera_h[2][1];
        self.draw_panel.right_screen_to_camera_h22 = right_screen_to_camera_h[2][2];
        self.draw_panel.left_screen_to_surface_h00 = left_screen_to_surface_h[0][0];
        self.draw_panel.left_screen_to_surface_h01 = left_screen_to_surface_h[0][1];
        self.draw_panel.left_screen_to_surface_h02 = left_screen_to_surface_h[0][2];
        self.draw_panel.left_screen_to_surface_h10 = left_screen_to_surface_h[1][0];
        self.draw_panel.left_screen_to_surface_h11 = left_screen_to_surface_h[1][1];
        self.draw_panel.left_screen_to_surface_h12 = left_screen_to_surface_h[1][2];
        self.draw_panel.left_screen_to_surface_h20 = left_screen_to_surface_h[2][0];
        self.draw_panel.left_screen_to_surface_h21 = left_screen_to_surface_h[2][1];
        self.draw_panel.left_screen_to_surface_h22 = left_screen_to_surface_h[2][2];
        self.draw_panel.right_screen_to_surface_h00 = right_screen_to_surface_h[0][0];
        self.draw_panel.right_screen_to_surface_h01 = right_screen_to_surface_h[0][1];
        self.draw_panel.right_screen_to_surface_h02 = right_screen_to_surface_h[0][2];
        self.draw_panel.right_screen_to_surface_h10 = right_screen_to_surface_h[1][0];
        self.draw_panel.right_screen_to_surface_h11 = right_screen_to_surface_h[1][1];
        self.draw_panel.right_screen_to_surface_h12 = right_screen_to_surface_h[1][2];
        self.draw_panel.right_screen_to_surface_h20 = right_screen_to_surface_h[2][0];
        self.draw_panel.right_screen_to_surface_h21 = right_screen_to_surface_h[2][1];
        self.draw_panel.right_screen_to_surface_h22 = right_screen_to_surface_h[2][2];
        self.draw_panel.source_sample_y_flip = source_sample_y_flip.clamp(0.0, 1.0);
        self.draw_panel.projection_content_mapping_mode =
            projection_content_mapping_mode.clamp(0.0, 1.0);
        self.draw_panel.content_uv_scale = TARGET_FULL_VIEW_CONTENT_UV_SCALE;
        self.draw_panel.display_source_eye_swap = if makepad_display_left_from_right_source() {
            1.0
        } else {
            0.0
        };
        self.draw_panel.display_eye_offset_meters = TARGET_DISPLAY_EYE_OFFSET_METERS;
        self.draw_panel.display_fov_y_degrees = TARGET_DISPLAY_FOV_Y_DEGREES;
        self.draw_panel.display_aspect = TARGET_DISPLAY_ASPECT;
        self.draw_panel.projection_depth_meters = makepad_projection_depth_meters();
        self.draw_panel.projection_preview_offset_y_meters =
            makepad_projection_preview_offset_y_meters();
        self.draw_panel.projection_preview_fov_y_degrees =
            makepad_projection_preview_fov_y_degrees();
        self.draw_panel.projection_raw_overscan = makepad_projection_raw_overscan();
        self.draw_panel.suppress_live_camera_sampling = if SUPPRESS_LIVE_CAMERA_SAMPLING {
            1.0
        } else {
            0.0
        };
        self.draw_panel.force_full_surface_live_camera_uv = if FORCE_FULL_SURFACE_LIVE_CAMERA_UV {
            1.0
        } else {
            0.0
        };
        self.draw_panel.force_in_surface_camera_window = if FORCE_IN_SURFACE_CAMERA_WINDOW {
            1.0
        } else {
            0.0
        };
        self.draw_panel.projection_border_opacity = TARGET_PROJECTION_BORDER_OPACITY;
        self.draw_panel.projection_border_policy =
            MakepadProjectionBorderPolicy::current().shader_code();
        self.draw_panel.processing_layer = MakepadProcessingLayer::current().shader_code();
        self.draw_panel.blur_radius_px = makepad_blur_radius_px();
        self.draw_panel.projection_area_diagnostic = TARGET_PROJECTION_AREA_DIAGNOSTIC;
        self.draw_panel.projection_area_offset_left_uv = TARGET_PROJECTION_AREA_OFFSET_LEFT_UV;
        self.draw_panel.projection_area_offset_right_uv = TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV;
        self.draw_panel.projection_area_offset_vertical_uv =
            TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV;
        self.draw_panel.projection_area_scale_x = TARGET_PROJECTION_AREA_SCALE_X;
        self.draw_panel.projection_area_scale_y = TARGET_PROJECTION_AREA_SCALE_Y;
        self.draw_panel.projection_area_radius_x_uv = TARGET_PROJECTION_AREA_RADIUS_X_UV;
        self.draw_panel.projection_area_radius_y_uv = TARGET_PROJECTION_AREA_RADIUS_Y_UV;
        self.draw_panel.projection_area_corner_radius_uv = TARGET_PROJECTION_AREA_CORNER_RADIUS_UV;
        self.draw_panel.projection_area_keystone_x = TARGET_PROJECTION_AREA_KEYSTONE_X;
        self.draw_panel.projection_area_bow_x = TARGET_PROJECTION_AREA_BOW_X;
        self.draw_panel.projection_area_opacity = TARGET_PROJECTION_AREA_OPACITY;
        self.draw_panel.projection_alpha_mode = MakepadProjectionAlphaMode::current().shader_code();
        self.draw_panel.projection_alpha_scale = makepad_projection_alpha_scale();
        self.draw_panel.projection_alpha_bias = makepad_projection_alpha_bias();
        self.set_horizontal_alignment_tuning(cx, App::horizontal_alignment_tuning());
        self.draw_panel.camera_ready = 1.0;
        self.draw_panel.texture_probe_mode = 2.0;
        self.draw_panel.draw_vars.redraw(cx);
        self.draw_panel
            .draw_vars
            .set_instance_on_area(cx, live_id!(camera_ready), &[1.0]);
        self.draw_panel
            .draw_vars
            .set_uniform_on_area(cx, live_id!(camera_ready), &[1.0]);
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(yuv_mode),
            &[self.draw_panel.yuv_mode],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(yuv_mode),
            &[self.draw_panel.yuv_mode],
        );
        self.draw_panel
            .draw_vars
            .set_instance_on_area(cx, live_id!(proof_tint_strength), &[0.0]);
        self.draw_panel
            .draw_vars
            .set_uniform_on_area(cx, live_id!(proof_tint_strength), &[0.0]);
        self.draw_panel
            .draw_vars
            .set_instance_on_area(cx, live_id!(texture_probe_mode), &[2.0]);
        self.draw_panel
            .draw_vars
            .set_uniform_on_area(cx, live_id!(texture_probe_mode), &[2.0]);
        let suppress_live_camera_sampling = if SUPPRESS_LIVE_CAMERA_SAMPLING {
            1.0
        } else {
            0.0
        };
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(suppress_live_camera_sampling),
            &[suppress_live_camera_sampling],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(suppress_live_camera_sampling),
            &[suppress_live_camera_sampling],
        );
        let force_full_surface_live_camera_uv = if FORCE_FULL_SURFACE_LIVE_CAMERA_UV {
            1.0
        } else {
            0.0
        };
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(force_full_surface_live_camera_uv),
            &[force_full_surface_live_camera_uv],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(force_full_surface_live_camera_uv),
            &[force_full_surface_live_camera_uv],
        );
        let force_in_surface_camera_window = if FORCE_IN_SURFACE_CAMERA_WINDOW {
            1.0
        } else {
            0.0
        };
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(force_in_surface_camera_window),
            &[force_in_surface_camera_window],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(force_in_surface_camera_window),
            &[force_in_surface_camera_window],
        );
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(projection_border_opacity),
            &[TARGET_PROJECTION_BORDER_OPACITY],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(projection_border_opacity),
            &[TARGET_PROJECTION_BORDER_OPACITY],
        );
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(projection_border_policy),
            &[self.draw_panel.projection_border_policy],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(projection_border_policy),
            &[self.draw_panel.projection_border_policy],
        );
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(processing_layer),
            &[self.draw_panel.processing_layer],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(processing_layer),
            &[self.draw_panel.processing_layer],
        );
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(blur_radius_px),
            &[self.draw_panel.blur_radius_px],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(blur_radius_px),
            &[self.draw_panel.blur_radius_px],
        );
        self.draw_panel.draw_vars.set_instance_on_area(
            cx,
            live_id!(projection_area_diagnostic),
            &[TARGET_PROJECTION_AREA_DIAGNOSTIC],
        );
        self.draw_panel.draw_vars.set_uniform_on_area(
            cx,
            live_id!(projection_area_diagnostic),
            &[TARGET_PROJECTION_AREA_DIAGNOSTIC],
        );
        for (id, value) in [
            (
                live_id!(content_uv_scale),
                TARGET_FULL_VIEW_CONTENT_UV_SCALE,
            ),
            (
                live_id!(projection_border_opacity),
                TARGET_PROJECTION_BORDER_OPACITY,
            ),
            (live_id!(processing_layer), self.draw_panel.processing_layer),
            (live_id!(blur_radius_px), self.draw_panel.blur_radius_px),
            (
                live_id!(projection_area_diagnostic),
                TARGET_PROJECTION_AREA_DIAGNOSTIC,
            ),
            (
                live_id!(projection_area_offset_left_uv),
                TARGET_PROJECTION_AREA_OFFSET_LEFT_UV,
            ),
            (
                live_id!(projection_area_offset_right_uv),
                TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV,
            ),
            (
                live_id!(projection_area_offset_vertical_uv),
                TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV,
            ),
            (
                live_id!(projection_area_scale_x),
                TARGET_PROJECTION_AREA_SCALE_X,
            ),
            (
                live_id!(projection_area_scale_y),
                TARGET_PROJECTION_AREA_SCALE_Y,
            ),
            (
                live_id!(projection_area_keystone_x),
                TARGET_PROJECTION_AREA_KEYSTONE_X,
            ),
            (
                live_id!(projection_area_bow_x),
                TARGET_PROJECTION_AREA_BOW_X,
            ),
            (
                live_id!(projection_area_opacity),
                TARGET_PROJECTION_AREA_OPACITY,
            ),
            (
                live_id!(projection_alpha_mode),
                self.draw_panel.projection_alpha_mode,
            ),
            (
                live_id!(projection_alpha_scale),
                self.draw_panel.projection_alpha_scale,
            ),
            (
                live_id!(projection_alpha_bias),
                self.draw_panel.projection_alpha_bias,
            ),
            (
                live_id!(source_sample_y_flip),
                self.draw_panel.source_sample_y_flip,
            ),
            (
                live_id!(projection_content_mapping_mode),
                self.draw_panel.projection_content_mapping_mode,
            ),
            (
                live_id!(display_eye_offset_meters),
                TARGET_DISPLAY_EYE_OFFSET_METERS,
            ),
            (
                live_id!(display_fov_y_degrees),
                TARGET_DISPLAY_FOV_Y_DEGREES,
            ),
            (live_id!(display_aspect), TARGET_DISPLAY_ASPECT),
            (
                live_id!(projection_depth_meters),
                self.draw_panel.projection_depth_meters,
            ),
            (
                live_id!(projection_preview_offset_y_meters),
                self.draw_panel.projection_preview_offset_y_meters,
            ),
            (
                live_id!(projection_preview_fov_y_degrees),
                self.draw_panel.projection_preview_fov_y_degrees,
            ),
            (
                live_id!(projection_raw_overscan),
                self.draw_panel.projection_raw_overscan,
            ),
        ] {
            self.draw_panel.draw_vars.set_dyn_instance(cx, id, &[value]);
            self.draw_panel.draw_vars.set_uniform(cx, id, &[value]);
            self.draw_panel
                .draw_vars
                .set_instance_on_area(cx, id, &[value]);
            self.draw_panel
                .draw_vars
                .set_uniform_on_area(cx, id, &[value]);
        }
        for (id, value) in [
            (
                live_id!(left_projection_h00),
                left_surface_to_camera_h[0][0],
            ),
            (
                live_id!(left_projection_h01),
                left_surface_to_camera_h[0][1],
            ),
            (
                live_id!(left_projection_h02),
                left_surface_to_camera_h[0][2],
            ),
            (
                live_id!(left_projection_h10),
                left_surface_to_camera_h[1][0],
            ),
            (
                live_id!(left_projection_h11),
                left_surface_to_camera_h[1][1],
            ),
            (
                live_id!(left_projection_h12),
                left_surface_to_camera_h[1][2],
            ),
            (
                live_id!(left_projection_h20),
                left_surface_to_camera_h[2][0],
            ),
            (
                live_id!(left_projection_h21),
                left_surface_to_camera_h[2][1],
            ),
            (
                live_id!(left_projection_h22),
                left_surface_to_camera_h[2][2],
            ),
            (
                live_id!(right_projection_h00),
                right_surface_to_camera_h[0][0],
            ),
            (
                live_id!(right_projection_h01),
                right_surface_to_camera_h[0][1],
            ),
            (
                live_id!(right_projection_h02),
                right_surface_to_camera_h[0][2],
            ),
            (
                live_id!(right_projection_h10),
                right_surface_to_camera_h[1][0],
            ),
            (
                live_id!(right_projection_h11),
                right_surface_to_camera_h[1][1],
            ),
            (
                live_id!(right_projection_h12),
                right_surface_to_camera_h[1][2],
            ),
            (
                live_id!(right_projection_h20),
                right_surface_to_camera_h[2][0],
            ),
            (
                live_id!(right_projection_h21),
                right_surface_to_camera_h[2][1],
            ),
            (
                live_id!(right_projection_h22),
                right_surface_to_camera_h[2][2],
            ),
            (
                live_id!(left_screen_to_camera_h00),
                left_screen_to_camera_h[0][0],
            ),
            (
                live_id!(left_screen_to_camera_h01),
                left_screen_to_camera_h[0][1],
            ),
            (
                live_id!(left_screen_to_camera_h02),
                left_screen_to_camera_h[0][2],
            ),
            (
                live_id!(left_screen_to_camera_h10),
                left_screen_to_camera_h[1][0],
            ),
            (
                live_id!(left_screen_to_camera_h11),
                left_screen_to_camera_h[1][1],
            ),
            (
                live_id!(left_screen_to_camera_h12),
                left_screen_to_camera_h[1][2],
            ),
            (
                live_id!(left_screen_to_camera_h20),
                left_screen_to_camera_h[2][0],
            ),
            (
                live_id!(left_screen_to_camera_h21),
                left_screen_to_camera_h[2][1],
            ),
            (
                live_id!(left_screen_to_camera_h22),
                left_screen_to_camera_h[2][2],
            ),
            (
                live_id!(right_screen_to_camera_h00),
                right_screen_to_camera_h[0][0],
            ),
            (
                live_id!(right_screen_to_camera_h01),
                right_screen_to_camera_h[0][1],
            ),
            (
                live_id!(right_screen_to_camera_h02),
                right_screen_to_camera_h[0][2],
            ),
            (
                live_id!(right_screen_to_camera_h10),
                right_screen_to_camera_h[1][0],
            ),
            (
                live_id!(right_screen_to_camera_h11),
                right_screen_to_camera_h[1][1],
            ),
            (
                live_id!(right_screen_to_camera_h12),
                right_screen_to_camera_h[1][2],
            ),
            (
                live_id!(right_screen_to_camera_h20),
                right_screen_to_camera_h[2][0],
            ),
            (
                live_id!(right_screen_to_camera_h21),
                right_screen_to_camera_h[2][1],
            ),
            (
                live_id!(right_screen_to_camera_h22),
                right_screen_to_camera_h[2][2],
            ),
        ] {
            self.draw_panel.draw_vars.set_dyn_instance(cx, id, &[value]);
            self.draw_panel.draw_vars.set_uniform(cx, id, &[value]);
            self.draw_panel
                .draw_vars
                .set_instance_on_area(cx, id, &[value]);
            self.draw_panel
                .draw_vars
                .set_uniform_on_area(cx, id, &[value]);
        }
        for (id, value) in [
            (
                live_id!(left_screen_to_surface_h00),
                left_screen_to_surface_h[0][0],
            ),
            (
                live_id!(left_screen_to_surface_h01),
                left_screen_to_surface_h[0][1],
            ),
            (
                live_id!(left_screen_to_surface_h02),
                left_screen_to_surface_h[0][2],
            ),
            (
                live_id!(left_screen_to_surface_h10),
                left_screen_to_surface_h[1][0],
            ),
            (
                live_id!(left_screen_to_surface_h11),
                left_screen_to_surface_h[1][1],
            ),
            (
                live_id!(left_screen_to_surface_h12),
                left_screen_to_surface_h[1][2],
            ),
            (
                live_id!(left_screen_to_surface_h20),
                left_screen_to_surface_h[2][0],
            ),
            (
                live_id!(left_screen_to_surface_h21),
                left_screen_to_surface_h[2][1],
            ),
            (
                live_id!(left_screen_to_surface_h22),
                left_screen_to_surface_h[2][2],
            ),
            (
                live_id!(right_screen_to_surface_h00),
                right_screen_to_surface_h[0][0],
            ),
            (
                live_id!(right_screen_to_surface_h01),
                right_screen_to_surface_h[0][1],
            ),
            (
                live_id!(right_screen_to_surface_h02),
                right_screen_to_surface_h[0][2],
            ),
            (
                live_id!(right_screen_to_surface_h10),
                right_screen_to_surface_h[1][0],
            ),
            (
                live_id!(right_screen_to_surface_h11),
                right_screen_to_surface_h[1][1],
            ),
            (
                live_id!(right_screen_to_surface_h12),
                right_screen_to_surface_h[1][2],
            ),
            (
                live_id!(right_screen_to_surface_h20),
                right_screen_to_surface_h[2][0],
            ),
            (
                live_id!(right_screen_to_surface_h21),
                right_screen_to_surface_h[2][1],
            ),
            (
                live_id!(right_screen_to_surface_h22),
                right_screen_to_surface_h[2][2],
            ),
        ] {
            self.draw_panel.draw_vars.set_dyn_instance(cx, id, &[value]);
            self.draw_panel.draw_vars.set_uniform(cx, id, &[value]);
            self.draw_panel
                .draw_vars
                .set_instance_on_area(cx, id, &[value]);
            self.draw_panel
                .draw_vars
                .set_uniform_on_area(cx, id, &[value]);
        }
        self.camera_ready = true;
        self.node.redraw(cx);
    }

    fn set_horizontal_alignment_tuning(&mut self, cx: &mut Cx, tuning: HorizontalAlignmentTuning) {
        self.draw_panel.horizontal_alignment_strength = tuning.strength;
        self.draw_panel.manual_horizontal_offset_left_uv = tuning.left_offset_uv;
        self.draw_panel.manual_horizontal_offset_right_uv = tuning.right_offset_uv;
        self.draw_panel.manual_vertical_offset_uv = tuning.vertical_offset_uv;
        self.draw_panel.content_uv_scale = tuning.content_uv_scale;
        self.draw_panel.projection_border_opacity = tuning.projection_border_opacity;
        self.draw_panel.projection_border_policy = tuning.projection_border_policy;
        self.draw_panel.processing_layer = tuning.processing_layer;
        self.draw_panel.blur_radius_px = tuning.blur_radius_px;
        self.draw_panel.projection_area_diagnostic = tuning.projection_area_diagnostic;
        self.draw_panel.projection_area_offset_left_uv = tuning.projection_area_offset_left_uv;
        self.draw_panel.projection_area_offset_right_uv = tuning.projection_area_offset_right_uv;
        self.draw_panel.projection_area_offset_vertical_uv =
            tuning.projection_area_offset_vertical_uv;
        self.draw_panel.projection_area_scale_x = tuning.projection_area_scale_x;
        self.draw_panel.projection_area_scale_y = tuning.projection_area_scale_y;
        self.draw_panel.projection_area_radius_x_uv = tuning.projection_area_radius_x_uv;
        self.draw_panel.projection_area_radius_y_uv = tuning.projection_area_radius_y_uv;
        self.draw_panel.projection_area_corner_radius_uv = tuning.projection_area_corner_radius_uv;
        self.draw_panel.projection_area_keystone_x = tuning.projection_area_keystone_x;
        self.draw_panel.projection_area_bow_x = tuning.projection_area_bow_x;
        self.draw_panel.projection_area_opacity = tuning.projection_area_opacity;
        self.draw_panel.projection_alpha_mode = tuning.projection_alpha_mode;
        self.draw_panel.projection_alpha_scale = tuning.projection_alpha_scale;
        self.draw_panel.projection_alpha_bias = tuning.projection_alpha_bias;
        for (id, value) in [
            (live_id!(horizontal_alignment_strength), tuning.strength),
            (
                live_id!(manual_horizontal_offset_left_uv),
                tuning.left_offset_uv,
            ),
            (
                live_id!(manual_horizontal_offset_right_uv),
                tuning.right_offset_uv,
            ),
            (
                live_id!(manual_vertical_offset_uv),
                tuning.vertical_offset_uv,
            ),
            (live_id!(content_uv_scale), tuning.content_uv_scale),
            (
                live_id!(projection_border_opacity),
                tuning.projection_border_opacity,
            ),
            (
                live_id!(projection_border_policy),
                tuning.projection_border_policy,
            ),
            (live_id!(processing_layer), tuning.processing_layer),
            (live_id!(blur_radius_px), tuning.blur_radius_px),
            (
                live_id!(projection_area_diagnostic),
                tuning.projection_area_diagnostic,
            ),
            (
                live_id!(projection_area_offset_left_uv),
                tuning.projection_area_offset_left_uv,
            ),
            (
                live_id!(projection_area_offset_right_uv),
                tuning.projection_area_offset_right_uv,
            ),
            (
                live_id!(projection_area_offset_vertical_uv),
                tuning.projection_area_offset_vertical_uv,
            ),
            (
                live_id!(projection_area_scale_x),
                tuning.projection_area_scale_x,
            ),
            (
                live_id!(projection_area_scale_y),
                tuning.projection_area_scale_y,
            ),
            (
                live_id!(projection_area_radius_x_uv),
                tuning.projection_area_radius_x_uv,
            ),
            (
                live_id!(projection_area_radius_y_uv),
                tuning.projection_area_radius_y_uv,
            ),
            (
                live_id!(projection_area_corner_radius_uv),
                tuning.projection_area_corner_radius_uv,
            ),
            (
                live_id!(projection_area_keystone_x),
                tuning.projection_area_keystone_x,
            ),
            (
                live_id!(projection_area_bow_x),
                tuning.projection_area_bow_x,
            ),
            (
                live_id!(projection_area_opacity),
                tuning.projection_area_opacity,
            ),
            (
                live_id!(projection_alpha_mode),
                tuning.projection_alpha_mode,
            ),
            (
                live_id!(projection_alpha_scale),
                tuning.projection_alpha_scale,
            ),
            (
                live_id!(projection_alpha_bias),
                tuning.projection_alpha_bias,
            ),
        ] {
            self.draw_panel.draw_vars.set_dyn_instance(cx, id, &[value]);
            self.draw_panel.draw_vars.set_uniform(cx, id, &[value]);
            self.draw_panel
                .draw_vars
                .set_instance_on_area(cx, id, &[value]);
            self.draw_panel
                .draw_vars
                .set_uniform_on_area(cx, id, &[value]);
        }
        self.draw_panel.draw_vars.redraw(cx);
    }
}

impl ScriptHook for MakepadStereoCameraPanel {
    fn on_after_apply(
        &mut self,
        _vm: &mut ScriptVm,
        _apply: &Apply,
        _scope: &mut Scope,
        _value: ScriptValue,
    ) {
        self.node.set_implicit_physics_size(self.size);
    }
}

impl Widget for MakepadStereoCameraPanel {
    fn draw_3d(&mut self, cx: &mut Cx3d, scope: &mut Scope) -> DrawStep {
        if cx.scene_state_3d().is_none() {
            return self.node.draw_3d(cx, scope);
        }
        if !CAMERA_PANEL_DRAW_MARKER_EMITTED.swap(true, Ordering::AcqRel) {
            emit_marker_line(&format!(
                "RUSTY_XR_MAKEPAD_STEREO_PROJECTION schema=rusty.xr.makepad-stereo-projection.v1 phase=visible-panel-draw status=ok visibleCameraPanelDrawn=true cameraTextureReady={} renderPath=makepad-xr sceneOwnedPanel=true projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 colorReference=android-yuv420-888-plane-order perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review projectionDepthMeters={:.2} panelTargetDepthMeters={:.2} panelTargetPreviewFovYDegrees={:.3} panelTargetPreviewOffsetYMeters={:.3} panelTargetRawOverscan={:.3} {} diagnosticVisualLayer=none neutralWaitingPanel=true visualIsolation=s118_projected_footprint_solid_red_exterior depthClip=false environmentDepthClip=false visualInspection=required visualReleaseAccepted=false",
                self.camera_ready,
                self.draw_panel.projection_depth_meters,
                self.draw_panel.projection_depth_meters,
                self.draw_panel.projection_preview_fov_y_degrees,
                self.draw_panel.projection_preview_offset_y_meters,
                self.draw_panel.projection_raw_overscan,
                makepad_projection_target_marker_fields()
            ));
        }
        let _world = xr_widget_world_transform(cx, scope, self.widget_uid(), &self.node);
        self.draw_panel.cube_pos = vec3f(0.0, 0.0, 0.0);
        self.draw_panel.cube_size = vec3f(1.0, 1.0, 0.0);
        self.draw_panel.depth_clip = 0.0;
        self.draw_panel.draw(cx);

        self.node.draw_3d(cx, scope)
    }

    fn draw_walk(&mut self, _cx: &mut Cx2d, _scope: &mut Scope, _walk: Walk) -> DrawStep {
        DrawStep::done()
    }
}

impl App {
    fn emit_startup_markers_once(phase: &str) {
        if STARTUP_MARKERS_EMITTED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        Self::emit_status_marker(phase);
        Self::emit_stereo_comparison_marker(phase);
        if Self::broker_h264_enabled() {
            emit_marker_line(
                "RUSTY_XR_MAKEPAD_CAMERA2_ACQUISITION schema=rusty.xr.makepad-camera2.acquisition.v1 phase=start status=skipped reason=broker-h264-enabled import=broker-h264",
            );
        } else {
            Self::start_camera_probe_once();
        }
    }

    fn emit_status_marker(phase: &str) {
        let config = Self::runtime_config();

        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_CAMERA_STATUS schema=rusty.xr.makepad-camera.status.v1 phase={} profile={} transport={} renderer=makepad android_packager=cargo-makepad makepad_rev={} studio_host={}",
            phase,
            runtime_text(&config, KEY_RUNTIME_PROFILE),
            runtime_text(&config, KEY_TRANSPORT_PROFILE),
            runtime_text(&config, KEY_MAKEPAD_REVISION),
            runtime_text(&config, KEY_STUDIO_HOST)
        ));
    }

    fn emit_stereo_comparison_marker(phase: &str) {
        let config = Self::runtime_config();
        let tuning = Self::horizontal_alignment_tuning();

        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_STEREO_COMPARISON schema=rusty.xr.makepad-stereo-comparison.v1 phase={} profile={} comparisonBaseline={} cameraTier={} acquisition={} transport={} projectionMode={} syntheticScene={} leftEyeSource=synthetic-left rightEyeSource=synthetic-right sourceEyeMapping=display-eye projectionScale={:.2} xrRenderScale={:.2} pairedLeftRightGpuBuffers=false alignedProjection=false renderPath=makepad-xr makepadForkBranch={} makepadForkCommit={} {} nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s102FullSurfaceLiveCameraCoverageControl=false s103InSurfaceCameraWindowBorderControl=true s104HorizontalWindowAlignmentControl=false s105HotloadHorizontalAlignmentControl=true s106SafeHorizontalWindowSampling=true s107WindowScaleHotload=true s108BorderlessWindowScale=false s109SolidRedProjectionExterior=true s110VerticalWindowOffsetHotload=true horizontalAlignmentSource=screen_to_camera_center_delta_projection_area_source_valid_window manualHorizontalOffsetHotload=true verticalOffsetHotload=true contentUvScaleHotload=true borderlessWindowMask=false solidRedProjectionExterior=true horizontalAlignmentStrength={:.3} manualLeftUv={:.4} manualRightUv={:.4} manualVerticalUv={:.4} contentUvScale={:.4} liveCameraSamplingSuppressed=false forceFullSurfaceLiveCameraUv=false forceInSurfaceCameraWindow=true liveCameraWindowDomain=projected_camera_uv fullSurfaceLayerActive=false cameraCoverageInShader=true layerNotResized=false panelSizedFromProjectionSurface=true projectionValidMaskDisabled=false visualIsolation=s118_projected_footprint_solid_red_exterior",
            phase,
            runtime_text(&config, KEY_RUNTIME_PROFILE),
            runtime_text(&config, KEY_COMPARISON_BASELINE),
            runtime_text(&config, KEY_CAMERA_TIER),
            runtime_text(&config, KEY_ACQUISITION_PROFILE),
            runtime_text(&config, KEY_TRANSPORT_PROFILE),
            runtime_text(&config, KEY_CAMERA_PROJECTION_MODE),
            runtime_text(&config, KEY_SYNTHETIC_SCENE),
            runtime_float(&config, KEY_PROJECTION_SCALE),
            runtime_float(&config, KEY_XR_RENDER_SCALE),
            runtime_text(&config, KEY_MAKEPAD_BRANCH),
            runtime_text(&config, KEY_MAKEPAD_REVISION),
            makepad_projection_target_marker_fields(),
            tuning.strength,
            tuning.left_offset_uv,
            tuning.right_offset_uv,
            tuning.vertical_offset_uv,
            tuning.content_uv_scale
        ));
        Self::emit_projection_runtime_manifest_marker(phase, &config, tuning);
    }

    fn emit_projection_runtime_manifest_marker(
        phase: &str,
        config: &RuntimeConfig,
        tuning: HorizontalAlignmentTuning,
    ) {
        for line in makepad_projection_runtime_manifest_lines(phase, config, tuning) {
            emit_marker_line(&line);
        }
    }

    fn runtime_config() -> RuntimeConfig {
        let mut config = RuntimeConfig::new();
        set_runtime_text(
            &mut config,
            KEY_RUNTIME_PROFILE,
            std::env::var("RUSTY_XR_RUNTIME_PROFILE")
                .unwrap_or_else(|_| DEFAULT_PROFILE.to_string()),
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
            std::env::var("RUSTY_XR_CAMERA_TIER")
                .unwrap_or_else(|_| DEFAULT_CAMERA_TIER.to_string()),
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
                0.0,
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

    fn broker_h264_enabled() -> bool {
        let transport_requests_broker = std::env::var("RUSTY_XR_TRANSPORT_PROFILE")
            .map(|value| value.to_ascii_lowercase().contains("broker-h264"))
            .unwrap_or(false);
        hotload_bool(
            KEY_MAKEPAD_BROKER_H264_ENABLED,
            DEFAULT_BROKER_H264_ENABLED || transport_requests_broker,
        )
    }

    fn direct_camera_projection_geometry_profile() -> String {
        if makepad_camera_projection_mode_is_world_canvas() {
            return "full-frame-diagnostic".to_string();
        }
        normalize_direct_camera_projection_geometry_profile(&hotload_text(
            KEY_MAKEPAD_CAMERA_PROJECTION_GEOMETRY_PROFILE,
            DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE,
        ))
    }

    fn broker_h264_stream_port(eye: StereoEye) -> u16 {
        match eye {
            StereoEye::Left => hotload_u16(
                KEY_MAKEPAD_BROKER_H264_STREAM_PORT,
                DEFAULT_BROKER_H264_STREAM_PORT,
                1,
                u16::MAX,
            ),
            StereoEye::Right => hotload_u16(
                KEY_MAKEPAD_BROKER_H264_RIGHT_STREAM_PORT,
                DEFAULT_BROKER_H264_RIGHT_STREAM_PORT,
                1,
                u16::MAX,
            ),
        }
    }

    fn broker_h264_source_for_eye(eye: StereoEye) -> BrokerH264VideoSource {
        let synthetic_projection_profile = hotload_text(
            KEY_MAKEPAD_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE,
            DEFAULT_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE,
        );
        let projection_geometry_profile = hotload_text(
            KEY_MAKEPAD_BROKER_H264_PROJECTION_GEOMETRY_PROFILE,
            &synthetic_projection_profile,
        );
        BrokerH264VideoSource {
            broker_host: hotload_text(KEY_MAKEPAD_BROKER_H264_HOST, DEFAULT_BROKER_H264_HOST),
            broker_port: hotload_u16(
                KEY_MAKEPAD_BROKER_H264_BROKER_PORT,
                DEFAULT_BROKER_H264_BROKER_PORT,
                1,
                u16::MAX,
            ),
            stream_port: Self::broker_h264_stream_port(eye),
            source_mode: hotload_text(
                KEY_MAKEPAD_BROKER_H264_SOURCE_MODE,
                DEFAULT_BROKER_H264_SOURCE_MODE,
            ),
            synthetic_pattern: hotload_text(
                KEY_MAKEPAD_BROKER_H264_SYNTHETIC_PATTERN,
                DEFAULT_BROKER_H264_SYNTHETIC_PATTERN,
            ),
            synthetic_projection_profile: projection_geometry_profile,
            camera_id: match eye {
                StereoEye::Left => hotload_text(
                    KEY_MAKEPAD_BROKER_H264_LEFT_CAMERA_ID,
                    DEFAULT_BROKER_H264_LEFT_CAMERA_ID,
                ),
                StereoEye::Right => hotload_text(
                    KEY_MAKEPAD_BROKER_H264_RIGHT_CAMERA_ID,
                    DEFAULT_BROKER_H264_RIGHT_CAMERA_ID,
                ),
            },
            preferred_width: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_WIDTH,
                DEFAULT_BROKER_H264_WIDTH,
                16,
                4096,
            ),
            preferred_height: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_HEIGHT,
                DEFAULT_BROKER_H264_HEIGHT,
                16,
                4096,
            ),
            capture_ms: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_CAPTURE_MS,
                DEFAULT_BROKER_H264_CAPTURE_MS,
                0,
                120_000,
            ),
            max_packets: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_MAX_PACKETS,
                DEFAULT_BROKER_H264_MAX_PACKETS,
                0,
                2400,
            ),
            bitrate_bps: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_BITRATE_BPS,
                DEFAULT_BROKER_H264_BITRATE_BPS,
                100_000,
                20_000_000,
            ),
            frame_rate_hz: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_FRAME_RATE_HZ,
                DEFAULT_BROKER_H264_FRAME_RATE_HZ,
                1,
                120,
            ),
            command_timeout_ms: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_COMMAND_TIMEOUT_MS,
                DEFAULT_BROKER_H264_COMMAND_TIMEOUT_MS,
                500,
                60_000,
            ),
            stream_timeout_ms: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_STREAM_TIMEOUT_MS,
                DEFAULT_BROKER_H264_STREAM_TIMEOUT_MS,
                500,
                120_000,
            ),
            decode_timeout_ms: hotload_u32(
                KEY_MAKEPAD_BROKER_H264_DECODE_TIMEOUT_MS,
                DEFAULT_BROKER_H264_DECODE_TIMEOUT_MS,
                500,
                60_000,
            ),
            live_stream: hotload_bool(
                KEY_MAKEPAD_BROKER_H264_LIVE_STREAM,
                DEFAULT_BROKER_H264_LIVE_STREAM,
            ),
        }
    }

    fn broker_h264_source() -> BrokerH264VideoSource {
        Self::broker_h264_source_for_eye(StereoEye::Left)
    }

    fn emit_hardware_buffer_import_marker(body: &str) {
        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 {}",
            body
        ));
    }

    fn emit_stereo_projection_marker(body: &str) {
        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_STEREO_PROJECTION schema=rusty.xr.makepad-stereo-projection.v1 {}",
            body
        ));
    }

    fn emit_cadence_marker(body: &str) {
        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_CADENCE schema=rusty.xr.makepad-cadence.v1 {}",
            body
        ));
    }

    fn arm_cadence_probe(&mut self, cx: &mut Cx) {
        self.cadence_next_frame = Some(cx.new_next_frame());
        Self::emit_cadence_marker(&format!(
            "phase=start status=started samplePeriodSeconds={:.1} appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated",
            CADENCE_SAMPLE_SECONDS
        ));
    }

    #[cfg(target_os = "android")]
    fn update_runtime_xr_projection(&mut self, update: &XrUpdateEvent) {
        let state = update.state.as_ref();
        let left = state.left_eye_view;
        let right = state.right_eye_view;
        let predicted_display_time_ns = (state.time * 1_000_000_000.0).round() as i64;
        let views = android_camera_probe::XrDisplayViews {
            left: android_camera_probe::XrDisplayEyeView {
                position: [
                    left.pose.position.x,
                    left.pose.position.y,
                    left.pose.position.z,
                ],
                orientation: [
                    left.pose.orientation.x,
                    left.pose.orientation.y,
                    left.pose.orientation.z,
                    left.pose.orientation.w,
                ],
                angle_left: left.fov.angle_left,
                angle_right: left.fov.angle_right,
                angle_up: left.fov.angle_up,
                angle_down: left.fov.angle_down,
                valid: left.valid,
            },
            right: android_camera_probe::XrDisplayEyeView {
                position: [
                    right.pose.position.x,
                    right.pose.position.y,
                    right.pose.position.z,
                ],
                orientation: [
                    right.pose.orientation.x,
                    right.pose.orientation.y,
                    right.pose.orientation.z,
                    right.pose.orientation.w,
                ],
                angle_left: right.fov.angle_left,
                angle_right: right.fov.angle_right,
                angle_up: right.fov.angle_up,
                angle_down: right.fov.angle_down,
                valid: right.valid,
            },
            predicted_display_time_ns,
            reference_space: "makepad-platform-local-space",
            projection_depth_meters: makepad_projection_depth_meters(),
            projection_preview_fov_y_degrees: makepad_projection_preview_fov_y_degrees(),
            projection_preview_offset_y_meters: makepad_projection_preview_offset_y_meters(),
            projection_raw_overscan: makepad_projection_raw_overscan(),
        };
        let updated = if Self::broker_h264_enabled() {
            self.refresh_broker_h264_projection_plan(views)
        } else {
            android_camera_probe::update_stereo_projection_from_xr_views(views)
        };
        if updated {
            self.refresh_paired_import_projection_plan();
        }
    }

    #[cfg(target_os = "android")]
    fn refresh_broker_h264_projection_plan(
        &mut self,
        views: android_camera_probe::XrDisplayViews,
    ) -> bool {
        let (Some(left_metadata), Some(right_metadata)) = (
            self.broker_h264_left_projection_metadata.as_ref(),
            self.broker_h264_right_projection_metadata.as_ref(),
        ) else {
            return false;
        };
        let Some(pair) = self.paired_import_choice.as_mut() else {
            return false;
        };
        let Some((left_width, left_height)) =
            left_metadata.ready_size(pair.left.width, pair.left.height)
        else {
            return false;
        };
        let Some((right_width, right_height)) =
            right_metadata.ready_size(pair.right.width, pair.right.height)
        else {
            return false;
        };
        if left_width != right_width || left_height != right_height {
            return false;
        }
        let full_frame_projection = left_metadata.is_full_frame_diagnostic_projection()
            && right_metadata.is_full_frame_diagnostic_projection();
        let explicit_full_frame_content_mapping = left_metadata
            .requests_explicit_full_frame_content_mapping()
            && right_metadata.requests_explicit_full_frame_content_mapping();
        let metadata_backed_projection = left_metadata.has_camera_projection_metadata()
            && right_metadata.has_camera_projection_metadata();
        let camera_projection_mapping = left_metadata.requests_camera_projection_mapping()
            && right_metadata.requests_camera_projection_mapping();
        let head_anchored_projection = left_metadata
            .requests_head_anchored_projection_area_mapping()
            && right_metadata.requests_head_anchored_projection_area_mapping();
        let camera_matched = camera_projection_mapping
            && left_metadata.projection_profile_is("camera-matched")
            && right_metadata.projection_profile_is("camera-matched");
        let Some(plan) = (if explicit_full_frame_content_mapping
            || (full_frame_projection && !metadata_backed_projection)
        {
            android_camera_probe::broker_full_frame_projection_plan_from_xr_views(
                &left_metadata.camera_id,
                &right_metadata.camera_id,
                left_width,
                left_height,
                views,
            )
            .map(Camera2StereoPlan::from)
        } else if (metadata_backed_projection && full_frame_projection) || camera_projection_mapping
        {
            left_metadata
                .android_projection_source()
                .zip(right_metadata.android_projection_source())
                .and_then(|(left_source, right_source)| {
                    android_camera_probe::broker_physical_projection_plan_from_xr_views(
                        left_source,
                        right_source,
                        left_width,
                        left_height,
                        views,
                    )
                    .map(Camera2StereoPlan::from)
                })
                .map(|mut plan| {
                    plan.left_camera_id = left_metadata.camera_id.clone();
                    plan.right_camera_id = right_metadata.camera_id.clone();
                    plan.width = left_width;
                    plan.height = left_height;
                    plan.coordinate_chain = format!(
                        "broker-h264-camera-projection-stream-header/{}",
                        plan.coordinate_chain
                    );
                    plan
                })
                .or_else(|| {
                    camera_matched.then_some(())?;
                    Self::camera2_stereo_plan().map(|mut plan| {
                        plan.left_camera_id = left_metadata.camera_id.clone();
                        plan.right_camera_id = right_metadata.camera_id.clone();
                        plan.width = left_width;
                        plan.height = left_height;
                        plan.coordinate_chain = format!(
                            "broker-h264-camera-matched-live-camera2-fallback/{}",
                            plan.coordinate_chain
                        );
                        plan.fallback_reason =
                            "camera_matched_stream_header_missing_camera_projection_metadata"
                                .to_string();
                        plan
                    })
                })
        } else if head_anchored_projection {
            android_camera_probe::broker_synthetic_projection_plan_from_xr_views(
                &left_metadata.camera_id,
                &right_metadata.camera_id,
                left_width,
                left_height,
                views,
            )
            .map(Camera2StereoPlan::from)
        } else {
            None
        }) else {
            return false;
        };
        let source_binding_mode = if full_frame_projection {
            "broker-h264-stream-header-full-frame-diagnostic"
        } else if camera_matched {
            "broker-h264-stream-header-camera-matched"
        } else if camera_projection_mapping {
            "broker-h264-stream-header-camera-projection"
        } else if head_anchored_projection {
            "broker-h264-stream-header-head-anchored"
        } else {
            "broker-h264-stream-header"
        };
        let projection_geometry_profile = if full_frame_projection {
            "full-frame-diagnostic"
        } else if camera_matched {
            "camera-matched"
        } else if camera_projection_mapping {
            left_metadata.projection_mapping_profile_id()
        } else if head_anchored_projection {
            "head-anchored-virtual-camera"
        } else {
            left_metadata.projection_geometry_profile.as_str()
        };

        pair.left.camera_id = Some(plan.left_camera_id.clone());
        pair.right.camera_id = Some(plan.right_camera_id.clone());
        pair.left.width = plan.width as usize;
        pair.left.height = plan.height as usize;
        pair.right.width = plan.width as usize;
        pair.right.height = plan.height as usize;
        pair.projection_metadata_ready =
            left_metadata.projection_metadata_ready && right_metadata.projection_metadata_ready;
        pair.projection_geometry_profile = projection_geometry_profile.to_string();
        pair.pose_source = broker_pair_pose_source(left_metadata, right_metadata);
        pair.source_eye_mapping = plan.source_eye_mapping.to_string();
        pair.source_binding_mode = source_binding_mode.to_string();
        pair.coordinate_chain = plan.coordinate_chain.to_string();
        pair.fallback_reason = plan.fallback_reason.to_string();
        pair.left_surface_to_camera_h = plan.left_surface_to_camera_h;
        pair.right_surface_to_camera_h = plan.right_surface_to_camera_h;
        pair.left_surface_to_screen_h = plan.left_surface_to_screen_h;
        pair.right_surface_to_screen_h = plan.right_surface_to_screen_h;
        pair.left_screen_to_camera_h = plan.left_screen_to_camera_h;
        pair.right_screen_to_camera_h = plan.right_screen_to_camera_h;
        pair.left_screen_to_surface_h = plan.left_screen_to_surface_h;
        pair.right_screen_to_surface_h = plan.right_screen_to_surface_h;
        pair.left_source_valid_uv_rect = left_metadata.source_valid_uv_rect;
        pair.right_source_valid_uv_rect = right_metadata.source_valid_uv_rect;
        pair.projection_homography_ready = plan.projection_homography_ready;
        pair.runtime_xr_view_state_ready = plan.runtime_xr_view_state_ready;
        pair.openxr_contract = plan.openxr_contract.clone();
        if !self.broker_h264_projection_plan_logged {
            self.broker_h264_projection_plan_logged = true;
            let config = Self::runtime_config();
            Self::emit_projection_runtime_manifest_marker(
                "broker-h264-projection-plan",
                &config,
                Self::horizontal_alignment_tuning(),
            );
            Self::emit_stereo_projection_marker(&broker_projection_plan_marker_fields(
                pair,
                &plan,
                left_metadata,
                right_metadata,
            ));
        }
        true
    }

    #[cfg(target_os = "android")]
    fn refresh_paired_import_projection_plan(&mut self) {
        let Some(plan) = Self::camera2_stereo_plan() else {
            return;
        };
        let Some(pair) = self.paired_import_choice.as_mut() else {
            return;
        };
        if !pair.matches_camera2_plan(&plan)
            || pair.left.width != plan.width as usize
            || pair.left.height != plan.height as usize
            || pair.right.width != plan.width as usize
            || pair.right.height != plan.height as usize
        {
            return;
        }

        pair.projection_metadata_ready = plan.projection_metadata_ready;
        pair.pose_source = plan.pose_source;
        pair.source_eye_mapping = makepad_display_source_eye_mapping().to_string();
        pair.coordinate_chain = plan.coordinate_chain;
        pair.fallback_reason = plan.fallback_reason;
        pair.left_surface_to_camera_h = plan.left_surface_to_camera_h;
        pair.right_surface_to_camera_h = plan.right_surface_to_camera_h;
        pair.left_surface_to_screen_h = plan.left_surface_to_screen_h;
        pair.right_surface_to_screen_h = plan.right_surface_to_screen_h;
        pair.left_screen_to_camera_h = plan.left_screen_to_camera_h;
        pair.right_screen_to_camera_h = plan.right_screen_to_camera_h;
        pair.left_screen_to_surface_h = plan.left_screen_to_surface_h;
        pair.right_screen_to_surface_h = plan.right_screen_to_surface_h;
        pair.projection_homography_ready = plan.projection_homography_ready;
        pair.runtime_xr_view_state_ready = plan.runtime_xr_view_state_ready;
        pair.openxr_contract = plan.openxr_contract.clone();
    }

    fn horizontal_alignment_tuning() -> HorizontalAlignmentTuning {
        let legacy = Self::legacy_horizontal_alignment_tuning();
        if !makepad_projection_runtime_resolution_enabled() {
            return legacy;
        }

        let config = Self::runtime_config();
        let runtime = makepad_projection_runtime_resolution(&config, legacy);
        makepad_horizontal_alignment_tuning_from_resolution(legacy, &runtime.resolution)
    }

    fn legacy_horizontal_alignment_tuning() -> HorizontalAlignmentTuning {
        let strength = hotload_f32(
            KEY_MAKEPAD_HORIZONTAL_ALIGNMENT_STRENGTH,
            TARGET_HORIZONTAL_ALIGNMENT_STRENGTH,
            -4.0,
            4.0,
        );
        let global_offset = hotload_f32(KEY_MAKEPAD_HORIZONTAL_OFFSET_UV, 0.0, -0.5, 0.5);
        let left_offset = global_offset
            + hotload_f32(
                KEY_MAKEPAD_HORIZONTAL_OFFSET_LEFT_UV,
                TARGET_MANUAL_HORIZONTAL_OFFSET_LEFT_UV,
                -0.5,
                0.5,
            );
        let right_offset = global_offset
            + hotload_f32(
                KEY_MAKEPAD_HORIZONTAL_OFFSET_RIGHT_UV,
                TARGET_MANUAL_HORIZONTAL_OFFSET_RIGHT_UV,
                -0.5,
                0.5,
            );
        let vertical_offset = hotload_f32(
            KEY_MAKEPAD_VERTICAL_OFFSET_UV,
            TARGET_MANUAL_VERTICAL_OFFSET_UV,
            -0.5,
            0.5,
        );
        let content_uv_scale = hotload_f32(
            KEY_MAKEPAD_CONTENT_UV_SCALE,
            TARGET_FULL_VIEW_CONTENT_UV_SCALE,
            1.0,
            2.4,
        );
        let projection_border_opacity = makepad_projection_border_opacity();
        let projection_border_policy = MakepadProjectionBorderPolicy::current().shader_code();
        let processing_layer = MakepadProcessingLayer::current().shader_code();
        let blur_radius_px = makepad_blur_radius_px();
        let projection_area_diagnostic = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_DIAGNOSTIC,
            TARGET_PROJECTION_AREA_DIAGNOSTIC,
            0.0,
            2.0,
        );
        let projection_area_offset_left_uv = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_OFFSET_LEFT_UV,
            TARGET_PROJECTION_AREA_OFFSET_LEFT_UV,
            -0.5,
            0.5,
        );
        let projection_area_offset_right_uv = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_OFFSET_RIGHT_UV,
            TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV,
            -0.5,
            0.5,
        );
        let projection_area_offset_vertical_uv = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_OFFSET_VERTICAL_UV,
            TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV,
            -0.5,
            0.5,
        );
        let projection_area_scale_x = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_SCALE_X,
            TARGET_PROJECTION_AREA_SCALE_X,
            0.5,
            1.5,
        );
        let projection_area_scale_y = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_SCALE_Y,
            TARGET_PROJECTION_AREA_SCALE_Y,
            0.5,
            1.5,
        );
        let projection_area_radius_x_uv = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_RADIUS_X_UV,
            TARGET_PROJECTION_AREA_RADIUS_X_UV,
            0.05,
            0.5,
        );
        let projection_area_radius_y_uv = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_RADIUS_Y_UV,
            TARGET_PROJECTION_AREA_RADIUS_Y_UV,
            0.05,
            0.5,
        );
        let projection_area_corner_radius_uv = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_CORNER_RADIUS_UV,
            TARGET_PROJECTION_AREA_CORNER_RADIUS_UV,
            0.0,
            0.5,
        );
        let projection_area_keystone_x = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_KEYSTONE_X,
            TARGET_PROJECTION_AREA_KEYSTONE_X,
            -0.45,
            0.45,
        );
        let projection_area_bow_x = hotload_f32(
            KEY_MAKEPAD_PROJECTION_AREA_BOW_X,
            TARGET_PROJECTION_AREA_BOW_X,
            -0.25,
            0.25,
        );
        let projection_area_opacity = makepad_projection_area_opacity();
        let projection_alpha_mode = MakepadProjectionAlphaMode::current().shader_code();
        let projection_alpha_scale = makepad_projection_alpha_scale();
        let projection_alpha_bias = makepad_projection_alpha_bias();
        HorizontalAlignmentTuning {
            strength,
            left_offset_uv: left_offset.clamp(-0.5, 0.5),
            right_offset_uv: right_offset.clamp(-0.5, 0.5),
            vertical_offset_uv: vertical_offset,
            content_uv_scale,
            projection_border_opacity,
            projection_border_policy,
            processing_layer,
            blur_radius_px,
            projection_area_diagnostic,
            projection_area_offset_left_uv,
            projection_area_offset_right_uv,
            projection_area_offset_vertical_uv,
            projection_area_scale_x,
            projection_area_scale_y,
            projection_area_radius_x_uv,
            projection_area_radius_y_uv,
            projection_area_corner_radius_uv,
            projection_area_keystone_x,
            projection_area_bow_x,
            projection_area_opacity,
            projection_alpha_mode,
            projection_alpha_scale,
            projection_alpha_bias,
        }
    }

    fn current_horizontal_alignment_tuning(&self) -> HorizontalAlignmentTuning {
        if self.horizontal_alignment_tuning_ready {
            HorizontalAlignmentTuning {
                strength: self.horizontal_alignment_strength,
                left_offset_uv: self.manual_horizontal_offset_left_uv,
                right_offset_uv: self.manual_horizontal_offset_right_uv,
                vertical_offset_uv: self.manual_vertical_offset_uv,
                content_uv_scale: self.content_uv_scale,
                projection_border_opacity: self.projection_border_opacity,
                projection_border_policy: self.projection_border_policy,
                processing_layer: self.processing_layer,
                blur_radius_px: self.blur_radius_px,
                projection_area_diagnostic: self.projection_area_diagnostic,
                projection_area_offset_left_uv: self.projection_area_offset_left_uv,
                projection_area_offset_right_uv: self.projection_area_offset_right_uv,
                projection_area_offset_vertical_uv: self.projection_area_offset_vertical_uv,
                projection_area_scale_x: self.projection_area_scale_x,
                projection_area_scale_y: self.projection_area_scale_y,
                projection_area_radius_x_uv: self.projection_area_radius_x_uv,
                projection_area_radius_y_uv: self.projection_area_radius_y_uv,
                projection_area_corner_radius_uv: self.projection_area_corner_radius_uv,
                projection_area_keystone_x: self.projection_area_keystone_x,
                projection_area_bow_x: self.projection_area_bow_x,
                projection_area_opacity: self.projection_area_opacity,
                projection_alpha_mode: self.projection_alpha_mode,
                projection_alpha_scale: self.projection_alpha_scale,
                projection_alpha_bias: self.projection_alpha_bias,
            }
        } else {
            HorizontalAlignmentTuning::default()
        }
    }

    fn refresh_horizontal_alignment_tuning(&mut self, cx: &mut Cx) {
        let tuning = Self::horizontal_alignment_tuning();
        let changed = !self.horizontal_alignment_tuning_ready
            || (self.horizontal_alignment_strength - tuning.strength).abs() > 0.0001
            || (self.manual_horizontal_offset_left_uv - tuning.left_offset_uv).abs() > 0.0001
            || (self.manual_horizontal_offset_right_uv - tuning.right_offset_uv).abs() > 0.0001
            || (self.manual_vertical_offset_uv - tuning.vertical_offset_uv).abs() > 0.0001
            || (self.content_uv_scale - tuning.content_uv_scale).abs() > 0.0001
            || (self.projection_border_opacity - tuning.projection_border_opacity).abs() > 0.0001
            || (self.projection_border_policy - tuning.projection_border_policy).abs() > 0.0001
            || (self.processing_layer - tuning.processing_layer).abs() > 0.0001
            || (self.blur_radius_px - tuning.blur_radius_px).abs() > 0.0001
            || (self.projection_area_diagnostic - tuning.projection_area_diagnostic).abs() > 0.0001
            || (self.projection_area_offset_left_uv - tuning.projection_area_offset_left_uv).abs()
                > 0.0001
            || (self.projection_area_offset_right_uv - tuning.projection_area_offset_right_uv)
                .abs()
                > 0.0001
            || (self.projection_area_offset_vertical_uv
                - tuning.projection_area_offset_vertical_uv)
                .abs()
                > 0.0001
            || (self.projection_area_scale_x - tuning.projection_area_scale_x).abs() > 0.0001
            || (self.projection_area_scale_y - tuning.projection_area_scale_y).abs() > 0.0001
            || (self.projection_area_radius_x_uv - tuning.projection_area_radius_x_uv).abs()
                > 0.0001
            || (self.projection_area_radius_y_uv - tuning.projection_area_radius_y_uv).abs()
                > 0.0001
            || (self.projection_area_corner_radius_uv - tuning.projection_area_corner_radius_uv)
                .abs()
                > 0.0001
            || (self.projection_area_keystone_x - tuning.projection_area_keystone_x).abs() > 0.0001
            || (self.projection_area_bow_x - tuning.projection_area_bow_x).abs() > 0.0001
            || (self.projection_area_opacity - tuning.projection_area_opacity).abs() > 0.0001
            || (self.projection_alpha_mode - tuning.projection_alpha_mode).abs() > 0.0001
            || (self.projection_alpha_scale - tuning.projection_alpha_scale).abs() > 0.0001
            || (self.projection_alpha_bias - tuning.projection_alpha_bias).abs() > 0.0001;
        if !changed {
            return;
        }

        self.horizontal_alignment_tuning_ready = true;
        self.horizontal_alignment_strength = tuning.strength;
        self.manual_horizontal_offset_left_uv = tuning.left_offset_uv;
        self.manual_horizontal_offset_right_uv = tuning.right_offset_uv;
        self.manual_vertical_offset_uv = tuning.vertical_offset_uv;
        self.content_uv_scale = tuning.content_uv_scale;
        self.projection_border_opacity = tuning.projection_border_opacity;
        self.projection_border_policy = tuning.projection_border_policy;
        self.processing_layer = tuning.processing_layer;
        self.blur_radius_px = tuning.blur_radius_px;
        self.projection_area_diagnostic = tuning.projection_area_diagnostic;
        self.projection_area_offset_left_uv = tuning.projection_area_offset_left_uv;
        self.projection_area_offset_right_uv = tuning.projection_area_offset_right_uv;
        self.projection_area_offset_vertical_uv = tuning.projection_area_offset_vertical_uv;
        self.projection_area_scale_x = tuning.projection_area_scale_x;
        self.projection_area_scale_y = tuning.projection_area_scale_y;
        self.projection_area_radius_x_uv = tuning.projection_area_radius_x_uv;
        self.projection_area_radius_y_uv = tuning.projection_area_radius_y_uv;
        self.projection_area_corner_radius_uv = tuning.projection_area_corner_radius_uv;
        self.projection_area_keystone_x = tuning.projection_area_keystone_x;
        self.projection_area_bow_x = tuning.projection_area_bow_x;
        self.projection_area_opacity = tuning.projection_area_opacity;
        self.projection_alpha_mode = tuning.projection_alpha_mode;
        self.projection_alpha_scale = tuning.projection_alpha_scale;
        self.projection_alpha_bias = tuning.projection_alpha_bias;
        let panel_bound = self.apply_horizontal_alignment_tuning_to_panel(cx, tuning);
        Self::emit_stereo_projection_marker(&format!(
            "phase=horizontal-alignment-hotload status=applied s105HotloadHorizontalAlignmentControl=true s106SafeHorizontalWindowSampling=true s107WindowScaleHotload=true s108BorderlessWindowScale=false s109SolidRedProjectionExterior=true s110VerticalWindowOffsetHotload=true s111ProjectionAreaDiagnostic=true s112ProjectionAreaScreenOffset=true s113ProjectionAreaScreenScale=true s114ProjectionAreaFootprintOnlyDiagnostic=true s115ProjectionAreaKeystone=true s116ProjectionAreaMidpointBow=true s117PreHomographyDiagnosticOnly=true s118ProjectedFootprintLiveWindow=true s119ProcessingLayerHotload=true s120ProjectionAreaOpacityHotload=true s121ProjectionAreaRoundedMaskHotload=true s122ProjectionAlphaMaskHotload=true horizontalAlignmentSource=screen_to_camera_center_delta_projection_area_source_valid_window manualHorizontalOffsetHotload=true verticalOffsetHotload=true contentUvScaleHotload=true projectionBorderOpacityHotload=true projectionBorderPolicyHotload=true processingLayerHotload=true projectionAreaDiagnosticHotload=true projectionAreaScreenOffsetHotload=true projectionAreaScreenScaleHotload=true projectionAreaRoundedMaskHotload=true projectionAreaKeystoneHotload=true projectionAreaBowHotload=true projectionAreaOpacityHotload=true projectionAlphaMaskHotload=true projectionAreaTransformStage=pre_homography_screen_uv borderlessWindowMask=false solidRedProjectionExterior={} propertyPrefix=debug.rustyxr {} projectionAreaDiagnosticMode=0_off_1_full_2_footprint_only horizontalAlignmentStrength={:.4} manualLeftUv={:.4} manualRightUv={:.4} manualVerticalUv={:.4} contentUvScale={:.4} projectionBorderOpacity={:.4} projectionAreaOpacity={:.4} projectionAlphaMode={} projectionAlphaScale={:.4} projectionAlphaBias={:.4} processingLayer={} blurRadiusPx={:.2} projectionAreaDiagnostic={:.1} projectionAreaLeftUv={:.4} projectionAreaRightUv={:.4} projectionAreaVerticalUv={:.4} projectionAreaScaleX={:.4} projectionAreaScaleY={:.4} projectionAreaRadiusXUv={:.4} projectionAreaRadiusYUv={:.4} projectionAreaCornerRadiusUv={:.4} projectionAreaKeystoneX={:.4} projectionAreaBowX={:.4} panelBound={} visualInspection=required",
            tuning.projection_border_opacity > 0.0001,
            makepad_projection_target_marker_fields(),
            tuning.strength,
            tuning.left_offset_uv,
            tuning.right_offset_uv,
            tuning.vertical_offset_uv,
            tuning.content_uv_scale,
            tuning.projection_border_opacity,
            tuning.projection_area_opacity,
            MakepadProjectionAlphaMode::current().stable_id(),
            tuning.projection_alpha_scale,
            tuning.projection_alpha_bias,
            MakepadProcessingLayer::current().stable_id(),
            tuning.blur_radius_px,
            tuning.projection_area_diagnostic,
            tuning.projection_area_offset_left_uv,
            tuning.projection_area_offset_right_uv,
            tuning.projection_area_offset_vertical_uv,
            tuning.projection_area_scale_x,
            tuning.projection_area_scale_y,
            tuning.projection_area_radius_x_uv,
            tuning.projection_area_radius_y_uv,
            tuning.projection_area_corner_radius_uv,
            tuning.projection_area_keystone_x,
            tuning.projection_area_bow_x,
            panel_bound,
        ));
    }

    fn apply_horizontal_alignment_tuning_to_panel(
        &mut self,
        cx: &mut Cx,
        tuning: HorizontalAlignmentTuning,
    ) -> bool {
        let panel_ref = self.ui.widget(cx, ids!(camera_projection_panel));
        let Some(mut panel) = panel_ref.borrow_mut::<MakepadStereoCameraPanel>() else {
            return false;
        };
        panel.set_horizontal_alignment_tuning(cx, tuning);
        true
    }

    fn handle_cadence_event(&mut self, cx: &mut Cx, event: &Event) {
        if matches!(event, Event::Startup | Event::XrUpdate(_)) {
            self.refresh_horizontal_alignment_tuning(cx);
        }

        if matches!(event, Event::Startup) && self.cadence_next_frame.is_none() {
            self.arm_cadence_probe(cx);
            return;
        }

        match event {
            Event::XrUpdate(_update) => {
                self.cadence_xr_update_count = self.cadence_xr_update_count.saturating_add(1);
                #[cfg(target_os = "android")]
                self.update_runtime_xr_projection(_update);
            }
            Event::Draw(_) => {
                self.cadence_draw_event_count = self.cadence_draw_event_count.saturating_add(1);
            }
            _ => {}
        }

        let Some(next_frame) = self.cadence_next_frame else {
            return;
        };
        let Some(next_frame_event) = next_frame.is_event(event) else {
            return;
        };

        if !self.cadence_started {
            self.cadence_started = true;
            self.cadence_start_time = next_frame_event.time;
            self.cadence_last_sample_time = next_frame_event.time;
        }

        self.cadence_frame_count = self.cadence_frame_count.saturating_add(1);
        let interval_seconds = (next_frame_event.time - self.cadence_last_sample_time).max(0.0);
        if interval_seconds >= CADENCE_SAMPLE_SECONDS {
            self.emit_cadence_sample(next_frame_event.time, interval_seconds);
        }

        self.cadence_next_frame = Some(cx.new_next_frame());
    }

    fn record_camera_texture_update(&mut self, side: StereoEye, position_ms: u128) {
        match side {
            StereoEye::Left => {
                self.cadence_left_texture_update_count =
                    self.cadence_left_texture_update_count.saturating_add(1);
                self.cadence_left_last_position_ms = position_ms;
            }
            StereoEye::Right => {
                self.cadence_right_texture_update_count =
                    self.cadence_right_texture_update_count.saturating_add(1);
                self.cadence_right_last_position_ms = position_ms;
            }
        }
    }

    fn emit_cadence_sample(&mut self, now_seconds: f64, interval_seconds: f64) {
        let elapsed_seconds = (now_seconds - self.cadence_start_time).max(0.0);
        let frame_delta = self
            .cadence_frame_count
            .saturating_sub(self.cadence_frame_count_at_last_sample);
        let left_delta = self
            .cadence_left_texture_update_count
            .saturating_sub(self.cadence_left_texture_update_count_at_last_sample);
        let right_delta = self
            .cadence_right_texture_update_count
            .saturating_sub(self.cadence_right_texture_update_count_at_last_sample);
        let xr_update_delta = self
            .cadence_xr_update_count
            .saturating_sub(self.cadence_xr_update_count_at_last_sample);
        let draw_event_delta = self
            .cadence_draw_event_count
            .saturating_sub(self.cadence_draw_event_count_at_last_sample);
        let paired_delta = left_delta.min(right_delta);
        let app_frame_rate_hz = rate_hz(frame_delta, interval_seconds);
        let xr_update_rate_hz = rate_hz(xr_update_delta, interval_seconds);
        let draw_event_rate_hz = rate_hz(draw_event_delta, interval_seconds);
        let left_texture_rate_hz = rate_hz(left_delta, interval_seconds);
        let right_texture_rate_hz = rate_hz(right_delta, interval_seconds);
        let paired_texture_rate_hz = rate_hz(paired_delta, interval_seconds);
        let paired_buffers_ready =
            self.paired_import_left_updated && self.paired_import_right_updated;
        let projection_ready = self
            .paired_import_choice
            .as_ref()
            .map(|pair| pair.projection_homography_ready)
            .unwrap_or(false);
        let (projection_mapping_ready, aligned_projection) = if paired_buffers_ready {
            (projection_ready, projection_ready)
        } else {
            (false, false)
        };

        Self::emit_cadence_marker(&format!(
            "phase=sample status=ok elapsedMs={:.0} intervalMs={:.0} appFrameCount={} appFrameDelta={} appFrameRateHz={:.2} xrUpdateCount={} xrUpdateDelta={} xrUpdateRateHz={:.2} drawEventCount={} drawEventDelta={} drawEventRateHz={:.2} leftTextureUpdateCount={} rightTextureUpdateCount={} pairedTextureUpdateCount={} leftTextureUpdateDelta={} rightTextureUpdateDelta={} pairedTextureUpdateDelta={} leftTextureUpdateRateHz={:.2} rightTextureUpdateRateHz={:.2} pairedTextureUpdateRateHz={:.2} leftLastPositionMs={} rightLastPositionMs={} pairedLeftRightCameraFrames={} projectionMappingReady={} alignedProjection={} visibleCameraProjectionReady={} cpuUploadPath=makepad-camera-cpu-yuv-plane renderPath=makepad-xr appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated",
            elapsed_seconds * 1000.0,
            interval_seconds * 1000.0,
            self.cadence_frame_count,
            frame_delta,
            app_frame_rate_hz,
            self.cadence_xr_update_count,
            xr_update_delta,
            xr_update_rate_hz,
            self.cadence_draw_event_count,
            draw_event_delta,
            draw_event_rate_hz,
            self.cadence_left_texture_update_count,
            self.cadence_right_texture_update_count,
            self.cadence_left_texture_update_count.min(self.cadence_right_texture_update_count),
            left_delta,
            right_delta,
            paired_delta,
            left_texture_rate_hz,
            right_texture_rate_hz,
            paired_texture_rate_hz,
            self.cadence_left_last_position_ms,
            self.cadence_right_last_position_ms,
            paired_buffers_ready,
            projection_mapping_ready,
            aligned_projection,
            self.camera_projection_textures_bound,
        ));

        self.cadence_last_sample_time = now_seconds;
        self.cadence_frame_count_at_last_sample = self.cadence_frame_count;
        self.cadence_xr_update_count_at_last_sample = self.cadence_xr_update_count;
        self.cadence_draw_event_count_at_last_sample = self.cadence_draw_event_count;
        self.cadence_left_texture_update_count_at_last_sample =
            self.cadence_left_texture_update_count;
        self.cadence_right_texture_update_count_at_last_sample =
            self.cadence_right_texture_update_count;
    }

    fn arm_paired_import_timer(&mut self, cx: &mut Cx, delay_seconds: f64, reason: &str) {
        if self.paired_import_finished {
            return;
        }
        self.paired_import_timer = cx.start_timeout(delay_seconds);
        PAIRED_IMPORT_SIGNAL_READY.store(false, Ordering::Release);
        thread::spawn(move || {
            thread::sleep(Duration::from_secs_f64(delay_seconds.max(0.0)));
            PAIRED_IMPORT_SIGNAL_READY.store(true, Ordering::Release);
            SignalToUI::set_ui_signal();
        });
        Self::emit_hardware_buffer_import_marker(&format!(
            "phase=timer status=armed reason={} delaySeconds={:.1} signalFallback=true importPlan=paired-makepad-video-hardware-buffer",
            marker_token(reason),
            delay_seconds,
        ));
    }

    fn handle_broker_h264_projection_metadata(&mut self, video_id: LiveId, metadata_json: &str) {
        emit_raw_video_event_marker("metadata", video_id);
        if !Self::broker_h264_enabled() {
            return;
        }
        let Some(side) = StereoEye::from_video_id(video_id) else {
            Self::emit_hardware_buffer_import_marker(&format!(
                "phase=stream-header-metadata status=ignored side=unknown videoId={} reason=unexpected_video_id importPlan=broker-h264-stereo-mediacodec-yuv-texture",
                video_id.0,
            ));
            return;
        };
        match BrokerH264ProjectionMetadata::parse(metadata_json) {
            Ok(metadata) => {
                match side {
                    StereoEye::Left => {
                        self.broker_h264_left_projection_metadata = Some(metadata.clone())
                    }
                    StereoEye::Right => {
                        self.broker_h264_right_projection_metadata = Some(metadata.clone())
                    }
                }
                if let Some(pair) = self.paired_import_choice.as_mut() {
                    match side {
                        StereoEye::Left => {
                            pair.left.camera_id = Some(metadata.camera_id.clone());
                            if metadata.delivered_width > 0 {
                                pair.left.width = metadata.delivered_width as usize;
                            }
                            if metadata.delivered_height > 0 {
                                pair.left.height = metadata.delivered_height as usize;
                            }
                        }
                        StereoEye::Right => {
                            pair.right.camera_id = Some(metadata.camera_id.clone());
                            if metadata.delivered_width > 0 {
                                pair.right.width = metadata.delivered_width as usize;
                            }
                            if metadata.delivered_height > 0 {
                                pair.right.height = metadata.delivered_height as usize;
                            }
                        }
                    }
                    pair.projection_metadata_ready = self
                        .broker_h264_left_projection_metadata
                        .as_ref()
                        .is_some_and(|metadata| metadata.projection_metadata_ready)
                        && self
                            .broker_h264_right_projection_metadata
                            .as_ref()
                            .is_some_and(|metadata| metadata.projection_metadata_ready);
                    pair.pose_source = match (
                        self.broker_h264_left_projection_metadata.as_ref(),
                        self.broker_h264_right_projection_metadata.as_ref(),
                    ) {
                        (Some(left), Some(right)) => broker_pair_pose_source(left, right),
                        _ => metadata.pose_source.clone(),
                    };
                    pair.source_binding_mode = "broker-h264-stream-header".to_string();
                    pair.coordinate_chain =
                        "broker-h264-stream-header-to-runtime-openxr-view".to_string();
                    pair.fallback_reason = if pair.projection_metadata_ready {
                        "waiting_for_runtime_xr_view_projection".to_string()
                    } else {
                        "broker_stream_metadata_not_projection_ready".to_string()
                    };
                }
                Self::emit_hardware_buffer_import_marker(&stream_header_metadata_marker_fields(
                    side.label(),
                    &metadata,
                ));
            }
            Err(error) => {
                Self::emit_hardware_buffer_import_marker(&format!(
                    "phase=stream-header-metadata status=error side={} metadataBytes={} error={} importPlan=broker-h264-stereo-mediacodec-yuv-texture",
                    side.label(),
                    metadata_json.len(),
                    marker_token(&error),
                ));
            }
        }
    }

    fn handle_paired_import_event(&mut self, cx: &mut Cx, event: &Event) {
        match event {
            Event::Startup => {
                if Self::broker_h264_enabled() {
                    let source = Self::broker_h264_source();
                    self.paired_import_choice =
                        Some(MakepadCameraPair::from_broker_h264_source(&source));
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=startup status=broker-h264-enabled brokerHost={} brokerPort={} leftStreamPort={} rightStreamPort={} sourceMode={} syntheticPattern={} preferredWidth={} preferredHeight={} liveStream={} importPlan=broker-h264-stereo-mediacodec-yuv-texture",
                        marker_token(&source.broker_host),
                        source.broker_port,
                        source.stream_port,
                        Self::broker_h264_stream_port(StereoEye::Right),
                        marker_token(&source.source_mode),
                        marker_token(&source.synthetic_pattern),
                        source.preferred_width,
                        source.preferred_height,
                        source.live_stream,
                    ));
                } else {
                    cx.request_permission(Permission::Camera);
                    cx.request_permission(Permission::HeadsetCamera);
                }
                self.arm_paired_import_timer(cx, PAIRED_IMPORT_DELAY_SECONDS, "startup");
            }
            Event::VideoInputs(inputs) => {
                if Self::broker_h264_enabled() {
                    return;
                }
                self.paired_import_choice = Self::pick_makepad_camera_pair(inputs);
                if !self.paired_import_selection_logged {
                    self.paired_import_selection_logged = true;
                    self.emit_makepad_camera_selection_marker(inputs);
                }
                if self.paired_import_timer.is_empty()
                    && !self.paired_import_started
                    && !self.paired_import_finished
                {
                    self.arm_paired_import_timer(cx, PAIRED_IMPORT_DELAY_SECONDS, "video-inputs");
                }
            }
            Event::TextureHandleReady(ready) => {
                self.maybe_prepare_broker_h264_import(cx, ready);
            }
            Event::VideoYuvTexturesReady(ready) => {
                emit_raw_video_event_marker("yuv-textures-ready", ready.video_id);
                if let Some(side) = StereoEye::from_video_id(ready.video_id) {
                    if Self::broker_h264_enabled() {
                        let textures = MakepadCameraYuvTextures::new(
                            ready.tex_y.clone(),
                            ready.tex_u.clone(),
                            ready.tex_v.clone(),
                        );
                        match side {
                            StereoEye::Left => {
                                self.paired_import_left_yuv_textures = Some(textures)
                            }
                            StereoEye::Right => {
                                self.paired_import_right_yuv_textures = Some(textures)
                            }
                        }
                        self.camera_projection_textures_bound = false;
                        self.camera_projection_paired_textures_bound = false;
                        Self::emit_hardware_buffer_import_marker(&format!(
                            "phase=yuv-textures-ready status=ok side={} textureMode=cpu-yuv-decoded-broker-h264 importPlan=broker-h264-stereo-mediacodec-yuv-texture",
                            side.label(),
                        ));
                        self.bind_camera_projection_panel(cx);
                        return;
                    }
                    let textures = MakepadCameraYuvTextures::new(
                        ready.tex_y.clone(),
                        ready.tex_u.clone(),
                        ready.tex_v.clone(),
                    );
                    match side {
                        StereoEye::Left => self.paired_import_left_yuv_textures = Some(textures),
                        StereoEye::Right => self.paired_import_right_yuv_textures = Some(textures),
                    }
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=yuv-textures-ready status=ok side={} textureMode=makepad-yuv-plane visualProofPath=single-stream-yuv-proof depthClip=false environmentDepthClip=false",
                        side.label(),
                    ));
                }
            }
            Event::VideoPlaybackMetadata(metadata) => {
                self.handle_broker_h264_projection_metadata(
                    metadata.video_id,
                    &metadata.metadata_json,
                );
                self.emit_paired_projection_progress("stream-header-metadata");
            }
            Event::VideoPlaybackPrepared(prepared) => {
                emit_raw_video_event_marker("prepared", prepared.video_id);
                if let Some(side) = StereoEye::from_video_id(prepared.video_id) {
                    match side {
                        StereoEye::Left => self.paired_import_left_prepared = true,
                        StereoEye::Right => self.paired_import_right_prepared = true,
                    }
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=prepared status=ok side={} width={} height={} importPath={} textureMode={} importPlan={}",
                        side.label(),
                        prepared.video_width,
                        prepared.video_height,
                        if Self::broker_h264_enabled() {
                            "broker-h264-mediacodec-cpu-yuv"
                        } else {
                            "makepad-android-camera-yuv-plane-cpu-proof"
                        },
                        if Self::broker_h264_enabled() {
                            "cpu-yuv"
                        } else {
                            "yuv-plane"
                        },
                        if Self::broker_h264_enabled() {
                            "broker-h264-stereo-mediacodec-yuv-texture"
                        } else {
                            "single-stream-yuv-proof"
                        },
                    ));
                    self.emit_paired_projection_progress("prepared");
                }
            }
            Event::VideoTextureUpdated(updated) => {
                emit_raw_video_event_marker("texture-updated", updated.video_id);
                if let Some(side) = StereoEye::from_video_id(updated.video_id) {
                    self.record_camera_texture_update(side, updated.current_position_ms);
                    if !Self::broker_h264_enabled() {
                        self.emit_yuv_texture_content_probe(cx, side, updated.yuv);
                    }
                    if self.paired_import_finished {
                        self.bind_camera_projection_panel(cx);
                        return;
                    }
                    match side {
                        StereoEye::Left => {
                            self.paired_import_left_updated = true;
                            self.paired_import_left_rotation_steps = updated.yuv.rotation_steps;
                        }
                        StereoEye::Right => {
                            self.paired_import_right_updated = true;
                            self.paired_import_right_rotation_steps = updated.yuv.rotation_steps;
                        }
                    }
                    if TEXTURE_UPDATE_MARKERS_EMITTED.fetch_add(1, Ordering::AcqRel)
                        < TEXTURE_UPDATE_MARKER_LIMIT
                    {
                        Self::emit_hardware_buffer_import_marker(&format!(
                            "phase=texture-updated status=ok side={} makepadVulkanImport=false yuvEnabled={} yuvBiplanar={} rotationSteps={:.0} importPlan={} cpuUploadPath={}",
                            side.label(),
                            updated.yuv.enabled,
                            updated.yuv.biplanar,
                            updated.yuv.rotation_steps,
                            if Self::broker_h264_enabled() {
                                "broker-h264-stereo-mediacodec-yuv-texture"
                            } else {
                                "single-stream-yuv-proof"
                            },
                            if Self::broker_h264_enabled() {
                                "broker-h264-mediacodec-cpu-yuv"
                            } else {
                                "makepad-camera-cpu-yuv-plane"
                            },
                        ));
                    }
                    self.complete_paired_import_if_ready(cx);
                }
            }
            Event::VideoDecodingError(error) => {
                emit_raw_video_event_marker("decode-error", error.video_id);
                if let Some(side) = StereoEye::from_video_id(error.video_id) {
                    self.paired_import_finished = true;
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=complete status=error side={} errorKind=makepad_video_import_failed message={}",
                        side.label(),
                        marker_token(&error.error),
                    ));
                    Self::emit_stereo_projection_marker(&format!(
                        "phase=complete status=error side={} pairedLeftRightGpuBuffers=false projectionMappingReady=false alignedProjection=false fallbackReason=makepad_video_import_failed",
                        side.label()
                    ));
                }
            }
            _ => {}
        }

        if !self.paired_import_timer.is_empty()
            && self.paired_import_timer.is_event(event).is_some()
        {
            self.paired_import_timer = Timer::empty();
            Self::emit_hardware_buffer_import_marker(&format!(
                "phase=timer status=fired source=makepad-timer hasPair={} importStarted={} importFinished={} importPlan=paired-makepad-video-hardware-buffer",
                self.paired_import_choice.is_some(),
                self.paired_import_started,
                self.paired_import_finished,
            ));
            self.try_start_paired_import(cx);
        }

        if !self.paired_import_timer.is_empty()
            && matches!(event, Event::Signal)
            && PAIRED_IMPORT_SIGNAL_READY.swap(false, Ordering::AcqRel)
        {
            self.paired_import_timer = Timer::empty();
            Self::emit_hardware_buffer_import_marker(&format!(
                "phase=timer status=fired source=signal-fallback hasPair={} importStarted={} importFinished={} importPlan=paired-makepad-video-hardware-buffer",
                self.paired_import_choice.is_some(),
                self.paired_import_started,
                self.paired_import_finished,
            ));
            self.try_start_paired_import(cx);
        }

        if !self.native_video_widget_retry_timer.is_empty()
            && self
                .native_video_widget_retry_timer
                .is_event(event)
                .is_some()
        {
            self.native_video_widget_retry_timer = Timer::empty();
            if let Some(pair) = self.native_video_widget_retry_pair.clone() {
                if self.start_native_video_widget_surface(cx, &pair) {
                    self.paired_import_finished = true;
                    self.native_video_widget_retry_pair = None;
                }
            }
        }
    }

    fn maybe_prepare_broker_h264_import(&mut self, cx: &mut Cx, ready: &TextureHandleReadyEvent) {
        if !Self::broker_h264_enabled() {
            return;
        }

        let left_texture_id = self
            .paired_import_left_texture
            .as_ref()
            .map(Texture::texture_id);
        let right_texture_id = self
            .paired_import_right_texture
            .as_ref()
            .map(Texture::texture_id);
        let side = if Some(ready.texture_id) == left_texture_id {
            StereoEye::Left
        } else if Some(ready.texture_id) == right_texture_id {
            StereoEye::Right
        } else {
            return;
        };

        let already_requested = match side {
            StereoEye::Left => self.broker_h264_left_playback_requested,
            StereoEye::Right => self.broker_h264_right_playback_requested,
        };
        if already_requested {
            return;
        }

        let source = Self::broker_h264_source_for_eye(side);
        match side {
            StereoEye::Left => self.broker_h264_left_playback_requested = true,
            StereoEye::Right => self.broker_h264_right_playback_requested = true,
        }
        Self::emit_hardware_buffer_import_marker(&format!(
            "phase=texture-handle-ready status=ok side={} textureHandle={} textureMode=external-oes brokerHost={} brokerPort={} streamPort={} sourceMode={} syntheticPattern={} liveStream={} importPlan=broker-h264-stereo-surface-texture",
            side.label(),
            ready.handle,
            marker_token(&source.broker_host),
            source.broker_port,
            source.stream_port,
            marker_token(&source.source_mode),
            marker_token(&source.synthetic_pattern),
            source.live_stream,
        ));
        cx.prepare_video_playback(
            side.video_id(),
            VideoSource::BrokerH264(source),
            CameraPreviewMode::Texture,
            ready.handle,
            ready.texture_id,
            true,
            false,
        );
    }

    fn request_broker_h264_cpu_yuv_import(
        &mut self,
        cx: &mut Cx,
        side: StereoEye,
        texture_id: TextureId,
    ) {
        if !Self::broker_h264_enabled() {
            return;
        }
        let already_requested = match side {
            StereoEye::Left => self.broker_h264_left_playback_requested,
            StereoEye::Right => self.broker_h264_right_playback_requested,
        };
        if already_requested {
            return;
        }

        let source = Self::broker_h264_source_for_eye(side);
        match side {
            StereoEye::Left => self.broker_h264_left_playback_requested = true,
            StereoEye::Right => self.broker_h264_right_playback_requested = true,
        }
        Self::emit_hardware_buffer_import_marker(&format!(
            "phase=broker-h264-prepare-request status=sent side={} textureHandle=0 textureMode=cpu-yuv brokerHost={} brokerPort={} streamPort={} sourceMode={} syntheticPattern={} liveStream={} importPlan=broker-h264-stereo-mediacodec-yuv-texture",
            side.label(),
            marker_token(&source.broker_host),
            source.broker_port,
            source.stream_port,
            marker_token(&source.source_mode),
            marker_token(&source.synthetic_pattern),
            source.live_stream,
        ));
        cx.prepare_video_playback(
            side.video_id(),
            VideoSource::BrokerH264(source),
            CameraPreviewMode::Texture,
            0,
            texture_id,
            true,
            false,
        );
    }

    fn try_start_paired_import(&mut self, cx: &mut Cx) {
        if self.paired_import_started || self.paired_import_finished {
            return;
        }

        if Self::broker_h264_enabled() && self.paired_import_choice.is_none() {
            let source = Self::broker_h264_source();
            self.paired_import_choice = Some(MakepadCameraPair::from_broker_h264_source(&source));
        }

        let Some(pair) = self.paired_import_choice.clone() else {
            self.paired_import_wait_count = self.paired_import_wait_count.saturating_add(1);
            if self.paired_import_wait_count > PAIRED_IMPORT_MAX_WAITS {
                self.paired_import_finished = true;
                Self::emit_hardware_buffer_import_marker(
                    "phase=start status=error errorKind=no_makepad_camera_stereo_pair",
                );
                Self::emit_stereo_projection_marker(
                    "phase=start status=error pairedLeftRightGpuBuffers=false projectionMappingReady=false alignedProjection=false fallbackReason=no_makepad_camera_stereo_pair",
                );
            } else {
                Self::emit_hardware_buffer_import_marker(&format!(
                    "phase=start status=waiting waitCount={} reason=no_makepad_camera_stereo_pair_yet",
                    self.paired_import_wait_count,
                ));
                self.arm_paired_import_timer(cx, PAIRED_IMPORT_RETRY_SECONDS, "stereo-pair-retry");
            }
            return;
        };

        let left_texture = Texture::new_with_format(cx, TextureFormat::VideoExternal);
        let right_texture = Texture::new_with_format(cx, TextureFormat::VideoExternal);
        self.paired_import_left_texture = Some(left_texture);
        self.paired_import_right_texture = Some(right_texture);
        self.paired_import_started = true;

        let broker_h264_enabled = Self::broker_h264_enabled();
        Self::emit_hardware_buffer_import_marker(&format!(
            "phase=start status=started importPlan={} leftSourceIndex={} rightSourceIndex={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftFrameRate={} rightFrameRate={} pixelFormat={} leftStreamPort={} rightStreamPort={} importPath={} textureFormat={} depthClip=false environmentDepthClip=false delayedAfterAcquisitionSeconds={:.0}",
            if broker_h264_enabled {
                "broker-h264-stereo-mediacodec-yuv-texture"
            } else {
                "single-stream-yuv-proof"
            },
            pair.left.source_index,
            pair.right.source_index,
            pair.left.source_class,
            pair.right.source_class,
            pair.left.width,
            pair.left.height,
            pair.right.width,
            pair.right.height,
            frame_rate_token(pair.left.frame_rate),
            frame_rate_token(pair.right.frame_rate),
            pixel_format_label(pair.left.pixel_format),
            if broker_h264_enabled {
                Self::broker_h264_stream_port(StereoEye::Left).to_string()
            } else {
                "none".to_string()
            },
            if broker_h264_enabled {
                Self::broker_h264_stream_port(StereoEye::Right).to_string()
            } else {
                "none".to_string()
            },
            if broker_h264_enabled {
                "broker-h264-mediacodec-cpu-yuv"
            } else {
                "makepad-android-camera-yuv-plane-cpu-proof"
            },
            if broker_h264_enabled {
                "VideoYuvPlaneStereo"
            } else {
                "VideoYuvPlane"
            },
            PAIRED_IMPORT_DELAY_SECONDS,
        ));
        Self::emit_stereo_projection_marker(&format!(
            "phase=start status=started pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} {} leftSourceIndex={} rightSourceIndex={} projectionMode={} projectionScale={:.2} xrRenderScale={:.2} fallbackReason={}",
            pair.projection_homography_ready,
            pair.projection_metadata_ready,
            pair.pose_source,
            pair.source_eye_mapping,
            pair.coordinate_chain,
            projection_homography_marker_fields(&pair),
            pair.left.source_index,
            pair.right.source_index,
            runtime_text(&Self::runtime_config(), KEY_CAMERA_PROJECTION_MODE),
            runtime_float(&Self::runtime_config(), KEY_PROJECTION_SCALE),
            runtime_float(&Self::runtime_config(), KEY_XR_RENDER_SCALE),
            marker_token(&pair.fallback_reason),
        ));

        if NATIVE_VIDEO_WIDGET_SURFACE_DIAGNOSTIC {
            if self.start_native_video_widget_surface(cx, &pair) {
                self.paired_import_finished = true;
            }
            return;
        }

        if broker_h264_enabled {
            self.bind_camera_projection_panel(cx);
            if let Some(texture) = self.paired_import_left_texture.as_ref() {
                self.request_broker_h264_cpu_yuv_import(cx, StereoEye::Left, texture.texture_id());
            }
            if let Some(texture) = self.paired_import_right_texture.as_ref() {
                self.request_broker_h264_cpu_yuv_import(cx, StereoEye::Right, texture.texture_id());
            }
            return;
        }

        cx.prepare_headset_camera_playback(
            StereoEye::Left.video_id(),
            VideoSource::Camera(pair.left.input_id, pair.left.format_id),
            CameraPreviewMode::Texture,
            0,
            TextureId::default(),
            true,
            false,
        );
        cx.prepare_headset_camera_playback(
            StereoEye::Right.video_id(),
            VideoSource::Camera(pair.right.input_id, pair.right.format_id),
            CameraPreviewMode::Texture,
            0,
            TextureId::default(),
            true,
            false,
        );
    }

    fn start_native_video_widget_surface(&mut self, cx: &mut Cx, pair: &MakepadCameraPair) -> bool {
        if self.native_video_widget_started {
            return true;
        }

        let left_video = self.ui.video(cx, &[live_id!(left_camera_video)]);
        let right_video = self.ui.video(cx, &[live_id!(right_camera_video)]);
        let left_unprepared = left_video.is_unprepared();
        let right_unprepared = right_video.is_unprepared();
        if !left_unprepared || !right_unprepared {
            if self.native_video_widget_retry_count >= NATIVE_VIDEO_WIDGET_MAX_RESETS {
                Self::emit_stereo_projection_marker(&format!(
                    "phase=native-video-widget-reset status=error leftUnprepared={} rightUnprepared={} leftPlaying={} rightPlaying={} leftCleaningUp={} rightCleaningUp={} resetCount={} fallbackReason=makepad_video_widget_not_unprepared",
                    left_unprepared,
                    right_unprepared,
                    left_video.is_playing(),
                    right_video.is_playing(),
                    left_video.is_cleaning_up(),
                    right_video.is_cleaning_up(),
                    self.native_video_widget_retry_count,
                ));
                return true;
            }

            if !left_unprepared && !left_video.is_cleaning_up() {
                left_video.stop_and_cleanup_resources(cx);
            }
            if !right_unprepared && !right_video.is_cleaning_up() {
                right_video.stop_and_cleanup_resources(cx);
            }
            self.native_video_widget_retry_count =
                self.native_video_widget_retry_count.saturating_add(1);
            self.native_video_widget_retry_pair = Some(pair.clone());
            self.native_video_widget_retry_timer =
                cx.start_timeout(NATIVE_VIDEO_WIDGET_RETRY_SECONDS);
            Self::emit_stereo_projection_marker(&format!(
                "phase=native-video-widget-reset status=waiting leftUnprepared={} rightUnprepared={} leftPlaying={} rightPlaying={} leftCleaningUp={} rightCleaningUp={} resetCount={} retrySeconds={:.1} fallbackReason=makepad_video_widget_not_unprepared",
                left_unprepared,
                right_unprepared,
                left_video.is_playing(),
                right_video.is_playing(),
                left_video.is_cleaning_up(),
                right_video.is_cleaning_up(),
                self.native_video_widget_retry_count,
                NATIVE_VIDEO_WIDGET_RETRY_SECONDS,
            ));
            return false;
        }

        left_video.set_camera_preview_mode(cx, VideoCameraPreviewMode::Texture);
        right_video.set_camera_preview_mode(cx, VideoCameraPreviewMode::Texture);
        left_video.set_camera_permission(VideoCameraPermission::HeadsetCamera);
        right_video.set_camera_permission(VideoCameraPermission::HeadsetCamera);
        left_video.set_source_camera(cx, pair.left.input_id, pair.left.format_id);
        right_video.set_source_camera(cx, pair.right.input_id, pair.right.format_id);
        left_video.should_dispatch_texture_updates(true);
        right_video.should_dispatch_texture_updates(true);
        left_video.begin_playback(cx);
        right_video.begin_playback(cx);
        self.native_video_widget_started = true;

        Self::emit_stereo_projection_marker(&format!(
            "phase=native-video-widget-surface status=started renderPath=makepad-xr-view-video-widget visibleCameraProjectionReady=false leftSourceIndex={} rightSourceIndex={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} resetCount={} projectionShaderPath=makepad-video-widget-yuv visualInspection=required visualReleaseAccepted=false",
            pair.left.source_index,
            pair.right.source_index,
            pair.left.width,
            pair.left.height,
            pair.right.width,
            pair.right.height,
            self.native_video_widget_retry_count,
        ));
        true
    }

    fn emit_makepad_camera_selection_marker(&self, inputs: &VideoInputsEvent) {
        let source_count = inputs.descs.len();
        let format_count: usize = inputs.descs.iter().map(|desc| desc.formats.len()).sum();
        match &self.paired_import_choice {
            Some(pair) => {
                Self::emit_hardware_buffer_import_marker(&format!(
                "phase=enumerated status=ok makepadSourceCount={} makepadFormatCount={} selected=true importPlan=paired-makepad-video-hardware-buffer sourceBindingMode={} leftSourceIndex={} rightSourceIndex={} leftCameraId={} rightCameraId={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftFrameRate={} rightFrameRate={} pixelFormat={}",
                source_count,
                format_count,
                pair.source_binding_mode,
                pair.left.source_index,
                pair.right.source_index,
                marker_token(pair.left.camera_id.as_deref().unwrap_or("unknown")),
                marker_token(pair.right.camera_id.as_deref().unwrap_or("unknown")),
                pair.left.source_class,
                pair.right.source_class,
                pair.left.width,
                pair.left.height,
                pair.right.width,
                pair.right.height,
                frame_rate_token(pair.left.frame_rate),
                frame_rate_token(pair.right.frame_rate),
                pixel_format_label(pair.left.pixel_format),
            ));
                Self::emit_stereo_projection_marker(&format!(
                    "phase=enumerated status=ok makepadSourceCount={} makepadFormatCount={} pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} {} leftSourceIndex={} rightSourceIndex={} sourceBindingMode={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} fallbackReason={}",
                    source_count,
                    format_count,
                    pair.projection_homography_ready,
                    pair.projection_metadata_ready,
                    pair.pose_source,
                    pair.source_eye_mapping,
                    pair.coordinate_chain,
                    projection_homography_marker_fields(pair),
                    pair.left.source_index,
                    pair.right.source_index,
                    pair.source_binding_mode,
                    pair.left.source_class,
                    pair.right.source_class,
                    pair.left.width,
                    pair.left.height,
                    pair.right.width,
                    pair.right.height,
                    marker_token(&pair.fallback_reason),
                ));
            }
            None => Self::emit_hardware_buffer_import_marker(&format!(
                "phase=enumerated status=error makepadSourceCount={} makepadFormatCount={} selected=false errorKind=no_yuv420_makepad_camera_stereo_pair",
                source_count,
                format_count,
            )),
        }
    }

    fn pick_makepad_camera_pair(inputs: &VideoInputsEvent) -> Option<MakepadCameraPair> {
        if Self::broker_h264_enabled() {
            let source = Self::broker_h264_source();
            return Some(MakepadCameraPair::from_broker_h264_source(&source));
        }
        let choices = collect_makepad_camera_choices(inputs);
        let camera2_plan = Self::latest_camera2_stereo_plan();
        camera2_plan
            .as_ref()
            .and_then(|plan| MakepadCameraPair::from_camera2_plan(&choices, plan))
            .or_else(|| MakepadCameraPair::from_best_available_pair(&choices))
    }

    fn emit_paired_projection_progress(&self, phase: &str) {
        let Some(pair) = &self.paired_import_choice else {
            return;
        };
        Self::emit_stereo_projection_marker(&format!(
            "phase={} status=progress leftPrepared={} rightPrepared={} leftUpdated={} rightUpdated={} pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} {} leftSourceIndex={} rightSourceIndex={} fallbackReason={}",
            phase,
            self.paired_import_left_prepared,
            self.paired_import_right_prepared,
            self.paired_import_left_updated,
            self.paired_import_right_updated,
            pair.projection_homography_ready,
            pair.projection_metadata_ready,
            pair.pose_source,
            pair.source_eye_mapping,
            projection_homography_marker_fields(pair),
            pair.left.source_index,
            pair.right.source_index,
            marker_token(&pair.fallback_reason),
        ));
    }

    fn emit_yuv_texture_content_probe(&self, cx: &mut Cx, side: StereoEye, yuv: VideoYuvMetadata) {
        if TEXTURE_CONTENT_PROBE_MARKERS_EMITTED.fetch_add(1, Ordering::AcqRel)
            >= TEXTURE_CONTENT_PROBE_MARKER_LIMIT
        {
            return;
        }

        let textures = match side {
            StereoEye::Left => self.paired_import_left_yuv_textures.clone(),
            StereoEye::Right => self.paired_import_right_yuv_textures.clone(),
        };

        let Some(textures) = textures else {
            Self::emit_stereo_projection_marker(&format!(
                "phase=texture-content-probe status=missing side={} textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 yuvEnabled={} yuvBiplanar={} yuvMatrix={:.1} rotationSteps={:.0} cpuPlaneContentPresent=false visualInspection=required visualReleaseAccepted=false",
                side.label(),
                yuv.enabled,
                yuv.biplanar,
                yuv.matrix,
                yuv.rotation_steps,
            ));
            return;
        };

        let y_stats = texture_plane_content_stats(cx, &textures.y);
        let u_stats = texture_plane_content_stats(cx, &textures.u);
        let v_stats = texture_plane_content_stats(cx, &textures.v);
        let cpu_content_present =
            y_stats.readable && y_stats.data_present && y_stats.sample_count > 0 && y_stats.max > 0;

        Self::emit_stereo_projection_marker(&format!(
            "phase=texture-content-probe status=ok side={} textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 yuvEnabled={} yuvBiplanar={} yuvMatrix={:.1} rotationSteps={:.0} cpuPlaneContentPresent={} {} {} {} gpuSamplingStillVisual=full-frame-source-display-row-vertical-uv-yuv visualInspection=required visualReleaseAccepted=false",
            side.label(),
            yuv.enabled,
            yuv.biplanar,
            yuv.matrix,
            yuv.rotation_steps,
            cpu_content_present,
            y_stats.marker_fields("y"),
            u_stats.marker_fields("u"),
            v_stats.marker_fields("v"),
        ));
    }

    fn bind_camera_projection_panel(&mut self, cx: &mut Cx) -> bool {
        let broker_h264_enabled = Self::broker_h264_enabled();
        let paired_streams_available =
            self.paired_import_left_updated && self.paired_import_right_updated;
        if self.camera_projection_textures_bound
            && (!paired_streams_available || self.camera_projection_paired_textures_bound)
        {
            return true;
        }

        let (Some(left_texture), Some(right_texture), Some(pair)) = (
            self.paired_import_left_texture.clone(),
            self.paired_import_right_texture.clone(),
            self.paired_import_choice.clone(),
        ) else {
            return false;
        };
        let left_updated_yuv = if self.paired_import_left_updated {
            self.paired_import_left_yuv_textures.clone()
        } else {
            None
        };
        let right_updated_yuv = if self.paired_import_right_updated {
            self.paired_import_right_yuv_textures.clone()
        } else {
            None
        };
        let proof_source_side = match (left_updated_yuv.is_some(), right_updated_yuv.is_some()) {
            (true, true) => "paired",
            (true, false) => "left",
            (false, true) => "right",
            (false, false) => "ready-only",
        };
        let (left_yuv_source, right_yuv_source) =
            match (left_updated_yuv.clone(), right_updated_yuv.clone()) {
                (Some(left), Some(right)) => (Some(left), Some(right)),
                (Some(left), None) => (Some(left.clone()), Some(left)),
                (None, Some(right)) => (Some(right.clone()), Some(right)),
                (None, None) => {
                    let left_ready = self
                        .paired_import_left_yuv_textures
                        .clone()
                        .or_else(|| self.paired_import_right_yuv_textures.clone());
                    let right_ready = self
                        .paired_import_right_yuv_textures
                        .clone()
                        .or_else(|| left_ready.clone());
                    (left_ready, right_ready)
                }
            };
        let single_stream_visual_proof =
            !(self.paired_import_left_updated && self.paired_import_right_updated);
        let broker_h264_cpu_yuv_decode = broker_h264_enabled
            && (left_yuv_source.is_some()
                || right_yuv_source.is_some()
                || self.paired_import_left_yuv_textures.is_some()
                || self.paired_import_right_yuv_textures.is_some());
        let explicit_top_left_broker_stimulus = broker_h264_enabled
            && self
                .broker_h264_left_projection_metadata
                .as_ref()
                .is_some_and(
                    BrokerH264ProjectionMetadata::has_explicit_top_left_stimulus_orientation,
                )
            && self
                .broker_h264_right_projection_metadata
                .as_ref()
                .is_some_and(
                    BrokerH264ProjectionMetadata::has_explicit_top_left_stimulus_orientation,
                );
        let orientation_decision = if broker_h264_enabled {
            match (
                self.broker_h264_left_projection_metadata.as_ref(),
                self.broker_h264_right_projection_metadata.as_ref(),
            ) {
                (Some(left), Some(right)) => {
                    FrameOrientationDecision::from_broker_pair(left, right)
                }
                _ => FrameOrientationDecision::fallback("broker-h264-orientation-metadata-missing"),
            }
        } else {
            FrameOrientationDecision::direct_camera2()
        };
        let source_sample_y_flip = orientation_decision.source_sample_y_flip;
        let full_frame_diagnostic = pair.source_binding_mode.contains("full-frame-diagnostic")
            || pair.projection_geometry_profile == "full-frame-diagnostic";
        // Full-frame diagnostic content still has to land on the solved
        // head-anchored surface. The fullscreen Makepad draw pass therefore
        // uses the display-indexed screen-to-surface rows instead of mapping
        // directly into the projection-area rectangle.
        let projection_content_mapping_mode = 0.0;
        let source_sample_transform = if source_sample_y_flip >= 0.5 {
            "stimulus-raster-y-flip"
        } else if orientation_decision.raster_orientation == FRAME_RASTER_TOP_LEFT_Y_DOWN {
            "identity-top-left-stimulus-raster"
        } else {
            "identity-y-to-match-raster-metadata"
        };
        let (left_yuv, right_yuv) = if broker_h264_enabled {
            if broker_h264_cpu_yuv_decode {
                let left_yuv = left_yuv_source
                    .clone()
                    .or_else(|| right_yuv_source.clone())
                    .or_else(|| self.paired_import_left_yuv_textures.clone())
                    .or_else(|| self.paired_import_right_yuv_textures.clone());
                let right_yuv = right_yuv_source
                    .clone()
                    .or_else(|| left_yuv_source.clone())
                    .or_else(|| self.paired_import_right_yuv_textures.clone())
                    .or_else(|| left_yuv.clone());
                (left_yuv, right_yuv)
            } else {
                (None, None)
            }
        } else {
            let (Some(left_yuv), Some(right_yuv)) = (left_yuv_source, right_yuv_source) else {
                if !self.camera_projection_bind_error_logged {
                    Self::emit_stereo_projection_marker(
                        "phase=visible-panel-bound status=waiting visibleCameraProjectionReady=false fallbackReason=makepad_camera_yuv_plane_textures_missing",
                    );
                    self.camera_projection_bind_error_logged = true;
                }
                return false;
            };
            (Some(left_yuv), Some(right_yuv))
        };

        let panel_ref = self.ui.widget(cx, ids!(camera_projection_panel));
        let Some(mut panel) = panel_ref.borrow_mut::<MakepadStereoCameraPanel>() else {
            if !self.camera_projection_bind_error_logged {
                Self::emit_stereo_projection_marker(
                    "phase=visible-panel-bound status=error visibleCameraProjectionReady=false fallbackReason=makepad_camera_projection_panel_missing",
                );
                self.camera_projection_bind_error_logged = true;
            }
            return false;
        };

        panel.apply_projection_panel_geometry(cx);
        panel.set_camera_textures(
            cx,
            Some(left_texture),
            Some(right_texture),
            left_yuv,
            right_yuv,
            self.paired_import_left_rotation_steps,
            self.paired_import_right_rotation_steps,
            pair.left_surface_to_camera_h,
            pair.right_surface_to_camera_h,
            pair.left_screen_to_camera_h,
            pair.right_screen_to_camera_h,
            pair.left_screen_to_surface_h,
            pair.right_screen_to_surface_h,
            source_sample_y_flip,
            projection_content_mapping_mode,
        );
        panel.set_horizontal_alignment_tuning(cx, self.current_horizontal_alignment_tuning());
        self.camera_projection_textures_bound = true;
        self.camera_projection_paired_textures_bound = !single_stream_visual_proof;
        let content_geometry_fields = if broker_h264_enabled {
            match (
                self.broker_h264_left_projection_metadata.as_ref(),
                self.broker_h264_right_projection_metadata.as_ref(),
            ) {
                (Some(left), Some(right)) => {
                    broker_pair_content_geometry_marker_fields(left, right)
                }
                _ => missing_broker_content_geometry_marker_fields(),
            }
        } else {
            direct_camera2_content_geometry_marker_fields(
                pair.left.width,
                pair.left.height,
                &pair.projection_geometry_profile,
            )
        };
        let source_color_contract = makepad_current_source_color_contract_fields();
        let source_sampling_fields = MakepadSourceSamplingHandoff::new(
            broker_h264_enabled,
            explicit_top_left_broker_stimulus,
            &orientation_decision,
            projection_content_mapping_mode,
            full_frame_diagnostic,
            &pair.source_eye_mapping,
            source_sample_transform,
            &content_geometry_fields,
            &source_color_contract,
        )
        .marker_fields();
        Self::emit_stereo_projection_marker(&source_sampling_fields);
        let cpu_yuv_path = !broker_h264_enabled || broker_h264_cpu_yuv_decode;
        Self::emit_stereo_projection_marker(&makepad_draw_vars_bound_marker_fields(
            &pair,
            cpu_yuv_path,
            broker_h264_enabled && !broker_h264_cpu_yuv_decode,
            single_stream_visual_proof,
            proof_source_side,
        ));
        if !self.synthetic_scene_hidden_for_camera {
            self.synthetic_scene_hidden_for_camera = true;
            Self::emit_stereo_projection_marker(
                "phase=synthetic-scene-hidden status=ok visibleCameraProjectionReady=true fallbackSceneVisible=false fallbackReason=makepad_synthetic_scene_removed_for_visual_gate",
            );
        }
        Self::emit_stereo_projection_marker(&makepad_visible_panel_bound_marker_fields(
            &pair,
            self.paired_import_left_rotation_steps,
            self.paired_import_right_rotation_steps,
            single_stream_visual_proof,
            proof_source_side,
        ));
        true
    }

    fn complete_paired_import_if_ready(&mut self, cx: &mut Cx) {
        if self.paired_import_finished {
            return;
        }

        let broker_h264_enabled = Self::broker_h264_enabled();
        let paired_streams_ready =
            self.paired_import_left_updated && self.paired_import_right_updated;
        let updated_stream_visual_proof_side = match (
            self.paired_import_left_updated,
            self.paired_import_right_updated,
        ) {
            (true, true) => "paired",
            (true, false) => "left",
            (false, true) => "right",
            (false, false) => "none",
        };
        let single_stream_ready = if broker_h264_enabled {
            false
        } else {
            (self.paired_import_left_updated || self.paired_import_right_updated)
                && (self.paired_import_left_yuv_textures.is_some()
                    || self.paired_import_right_yuv_textures.is_some())
        };
        if !paired_streams_ready && !single_stream_ready {
            self.emit_paired_projection_progress("texture-updated");
            return;
        }

        let Some(pair) = self.paired_import_choice.clone() else {
            return;
        };
        if !paired_streams_ready && !broker_h264_enabled {
            let visible_projection_ready = self.bind_camera_projection_panel(cx);
            if !self.camera_projection_single_stream_logged {
                self.camera_projection_single_stream_logged = true;
                Self::emit_stereo_projection_marker(&format!(
                    "phase=single-stream-proof status=waiting pairedLeftRightCameraFrames=false singleStreamCameraPixels=true leftUpdated={} rightUpdated={} leftYuvReady={} rightYuvReady={} projectionMappingReady={} alignedProjection=false visibleCameraProjectionReady={} sceneOwnedPanel=true projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review legacyPanelTargetDefaults=deprecated panelTargetFields=runtime diagnosticVisualLayer=none neutralWaitingPanel=true depthClip=false environmentDepthClip=false drawVarsTextureRedraw=true shaderAreaStateUpdate=true updatedStreamVisualProofSide={} visualInspection=required visualReleaseAccepted=false fallbackReason=waiting_for_second_cpu_yuv_stream",
                    self.paired_import_left_updated,
                    self.paired_import_right_updated,
                    self.paired_import_left_yuv_textures.is_some(),
                    self.paired_import_right_yuv_textures.is_some(),
                    pair.projection_homography_ready,
                    visible_projection_ready,
                    updated_stream_visual_proof_side,
                ));
            }
            return;
        }
        self.paired_import_finished = true;
        let aligned_projection = pair.projection_homography_ready && paired_streams_ready;
        let visible_projection_ready = self.bind_camera_projection_panel(cx);
        Self::emit_stereo_projection_marker(&format!(
            "phase=complete status=ok pairedLeftRightCameraFrames={} brokerH264SurfaceTexture={} makepadVulkanImport=false projectionMappingReady={} alignedProjection={} visibleCameraProjectionReady={} projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} projectionMode={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftRotationSteps={:.0} rightRotationSteps={:.0} projectionScale={:.2} xrRenderScale={:.2} renderPath=makepad-xr projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 {} nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review legacyPanelTargetDefaults=deprecated panelTargetFields=runtime cpuUploadPath={} diagnosticVisualLayer=none neutralWaitingPanel=true visualIsolation=s118_projected_footprint_solid_red_exterior depthClip=false environmentDepthClip=false drawVarsTextureRedraw=true shaderAreaStateUpdate=true visualInspection=required visualReleaseAccepted=false fallbackReason={}",
            paired_streams_ready,
            broker_h264_enabled,
            pair.projection_homography_ready,
            aligned_projection,
            visible_projection_ready,
            pair.projection_metadata_ready,
            pair.pose_source,
            pair.source_eye_mapping,
            pair.coordinate_chain,
            runtime_text(&Self::runtime_config(), KEY_CAMERA_PROJECTION_MODE),
            pair.left.source_index,
            pair.right.source_index,
            pair.left.source_class,
            pair.right.source_class,
            pair.left.width,
            pair.left.height,
            pair.right.width,
            pair.right.height,
            self.paired_import_left_rotation_steps,
            self.paired_import_right_rotation_steps,
            runtime_float(&Self::runtime_config(), KEY_PROJECTION_SCALE),
            runtime_float(&Self::runtime_config(), KEY_XR_RENDER_SCALE),
            projection_homography_marker_fields(&pair),
            if broker_h264_enabled {
                "broker-h264-mediacodec-cpu-yuv"
            } else {
                "makepad-camera-cpu-yuv-plane"
            },
            marker_token(&pair.fallback_reason),
        ));
        Self::emit_stereo_comparison_parity_marker(
            "paired-projection-ready",
            &pair,
            aligned_projection,
            visible_projection_ready,
        );
    }

    fn emit_stereo_comparison_parity_marker(
        phase: &str,
        pair: &MakepadCameraPair,
        aligned_projection: bool,
        visible_projection_ready: bool,
    ) {
        let config = Self::runtime_config();
        Self::emit_projection_runtime_manifest_marker(
            phase,
            &config,
            Self::horizontal_alignment_tuning(),
        );
        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_STEREO_COMPARISON schema=rusty.xr.makepad-stereo-comparison.v1 phase={} profile={} comparisonBaseline={} cameraTier={} acquisition={} transport={} projectionMode={} syntheticScene={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} sourceEyeMapping={} projectionScale={:.2} xrRenderScale={:.2} pairedLeftRightCameraFrames=true alignedProjection={} visibleCameraProjectionReady={} renderPath=makepad-xr projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 colorReference=android-yuv420-888-plane-order perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 {} makepadForkBranch={} makepadForkCommit={} nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review legacyPanelTargetDefaults=deprecated panelTargetFields=runtime diagnosticVisualLayer=none neutralWaitingPanel=true visualIsolation=s118_projected_footprint_solid_red_exterior depthClip=false environmentDepthClip=false cpuUploadPath=makepad-camera-cpu-yuv-plane drawVarsTextureRedraw=true shaderAreaStateUpdate=true visualInspection=required visualReleaseAccepted=false",
            phase,
            runtime_text(&config, KEY_RUNTIME_PROFILE),
            runtime_text(&config, KEY_COMPARISON_BASELINE),
            runtime_text(&config, KEY_CAMERA_TIER),
            runtime_text(&config, KEY_ACQUISITION_PROFILE),
            runtime_text(&config, KEY_TRANSPORT_PROFILE),
            runtime_text(&config, KEY_CAMERA_PROJECTION_MODE),
            runtime_text(&config, KEY_SYNTHETIC_SCENE),
            pair.left.source_index,
            pair.right.source_index,
            pair.source_eye_mapping,
            runtime_float(&config, KEY_PROJECTION_SCALE),
            runtime_float(&config, KEY_XR_RENDER_SCALE),
            aligned_projection,
            visible_projection_ready,
            projection_homography_marker_fields(pair),
            runtime_text(&config, KEY_MAKEPAD_BRANCH),
            runtime_text(&config, KEY_MAKEPAD_REVISION)
        ));
    }

    #[cfg(target_os = "android")]
    fn camera2_stereo_plan() -> Option<Camera2StereoPlan> {
        android_camera_probe::latest_stereo_projection_plan().map(Camera2StereoPlan::from)
    }

    #[cfg(not(target_os = "android"))]
    fn camera2_stereo_plan() -> Option<Camera2StereoPlan> {
        None
    }

    fn latest_camera2_stereo_plan() -> Option<Camera2StereoPlan> {
        let profile = Self::direct_camera_projection_geometry_profile();
        Self::camera2_stereo_plan().map(|mut plan| {
            plan.apply_projection_geometry_profile(&profile);
            plan
        })
    }

    #[cfg(target_os = "android")]
    fn start_camera_probe_once() {
        android_camera_probe::start_camera_probe_once();
    }

    #[cfg(not(target_os = "android"))]
    fn start_camera_probe_once() {}
}

fn collect_makepad_camera_choices(inputs: &VideoInputsEvent) -> Vec<MakepadCameraChoice> {
    inputs
        .descs
        .iter()
        .enumerate()
        .flat_map(|(source_index, desc)| {
            desc.formats
                .iter()
                .filter(|format| format.pixel_format == VideoPixelFormat::YUV420)
                .map(move |format| {
                    MakepadCameraChoice::new(
                        source_index,
                        desc.input_id,
                        *format,
                        camera_source_class(&desc.name),
                        camera_id_from_makepad_desc_name(&desc.name),
                    )
                })
        })
        .collect()
}

#[derive(Clone)]
struct MakepadCameraChoice {
    source_index: usize,
    input_id: makepad_widgets::makepad_platform::video::VideoInputId,
    format_id: makepad_widgets::makepad_platform::video::VideoFormatId,
    camera_id: Option<String>,
    source_class: &'static str,
    width: usize,
    height: usize,
    frame_rate: Option<f64>,
    pixel_format: VideoPixelFormat,
}

type MakepadCameraPairScore = (i32, i64, i64, i64, i64);
type MakepadCameraPairCandidate = (
    MakepadCameraChoice,
    MakepadCameraChoice,
    MakepadCameraPairScore,
);

impl MakepadCameraChoice {
    fn new(
        source_index: usize,
        input_id: makepad_widgets::makepad_platform::video::VideoInputId,
        format: VideoFormat,
        source_class: &'static str,
        camera_id: Option<String>,
    ) -> Self {
        Self {
            source_index,
            input_id,
            format_id: format.format_id,
            camera_id,
            source_class,
            width: format.width,
            height: format.height,
            frame_rate: format.frame_rate,
            pixel_format: format.pixel_format,
        }
    }

    fn score(&self) -> (i32, i64, i64, i64) {
        let source_rank = match self.source_class {
            "back" => 3,
            "external" => 2,
            "front" => 1,
            _ => 0,
        };
        let frame_rate_milli = self
            .frame_rate
            .filter(|rate| rate.is_finite() && *rate > 0.0)
            .map(|rate| (rate * 1000.0).round() as i64)
            .unwrap_or(0);
        let target_penalty = self.width.abs_diff(1280) + self.height.abs_diff(1280);
        let square_penalty = self.width.abs_diff(self.height);
        let area = (self.width as i64) * (self.height as i64);
        (
            source_rank,
            frame_rate_milli,
            area - (target_penalty as i64) * 2048 - (square_penalty as i64) * 4096,
            area,
        )
    }

    fn broker_h264(label: &'static str, width: u32, height: u32) -> Self {
        let source_index = if label == "right" { 1 } else { 0 };
        Self {
            source_index,
            input_id: Default::default(),
            format_id: Default::default(),
            camera_id: Some(format!("broker-h264-{label}")),
            source_class: "synthetic",
            width: width as usize,
            height: height as usize,
            frame_rate: None,
            pixel_format: VideoPixelFormat::Unsupported(0x6832_3634),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StereoEye {
    Left,
    Right,
}

impl StereoEye {
    fn label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    fn video_id(self) -> LiveId {
        match self {
            Self::Left => live_id!(rusty_xr_makepad_left_camera_import_probe),
            Self::Right => live_id!(rusty_xr_makepad_right_camera_import_probe),
        }
    }

    fn from_video_id(video_id: LiveId) -> Option<Self> {
        if video_id == Self::Left.video_id() {
            Some(Self::Left)
        } else if video_id == Self::Right.video_id() {
            Some(Self::Right)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct FrameOrientationDecision {
    source_sample_y_flip: f32,
    source_sample_y_flip_reason: String,
    orientation_kind: String,
    raster_orientation: String,
    upright_marker: String,
    metadata_source: String,
    orientation_default: bool,
    fallback_reason: String,
}

impl FrameOrientationDecision {
    fn direct_camera2() -> Self {
        Self {
            source_sample_y_flip: 0.0,
            source_sample_y_flip_reason:
                "direct-camera2-generated-stimulus-top-left-raster-matches-makepad-video-sampler-origin".to_string(),
            orientation_kind: "camera-frame".to_string(),
            raster_orientation: FRAME_RASTER_TOP_LEFT_Y_DOWN.to_string(),
            upright_marker: "camera-native-upright".to_string(),
            metadata_source: "generated-direct-camera2-stimulus-metadata".to_string(),
            orientation_default: false,
            fallback_reason: "none".to_string(),
        }
    }

    fn fallback(reason: &str) -> Self {
        Self {
            source_sample_y_flip: 0.0,
            source_sample_y_flip_reason:
                "standard-stimulus-default-top-left-raster-matches-makepad-video-sampler-origin"
                    .to_string(),
            orientation_kind: "standard-stimulus-default".to_string(),
            raster_orientation: FRAME_RASTER_TOP_LEFT_Y_DOWN.to_string(),
            upright_marker: "unspecified".to_string(),
            metadata_source: "standard-stimulus-orientation-default".to_string(),
            orientation_default: true,
            fallback_reason: reason.to_string(),
        }
    }

    fn from_broker_pair(
        left: &BrokerH264ProjectionMetadata,
        right: &BrokerH264ProjectionMetadata,
    ) -> Self {
        if !left.has_explicit_stimulus_orientation() || !right.has_explicit_stimulus_orientation() {
            return Self::fallback("broker-h264-explicit-stimulus-orientation-missing");
        }
        if left.stimulus_raster_orientation != right.stimulus_raster_orientation {
            return Self::fallback("broker-h264-left-right-stimulus-orientation-mismatch");
        }
        let source_sample_y_flip = match left.stimulus_raster_orientation.as_str() {
            FRAME_RASTER_TOP_LEFT_Y_DOWN => 0.0,
            FRAME_RASTER_BOTTOM_LEFT_Y_UP => 1.0,
            _ => return Self::fallback("broker-h264-unsupported-stimulus-orientation"),
        };
        let source_sample_y_flip_reason = match left.stimulus_raster_orientation.as_str() {
            FRAME_RASTER_TOP_LEFT_Y_DOWN => {
                "broker-stimulus-top-left-raster-matches-makepad-video-sampler-origin"
            }
            FRAME_RASTER_BOTTOM_LEFT_Y_UP => {
                "broker-stimulus-bottom-left-raster-to-makepad-video-sampler-origin"
            }
            _ => "broker-stimulus-raster-unsupported",
        };
        Self {
            source_sample_y_flip,
            source_sample_y_flip_reason: source_sample_y_flip_reason.to_string(),
            orientation_kind: if left.orientation_kind == right.orientation_kind {
                left.orientation_kind.clone()
            } else {
                format!("{}+{}", left.orientation_kind, right.orientation_kind)
            },
            raster_orientation: left.stimulus_raster_orientation.clone(),
            upright_marker: if left.stimulus_upright_marker == right.stimulus_upright_marker {
                left.stimulus_upright_marker.clone()
            } else {
                format!(
                    "{}+{}",
                    left.stimulus_upright_marker, right.stimulus_upright_marker
                )
            },
            metadata_source: if left.stimulus_orientation_metadata_source
                == right.stimulus_orientation_metadata_source
            {
                left.stimulus_orientation_metadata_source.clone()
            } else {
                format!(
                    "{}+{}",
                    left.stimulus_orientation_metadata_source,
                    right.stimulus_orientation_metadata_source
                )
            },
            orientation_default: false,
            fallback_reason: "none".to_string(),
        }
    }
}

fn broker_pair_pose_source(
    left: &BrokerH264ProjectionMetadata,
    right: &BrokerH264ProjectionMetadata,
) -> String {
    if left.pose_source == right.pose_source {
        left.pose_source.clone()
    } else {
        format!("{}+{}", left.pose_source, right.pose_source)
    }
}

fn emit_raw_video_event_marker(event_name: &str, video_id: LiveId) {
    let marker_index = VIDEO_EVENT_RAW_MARKERS_EMITTED.fetch_add(1, Ordering::AcqRel);
    if marker_index >= RAW_VIDEO_EVENT_MARKER_LIMIT {
        return;
    }
    let side = StereoEye::from_video_id(video_id)
        .map(StereoEye::label)
        .unwrap_or("unknown");
    emit_marker_line(&format!(
        "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=raw-video-event status=seen event={} side={} videoId={} leftVideoId={} rightVideoId={} depthClip=false environmentDepthClip=false importPlan=single-stream-yuv-proof",
        event_name,
        side,
        video_id.0,
        StereoEye::Left.video_id().0,
        StereoEye::Right.video_id().0,
    ));
}

#[derive(Clone)]
struct MakepadCameraYuvTextures {
    y: Texture,
    u: Texture,
    v: Texture,
}

impl MakepadCameraYuvTextures {
    fn new(y: Texture, u: Texture, v: Texture) -> Self {
        Self { y, u, v }
    }
}

#[derive(Clone)]
struct MakepadCameraPair {
    left: MakepadCameraChoice,
    right: MakepadCameraChoice,
    projection_metadata_ready: bool,
    projection_geometry_profile: String,
    pose_source: String,
    source_eye_mapping: String,
    source_binding_mode: String,
    coordinate_chain: String,
    fallback_reason: String,
    left_surface_to_camera_h: [[f32; 3]; 3],
    right_surface_to_camera_h: [[f32; 3]; 3],
    left_surface_to_screen_h: [[f32; 3]; 3],
    right_surface_to_screen_h: [[f32; 3]; 3],
    left_screen_to_camera_h: [[f32; 3]; 3],
    right_screen_to_camera_h: [[f32; 3]; 3],
    left_screen_to_surface_h: [[f32; 3]; 3],
    right_screen_to_surface_h: [[f32; 3]; 3],
    left_source_valid_uv_rect: Rect2,
    right_source_valid_uv_rect: Rect2,
    projection_homography_ready: bool,
    runtime_xr_view_state_ready: bool,
    openxr_contract: MakepadOpenXrProjectionContract,
}

impl MakepadCameraPair {
    fn from_broker_h264_source(source: &BrokerH264VideoSource) -> Self {
        let width = source.preferred_width.max(1);
        let height = source.preferred_height.max(1);
        let left = MakepadCameraChoice::broker_h264("left", width, height);
        let right = MakepadCameraChoice::broker_h264("right", width, height);
        let source_mode = source
            .source_mode
            .trim()
            .to_ascii_lowercase()
            .replace('_', "-");
        let source_binding_mode = match source_mode.as_str() {
            "broker-camera" | "camera" | "camera2" => "broker-h264-camera-stereo-stream",
            "existing-stream" | "existing" | "remote" | "proxied" | "proxy" | "proxy-stream"
            | "incoming" | "incoming-stream" => "broker-h264-existing-stereo-stream",
            _ => "broker-h264-synthetic-stereo-stream",
        };
        let pose_source = match source_binding_mode {
            "broker-h264-camera-stereo-stream" => "broker-camera-h264-stream-header-pending",
            "broker-h264-existing-stereo-stream" => "broker-existing-h264-stream-header-pending",
            _ => "broker-synthetic-h264-stream-header-pending",
        };
        Self {
            left,
            right,
            projection_metadata_ready: false,
            projection_geometry_profile: source.synthetic_projection_profile.clone(),
            pose_source: pose_source.to_string(),
            source_eye_mapping: "left-right".to_string(),
            source_binding_mode: source_binding_mode.to_string(),
            coordinate_chain: "broker-h264-delivered-stereo-images-to-shader-surface".to_string(),
            fallback_reason: "waiting_for_broker_h264_stream_header".to_string(),
            left_surface_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_surface_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_surface_to_screen_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_surface_to_screen_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_screen_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_screen_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_screen_to_surface_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_screen_to_surface_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_source_valid_uv_rect: Rect2::UNIT,
            right_source_valid_uv_rect: Rect2::UNIT,
            projection_homography_ready: false,
            runtime_xr_view_state_ready: false,
            openxr_contract: MakepadOpenXrProjectionContract::missing(),
        }
    }

    fn from_camera2_plan(
        choices: &[MakepadCameraChoice],
        plan: &Camera2StereoPlan,
    ) -> Option<Self> {
        let left = best_choice_for_camera_id(choices, &plan.left_camera_id, plan.size()).or_else(
            || best_choice_for_source_index(choices, plan.left_source_index, plan.size()),
        )?;
        let right = best_choice_for_camera_id(choices, &plan.right_camera_id, plan.size())
            .or_else(|| {
                best_choice_for_source_index(choices, plan.right_source_index, plan.size())
            })?;
        if left.source_index == right.source_index {
            return None;
        }
        let source_binding_mode = if left.camera_id.as_deref() == Some(plan.left_camera_id.as_str())
            && right.camera_id.as_deref() == Some(plan.right_camera_id.as_str())
        {
            "camera-id"
        } else {
            "source-index-fallback"
        };
        let source_binding_mode = if plan.projection_geometry_profile == "full-frame-diagnostic" {
            format!("direct-camera2-full-frame-diagnostic-{source_binding_mode}")
        } else {
            source_binding_mode.to_string()
        };

        let _input_source_eye_mapping = &plan.source_eye_mapping;
        Some(Self {
            left,
            right,
            projection_metadata_ready: plan.projection_metadata_ready,
            projection_geometry_profile: plan.projection_geometry_profile.clone(),
            pose_source: plan.pose_source.clone(),
            source_eye_mapping: makepad_display_source_eye_mapping().to_string(),
            source_binding_mode,
            coordinate_chain: plan.coordinate_chain.clone(),
            fallback_reason: plan.fallback_reason.clone(),
            left_surface_to_camera_h: plan.left_surface_to_camera_h,
            right_surface_to_camera_h: plan.right_surface_to_camera_h,
            left_surface_to_screen_h: plan.left_surface_to_screen_h,
            right_surface_to_screen_h: plan.right_surface_to_screen_h,
            left_screen_to_camera_h: plan.left_screen_to_camera_h,
            right_screen_to_camera_h: plan.right_screen_to_camera_h,
            left_screen_to_surface_h: plan.left_screen_to_surface_h,
            right_screen_to_surface_h: plan.right_screen_to_surface_h,
            left_source_valid_uv_rect: Rect2::UNIT,
            right_source_valid_uv_rect: Rect2::UNIT,
            projection_homography_ready: plan.projection_homography_ready,
            runtime_xr_view_state_ready: plan.runtime_xr_view_state_ready,
            openxr_contract: plan.openxr_contract.clone(),
        })
    }

    fn from_best_available_pair(choices: &[MakepadCameraChoice]) -> Option<Self> {
        let mut best: Option<MakepadCameraPairCandidate> = None;

        for left in choices {
            for right in choices {
                if left.source_index == right.source_index
                    || left.pixel_format != right.pixel_format
                    || left.width != right.width
                    || left.height != right.height
                {
                    continue;
                }

                let source_rank =
                    source_class_rank(left.source_class) + source_class_rank(right.source_class);
                let frame_rate_milli = left
                    .frame_rate
                    .zip(right.frame_rate)
                    .filter(|(left_rate, right_rate)| {
                        left_rate.is_finite()
                            && right_rate.is_finite()
                            && *left_rate > 0.0
                            && *right_rate > 0.0
                    })
                    .map(|(left_rate, right_rate)| left_rate.min(right_rate))
                    .map(|rate| (rate * 1000.0).round() as i64)
                    .unwrap_or(0);
                let area = (left.width as i64) * (left.height as i64);
                let target_penalty = left.width.abs_diff(1280) + left.height.abs_diff(1280);
                let square_penalty = left.width.abs_diff(left.height);
                let index_spacing = left.source_index.abs_diff(right.source_index) as i64;
                let score = (
                    source_rank,
                    frame_rate_milli,
                    area - (target_penalty as i64) * 2048 - (square_penalty as i64) * 4096,
                    area,
                    -index_spacing,
                );

                if best
                    .as_ref()
                    .map(|(_, _, best_score)| score > *best_score)
                    .unwrap_or(true)
                {
                    best = Some((left.clone(), right.clone(), score));
                }
            }
        }

        let (left, right, _) = best?;
        Some(Self {
            left,
            right,
            projection_metadata_ready: false,
            projection_geometry_profile: DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE.to_string(),
            pose_source: "missing".to_string(),
            source_eye_mapping: makepad_display_source_eye_mapping().to_string(),
            source_binding_mode: "best-available-fallback".to_string(),
            coordinate_chain: "unresolved".to_string(),
            fallback_reason: "camera2 stereo projection metadata was not correlated".to_string(),
            left_surface_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_surface_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_surface_to_screen_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_surface_to_screen_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_screen_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_screen_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_screen_to_surface_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_screen_to_surface_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_source_valid_uv_rect: Rect2::UNIT,
            right_source_valid_uv_rect: Rect2::UNIT,
            projection_homography_ready: false,
            runtime_xr_view_state_ready: false,
            openxr_contract: MakepadOpenXrProjectionContract::missing(),
        })
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    fn matches_camera2_plan(&self, plan: &Camera2StereoPlan) -> bool {
        let camera_id_match = self.left.camera_id.as_deref() == Some(plan.left_camera_id.as_str())
            && self.right.camera_id.as_deref() == Some(plan.right_camera_id.as_str());
        let source_index_match = self.left.source_index == plan.left_source_index
            && self.right.source_index == plan.right_source_index;
        camera_id_match || source_index_match
    }
}

#[derive(Clone)]
struct Camera2StereoPlan {
    left_source_index: usize,
    right_source_index: usize,
    left_camera_id: String,
    right_camera_id: String,
    width: u32,
    height: u32,
    projection_metadata_ready: bool,
    projection_geometry_profile: String,
    pose_source: String,
    source_eye_mapping: String,
    coordinate_chain: String,
    fallback_reason: String,
    left_surface_to_camera_h: [[f32; 3]; 3],
    right_surface_to_camera_h: [[f32; 3]; 3],
    left_surface_to_screen_h: [[f32; 3]; 3],
    right_surface_to_screen_h: [[f32; 3]; 3],
    left_screen_to_camera_h: [[f32; 3]; 3],
    right_screen_to_camera_h: [[f32; 3]; 3],
    left_screen_to_surface_h: [[f32; 3]; 3],
    right_screen_to_surface_h: [[f32; 3]; 3],
    projection_homography_ready: bool,
    runtime_xr_view_state_ready: bool,
    openxr_contract: MakepadOpenXrProjectionContract,
}

impl Camera2StereoPlan {
    fn size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }

    fn apply_projection_geometry_profile(&mut self, profile: &str) {
        let profile = normalize_direct_camera_projection_geometry_profile(profile);
        self.projection_geometry_profile = profile.clone();
        if profile == "camera-projection" {
            if !self
                .coordinate_chain
                .contains("direct-camera2-screen-to-camera-homography")
            {
                self.coordinate_chain = format!(
                    "direct-camera2-screen-to-camera-homography/{}",
                    self.coordinate_chain
                );
            }
            if self.fallback_reason == "unsupported_direct_camera_projection_geometry_profile" {
                self.fallback_reason = if self.projection_homography_ready {
                    "none".to_string()
                } else {
                    "waiting_for_camera2_projection_homography".to_string()
                };
            }
            return;
        }
        if profile != "full-frame-diagnostic" {
            self.projection_metadata_ready = false;
            self.projection_homography_ready = false;
            self.fallback_reason = format!(
                "unsupported_direct_camera_projection_geometry_profile:{}",
                marker_token(&profile)
            );
            if !self
                .coordinate_chain
                .contains("unsupported-direct-camera-projection-geometry-profile")
            {
                self.coordinate_chain = format!(
                    "unsupported-direct-camera-projection-geometry-profile:{}/{}",
                    marker_token(&profile),
                    self.coordinate_chain
                );
            }
            return;
        }

        self.projection_metadata_ready = self.runtime_xr_view_state_ready;
        self.pose_source = "projection-surface".to_string();
        self.fallback_reason = if self.runtime_xr_view_state_ready {
            "none".to_string()
        } else {
            "waiting_for_runtime_openxr_view_state".to_string()
        };
        self.left_surface_to_camera_h = IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY;
        self.right_surface_to_camera_h = IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY;
        self.left_screen_to_camera_h = self.left_screen_to_surface_h;
        self.right_screen_to_camera_h = self.right_screen_to_surface_h;
        self.projection_homography_ready = self.runtime_xr_view_state_ready;
        if !self
            .coordinate_chain
            .contains("direct-camera2-full-frame-diagnostic-projection-surface")
        {
            self.coordinate_chain = format!(
                "direct-camera2-full-frame-diagnostic-projection-surface/{}",
                self.coordinate_chain
            );
        }
    }
}

#[cfg(target_os = "android")]
impl From<android_camera_probe::StereoProjectionPlan> for Camera2StereoPlan {
    fn from(plan: android_camera_probe::StereoProjectionPlan) -> Self {
        Self {
            left_source_index: plan.left_source_index,
            right_source_index: plan.right_source_index,
            left_camera_id: plan.left_camera_id,
            right_camera_id: plan.right_camera_id,
            width: plan.width,
            height: plan.height,
            projection_metadata_ready: plan.projection_metadata_ready,
            projection_geometry_profile: "camera2-platform-unprofiled".to_string(),
            pose_source: plan.pose_source.to_string(),
            source_eye_mapping: plan.source_eye_mapping.to_string(),
            coordinate_chain: plan.coordinate_chain.to_string(),
            fallback_reason: plan.fallback_reason.to_string(),
            left_surface_to_camera_h: plan.left_surface_to_camera_h,
            right_surface_to_camera_h: plan.right_surface_to_camera_h,
            left_surface_to_screen_h: plan.left_surface_to_screen_h,
            right_surface_to_screen_h: plan.right_surface_to_screen_h,
            left_screen_to_camera_h: plan.left_screen_to_camera_h,
            right_screen_to_camera_h: plan.right_screen_to_camera_h,
            left_screen_to_surface_h: plan.left_screen_to_surface_h,
            right_screen_to_surface_h: plan.right_screen_to_surface_h,
            projection_homography_ready: plan.projection_homography_ready,
            runtime_xr_view_state_ready: plan.runtime_xr_view_state_ready,
            openxr_contract: MakepadOpenXrProjectionContract::from_android(plan.openxr_contract),
        }
    }
}

fn best_choice_for_source_index(
    choices: &[MakepadCameraChoice],
    source_index: usize,
    preferred_size: (usize, usize),
) -> Option<MakepadCameraChoice> {
    choices
        .iter()
        .filter(|choice| choice.source_index == source_index)
        .max_by_key(|choice| {
            let preferred_match =
                (choice.width == preferred_size.0 && choice.height == preferred_size.1) as i32;
            (preferred_match, choice.score())
        })
        .cloned()
}

fn best_choice_for_camera_id(
    choices: &[MakepadCameraChoice],
    camera_id: &str,
    preferred_size: (usize, usize),
) -> Option<MakepadCameraChoice> {
    if camera_id.is_empty() {
        return None;
    }
    choices
        .iter()
        .filter(|choice| choice.camera_id.as_deref() == Some(camera_id))
        .max_by_key(|choice| {
            let preferred_match =
                (choice.width == preferred_size.0 && choice.height == preferred_size.1) as i32;
            (preferred_match, choice.score())
        })
        .cloned()
}

fn source_class_rank(source_class: &str) -> i32 {
    match source_class {
        "back" => 3,
        "external" => 2,
        "front" => 1,
        _ => 0,
    }
}

fn camera_id_from_makepad_desc_name(name: &str) -> Option<String> {
    let marker = "cameraId=";
    let start = name.find(marker)? + marker.len();
    let value = name[start..]
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
        .collect::<String>();
    (!value.is_empty()).then_some(value)
}

fn camera_source_class(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.contains("back") {
        "back"
    } else if lower.contains("external") {
        "external"
    } else if lower.contains("front") {
        "front"
    } else {
        "unknown"
    }
}

fn pixel_format_label(format: VideoPixelFormat) -> &'static str {
    match format {
        VideoPixelFormat::RGB24 => "rgb24",
        VideoPixelFormat::YUY2 => "yuy2",
        VideoPixelFormat::NV12 => "nv12",
        VideoPixelFormat::YUV420 => "yuv420",
        VideoPixelFormat::GRAY => "gray",
        VideoPixelFormat::MJPEG => "mjpeg",
        VideoPixelFormat::Unsupported(_) => "unsupported",
    }
}

fn frame_rate_token(frame_rate: Option<f64>) -> String {
    frame_rate
        .filter(|rate| rate.is_finite() && *rate > 0.0)
        .map(|rate| format!("{rate:.2}"))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yuv_choice(
        source_index: usize,
        camera_id: Option<&str>,
        width: usize,
        height: usize,
    ) -> MakepadCameraChoice {
        MakepadCameraChoice::new(
            source_index,
            Default::default(),
            VideoFormat {
                format_id: Default::default(),
                width,
                height,
                frame_rate: Some(72.0),
                pixel_format: VideoPixelFormat::YUV420,
            },
            "back",
            camera_id.map(str::to_string),
        )
    }

    fn test_plan() -> Camera2StereoPlan {
        Camera2StereoPlan {
            left_source_index: 0,
            right_source_index: 1,
            left_camera_id: "50".to_string(),
            right_camera_id: "51".to_string(),
            width: 1280,
            height: 1280,
            projection_metadata_ready: true,
            projection_geometry_profile: "camera2-platform-unprofiled".to_string(),
            pose_source: "platform-openxr-view".to_string(),
            source_eye_mapping: DEFAULT_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING.to_string(),
            coordinate_chain: "camera2-sensor-reference-to-openxr-head-basis".to_string(),
            fallback_reason: "none".to_string(),
            left_surface_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_surface_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_surface_to_screen_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_surface_to_screen_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_screen_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_screen_to_camera_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            left_screen_to_surface_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            right_screen_to_surface_h: IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY,
            projection_homography_ready: true,
            runtime_xr_view_state_ready: true,
            openxr_contract: MakepadOpenXrProjectionContract::missing(),
        }
    }

    #[test]
    fn parses_makepad_descriptor_camera_id_token() {
        assert_eq!(
            camera_id_from_makepad_desc_name("Back Camera cameraId=50").as_deref(),
            Some("50")
        );
        assert_eq!(
            camera_id_from_makepad_desc_name("External cameraId=cam_12-3.4 fps=72").as_deref(),
            Some("cam_12-3.4")
        );
        assert_eq!(camera_id_from_makepad_desc_name("Back Camera"), None);
    }

    #[test]
    fn camera_id_choice_prefers_requested_size() {
        let choices = vec![
            yuv_choice(0, Some("50"), 640, 640),
            yuv_choice(1, Some("51"), 1280, 1280),
            yuv_choice(2, Some("50"), 1280, 1280),
        ];

        let choice = best_choice_for_camera_id(&choices, "50", (1280, 1280)).unwrap();

        assert_eq!(choice.source_index, 2);
        assert_eq!(choice.camera_id.as_deref(), Some("50"));
        assert_eq!((choice.width, choice.height), (1280, 1280));
    }

    #[test]
    fn camera_id_pair_binding_overrides_misleading_source_index() {
        let choices = vec![
            yuv_choice(0, Some("51"), 1280, 1280),
            yuv_choice(1, Some("50"), 1280, 1280),
        ];
        let plan = test_plan();

        let pair = MakepadCameraPair::from_camera2_plan(&choices, &plan).unwrap();

        assert_eq!(pair.source_binding_mode, "camera-id");
        assert_eq!(pair.left.camera_id.as_deref(), Some("50"));
        assert_eq!(pair.right.camera_id.as_deref(), Some("51"));
        assert_eq!(pair.left.source_index, 1);
        assert_eq!(pair.right.source_index, 0);
        assert!(pair.matches_camera2_plan(&plan));
    }

    #[test]
    fn direct_full_frame_profile_marks_source_binding_and_homography() {
        let choices = vec![
            yuv_choice(0, Some("51"), 1280, 1280),
            yuv_choice(1, Some("50"), 1280, 1280),
        ];
        let mut plan = test_plan();

        plan.apply_projection_geometry_profile("full-frame-diagnostic");
        let pair = MakepadCameraPair::from_camera2_plan(&choices, &plan).unwrap();

        assert_eq!(
            pair.source_binding_mode,
            "direct-camera2-full-frame-diagnostic-camera-id"
        );
        assert_eq!(
            pair.left_surface_to_camera_h,
            IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY
        );
        assert_eq!(
            pair.left_screen_to_camera_h,
            IDENTITY_SURFACE_TO_CAMERA_HOMOGRAPHY
        );
        assert!(pair
            .coordinate_chain
            .contains("direct-camera2-full-frame-diagnostic-projection-surface"));
    }

    #[test]
    fn broker_camera_metadata_maps_legacy_physical_profile_to_camera_projection() {
        let metadata = BrokerH264ProjectionMetadata::parse(
            r#"{
                "source": "broker_app.camera2_h264_stream",
                "cameraId": "50",
                "deliveredWidth": 1280,
                "deliveredHeight": 1280,
                "projectionGeometryProfile": "physical-camera",
                "projectionMetadataReady": true,
                "poseSource": "platform",
                "poseCoordinateConvention": "android-camera2-lens-pose-reference-from-camera",
                "intrinsics": {
                    "fx": 1024.0,
                    "fy": 1025.0,
                    "cx": 640.0,
                    "cy": 641.0,
                    "skew": 0.5
                },
                "intrinsicsDomain": {
                    "kind": "activeArray",
                    "width": 4096,
                    "height": 3072
                },
                "extrinsics": {
                    "px": 0.01,
                    "py": 0.02,
                    "pz": 0.03,
                    "qx": 0.0,
                    "qy": 0.0,
                    "qz": 0.0,
                    "qw": 1.0
                }
            }"#,
        )
        .unwrap();

        assert!(metadata.has_camera_projection_metadata());
        assert!(metadata.requests_camera_projection_mapping());
        assert_eq!(
            metadata.projection_mapping_profile_id(),
            "camera-projection"
        );
        assert_eq!(metadata.camera_id, "50");
        assert_eq!(metadata.intrinsics.unwrap().fx, 1024.0);
        assert_eq!(metadata.intrinsics_domain.unwrap().width, 4096);
        assert_eq!(metadata.extrinsics.unwrap().rotation[3], 1.0);
    }

    #[test]
    fn broker_full_frame_camera_metadata_keeps_projection_metadata_authoritative() {
        let metadata = BrokerH264ProjectionMetadata::parse(
            r#"{
                "source": "broker_app.camera2_h264_stream",
                "cameraId": "50",
                "deliveredWidth": 1280,
                "deliveredHeight": 1280,
                "projectionGeometryProfile": "full-frame-diagnostic",
                "contentMappingIntent": "map-camera-frame-to-full-frame-projection-area",
                "projectionMetadataReady": true,
                "poseSource": "platform",
                "poseCoordinateConvention": "android-camera2-lens-pose-reference-from-camera",
                "intrinsics": {
                    "fx": 1024.0,
                    "fy": 1024.0,
                    "cx": 640.0,
                    "cy": 640.0
                },
                "extrinsics": {
                    "px": 0.01,
                    "py": 0.02,
                    "pz": 0.03,
                    "qx": 0.0,
                    "qy": 0.0,
                    "qz": 0.0,
                    "qw": 1.0
                }
            }"#,
        )
        .unwrap();

        assert!(metadata.is_full_frame_diagnostic_projection());
        assert!(metadata.has_camera_projection_metadata());
        assert!(!metadata.requests_explicit_full_frame_content_mapping());
    }

    #[test]
    fn broker_explicit_full_frame_content_intent_is_distinct_from_profile_label() {
        let metadata = BrokerH264ProjectionMetadata::parse(
            r#"{
                "source": "broker_app.synthetic_h264_stream",
                "cameraId": "synthetic-left",
                "deliveredWidth": 1280,
                "deliveredHeight": 1280,
                "projectionGeometryProfile": "full-frame-diagnostic",
                "contentMappingIntent": "map-full-frame-stimulus-to-projection-surface",
                "projectionMetadataReady": true
            }"#,
        )
        .unwrap();

        assert!(metadata.is_full_frame_diagnostic_projection());
        assert!(metadata.requests_explicit_full_frame_content_mapping());
        assert!(!metadata.has_camera_projection_metadata());
    }

    #[test]
    fn broker_orientation_sampling_uses_stimulus_metadata_only() {
        let metadata = BrokerH264ProjectionMetadata::parse(
            r#"{
                "source": "broker_app.camera2_h264_stream",
                "cameraId": "50",
                "deliveredWidth": 1280,
                "deliveredHeight": 1280,
                "orientationKind": "camera-frame",
                "rasterOrientation": "top-left-origin-y-down",
                "orientationMetadataSource": "legacy-stream-field",
                "orientationDefault": false,
                "stimulusRasterOrientation": "bottom-left-origin-y-up",
                "stimulusUprightMarker": "camera-native-upright",
                "stimulusOrientationMetadataSource": "stream-stimulus-contract",
                "stimulusOrientationDefault": false
            }"#,
        )
        .unwrap();

        let decision = FrameOrientationDecision::from_broker_pair(&metadata, &metadata);

        assert_eq!(decision.source_sample_y_flip, 1.0);
        assert_eq!(decision.raster_orientation, FRAME_RASTER_BOTTOM_LEFT_Y_UP);
        assert_eq!(decision.metadata_source, "stream-stimulus-contract");
    }

    #[test]
    fn direct_camera_orientation_sampling_keeps_top_left_stimulus_unflipped() {
        let decision = FrameOrientationDecision::direct_camera2();

        assert_eq!(decision.source_sample_y_flip, 0.0);
        assert_eq!(decision.raster_orientation, FRAME_RASTER_TOP_LEFT_Y_DOWN);
        assert_eq!(
            decision.metadata_source,
            "generated-direct-camera2-stimulus-metadata"
        );
    }

    #[test]
    fn compile_time_source_eye_mapping_is_sanitized() {
        let expected = match option_env!("RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING") {
            Some("display-left-from-left-source") => "display-left-from-left-source",
            Some("display-left-from-right-source") => "display-left-from-right-source",
            _ => DEFAULT_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING,
        };
        assert_eq!(makepad_display_source_eye_mapping(), expected);
    }

    #[test]
    fn default_source_eye_mapping_matches_hwb_and_oes() {
        assert_eq!(
            DEFAULT_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING,
            "display-left-from-left-source"
        );
    }
}

fn makepad_display_source_eye_mapping() -> &'static str {
    match option_env!("RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING") {
        Some("display-left-from-left-source") => "display-left-from-left-source",
        Some("display-left-from-right-source") => "display-left-from-right-source",
        _ => DEFAULT_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING,
    }
}

fn makepad_display_left_from_right_source() -> bool {
    makepad_display_source_eye_mapping() == "display-left-from-right-source"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MakepadProjectionBorderPolicy {
    SolidRed,
    PassthroughUnderlay,
}

impl MakepadProjectionBorderPolicy {
    fn current() -> Self {
        let value = hotload_text(KEY_MAKEPAD_PROJECTION_BORDER_POLICY, "solid-red");
        Self::from_stable_id(&value)
    }

    fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "passthrough-underlay" => Self::PassthroughUnderlay,
            _ => Self::SolidRed,
        }
    }

    fn from_shader_code(value: f32) -> Self {
        if value >= 0.5 {
            Self::PassthroughUnderlay
        } else {
            Self::SolidRed
        }
    }

    fn stable_id(self) -> &'static str {
        match self {
            Self::SolidRed => "solid-red",
            Self::PassthroughUnderlay => "passthrough-underlay",
        }
    }

    fn shared_fill_policy_id(self) -> &'static str {
        match self {
            Self::SolidRed => "solid-color",
            Self::PassthroughUnderlay => "passthrough-underlay",
        }
    }

    fn shader_code(self) -> f32 {
        match self {
            Self::SolidRed => 0.0,
            Self::PassthroughUnderlay => 1.0,
        }
    }

    fn wants_native_passthrough(self) -> bool {
        matches!(self, Self::PassthroughUnderlay)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MakepadSourceColorTransfer {
    Identity,
}

impl MakepadSourceColorTransfer {
    const fn stable_id(self) -> &'static str {
        match self {
            Self::Identity => "identity",
        }
    }

    const fn input_encoding(self) -> &'static str {
        match self {
            Self::Identity => "makepad-sampled-rgb",
        }
    }

    const fn output_encoding(self) -> &'static str {
        match self {
            Self::Identity => "makepad-renderer-native-rgb",
        }
    }

    const fn transform_applied(self) -> bool {
        match self {
            Self::Identity => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MakepadProjectionAlphaMode {
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

impl MakepadProjectionAlphaMode {
    fn current() -> Self {
        let value = hotload_text(KEY_MAKEPAD_PROJECTION_ALPHA_MODE, "fixed");
        Self::from_stable_id(&value)
    }

    fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "red" | "r" | "channel-r" => Self::Red,
            "green" | "g" | "channel-g" => Self::Green,
            "blue" | "b" | "channel-b" => Self::Blue,
            "luma" | "luminance" | "brightness" | "value" => Self::Luma,
            "inverse-red" | "red-inverse" | "inv-red" | "one-minus-red" | "1-red" | "1-r" => {
                Self::InverseRed
            }
            "inverse-green" | "green-inverse" | "inv-green" | "one-minus-green" | "1-green"
            | "1-g" => Self::InverseGreen,
            "inverse-blue" | "blue-inverse" | "inv-blue" | "one-minus-blue" | "1-blue" | "1-b" => {
                Self::InverseBlue
            }
            "inverse-luma" | "luma-inverse" | "inv-luma" | "inverse-brightness"
            | "one-minus-luma" | "1-luma" | "1-brightness" => Self::InverseLuma,
            "red-dominance" | "dominant-red" | "red-key" | "red-chroma" | "red-minus-max" => {
                Self::RedDominance
            }
            "green-dominance" | "dominant-green" | "green-key" | "green-chroma"
            | "green-minus-max" | "screen-green" => Self::GreenDominance,
            "blue-dominance" | "dominant-blue" | "blue-key" | "blue-chroma" | "blue-minus-max" => {
                Self::BlueDominance
            }
            "saturation" | "chroma" | "max-min" | "colorfulness" => Self::Saturation,
            "inverse-saturation"
            | "saturation-inverse"
            | "inverse-chroma"
            | "inv-chroma"
            | "one-minus-saturation"
            | "1-saturation" => Self::InverseSaturation,
            _ => Self::Fixed,
        }
    }

    fn from_shader_code(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::Red,
            2 => Self::Green,
            3 => Self::Blue,
            4 => Self::Luma,
            5 => Self::InverseRed,
            6 => Self::InverseGreen,
            7 => Self::InverseBlue,
            8 => Self::InverseLuma,
            9 => Self::RedDominance,
            10 => Self::GreenDominance,
            11 => Self::BlueDominance,
            12 => Self::Saturation,
            13 => Self::InverseSaturation,
            _ => Self::Fixed,
        }
    }

    fn stable_id(self) -> &'static str {
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

    fn shader_code(self) -> f32 {
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

    fn uses_dynamic_alpha(self) -> bool {
        !matches!(self, Self::Fixed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MakepadProcessingLayer {
    Raw,
    Blur,
}

impl MakepadProcessingLayer {
    fn current() -> Self {
        let value = hotload_text(KEY_MAKEPAD_PROCESSING_LAYER, "raw");
        match value.trim().to_ascii_lowercase().as_str() {
            "blur" => Self::Blur,
            _ => Self::Raw,
        }
    }

    fn stable_id(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Blur => "blur",
        }
    }

    fn shader_code(self) -> f32 {
        match self {
            Self::Raw => 0.0,
            Self::Blur => 1.0,
        }
    }
}

fn makepad_blur_radius_px() -> f32 {
    hotload_f32(KEY_MAKEPAD_BLUR_RADIUS_PX, 2.0, 0.0, 16.0)
}

fn makepad_source_color_contract_fields(transfer: MakepadSourceColorTransfer) -> String {
    format!(
        "sourceColorInputEncoding={} sourceColorTransformStage=post_makepad_source_sample_pre_processing_layer sourceColorTransform={} sourceColorTransformOwner=makepad-camera-panel-shader sourceColorTransformApplied={} sourceColorOutputEncoding={} cameraColorControlStage=none",
        transfer.input_encoding(),
        transfer.stable_id(),
        transfer.transform_applied(),
        transfer.output_encoding()
    )
}

fn makepad_current_source_color_contract_fields() -> String {
    makepad_source_color_contract_fields(MakepadSourceColorTransfer::Identity)
}

fn makepad_projection_depth_meters() -> f32 {
    if makepad_projection_runtime_resolution_enabled() {
        return makepad_current_projection_runtime_float(
            rxrc::KEY_PROJECTION_DEPTH_METERS,
            TARGET_PROJECTION_DEPTH_METERS,
            0.05,
            10.0,
        );
    }
    makepad_legacy_projection_depth_meters()
}

fn makepad_legacy_projection_depth_meters() -> f32 {
    hotload_f32(
        KEY_PROJECTION_DEPTH_METERS,
        TARGET_PROJECTION_DEPTH_METERS,
        0.05,
        10.0,
    )
}

fn makepad_camera_projection_mode() -> String {
    hotload_text(KEY_CAMERA_PROJECTION_MODE, DEFAULT_CAMERA_PROJECTION_MODE)
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn makepad_camera_projection_mode_is_world_canvas() -> bool {
    matches!(
        makepad_camera_projection_mode().as_str(),
        "world-canvas" | "world-canvas-mode" | "world-space-canvas" | "world-space-quad"
    )
}

fn makepad_projection_preview_fov_y_degrees() -> f32 {
    if makepad_projection_runtime_resolution_enabled() {
        return makepad_current_projection_runtime_float(
            rxrc::KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
            TARGET_PROJECTION_PREVIEW_FOV_Y_DEGREES,
            1.0,
            175.0,
        );
    }
    makepad_legacy_projection_preview_fov_y_degrees()
}

fn makepad_legacy_projection_preview_fov_y_degrees() -> f32 {
    hotload_f32(
        KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        TARGET_PROJECTION_PREVIEW_FOV_Y_DEGREES,
        1.0,
        175.0,
    )
}

fn makepad_projection_preview_offset_y_meters() -> f32 {
    if makepad_projection_runtime_resolution_enabled() {
        return makepad_current_projection_runtime_float(
            rxrc::KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
            0.0,
            -2.0,
            2.0,
        );
    }
    makepad_legacy_projection_preview_offset_y_meters()
}

fn makepad_legacy_projection_preview_offset_y_meters() -> f32 {
    hotload_f32(KEY_CAMERA_PREVIEW_OFFSET_Y_METERS, 0.0, -2.0, 2.0)
}

fn makepad_projection_raw_overscan() -> f32 {
    if makepad_projection_runtime_resolution_enabled() {
        return makepad_current_projection_runtime_float(
            rxrc::KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
            TARGET_PROJECTION_RAW_OVERSCAN,
            1.0,
            16.0,
        );
    }
    makepad_legacy_projection_raw_overscan()
}

fn makepad_legacy_projection_raw_overscan() -> f32 {
    hotload_f32(
        KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        TARGET_PROJECTION_RAW_OVERSCAN,
        1.0,
        16.0,
    )
}

fn makepad_projection_panel_geometry() -> ProjectionPanelGeometry {
    let depth_meters = makepad_projection_depth_meters().max(0.05);
    let fov_y_degrees = makepad_projection_preview_fov_y_degrees().clamp(1.0, 175.0);
    let raw_overscan = makepad_projection_raw_overscan().max(1.0);
    let half_height = (fov_y_degrees * 0.5).to_radians().tan() * depth_meters * raw_overscan;
    let height_meters = (half_height * 2.0).max(0.01);
    let width_meters = height_meters * TARGET_DISPLAY_ASPECT.max(0.1);
    let offset_y_meters = makepad_projection_preview_offset_y_meters().clamp(-2.0, 2.0);
    ProjectionPanelGeometry {
        width_meters,
        height_meters,
        depth_meters,
        offset_y_meters,
        z_meters: -depth_meters,
    }
}

fn makepad_projection_area_opacity() -> f32 {
    hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_OPACITY,
        TARGET_PROJECTION_AREA_OPACITY,
        0.0,
        1.0,
    )
}

fn makepad_projection_border_opacity() -> f32 {
    hotload_f32(
        KEY_MAKEPAD_PROJECTION_BORDER_OPACITY,
        TARGET_PROJECTION_BORDER_OPACITY,
        0.0,
        1.0,
    )
}

fn makepad_projection_alpha_scale() -> f32 {
    hotload_f32(KEY_MAKEPAD_PROJECTION_ALPHA_SCALE, 1.0, 0.0, 4.0)
}

fn makepad_projection_alpha_bias() -> f32 {
    hotload_f32(KEY_MAKEPAD_PROJECTION_ALPHA_BIAS, 0.0, -1.0, 1.0)
}

fn makepad_native_passthrough_enabled() -> bool {
    let policy = MakepadProjectionBorderPolicy::current();
    let alpha_mode = MakepadProjectionAlphaMode::current();
    let opacity_needs_passthrough =
        makepad_projection_area_opacity() < 0.999 || makepad_projection_border_opacity() < 0.999;
    hotload_bool(
        KEY_MAKEPAD_NATIVE_PASSTHROUGH_ENABLED,
        policy.wants_native_passthrough()
            || opacity_needs_passthrough
            || alpha_mode.uses_dynamic_alpha(),
    )
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


fn runtime_text(config: &RuntimeConfig, key: &str) -> String {
    config
        .get(key)
        .and_then(RuntimeValue::as_text)
        .unwrap_or("")
        .to_string()
}

fn runtime_float(config: &RuntimeConfig, key: &str) -> f64 {
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

fn hotload_f32(key: &'static str, default: f32, min: f32, max: f32) -> f32 {
    runtime_property_value(key)
        .or_else(|| std::env::var(runtime_env_key(key)).ok())
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn hotload_bool(key: &'static str, default: bool) -> bool {
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

fn hotload_text(key: &'static str, default: &str) -> String {
    runtime_property_value(key)
        .or_else(|| std::env::var(runtime_env_key(key)).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn hotload_u32(key: &'static str, default: u32, min: u32, max: u32) -> u32 {
    runtime_property_value(key)
        .or_else(|| std::env::var(runtime_env_key(key)).ok())
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn hotload_u16(key: &'static str, default: u16, min: u16, max: u16) -> u16 {
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
fn runtime_property_value(key: &'static str) -> Option<String> {
    android_system_property_value(&runtime_property_name(key))
}

#[cfg(target_os = "android")]
fn android_system_property_value(name: &str) -> Option<String> {
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
fn runtime_property_value(_key: &'static str) -> Option<String> {
    None
}

#[cfg(not(target_os = "android"))]
fn android_system_property_value(_name: &str) -> Option<String> {
    None
}

fn marker_token(value: &str) -> String {
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

#[derive(Clone, Copy)]
struct TexturePlaneContentStats {
    format: &'static str,
    readable: bool,
    data_present: bool,
    width: usize,
    height: usize,
    len: usize,
    updated: &'static str,
    sample_count: usize,
    min: u8,
    max: u8,
    mean_x1000: u32,
    nonzero_samples: usize,
}

impl TexturePlaneContentStats {
    fn unreadable(format: &'static str) -> Self {
        Self {
            format,
            readable: false,
            data_present: false,
            width: 0,
            height: 0,
            len: 0,
            updated: "n/a",
            sample_count: 0,
            min: 0,
            max: 0,
            mean_x1000: 0,
            nonzero_samples: 0,
        }
    }

    fn marker_fields(&self, prefix: &str) -> String {
        format!(
            "{}Format={} {}Readable={} {}DataPresent={} {}Width={} {}Height={} {}Len={} {}Updated={} {}SampleCount={} {}Min={} {}Max={} {}MeanX1000={} {}NonZeroSamples={}",
            prefix,
            self.format,
            prefix,
            self.readable,
            prefix,
            self.data_present,
            prefix,
            self.width,
            prefix,
            self.height,
            prefix,
            self.len,
            prefix,
            self.updated,
            prefix,
            self.sample_count,
            prefix,
            self.min,
            prefix,
            self.max,
            prefix,
            self.mean_x1000,
            prefix,
            self.nonzero_samples,
        )
    }
}

fn texture_plane_content_stats(cx: &mut Cx, texture: &Texture) -> TexturePlaneContentStats {
    match texture.get_format(cx) {
        TextureFormat::VecRu8 {
            width,
            height,
            data,
            updated,
            ..
        } => compute_u8_plane_content_stats("VecRu8", *width, *height, data.as_ref(), 1, updated),
        TextureFormat::VecRGu8 {
            width,
            height,
            data,
            updated,
            ..
        } => compute_u8_plane_content_stats("VecRGu8", *width, *height, data.as_ref(), 2, updated),
        TextureFormat::VideoYuvPlane => TexturePlaneContentStats::unreadable("VideoYuvPlane"),
        TextureFormat::VideoExternal => TexturePlaneContentStats::unreadable("VideoExternal"),
        TextureFormat::VideoRgbaHardwareBuffer => {
            TexturePlaneContentStats::unreadable("VideoRgbaHardwareBuffer")
        }
        _ => TexturePlaneContentStats::unreadable("Other"),
    }
}

fn compute_u8_plane_content_stats(
    format: &'static str,
    width: usize,
    height: usize,
    data: Option<&Vec<u8>>,
    bytes_per_sample: usize,
    updated: &TextureUpdated,
) -> TexturePlaneContentStats {
    let Some(data) = data else {
        return TexturePlaneContentStats {
            format,
            readable: true,
            data_present: false,
            width,
            height,
            len: 0,
            updated: texture_updated_label(updated),
            sample_count: 0,
            min: 0,
            max: 0,
            mean_x1000: 0,
            nonzero_samples: 0,
        };
    };

    let bytes_per_sample = bytes_per_sample.max(1);
    let sample_len = data.len() / bytes_per_sample;
    if sample_len == 0 {
        return TexturePlaneContentStats {
            format,
            readable: true,
            data_present: true,
            width,
            height,
            len: data.len(),
            updated: texture_updated_label(updated),
            sample_count: 0,
            min: 0,
            max: 0,
            mean_x1000: 0,
            nonzero_samples: 0,
        };
    }

    let step = (sample_len / 4096).max(1);
    let mut min_value = u8::MAX;
    let mut max_value = u8::MIN;
    let mut sum = 0_u64;
    let mut sample_count = 0_usize;
    let mut nonzero_samples = 0_usize;

    let mut sample_index = 0_usize;
    while sample_index < sample_len {
        let value = data[sample_index * bytes_per_sample];
        min_value = min_value.min(value);
        max_value = max_value.max(value);
        sum += value as u64;
        sample_count += 1;
        if value != 0 {
            nonzero_samples += 1;
        }
        sample_index = sample_index.saturating_add(step);
    }

    let mean_x1000 = if sample_count == 0 {
        0
    } else {
        ((sum * 1000) / sample_count as u64) as u32
    };

    TexturePlaneContentStats {
        format,
        readable: true,
        data_present: true,
        width,
        height,
        len: data.len(),
        updated: texture_updated_label(updated),
        sample_count,
        min: min_value,
        max: max_value,
        mean_x1000,
        nonzero_samples,
    }
}

fn texture_updated_label(updated: &TextureUpdated) -> &'static str {
    match updated {
        TextureUpdated::Empty => "empty",
        TextureUpdated::Partial(_) => "partial",
        TextureUpdated::Full => "full",
    }
}

fn marker_line_with_runtime_projection_target_fields(line: &str) -> std::borrow::Cow<'_, str> {
    const LEGACY_TARGET_FIELDS: &str =
        "panelTargetPreviewFovYDegrees=60 panelTargetRawOverscan=1.06";
    if line.contains(LEGACY_TARGET_FIELDS) {
        std::borrow::Cow::Owned(line.replace(
            LEGACY_TARGET_FIELDS,
            &makepad_projection_target_marker_fields(),
        ))
    } else {
        std::borrow::Cow::Borrowed(line)
    }
}

#[cfg(target_os = "android")]
fn emit_marker_line(line: &str) {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};

    const ANDROID_LOG_INFO: c_int = 4;

    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    }

    let line = marker_line_with_runtime_projection_target_fields(line);
    let tag = CString::new("RustyXRMakepad");
    let msg = CString::new(line.as_ref());
    if let (Ok(tag), Ok(msg)) = (tag, msg) {
        unsafe {
            __android_log_write(ANDROID_LOG_INFO, tag.as_ptr(), msg.as_ptr());
        }
    }
}

#[cfg(not(target_os = "android"))]
fn emit_marker_line(line: &str) {
    let line = marker_line_with_runtime_projection_target_fields(line);
    log!("{}", line.as_ref());
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        Self::emit_startup_markers_once("startup");
        let config = Self::runtime_config();
        cx.xr_set_native_passthrough(makepad_native_passthrough_enabled());
        cx.xr_set_render_scale(runtime_float(&config, KEY_XR_RENDER_SCALE) as f32);
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self
            .ui
            .button(cx, ids!(emit_marker_button))
            .clicked(actions)
        {
            Self::emit_status_marker("button");
        }
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        crate::makepad_widgets::script_mod(vm);
        makepad_xr::script_mod(vm);
        self::script_mod(vm)
    }

    fn after_new_from_script(_vm: &mut ScriptVm, _app: &mut Self) {
        Self::emit_startup_markers_once("startup");
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.handle_cadence_event(cx, event);
        self.handle_paired_import_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}

fn rate_hz(count: u64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        0.0
    } else {
        count as f64 / seconds
    }
}
