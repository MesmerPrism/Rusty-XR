use std::{
    mem,
    os::raw::{c_int, c_void},
    ptr,
};

use rusty_xr_quest_diagnostics::GlFramebufferCompleteness;

use super::{
    compile_shader, delete_shader, diagnostic_blur_source_texel_size, glActiveTexture,
    glBindBuffer, glBindFramebuffer, glBindTexture, glBufferData, glCheckFramebufferStatus,
    glClear, glClearColor, glDeleteBuffers, glDeleteFramebuffers, glDeleteProgram, glDisable,
    glDisableVertexAttribArray, glDrawArrays, glEnable, glEnableVertexAttribArray,
    glFramebufferTexture2D, glGenBuffers, glGenFramebuffers, glGetError, glScissor, glUniform1f,
    glUniform1i, glUniform2f, glUniform3f, glUniform4f, glUniformMatrix4fv, glUseProgram,
    glVertexAttribPointer, glViewport, link_program,
    openxr_gles_config::{
        OesColorControls, OesContentMappingMode, OesProcessingLayer, OesProjectionAlphaMode,
        OesProjectionBorderPolicy,
    },
    projection_geometry::identity_homography,
    projection_geometry::OesEyeProjection,
    uniform_location, GL_ARRAY_BUFFER, GL_COLOR_ATTACHMENT0, GL_COLOR_BUFFER_BIT, GL_FLOAT,
    GL_FRAGMENT_SHADER, GL_FRAMEBUFFER, GL_FRAMEBUFFER_COMPLETE,
    GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT, GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT,
    GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE, GL_FRAMEBUFFER_UNSUPPORTED, GL_NO_ERROR,
    GL_SCISSOR_TEST, GL_STATIC_DRAW, GL_TEXTURE0, GL_TEXTURE_2D, GL_TEXTURE_EXTERNAL_OES,
    GL_TRIANGLE_STRIP, GL_VERTEX_SHADER,
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
        let vertex_shader = compile_shader(
            GL_VERTEX_SHADER,
            r#"#version 300 es
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_uv;
out vec2 v_uv;
void main() {
v_uv = a_uv;
gl_Position = vec4(a_position, 0.0, 1.0);
}"#,
        )?;
        let fragment_shader = match compile_shader(
            GL_FRAGMENT_SHADER,
            r#"#version 300 es
#extension GL_OES_EGL_image_external_essl3 : require
precision mediump float;
uniform samplerExternalOES u_source;
uniform vec3 u_screen_to_camera_h0;
uniform vec3 u_screen_to_camera_h1;
uniform vec3 u_screen_to_camera_h2;
uniform mat4 u_source_transform;
uniform int u_eye_index;
uniform int u_content_mapping_mode;
uniform int u_projection_border_policy;
uniform int u_processing_layer;
uniform float u_blur_radius_px;
uniform vec4 u_projection_area_eye_offset_uv;
uniform vec2 u_projection_area_scale;
uniform vec2 u_projection_area_radius;
uniform float u_projection_area_corner_radius_uv;
uniform float u_projection_area_opacity;
uniform float u_projection_border_opacity;
uniform int u_projection_alpha_mode;
uniform vec2 u_projection_alpha_transform;
uniform vec2 u_source_texel_size;
uniform vec3 u_color_matrix_r0;
uniform vec3 u_color_matrix_r1;
uniform vec3 u_color_matrix_r2;
uniform vec3 u_color_offset;
uniform vec3 u_color_adjust;
uniform int u_source_color_transfer;
in vec2 v_uv;
out vec4 out_color;
vec4 premultiplied_alpha_color(vec3 rgb, float alpha) {
float a = clamp(alpha, 0.0, 1.0);
return vec4(clamp(rgb, vec3(0.0), vec3(1.0)) * a, a);
}
vec4 intended_projection_mask_color() {
if (u_projection_border_policy == 1) {
    return vec4(0.0, 0.0, 0.0, 0.0);
}
return premultiplied_alpha_color(vec3(1.0, 0.0, 0.0), u_projection_border_opacity);
}
vec4 source_invalid_color() {
if (u_projection_border_policy == 1) {
    return vec4(0.0, 0.0, 0.0, 0.0);
}
return premultiplied_alpha_color(vec3(1.0, 0.0, 0.0), u_projection_border_opacity);
}
float projection_area_distance(vec2 uv) {
vec2 half_size = vec2(
    clamp(u_projection_area_radius.x, 0.05, 0.50),
    clamp(u_projection_area_radius.y, 0.05, 0.50)
);
float corner_radius = clamp(
    u_projection_area_corner_radius_uv,
    0.0,
    min(half_size.x, half_size.y) - 0.001
);
vec2 q = abs(uv - vec2(0.5)) - (half_size - vec2(corner_radius));
float outside = length(max(q, vec2(0.0)));
float inside = min(max(q.x, q.y), 0.0);
float signed_distance = outside + inside - corner_radius;
return clamp(1.0 + signed_distance / max(min(half_size.x, half_size.y), 0.001), 0.0, 2.0);
}
vec2 projection_area_content_uv(vec2 area_uv) {
vec2 half_size = vec2(
    clamp(u_projection_area_radius.x, 0.05, 0.50),
    clamp(u_projection_area_radius.y, 0.05, 0.50)
);
return (area_uv - (vec2(0.5) - half_size)) / max(half_size * 2.0, vec2(0.001));
}
float srgb_channel_to_linear(float value) {
float c = clamp(value, 0.0, 1.0);
return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
}
vec3 apply_source_color_transfer(vec3 rgb) {
if (u_source_color_transfer == 1) {
    return vec3(
        srgb_channel_to_linear(rgb.r),
        srgb_channel_to_linear(rgb.g),
        srgb_channel_to_linear(rgb.b)
    );
}
return rgb;
}
vec3 adjusted_camera_rgb(vec2 uv) {
vec4 transformed = u_source_transform * vec4(clamp(uv, vec2(0.0), vec2(1.0)), 0.0, 1.0);
vec2 texture_uv = clamp(transformed.xy, vec2(0.0), vec2(1.0));
vec3 source_rgb = apply_source_color_transfer(texture(u_source, texture_uv).rgb);
vec3 adjusted_rgb = vec3(
    dot(u_color_matrix_r0, source_rgb),
    dot(u_color_matrix_r1, source_rgb),
    dot(u_color_matrix_r2, source_rgb)
) + u_color_offset;
float luma = dot(adjusted_rgb, vec3(0.2126, 0.7152, 0.0722));
adjusted_rgb = mix(vec3(luma), adjusted_rgb, max(u_color_adjust.z, 0.0));
adjusted_rgb = (adjusted_rgb - vec3(0.5)) * max(u_color_adjust.x, 0.0) +
    vec3(0.5 + u_color_adjust.y);
return clamp(adjusted_rgb, vec3(0.0), vec3(1.0));
}
float projection_alpha_mask(vec3 rgb) {
vec3 color = clamp(rgb, vec3(0.0), vec3(1.0));
float luma = dot(color, vec3(0.2126, 0.7152, 0.0722));
float max_channel = max(max(color.r, color.g), color.b);
float min_channel = min(min(color.r, color.g), color.b);
float saturation = max_channel - min_channel;
if (u_projection_alpha_mode == 1) {
    return color.r;
}
if (u_projection_alpha_mode == 2) {
    return color.g;
}
if (u_projection_alpha_mode == 3) {
    return color.b;
}
if (u_projection_alpha_mode == 4) {
    return luma;
}
if (u_projection_alpha_mode == 5) {
    return 1.0 - color.r;
}
if (u_projection_alpha_mode == 6) {
    return 1.0 - color.g;
}
if (u_projection_alpha_mode == 7) {
    return 1.0 - color.b;
}
if (u_projection_alpha_mode == 8) {
    return 1.0 - luma;
}
if (u_projection_alpha_mode == 9) {
    return max(color.r - max(color.g, color.b), 0.0);
}
if (u_projection_alpha_mode == 10) {
    return max(color.g - max(color.r, color.b), 0.0);
}
if (u_projection_alpha_mode == 11) {
    return max(color.b - max(color.r, color.g), 0.0);
}
if (u_projection_alpha_mode == 12) {
    return saturation;
}
if (u_projection_alpha_mode == 13) {
    return 1.0 - saturation;
}
return 1.0;
}
float projection_color_alpha(vec3 rgb) {
float mask = projection_alpha_mask(rgb) * max(u_projection_alpha_transform.x, 0.0) +
    u_projection_alpha_transform.y;
return clamp(u_projection_area_opacity * clamp(mask, 0.0, 1.0), 0.0, 1.0);
}
vec4 camera_sample(vec2 uv) {
vec3 rgb = adjusted_camera_rgb(uv);
return premultiplied_alpha_color(rgb, projection_color_alpha(rgb));
}
vec4 blurred_camera_sample(vec2 uv) {
float radius = max(u_blur_radius_px, 0.0);
if (radius <= 0.001) {
    return camera_sample(uv);
}
vec2 texel = u_source_texel_size * radius * 4.0;
vec2 sample_uv = clamp(uv, vec2(0.0), vec2(1.0));
vec3 sum = vec3(0.0);
for (int y = -2; y <= 2; ++y) {
    for (int x = -2; x <= 2; ++x) {
        sum += adjusted_camera_rgb(sample_uv + vec2(float(x), float(y)) * texel);
    }
}
vec3 rgb = sum / 25.0;
return premultiplied_alpha_color(rgb, projection_color_alpha(rgb));
}
void main() {
vec2 renderer_surface_uv = v_uv;
vec2 screen_uv = vec2(renderer_surface_uv.x, 1.0 - renderer_surface_uv.y);
vec2 projection_scale = max(u_projection_area_scale, vec2(0.05));
vec2 requested_projection_area_offset_uv = u_eye_index == 0
    ? u_projection_area_eye_offset_uv.xy
    : u_projection_area_eye_offset_uv.zw;
vec2 projection_area_offset_uv = vec2(
    clamp(requested_projection_area_offset_uv.x, -0.5, 0.5),
    clamp(requested_projection_area_offset_uv.y, -0.5, 0.5)
);
vec2 projection_area_uv =
    (screen_uv - vec2(0.5)) * projection_scale + vec2(0.5) -
    projection_area_offset_uv;
float area_distance = projection_area_distance(projection_area_uv);
if (area_distance > 1.0) {
    out_color = intended_projection_mask_color();
    return;
}
vec2 camera_uv = vec2(0.0);
if (u_content_mapping_mode == 1) {
    camera_uv = projection_area_content_uv(projection_area_uv);
} else {
    vec3 input_uv = vec3(screen_uv, 1.0);
    vec3 camera_uv_h = vec3(
        dot(u_screen_to_camera_h0, input_uv),
        dot(u_screen_to_camera_h1, input_uv),
        dot(u_screen_to_camera_h2, input_uv)
    );
    if (abs(camera_uv_h.z) < 0.00001) {
        out_color = source_invalid_color();
        return;
    }
    camera_uv = camera_uv_h.xy / camera_uv_h.z;
}
if (camera_uv.x < 0.0 || camera_uv.x > 1.0 || camera_uv.y < 0.0 || camera_uv.y > 1.0) {
    out_color = source_invalid_color();
    return;
}
out_color = u_processing_layer == 1
    ? blurred_camera_sample(camera_uv)
    : camera_sample(camera_uv);
}"#,
        ) {
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
                projection_area_eye_offset_uv,
                projection_area_scale,
                projection_area_radius,
                projection_area_corner_radius_uv,
                projection_area_opacity,
                projection_border_opacity,
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
