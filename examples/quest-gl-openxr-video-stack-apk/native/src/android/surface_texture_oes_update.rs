use jni::JavaVM;
use rusty_xr_quest_diagnostics::{
    FrameRateSummary, SurfaceTextureOesIngestState, SurfaceTextureOesIngestStatus,
};
use std::time::Instant;

use super::{
    log_error,
    surface_texture_oes_callbacks::decode_frame_snapshot,
    surface_texture_oes_gl::identity_texture_transform,
    surface_texture_oes_outputs::SurfaceTextureOesOutputResources,
    surface_texture_oes_transform::{
        log_surface_texture_transform_matrix, sample_surface_texture_transform_matrix,
        transform_matrix_hash,
    },
    EglContext, VIEW_COUNT,
};

pub(super) struct SurfaceTextureOesUpdateState {
    consumed_frame_available_counts: [u64; VIEW_COUNT],
    latest_transform_matrices: [[f32; 16]; VIEW_COUNT],
    update_rate_start: Instant,
}

impl SurfaceTextureOesUpdateState {
    pub(super) fn new() -> Self {
        Self {
            consumed_frame_available_counts: [0; VIEW_COUNT],
            latest_transform_matrices: [identity_texture_transform(); VIEW_COUNT],
            update_rate_start: Instant::now(),
        }
    }

    pub(super) fn update_textures(
        &mut self,
        egl: &EglContext,
        java_vm: &JavaVM,
        resources: Option<&SurfaceTextureOesOutputResources>,
        status: &mut SurfaceTextureOesIngestStatus,
        frame_count: u64,
    ) -> bool {
        let mut updated_any = false;
        for view_index in 0..VIEW_COUNT {
            let (available_count, latest_sequence, latest_pts_us) =
                decode_frame_snapshot(view_index);
            if available_count <= self.consumed_frame_available_counts[view_index] {
                if let Some(eye) = status.eyes.get_mut(view_index) {
                    eye.frame_available_count = available_count;
                }
                continue;
            }

            let skipped = available_count
                .saturating_sub(self.consumed_frame_available_counts[view_index])
                .saturating_sub(1);
            match update_surface_texture(egl, java_vm, resources, view_index) {
                Ok((timestamp_ns, transform_hash, transform_matrix)) => {
                    self.latest_transform_matrices[view_index] = transform_matrix;
                    if let Some(eye) = status.eyes.get_mut(view_index) {
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
                    if let Some(eye) = status.eyes.get_mut(view_index) {
                        eye.frame_available_count = available_count;
                        eye.decoder_error_count = eye.decoder_error_count.saturating_add(1);
                        eye.latest_decoder_error = Some(error.clone());
                    }
                    status
                        .issue_codes
                        .push(String::from("surface_texture_update_failed"));
                    log_error(format!(
                        "Rusty XR SurfaceTexture OES update failed eye={view_index}: {error}"
                    ));
                }
            }
        }

        if updated_any {
            status.state = SurfaceTextureOesIngestState::TextureUpdated;
            self.refresh_texture_update_rate(status);
        }

        updated_any
    }

    pub(super) fn transform_matrix(&self, view_index: usize) -> [f32; 16] {
        self.latest_transform_matrices
            .get(view_index)
            .copied()
            .unwrap_or_else(identity_texture_transform)
    }

    fn refresh_texture_update_rate(&mut self, status: &mut SurfaceTextureOesIngestStatus) {
        let elapsed = self.update_rate_start.elapsed().as_secs_f32();
        if elapsed <= 0.0 {
            return;
        }
        let sample_count = status
            .eyes
            .iter()
            .map(|eye| eye.update_tex_image_count)
            .sum::<u64>();
        let average_fps = sample_count as f32 / elapsed;
        status.texture_update_rate = Some(FrameRateSummary {
            sample_count,
            average_fps,
            min_fps: average_fps,
            max_fps: average_fps,
        });
    }
}

fn update_surface_texture(
    egl: &EglContext,
    java_vm: &JavaVM,
    resources: Option<&SurfaceTextureOesOutputResources>,
    view_index: usize,
) -> Result<(i64, String, [f32; 16]), String> {
    egl.make_current()?;
    let surface_texture = resources
        .and_then(|resources| resources.surface_texture(view_index))
        .ok_or_else(|| format!("SurfaceTexture eye index {view_index} is out of range"))?;
    let mut env = java_vm
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
