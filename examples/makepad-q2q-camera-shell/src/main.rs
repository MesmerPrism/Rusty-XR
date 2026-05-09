pub use makepad_xr::makepad_widgets;

#[cfg(target_os = "android")]
mod acamera_sys;
#[cfg(target_os = "android")]
mod android_camera_probe;

use makepad_widgets::makepad_platform::{
    event::video_playback::{CameraPreviewMode, VideoSource},
    permission::Permission,
    video::{VideoFormat, VideoInputsEvent, VideoPixelFormat},
};
use makepad_widgets::*;
use rusty_xr_runtime_config::{RuntimeConfig, RuntimeConfigSource, RuntimeValue};
use std::sync::atomic::{AtomicBool, Ordering};

app_main!(App);

static STARTUP_MARKERS_EMITTED: AtomicBool = AtomicBool::new(false);

const DEFAULT_PROFILE: &str = "makepad-camera2-acquisition-probe";
const DEFAULT_TRANSPORT: &str = "synthetic";
const DEFAULT_CAMERA_TIER: &str = "native-camera2-makepad-vulkan-import-probe";
const DEFAULT_CAMERA_PROJECTION_MODE: &str = "synthetic-stereo-panels";
const DEFAULT_COMPARISON_BASELINE: &str = "custom-apk-camera-stereo-gpu-composite";
const DEFAULT_SYNTHETIC_SCENE: &str = "dual-panel-grid-v1";
const DEFAULT_ACQUISITION_PROFILE: &str = "bounded-camera2-private-plus-makepad-import-probe";
const DEFAULT_PROJECTION_SCALE: f64 = 0.75;
const DEFAULT_XR_RENDER_SCALE: f64 = 0.75;
const MAKEPAD_BRANCH: &str = "rusty-xr/android-libstd-packaging";
const MAKEPAD_REV: &str = "aebeabf32278";
const HARDWARE_BUFFER_IMPORT_DELAY_SECONDS: f64 = 6.0;
const HARDWARE_BUFFER_IMPORT_RETRY_SECONDS: f64 = 1.0;
const HARDWARE_BUFFER_IMPORT_MAX_WAITS: usize = 10;
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
    use mod.prelude.widgets.*
    use mod.widgets.*

    startup() do #(App::script_component(vm)){
        ui: XrRoot{
            window.inner_size: vec2(760, 480)
            pass.clear_color: #x10171f
            camera.fov_y: 36.0
            camera.desktop_target: vec3(0.0, -0.05, -0.72)
            camera.distance: 1.65
            env.gravity: 0.0
            env.env_cube: false
            env.depth_mesh: false

            comparison_scene := XrNode{
                pos: vec3(0.0, -0.04, -0.78)

                on_render: ||{
                    Cube{
                        body: mod.widgets.XrBodyKind.Fixed
                        size: vec3(0.52, 0.32, 0.018)
                        corner_radius: 0.012
                        roughness: 0.72
                        metallic: 0.0
                        color: #x214966
                        pos: vec3(-0.31, 0.0, 0.0)
                    }

                    Cube{
                        body: mod.widgets.XrBodyKind.Fixed
                        size: vec3(0.52, 0.32, 0.018)
                        corner_radius: 0.012
                        roughness: 0.72
                        metallic: 0.0
                        color: #x5b315b
                        pos: vec3(0.31, 0.0, 0.0)
                    }

                    Cube{
                        body: mod.widgets.XrBodyKind.Fixed
                        size: vec3(0.018, 0.36, 0.026)
                        corner_radius: 0.006
                        roughness: 0.32
                        metallic: 0.02
                        color: #xe8edf2
                        pos: vec3(0.0, 0.0, 0.012)
                    }

                    Cube{
                        body: mod.widgets.XrBodyKind.Fixed
                        size: vec3(0.68, 0.018, 0.024)
                        corner_radius: 0.006
                        roughness: 0.32
                        metallic: 0.02
                        color: #xc8d2dc
                        pos: vec3(0.0, 0.18, 0.012)
                    }

                    Cube{
                        body: mod.widgets.XrBodyKind.Fixed
                        size: vec3(0.68, 0.018, 0.024)
                        corner_radius: 0.006
                        roughness: 0.32
                        metallic: 0.02
                        color: #xc8d2dc
                        pos: vec3(0.0, -0.18, 0.012)
                    }
                }
            }
        }
    }
}

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    hardware_buffer_import_timer: Timer,
    #[rust]
    hardware_buffer_import_wait_count: usize,
    #[rust]
    hardware_buffer_import_choice: Option<MakepadCameraChoice>,
    #[rust]
    hardware_buffer_import_selection_logged: bool,
    #[rust]
    hardware_buffer_import_started: bool,
    #[rust]
    hardware_buffer_import_finished: bool,
    #[rust]
    hardware_buffer_import_texture: Option<Texture>,
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

    fn arm_hardware_buffer_import_timer(&mut self, cx: &mut Cx, delay_seconds: f64) {
        if self.hardware_buffer_import_finished {
            return;
        }
        self.hardware_buffer_import_timer = cx.start_timeout(delay_seconds);
    }

    fn handle_hardware_buffer_import_event(&mut self, cx: &mut Cx, event: &Event) {
        match event {
            Event::Startup => {
                cx.request_permission(Permission::Camera);
                cx.request_permission(Permission::HeadsetCamera);
                self.arm_hardware_buffer_import_timer(cx, HARDWARE_BUFFER_IMPORT_DELAY_SECONDS);
            }
            Event::VideoInputs(inputs) => {
                self.hardware_buffer_import_choice = Self::pick_makepad_camera_choice(inputs);
                if !self.hardware_buffer_import_selection_logged {
                    self.hardware_buffer_import_selection_logged = true;
                    self.emit_makepad_camera_selection_marker(inputs);
                }
            }
            Event::VideoPlaybackPrepared(prepared) => {
                if prepared.video_id == hardware_buffer_import_video_id() {
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=prepared status=ok width={} height={} importPath=makepad-android-video-external-vulkan textureMode=hardware-buffer-external",
                        prepared.video_width,
                        prepared.video_height,
                    ));
                }
            }
            Event::VideoTextureUpdated(updated) => {
                if updated.video_id == hardware_buffer_import_video_id()
                    && !self.hardware_buffer_import_finished
                {
                    self.hardware_buffer_import_finished = true;
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=texture-updated status=ok makepadVulkanImport=true yuvEnabled={} yuvBiplanar={} rotationSteps={:.0} pairedLeftRightGpuBuffers=false alignedProjection=false",
                        updated.yuv.enabled,
                        updated.yuv.biplanar,
                        updated.yuv.rotation_steps,
                    ));
                }
            }
            Event::VideoDecodingError(error) => {
                if error.video_id == hardware_buffer_import_video_id()
                    && !self.hardware_buffer_import_finished
                {
                    self.hardware_buffer_import_finished = true;
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=complete status=error errorKind=makepad_video_import_failed message={}",
                        marker_token(&error.error),
                    ));
                }
            }
            _ => {}
        }

        if !self.hardware_buffer_import_timer.is_empty()
            && self.hardware_buffer_import_timer.is_event(event).is_some()
        {
            self.hardware_buffer_import_timer = Timer::empty();
            self.try_start_hardware_buffer_import(cx);
        }
    }

    fn try_start_hardware_buffer_import(&mut self, cx: &mut Cx) {
        if self.hardware_buffer_import_started || self.hardware_buffer_import_finished {
            return;
        }

        let Some(choice) = self.hardware_buffer_import_choice.clone() else {
            self.hardware_buffer_import_wait_count =
                self.hardware_buffer_import_wait_count.saturating_add(1);
            if self.hardware_buffer_import_wait_count > HARDWARE_BUFFER_IMPORT_MAX_WAITS {
                self.hardware_buffer_import_finished = true;
                Self::emit_hardware_buffer_import_marker(
                    "phase=start status=error errorKind=no_makepad_camera_source",
                );
            } else {
                Self::emit_hardware_buffer_import_marker(&format!(
                    "phase=start status=waiting waitCount={} reason=no_makepad_camera_source_yet",
                    self.hardware_buffer_import_wait_count,
                ));
                self.arm_hardware_buffer_import_timer(cx, HARDWARE_BUFFER_IMPORT_RETRY_SECONDS);
            }
            return;
        };

        let texture = Texture::new_with_format(cx, TextureFormat::VideoExternal);
        let texture_id = texture.texture_id();
        self.hardware_buffer_import_texture = Some(texture);
        self.hardware_buffer_import_started = true;

        Self::emit_hardware_buffer_import_marker(&format!(
            "phase=start status=started sourceClass={} width={} height={} pixelFormat={} importPath=makepad-android-video-external-vulkan textureFormat=VideoExternal delayedAfterAcquisitionSeconds={:.0}",
            choice.source_class,
            choice.width,
            choice.height,
            pixel_format_label(choice.pixel_format),
            HARDWARE_BUFFER_IMPORT_DELAY_SECONDS,
        ));

        cx.prepare_headset_camera_playback(
            hardware_buffer_import_video_id(),
            VideoSource::Camera(choice.input_id, choice.format_id),
            CameraPreviewMode::Texture,
            0,
            texture_id,
            true,
            false,
        );
    }

    fn emit_makepad_camera_selection_marker(&self, inputs: &VideoInputsEvent) {
        let source_count = inputs.descs.len();
        let format_count: usize = inputs.descs.iter().map(|desc| desc.formats.len()).sum();
        match &self.hardware_buffer_import_choice {
            Some(choice) => Self::emit_hardware_buffer_import_marker(&format!(
                "phase=enumerated status=ok makepadSourceCount={} makepadFormatCount={} selected=true sourceClass={} width={} height={} pixelFormat={} importPlan=single-makepad-video-hardware-buffer",
                source_count,
                format_count,
                choice.source_class,
                choice.width,
                choice.height,
                pixel_format_label(choice.pixel_format),
            )),
            None => Self::emit_hardware_buffer_import_marker(&format!(
                "phase=enumerated status=error makepadSourceCount={} makepadFormatCount={} selected=false errorKind=no_yuv420_makepad_camera_format",
                source_count,
                format_count,
            )),
        }
    }

    fn pick_makepad_camera_choice(inputs: &VideoInputsEvent) -> Option<MakepadCameraChoice> {
        inputs
            .descs
            .iter()
            .flat_map(|desc| {
                desc.formats.iter().filter_map(move |format| {
                    (format.pixel_format == VideoPixelFormat::YUV420).then(|| {
                        MakepadCameraChoice::new(
                            desc.input_id,
                            *format,
                            camera_source_class(&desc.name),
                        )
                    })
                })
            })
            .max_by_key(MakepadCameraChoice::score)
    }

    #[cfg(target_os = "android")]
    fn start_camera_probe_once() {
        android_camera_probe::start_camera_probe_once();
    }

    #[cfg(not(target_os = "android"))]
    fn start_camera_probe_once() {}
}

#[derive(Clone)]
struct MakepadCameraChoice {
    input_id: makepad_widgets::makepad_platform::video::VideoInputId,
    format_id: makepad_widgets::makepad_platform::video::VideoFormatId,
    source_class: &'static str,
    width: usize,
    height: usize,
    pixel_format: VideoPixelFormat,
}

impl MakepadCameraChoice {
    fn new(
        input_id: makepad_widgets::makepad_platform::video::VideoInputId,
        format: VideoFormat,
        source_class: &'static str,
    ) -> Self {
        Self {
            input_id,
            format_id: format.format_id,
            source_class,
            width: format.width,
            height: format.height,
            pixel_format: format.pixel_format,
        }
    }

    fn score(&self) -> (i32, i64, i64) {
        let source_rank = match self.source_class {
            "back" => 3,
            "external" => 2,
            "front" => 1,
            _ => 0,
        };
        let target_penalty = self.width.abs_diff(1280) + self.height.abs_diff(1280);
        let square_penalty = self.width.abs_diff(self.height);
        let area = (self.width as i64) * (self.height as i64);
        (
            source_rank,
            area - (target_penalty as i64) * 2048 - (square_penalty as i64) * 4096,
            area,
        )
    }
}

fn hardware_buffer_import_video_id() -> LiveId {
    live_id!(rusty_xr_makepad_hardware_buffer_import_probe)
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
    fn handle_startup(&mut self, _cx: &mut Cx) {
        Self::emit_startup_markers_once("startup");
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
        self.handle_hardware_buffer_import_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
