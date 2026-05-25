use super::openxr_gles_config::{
    activity_string_extra, OesCameraProjectionMode, OesProjectionBorderPolicy,
};
use super::projection_geometry::{
    projection_plan_from_metadata, OesEyeProjection, OesProjectionPlan,
};
use super::source_metadata::{
    stream_projection_metadata_log_message, OesInputSourceKind, OesProjectionMetadata,
};
use super::{
    glBindTexture, glGetError, log_error, log_info, EglContext, GL_NO_ERROR,
    GL_TEXTURE_EXTERNAL_OES, VIEW_COUNT,
};
use jni::{
    objects::{GlobalRef, JClass, JObject, JString, JValue},
    sys::{jint, jlong, jobject},
    JNIEnv, JavaVM,
};
use rusty_xr_quest_diagnostics::{
    FrameRateSummary, SurfaceTextureOesEyeStatus, SurfaceTextureOesIngestState,
    SurfaceTextureOesIngestStatus,
};
use std::{
    os::raw::c_int,
    sync::{
        atomic::{AtomicI64, AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::Instant,
};

const BROKER_H264_DEFAULT_HOST: &str = "127.0.0.1";
const BROKER_H264_LEFT_STREAM_PORT: i32 = 8879;
const BROKER_H264_RIGHT_STREAM_PORT: i32 = 8880;
const BROKER_H264_MAX_PACKETS: i32 = 0;
const BROKER_H264_CONNECT_TIMEOUT_MS: i32 = 5000;
const BROKER_H264_DECODE_TIMEOUT_MS: i32 = 0;
const DEFAULT_OES_SURFACE_WIDTH: i32 = 1280;
const DEFAULT_OES_SURFACE_HEIGHT: i32 = 1280;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_LINEAR: u32 = 0x2601;
const GL_CLAMP_TO_EDGE: u32 = 0x812F;

#[link(name = "GLESv3")]
unsafe extern "C" {
    fn glGenTextures(n: c_int, textures: *mut u32);
    fn glDeleteTextures(n: c_int, textures: *const u32);
    fn glTexParameteri(target: u32, pname: u32, param: c_int);
}

static OES_DECODE_CALLBACKS: OnceLock<OesDecodeCallbackState> = OnceLock::new();

struct OesDecodeCallbackState {
    frame_available_counts: [AtomicU64; VIEW_COUNT],
    latest_sequences: [AtomicU64; VIEW_COUNT],
    latest_queued_pts_us: [AtomicI64; VIEW_COUNT],
    report_sequence: AtomicU64,
    latest_report: Mutex<Option<String>>,
    projection_metadata_reports: Mutex<[Option<String>; VIEW_COUNT]>,
}

impl OesDecodeCallbackState {
    fn new() -> Self {
        Self {
            frame_available_counts: [AtomicU64::new(0), AtomicU64::new(0)],
            latest_sequences: [AtomicU64::new(0), AtomicU64::new(0)],
            latest_queued_pts_us: [AtomicI64::new(-1), AtomicI64::new(-1)],
            report_sequence: AtomicU64::new(0),
            latest_report: Mutex::new(None),
            projection_metadata_reports: Mutex::new([None, None]),
        }
    }

    fn reset(&self) {
        for index in 0..VIEW_COUNT {
            self.frame_available_counts[index].store(0, Ordering::Relaxed);
            self.latest_sequences[index].store(0, Ordering::Relaxed);
            self.latest_queued_pts_us[index].store(-1, Ordering::Relaxed);
        }
        self.report_sequence.store(0, Ordering::Relaxed);
        if let Ok(mut latest_report) = self.latest_report.lock() {
            *latest_report = None;
        }
        if let Ok(mut reports) = self.projection_metadata_reports.lock() {
            *reports = [None, None];
        }
    }

    fn mark_frame_available(&self, view_index: usize, sequence: u64, queued_pts_us: i64) {
        if view_index >= VIEW_COUNT {
            return;
        }
        self.latest_sequences[view_index].store(sequence, Ordering::Relaxed);
        self.latest_queued_pts_us[view_index].store(queued_pts_us, Ordering::Relaxed);
        self.frame_available_counts[view_index].fetch_add(1, Ordering::Relaxed);
    }

    fn frame_snapshot(&self, view_index: usize) -> (u64, u64, i64) {
        (
            self.frame_available_counts[view_index].load(Ordering::Relaxed),
            self.latest_sequences[view_index].load(Ordering::Relaxed),
            self.latest_queued_pts_us[view_index].load(Ordering::Relaxed),
        )
    }

    fn record_report(&self, report: String) {
        if let Some(view_index) = projection_metadata_report_view_index(&report) {
            if let Ok(mut reports) = self.projection_metadata_reports.lock() {
                reports[view_index] = Some(report.clone());
            }
        }
        if let Ok(mut latest_report) = self.latest_report.lock() {
            *latest_report = Some(report);
            self.report_sequence.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn latest_report_after(&self, last_seen_sequence: &mut u64) -> Option<String> {
        let sequence = self.report_sequence.load(Ordering::Relaxed);
        if sequence == *last_seen_sequence {
            return None;
        }
        *last_seen_sequence = sequence;
        self.latest_report
            .lock()
            .ok()
            .and_then(|report| report.clone())
    }

    fn projection_metadata_report_snapshot(&self) -> [Option<String>; VIEW_COUNT] {
        self.projection_metadata_reports
            .lock()
            .map(|reports| [reports[0].clone(), reports[1].clone()])
            .unwrap_or([None, None])
    }
}

fn projection_metadata_report_view_index(report: &str) -> Option<usize> {
    let report = serde_json::from_str::<serde_json::Value>(report).ok()?;
    report.get("header_projection_metadata")?;
    report_view_index(&report)
}

fn report_view_index(report: &serde_json::Value) -> Option<usize> {
    report
        .get("view_index")
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value < VIEW_COUNT)
}

fn oes_decode_callbacks() -> &'static OesDecodeCallbackState {
    OES_DECODE_CALLBACKS.get_or_init(OesDecodeCallbackState::new)
}

pub(super) fn probe_surface_texture_oes(
    app: &android_activity::AndroidApp,
    egl: &EglContext,
) -> Option<SurfaceTextureOesProbe> {
    let mut failure = SurfaceTextureOesIngestStatus::new();
    failure.state = SurfaceTextureOesIngestState::Failed;

    let probe = match SurfaceTextureOesProbe::create(app, egl) {
        Ok(probe) => probe,
        Err(error) => {
            failure
                .issue_codes
                .push(String::from("surface_texture_oes_probe_failed"));
            failure.notes.push(error);
            log_surface_texture_oes_status(&failure);
            return None;
        }
    };

    log_surface_texture_oes_status(&probe.status);
    Some(probe)
}

pub(super) struct SurfaceTextureOesProbe {
    status: SurfaceTextureOesIngestStatus,
    surface_textures: Vec<GlobalRef>,
    output_surfaces: Vec<GlobalRef>,
    decode_probe: Option<GlobalRef>,
    textures: Vec<u32>,
    java_vm: JavaVM,
    consumed_frame_available_counts: [u64; VIEW_COUNT],
    latest_transform_matrices: [[f32; 16]; VIEW_COUNT],
    update_rate_start: Instant,
    last_report_sequence: u64,
    projection_metadata: [Option<OesProjectionMetadata>; VIEW_COUNT],
}

pub(super) struct OesEyeTextureSample {
    pub(super) texture: u32,
    pub(super) source_sequence: u64,
    pub(super) queued_pts_us: Option<i64>,
    pub(super) surface_timestamp_ns: Option<i64>,
    pub(super) transform_hash: Option<String>,
    pub(super) transform_matrix: [f32; 16],
    pub(super) update_tex_image_count: u64,
}

impl OesEyeProjection {
    pub(super) fn source_transform_for_sample(
        &self,
        surface_texture_transform: [f32; 16],
    ) -> [f32; 16] {
        if self.use_surface_texture_transform {
            surface_texture_transform
        } else {
            identity_texture_transform()
        }
    }
}

impl SurfaceTextureOesProbe {
    fn create(app: &android_activity::AndroidApp, egl: &EglContext) -> Result<Self, String> {
        egl.make_current()?;
        oes_decode_callbacks().reset();
        let java_vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }
            .map_err(|error| format!("wrap Android JavaVM: {error}"))?;
        let source_kind = {
            let mut env = java_vm
                .attach_current_thread()
                .map_err(|error| format!("attach JNI thread for OES source selection: {error}"))?;
            let activity = unsafe {
                JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
            };
            OesInputSourceKind::from_label(
                activity_string_extra(&mut env, &activity, "rustyxr.videoSource").as_deref(),
            )
        };
        let mut textures = Vec::with_capacity(VIEW_COUNT);
        let mut status = SurfaceTextureOesIngestStatus::new();
        status.codec_mime = source_kind.codec_mime().map(String::from);
        status.notes.push(String::from(
            "Created SurfaceTexture-backed output surfaces; updateTexImage runs on the native GL render thread.",
        ));

        let (surface_textures, output_surfaces) = {
            let mut env = java_vm
                .attach_current_thread()
                .map_err(|error| format!("attach JNI thread for SurfaceTexture probe: {error}"))?;
            let mut surface_textures = Vec::with_capacity(VIEW_COUNT);
            let mut output_surfaces = Vec::with_capacity(VIEW_COUNT);

            for view_index in 0..VIEW_COUNT {
                let texture = create_external_oes_texture()?;
                let texture_name = i32::try_from(texture).map_err(|_| {
                    format!("external OES texture id {texture} does not fit JNI int")
                })?;
                let surface_texture = env
                    .new_object(
                        "android/graphics/SurfaceTexture",
                        "(I)V",
                        &[JValue::Int(texture_name)],
                    )
                    .map_err(|error| {
                        delete_gl_texture(texture);
                        format!("create Android SurfaceTexture for eye {view_index}: {error}")
                    })?;
                env.call_method(
                    &surface_texture,
                    "setDefaultBufferSize",
                    "(II)V",
                    &[
                        JValue::Int(DEFAULT_OES_SURFACE_WIDTH),
                        JValue::Int(DEFAULT_OES_SURFACE_HEIGHT),
                    ],
                )
                .map_err(|error| {
                    delete_gl_texture(texture);
                    format!("set SurfaceTexture default buffer size for eye {view_index}: {error}")
                })?;
                let output_surface = env
                    .new_object(
                        "android/view/Surface",
                        "(Landroid/graphics/SurfaceTexture;)V",
                        &[JValue::Object(&surface_texture)],
                    )
                    .map_err(|error| {
                        delete_gl_texture(texture);
                        format!("create Android Surface for eye {view_index}: {error}")
                    })?;
                let surface_texture_ref =
                    env.new_global_ref(&surface_texture).map_err(|error| {
                        delete_gl_texture(texture);
                        format!(
                            "promote SurfaceTexture global reference for eye {view_index}: {error}"
                        )
                    })?;
                let output_surface_ref = env.new_global_ref(&output_surface).map_err(|error| {
                    delete_gl_texture(texture);
                    format!("promote Surface global reference for eye {view_index}: {error}")
                })?;

                textures.push(texture);
                surface_textures.push(surface_texture_ref);
                output_surfaces.push(output_surface_ref);
                let eye_name = if view_index == 0 { "left" } else { "right" };
                let mut eye = SurfaceTextureOesEyeStatus::for_stream(
                    view_index as u32,
                    source_kind.stream_label(view_index),
                    eye_name,
                )
                .mark_surface_ready();
                eye.source_width = Some(DEFAULT_OES_SURFACE_WIDTH as u32);
                eye.source_height = Some(DEFAULT_OES_SURFACE_HEIGHT as u32);
                status.eyes.push(eye);
            }

            (surface_textures, output_surfaces)
        };

        let decode_probe = {
            let mut env = java_vm
                .attach_current_thread()
                .map_err(|error| format!("attach JNI thread for OES source start: {error}"))?;
            match source_kind {
                OesInputSourceKind::None => {
                    status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                    status.notes.push(String::from(
                        "No OES video source requested; rendering static GLES grids.",
                    ));
                    None
                }
                OesInputSourceKind::BrokerH264 => {
                    match start_broker_h264_oes_decode_probe(
                        &mut env,
                        app,
                        &output_surfaces,
                        &surface_textures,
                    ) {
                        Ok(probe) => {
                            for eye in &mut status.eyes {
                                eye.decoder_configured = true;
                                eye.decoder_started = true;
                            }
                            status.state = SurfaceTextureOesIngestState::DecoderStarted;
                            status.notes.push(format!(
                                "Started broker-compatible RXYRVID1 H.264 decode threads host={} leftPort={} rightPort={}.",
                                BROKER_H264_DEFAULT_HOST,
                                BROKER_H264_LEFT_STREAM_PORT,
                                BROKER_H264_RIGHT_STREAM_PORT
                            ));
                            Some(probe)
                        }
                        Err(error) => {
                            status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                            status
                                .issue_codes
                                .push(String::from("broker_h264_oes_decode_start_failed"));
                            status.notes.push(error);
                            None
                        }
                    }
                }
                OesInputSourceKind::DirectCamera2 => {
                    match start_direct_camera2_oes_probe(
                        &mut env,
                        app,
                        &output_surfaces,
                        &surface_textures,
                    ) {
                        Ok(probe) => {
                            status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                            status.notes.push(String::from(
                                "Started direct Camera2 capture into SurfaceTexture/OES output surfaces.",
                            ));
                            Some(probe)
                        }
                        Err(error) => {
                            status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                            status
                                .issue_codes
                                .push(String::from("direct_camera2_oes_start_failed"));
                            status.notes.push(error);
                            None
                        }
                    }
                }
            }
        };

        log_info(format!(
            "Rusty XR SurfaceTexture OES output surfaces ready eyes={} size={}x{} sourceStarted={}",
            status.eyes.len(),
            DEFAULT_OES_SURFACE_WIDTH,
            DEFAULT_OES_SURFACE_HEIGHT,
            decode_probe.is_some()
        ));

        Ok(Self {
            status,
            surface_textures,
            output_surfaces,
            decode_probe,
            textures,
            java_vm,
            consumed_frame_available_counts: [0; VIEW_COUNT],
            latest_transform_matrices: [identity_texture_transform(); VIEW_COUNT],
            update_rate_start: Instant::now(),
            last_report_sequence: 0,
            projection_metadata: std::array::from_fn(|_| None),
        })
    }

    pub(super) fn update_textures(&mut self, egl: &EglContext, frame_count: u64) {
        self.apply_latest_decode_report();
        let callbacks = oes_decode_callbacks();
        let mut updated_any = false;
        for view_index in 0..VIEW_COUNT {
            let (available_count, latest_sequence, latest_pts_us) =
                callbacks.frame_snapshot(view_index);
            if available_count <= self.consumed_frame_available_counts[view_index] {
                if let Some(eye) = self.status.eyes.get_mut(view_index) {
                    eye.frame_available_count = available_count;
                }
                continue;
            }

            let skipped = available_count
                .saturating_sub(self.consumed_frame_available_counts[view_index])
                .saturating_sub(1);
            match self.update_surface_texture(egl, view_index) {
                Ok((timestamp_ns, transform_hash, transform_matrix)) => {
                    self.latest_transform_matrices[view_index] = transform_matrix;
                    if let Some(eye) = self.status.eyes.get_mut(view_index) {
                        eye.record_update(
                            frame_count,
                            latest_sequence,
                            latest_pts_us,
                            timestamp_ns,
                            transform_hash.as_str(),
                        );
                        eye.frame_available_count = available_count;
                        eye.skipped_update_count = eye.skipped_update_count.saturating_add(skipped);
                        if eye.transform_matrix_sample_count == 1
                            || eye.transform_matrix_sample_count.is_multiple_of(120)
                        {
                            log_surface_texture_transform_matrix(
                                view_index,
                                eye.source_eye.as_deref(),
                                eye.update_tex_image_count,
                                timestamp_ns,
                                &transform_hash,
                                &transform_matrix,
                            );
                        }
                    }
                    self.consumed_frame_available_counts[view_index] = available_count;
                    updated_any = true;
                }
                Err(error) => {
                    if let Some(eye) = self.status.eyes.get_mut(view_index) {
                        eye.frame_available_count = available_count;
                        eye.decoder_error_count = eye.decoder_error_count.saturating_add(1);
                        eye.latest_decoder_error = Some(error.clone());
                    }
                    self.status
                        .issue_codes
                        .push(String::from("surface_texture_update_failed"));
                    log_error(format!(
                        "Rusty XR SurfaceTexture OES update failed eye={view_index}: {error}"
                    ));
                }
            }
        }

        if updated_any {
            self.status.state = SurfaceTextureOesIngestState::TextureUpdated;
            self.refresh_texture_update_rate();
            log_surface_texture_oes_status(&self.status);
        }
    }

    fn update_surface_texture(
        &self,
        egl: &EglContext,
        view_index: usize,
    ) -> Result<(i64, String, [f32; 16]), String> {
        egl.make_current()?;
        let surface_texture = self
            .surface_textures
            .get(view_index)
            .ok_or_else(|| format!("SurfaceTexture eye index {view_index} is out of range"))?;
        let mut env = self
            .java_vm
            .attach_current_thread()
            .map_err(|error| format!("attach JNI thread for updateTexImage: {error}"))?;
        env.call_method(surface_texture.as_obj(), "updateTexImage", "()V", &[])
            .map_err(|error| format!("updateTexImage: {error}"))?;
        let timestamp_ns = env
            .call_method(surface_texture.as_obj(), "getTimestamp", "()J", &[])
            .and_then(|value| value.j())
            .map_err(|error| format!("get SurfaceTexture timestamp: {error}"))?;
        let transform_matrix =
            sample_surface_texture_transform_matrix(&mut env, surface_texture.as_obj())?;
        let transform_hash = transform_matrix_hash(&transform_matrix);
        Ok((timestamp_ns, transform_hash, transform_matrix))
    }

    pub(super) fn updated_eye_texture(&self, view_index: usize) -> Option<OesEyeTextureSample> {
        let eye = self.status.eyes.get(view_index)?;
        if eye.update_tex_image_count == 0 || eye.decoder_error_count > 0 {
            return None;
        }
        let texture = *self.textures.get(view_index)?;
        Some(OesEyeTextureSample {
            texture,
            source_sequence: eye.latest_stream_sequence.unwrap_or_default(),
            queued_pts_us: eye.latest_queued_pts_us,
            surface_timestamp_ns: eye.latest_surface_texture_timestamp_ns,
            transform_hash: eye.latest_transform_matrix_hash.clone(),
            transform_matrix: self
                .latest_transform_matrices
                .get(view_index)
                .copied()
                .unwrap_or_else(identity_texture_transform),
            update_tex_image_count: eye.update_tex_image_count,
        })
    }

    pub(super) fn frame_age_at_submit_ms(&self, queued_pts_us: i64) -> Option<f32> {
        let now_ns = android_elapsed_realtime_nanos(&self.java_vm)?;
        let source_ns = queued_pts_us.checked_mul(1_000)?;
        let age_ns = now_ns.checked_sub(source_ns)?;
        if !(0..=10_000_000_000).contains(&age_ns) {
            return None;
        }
        Some(age_ns as f32 / 1_000_000.0)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn projection_plan_from_xr_views(
        &self,
        views: &[openxr::View],
        camera_projection_mode: OesCameraProjectionMode,
        projection_area_eye_offset_uv: [[f32; 2]; 2],
        projection_area_scale: [f32; 2],
        projection_area_radius: [f32; 2],
        projection_area_opacity: f32,
        projection_border_policy: OesProjectionBorderPolicy,
        projection_border_opacity: f32,
        projection_depth_meters: f32,
        projection_preview_fov_y_degrees: f32,
        projection_preview_offset_y_meters: f32,
        projection_raw_overscan: f32,
    ) -> Option<OesProjectionPlan> {
        let left = self.projection_metadata[0].as_ref()?;
        let right = self.projection_metadata[1].as_ref()?;
        projection_plan_from_metadata(
            left,
            right,
            views,
            camera_projection_mode,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
        )
    }

    fn refresh_texture_update_rate(&mut self) {
        let elapsed = self.update_rate_start.elapsed().as_secs_f32();
        if elapsed <= 0.0 {
            return;
        }
        let sample_count = self
            .status
            .eyes
            .iter()
            .map(|eye| eye.update_tex_image_count)
            .sum::<u64>();
        let average_fps = sample_count as f32 / elapsed;
        self.status.texture_update_rate = Some(FrameRateSummary {
            sample_count,
            average_fps,
            min_fps: average_fps,
            max_fps: average_fps,
        });
    }

    fn apply_latest_decode_report(&mut self) {
        let Some(report_json) =
            oes_decode_callbacks().latest_report_after(&mut self.last_report_sequence)
        else {
            return;
        };
        let Ok(report) = serde_json::from_str::<serde_json::Value>(&report_json) else {
            return;
        };
        if let Some(decoder_name) = report.get("decoder_name").and_then(|value| value.as_str()) {
            if !decoder_name.is_empty() {
                self.status.codec_name = Some(decoder_name.to_string());
            }
        }
        let event_name = report
            .get("event")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if event_name == "frame_available"
            && self.status.state == SurfaceTextureOesIngestState::DecoderStarted
        {
            self.status.state = SurfaceTextureOesIngestState::FrameAvailable;
        }
        let Some(view_index) = report_view_index(&report) else {
            return;
        };
        self.apply_projection_metadata_report(view_index, &report);
        self.apply_cached_projection_metadata_reports();
        let Some(eye) = self.status.eyes.get_mut(view_index) else {
            return;
        };
        if let Some(width) = report
            .get("width")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
        {
            eye.source_width = Some(width);
        }
        if let Some(height) = report
            .get("height")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
        {
            eye.source_height = Some(height);
        }
        if let Some(error) = report.get("error").and_then(|value| value.as_str()) {
            eye.decoder_error_count = eye.decoder_error_count.saturating_add(1);
            eye.latest_decoder_error = Some(error.to_string());
        } else if let Some(error) = report.get("last_error").and_then(|value| value.as_str()) {
            eye.latest_decoder_error = Some(error.to_string());
        }
    }

    fn apply_cached_projection_metadata_reports(&mut self) {
        for report_json in oes_decode_callbacks()
            .projection_metadata_report_snapshot()
            .into_iter()
            .flatten()
        {
            let Ok(report) = serde_json::from_str::<serde_json::Value>(&report_json) else {
                continue;
            };
            let Some(view_index) = report_view_index(&report) else {
                continue;
            };
            self.apply_projection_metadata_report(view_index, &report);
        }
    }

    fn apply_projection_metadata_report(&mut self, view_index: usize, report: &serde_json::Value) {
        if self
            .projection_metadata
            .get(view_index)
            .and_then(|value| value.as_ref())
            .is_some()
        {
            return;
        }
        let Some(metadata) = report
            .get("header_projection_metadata")
            .and_then(|metadata| OesProjectionMetadata::parse(metadata).ok())
        else {
            return;
        };
        log_info(stream_projection_metadata_log_message(
            view_index, &metadata,
        ));
        if let Some(slot) = self.projection_metadata.get_mut(view_index) {
            *slot = Some(metadata);
        }
    }
}

fn sample_surface_texture_transform_matrix(
    env: &mut JNIEnv<'_>,
    surface_texture: &JObject<'_>,
) -> Result<[f32; 16], String> {
    let transform_array = env
        .new_float_array(16)
        .map_err(|error| format!("allocate SurfaceTexture transform matrix array: {error}"))?;
    env.call_method(
        surface_texture,
        "getTransformMatrix",
        "([F)V",
        &[JValue::Object(&transform_array)],
    )
    .map_err(|error| format!("get SurfaceTexture transform matrix: {error}"))?;
    let mut transform = [0.0_f32; 16];
    env.get_float_array_region(&transform_array, 0, &mut transform)
        .map_err(|error| format!("read SurfaceTexture transform matrix: {error}"))?;
    Ok(transform)
}

fn transform_matrix_hash(transform: &[f32; 16]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in transform {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("m44:fnv1a64:{hash:016x}")
}

fn android_elapsed_realtime_nanos(java_vm: &JavaVM) -> Option<i64> {
    let mut env = java_vm.attach_current_thread().ok()?;
    env.call_static_method("android/os/SystemClock", "elapsedRealtimeNanos", "()J", &[])
        .ok()?
        .j()
        .ok()
}

fn log_surface_texture_transform_matrix(
    view_index: usize,
    source_eye: Option<&str>,
    update_tex_image_count: u64,
    timestamp_ns: i64,
    transform_hash: &str,
    transform_matrix: &[f32; 16],
) {
    let payload = serde_json::json!({
        "schema": "rusty.xr.quest.surface_texture_oes_transform_matrix.v1",
        "view_index": view_index,
        "source_eye": source_eye,
        "update_tex_image_count": update_tex_image_count,
        "surface_texture_timestamp_ns": timestamp_ns,
        "transform_matrix_hash": transform_hash,
        "transform_matrix": transform_matrix,
    });
    log_info(format!(
        "Rusty XR SurfaceTexture OES transform matrix {payload}"
    ));
}

pub(super) fn log_oes_submit_diagnostic(
    view_index: usize,
    frame_count: u64,
    source: &OesEyeTextureSample,
    frame_age_at_submit_ms: Option<f32>,
    render_path: &str,
) {
    let payload = serde_json::json!({
        "schema": "rusty.xr.quest.openxr_gles_oes_submit.v1",
        "view_index": view_index,
        "frame_count": frame_count,
        "source_sequence": source.source_sequence,
        "queued_pts_us": source.queued_pts_us,
        "surface_texture_timestamp_ns": source.surface_timestamp_ns,
        "transform_matrix_hash": source.transform_hash,
        "update_tex_image_count": source.update_tex_image_count,
        "frame_age_at_submit_ms": frame_age_at_submit_ms,
        "render_path": render_path,
    });
    log_info(format!(
        "Rusty XR OpenXR GLES OES submit diagnostic {payload}"
    ));
}

fn start_broker_h264_oes_decode_probe(
    env: &mut JNIEnv<'_>,
    app: &android_activity::AndroidApp,
    output_surfaces: &[GlobalRef],
    surface_textures: &[GlobalRef],
) -> Result<GlobalRef, String> {
    if output_surfaces.len() < VIEW_COUNT || surface_textures.len() < VIEW_COUNT {
        return Err(format!(
            "broker H.264 OES decode requires {VIEW_COUNT} output surfaces and SurfaceTextures"
        ));
    }
    let host = env
        .new_string(BROKER_H264_DEFAULT_HOST)
        .map_err(|error| jni_error(env, "create broker H.264 host string", error))?;
    let host_object = JObject::from(host);
    let class_name = env
        .new_string("com.example.rustyxr.opengles.BrokerH264OesDecodeProbe")
        .map_err(|error| jni_error(env, "create broker H.264 helper class string", error))?;
    let class_name_object = JObject::from(class_name);
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let class_loader = env
        .call_method(
            &activity,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "read Activity class loader", error))?;
    let helper_class_object = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name_object)],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "load broker H.264 OES helper class", error))?;
    let helper_class = JClass::from(helper_class_object);
    let probe = env
        .call_static_method(
            &helper_class,
            "start",
            "(Landroid/app/Activity;Ljava/lang/String;IILandroid/view/Surface;Landroid/view/Surface;Landroid/graphics/SurfaceTexture;Landroid/graphics/SurfaceTexture;III)Lcom/example/rustyxr/opengles/BrokerH264OesDecodeProbe;",
            &[
                JValue::Object(&activity),
                JValue::Object(&host_object),
                JValue::Int(BROKER_H264_LEFT_STREAM_PORT),
                JValue::Int(BROKER_H264_RIGHT_STREAM_PORT),
                JValue::Object(output_surfaces[0].as_obj()),
                JValue::Object(output_surfaces[1].as_obj()),
                JValue::Object(surface_textures[0].as_obj()),
                JValue::Object(surface_textures[1].as_obj()),
                JValue::Int(BROKER_H264_MAX_PACKETS),
                JValue::Int(BROKER_H264_CONNECT_TIMEOUT_MS),
                JValue::Int(BROKER_H264_DECODE_TIMEOUT_MS),
            ],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "start Java broker H.264 OES decode probe", error))?;
    if probe.is_null() {
        return Err("Java broker H.264 OES decode probe returned null".to_string());
    }
    env.new_global_ref(&probe).map_err(|error| {
        jni_error(
            env,
            "promote broker H.264 OES decode probe reference",
            error,
        )
    })
}

fn start_direct_camera2_oes_probe(
    env: &mut JNIEnv<'_>,
    app: &android_activity::AndroidApp,
    output_surfaces: &[GlobalRef],
    surface_textures: &[GlobalRef],
) -> Result<GlobalRef, String> {
    if output_surfaces.len() < VIEW_COUNT || surface_textures.len() < VIEW_COUNT {
        return Err(format!(
            "direct Camera2 OES probe requires {VIEW_COUNT} output surfaces and SurfaceTextures"
        ));
    }
    let class_name = env
        .new_string("com.example.rustyxr.opengles.DirectCamera2OesProbe")
        .map_err(|error| jni_error(env, "create direct Camera2 OES helper class string", error))?;
    let class_name_object = JObject::from(class_name);
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let class_loader = env
        .call_method(
            &activity,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "read Activity class loader", error))?;
    let helper_class_object = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name_object)],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "load direct Camera2 OES helper class", error))?;
    let helper_class = JClass::from(helper_class_object);
    let probe = env
        .call_static_method(
            &helper_class,
            "start",
            "(Landroid/app/Activity;Landroid/view/Surface;Landroid/view/Surface;Landroid/graphics/SurfaceTexture;Landroid/graphics/SurfaceTexture;III)Lcom/example/rustyxr/opengles/DirectCamera2OesProbe;",
            &[
                JValue::Object(&activity),
                JValue::Object(output_surfaces[0].as_obj()),
                JValue::Object(output_surfaces[1].as_obj()),
                JValue::Object(surface_textures[0].as_obj()),
                JValue::Object(surface_textures[1].as_obj()),
                JValue::Int(DEFAULT_OES_SURFACE_WIDTH),
                JValue::Int(DEFAULT_OES_SURFACE_HEIGHT),
                JValue::Int(50),
            ],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "start Java direct Camera2 OES probe", error))?;
    if probe.is_null() {
        return Err("Java direct Camera2 OES probe returned null".to_string());
    }
    env.new_global_ref(&probe)
        .map_err(|error| jni_error(env, "promote direct Camera2 OES probe reference", error))
}

fn jni_error(env: &mut JNIEnv<'_>, context: &str, error: impl std::fmt::Display) -> String {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    format!("{context}: {error}")
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_opengles_BrokerH264OesDecodeProbe_nativeBrokerH264FrameAvailable(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    view_index: jint,
    sequence: jlong,
    queued_pts_us: jlong,
) {
    let Ok(view_index) = usize::try_from(view_index) else {
        return;
    };
    let sequence = u64::try_from(sequence).unwrap_or(0);
    oes_decode_callbacks().mark_frame_available(view_index, sequence, queued_pts_us);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_opengles_BrokerH264OesDecodeProbe_nativeBrokerH264DecodeReport(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    report_json: JString<'_>,
) {
    let report = env
        .get_string(&report_json)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "{\"event\":\"invalidJniString\"}".to_string());
    log_info(format!("Rusty XR broker H.264 OES decode report {report}"));
    oes_decode_callbacks().record_report(report);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_opengles_DirectCamera2OesProbe_nativeDirectCamera2OesFrameAvailable(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    view_index: jint,
    sequence: jlong,
    queued_pts_us: jlong,
) {
    let Ok(view_index) = usize::try_from(view_index) else {
        return;
    };
    let sequence = u64::try_from(sequence).unwrap_or(0);
    oes_decode_callbacks().mark_frame_available(view_index, sequence, queued_pts_us);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_opengles_DirectCamera2OesProbe_nativeDirectCamera2OesReport(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    report_json: JString<'_>,
) {
    let report = env
        .get_string(&report_json)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "{\"event\":\"invalidJniString\"}".to_string());
    log_info(format!("Rusty XR direct Camera2 OES report {report}"));
    oes_decode_callbacks().record_report(report);
}

impl Drop for SurfaceTextureOesProbe {
    fn drop(&mut self) {
        if let Ok(mut env) = self.java_vm.attach_current_thread() {
            if let Some(decode_probe) = &self.decode_probe {
                let _ = env.call_method(decode_probe.as_obj(), "stop", "()V", &[]);
            }
            for surface in &self.output_surfaces {
                let _ = env.call_method(surface.as_obj(), "release", "()V", &[]);
            }
            for surface_texture in &self.surface_textures {
                let _ = env.call_method(surface_texture.as_obj(), "release", "()V", &[]);
            }
        }
        for texture in &self.textures {
            delete_gl_texture(*texture);
        }
    }
}

fn create_external_oes_texture() -> Result<u32, String> {
    unsafe {
        while glGetError() != GL_NO_ERROR {}
        let mut texture = 0;
        glGenTextures(1, &mut texture);
        if texture == 0 {
            return Err("glGenTextures returned texture id 0 for external OES texture".into());
        }
        glBindTexture(GL_TEXTURE_EXTERNAL_OES, texture);
        glTexParameteri(
            GL_TEXTURE_EXTERNAL_OES,
            GL_TEXTURE_MIN_FILTER,
            GL_LINEAR as c_int,
        );
        glTexParameteri(
            GL_TEXTURE_EXTERNAL_OES,
            GL_TEXTURE_MAG_FILTER,
            GL_LINEAR as c_int,
        );
        glTexParameteri(
            GL_TEXTURE_EXTERNAL_OES,
            GL_TEXTURE_WRAP_S,
            GL_CLAMP_TO_EDGE as c_int,
        );
        glTexParameteri(
            GL_TEXTURE_EXTERNAL_OES,
            GL_TEXTURE_WRAP_T,
            GL_CLAMP_TO_EDGE as c_int,
        );
        glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0);
        let error = glGetError();
        if error != GL_NO_ERROR {
            delete_gl_texture(texture);
            return Err(format!(
                "external OES texture setup returned GL error 0x{error:04x}"
            ));
        }
        Ok(texture)
    }
}

fn delete_gl_texture(texture: u32) {
    if texture != 0 {
        unsafe {
            glDeleteTextures(1, &texture);
        }
    }
}

fn identity_texture_transform() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

fn log_surface_texture_oes_status(status: &SurfaceTextureOesIngestStatus) {
    match serde_json::to_string(status) {
        Ok(json) => log_info(format!("Rusty XR SurfaceTexture OES ingest status {json}")),
        Err(error) => log_error(format!(
            "Rusty XR SurfaceTexture OES status serialization failed: {error}"
        )),
    }
}
