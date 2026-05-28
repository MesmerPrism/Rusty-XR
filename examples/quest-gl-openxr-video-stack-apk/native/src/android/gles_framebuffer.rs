use std::os::raw::c_int;

use rusty_xr_quest_diagnostics::GlFramebufferCompleteness;

use super::{
    glBindFramebuffer, glCheckFramebufferStatus, glClear, glClearColor, glDeleteFramebuffers,
    glDisable, glEnable, glFramebufferTexture2D, glGenFramebuffers, glScissor, glViewport,
    oes_copy_renderer::OesCopyRenderer,
    openxr_gles_config::{
        OesColorControls, OesPeripheralStretchConfig, OesProcessingLayer, OesProjectionAlphaMode,
        OesProjectionBorderPolicy,
    },
    projection_geometry::{identity_homography, OesEyeProjection},
    GL_COLOR_ATTACHMENT0, GL_COLOR_BUFFER_BIT, GL_FRAMEBUFFER, GL_FRAMEBUFFER_COMPLETE,
    GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT, GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT,
    GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE, GL_FRAMEBUFFER_UNSUPPORTED, GL_SCISSOR_TEST,
    GL_TEXTURE_2D,
};

const DIAGNOSTIC_BLUR_SOURCE_WIDTH_PX: f32 = 1280.0;
const DIAGNOSTIC_BLUR_SOURCE_HEIGHT_PX: f32 = 1280.0;

pub(super) struct GlFramebuffer {
    id: u32,
}

impl GlFramebuffer {
    pub(super) fn new() -> Self {
        let mut id = 0;
        unsafe {
            glGenFramebuffers(1, &mut id);
        }
        Self { id }
    }

    pub(super) fn render_grid(
        &mut self,
        texture: u32,
        width: u32,
        height: u32,
        view_index: usize,
    ) -> Result<GlFramebufferCompleteness, String> {
        unsafe {
            glBindFramebuffer(GL_FRAMEBUFFER, self.id);
            glFramebufferTexture2D(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                texture,
                0,
            );
            let raw_status = glCheckFramebufferStatus(GL_FRAMEBUFFER);
            let fbo_status = framebuffer_status(raw_status);
            if !fbo_status.is_complete() {
                glBindFramebuffer(GL_FRAMEBUFFER, 0);
                return Ok(fbo_status);
            }

            glViewport(0, 0, width as c_int, height as c_int);
            let (background, grid) = if view_index == 0 {
                ([0.12, 0.02, 0.02, 1.0], [0.0, 0.75, 0.85, 1.0])
            } else {
                ([0.02, 0.04, 0.18, 1.0], [1.0, 0.85, 0.05, 1.0])
            };
            glClearColor(background[0], background[1], background[2], background[3]);
            glClear(GL_COLOR_BUFFER_BIT);
            glEnable(GL_SCISSOR_TEST);
            glClearColor(grid[0], grid[1], grid[2], grid[3]);

            let vertical_step = (width / 8).max(1);
            for x in (0..width).step_by(vertical_step as usize) {
                glScissor(x as c_int, 0, 4, height as c_int);
                glClear(GL_COLOR_BUFFER_BIT);
            }
            let horizontal_step = (height / 8).max(1);
            for y in (0..height).step_by(horizontal_step as usize) {
                glScissor(0, y as c_int, width as c_int, 4);
                glClear(GL_COLOR_BUFFER_BIT);
            }

            let marker_width = (width / 5).max(16);
            let marker_height = (height / 5).max(16);
            let marker_x = if view_index == 0 {
                width / 12
            } else {
                width.saturating_sub(marker_width + width / 12)
            };
            let marker_y = height / 2 - marker_height / 2;
            glClearColor(0.95, 0.95, 0.95, 1.0);
            glScissor(
                marker_x as c_int,
                marker_y as c_int,
                marker_width as c_int,
                marker_height as c_int,
            );
            glClear(GL_COLOR_BUFFER_BIT);
            glDisable(GL_SCISSOR_TEST);
            glBindFramebuffer(GL_FRAMEBUFFER, 0);
            Ok(fbo_status)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_external_oes(
        &mut self,
        target_texture: u32,
        source_oes_texture: u32,
        source_transform: [f32; 16],
        width: u32,
        height: u32,
        view_index: usize,
        renderer: &mut OesCopyRenderer,
        projection: Option<&OesEyeProjection>,
        projection_border_policy: OesProjectionBorderPolicy,
        processing_layer: OesProcessingLayer,
        blur_radius_px: f32,
        peripheral_stretch: OesPeripheralStretchConfig,
        projection_area_eye_offset_uv: [[f32; 2]; 2],
        projection_area_scale: [f32; 2],
        projection_area_radius: [f32; 2],
        projection_area_corner_radius_uv: f32,
        projection_area_opacity: f32,
        projection_border_opacity: f32,
        target_footprint_from_metadata: bool,
        projection_alpha_mode: OesProjectionAlphaMode,
        projection_alpha_scale: f32,
        projection_alpha_bias: f32,
        camera_color_controls: OesColorControls,
    ) -> Result<GlFramebufferCompleteness, String> {
        unsafe {
            glBindFramebuffer(GL_FRAMEBUFFER, self.id);
            glFramebufferTexture2D(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                target_texture,
                0,
            );
            let raw_status = glCheckFramebufferStatus(GL_FRAMEBUFFER);
            let fbo_status = framebuffer_status(raw_status);
            if !fbo_status.is_complete() {
                glBindFramebuffer(GL_FRAMEBUFFER, 0);
                return Ok(fbo_status);
            }

            glViewport(0, 0, width as c_int, height as c_int);
            let (clear_r, clear_g, clear_b, clear_a) = projection_border_policy.clear_color();
            glClearColor(
                clear_r,
                clear_g,
                clear_b,
                clear_a * projection_border_opacity.clamp(0.0, 1.0),
            );
            glClear(GL_COLOR_BUFFER_BIT);
            renderer.render(
                source_oes_texture,
                view_index,
                projection
                    .map(|projection| projection.content_mapping_mode)
                    .unwrap_or_default(),
                projection
                    .map(|projection| projection.screen_to_camera_h)
                    .unwrap_or_else(identity_homography),
                source_transform,
                projection_border_policy,
                processing_layer,
                blur_radius_px,
                peripheral_stretch,
                projection_area_eye_offset_uv,
                projection_area_scale,
                projection_area_radius,
                projection_area_corner_radius_uv,
                projection_area_opacity,
                projection_border_opacity,
                target_footprint_from_metadata,
                projection_alpha_mode,
                projection_alpha_scale,
                projection_alpha_bias,
                diagnostic_blur_source_texel_size(),
                camera_color_controls,
            )?;
            glBindFramebuffer(GL_FRAMEBUFFER, 0);
            Ok(fbo_status)
        }
    }
}

impl Drop for GlFramebuffer {
    fn drop(&mut self) {
        unsafe {
            if self.id != 0 {
                glDeleteFramebuffers(1, &self.id);
            }
        }
    }
}

fn diagnostic_blur_source_texel_size() -> [f32; 2] {
    [
        1.0 / DIAGNOSTIC_BLUR_SOURCE_WIDTH_PX.max(1.0),
        1.0 / DIAGNOSTIC_BLUR_SOURCE_HEIGHT_PX.max(1.0),
    ]
}

fn framebuffer_status(raw: u32) -> GlFramebufferCompleteness {
    match raw {
        GL_FRAMEBUFFER_COMPLETE => GlFramebufferCompleteness::Complete,
        GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT => GlFramebufferCompleteness::IncompleteAttachment,
        GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => {
            GlFramebufferCompleteness::IncompleteMissingAttachment
        }
        GL_FRAMEBUFFER_UNSUPPORTED => GlFramebufferCompleteness::IncompleteUnsupported,
        GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE => GlFramebufferCompleteness::IncompleteMultisample,
        _ => GlFramebufferCompleteness::OtherIncomplete,
    }
}
