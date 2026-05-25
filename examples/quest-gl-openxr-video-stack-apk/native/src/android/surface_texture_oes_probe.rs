use super::openxr_gles_config::activity_string_extra;
use super::source_metadata::{
    stream_projection_metadata_log_message, OesInputSourceKind, OesProjectionMetadata,
};
use super::surface_texture_oes_callbacks::{
    decode_frame_snapshot, latest_decode_report_after, projection_metadata_report_snapshot,
    report_view_index, reset_decode_callbacks,
};
use super::surface_texture_oes_frame_sources::OesEyeTextureSample;
use super::surface_texture_oes_sources::{
    start_broker_h264_oes_decode_probe, start_direct_camera2_oes_probe, BROKER_H264_DEFAULT_HOST,
    BROKER_H264_LEFT_STREAM_PORT, BROKER_H264_RIGHT_STREAM_PORT,
};
use super::{
    glBindTexture, glGetError, log_error, log_info, EglContext, GL_NO_ERROR,
    GL_TEXTURE_EXTERNAL_OES, VIEW_COUNT,
};
use jni::{
    objects::{GlobalRef, JObject, JValue},
    sys::jobject,
    JNIEnv, JavaVM,
};
use rusty_xr_quest_diagnostics::{
    FrameRateSummary, SurfaceTextureOesEyeStatus, SurfaceTextureOesIngestState,
    SurfaceTextureOesIngestStatus,
};
use std::{os::raw::c_int, time::Instant};

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

impl SurfaceTextureOesProbe {
    fn create(app: &android_activity::AndroidApp, egl: &EglContext) -> Result<Self, String> {
        egl.make_current()?;
        reset_decode_callbacks();
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
                        DEFAULT_OES_SURFACE_WIDTH,
                        DEFAULT_OES_SURFACE_HEIGHT,
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
        let mut updated_any = false;
        for view_index in 0..VIEW_COUNT {
            let (available_count, latest_sequence, latest_pts_us) =
                decode_frame_snapshot(view_index);
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
            frame_age_at_submit_ms: None,
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

    pub(super) fn projection_metadata_pair(
        &self,
    ) -> Option<(&OesProjectionMetadata, &OesProjectionMetadata)> {
        Some((
            self.projection_metadata[0].as_ref()?,
            self.projection_metadata[1].as_ref()?,
        ))
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
        let Some(report_json) = latest_decode_report_after(&mut self.last_report_sequence) else {
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
        for report_json in projection_metadata_report_snapshot().into_iter().flatten() {
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
