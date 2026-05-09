pub use makepad_xr::makepad_widgets;

#[cfg(target_os = "android")]
mod acamera_sys;
#[cfg(target_os = "android")]
mod android_camera_probe;

use makepad_widgets::makepad_platform::{
    event::video_playback::{CameraPreviewMode, VideoSource},
    permission::Permission,
    thread::SignalToUI,
    video::{VideoFormat, VideoInputsEvent, VideoPixelFormat},
};
use makepad_widgets::*;
use rusty_xr_runtime_config::{RuntimeConfig, RuntimeConfigSource, RuntimeValue};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

app_main!(App);

static STARTUP_MARKERS_EMITTED: AtomicBool = AtomicBool::new(false);
static PAIRED_IMPORT_SIGNAL_READY: AtomicBool = AtomicBool::new(false);

const DEFAULT_PROFILE: &str = "makepad-stereo-projection-pair-probe";
const DEFAULT_TRANSPORT: &str = "makepad-video-external";
const DEFAULT_CAMERA_TIER: &str = "native-camera2-makepad-stereo-vulkan-import-probe";
const DEFAULT_CAMERA_PROJECTION_MODE: &str = "display-screen-homography";
const DEFAULT_COMPARISON_BASELINE: &str = "custom-apk-camera-stereo-gpu-composite";
const DEFAULT_SYNTHETIC_SCENE: &str = "dual-panel-grid-v1-with-camera-pair";
const DEFAULT_ACQUISITION_PROFILE: &str =
    "bounded-camera2-private-plus-makepad-paired-import-probe";
const DEFAULT_PROJECTION_SCALE: f64 = 0.75;
const DEFAULT_XR_RENDER_SCALE: f64 = 0.75;
const MAKEPAD_BRANCH: &str = "rusty-xr/android-libstd-packaging";
const MAKEPAD_REV: &str = "aebeabf32278";
const PAIRED_IMPORT_DELAY_SECONDS: f64 = 6.0;
const PAIRED_IMPORT_RETRY_SECONDS: f64 = 1.0;
const PAIRED_IMPORT_MAX_WAITS: usize = 10;
const CADENCE_SAMPLE_SECONDS: f64 = 5.0;
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
    paired_import_finished: bool,
    #[rust]
    paired_import_left_texture: Option<Texture>,
    #[rust]
    paired_import_right_texture: Option<Texture>,
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
            "phase=sample status=ok elapsedMs={:.0} intervalMs={:.0} appFrameCount={} appFrameDelta={} appFrameRateHz={:.2} xrUpdateCount={} xrUpdateDelta={} xrUpdateRateHz={:.2} drawEventCount={} drawEventDelta={} drawEventRateHz={:.2} leftTextureUpdateCount={} rightTextureUpdateCount={} pairedTextureUpdateCount={} leftTextureUpdateDelta={} rightTextureUpdateDelta={} pairedTextureUpdateDelta={} leftTextureUpdateRateHz={:.2} rightTextureUpdateRateHz={:.2} pairedTextureUpdateRateHz={:.2} leftLastPositionMs={} rightLastPositionMs={} pairedLeftRightGpuBuffers={} projectionMappingReady={} alignedProjection={} cpuUploadCount=0 renderPath=makepad-xr appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated",
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
            Event::VideoPlaybackPrepared(prepared) => {
                if let Some(side) = StereoEye::from_video_id(prepared.video_id) {
                    match side {
                        StereoEye::Left => self.paired_import_left_prepared = true,
                        StereoEye::Right => self.paired_import_right_prepared = true,
                    }
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=prepared status=ok side={} width={} height={} importPath=makepad-android-video-external-vulkan textureMode=hardware-buffer-external importPlan=paired-makepad-video-hardware-buffer",
                        side.label(),
                        prepared.video_width,
                        prepared.video_height,
                    ));
                    self.emit_paired_projection_progress("prepared");
                }
            }
            Event::VideoTextureUpdated(updated) => {
                if let Some(side) = StereoEye::from_video_id(updated.video_id) {
                    self.record_camera_texture_update(side, updated.current_position_ms);
                    if self.paired_import_finished {
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
                    Self::emit_hardware_buffer_import_marker(&format!(
                        "phase=texture-updated status=ok side={} makepadVulkanImport=true yuvEnabled={} yuvBiplanar={} rotationSteps={:.0} importPlan=paired-makepad-video-hardware-buffer",
                        side.label(),
                        updated.yuv.enabled,
                        updated.yuv.biplanar,
                        updated.yuv.rotation_steps,
                    ));
                    self.complete_paired_import_if_ready();
                }
            }
            Event::VideoDecodingError(error) => {
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
        let left_texture_id = left_texture.texture_id();
        let right_texture_id = right_texture.texture_id();
        self.paired_import_left_texture = Some(left_texture);
        self.paired_import_right_texture = Some(right_texture);
        self.paired_import_started = true;

        Self::emit_hardware_buffer_import_marker(&format!(
            "phase=start status=started importPlan=paired-makepad-video-hardware-buffer leftSourceIndex={} rightSourceIndex={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftFrameRate={} rightFrameRate={} pixelFormat={} importPath=makepad-android-video-external-vulkan textureFormat=VideoExternal delayedAfterAcquisitionSeconds={:.0}",
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

        cx.prepare_headset_camera_playback(
            StereoEye::Left.video_id(),
            VideoSource::Camera(pair.left.input_id, pair.left.format_id),
            CameraPreviewMode::Texture,
            0,
            left_texture_id,
            true,
            false,
        );
        cx.prepare_headset_camera_playback(
            StereoEye::Right.video_id(),
            VideoSource::Camera(pair.right.input_id, pair.right.format_id),
            CameraPreviewMode::Texture,
            0,
            right_texture_id,
            true,
            false,
        );
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

    fn complete_paired_import_if_ready(&mut self) {
        if self.paired_import_finished {
            return;
        }

        if !self.paired_import_left_updated || !self.paired_import_right_updated {
            self.emit_paired_projection_progress("texture-updated");
            return;
        }

        let Some(pair) = &self.paired_import_choice else {
            return;
        };
        self.paired_import_finished = true;
        let aligned_projection = pair.projection_metadata_ready;
        Self::emit_stereo_projection_marker(&format!(
            "phase=complete status=ok pairedLeftRightGpuBuffers=true makepadVulkanImport=true projectionMappingReady={} alignedProjection={} projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} projectionMode={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftRotationSteps={:.0} rightRotationSteps={:.0} projectionScale={:.2} xrRenderScale={:.2} renderPath=makepad-xr projectionShaderPath=makepad-video-external-pair-map cpuUploadCount=0 visualInspection=required visualReleaseAccepted=false fallbackReason={}",
            pair.projection_metadata_ready,
            aligned_projection,
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
            pair,
            aligned_projection,
        );
    }

    fn emit_stereo_comparison_parity_marker(
        phase: &str,
        pair: &MakepadCameraPair,
        aligned_projection: bool,
    ) {
        let config = Self::runtime_config();
        emit_marker_line(&format!(
            "RUSTY_XR_MAKEPAD_STEREO_COMPARISON schema=rusty.xr.makepad-stereo-comparison.v1 phase={} profile={} comparisonBaseline={} cameraTier={} acquisition={} transport={} projectionMode={} syntheticScene={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} sourceEyeMapping={} projectionScale={:.2} xrRenderScale={:.2} pairedLeftRightGpuBuffers=true alignedProjection={} renderPath=makepad-xr projectionShaderPath=makepad-video-external-pair-map makepadForkBranch={} makepadForkCommit={} visualInspection=required visualReleaseAccepted=false",
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
