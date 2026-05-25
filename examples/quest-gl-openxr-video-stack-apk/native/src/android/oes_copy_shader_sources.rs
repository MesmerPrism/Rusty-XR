pub(super) const OES_COPY_VERTEX_SHADER_SOURCE: &str = r#"#version 300 es
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_uv;
out vec2 v_uv;
void main() {
v_uv = a_uv;
gl_Position = vec4(a_position, 0.0, 1.0);
}"#;

pub(super) const OES_COPY_FRAGMENT_SHADER_SOURCE: &str = r#"#version 300 es
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
}"#;
