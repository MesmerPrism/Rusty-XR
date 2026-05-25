use openxr as xr;
use rusty_xr_quest_diagnostics::OpenXrGlesFeasibilityStatus;

use super::{
    egl_gles_context::EglContext,
    glFlush, log_error, log_info,
    oes_copy_renderer::{GlFramebuffer, OesCopyRenderer},
    openxr_gles_config::{
        OesColorControls, OesProcessingLayer, OesProjectionAlphaMode, OesProjectionBorderPolicy,
        OesProjectionRuntimeState,
    },
    openxr_gles_resources::{gl_format_label, EyeSwapchain},
    projection_geometry::{log_projection_diagnostics, OesProjectionPlan},
    surface_texture_oes_frame_sources::{log_oes_submit_diagnostic, OesRenderFrameSources},
    OES_COPY_RENDER_PATH, OES_PROJECTED_RENDER_PATH,
};

pub(super) struct OesRenderFrameInputs<'a> {
    pub(super) egl: &'a EglContext,
    pub(super) swapchains: &'a mut [EyeSwapchain],
    pub(super) frame_count: u64,
    pub(super) status: &'a mut OpenXrGlesFeasibilityStatus,
    pub(super) render_sources: &'a OesRenderFrameSources,
    pub(super) projection_plan: Option<&'a OesProjectionPlan>,
    pub(super) openxr_projection_fields: &'a str,
    pub(super) projection_area_target_fields: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OesRenderTuning {
    pub(super) projection_border_policy: OesProjectionBorderPolicy,
    pub(super) processing_layer: OesProcessingLayer,
    pub(super) blur_radius_px: f32,
    pub(super) projection_area_eye_offset_uv: [[f32; 2]; 2],
    pub(super) projection_area_scale: [f32; 2],
    pub(super) projection_area_radius: [f32; 2],
    pub(super) projection_area_corner_radius_uv: f32,
    pub(super) projection_area_opacity: f32,
    pub(super) projection_border_opacity: f32,
    pub(super) projection_alpha_mode: OesProjectionAlphaMode,
    pub(super) projection_alpha_scale: f32,
    pub(super) projection_alpha_bias: f32,
    pub(super) camera_color_controls: OesColorControls,
}

impl OesRenderTuning {
    pub(super) fn from_projection_state(
        projection_state: OesProjectionRuntimeState,
        processing_layer: OesProcessingLayer,
        blur_radius_px: f32,
        camera_color_controls: OesColorControls,
    ) -> Self {
        Self {
            projection_border_policy: projection_state.projection_border_policy,
            processing_layer,
            blur_radius_px,
            projection_area_eye_offset_uv: projection_state.projection_area_eye_offset_uv,
            projection_area_scale: projection_state.projection_area_scale,
            projection_area_radius: projection_state.projection_area_radius,
            projection_area_corner_radius_uv: projection_state.projection_area_corner_radius_uv,
            projection_area_opacity: projection_state.projection_area_opacity,
            projection_border_opacity: projection_state.projection_border_opacity,
            projection_alpha_mode: projection_state.projection_alpha_mode,
            projection_alpha_scale: projection_state.projection_alpha_scale,
            projection_alpha_bias: projection_state.projection_alpha_bias,
            camera_color_controls,
        }
    }
}

pub(super) struct OesRenderResources {
    fbo: GlFramebuffer,
    oes_copy_renderer: Option<OesCopyRenderer>,
}

impl OesRenderResources {
    pub(super) fn new(status: &mut OpenXrGlesFeasibilityStatus) -> Self {
        let oes_copy_renderer = match OesCopyRenderer::new() {
            Ok(renderer) => Some(renderer),
            Err(error) => {
                status
                    .issue_codes
                    .push(String::from("oes_copy_renderer_create_failed"));
                status.notes.push(format!(
                    "Could not create the public OES full-surface copy renderer: {error}"
                ));
                log_error(format!(
                    "Rusty XR OpenXR GLES OES copy renderer creation failed: {error}"
                ));
                None
            }
        };
        Self {
            fbo: GlFramebuffer::new(),
            oes_copy_renderer,
        }
    }

    pub(super) fn render_eye_swapchains(
        &mut self,
        inputs: OesRenderFrameInputs<'_>,
        tuning: OesRenderTuning,
    ) -> Result<(), String> {
        let OesRenderFrameInputs {
            egl,
            swapchains,
            frame_count,
            status,
            render_sources,
            projection_plan,
            openxr_projection_fields,
            projection_area_target_fields,
        } = inputs;
        let OesRenderTuning {
            projection_border_policy,
            processing_layer,
            blur_radius_px,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_corner_radius_uv,
            projection_area_opacity,
            projection_border_opacity,
            projection_alpha_mode,
            projection_alpha_scale,
            projection_alpha_bias,
            camera_color_controls,
        } = tuning;

        egl.make_current()?;
        for eye in swapchains {
            let image_index = eye.handle.acquire_image().map_err(|error| {
                format!(
                    "acquire GLES swapchain image eye {}: {error}",
                    eye.view_index
                )
            })?;
            eye.handle
                .wait_image(xr::Duration::INFINITE)
                .map_err(|error| {
                    format!("wait GLES swapchain image eye {}: {error}", eye.view_index)
                })?;
            let texture = eye
                .images
                .get(image_index as usize)
                .copied()
                .ok_or_else(|| format!("swapchain image index {image_index} is out of range"))?;

            let mut render_path = eye.pattern;
            let mut rendered_source_sequence = None;
            let fbo_status = if let (Some(source), Some(renderer)) = (
                render_sources.eye(eye.view_index),
                self.oes_copy_renderer.as_mut(),
            ) {
                let eye_projection = projection_plan.and_then(|plan| plan.eye(eye.view_index));
                let source_transform = eye_projection
                    .map(|projection| {
                        projection.source_transform_for_sample(source.transform_matrix)
                    })
                    .unwrap_or(source.transform_matrix);
                match self.fbo.render_external_oes(
                    texture,
                    source.texture,
                    source_transform,
                    eye.width,
                    eye.height,
                    eye.view_index,
                    renderer,
                    eye_projection,
                    projection_border_policy,
                    processing_layer,
                    blur_radius_px,
                    projection_area_eye_offset_uv,
                    projection_area_scale,
                    projection_area_radius,
                    projection_area_corner_radius_uv,
                    projection_area_opacity,
                    projection_border_opacity,
                    projection_alpha_mode,
                    projection_alpha_scale,
                    projection_alpha_bias,
                    camera_color_controls,
                ) {
                    Ok(fbo_status) => {
                        render_path = if eye_projection.is_some() {
                            OES_PROJECTED_RENDER_PATH
                        } else {
                            OES_COPY_RENDER_PATH
                        };
                        rendered_source_sequence = Some(source.source_sequence);
                        if frame_count == 0 || frame_count.is_multiple_of(120) {
                            log_oes_submit_diagnostic(
                                eye.view_index,
                                frame_count,
                                source,
                                render_path,
                            );
                            log_projection_diagnostics(
                                eye.view_index,
                                frame_count,
                                source.source_sequence,
                                eye_projection,
                                projection_border_policy,
                                camera_color_controls,
                                eye.color_format,
                                openxr_projection_fields,
                                projection_area_target_fields,
                            );
                        }
                        fbo_status
                    }
                    Err(error) => {
                        status
                            .issue_codes
                            .push(String::from("oes_to_swapchain_copy_failed"));
                        log_error(format!(
                            "Rusty XR OpenXR GLES OES copy failed eye={} frame={}: {error}",
                            eye.view_index, frame_count
                        ));
                        self.fbo
                            .render_grid(texture, eye.width, eye.height, eye.view_index)?
                    }
                }
            } else {
                self.fbo
                    .render_grid(texture, eye.width, eye.height, eye.view_index)?
            };
            if let Some(view) = status.views.get_mut(eye.view_index) {
                view.acquired_image_index = Some(image_index);
                view.fbo_status = fbo_status;
                view.viewport_x = 0;
                view.viewport_y = 0;
                view.viewport_width = eye.width;
                view.viewport_height = eye.height;
                view.diagnostic_pattern = render_path.to_string();
                view.last_rendered_frame_index = Some(frame_count);
            }
            if frame_count == 0 || frame_count.is_multiple_of(120) {
                log_info(format!(
                    "Rusty XR OpenXR GLES rendered eye={} imageIndex={} texture={} viewport={}x{} colorFormat={} fbo={:?} pattern={} sourceSequence={:?}",
                    eye.view_index,
                    image_index,
                    texture,
                    eye.width,
                    eye.height,
                    gl_format_label(eye.color_format),
                    fbo_status,
                    render_path,
                    rendered_source_sequence
                ));
            }
            eye.handle.release_image().map_err(|error| {
                format!(
                    "release GLES swapchain image eye {}: {error}",
                    eye.view_index
                )
            })?;
        }
        unsafe {
            glFlush();
        }
        Ok(())
    }
}
