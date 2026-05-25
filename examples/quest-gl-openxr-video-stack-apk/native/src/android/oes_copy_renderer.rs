use std::{
    mem,
    os::raw::{c_int, c_void},
    ptr,
};

use super::{
    glActiveTexture, glBindBuffer, glBindTexture, glBufferData, glDeleteBuffers, glDeleteProgram,
    glDisableVertexAttribArray, glDrawArrays, glEnableVertexAttribArray, glGenBuffers, glGetError,
    glUniform1f, glUniform1i, glUniform2f, glUniform3f, glUniform4f, glUniformMatrix4fv,
    glUseProgram, glVertexAttribPointer,
    gles_shader_program::{compile_shader, delete_shader, link_program, uniform_location},
    oes_copy_shader_sources::{OES_COPY_FRAGMENT_SHADER_SOURCE, OES_COPY_VERTEX_SHADER_SOURCE},
    openxr_gles_config::{
        OesColorControls, OesContentMappingMode, OesProcessingLayer, OesProjectionAlphaMode,
        OesProjectionBorderPolicy,
    },
    GL_ARRAY_BUFFER, GL_FLOAT, GL_FRAGMENT_SHADER, GL_NO_ERROR, GL_STATIC_DRAW, GL_TEXTURE0,
    GL_TEXTURE_EXTERNAL_OES, GL_TRIANGLE_STRIP, GL_VERTEX_SHADER,
};

pub(super) struct OesCopyRenderer {
    program: u32,
    vertex_buffer: u32,
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
    projection_area_eye_offset_uv_location: c_int,
    projection_area_scale_location: c_int,
    projection_area_radius_location: c_int,
    projection_area_corner_radius_uv_location: c_int,
    projection_area_opacity_location: c_int,
    projection_border_opacity_location: c_int,
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

impl OesCopyRenderer {
    pub(super) fn new() -> Result<Self, String> {
        let vertex_shader = compile_shader(GL_VERTEX_SHADER, OES_COPY_VERTEX_SHADER_SOURCE)?;
        let fragment_shader =
            match compile_shader(GL_FRAGMENT_SHADER, OES_COPY_FRAGMENT_SHADER_SOURCE) {
                Ok(shader) => shader,
                Err(error) => {
                    delete_shader(vertex_shader);
                    return Err(error);
                }
            };
        let program = match link_program(vertex_shader, fragment_shader) {
            Ok(program) => program,
            Err(error) => {
                delete_shader(vertex_shader);
                delete_shader(fragment_shader);
                return Err(error);
            }
        };
        delete_shader(vertex_shader);
        delete_shader(fragment_shader);

        let uniform_locations = (|| {
            Ok((
                uniform_location(program, "u_source")?,
                uniform_location(program, "u_screen_to_camera_h0")?,
                uniform_location(program, "u_screen_to_camera_h1")?,
                uniform_location(program, "u_screen_to_camera_h2")?,
                uniform_location(program, "u_source_transform")?,
                uniform_location(program, "u_eye_index")?,
                uniform_location(program, "u_content_mapping_mode")?,
                uniform_location(program, "u_projection_border_policy")?,
                uniform_location(program, "u_processing_layer")?,
                uniform_location(program, "u_blur_radius_px")?,
                uniform_location(program, "u_projection_area_eye_offset_uv")?,
                uniform_location(program, "u_projection_area_scale")?,
                uniform_location(program, "u_projection_area_radius")?,
                uniform_location(program, "u_projection_area_corner_radius_uv")?,
                uniform_location(program, "u_projection_area_opacity")?,
                uniform_location(program, "u_projection_border_opacity")?,
                uniform_location(program, "u_projection_alpha_mode")?,
                uniform_location(program, "u_projection_alpha_transform")?,
                uniform_location(program, "u_source_texel_size")?,
                uniform_location(program, "u_color_matrix_r0")?,
                uniform_location(program, "u_color_matrix_r1")?,
                uniform_location(program, "u_color_matrix_r2")?,
                uniform_location(program, "u_color_offset")?,
                uniform_location(program, "u_color_adjust")?,
                uniform_location(program, "u_source_color_transfer")?,
            ))
        })();
        let (
            sampler_location,
            screen_to_camera_h0_location,
            screen_to_camera_h1_location,
            screen_to_camera_h2_location,
            source_transform_location,
            eye_index_location,
            content_mapping_mode_location,
            projection_border_policy_location,
            processing_layer_location,
            blur_radius_px_location,
            projection_area_eye_offset_uv_location,
            projection_area_scale_location,
            projection_area_radius_location,
            projection_area_corner_radius_uv_location,
            projection_area_opacity_location,
            projection_border_opacity_location,
            projection_alpha_mode_location,
            projection_alpha_transform_location,
            source_texel_size_location,
            color_matrix_r0_location,
            color_matrix_r1_location,
            color_matrix_r2_location,
            color_offset_location,
            color_adjust_location,
            source_color_transfer_location,
        ) = match uniform_locations {
            Ok(locations) => locations,
            Err(error) => {
                unsafe {
                    glDeleteProgram(program);
                }
                return Err(error);
            }
        };

        let vertices: [f32; 16] = [
            -1.0, -1.0, 0.0, 0.0, //
            1.0, -1.0, 1.0, 0.0, //
            -1.0, 1.0, 0.0, 1.0, //
            1.0, 1.0, 1.0, 1.0,
        ];
        let mut vertex_buffer = 0;
        unsafe {
            glGenBuffers(1, &mut vertex_buffer);
            if vertex_buffer == 0 {
                glDeleteProgram(program);
                return Err("glGenBuffers returned 0 for OES copy quad".to_string());
            }
            glBindBuffer(GL_ARRAY_BUFFER, vertex_buffer);
            glBufferData(
                GL_ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<f32>()) as isize,
                vertices.as_ptr().cast(),
                GL_STATIC_DRAW,
            );
            glBindBuffer(GL_ARRAY_BUFFER, 0);
        }

        Ok(Self {
            program,
            vertex_buffer,
            sampler_location,
            screen_to_camera_h0_location,
            screen_to_camera_h1_location,
            screen_to_camera_h2_location,
            source_transform_location,
            eye_index_location,
            content_mapping_mode_location,
            projection_border_policy_location,
            processing_layer_location,
            blur_radius_px_location,
            projection_area_eye_offset_uv_location,
            projection_area_scale_location,
            projection_area_radius_location,
            projection_area_corner_radius_uv_location,
            projection_area_opacity_location,
            projection_border_opacity_location,
            projection_alpha_mode_location,
            projection_alpha_transform_location,
            source_texel_size_location,
            color_matrix_r0_location,
            color_matrix_r1_location,
            color_matrix_r2_location,
            color_offset_location,
            color_adjust_location,
            source_color_transfer_location,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render(
        &mut self,
        source_oes_texture: u32,
        eye_index: usize,
        content_mapping_mode: OesContentMappingMode,
        screen_to_camera_h: [[f32; 3]; 3],
        source_transform: [f32; 16],
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
        source_texel_size: [f32; 2],
        color_controls: OesColorControls,
    ) -> Result<(), String> {
        unsafe {
            glUseProgram(self.program);
            glActiveTexture(GL_TEXTURE0);
            glBindTexture(GL_TEXTURE_EXTERNAL_OES, source_oes_texture);
            glUniform1i(self.sampler_location, 0);
            glUniformMatrix4fv(
                self.source_transform_location,
                1,
                0,
                source_transform.as_ptr(),
            );
            glUniform1i(self.eye_index_location, eye_index as c_int);
            glUniform1i(
                self.content_mapping_mode_location,
                content_mapping_mode.shader_id(),
            );
            glUniform1i(
                self.projection_border_policy_location,
                projection_border_policy.shader_id(),
            );
            glUniform1i(self.processing_layer_location, processing_layer.shader_id());
            glUniform1f(
                self.blur_radius_px_location,
                blur_radius_px.clamp(0.0, 16.0),
            );
            glUniform4f(
                self.projection_area_eye_offset_uv_location,
                projection_area_eye_offset_uv[0][0].clamp(-0.5, 0.5),
                projection_area_eye_offset_uv[0][1].clamp(-0.5, 0.5),
                projection_area_eye_offset_uv[1][0].clamp(-0.5, 0.5),
                projection_area_eye_offset_uv[1][1].clamp(-0.5, 0.5),
            );
            glUniform2f(
                self.projection_area_scale_location,
                projection_area_scale[0].clamp(0.05, 4.0),
                projection_area_scale[1].clamp(0.05, 4.0),
            );
            glUniform2f(
                self.projection_area_radius_location,
                projection_area_radius[0].clamp(0.05, 0.5),
                projection_area_radius[1].clamp(0.05, 0.5),
            );
            glUniform1f(
                self.projection_area_corner_radius_uv_location,
                projection_area_corner_radius_uv.clamp(0.0, 0.5),
            );
            glUniform1f(
                self.projection_area_opacity_location,
                projection_area_opacity.clamp(0.0, 1.0),
            );
            glUniform1f(
                self.projection_border_opacity_location,
                projection_border_opacity.clamp(0.0, 1.0),
            );
            glUniform1i(
                self.projection_alpha_mode_location,
                projection_alpha_mode.shader_id(),
            );
            glUniform2f(
                self.projection_alpha_transform_location,
                projection_alpha_scale.clamp(0.0, 4.0),
                projection_alpha_bias.clamp(-1.0, 1.0),
            );
            glUniform2f(
                self.source_texel_size_location,
                source_texel_size[0],
                source_texel_size[1],
            );
            glUniform3f(
                self.color_matrix_r0_location,
                color_controls.matrix[0][0],
                color_controls.matrix[0][1],
                color_controls.matrix[0][2],
            );
            glUniform3f(
                self.color_matrix_r1_location,
                color_controls.matrix[1][0],
                color_controls.matrix[1][1],
                color_controls.matrix[1][2],
            );
            glUniform3f(
                self.color_matrix_r2_location,
                color_controls.matrix[2][0],
                color_controls.matrix[2][1],
                color_controls.matrix[2][2],
            );
            glUniform3f(
                self.color_offset_location,
                color_controls.offset[0].clamp(-1.0, 1.0),
                color_controls.offset[1].clamp(-1.0, 1.0),
                color_controls.offset[2].clamp(-1.0, 1.0),
            );
            glUniform3f(
                self.color_adjust_location,
                color_controls.contrast.clamp(0.0, 4.0),
                color_controls.brightness.clamp(-1.0, 1.0),
                color_controls.saturation.clamp(0.0, 4.0),
            );
            glUniform1i(
                self.source_color_transfer_location,
                color_controls.source_transfer.shader_id(),
            );
            glUniform3f(
                self.screen_to_camera_h0_location,
                screen_to_camera_h[0][0],
                screen_to_camera_h[0][1],
                screen_to_camera_h[0][2],
            );
            glUniform3f(
                self.screen_to_camera_h1_location,
                screen_to_camera_h[1][0],
                screen_to_camera_h[1][1],
                screen_to_camera_h[1][2],
            );
            glUniform3f(
                self.screen_to_camera_h2_location,
                screen_to_camera_h[2][0],
                screen_to_camera_h[2][1],
                screen_to_camera_h[2][2],
            );
            glBindBuffer(GL_ARRAY_BUFFER, self.vertex_buffer);
            let stride = (4 * mem::size_of::<f32>()) as c_int;
            glEnableVertexAttribArray(0);
            glVertexAttribPointer(0, 2, GL_FLOAT, 0, stride, ptr::null());
            glEnableVertexAttribArray(1);
            glVertexAttribPointer(
                1,
                2,
                GL_FLOAT,
                0,
                stride,
                (2 * mem::size_of::<f32>()) as *const c_void,
            );
            glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
            glDisableVertexAttribArray(0);
            glDisableVertexAttribArray(1);
            glBindBuffer(GL_ARRAY_BUFFER, 0);
            glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0);
            glUseProgram(0);
            let error = glGetError();
            if error != GL_NO_ERROR {
                return Err(format!(
                    "OES full-surface draw returned GL error 0x{error:04x}"
                ));
            }
        }
        Ok(())
    }
}

impl Drop for OesCopyRenderer {
    fn drop(&mut self) {
        unsafe {
            if self.vertex_buffer != 0 {
                glDeleteBuffers(1, &self.vertex_buffer);
            }
            if self.program != 0 {
                glDeleteProgram(self.program);
            }
        }
    }
}
