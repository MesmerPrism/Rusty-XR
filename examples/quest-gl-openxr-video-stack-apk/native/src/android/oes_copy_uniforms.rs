use std::os::raw::c_int;

use super::{
    glUniform1f, glUniform1i, glUniform2f, glUniform3f, glUniform4f, glUniformMatrix4fv,
    gles_shader_program::uniform_location,
    openxr_gles_config::{
        OesColorControls, OesContentMappingMode, OesPeripheralStretchConfig, OesProcessingLayer,
        OesProjectionAlphaMode, OesProjectionBorderPolicy,
    },
};

pub(super) struct OesCopyUniformLocations {
    sampler_location: c_int,
    screen_to_camera_h0_location: c_int,
    screen_to_camera_h1_location: c_int,
    screen_to_camera_h2_location: c_int,
    source_transform_location: c_int,
    eye_index_location: c_int,
    content_mapping_mode_location: c_int,
    projection_border_policy_location: c_int,
    processing_layer_location: c_int,
    blur_radius_px_location: c_int,
    peripheral_stretch_mode_location: c_int,
    peripheral_stretch_params_location: c_int,
    peripheral_stretch_blend_params_location: c_int,
    peripheral_stretch_corner_mode_location: c_int,
    peripheral_stretch_debug_location: c_int,
    projection_area_eye_offset_uv_location: c_int,
    projection_area_scale_location: c_int,
    projection_area_radius_location: c_int,
    projection_area_corner_radius_uv_location: c_int,
    projection_area_opacity_location: c_int,
    projection_border_opacity_location: c_int,
    target_footprint_from_metadata_location: c_int,
    projection_alpha_mode_location: c_int,
    projection_alpha_transform_location: c_int,
    source_texel_size_location: c_int,
    color_matrix_r0_location: c_int,
    color_matrix_r1_location: c_int,
    color_matrix_r2_location: c_int,
    color_offset_location: c_int,
    color_adjust_location: c_int,
    source_color_transfer_location: c_int,
}

impl OesCopyUniformLocations {
    pub(super) fn lookup(program: u32) -> Result<Self, String> {
        Ok(Self {
            sampler_location: uniform_location(program, "u_source")?,
            screen_to_camera_h0_location: uniform_location(program, "u_screen_to_camera_h0")?,
            screen_to_camera_h1_location: uniform_location(program, "u_screen_to_camera_h1")?,
            screen_to_camera_h2_location: uniform_location(program, "u_screen_to_camera_h2")?,
            source_transform_location: uniform_location(program, "u_source_transform")?,
            eye_index_location: uniform_location(program, "u_eye_index")?,
            content_mapping_mode_location: uniform_location(program, "u_content_mapping_mode")?,
            projection_border_policy_location: uniform_location(
                program,
                "u_projection_border_policy",
            )?,
            processing_layer_location: uniform_location(program, "u_processing_layer")?,
            blur_radius_px_location: uniform_location(program, "u_blur_radius_px")?,
            peripheral_stretch_mode_location: uniform_location(
                program,
                "u_peripheral_stretch_mode",
            )?,
            peripheral_stretch_params_location: uniform_location(
                program,
                "u_peripheral_stretch_params",
            )?,
            peripheral_stretch_blend_params_location: uniform_location(
                program,
                "u_peripheral_stretch_blend_params",
            )?,
            peripheral_stretch_corner_mode_location: uniform_location(
                program,
                "u_peripheral_stretch_corner_mode",
            )?,
            peripheral_stretch_debug_location: uniform_location(
                program,
                "u_peripheral_stretch_debug",
            )?,
            projection_area_eye_offset_uv_location: uniform_location(
                program,
                "u_projection_area_eye_offset_uv",
            )?,
            projection_area_scale_location: uniform_location(program, "u_projection_area_scale")?,
            projection_area_radius_location: uniform_location(program, "u_projection_area_radius")?,
            projection_area_corner_radius_uv_location: uniform_location(
                program,
                "u_projection_area_corner_radius_uv",
            )?,
            projection_area_opacity_location: uniform_location(
                program,
                "u_projection_area_opacity",
            )?,
            projection_border_opacity_location: uniform_location(
                program,
                "u_projection_border_opacity",
            )?,
            target_footprint_from_metadata_location: uniform_location(
                program,
                "u_target_footprint_from_metadata",
            )?,
            projection_alpha_mode_location: uniform_location(program, "u_projection_alpha_mode")?,
            projection_alpha_transform_location: uniform_location(
                program,
                "u_projection_alpha_transform",
            )?,
            source_texel_size_location: uniform_location(program, "u_source_texel_size")?,
            color_matrix_r0_location: uniform_location(program, "u_color_matrix_r0")?,
            color_matrix_r1_location: uniform_location(program, "u_color_matrix_r1")?,
            color_matrix_r2_location: uniform_location(program, "u_color_matrix_r2")?,
            color_offset_location: uniform_location(program, "u_color_offset")?,
            color_adjust_location: uniform_location(program, "u_color_adjust")?,
            source_color_transfer_location: uniform_location(program, "u_source_color_transfer")?,
        })
    }

    pub(super) fn apply(&self, uniforms: &OesCopyRenderUniforms) {
        unsafe {
            glUniform1i(self.sampler_location, 0);
            glUniformMatrix4fv(
                self.source_transform_location,
                1,
                0,
                uniforms.source_transform.as_ptr(),
            );
            glUniform1i(self.eye_index_location, uniforms.eye_index as c_int);
            glUniform1i(
                self.content_mapping_mode_location,
                uniforms.content_mapping_mode.shader_id(),
            );
            glUniform1i(
                self.projection_border_policy_location,
                uniforms.projection_border_policy.shader_id(),
            );
            glUniform1i(
                self.processing_layer_location,
                uniforms.processing_layer.shader_id(),
            );
            glUniform1f(
                self.blur_radius_px_location,
                uniforms.blur_radius_px.clamp(0.0, 16.0),
            );
            let peripheral_stretch = uniforms.peripheral_stretch.sanitized();
            glUniform1i(
                self.peripheral_stretch_mode_location,
                peripheral_stretch.mode.shader_id(),
            );
            glUniform4f(
                self.peripheral_stretch_params_location,
                peripheral_stretch.core_scale,
                peripheral_stretch.edge_inset_uv,
                peripheral_stretch.max_inset_uv,
                peripheral_stretch.curve,
            );
            glUniform4f(
                self.peripheral_stretch_blend_params_location,
                peripheral_stretch.inner_blend_uv,
                peripheral_stretch.blend_curve,
                peripheral_stretch.blend_mode.shader_id() as f32,
                0.0,
            );
            glUniform1i(
                self.peripheral_stretch_corner_mode_location,
                peripheral_stretch.corner_mode.shader_id(),
            );
            glUniform1i(
                self.peripheral_stretch_debug_location,
                peripheral_stretch.debug.shader_id(),
            );
            glUniform4f(
                self.projection_area_eye_offset_uv_location,
                uniforms.projection_area_eye_offset_uv[0][0].clamp(-0.5, 0.5),
                uniforms.projection_area_eye_offset_uv[0][1].clamp(-0.5, 0.5),
                uniforms.projection_area_eye_offset_uv[1][0].clamp(-0.5, 0.5),
                uniforms.projection_area_eye_offset_uv[1][1].clamp(-0.5, 0.5),
            );
            glUniform2f(
                self.projection_area_scale_location,
                uniforms.projection_area_scale[0].clamp(0.05, 4.0),
                uniforms.projection_area_scale[1].clamp(0.05, 4.0),
            );
            glUniform2f(
                self.projection_area_radius_location,
                uniforms.projection_area_radius[0].clamp(0.05, 0.5),
                uniforms.projection_area_radius[1].clamp(0.05, 0.5),
            );
            glUniform1f(
                self.projection_area_corner_radius_uv_location,
                uniforms.projection_area_corner_radius_uv.clamp(0.0, 0.5),
            );
            glUniform1f(
                self.projection_area_opacity_location,
                uniforms.projection_area_opacity.clamp(0.0, 1.0),
            );
            glUniform1f(
                self.projection_border_opacity_location,
                uniforms.projection_border_opacity.clamp(0.0, 1.0),
            );
            glUniform1i(
                self.target_footprint_from_metadata_location,
                if uniforms.target_footprint_from_metadata {
                    1
                } else {
                    0
                },
            );
            glUniform1i(
                self.projection_alpha_mode_location,
                uniforms.projection_alpha_mode.shader_id(),
            );
            glUniform2f(
                self.projection_alpha_transform_location,
                uniforms.projection_alpha_scale.clamp(0.0, 4.0),
                uniforms.projection_alpha_bias.clamp(-1.0, 1.0),
            );
            glUniform2f(
                self.source_texel_size_location,
                uniforms.source_texel_size[0],
                uniforms.source_texel_size[1],
            );
            glUniform3f(
                self.color_matrix_r0_location,
                uniforms.color_controls.matrix[0][0],
                uniforms.color_controls.matrix[0][1],
                uniforms.color_controls.matrix[0][2],
            );
            glUniform3f(
                self.color_matrix_r1_location,
                uniforms.color_controls.matrix[1][0],
                uniforms.color_controls.matrix[1][1],
                uniforms.color_controls.matrix[1][2],
            );
            glUniform3f(
                self.color_matrix_r2_location,
                uniforms.color_controls.matrix[2][0],
                uniforms.color_controls.matrix[2][1],
                uniforms.color_controls.matrix[2][2],
            );
            glUniform3f(
                self.color_offset_location,
                uniforms.color_controls.offset[0].clamp(-1.0, 1.0),
                uniforms.color_controls.offset[1].clamp(-1.0, 1.0),
                uniforms.color_controls.offset[2].clamp(-1.0, 1.0),
            );
            glUniform3f(
                self.color_adjust_location,
                uniforms.color_controls.contrast.clamp(0.0, 4.0),
                uniforms.color_controls.brightness.clamp(-1.0, 1.0),
                uniforms.color_controls.saturation.clamp(0.0, 4.0),
            );
            glUniform1i(
                self.source_color_transfer_location,
                uniforms.color_controls.source_transfer.shader_id(),
            );
            glUniform3f(
                self.screen_to_camera_h0_location,
                uniforms.screen_to_camera_h[0][0],
                uniforms.screen_to_camera_h[0][1],
                uniforms.screen_to_camera_h[0][2],
            );
            glUniform3f(
                self.screen_to_camera_h1_location,
                uniforms.screen_to_camera_h[1][0],
                uniforms.screen_to_camera_h[1][1],
                uniforms.screen_to_camera_h[1][2],
            );
            glUniform3f(
                self.screen_to_camera_h2_location,
                uniforms.screen_to_camera_h[2][0],
                uniforms.screen_to_camera_h[2][1],
                uniforms.screen_to_camera_h[2][2],
            );
        }
    }
}

pub(super) struct OesCopyRenderUniforms {
    pub(super) eye_index: usize,
    pub(super) content_mapping_mode: OesContentMappingMode,
    pub(super) screen_to_camera_h: [[f32; 3]; 3],
    pub(super) source_transform: [f32; 16],
    pub(super) projection_border_policy: OesProjectionBorderPolicy,
    pub(super) processing_layer: OesProcessingLayer,
    pub(super) blur_radius_px: f32,
    pub(super) peripheral_stretch: OesPeripheralStretchConfig,
    pub(super) projection_area_eye_offset_uv: [[f32; 2]; 2],
    pub(super) projection_area_scale: [f32; 2],
    pub(super) projection_area_radius: [f32; 2],
    pub(super) projection_area_corner_radius_uv: f32,
    pub(super) projection_area_opacity: f32,
    pub(super) projection_border_opacity: f32,
    pub(super) target_footprint_from_metadata: bool,
    pub(super) projection_alpha_mode: OesProjectionAlphaMode,
    pub(super) projection_alpha_scale: f32,
    pub(super) projection_alpha_bias: f32,
    pub(super) source_texel_size: [f32; 2],
    pub(super) color_controls: OesColorControls,
}
