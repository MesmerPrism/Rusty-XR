use super::{
    glActiveTexture, glBindTexture, glDeleteProgram, glGetError, glUseProgram,
    gles_shader_program::{compile_shader, delete_shader, link_program},
    oes_copy_quad::OesCopyQuad,
    oes_copy_shader_sources::{OES_COPY_FRAGMENT_SHADER_SOURCE, OES_COPY_VERTEX_SHADER_SOURCE},
    oes_copy_uniforms::{OesCopyRenderUniforms, OesCopyUniformLocations},
    openxr_gles_config::{
        OesColorControls, OesContentMappingMode, OesPeripheralStretchConfig, OesProcessingLayer,
        OesProjectionAlphaMode, OesProjectionBorderPolicy,
    },
    GL_FRAGMENT_SHADER, GL_NO_ERROR, GL_TEXTURE0, GL_TEXTURE_EXTERNAL_OES, GL_VERTEX_SHADER,
};

pub(super) struct OesCopyRenderer {
    program: u32,
    quad: OesCopyQuad,
    uniforms: OesCopyUniformLocations,
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

        let uniforms = match OesCopyUniformLocations::lookup(program) {
            Ok(uniforms) => uniforms,
            Err(error) => {
                unsafe {
                    glDeleteProgram(program);
                }
                return Err(error);
            }
        };

        let quad = match OesCopyQuad::new() {
            Ok(quad) => quad,
            Err(error) => {
                unsafe {
                    glDeleteProgram(program);
                }
                return Err(error);
            }
        };

        Ok(Self {
            program,
            quad,
            uniforms,
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
        source_texel_size: [f32; 2],
        color_controls: OesColorControls,
    ) -> Result<(), String> {
        unsafe {
            glUseProgram(self.program);
            glActiveTexture(GL_TEXTURE0);
            glBindTexture(GL_TEXTURE_EXTERNAL_OES, source_oes_texture);
            self.uniforms.apply(&OesCopyRenderUniforms {
                eye_index,
                content_mapping_mode,
                screen_to_camera_h,
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
                source_texel_size,
                color_controls,
            });
            self.quad.draw();
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
            if self.program != 0 {
                glDeleteProgram(self.program);
            }
        }
    }
}
