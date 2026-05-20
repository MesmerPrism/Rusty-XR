#version 450

#ifdef RUSTY_XR_SEPARATE_CAMERA_SAMPLER
layout(set = 0, binding = 0) uniform texture2D u_camera_left_tex;
layout(set = 0, binding = 1) uniform texture2D u_camera_right_tex;
layout(set = 0, binding = 3) uniform sampler u_camera_sampler;
#else
layout(set = 0, binding = 0) uniform sampler2D u_camera_left;
layout(set = 0, binding = 1) uniform sampler2D u_camera_right;
#endif
layout(set = 0, binding = 2, std140) uniform CameraProjectionSurfaceMap {
    vec4 left_screen_to_surface_h0;
    vec4 left_screen_to_surface_h1;
    vec4 left_screen_to_surface_h2;
    vec4 right_screen_to_surface_h0;
    vec4 right_screen_to_surface_h1;
    vec4 right_screen_to_surface_h2;
    vec4 left_surface_to_screen_h0;
    vec4 left_surface_to_screen_h1;
    vec4 left_surface_to_screen_h2;
    vec4 right_surface_to_screen_h0;
    vec4 right_surface_to_screen_h1;
    vec4 right_surface_to_screen_h2;
    vec4 color_matrix_r0;
    vec4 color_matrix_r1;
    vec4 color_matrix_r2;
    vec4 color_offset;
} surface_map;

layout(push_constant) uniform CameraProjectionPush {
    vec4 params;
    vec4 color_adjust;
    vec4 effect_params;
    vec4 area_params;
    vec4 area_offset_params;
    vec4 left_h0;
    vec4 left_h1;
    vec4 left_h2;
    vec4 right_h0;
    vec4 right_h1;
    vec4 right_h2;
} pc;

layout(location = 0) in vec2 v_surface_uv;
layout(location = 1) flat in int v_eye_index;
layout(location = 0) out vec4 out_color;

const float BORDER_INNER_COVERAGE = 0.30;
const float BORDER_OUTER_COVERAGE = 0.88;
const float BORDER_FEEDBACK_MIX = 0.62;
const float BORDER_PULLBACK = 0.16;
const float BORDER_SWIRL_STRENGTH = 0.18;
const float BORDER_ZOOM = 0.12;
const float BORDER_EDGE_BOOST = 0.50;
const float BORDER_FEATHER = 0.10;
const float BORDER_BRIGHTNESS_INSET = 0.16;
const float BORDER_BRIGHTNESS_CUTOFF = 0.25;
const float BORDER_BRIGHTNESS_FEATHER = 0.14;
const int CAMERA_FLAG_RAW_FEED = 8192;
const int CAMERA_FLAG_RAW_PROJECTION_FAST = 16384;
const int CAMERA_FLAG_PASSTHROUGH_UNDERLAY_ALPHA = 32768;
const int CAMERA_FLAG_RAW_PROJECTION_INVALID_FILL = 65536;
const int CAMERA_FLAG_RAW_PROJECTION_PERIMETER_FILL = 131072;
const int CAMERA_FLAG_RAW_PROJECTION_SOFT_BORDER = 262144;
const int CAMERA_FLAG_RAW_PROJECTION_STRONG_BORDER = 524288;
const int CAMERA_FLAG_RAW_PROJECTION_DYNAMIC_BORDER = 1048576;
const int CAMERA_FLAG_RAW_PROJECTION_WARM_BORDER = 2097152;
const int CAMERA_FLAG_RAW_PROJECTION_CYCLING_BORDER = 4194304;
const int CAMERA_FLAG_PROJECTION_AREA_DIAGNOSTIC = 8388608;
const int CAMERA_FLAG_FULL_FRAME_STIMULUS_MAPPING = 16777216;

vec3 clamp01(vec3 color) {
    return clamp(color, vec3(0.0), vec3(1.0));
}

float smooth_unit(float value) {
    value = clamp(value, 0.0, 1.0);
    return value * value * (3.0 - 2.0 * value);
}

float luma(vec3 color) {
    return clamp(dot(color, vec3(0.2126, 0.7152, 0.0722)), 0.0, 1.0);
}

vec3 decode_external_camera_sample(vec3 raw_sample) {
    float y = clamp((raw_sample.y - (16.0 / 255.0)) * (255.0 / 219.0), 0.0, 1.0);
    float cb = (raw_sample.z - (128.0 / 255.0)) * (255.0 / 224.0);
    float cr = (raw_sample.x - (128.0 / 255.0)) * (255.0 / 224.0);
    float r = y + 1.402 * cr;
    float g = y - 0.344136 * cb - 0.714136 * cr;
    float b = y + 1.772 * cb;
    return clamp01(vec3(r, g, b));
}

vec4 normalize_camera_sample(vec4 raw_sample) {
    int packed_flags = int(floor(pc.params.w + 0.5));
    if ((packed_flags & 4096) != 0) {
        return vec4(raw_sample.r, 0.0, 0.0, raw_sample.a);
    }
    if ((packed_flags & 2048) == 0) {
        return raw_sample;
    }
    return vec4(decode_external_camera_sample(raw_sample.rgb), raw_sample.a);
}

vec3 apply_camera_color_calibration(vec3 color) {
    vec3 calibrated = vec3(
        dot(surface_map.color_matrix_r0.xyz, color) + surface_map.color_offset.x,
        dot(surface_map.color_matrix_r1.xyz, color) + surface_map.color_offset.y,
        dot(surface_map.color_matrix_r2.xyz, color) + surface_map.color_offset.z
    );
    return clamp01(calibrated);
}

vec3 apply_camera_color_adjust(vec3 color) {
    float contrast = max(pc.color_adjust.x, 0.0);
    float brightness = pc.color_adjust.y;
    float saturation = max(pc.color_adjust.z, 0.0);
    vec3 contrasted = ((color - vec3(0.5)) * contrast) + vec3(0.5);
    float luminance = dot(contrasted, vec3(0.2126, 0.7152, 0.0722));
    vec3 saturated = mix(vec3(luminance), contrasted, saturation);
    return clamp01(saturated + vec3(brightness));
}

vec2 apply_camera_texture_transform(vec2 uv, int flags) {
    int turns = flags & 3;
    if (turns == 1) {
        uv = vec2(uv.y, 1.0 - uv.x);
    } else if (turns == 2) {
        uv = vec2(1.0 - uv.x, 1.0 - uv.y);
    } else if (turns == 3) {
        uv = vec2(1.0 - uv.y, uv.x);
    }
    if ((flags & 4) != 0 || (flags & 16) != 0) {
        uv.x = 1.0 - uv.x;
    }
    if ((flags & 8) != 0) {
        uv.y = 1.0 - uv.y;
    }
    return uv;
}

int source_eye_for_display_eye(int display_eye, int packed_flags) {
    bool swap = (packed_flags & 1024) != 0;
    return swap ? 1 - display_eye : display_eye;
}

int transform_flags_for_source_eye(int source_eye, int packed_flags) {
    return source_eye == 0
        ? (packed_flags & 31)
        : ((packed_flags >> 5) & 31);
}

vec2 apply_homography(vec2 uv, vec4 h0, vec4 h1, vec4 h2, out bool valid) {
    vec3 p = vec3(uv, 1.0);
    float w = dot(h2.xyz, p);
    valid = abs(w) > 0.00001;
    return vec2(dot(h0.xyz, p), dot(h1.xyz, p)) / (valid ? w : 1.0);
}

vec4 sample_source_eye_raw(int source_eye, vec2 uv) {
#ifdef RUSTY_XR_SEPARATE_CAMERA_SAMPLER
    vec4 raw_sample = source_eye == 0
        ? texture(sampler2D(u_camera_left_tex, u_camera_sampler), uv)
        : texture(sampler2D(u_camera_right_tex, u_camera_sampler), uv);
#else
    vec4 raw_sample = source_eye == 0
        ? texture(u_camera_left, uv)
        : texture(u_camera_right, uv);
#endif
    vec4 normalized = normalize_camera_sample(raw_sample);
    int packed_flags = int(floor(pc.params.w + 0.5));
    if ((packed_flags & CAMERA_FLAG_RAW_FEED) != 0) {
        return normalized;
    }
    vec3 calibrated = apply_camera_color_calibration(normalized.rgb);
    return vec4(apply_camera_color_adjust(calibrated), normalized.a);
}

vec4 sample_source_eye_clamped(int source_eye, vec2 uv) {
    return sample_source_eye_raw(source_eye, clamp(uv, vec2(0.0), vec2(1.0)));
}

vec4 sample_source_eye_oriented(int source_eye, vec2 uv, int transform_flags) {
    return sample_source_eye_clamped(
        source_eye,
        apply_camera_texture_transform(uv, transform_flags)
    );
}

vec2 surface_uv_from_content_uv(vec2 content_uv, float content_uv_scale) {
    return (content_uv - vec2(0.5)) / max(content_uv_scale, 1.0) + vec2(0.5);
}

vec2 content_uv_from_screen_uv(vec2 screen_uv, int display_eye, bool projected, out bool valid) {
    if (!projected) {
        valid = true;
        return screen_uv;
    }
    return display_eye == 0
        ? apply_homography(
            screen_uv,
            surface_map.left_screen_to_surface_h0,
            surface_map.left_screen_to_surface_h1,
            surface_map.left_screen_to_surface_h2,
            valid
        )
        : apply_homography(
            screen_uv,
            surface_map.right_screen_to_surface_h0,
            surface_map.right_screen_to_surface_h1,
            surface_map.right_screen_to_surface_h2,
            valid
        );
}

vec2 screen_uv_from_content_uv(
    vec2 content_uv,
    int display_eye,
    bool projected,
    float content_uv_scale,
    out bool valid
) {
    if (!projected) {
        valid = true;
        return surface_uv_from_content_uv(content_uv, content_uv_scale);
    }
    return display_eye == 0
        ? apply_homography(
            content_uv,
            surface_map.left_surface_to_screen_h0,
            surface_map.left_surface_to_screen_h1,
            surface_map.left_surface_to_screen_h2,
            valid
        )
        : apply_homography(
            content_uv,
            surface_map.right_surface_to_screen_h0,
            surface_map.right_surface_to_screen_h1,
            surface_map.right_surface_to_screen_h2,
            valid
        );
}

vec2 projected_camera_uv(
    vec2 projection_uv,
    int display_eye,
    int transform_flags,
    bool projected,
    out bool valid
) {
    bool homography_valid = true;
    vec2 uv = projection_uv;
    if (projected) {
        uv = display_eye == 0
            ? apply_homography(projection_uv, pc.left_h0, pc.left_h1, pc.left_h2, homography_valid)
            : apply_homography(projection_uv, pc.right_h0, pc.right_h1, pc.right_h2, homography_valid);
    }

    uv = apply_camera_texture_transform(uv, transform_flags);
    valid =
        homography_valid &&
        uv.x >= 0.0 &&
        uv.y >= 0.0 &&
        uv.x <= 1.0 &&
        uv.y <= 1.0;
    return uv;
}

vec4 sample_projected_content_source(
    vec2 content_uv,
    int display_eye,
    bool projected,
    float content_uv_scale,
    out bool valid,
    out int source_eye,
    out int transform_flags,
    out vec2 camera_uv
) {
    int packed_flags = int(floor(pc.params.w + 0.5));
    source_eye = source_eye_for_display_eye(display_eye, packed_flags);
    transform_flags = transform_flags_for_source_eye(source_eye, packed_flags);
    bool surface_valid = true;
    vec2 projection_uv = projected
        ? screen_uv_from_content_uv(
            content_uv,
            display_eye,
            projected,
            content_uv_scale,
            surface_valid
        )
        : clamp(content_uv, vec2(0.0), vec2(1.0));
    camera_uv = projected_camera_uv(
        projection_uv,
        display_eye,
        transform_flags,
        projected,
        valid
    );
    valid = valid && surface_valid;
    if (valid) {
        return sample_source_eye_raw(source_eye, camera_uv);
    }
    return sample_source_eye_oriented(
        source_eye,
        clamp(content_uv, vec2(0.0), vec2(1.0)),
        transform_flags
    );
}

float projection_coverage(vec2 projected_uv, bool valid, float edge_fade) {
    if (!valid) {
        return 0.0;
    }
    float source_edge_distance = min(
        min(projected_uv.x, 1.0 - projected_uv.x),
        min(projected_uv.y, 1.0 - projected_uv.y)
    );
    return smoothstep(0.0, max(edge_fade, 0.0001), source_edge_distance);
}

float resolve_fov_border_mix(float coverage) {
    return (1.0 - smoothstep(BORDER_INNER_COVERAGE, BORDER_OUTER_COVERAGE, coverage))
        * BORDER_FEEDBACK_MIX;
}

float resolve_camera_oval_distance(vec2 content_uv) {
    vec2 half_size = vec2(
        clamp(pc.area_params.x, 0.05, 0.50),
        clamp(pc.area_params.y, 0.05, 0.50)
    );
    float corner_radius = clamp(
        pc.area_params.z,
        0.0,
        min(half_size.x, half_size.y) - 0.001
    );
    vec2 q = abs(content_uv - vec2(0.5)) - (half_size - vec2(corner_radius));
    float outside = length(max(q, vec2(0.0)));
    float inside = min(max(q.x, q.y), 0.0);
    float signed_distance = outside + inside - corner_radius;
    return clamp(1.0 + signed_distance / max(min(half_size.x, half_size.y), 0.001), 0.0, 2.0);
}

float resolve_camera_oval_border_mix_from_distance(float oval_distance) {
    return smoothstep(1.0 - BORDER_FEATHER, 1.0 + BORDER_FEATHER, oval_distance);
}

float bleed_noise(vec2 content_uv) {
    float n0 = sin(dot(content_uv, vec2(21.17, 37.31)));
    float n1 = sin(dot(content_uv, vec2(-43.11, 18.53)));
    return clamp((n0 + n1) * 0.5, -1.0, 1.0);
}

float resolve_brightness_bleed_spatial_gate(float oval_distance, float noise) {
    float inner_distance = 1.0 - clamp(BORDER_BRIGHTNESS_INSET, 0.0, 0.45);
    float feather = clamp(BORDER_FEATHER * 0.25, 0.012, 0.025);
    float shifted_inner = inner_distance + noise * feather * 0.45;
    float gate_start = max(shifted_inner - feather, 0.0);
    float gate_end = max(min(shifted_inner + feather, 1.0), gate_start + 0.001);
    return smoothstep(gate_start, gate_end, oval_distance);
}

float resolve_brightness_cutoff_feedback_mix(
    float oval_distance,
    float oval_shape_mix,
    float guide_brightness,
    float raw_feedback_signal,
    float noise
) {
    float rim_mix = resolve_brightness_bleed_spatial_gate(oval_distance, noise)
        * (1.0 - oval_shape_mix);
    float inset = max(clamp(BORDER_BRIGHTNESS_INSET, 0.0, 0.45), 0.001);
    float inward = clamp((1.0 - oval_distance) / inset, 0.0, 1.0);
    float inward_ramp = smooth_unit(inward);
    float base_feather = max(BORDER_BRIGHTNESS_FEATHER, 0.001);
    float near_black_cutoff = min(BORDER_BRIGHTNESS_CUTOFF, 0.018);
    float effective_cutoff = mix(BORDER_BRIGHTNESS_CUTOFF, near_black_cutoff, inward_ramp);
    float feather = mix(base_feather, min(base_feather * 0.18, 0.018), inward_ramp);
    float cutoff = clamp(effective_cutoff + noise * feather * 0.35 * (1.0 - inward_ramp), 0.0, 1.0);
    float fade_start = max(cutoff - feather, 0.0);
    float fade_end = max(cutoff, fade_start + 0.001);
    float dark_output = 1.0 - smoothstep(fade_start, fade_end, clamp(guide_brightness, 0.0, 1.0));
    return clamp(raw_feedback_signal, 0.0, 1.0)
        * rim_mix
        * dark_output
        * BORDER_FEEDBACK_MIX;
}

vec2 camera_texel_size(int source_eye) {
#ifdef RUSTY_XR_SEPARATE_CAMERA_SAMPLER
    ivec2 size = source_eye == 0
        ? textureSize(sampler2D(u_camera_left_tex, u_camera_sampler), 0)
        : textureSize(sampler2D(u_camera_right_tex, u_camera_sampler), 0);
#else
    ivec2 size = source_eye == 0 ? textureSize(u_camera_left, 0) : textureSize(u_camera_right, 0);
#endif
    vec2 dims = vec2(float(max(size.x, 1)), float(max(size.y, 1)));
    return 1.0 / dims;
}

vec2 projection_area_content_uv(vec2 area_uv) {
    vec2 half_size = vec2(
        clamp(pc.area_params.x, 0.05, 0.50),
        clamp(pc.area_params.y, 0.05, 0.50)
    );
    return (area_uv - (vec2(0.5) - half_size)) / max(half_size * 2.0, vec2(0.001));
}

vec3 sample_source_eye_blur_raw(int source_eye, vec2 camera_uv, float radius_px) {
    float radius = max(radius_px, 0.0);
    if (radius <= 0.001) {
        return sample_source_eye_raw(source_eye, camera_uv).rgb;
    }
    vec2 texel = camera_texel_size(source_eye) * radius;
    vec2 uv = clamp(camera_uv, vec2(0.0), vec2(1.0));
    vec3 center = sample_source_eye_raw(source_eye, uv).rgb * 0.36;
    vec3 axis =
        sample_source_eye_raw(source_eye, clamp(uv + vec2(texel.x, 0.0), vec2(0.0), vec2(1.0))).rgb +
        sample_source_eye_raw(source_eye, clamp(uv - vec2(texel.x, 0.0), vec2(0.0), vec2(1.0))).rgb +
        sample_source_eye_raw(source_eye, clamp(uv + vec2(0.0, texel.y), vec2(0.0), vec2(1.0))).rgb +
        sample_source_eye_raw(source_eye, clamp(uv - vec2(0.0, texel.y), vec2(0.0), vec2(1.0))).rgb;
    vec3 diag =
        sample_source_eye_raw(source_eye, clamp(uv + texel, vec2(0.0), vec2(1.0))).rgb +
        sample_source_eye_raw(source_eye, clamp(uv - texel, vec2(0.0), vec2(1.0))).rgb +
        sample_source_eye_raw(source_eye, clamp(uv + vec2(texel.x, -texel.y), vec2(0.0), vec2(1.0))).rgb +
        sample_source_eye_raw(source_eye, clamp(uv + vec2(-texel.x, texel.y), vec2(0.0), vec2(1.0))).rgb;
    return clamp01(center + axis * 0.12 + diag * 0.04);
}

float source_luma(vec2 sample_uv, int source_eye, int transform_flags) {
    return luma(sample_source_eye_oriented(source_eye, sample_uv, transform_flags).rgb);
}

float projected_source_luma(
    vec2 content_uv,
    int display_eye,
    bool projected,
    float content_uv_scale
) {
    bool valid = false;
    int source_eye = 0;
    int transform_flags = 0;
    vec2 camera_uv = vec2(0.0);
    return luma(sample_projected_content_source(
        content_uv,
        display_eye,
        projected,
        content_uv_scale,
        valid,
        source_eye,
        transform_flags,
        camera_uv
    ).rgb);
}

float guide_luma(vec2 sample_uv, int source_eye, int transform_flags) {
    vec2 texel = camera_texel_size(source_eye);
    float center = source_luma(sample_uv, source_eye, transform_flags);
    float sides =
        source_luma(sample_uv + vec2(texel.x * 3.0, 0.0), source_eye, transform_flags) +
        source_luma(sample_uv - vec2(texel.x * 3.0, 0.0), source_eye, transform_flags) +
        source_luma(sample_uv + vec2(0.0, texel.y * 3.0), source_eye, transform_flags) +
        source_luma(sample_uv - vec2(0.0, texel.y * 3.0), source_eye, transform_flags);
    return clamp(center * 0.42 + sides * 0.145, 0.0, 1.0);
}

float projected_guide_luma(
    vec2 content_uv,
    int display_eye,
    bool projected,
    float content_uv_scale
) {
    vec2 step_size = vec2(1.0 / 2048.0);
    float center = projected_source_luma(content_uv, display_eye, projected, content_uv_scale);
    float sides =
        projected_source_luma(content_uv + vec2(step_size.x * 3.0, 0.0), display_eye, projected, content_uv_scale) +
        projected_source_luma(content_uv - vec2(step_size.x * 3.0, 0.0), display_eye, projected, content_uv_scale) +
        projected_source_luma(content_uv + vec2(0.0, step_size.y * 3.0), display_eye, projected, content_uv_scale) +
        projected_source_luma(content_uv - vec2(0.0, step_size.y * 3.0), display_eye, projected, content_uv_scale);
    return clamp(center * 0.42 + sides * 0.145, 0.0, 1.0);
}

float source_edge_strength(vec2 sample_uv, int source_eye, int transform_flags) {
    vec2 step_size = max(camera_texel_size(source_eye) * 2.0, vec2(1.0 / 2048.0));
    float left = source_luma(sample_uv - vec2(step_size.x, 0.0), source_eye, transform_flags);
    float right = source_luma(sample_uv + vec2(step_size.x, 0.0), source_eye, transform_flags);
    float up = source_luma(sample_uv - vec2(0.0, step_size.y), source_eye, transform_flags);
    float down = source_luma(sample_uv + vec2(0.0, step_size.y), source_eye, transform_flags);
    return smoothstep(0.025, 0.18, length(vec2(right - left, down - up)));
}

float projected_source_edge_strength(
    vec2 content_uv,
    int display_eye,
    bool projected,
    float content_uv_scale
) {
    vec2 step_size = vec2(2.0 / 2048.0);
    float left = projected_source_luma(content_uv - vec2(step_size.x, 0.0), display_eye, projected, content_uv_scale);
    float right = projected_source_luma(content_uv + vec2(step_size.x, 0.0), display_eye, projected, content_uv_scale);
    float up = projected_source_luma(content_uv - vec2(0.0, step_size.y), display_eye, projected, content_uv_scale);
    float down = projected_source_luma(content_uv + vec2(0.0, step_size.y), display_eye, projected, content_uv_scale);
    return smoothstep(0.025, 0.18, length(vec2(right - left, down - up)));
}

vec2 clamp_border_seed_uv(vec2 seed_uv) {
    vec2 center = vec2(0.5);
    vec2 radius = vec2(0.31, 0.28);
    vec2 p = (seed_uv - center) / radius;
    float len = max(length(p), 1.0);
    return center + (p / len) * radius;
}

vec3 brightness_gradient_color(float t) {
    float wrapped = fract(t);
    vec3 c0 = vec3(0.09, 0.03, 0.16);
    vec3 c1 = vec3(0.14, 0.32, 0.96);
    vec3 c2 = vec3(0.04, 0.92, 0.84);
    vec3 c3 = vec3(0.98, 0.88, 0.12);
    vec3 c4 = vec3(1.0, 0.36, 0.14);
    if (wrapped < 0.18) {
        return mix(c0, c1, wrapped / 0.18);
    }
    if (wrapped < 0.38) {
        return mix(c1, c2, (wrapped - 0.18) / 0.20);
    }
    if (wrapped < 0.62) {
        return mix(c2, c3, (wrapped - 0.38) / 0.24);
    }
    if (wrapped < 0.82) {
        return mix(c3, c4, (wrapped - 0.62) / 0.20);
    }
    return mix(c4, c0, (wrapped - 0.82) / 0.18);
}

float border_cycle_phase() {
    return fract(surface_map.color_offset.w);
}

vec3 spectral_border_cycle_color(float t) {
    vec3 ramp = abs(fract(vec3(t, t + 0.66, t + 0.33)) * 6.0 - 3.0);
    vec3 rgb = clamp(ramp - 1.0, vec3(0.0), vec3(1.0));
    float pulse = 0.5 + 0.5 * sin(fract(t) * 6.2831853);
    return clamp01(mix(vec3(0.055, 0.065, 0.080), rgb, 0.78 + pulse * 0.10));
}

vec3 resolve_fov_border_color(
    vec2 content_uv,
    int display_eye,
    int source_eye,
    int transform_flags,
    vec2 raw_projected_uv,
    float coverage,
    bool projected,
    float content_uv_scale,
    float guide_brightness,
    float raw_feedback_signal
) {
    vec2 center = vec2(0.5);
    vec2 clamped_uv = clamp(raw_projected_uv, vec2(0.002), vec2(0.998));
    vec2 outside_delta = raw_projected_uv - clamped_uv;
    float outside_distance = clamp(length(outside_delta * vec2(1.35, 1.0)) * 2.4, 0.0, 1.0);
    float border_amount = clamp(max(outside_distance, 1.0 - coverage), 0.0, 1.0);
    vec2 screen_delta = content_uv - center;
    float screen_radius = max(length(screen_delta), 0.0001);
    vec2 screen_dir = screen_delta / screen_radius;
    float seed_radius = clamp(
        screen_radius * (0.58 - border_amount * (0.10 + BORDER_PULLBACK * 0.34)),
        0.04,
        0.34
    );
    vec2 edge_seed = clamp_border_seed_uv(center + screen_dir * seed_radius);
    vec2 radial = edge_seed - center;
    float radial_length = max(length(radial), 0.0001);
    vec2 radial_dir = radial / radial_length;
    vec2 tangent = vec2(-radial_dir.y, radial_dir.x);
    float dark_signal = 1.0 - smoothstep(0.10, 0.55, guide_brightness);
    float geometry_signal = clamp(max(raw_feedback_signal, dark_signal * 0.72), 0.0, 1.0);
    float local_feedback = smoothstep(0.08, 0.72, geometry_signal);
    float organic = 0.5 + 0.5 * sin(dot(edge_seed, vec2(12.1, 7.7)) + border_amount * 1.70);
    float swirl_gain = BORDER_SWIRL_STRENGTH *
        (0.12 + local_feedback * 0.58 + border_amount * 0.30);
    float angle = (
        (border_amount - 0.5) * 1.4 +
        (organic - 0.5) * 0.8
    ) * swirl_gain;
    float zoom = 1.0 + BORDER_ZOOM * border_amount * (0.70 + local_feedback * 0.65);
    vec2 lateral = tangent * (organic - 0.5) * 0.12 * border_amount * (0.25 + local_feedback);
    vec2 inward = radial_dir * border_amount * (0.045 + BORDER_PULLBACK * 0.070) * (0.35 + local_feedback);
    vec2 mirrored_seed = clamp_border_seed_uv(
        center - radial_dir * seed_radius * (0.45 + border_amount * 0.30) + lateral
    );
    vec2 trail_anchor = center + ((mirrored_seed - inward) - center) / zoom;
    vec2 trail_a_uv = clamp_border_seed_uv(trail_anchor + tangent * angle * 0.040);

    bool trail_valid = false;
    int trail_source_eye = source_eye;
    int trail_transform_flags = transform_flags;
    vec2 trail_camera_uv = vec2(0.0);
    vec3 base = sample_projected_content_source(
        trail_a_uv,
        display_eye,
        projected,
        content_uv_scale,
        trail_valid,
        trail_source_eye,
        trail_transform_flags,
        trail_camera_uv
    ).rgb;
    float base_luma = luma(base);
    float guide_mix_luma = mix(guide_brightness, base_luma, 0.35);
    vec3 gradient = brightness_gradient_color(
        guide_mix_luma * 0.56 +
        border_amount * 0.12 +
        organic * 0.04
    );
    float swirl_wave = clamp(0.5 + (organic - 0.5) * (0.65 + local_feedback * 0.35), 0.0, 1.0);
    vec3 tint_shuffle = vec3(base.z, base.x, base.y);
    vec3 tinted_base = mix(base, tint_shuffle, 0.10 + border_amount * 0.08);
    float synthetic_mix = clamp(0.24 + local_feedback * 0.24 + border_amount * 0.22, 0.0, 0.70);
    vec3 synthetic_trail = clamp01(
        mix(tinted_base, gradient, synthetic_mix) +
        vec3((organic - 0.5) * 0.09 + (swirl_wave - 0.5) * geometry_signal * 0.14)
    );
    vec3 feedback = mix(base, synthetic_trail, 0.30 + local_feedback * 0.34 + border_amount * 0.18);
    float trail_luma = max(base_luma, luma(synthetic_trail));
    float source_valid = smoothstep(0.025, 0.10, trail_luma);
    vec3 fallback_color = mix(base, gradient, 0.28 + border_amount * 0.22);
    vec3 feedback_color = mix(fallback_color, feedback, source_valid);
    float tint_amount = clamp(
        geometry_signal * BORDER_EDGE_BOOST * (0.28 + border_amount * 0.72),
        0.0,
        0.70
    );
    vec3 edge_tint = mix(feedback_color, gradient, tint_amount);
    return clamp01(mix(feedback_color, edge_tint, 0.58 + local_feedback * 0.26));
}

float resolve_fov_border_composite_mix(
    vec2 content_uv,
    float guide_brightness,
    float raw_feedback_signal
) {
    float oval_distance = resolve_camera_oval_distance(content_uv);
    float noise = bleed_noise(content_uv);
    float spatial_gate = resolve_brightness_bleed_spatial_gate(oval_distance, noise);
    if (spatial_gate <= 0.0001) {
        return 0.0;
    }
    float feather = max(BORDER_BRIGHTNESS_FEATHER, 0.001);
    float cutoff = clamp(BORDER_BRIGHTNESS_CUTOFF + noise * feather * 0.45, 0.0, 1.0);
    float fade_start = max(cutoff - feather, 0.0);
    float fade_end = max(cutoff, fade_start + 0.001);
    float dark_push = 1.0 - smoothstep(fade_start, fade_end, clamp(guide_brightness, 0.0, 1.0));
    float organic_push = 0.5 + 0.5 * sin(dot(content_uv, vec2(23.7, -17.9)));
    float pushed_oval_distance = oval_distance +
        dark_push * spatial_gate * (0.09 + raw_feedback_signal * 0.15 + organic_push * 0.10);
    float oval_shape_mix = resolve_camera_oval_border_mix_from_distance(pushed_oval_distance);
    float oval_border_mix = oval_shape_mix * BORDER_FEEDBACK_MIX * spatial_gate;
    float brightness_border_mix = resolve_brightness_cutoff_feedback_mix(
        oval_distance,
        oval_shape_mix,
        guide_brightness,
        raw_feedback_signal,
        noise
    );
    return max(oval_border_mix, brightness_border_mix);
}

vec3 resolve_fov_border_composite(
    vec2 content_uv,
    int display_eye,
    int source_eye,
    int transform_flags,
    vec3 center_color,
    vec2 raw_projected_uv,
    float coverage,
    bool projection_valid,
    float edge_fade,
    bool projected,
    float content_uv_scale
) {
    vec2 center = vec2(0.5);
    vec2 screen_delta = content_uv - center;
    float screen_radius = max(length(screen_delta), 0.0001);
    float oval_distance = resolve_camera_oval_distance(content_uv);
    float noise = bleed_noise(content_uv);
    float spatial_gate = resolve_brightness_bleed_spatial_gate(oval_distance, noise);
    float oval_shape_mix = resolve_camera_oval_border_mix_from_distance(oval_distance);
    float projection_gap_mix = (1.0 - coverage) * oval_shape_mix * spatial_gate;
    float coverage_mix = resolve_fov_border_mix(coverage) * (1.0 - smoothstep(0.0, max(edge_fade, 0.0001), coverage));
    float invalid_boost = projection_valid ? 0.0 : projection_gap_mix;
    float cheap_border_hint = max(projection_gap_mix, max(coverage_mix, invalid_boost));
    if (spatial_gate <= 0.0001 && cheap_border_hint <= 0.0001) {
        return center_color;
    }
    vec2 guide_uv = clamp_border_seed_uv(
        center + (screen_delta / screen_radius) * clamp(screen_radius * 0.50, 0.04, 0.36)
    );
    float guide_brightness = projected_guide_luma(guide_uv, display_eye, projected, content_uv_scale);
    float guide_feedback_signal = max(
        projected_source_edge_strength(guide_uv, display_eye, projected, content_uv_scale),
        1.0 - smoothstep(0.10, 0.55, guide_brightness)
    );
    float raw_border_brightness = luma(center_color);
    float smoothed_border_brightness = projected_guide_luma(
        content_uv,
        display_eye,
        projected,
        content_uv_scale
    );
    float border_brightness = mix(smoothed_border_brightness, raw_border_brightness, 0.20);
    float border_feedback_signal = 1.0 - smoothstep(0.10, 0.55, border_brightness);
    float border_mix = resolve_fov_border_composite_mix(
        content_uv,
        border_brightness,
        border_feedback_signal
    );
    float resolved_border_mix = max(max(border_mix, projection_gap_mix), max(coverage_mix, invalid_boost));
    if (resolved_border_mix <= 0.0001) {
        return center_color;
    }
    vec3 border_color = resolve_fov_border_color(
        content_uv,
        display_eye,
        source_eye,
        transform_flags,
        raw_projected_uv,
        min(coverage, 1.0 - resolved_border_mix),
        projected,
        content_uv_scale,
        guide_brightness,
        guide_feedback_signal
    );
    return mix(center_color, border_color, clamp(resolved_border_mix, 0.0, pc.color_adjust.a));
}

vec3 resolve_raw_projection_invalid_fill(
    vec3 center_color,
    vec3 undimmed_fallback_color,
    bool projection_valid
) {
    return projection_valid ? center_color : undimmed_fallback_color;
}

vec3 resolve_raw_projection_perimeter_fill(
    vec2 content_uv,
    int source_eye,
    int transform_flags,
    vec3 center_color,
    vec3 undimmed_fallback_color,
    bool projection_valid
) {
    vec2 center = vec2(0.5);
    vec2 delta = content_uv - center;
    float radius = max(length(delta), 0.0001);
    float oval_distance = resolve_camera_oval_distance(content_uv);
    float rim_mix = resolve_camera_oval_border_mix_from_distance(oval_distance);
    float invalid_mix = projection_valid ? 0.0 : 1.0;
    float fill_mix = max(rim_mix, invalid_mix);
    if (fill_mix <= 0.0001) {
        return center_color;
    }
    vec2 seed_uv = clamp_border_seed_uv(
        center + (delta / radius) * clamp(radius, 0.06, 0.48)
    );
    vec3 rim_color = sample_source_eye_oriented(source_eye, seed_uv, transform_flags).rgb;
    vec3 fill_color = projection_valid ? rim_color : undimmed_fallback_color;
    return mix(center_color, fill_color, clamp(fill_mix * 0.85, 0.0, 1.0));
}

vec3 resolve_raw_projection_soft_border(
    vec2 content_uv,
    int display_eye,
    int source_eye,
    int transform_flags,
    vec3 center_color,
    vec3 undimmed_fallback_color,
    vec2 raw_projected_uv,
    float coverage,
    bool projection_valid,
    float edge_fade,
    bool projected,
    float content_uv_scale,
    float mix_scale,
    float seed_radius_base,
    float seed_pullback,
    float color_lift,
    float dynamic_color_strength,
    float dynamic_color_warmth,
    float cycle_color_strength
) {
    vec2 center = vec2(0.5);
    vec2 screen_delta = content_uv - center;
    float screen_radius = max(length(screen_delta), 0.0001);
    float oval_distance = resolve_camera_oval_distance(content_uv);
    float noise = bleed_noise(content_uv);
    float spatial_gate = resolve_brightness_bleed_spatial_gate(oval_distance, noise);
    float oval_shape_mix = resolve_camera_oval_border_mix_from_distance(oval_distance);
    float projection_gap_mix = (1.0 - coverage) * oval_shape_mix * spatial_gate;
    float coverage_mix = resolve_fov_border_mix(coverage) *
        (1.0 - smoothstep(0.0, max(edge_fade, 0.0001), coverage));
    float invalid_boost = projection_valid ? 0.0 : projection_gap_mix;
    float cheap_border_hint = max(projection_gap_mix, max(coverage_mix, invalid_boost));
    if (spatial_gate <= 0.0001 && cheap_border_hint <= 0.0001) {
        return center_color;
    }

    float border_brightness = luma(center_color);
    float border_feedback_signal = 1.0 - smoothstep(0.10, 0.55, border_brightness);
    float border_mix = resolve_fov_border_composite_mix(
        content_uv,
        border_brightness,
        border_feedback_signal
    );
    float resolved_border_mix = max(max(border_mix, projection_gap_mix), max(coverage_mix, invalid_boost));
    if (resolved_border_mix <= 0.0001) {
        return center_color;
    }

    vec2 screen_dir = screen_delta / screen_radius;
    float seed_radius = clamp(
        screen_radius * (seed_radius_base - resolved_border_mix * seed_pullback),
        0.04,
        0.36
    );
    vec2 seed_uv = clamp_border_seed_uv(center + screen_dir * seed_radius);
    bool seed_valid = false;
    int seed_source_eye = source_eye;
    int seed_transform_flags = transform_flags;
    vec2 seed_camera_uv = raw_projected_uv;
    vec3 fill_color = sample_projected_content_source(
        seed_uv,
        display_eye,
        projected,
        content_uv_scale,
        seed_valid,
        seed_source_eye,
        seed_transform_flags,
        seed_camera_uv
    ).rgb;
    if (!seed_valid) {
        fill_color = undimmed_fallback_color;
    }
    if (color_lift > 0.0) {
        float fill_luma = luma(fill_color);
        fill_color = clamp01(mix(vec3(fill_luma), fill_color, 1.0 + color_lift * 2.0));
        fill_color = clamp01(fill_color + vec3(color_lift * resolved_border_mix));
    }
    float cycle_strength = clamp(cycle_color_strength, 0.0, 1.5);
    if (dynamic_color_strength > 0.0 || cycle_strength > 0.0) {
        float fill_luma = luma(fill_color);
        float center_luma = luma(center_color);
        float cycle_phase = border_cycle_phase();
        float organic = 0.5 + 0.5 * sin(
            dot(seed_uv, vec2(12.1, 7.7)) +
            resolved_border_mix * 1.70 +
            cycle_phase * 6.2831853 * cycle_strength
        );
        float dark_feedback = 1.0 - smoothstep(0.12, 0.60, center_luma);
        float feedback = clamp(resolved_border_mix * (0.45 + dark_feedback * 0.55), 0.0, 1.0);
        vec3 dynamic_color;
        if (cycle_strength > 0.0) {
            float cycle_feedback = clamp(
                feedback * (0.55 + border_feedback_signal * 0.30 + resolved_border_mix * 0.20),
                0.0,
                1.0
            );
            vec3 gradient = spectral_border_cycle_color(
                cycle_phase +
                fill_luma * 0.48 +
                resolved_border_mix * 0.13 +
                organic * 0.05
            );
            vec3 channel_echo = vec3(fill_color.z, fill_color.x, fill_color.y);
            vec3 chroma_echo = mix(fill_color, channel_echo, 0.07 + cycle_feedback * 0.08);
            dynamic_color = clamp01(mix(
                chroma_echo,
                gradient,
                clamp(0.18 + cycle_feedback * 0.30 + resolved_border_mix * 0.08, 0.0, 0.58)
            ));
            float cycle_luma = max(luma(dynamic_color), 0.0001);
            dynamic_color = mix(
                dynamic_color,
                clamp01(dynamic_color * (max(fill_luma, 0.06) / cycle_luma)),
                0.34
            );
        } else if (dynamic_color_warmth > 0.0) {
            float warm_feedback = clamp(feedback * dynamic_color_warmth, 0.0, 1.0);
            vec3 warm_shift = vec3(
                fill_color.r + (1.0 - fill_color.r) * (0.08 + organic * 0.05) * warm_feedback,
                fill_color.g * (1.0 - 0.04 * warm_feedback),
                fill_color.b * (1.0 - 0.12 * warm_feedback)
            );
            float warm_luma = max(luma(warm_shift), 0.0001);
            vec3 luma_matched = clamp01(warm_shift * (fill_luma / warm_luma));
            float chroma_boost = 1.0 + warm_feedback * 0.18;
            dynamic_color = clamp01(mix(vec3(luma(luma_matched)), luma_matched, chroma_boost));
        } else {
            vec3 gradient = brightness_gradient_color(
                fill_luma * 0.52 +
                resolved_border_mix * 0.14 +
                organic * 0.05
            );
            vec3 channel_echo = vec3(fill_color.z, fill_color.x, fill_color.y);
            vec3 chroma_echo = mix(fill_color, channel_echo, 0.06 + feedback * 0.08);
            dynamic_color = clamp01(mix(
                chroma_echo,
                gradient,
                clamp(0.10 + feedback * 0.22 + resolved_border_mix * 0.08, 0.0, 0.42)
            ));
        }
        float color_mix_strength = max(dynamic_color_strength, cycle_strength);
        fill_color = mix(fill_color, dynamic_color, clamp(color_mix_strength * feedback, 0.0, 1.0));
    }
    float mix_amount = clamp(resolved_border_mix * mix_scale, 0.0, 1.0);
    return mix(center_color, fill_color, mix_amount);
}

float diagnostic_domain_edge_mask(vec2 uv, float width, float pad) {
    float near_domain =
        step(-pad, uv.x) *
        step(uv.x, 1.0 + pad) *
        step(-pad, uv.y) *
        step(uv.y, 1.0 + pad);
    vec2 edge_distance = min(abs(uv), abs(vec2(1.0) - uv));
    return (1.0 - step(width, min(edge_distance.x, edge_distance.y))) * near_domain;
}

float diagnostic_axis_mask(vec2 uv, float axis, float width) {
    return max(
        1.0 - step(width, abs(uv.x - axis)),
        1.0 - step(width, abs(uv.y - axis))
    );
}

vec3 resolve_projection_area_diagnostic(
    vec2 content_uv,
    vec2 raw_projected_uv,
    bool projection_valid,
    bool content_surface_valid,
    int display_eye
) {
    vec2 diagnostic_uv = clamp(raw_projected_uv, vec2(0.0), vec2(1.0));
    float valid = projection_valid && content_surface_valid ? 1.0 : 0.0;
    float border = diagnostic_domain_edge_mask(raw_projected_uv, 0.018, 0.060);
    float major_axes = diagnostic_axis_mask(diagnostic_uv, 0.5, 0.010);
    float quarter_axes = max(
        diagnostic_axis_mask(diagnostic_uv, 0.25, 0.006),
        diagnostic_axis_mask(diagnostic_uv, 0.75, 0.006)
    );
    float diagonal =
        1.0 - step(0.010, abs((diagnostic_uv.x - diagnostic_uv.y))) +
        1.0 - step(0.010, abs((diagnostic_uv.x + diagnostic_uv.y) - 1.0));
    diagonal = clamp(diagonal, 0.0, 1.0);

    vec3 left_color = vec3(0.02, 0.25, 0.98);
    vec3 right_color = vec3(0.95, 0.08, 0.58);
    vec3 base = mix(left_color, right_color, float(display_eye));
    vec3 ramp = vec3(
        0.18 + diagnostic_uv.x * 0.62,
        0.12 + diagnostic_uv.y * 0.76,
        0.90 - diagnostic_uv.x * 0.22
    );
    vec3 color = mix(base, ramp, 0.42);
    color = mix(color, vec3(1.0), clamp(major_axes * 0.82, 0.0, 1.0));
    color = mix(color, vec3(0.05, 1.0, 0.72), clamp(quarter_axes * 0.52, 0.0, 1.0));
    color = mix(color, vec3(1.0, 0.86, 0.04), clamp(diagonal * 0.44, 0.0, 1.0));
    color = mix(vec3(0.0), color, valid);
    color = mix(color, vec3(0.0, 1.0, 1.0), clamp(border, 0.0, 1.0));
    float surface_edge = diagnostic_domain_edge_mask(content_uv, 0.010, 0.035);
    color = mix(color, vec3(1.0, 1.0, 1.0), clamp(surface_edge * valid * 0.70, 0.0, 1.0));
    return clamp01(color);
}

void main() {
    int eye = clamp(v_eye_index, 0, 1);
    int packed_flags = int(floor(pc.params.w + 0.5));
    int source_eye = source_eye_for_display_eye(eye, packed_flags);
    int transform_flags = transform_flags_for_source_eye(source_eye, packed_flags);
    bool projected = pc.params.x < 0.0;
    float overscan = max(abs(pc.params.x), 1.0);
    float edge_fade = clamp(pc.params.y, 0.0, 0.5);
    float content_uv_scale = max(pc.params.z, 1.0);
    float projection_area_opacity = clamp(pc.effect_params.y, 0.0, 1.0);
    float projection_border_opacity = clamp(pc.effect_params.z, 0.0, 1.0);
    vec2 projection_area_offset = clamp(
        eye == 0 ? pc.area_offset_params.xy : pc.area_offset_params.zw,
        vec2(-0.5),
        vec2(0.5)
    );
    float projection_area_scale = clamp(pc.area_params.w, 0.05, 4.0);
    bool full_frame_stimulus_mapping =
        (packed_flags & CAMERA_FLAG_FULL_FRAME_STIMULUS_MAPPING) != 0;
    vec2 projection_screen_uv_base =
        (v_surface_uv - vec2(0.5)) * projection_area_scale + vec2(0.5);
    vec2 projection_screen_uv = full_frame_stimulus_mapping
        ? projection_screen_uv_base - projection_area_offset
        : projection_screen_uv_base;
    vec2 projection_area_domain_uv = projection_screen_uv_base - projection_area_offset;

    vec2 local_uv = vec2(0.5) + ((v_surface_uv - vec2(0.5)) / overscan);
    bool content_surface_valid = true;
    vec2 projected_content_uv = content_uv_from_screen_uv(
        projection_screen_uv,
        eye,
        projected,
        content_surface_valid
    );
    vec2 content_uv = projected
        ? projected_content_uv
        : (v_surface_uv - vec2(0.5)) * content_uv_scale + vec2(0.5);
    vec2 full_frame_content_uv = projection_area_content_uv(projection_screen_uv);
    vec2 sample_content_uv = full_frame_stimulus_mapping
        ? full_frame_content_uv
        : (projected ? content_uv : clamp(local_uv, vec2(0.0), vec2(1.0)));
    vec2 projection_uv = full_frame_stimulus_mapping
        ? full_frame_content_uv
        : (projected ? projection_screen_uv : sample_content_uv);

    bool projection_valid = false;
    vec2 raw_projected_uv = projected_camera_uv(
        projection_uv,
        eye,
        transform_flags,
        projected && !full_frame_stimulus_mapping,
        projection_valid
    );
    projection_valid =
        projection_valid && (full_frame_stimulus_mapping || content_surface_valid);
    float coverage = projection_coverage(raw_projected_uv, projection_valid, max(edge_fade, 0.012));
    if ((packed_flags & CAMERA_FLAG_PROJECTION_AREA_DIAGNOSTIC) != 0) {
        out_color = vec4(resolve_projection_area_diagnostic(
            projection_area_domain_uv,
            raw_projected_uv,
            projection_valid,
            content_surface_valid,
            eye
        ), 1.0);
        return;
    }

    vec3 raw_center_color = projection_valid
        ? sample_source_eye_raw(source_eye, raw_projected_uv).rgb
        : sample_source_eye_oriented(
            source_eye,
            clamp_border_seed_uv(clamp(sample_content_uv, vec2(0.0), vec2(1.0))),
            transform_flags
        ).rgb;
    vec3 center_color = raw_center_color;
    if (!projection_valid && projected) {
        center_color *= 0.12;
    }

#ifdef RUSTY_XR_CAMERA_PROJECTION_FAST_ONLY
    float fast_surface_edge_distance = min(
        min(v_surface_uv.x, 1.0 - v_surface_uv.x),
        min(v_surface_uv.y, 1.0 - v_surface_uv.y)
    );
    float fast_surface_edge_dim = edge_fade > 0.0
        ? mix(0.90, 1.0, smoothstep(0.0, edge_fade, fast_surface_edge_distance))
        : 1.0;
    float fast_source_edge_dim = mix(0.94, 1.0, coverage);
    out_color = vec4(clamp01(center_color * fast_surface_edge_dim * fast_source_edge_dim), 1.0);
    return;
#endif

    bool raw_projection_fast = (packed_flags & CAMERA_FLAG_RAW_PROJECTION_FAST) != 0;
    bool raw_projection_invalid_fill = (packed_flags & CAMERA_FLAG_RAW_PROJECTION_INVALID_FILL) != 0;
    bool raw_projection_perimeter_fill = (packed_flags & CAMERA_FLAG_RAW_PROJECTION_PERIMETER_FILL) != 0;
    bool raw_projection_soft_border = (packed_flags & CAMERA_FLAG_RAW_PROJECTION_SOFT_BORDER) != 0;
    bool raw_projection_strong_border = (packed_flags & CAMERA_FLAG_RAW_PROJECTION_STRONG_BORDER) != 0;
    bool raw_projection_dynamic_border = (packed_flags & CAMERA_FLAG_RAW_PROJECTION_DYNAMIC_BORDER) != 0;
    bool raw_projection_warm_border = (packed_flags & CAMERA_FLAG_RAW_PROJECTION_WARM_BORDER) != 0;
    bool raw_projection_cycling_border = (packed_flags & CAMERA_FLAG_RAW_PROJECTION_CYCLING_BORDER) != 0;
    bool passthrough_underlay_alpha = (packed_flags & CAMERA_FLAG_PASSTHROUGH_UNDERLAY_ALPHA) != 0;
    bool raw_projection_solid_red = raw_projection_invalid_fill && raw_projection_perimeter_fill;
    bool raw_projection_blur = raw_projection_soft_border && raw_projection_strong_border;
    bool raw_projection_area_mask = raw_projection_solid_red || passthrough_underlay_alpha;
    float projection_area_distance = resolve_camera_oval_distance(projection_area_domain_uv);
    bool projection_area_inside = projection_area_distance <= 1.0;
    bool masked_projection_valid = projection_valid && (!raw_projection_area_mask || projection_area_inside);
    vec3 diagnostic_intended_mask_color = vec3(0.36, 0.0, 0.28);
    vec3 diagnostic_source_invalid_color = vec3(1.0, 0.0, 0.0);
    vec3 diagnostic_guide_color = eye == 0 ? vec3(0.0, 0.95, 1.0) : vec3(1.0, 0.86, 0.0);
    bool diagnostic_intended_mask = raw_projection_area_mask && !projection_area_inside;
    bool diagnostic_source_invalid = raw_projection_area_mask && projection_area_inside && !projection_valid;
    vec3 raw_projection_diagnostic_color = diagnostic_intended_mask
        ? diagnostic_intended_mask_color
        : diagnostic_source_invalid_color;
    float projection_area_guide = raw_projection_area_mask
        ? 1.0 - smoothstep(0.0, 0.018, abs(projection_area_distance - 1.0))
        : 0.0;
    vec3 color = center_color;
    if (raw_projection_blur) {
        color = masked_projection_valid
            ? sample_source_eye_blur_raw(source_eye, raw_projected_uv, pc.effect_params.x)
            : (raw_projection_solid_red ? raw_projection_diagnostic_color : center_color);
    } else if (raw_projection_solid_red) {
        color = masked_projection_valid ? center_color : raw_projection_diagnostic_color;
    } else if (raw_projection_cycling_border) {
        color = resolve_raw_projection_soft_border(
            sample_content_uv,
            eye,
            source_eye,
            transform_flags,
            center_color,
            raw_center_color,
            raw_projected_uv,
            coverage,
            projection_valid,
            edge_fade,
            projected,
            content_uv_scale,
            1.28,
            0.43,
            0.18,
            0.035,
            0.64,
            0.0,
            1.0
        );
    } else if (raw_projection_warm_border) {
        color = resolve_raw_projection_soft_border(
            sample_content_uv,
            eye,
            source_eye,
            transform_flags,
            center_color,
            raw_center_color,
            raw_projected_uv,
            coverage,
            projection_valid,
            edge_fade,
            projected,
            content_uv_scale,
            1.28,
            0.43,
            0.18,
            0.035,
            0.58,
            1.0,
            0.0
        );
    } else if (raw_projection_dynamic_border) {
        color = resolve_raw_projection_soft_border(
            sample_content_uv,
            eye,
            source_eye,
            transform_flags,
            center_color,
            raw_center_color,
            raw_projected_uv,
            coverage,
            projection_valid,
            edge_fade,
            projected,
            content_uv_scale,
            1.28,
            0.43,
            0.18,
            0.020,
            0.62,
            0.0,
            0.0
        );
    } else if (raw_projection_strong_border) {
        color = resolve_raw_projection_soft_border(
            sample_content_uv,
            eye,
            source_eye,
            transform_flags,
            center_color,
            raw_center_color,
            raw_projected_uv,
            coverage,
            projection_valid,
            edge_fade,
            projected,
            content_uv_scale,
            1.28,
            0.43,
            0.18,
            0.035,
            0.0,
            0.0,
            0.0
        );
    } else if (raw_projection_soft_border) {
        color = resolve_raw_projection_soft_border(
            sample_content_uv,
            eye,
            source_eye,
            transform_flags,
            center_color,
            raw_center_color,
            raw_projected_uv,
            coverage,
            projection_valid,
            edge_fade,
            projected,
            content_uv_scale,
            0.78,
            0.52,
            0.10,
            0.0,
            0.0,
            0.0,
            0.0
        );
    } else if (raw_projection_perimeter_fill) {
        color = resolve_raw_projection_perimeter_fill(
            sample_content_uv,
            source_eye,
            transform_flags,
            center_color,
            raw_center_color,
            projection_valid
        );
    } else if (raw_projection_invalid_fill) {
        color = resolve_raw_projection_invalid_fill(
            center_color,
            raw_center_color,
            projection_valid
        );
    } else if (raw_projection_fast) {
        color = center_color;
    } else {
        color = resolve_fov_border_composite(
            sample_content_uv,
            eye,
            source_eye,
            transform_flags,
            center_color,
            raw_projected_uv,
            coverage,
            projection_valid,
            edge_fade,
            projected,
            content_uv_scale
        );
    }

    float surface_edge_distance = min(
        min(v_surface_uv.x, 1.0 - v_surface_uv.x),
        min(v_surface_uv.y, 1.0 - v_surface_uv.y)
    );
    float surface_edge_dim = edge_fade > 0.0
        ? mix(0.90, 1.0, smoothstep(0.0, edge_fade, surface_edge_distance))
        : 1.0;
    float source_edge_dim = mix(0.94, 1.0, coverage);
    float out_alpha = 1.0;
    if (raw_projection_area_mask) {
        out_alpha = masked_projection_valid
            ? projection_area_opacity
            : (passthrough_underlay_alpha ? 0.0 : projection_border_opacity);
    }
    vec3 final_color = color * surface_edge_dim * source_edge_dim;
    if (raw_projection_solid_red && !masked_projection_valid) {
        final_color = raw_projection_diagnostic_color;
    }
    if (raw_projection_solid_red) {
        final_color = mix(
            final_color,
            diagnostic_guide_color,
            clamp(projection_area_guide * projection_border_opacity, 0.0, 1.0)
        );
    }
    out_color = vec4(clamp01(final_color), out_alpha);
}
