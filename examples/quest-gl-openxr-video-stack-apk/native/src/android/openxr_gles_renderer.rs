use openxr as xr;
use rusty_xr_quest_diagnostics::OpenXrGlesFeasibilityStatus;

use super::{
    egl_gles_context::EglContext,
    glFlush, log_error, log_info,
    oes_copy_renderer::{GlFramebuffer, OesColorControls, OesCopyRenderer},
    openxr_gles_resources::{gl_format_label, EyeSwapchain},
    projection_geometry::{log_projection_diagnostics, OesProjectionPlan},
    surface_texture_oes_probe::{log_oes_submit_diagnostic, SurfaceTextureOesProbe},
    OesProcessingLayer, OesProjectionAlphaMode, OesProjectionBorderPolicy, OES_COPY_RENDER_PATH,
    OES_PROJECTED_RENDER_PATH,
};

pub(super) fn render_eye_swapchains(
    egl: &EglContext,
    fbo: &mut GlFramebuffer,
    swapchains: &mut [EyeSwapchain],
    frame_count: u64,
    status: &mut OpenXrGlesFeasibilityStatus,
    surface_texture_oes_probe: Option<&SurfaceTextureOesProbe>,
    projection_plan: Option<&OesProjectionPlan>,
    oes_copy_renderer: &mut Option<OesCopyRenderer>,
    projection_border_policy: OesProjectionBorderPolicy,
    processing_layer: OesProcessingLayer,
    blur_radius_px: f32,
    projection_area_eye_offset_uv: [[f32; 2]; 2],
    projection_area_scale: [f32; 2],
    projection_area_radius: [f32; 2],
    projection_area_corner_radius_uv: f32,
    projection_area_opacity: f32,
    projection_border_opacity: f32,
    projection_alpha_mode: OesProjectionAlphaMode,
    projection_alpha_scale: f32,
    projection_alpha_bias: f32,
    camera_color_controls: OesColorControls,
    openxr_projection_fields: &str,
    projection_area_target_fields: &str,
) -> Result<(), String> {
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
        let fbo_status = if let (Some(probe), Some(renderer)) =
            (surface_texture_oes_probe, oes_copy_renderer.as_mut())
        {
            if let Some(source) = probe.updated_eye_texture(eye.view_index) {
                let eye_projection = projection_plan.and_then(|plan| plan.eye(eye.view_index));
                let source_transform = eye_projection
                    .map(|projection| {
                        projection.source_transform_for_sample(source.transform_matrix)
                    })
                    .unwrap_or(source.transform_matrix);
                match fbo.render_external_oes(
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
                            let frame_age_at_submit_ms = source
                                .queued_pts_us
                                .and_then(|pts_us| probe.frame_age_at_submit_ms(pts_us));
                            log_oes_submit_diagnostic(
                                eye.view_index,
                                frame_count,
                                &source,
                                frame_age_at_submit_ms,
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
                                &projection_area_target_fields,
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
                        fbo.render_grid(texture, eye.width, eye.height, eye.view_index)?
                    }
                }
            } else {
                fbo.render_grid(texture, eye.width, eye.height, eye.view_index)?
            }
        } else {
            fbo.render_grid(texture, eye.width, eye.height, eye.view_index)?
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
