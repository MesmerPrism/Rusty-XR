use super::openxr_gles_config::activity_string_extra;
use super::source_metadata::{OesInputSourceKind, OesProjectionMetadata};
use super::surface_texture_oes_callbacks::reset_decode_callbacks;
use super::surface_texture_oes_frame_sources::OesEyeTextureSample;
use super::surface_texture_oes_outputs::{
    SurfaceTextureOesOutputResources, DEFAULT_OES_SURFACE_HEIGHT, DEFAULT_OES_SURFACE_WIDTH,
};
use super::surface_texture_oes_reports::SurfaceTextureOesReportState;
use super::surface_texture_oes_sources::{
    start_broker_h264_oes_decode_probe, start_direct_camera2_oes_probe, BROKER_H264_DEFAULT_HOST,
    BROKER_H264_LEFT_STREAM_PORT, BROKER_H264_RIGHT_STREAM_PORT,
};
use super::surface_texture_oes_transform::android_elapsed_realtime_nanos;
use super::surface_texture_oes_update::SurfaceTextureOesUpdateState;
use super::{log_error, log_info, EglContext};
use jni::{objects::JObject, sys::jobject, JavaVM};
use rusty_xr_quest_diagnostics::{SurfaceTextureOesIngestState, SurfaceTextureOesIngestStatus};

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
    update_state: SurfaceTextureOesUpdateState,
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
            update_state: SurfaceTextureOesUpdateState::new(),
            reports: SurfaceTextureOesReportState::new(),
        })
    }

    pub(super) fn update_textures(&mut self, egl: &EglContext, frame_count: u64) {
        self.reports.apply_latest_decode_report(&mut self.status);
        let updated_any = self.update_state.update_textures(
            egl,
            &self.java_vm,
            self.resources.as_ref(),
            &mut self.status,
            frame_count,
        );
        if updated_any {
            log_surface_texture_oes_status(&self.status);
        }
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
            transform_matrix: self.update_state.transform_matrix(view_index),
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
