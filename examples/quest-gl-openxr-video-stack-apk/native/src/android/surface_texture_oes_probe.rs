use super::openxr_gles_config::activity_string_extra;
use super::source_metadata::{OesInputSourceKind, OesProjectionMetadata};
use super::surface_texture_oes_callbacks::{decode_frame_snapshot, reset_decode_callbacks};
use super::surface_texture_oes_frame_sources::OesEyeTextureSample;
use super::surface_texture_oes_gl::identity_texture_transform;
use super::surface_texture_oes_outputs::{
    SurfaceTextureOesOutputResources, DEFAULT_OES_SURFACE_HEIGHT, DEFAULT_OES_SURFACE_WIDTH,
};
use super::surface_texture_oes_reports::SurfaceTextureOesReportState;
use super::surface_texture_oes_sources::{
    start_broker_h264_oes_decode_probe, start_direct_camera2_oes_probe, BROKER_H264_DEFAULT_HOST,
    BROKER_H264_LEFT_STREAM_PORT, BROKER_H264_RIGHT_STREAM_PORT,
};
use super::surface_texture_oes_transform::{
    android_elapsed_realtime_nanos, log_surface_texture_transform_matrix,
    sample_surface_texture_transform_matrix, transform_matrix_hash,
};
use super::{log_error, log_info, EglContext, VIEW_COUNT};
use jni::{objects::JObject, sys::jobject, JavaVM};
use rusty_xr_quest_diagnostics::{
    FrameRateSummary, SurfaceTextureOesIngestState, SurfaceTextureOesIngestStatus,
};
use std::time::Instant;

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
    resources: Option<SurfaceTextureOesOutputResources>,
    java_vm: JavaVM,
    consumed_frame_available_counts: [u64; VIEW_COUNT],
    latest_transform_matrices: [[f32; 16]; VIEW_COUNT],
    update_rate_start: Instant,
    reports: SurfaceTextureOesReportState,
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
        let mut status = SurfaceTextureOesIngestStatus::new();
        status.codec_mime = source_kind.codec_mime().map(String::from);
        status.notes.push(String::from(
            "Created SurfaceTexture-backed output surfaces; updateTexImage runs on the native GL render thread.",
        ));

        let mut resources =
            SurfaceTextureOesOutputResources::create(&java_vm, egl, source_kind, &mut status)?;

        {
            let mut env = java_vm
                .attach_current_thread()
                .map_err(|error| format!("attach JNI thread for OES source start: {error}"))?;
            match source_kind {
                OesInputSourceKind::None => {
                    status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                    status.notes.push(String::from(
                        "No OES video source requested; rendering static GLES grids.",
                    ));
                }
                OesInputSourceKind::BrokerH264 => {
                    match start_broker_h264_oes_decode_probe(
                        &mut env,
                        app,
                        resources.output_surfaces(),
                        resources.surface_textures(),
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
                            resources.set_decode_probe(probe);
                        }
                        Err(error) => {
                            status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                            status
                                .issue_codes
                                .push(String::from("broker_h264_oes_decode_start_failed"));
                            status.notes.push(error);
                        }
                    }
                }
                OesInputSourceKind::DirectCamera2 => {
                    match start_direct_camera2_oes_probe(
                        &mut env,
                        app,
                        resources.output_surfaces(),
                        resources.surface_textures(),
                        DEFAULT_OES_SURFACE_WIDTH,
                        DEFAULT_OES_SURFACE_HEIGHT,
                    ) {
                        Ok(probe) => {
                            status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                            status.notes.push(String::from(
                                "Started direct Camera2 capture into SurfaceTexture/OES output surfaces.",
                            ));
                            resources.set_decode_probe(probe);
                        }
                        Err(error) => {
                            status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                            status
                                .issue_codes
                                .push(String::from("direct_camera2_oes_start_failed"));
                            status.notes.push(error);
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
            resources.has_decode_probe()
        ));

        Ok(Self {
            status,
            resources: Some(resources),
            java_vm,
            consumed_frame_available_counts: [0; VIEW_COUNT],
            latest_transform_matrices: [identity_texture_transform(); VIEW_COUNT],
            update_rate_start: Instant::now(),
            reports: SurfaceTextureOesReportState::new(),
        })
    }

    pub(super) fn update_textures(&mut self, egl: &EglContext, frame_count: u64) {
        self.reports.apply_latest_decode_report(&mut self.status);
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
            .resources
            .as_ref()
            .and_then(|resources| resources.surface_texture(view_index))
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
        let texture = self.resources.as_ref()?.texture(view_index)?;
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
        self.reports.projection_metadata_pair()
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
}

impl Drop for SurfaceTextureOesProbe {
    fn drop(&mut self) {
        if let Some(resources) = self.resources.take() {
            resources.release(&self.java_vm);
        }
    }
}

fn log_surface_texture_oes_status(status: &SurfaceTextureOesIngestStatus) {
    match serde_json::to_string(status) {
        Ok(json) => log_info(format!("Rusty XR SurfaceTexture OES ingest status {json}")),
        Err(error) => log_error(format!(
            "Rusty XR SurfaceTexture OES status serialization failed: {error}"
        )),
    }
}
