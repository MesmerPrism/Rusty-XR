pub use makepad_xr::makepad_widgets;

#[cfg(target_os = "android")]
mod acamera_sys;
#[cfg(target_os = "android")]
mod android_camera_probe;

use makepad_widgets::makepad_platform::{
    event::video_playback::{CameraPreviewMode, VideoSource, VideoYuvMetadata},
    permission::Permission,
    thread::SignalToUI,
    video::{VideoFormat, VideoInputsEvent, VideoPixelFormat},
    TextureFormat, TextureId, TextureUpdated,
};
use makepad_widgets::*;
use makepad_xr::scene::{xr_widget_world_transform, XrNode};
use rusty_xr_runtime_config::{RuntimeConfig, RuntimeConfigSource, RuntimeValue};
use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
    time::Duration,
};

app_main!(App);

static STARTUP_MARKERS_EMITTED: AtomicBool = AtomicBool::new(false);
static PAIRED_IMPORT_SIGNAL_READY: AtomicBool = AtomicBool::new(false);
static CAMERA_PANEL_DRAW_MARKER_EMITTED: AtomicBool = AtomicBool::new(false);
static VIDEO_EVENT_RAW_MARKERS_EMITTED: AtomicUsize = AtomicUsize::new(0);
static TEXTURE_UPDATE_MARKERS_EMITTED: AtomicUsize = AtomicUsize::new(0);
static TEXTURE_CONTENT_PROBE_MARKERS_EMITTED: AtomicUsize = AtomicUsize::new(0);

const DEFAULT_PROFILE: &str = "makepad-stereo-projection-pair-probe";
const DEFAULT_TRANSPORT: &str = "makepad-s68-active-eye-nonworld-panel-control";
const DEFAULT_CAMERA_TIER: &str = "native-camera2-makepad-stereo-vulkan-import-probe";
const DEFAULT_CAMERA_PROJECTION_MODE: &str = "display-screen-homography";
const DEFAULT_COMPARISON_BASELINE: &str = "custom-apk-camera-stereo-gpu-composite";
const DEFAULT_SYNTHETIC_SCENE: &str = "camera-panel-s68-active-eye-nonworld-placement";
const DEFAULT_ACQUISITION_PROFILE: &str =
    "bounded-camera2-private-plus-makepad-paired-import-probe";
const DEFAULT_PROJECTION_SCALE: f64 = 0.75;
const DEFAULT_XR_RENDER_SCALE: f64 = 0.75;
const MAKEPAD_BRANCH: &str = "rusty-xr/android-libstd-packaging";
const MAKEPAD_REV: &str = "c3ea53e";
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
const KEY_XR_RENDER_SCALE: &str = "xr_render_scale";
const KEY_RENDERER: &str = "renderer";
const KEY_ANDROID_PACKAGER: &str = "android_packager";
const KEY_MAKEPAD_REVISION: &str = "makepad_revision";
const KEY_MAKEPAD_BRANCH: &str = "makepad_branch";
const KEY_STUDIO_HOST: &str = "studio_host";

script_mod! {
    use mod.pod.*
    use mod.math.*
    use mod.shader.*
    use mod.draw
    use mod.geom
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.draw.DrawMakepadStereoCameraPanel = mod.std.set_type_default() do #(DrawMakepadStereoCameraPanel::script_shader(vm)){
        ..mod.draw.DrawCube
        backface_culling: false

        left_camera_texture: texture_video()
        right_camera_texture: texture_video()
        left_tex_y: texture_2d(float)
        left_tex_u: texture_2d(float)
        left_tex_v: texture_2d(float)
        right_tex_y: texture_2d(float)
        right_tex_u: texture_2d(float)
        right_tex_v: texture_2d(float)
        v_uv: varying(vec2f)

        vertex: fn() {
            let pos = self.get_size() * self.geom.geom_pos + self.get_pos();
            let eye_origin = self.draw_pass.camera_inv * vec4(0.0, 0.0, 0.0, 1.0);
            let eye_right = self.draw_pass.camera_inv * vec4(1.0, 0.0, 0.0, 0.0);
            let eye_up = self.draw_pass.camera_inv * vec4(0.0, 1.0, 0.0, 0.0);
            let eye_forward = self.draw_pass.camera_inv * vec4(0.0, 0.0, -1.0, 0.0);
            let panel_center = eye_origin.xyz + eye_forward.xyz * 0.72;
            let panel_pos = panel_center + eye_right.xyz * pos.x + eye_up.xyz * pos.y + eye_forward.xyz * pos.z;
            self.world = vec4(panel_pos.x, panel_pos.y, panel_pos.z, 1.0);
            self.v_uv = self.geom.geom_uv;
            let view_pos = self.draw_pass.camera_view * self.world;
            self.vertex_pos = self.draw_pass.camera_projection * view_pos;
        }

        active_eye_is_right: fn() -> float {
            return clamp(xr_view_id(), 0.0, 1.0);
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

        guide_mask: fn(coord: vec2f) -> float {
            let edge_x = min(coord.x, 1.0 - coord.x);
            let edge_y = min(coord.y, 1.0 - coord.y);
            let border = 1.0 - step(0.015, min(edge_x, edge_y));
            return clamp(border, 0.0, 1.0);
        }

        pixel: fn() {
            let panel_uv = clamp(self.v_uv, vec2(0.0, 0.0), vec2(1.0, 1.0));
            let proof_guide = self.guide_mask(panel_uv);
            let sample_uv = vec2(panel_uv.x, 1.0 - panel_uv.y);
            let eye_selector = self.active_eye_is_right();
            let left_y = self.left_tex_y.sample(sample_uv).x;
            let left_u = self.left_tex_u.sample(sample_uv).x;
            let left_v = self.left_tex_v.sample(sample_uv).x;
            let right_y = self.right_tex_y.sample(sample_uv).x;
            let right_u = self.right_tex_u.sample(sample_uv).x;
            let right_v = self.right_tex_v.sample(sample_uv).x;
            let y_val = mix(left_y, right_y, eye_selector);
            let u_val = mix(left_u, right_u, eye_selector);
            let v_val = mix(left_v, right_v, eye_selector);
            let direct_rgb = self.yuv_to_rgb_limited_601(y_val, u_val, v_val);
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
        size: vec3(1.36, 0.88, 0.010)
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
                pos: vec3(0.0, 0.92, -0.762)

                camera_projection_panel := mod.widgets.MakepadStereoCameraPanel{
                    body: mod.widgets.XrBodyKind.Fixed
                    size: vec3(1.36, 0.88, 0.010)
                    pos: vec3(0.0, 0.0, 0.0)
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
    pub draw_super: DrawCube,
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
    #[live(0.0_f32)]
    pub depth_clip: f32,
}

impl DrawMakepadStereoCameraPanel {
    fn assign_texture_slot(&mut self, slot: usize, texture: Option<Texture>) {
        match texture {
            Some(texture) => self.draw_super.draw_vars.set_texture(slot, &texture),
            None => self.draw_super.draw_vars.empty_texture(slot),
        }
    }

    fn set_camera_textures(&mut self, cx: &mut Cx, left: Option<Texture>, right: Option<Texture>) {
        self.assign_texture_slot(0, left);
        self.assign_texture_slot(1, right);
        self.draw_super.draw_vars.redraw(cx);
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
        self.draw_super.draw_vars.redraw(cx);
    }

    fn draw(&mut self, cx: &mut CxDraw) {
        self.draw_super.draw(cx);
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

impl MakepadStereoCameraPanel {
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

    fn set_camera_textures(
        &mut self,
        cx: &mut Cx,
        left: Option<Texture>,
        right: Option<Texture>,
        left_yuv: Option<MakepadCameraYuvTextures>,
        right_yuv: Option<MakepadCameraYuvTextures>,
        left_rotation_steps: f32,
        right_rotation_steps: f32,
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
        self.draw_panel.camera_ready = 1.0;
        self.draw_panel.texture_probe_mode = 2.0;
        self.draw_panel.draw_super.draw_vars.redraw(cx);
        self.draw_panel.draw_super.draw_vars.set_instance_on_area(
            cx,
            live_id!(camera_ready),
            &[1.0],
        );
        self.draw_panel.draw_super.draw_vars.set_uniform_on_area(
            cx,
            live_id!(camera_ready),
            &[1.0],
        );
        self.draw_panel
            .draw_super
            .draw_vars
            .set_instance_on_area(cx, live_id!(yuv_mode), &[1.0]);
        self.draw_panel
            .draw_super
            .draw_vars
            .set_uniform_on_area(cx, live_id!(yuv_mode), &[1.0]);
        self.draw_panel.draw_super.draw_vars.set_instance_on_area(
            cx,
            live_id!(proof_tint_strength),
            &[0.0],
        );
        self.draw_panel.draw_super.draw_vars.set_uniform_on_area(
            cx,
            live_id!(proof_tint_strength),
            &[0.0],
        );
        self.draw_panel.draw_super.draw_vars.set_instance_on_area(
            cx,
            live_id!(texture_probe_mode),
            &[2.0],
        );
        self.draw_panel.draw_super.draw_vars.set_uniform_on_area(
            cx,
            live_id!(texture_probe_mode),
            &[2.0],
        );
        self.camera_ready = true;
        self.node.redraw(cx);
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
                "RUSTY_XR_MAKEPAD_STEREO_PROJECTION schema=rusty.xr.makepad-stereo-projection.v1 phase=visible-panel-draw status=ok visibleCameraPanelDrawn=true cameraTextureReady={} renderPath=makepad-xr sceneOwnedPanel=true projectionShaderPath=makepad-s68-active-eye-nonworld-panel-control textureProbeMode=s68-active-eye-nonworld-panel-control syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 colorReference=android-yuv420-888-plane-order perEyeTextureSelection=true activeEyeSelector=xr_view_id diagnosticPanelPlacement=active-eye-camera-inv-forward s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true nativePassthrough=false backgroundClearColor=203040 diagnosticUvTransform=flip-y diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=true diagnosticSolidPanel=false debugAlignmentGuide=false borderOnlyGuide=true paleBorderGuide=true proofTintStrength=0.0 neutralWaitingPanel=true visualIsolation=s68_active_eye_nonworld_panel_on_passthrough_off_background depthClip=false environmentDepthClip=false visualInspection=required visualReleaseAccepted=false",
                self.camera_ready
            ));
        }
        let world = xr_widget_world_transform(cx, scope, self.widget_uid(), &self.node);
        self.draw_panel.draw_super.transform = world;
        self.draw_panel.draw_super.cube_pos = vec3f(0.0, 0.0, 0.0);
        self.draw_panel.draw_super.cube_size = self.size;
        self.draw_panel.draw_super.depth_clip = 0.0;
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
        Self::start_camera_probe_once();
    }

    fn emit_status_marker(phase: &str) {
        let config = Self::runtime_config();

        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_Q2Q_STATUS schema=rusty.xr.makepad-q2q.status.v1 phase={} profile={} transport={} renderer=makepad android_packager=cargo-makepad makepad_rev={} studio_host={}",
            phase,
            runtime_text(&config, KEY_RUNTIME_PROFILE),
            runtime_text(&config, KEY_TRANSPORT_PROFILE),
            runtime_text(&config, KEY_MAKEPAD_REVISION),
            runtime_text(&config, KEY_STUDIO_HOST)
        ));
    }

    fn emit_stereo_comparison_marker(phase: &str) {
        let config = Self::runtime_config();

        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_STEREO_COMPARISON schema=rusty.xr.makepad-stereo-comparison.v1 phase={} profile={} comparisonBaseline={} cameraTier={} acquisition={} transport={} projectionMode={} syntheticScene={} leftEyeSource=synthetic-left rightEyeSource=synthetic-right sourceEyeMapping=display-eye projectionScale={:.2} xrRenderScale={:.2} pairedLeftRightGpuBuffers=false alignedProjection=false renderPath=makepad-xr makepadForkBranch={} makepadForkCommit={}",
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
            runtime_text(&config, KEY_MAKEPAD_REVISION)
        ));
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
            std::env::var("RUSTY_XR_CAMERA_PROJECTION_MODE")
                .unwrap_or_else(|_| DEFAULT_CAMERA_PROJECTION_MODE.to_string()),
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
            env_f64("RUSTY_XR_PROJECTION_SCALE", DEFAULT_PROJECTION_SCALE),
            RuntimeConfigSource::Environment,
        );
        set_runtime_float(
            &mut config,
            KEY_XR_RENDER_SCALE,
            env_f64("RUSTY_XR_RENDER_SCALE", DEFAULT_XR_RENDER_SCALE),
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

    fn handle_cadence_event(&mut self, cx: &mut Cx, event: &Event) {
        if matches!(event, Event::Startup) && self.cadence_next_frame.is_none() {
            self.arm_cadence_probe(cx);
            return;
        }

        match event {
            Event::XrUpdate(_) => {
                self.cadence_xr_update_count = self.cadence_xr_update_count.saturating_add(1);
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
            .map(|pair| pair.projection_metadata_ready)
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

    fn handle_paired_import_event(&mut self, cx: &mut Cx, event: &Event) {
        match event {
            Event::Startup => {
                cx.request_permission(Permission::Camera);
                cx.request_permission(Permission::HeadsetCamera);
                self.arm_paired_import_timer(cx, PAIRED_IMPORT_DELAY_SECONDS, "startup");
            }
            Event::VideoInputs(inputs) => {
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
            Event::VideoYuvTexturesReady(ready) => {
                emit_raw_video_event_marker("yuv-textures-ready", ready.video_id);
                if let Some(side) = StereoEye::from_video_id(ready.video_id) {
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
            Event::VideoPlaybackPrepared(prepared) => {
                emit_raw_video_event_marker("prepared", prepared.video_id);
                if let Some(side) = StereoEye::from_video_id(prepared.video_id) {
                    match side {
                        StereoEye::Left => self.paired_import_left_prepared = true,
                        StereoEye::Right => self.paired_import_right_prepared = true,
                    }
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=prepared status=ok side={} width={} height={} importPath=makepad-android-camera-yuv-plane-cpu-proof textureMode=yuv-plane importPlan=single-stream-yuv-proof",
                        side.label(),
                        prepared.video_width,
                        prepared.video_height,
                    ));
                    self.emit_paired_projection_progress("prepared");
                }
            }
            Event::VideoTextureUpdated(updated) => {
                emit_raw_video_event_marker("texture-updated", updated.video_id);
                if let Some(side) = StereoEye::from_video_id(updated.video_id) {
                    self.record_camera_texture_update(side, updated.current_position_ms);
                    self.emit_yuv_texture_content_probe(cx, side, updated.yuv);
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
                            "phase=texture-updated status=ok side={} makepadVulkanImport=false yuvEnabled={} yuvBiplanar={} rotationSteps={:.0} importPlan=single-stream-yuv-proof cpuUploadPath=makepad-camera-cpu-yuv-plane",
                            side.label(),
                            updated.yuv.enabled,
                            updated.yuv.biplanar,
                            updated.yuv.rotation_steps,
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

    fn try_start_paired_import(&mut self, cx: &mut Cx) {
        if self.paired_import_started || self.paired_import_finished {
            return;
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

        Self::emit_hardware_buffer_import_marker(&format!(
            "phase=start status=started importPlan=single-stream-yuv-proof leftSourceIndex={} rightSourceIndex={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftFrameRate={} rightFrameRate={} pixelFormat={} importPath=makepad-android-camera-yuv-plane-cpu-proof textureFormat=VideoYuvPlane depthClip=false environmentDepthClip=false delayedAfterAcquisitionSeconds={:.0}",
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
            PAIRED_IMPORT_DELAY_SECONDS,
        ));
        Self::emit_stereo_projection_marker(&format!(
            "phase=start status=started pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} leftSourceIndex={} rightSourceIndex={} projectionMode={} projectionScale={:.2} xrRenderScale={:.2} fallbackReason={}",
            pair.projection_metadata_ready,
            pair.projection_metadata_ready,
            pair.pose_source,
            pair.source_eye_mapping,
            pair.coordinate_chain,
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
                "phase=enumerated status=ok makepadSourceCount={} makepadFormatCount={} selected=true importPlan=paired-makepad-video-hardware-buffer leftSourceIndex={} rightSourceIndex={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftFrameRate={} rightFrameRate={} pixelFormat={}",
                source_count,
                format_count,
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
            ));
                Self::emit_stereo_projection_marker(&format!(
                    "phase=enumerated status=ok makepadSourceCount={} makepadFormatCount={} pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} leftSourceIndex={} rightSourceIndex={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} fallbackReason={}",
                    source_count,
                    format_count,
                    pair.projection_metadata_ready,
                    pair.projection_metadata_ready,
                    pair.pose_source,
                    pair.source_eye_mapping,
                    pair.coordinate_chain,
                    pair.left.source_index,
                    pair.right.source_index,
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
            "phase={} status=progress leftPrepared={} rightPrepared={} leftUpdated={} rightUpdated={} pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} leftSourceIndex={} rightSourceIndex={} fallbackReason={}",
            phase,
            self.paired_import_left_prepared,
            self.paired_import_right_prepared,
            self.paired_import_left_updated,
            self.paired_import_right_updated,
            pair.projection_metadata_ready,
            pair.projection_metadata_ready,
            pair.pose_source,
            pair.source_eye_mapping,
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
                "phase=texture-content-probe status=missing side={} textureProbeMode=s68-active-eye-nonworld-panel-control syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true nativePassthrough=false backgroundClearColor=203040 yuvEnabled={} yuvBiplanar={} yuvMatrix={:.1} rotationSteps={:.0} cpuPlaneContentPresent=false visualInspection=required visualReleaseAccepted=false",
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
            "phase=texture-content-probe status=ok side={} textureProbeMode=s68-active-eye-nonworld-panel-control syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true nativePassthrough=false backgroundClearColor=203040 yuvEnabled={} yuvBiplanar={} yuvMatrix={:.1} rotationSteps={:.0} cpuPlaneContentPresent={} {} {} {} gpuSamplingStillVisual=s68-active-eye-nonworld-panel-yuv visualInspection=required visualReleaseAccepted=false",
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
        let (Some(left_yuv), Some(right_yuv)) = (left_yuv_source, right_yuv_source) else {
            if !self.camera_projection_bind_error_logged {
                Self::emit_stereo_projection_marker(
                    "phase=visible-panel-bound status=waiting visibleCameraProjectionReady=false fallbackReason=makepad_camera_yuv_plane_textures_missing",
                );
                self.camera_projection_bind_error_logged = true;
            }
            return false;
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

        panel.set_camera_textures(
            cx,
            Some(left_texture),
            Some(right_texture),
            Some(left_yuv),
            Some(right_yuv),
            self.paired_import_left_rotation_steps,
            self.paired_import_right_rotation_steps,
        );
        self.camera_projection_textures_bound = true;
        self.camera_projection_paired_textures_bound = !single_stream_visual_proof;
        Self::emit_stereo_projection_marker(&format!(
            "phase=draw-vars-bound status=ok cameraReady=true yuvMode=true proofTintStrength=0.0 neutralWaitingPanel=true textureProbeMode=s68-active-eye-nonworld-panel-control syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id drawVarsTextureRedraw=true shaderAreaStateUpdate=true leftYuvTextureBound=true rightYuvTextureBound=true singleStreamVisualProof={} updatedStreamVisualProofSide={} visibleCameraProjectionReady=true sceneOwnedPanel=true projectionShaderPath=makepad-s68-active-eye-nonworld-panel-control diagnosticPanelPlacement=active-eye-camera-inv-forward s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true nativePassthrough=false backgroundClearColor=203040 diagnosticUvTransform=flip-y diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=true borderOnlyGuide=true paleBorderGuide=true depthClip=false environmentDepthClip=false visualInspection=required visualReleaseAccepted=false",
            single_stream_visual_proof,
            proof_source_side,
        ));
        if !self.synthetic_scene_hidden_for_camera {
            self.synthetic_scene_hidden_for_camera = true;
            Self::emit_stereo_projection_marker(
                "phase=synthetic-scene-hidden status=ok visibleCameraProjectionReady=true fallbackSceneVisible=false fallbackReason=makepad_synthetic_scene_removed_for_visual_gate",
            );
        }
        Self::emit_stereo_projection_marker(&format!(
            "phase=visible-panel-bound status=ok visibleCameraProjectionReady=true eyeSelection=per-eye-direct-camera-yuv-color-limited601-noswap-border sourceEyeMapping={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} leftRotationSteps={:.0} rightRotationSteps={:.0} sceneOwnedPanel=true projectionShaderPath=makepad-s68-active-eye-nonworld-panel-control textureProbeMode=s68-active-eye-nonworld-panel-control syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 colorReference=android-yuv420-888-plane-order perEyeTextureSelection=true activeEyeSelector=xr_view_id diagnosticPanelPlacement=active-eye-camera-inv-forward s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true nativePassthrough=false backgroundClearColor=203040 diagnosticUvTransform=flip-y diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=true diagnosticSolidPanel=false debugAlignmentGuide=false borderOnlyGuide=true paleBorderGuide=true proofTintStrength=0.0 neutralWaitingPanel=true visualIsolation=s68_active_eye_nonworld_panel_on_passthrough_off_background depthClip=false environmentDepthClip=false singleStreamVisualProof={} updatedStreamVisualProofSide={} cpuUploadPath=makepad-camera-cpu-yuv-plane drawVarsTextureRedraw=true shaderAreaStateUpdate=true visualInspection=required visualReleaseAccepted=false",
            pair.source_eye_mapping,
            pair.left.source_index,
            pair.right.source_index,
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
        let single_stream_ready = (self.paired_import_left_updated
            || self.paired_import_right_updated)
            && (self.paired_import_left_yuv_textures.is_some()
                || self.paired_import_right_yuv_textures.is_some());
        if !paired_streams_ready && !single_stream_ready {
            self.emit_paired_projection_progress("texture-updated");
            return;
        }

        let Some(pair) = self.paired_import_choice.clone() else {
            return;
        };
        if !paired_streams_ready {
            let visible_projection_ready = self.bind_camera_projection_panel(cx);
            if !self.camera_projection_single_stream_logged {
                self.camera_projection_single_stream_logged = true;
                Self::emit_stereo_projection_marker(&format!(
                    "phase=single-stream-proof status=waiting pairedLeftRightCameraFrames=false singleStreamCameraPixels=true leftUpdated={} rightUpdated={} leftYuvReady={} rightYuvReady={} projectionMappingReady={} alignedProjection=false visibleCameraProjectionReady={} sceneOwnedPanel=true projectionShaderPath=makepad-s68-active-eye-nonworld-panel-control textureProbeMode=s68-active-eye-nonworld-panel-control syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id diagnosticPanelPlacement=active-eye-camera-inv-forward s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true nativePassthrough=false backgroundClearColor=203040 diagnosticUvTransform=flip-y diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=true proofTintStrength=0.0 neutralWaitingPanel=true borderOnlyGuide=true paleBorderGuide=true depthClip=false environmentDepthClip=false drawVarsTextureRedraw=true shaderAreaStateUpdate=true updatedStreamVisualProofSide={} visualInspection=required visualReleaseAccepted=false fallbackReason=waiting_for_second_cpu_yuv_stream",
                    self.paired_import_left_updated,
                    self.paired_import_right_updated,
                    self.paired_import_left_yuv_textures.is_some(),
                    self.paired_import_right_yuv_textures.is_some(),
                    pair.projection_metadata_ready,
                    visible_projection_ready,
                    updated_stream_visual_proof_side,
                ));
            }
            return;
        }
        self.paired_import_finished = true;
        let aligned_projection = pair.projection_metadata_ready && paired_streams_ready;
        let visible_projection_ready = self.bind_camera_projection_panel(cx);
        Self::emit_stereo_projection_marker(&format!(
            "phase=complete status=ok pairedLeftRightCameraFrames=true makepadVulkanImport=false projectionMappingReady={} alignedProjection={} visibleCameraProjectionReady={} projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} projectionMode={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftRotationSteps={:.0} rightRotationSteps={:.0} projectionScale={:.2} xrRenderScale={:.2} renderPath=makepad-xr projectionShaderPath=makepad-s68-active-eye-nonworld-panel-control textureProbeMode=s68-active-eye-nonworld-panel-control syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id diagnosticPanelPlacement=active-eye-camera-inv-forward s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true nativePassthrough=false backgroundClearColor=203040 diagnosticUvTransform=flip-y diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=true cpuUploadPath=makepad-camera-cpu-yuv-plane debugAlignmentGuide=false borderOnlyGuide=true paleBorderGuide=true proofTintStrength=0.0 neutralWaitingPanel=true visualIsolation=s68_active_eye_nonworld_panel_on_passthrough_off_background depthClip=false environmentDepthClip=false drawVarsTextureRedraw=true shaderAreaStateUpdate=true visualInspection=required visualReleaseAccepted=false fallbackReason={}",
            pair.projection_metadata_ready,
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
        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_STEREO_COMPARISON schema=rusty.xr.makepad-stereo-comparison.v1 phase={} profile={} comparisonBaseline={} cameraTier={} acquisition={} transport={} projectionMode={} syntheticScene={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} sourceEyeMapping={} projectionScale={:.2} xrRenderScale={:.2} pairedLeftRightCameraFrames=true alignedProjection={} visibleCameraProjectionReady={} renderPath=makepad-xr projectionShaderPath=makepad-s68-active-eye-nonworld-panel-control textureProbeMode=s68-active-eye-nonworld-panel-control syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 colorReference=android-yuv420-888-plane-order perEyeTextureSelection=true activeEyeSelector=xr_view_id diagnosticPanelPlacement=active-eye-camera-inv-forward s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true nativePassthrough=false backgroundClearColor=203040 diagnosticUvTransform=flip-y diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=true makepadForkBranch={} makepadForkCommit={} debugAlignmentGuide=false borderOnlyGuide=true paleBorderGuide=true proofTintStrength=0.0 neutralWaitingPanel=true visualIsolation=s68_active_eye_nonworld_panel_on_passthrough_off_background depthClip=false environmentDepthClip=false cpuUploadPath=makepad-camera-cpu-yuv-plane drawVarsTextureRedraw=true shaderAreaStateUpdate=true visualInspection=required visualReleaseAccepted=false",
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
        Self::camera2_stereo_plan()
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
            desc.formats.iter().filter_map(move |format| {
                (format.pixel_format == VideoPixelFormat::YUV420).then(|| {
                    MakepadCameraChoice::new(
                        source_index,
                        desc.input_id,
                        *format,
                        camera_source_class(&desc.name),
                    )
                })
            })
        })
        .collect()
}

#[derive(Clone)]
struct MakepadCameraChoice {
    source_index: usize,
    input_id: makepad_widgets::makepad_platform::video::VideoInputId,
    format_id: makepad_widgets::makepad_platform::video::VideoFormatId,
    source_class: &'static str,
    width: usize,
    height: usize,
    frame_rate: Option<f64>,
    pixel_format: VideoPixelFormat,
}

impl MakepadCameraChoice {
    fn new(
        source_index: usize,
        input_id: makepad_widgets::makepad_platform::video::VideoInputId,
        format: VideoFormat,
        source_class: &'static str,
    ) -> Self {
        Self {
            source_index,
            input_id,
            format_id: format.format_id,
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
    pose_source: String,
    source_eye_mapping: String,
    coordinate_chain: String,
    fallback_reason: String,
}

impl MakepadCameraPair {
    fn from_camera2_plan(
        choices: &[MakepadCameraChoice],
        plan: &Camera2StereoPlan,
    ) -> Option<Self> {
        let left = best_choice_for_source_index(choices, plan.left_source_index, plan.size())?;
        let right = best_choice_for_source_index(choices, plan.right_source_index, plan.size())?;
        if left.source_index == right.source_index {
            return None;
        }

        Some(Self {
            left,
            right,
            projection_metadata_ready: plan.projection_metadata_ready,
            pose_source: plan.pose_source.clone(),
            source_eye_mapping: plan.source_eye_mapping.clone(),
            coordinate_chain: plan.coordinate_chain.clone(),
            fallback_reason: plan.fallback_reason.clone(),
        })
    }

    fn from_best_available_pair(choices: &[MakepadCameraChoice]) -> Option<Self> {
        let mut best: Option<(
            MakepadCameraChoice,
            MakepadCameraChoice,
            (i32, i64, i64, i64, i64),
        )> = None;

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
            pose_source: "missing".to_string(),
            source_eye_mapping: "display-eye-by-makepad-heuristic".to_string(),
            coordinate_chain: "unresolved".to_string(),
            fallback_reason: "camera2 stereo projection metadata was not correlated".to_string(),
        })
    }
}

#[derive(Clone)]
struct Camera2StereoPlan {
    left_source_index: usize,
    right_source_index: usize,
    width: u32,
    height: u32,
    projection_metadata_ready: bool,
    pose_source: String,
    source_eye_mapping: String,
    coordinate_chain: String,
    fallback_reason: String,
}

impl Camera2StereoPlan {
    fn size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }
}

#[cfg(target_os = "android")]
impl From<android_camera_probe::StereoProjectionPlan> for Camera2StereoPlan {
    fn from(plan: android_camera_probe::StereoProjectionPlan) -> Self {
        Self {
            left_source_index: plan.left_source_index,
            right_source_index: plan.right_source_index,
            width: plan.width,
            height: plan.height,
            projection_metadata_ready: plan.projection_metadata_ready,
            pose_source: plan.pose_source.to_string(),
            source_eye_mapping: plan.source_eye_mapping.to_string(),
            coordinate_chain: plan.coordinate_chain.to_string(),
            fallback_reason: plan.fallback_reason.to_string(),
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

fn source_class_rank(source_class: &str) -> i32 {
    match source_class {
        "back" => 3,
        "external" => 2,
        "front" => 1,
        _ => 0,
    }
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

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
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

#[cfg(target_os = "android")]
fn emit_marker_line(line: &str) {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};

    const ANDROID_LOG_INFO: c_int = 4;

    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    }

    let tag = CString::new("RustyXRMakepad");
    let msg = CString::new(line);
    if let (Ok(tag), Ok(msg)) = (tag, msg) {
        unsafe {
            __android_log_write(ANDROID_LOG_INFO, tag.as_ptr(), msg.as_ptr());
        }
    }
}

#[cfg(not(target_os = "android"))]
fn emit_marker_line(line: &str) {
    log!("{}", line);
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        Self::emit_startup_markers_once("startup");
        let config = Self::runtime_config();
        cx.xr_set_native_passthrough(false);
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
