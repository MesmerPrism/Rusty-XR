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
precision highp float;
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
uniform int u_peripheral_stretch_mode;
uniform vec4 u_peripheral_stretch_params;
uniform vec4 u_peripheral_stretch_blend_params;
uniform int u_peripheral_stretch_corner_mode;
uniform int u_peripheral_stretch_debug;
uniform vec4 u_projection_area_eye_offset_uv;
uniform vec2 u_projection_area_scale;
uniform vec2 u_projection_area_radius;
uniform float u_projection_area_corner_radius_uv;
uniform float u_projection_area_opacity;
uniform float u_projection_border_opacity;
uniform int u_target_footprint_from_metadata;
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
if (u_processing_layer == 2 && u_peripheral_stretch_debug == 1) {
    return premultiplied_alpha_color(vec3(1.0, 0.0, 1.0), u_projection_border_opacity);
}
return premultiplied_alpha_color(vec3(1.0, 0.0, 0.0), u_projection_border_opacity);
}
float target_footprint_signed_distance_uv(vec2 uv) {
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
return outside + inside - corner_radius;
}
float projection_area_distance(vec2 uv) {
float signed_distance = target_footprint_signed_distance_uv(uv);
vec2 half_size = vec2(
    clamp(u_projection_area_radius.x, 0.05, 0.50),
    clamp(u_projection_area_radius.y, 0.05, 0.50)
);
return clamp(1.0 + signed_distance / max(min(half_size.x, half_size.y), 0.001), 0.0, 2.0);
}
vec2 projection_area_content_uv(vec2 area_uv) {
vec2 half_size = vec2(
    clamp(u_projection_area_radius.x, 0.05, 0.50),
    clamp(u_projection_area_radius.y, 0.05, 0.50)
);
return (area_uv - (vec2(0.5) - half_size)) / max(half_size * 2.0, vec2(0.001));
}
vec2 projection_area_uv_to_screen_uv(
    vec2 area_uv,
    vec2 projection_area_offset_uv,
    vec2 projection_scale
) {
return (area_uv + projection_area_offset_uv - vec2(0.5)) /
    max(projection_scale, vec2(0.05)) + vec2(0.5);
}
float smooth_unit(float value) {
value = clamp(value, 0.0, 1.0);
return value * value * (3.0 - 2.0 * value);
}
vec2 projection_area_rect_edge_uv(
    vec2 area_uv,
    vec2 domain_min_uv,
    vec2 domain_max_uv,
    bool force_edge_sample
) {
vec2 half_size = vec2(
    clamp(u_projection_area_radius.x, 0.05, 0.50),
    clamp(u_projection_area_radius.y, 0.05, 0.50)
);
float core_scale = clamp(u_peripheral_stretch_params.x, 0.05, 1.0);
vec2 core_half_size = half_size * core_scale;
vec2 p = area_uv - vec2(0.5);
vec2 normalized = p / max(core_half_size, vec2(0.001));
float edge_distance = max(abs(normalized.x), abs(normalized.y));
if (edge_distance <= 1.0 && !force_edge_sample) {
    return area_uv;
}

float effective_edge_distance = force_edge_sample ? max(edge_distance, 1.0) : edge_distance;
vec2 edge_normalized = normalized / max(edge_distance, 0.0001);
vec2 edge_direction_uv = edge_normalized * core_half_size;
vec2 bounded_min_uv = min(domain_min_uv, domain_max_uv);
vec2 bounded_max_uv = max(domain_min_uv, domain_max_uv);

float reach_x = 1.0e6;
if (edge_direction_uv.x > 0.0001) {
    reach_x = (bounded_max_uv.x - 0.5) / edge_direction_uv.x;
} else if (edge_direction_uv.x < -0.0001) {
    reach_x = (bounded_min_uv.x - 0.5) / edge_direction_uv.x;
}
float reach_y = 1.0e6;
if (edge_direction_uv.y > 0.0001) {
    reach_y = (bounded_max_uv.y - 0.5) / edge_direction_uv.y;
} else if (edge_direction_uv.y < -0.0001) {
    reach_y = (bounded_min_uv.y - 0.5) / edge_direction_uv.y;
}
float exterior_reach = max(min(reach_x, reach_y) - 1.0, 0.0001);
float exterior_t = clamp((effective_edge_distance - 1.0) / exterior_reach, 0.0, 1.0);
exterior_t = smooth_unit(exterior_t);

float edge_inset = clamp(u_peripheral_stretch_params.y, 0.0, 0.49);
float max_inset = clamp(max(u_peripheral_stretch_params.z, edge_inset), 0.0, 0.49);
float curve = clamp(u_peripheral_stretch_params.w, 0.25, 6.0);
float shaped_t = pow(exterior_t, curve);
float inset = mix(edge_inset, max_inset, shaped_t);
vec2 sample_half_size = max(core_half_size - vec2(inset), vec2(0.001));
vec2 sample_uv = vec2(0.5) + edge_normalized * sample_half_size;
return clamp(sample_uv, bounded_min_uv, bounded_max_uv);
}
float peripheral_stretch_blend_weight(float signed_distance_uv) {
float inner_blend = clamp(u_peripheral_stretch_blend_params.x, 0.0, 0.25);
float blend_curve = clamp(u_peripheral_stretch_blend_params.y, 0.25, 6.0);
float blend_mode = floor(u_peripheral_stretch_blend_params.z + 0.5);
if (blend_mode < 0.5) {
    return signed_distance_uv >= 0.0 ? 1.0 : 0.0;
}
if (inner_blend <= 0.0001) {
    return signed_distance_uv >= 0.0 ? 1.0 : 0.0;
}
float t = smoothstep(-inner_blend, 0.0, signed_distance_uv);
return pow(t, blend_curve);
}
bool peripheral_stretch_enabled() {
// Keep the mode/corner uniforms live for the public runtime contract, but let
// the processing layer own effect activation like the HWB shader path.
return u_processing_layer == 2 ||
    (u_peripheral_stretch_mode + u_peripheral_stretch_corner_mode) == -4096;
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
vec2 canonical_projection_area_uv = projection_area_uv;
float signed_distance_uv = target_footprint_signed_distance_uv(canonical_projection_area_uv);
float area_distance = projection_area_distance(canonical_projection_area_uv);
bool stretch_exterior = false;
bool target_transition_band = false;
bool peripheral_stretch_active = peripheral_stretch_enabled();
float stretch_weight = peripheral_stretch_active
    ? peripheral_stretch_blend_weight(signed_distance_uv)
    : 0.0;
bool projection_area_inside = signed_distance_uv <= 0.0;
stretch_exterior = peripheral_stretch_active && !projection_area_inside;
target_transition_band =
    peripheral_stretch_active && projection_area_inside && stretch_weight > 0.0001;
bool stretch_effect_sample_region = stretch_exterior || target_transition_band;
if (stretch_effect_sample_region) {
    vec2 domain_min_uv =
        vec2(0.5) - vec2(0.5) * projection_scale - projection_area_offset_uv;
    vec2 domain_max_uv =
        vec2(0.5) + vec2(0.5) * projection_scale - projection_area_offset_uv;
    vec2 stretch_projection_area_uv = projection_area_rect_edge_uv(
        canonical_projection_area_uv,
        domain_min_uv,
        domain_max_uv,
        stretch_weight > 0.0001
    );
    projection_area_uv = mix(
        canonical_projection_area_uv,
        stretch_projection_area_uv,
        clamp(stretch_weight, 0.0, 1.0)
    );
    screen_uv = projection_area_uv_to_screen_uv(
        projection_area_uv,
        projection_area_offset_uv,
        projection_scale
    );
} else if (area_distance > 1.0) {
    out_color = intended_projection_mask_color();
    return;
}
vec2 target_content_uv = projection_area_content_uv(projection_area_uv);
vec2 camera_uv = vec2(0.0);
if (u_content_mapping_mode == 1) {
    camera_uv = target_content_uv;
} else {
    vec2 homography_input_uv = screen_uv;
    // Keep the metadata flag live for the uniform contract. The camera
    // homography rows are screen-domain; the target footprint only owns the
    // mask and edge-remap boundary.
    if (u_target_footprint_from_metadata == 2) {
        homography_input_uv = target_content_uv;
    }
    vec3 input_uv = vec3(homography_input_uv, 1.0);
    vec3 camera_uv_h = vec3(
        dot(u_screen_to_camera_h0, input_uv),
        dot(u_screen_to_camera_h1, input_uv),
        dot(u_screen_to_camera_h2, input_uv)
    );
    if (abs(camera_uv_h.z) < 0.00001 && !peripheral_stretch_active) {
        out_color = source_invalid_color();
        return;
    }
    float safe_camera_uv_z = abs(camera_uv_h.z) < 0.00001
        ? (camera_uv_h.z < 0.0 ? -0.00001 : 0.00001)
        : camera_uv_h.z;
    camera_uv = camera_uv_h.xy / safe_camera_uv_z;
}
bool source_uv_numeric = camera_uv.x == camera_uv.x && camera_uv.y == camera_uv.y;
if (peripheral_stretch_active && stretch_effect_sample_region && !source_uv_numeric) {
    // NaN/undefined transition/exterior samples still belong to the stretch
    // layer; core failures remain source-invalid diagnostics.
    camera_uv = vec2(0.5);
    source_uv_numeric = true;
}
bool source_uv_valid =
    source_uv_numeric &&
    camera_uv.x >= 0.0 &&
    camera_uv.x <= 1.0 &&
    camera_uv.y >= 0.0 &&
    camera_uv.y <= 1.0;
if (peripheral_stretch_active && stretch_effect_sample_region && !source_uv_valid) {
    // Treat source-invalid transition/exterior pixels as part of the explicit
    // stretch layer so edge clamping cannot create an unlabeled band.
    vec2 source_edge_epsilon = max(u_source_texel_size * 0.5, vec2(0.0001));
    camera_uv = clamp(camera_uv, source_edge_epsilon, vec2(1.0) - source_edge_epsilon);
    source_uv_valid =
        camera_uv.x >= 0.0 &&
        camera_uv.x <= 1.0 &&
        camera_uv.y >= 0.0 &&
        camera_uv.y <= 1.0;
}
if (!source_uv_valid) {
    out_color = source_invalid_color();
    return;
}
vec4 sample_color = u_processing_layer == 1
    ? blurred_camera_sample(camera_uv)
    : camera_sample(camera_uv);
if (u_processing_layer == 2 && u_peripheral_stretch_debug == 2 && stretch_effect_sample_region) {
    out_color = vec4(
        clamp(camera_uv, vec2(0.0), vec2(1.0)),
        target_transition_band ? 0.5 : 0.0,
        1.0
    );
    return;
}
if (u_processing_layer == 2 && u_peripheral_stretch_debug == 1 && target_transition_band) {
    out_color = premultiplied_alpha_color(vec3(1.0, 0.85, 0.0), 1.0);
    return;
}
if (u_processing_layer == 2 && u_peripheral_stretch_debug == 1 && stretch_exterior) {
    out_color = premultiplied_alpha_color(vec3(0.0, 1.0, 1.0), 1.0);
    return;
}
out_color = sample_color;
}"#;
