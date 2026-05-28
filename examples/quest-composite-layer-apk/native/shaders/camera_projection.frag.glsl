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
    vec4 left_source_uv_rect;
    vec4 right_source_uv_rect;
    vec4 left_canvas_clip0;
    vec4 left_canvas_clip1;
    vec4 left_canvas_clip2;
    vec4 left_canvas_clip3;
    vec4 right_canvas_clip0;
    vec4 right_canvas_clip1;
    vec4 right_canvas_clip2;
    vec4 right_canvas_clip3;
} surface_map;

layout(push_constant) uniform CameraProjectionPush {
    vec4 params;
    vec4 color_adjust;
    vec4 effect_params;
    vec4 stretch_params;
    vec4 stretch_blend_params;
    vec4 alpha_params;
    vec4 area_params;
    vec4 area_offset_params;
    vec4 area_radius_params;
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
const int CAMERA_FLAG_RAW_PROJECTION = 16384;
const int CAMERA_FLAG_PASSTHROUGH_UNDERLAY_ALPHA = 32768;
const int CAMERA_FLAG_PROJECTION_BORDER_SOLID_RED = 65536;
const int CAMERA_FLAG_PROJECTION_AREA_DIAGNOSTIC = 8388608;
const int CAMERA_FLAG_TARGET_LOCAL_RASTER_SAMPLING = 16777216;
const int CAMERA_FLAG_TARGET_FOOTPRINT_FROM_METADATA = 33554432;
const int CAMERA_EFFECT_RAW_PROJECTION_BLUR = 5;
const int CAMERA_EFFECT_PERIPHERAL_STRETCH = 6;
const vec2 CAMERA_DIAGNOSTIC_BLUR_SOURCE_SIZE_PX = vec2(1280.0, 1280.0);
const float CAMERA_DIAGNOSTIC_BLUR_SAMPLE_STEP_GAIN = 4.0;

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

vec2 apply_source_uv_rect(vec2 uv, int source_eye) {
    vec4 rect = source_eye == 0
        ? surface_map.left_source_uv_rect
        : surface_map.right_source_uv_rect;
    vec2 scale = max(rect.zw, vec2(0.0));
    return rect.xy + uv * scale;
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
    int source_eye,
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

    bool source_content_valid =
        uv.x >= 0.0 &&
        uv.y >= 0.0 &&
        uv.x <= 1.0 &&
        uv.y <= 1.0;
    uv = apply_source_uv_rect(uv, source_eye);
    uv = apply_camera_texture_transform(uv, transform_flags);
    valid =
        homography_valid &&
        source_content_valid &&
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
        source_eye,
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

vec2 projection_area_half_size(int eye) {
    vec2 per_eye_radius = eye == 0 ? pc.area_radius_params.xy : pc.area_radius_params.zw;
    vec2 fallback_radius = pc.area_params.xy;
    vec2 radius = vec2(
        per_eye_radius.x > 0.001 ? per_eye_radius.x : fallback_radius.x,
        per_eye_radius.y > 0.001 ? per_eye_radius.y : fallback_radius.y
    );
    return vec2(
        clamp(radius.x, 0.05, 0.50),
        clamp(radius.y, 0.05, 0.50)
    );
}

float target_footprint_signed_distance_uv(vec2 content_uv, int eye) {
    vec2 half_size = projection_area_half_size(eye);
    float corner_radius = clamp(
        pc.area_params.z,
        0.0,
        min(half_size.x, half_size.y) - 0.001
    );
    vec2 q = abs(content_uv - vec2(0.5)) - (half_size - vec2(corner_radius));
    float outside = length(max(q, vec2(0.0)));
    float inside = min(max(q.x, q.y), 0.0);
    return outside + inside - corner_radius;
}

float resolve_camera_oval_distance(vec2 content_uv, int eye) {
    float signed_distance = target_footprint_signed_distance_uv(content_uv, eye);
    vec2 half_size = projection_area_half_size(eye);
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

vec2 diagnostic_blur_texel_size() {
    return 1.0 / max(CAMERA_DIAGNOSTIC_BLUR_SOURCE_SIZE_PX, vec2(1.0));
}

vec2 projection_area_content_uv(vec2 area_uv, int eye) {
    vec2 half_size = projection_area_half_size(eye);
    return (area_uv - (vec2(0.5) - half_size)) / max(half_size * 2.0, vec2(0.001));
}

vec2 projection_area_rect_edge_uv(
    vec2 area_uv,
    int eye,
    vec2 domain_min_uv,
    vec2 domain_max_uv,
    bool force_edge_sample
) {
    vec2 half_size = projection_area_half_size(eye);
    float core_scale = clamp(pc.stretch_params.x, 0.05, 1.0);
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

    float edge_inset = clamp(pc.stretch_params.y, 0.0, 0.49);
    float max_inset = clamp(max(pc.stretch_params.z, edge_inset), 0.0, 0.49);
    float curve = clamp(pc.stretch_params.w, 0.25, 6.0);
    float shaped_t = pow(exterior_t, curve);
    float inset = mix(edge_inset, max_inset, shaped_t);
    vec2 sample_half_size = max(core_half_size - vec2(inset), vec2(0.001));
    vec2 sample_uv = vec2(0.5) + edge_normalized * sample_half_size;
    return clamp(sample_uv, bounded_min_uv, bounded_max_uv);
}

float peripheral_stretch_blend_weight(float signed_distance_uv) {
    float inner_blend = clamp(pc.stretch_blend_params.x, 0.0, 0.25);
    float blend_curve = clamp(pc.stretch_blend_params.y, 0.25, 6.0);
    float blend_mode = floor(pc.stretch_blend_params.z + 0.5);
    if (blend_mode < 0.5) {
        return signed_distance_uv >= 0.0 ? 1.0 : 0.0;
    }
    if (inner_blend <= 0.0001) {
        return signed_distance_uv >= 0.0 ? 1.0 : 0.0;
    }
    float t = smoothstep(-inner_blend, 0.0, signed_distance_uv);
    return pow(t, blend_curve);
}

vec3 sample_source_eye_blur_raw(int source_eye, vec2 camera_uv, float radius_px) {
    float radius = max(radius_px, 0.0);
    if (radius <= 0.001) {
        return sample_source_eye_raw(source_eye, camera_uv).rgb;
    }
    vec2 texel = diagnostic_blur_texel_size() * radius * CAMERA_DIAGNOSTIC_BLUR_SAMPLE_STEP_GAIN;
    vec2 uv = clamp(camera_uv, vec2(0.0), vec2(1.0));
    vec3 sum = vec3(0.0);
    for (int y = -2; y <= 2; ++y) {
        for (int x = -2; x <= 2; ++x) {
            sum += sample_source_eye_raw(
                source_eye,
                clamp(uv + vec2(float(x), float(y)) * texel, vec2(0.0), vec2(1.0))
            ).rgb;
        }
    }
    return clamp01(sum / 25.0);
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
    int display_eye,
    float guide_brightness,
    float raw_feedback_signal
) {
    float oval_distance = resolve_camera_oval_distance(content_uv, display_eye);
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
    float oval_distance = resolve_camera_oval_distance(content_uv, display_eye);
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
        display_eye,
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

float display_eye_uv_domain(vec2 uv) {
    return step(0.0, uv.x) * step(uv.x, 1.0) * step(0.0, uv.y) * step(uv.y, 1.0);
}

float display_eye_uv_grid_line(vec2 uv, float spacing, float width) {
    vec2 cell = abs(fract(uv / spacing + vec2(0.5)) - vec2(0.5)) * spacing;
    return 1.0 - step(width, min(cell.x, cell.y));
}

float display_eye_uv_marker_mask(vec2 uv, vec2 center) {
    vec2 delta = abs(uv - center);
    float disk = 1.0 - smoothstep(0.024, 0.030, length(uv - center));
    float horizontal = (1.0 - step(0.006, delta.y)) * (1.0 - step(0.062, delta.x));
    float vertical = (1.0 - step(0.006, delta.x)) * (1.0 - step(0.062, delta.y));
    return clamp(max(disk, max(horizontal, vertical)), 0.0, 1.0);
}

void blend_display_eye_uv_marker(
    vec2 uv,
    vec2 center,
    vec3 marker_color,
    inout vec3 color
) {
    float mask = display_eye_uv_marker_mask(uv, center);
    color = mix(color, marker_color, mask);
}

vec3 resolve_display_eye_uv_fiducial(vec2 display_uv, int display_eye) {
    float domain = display_eye_uv_domain(display_uv);
    vec3 base_tint = display_eye == 0 ? vec3(0.018, 0.025, 0.034) : vec3(0.030, 0.023, 0.030);
    vec3 color = base_tint;
    float minor_grid = display_eye_uv_grid_line(display_uv, 0.125, 0.0018);
    float major_grid = display_eye_uv_grid_line(display_uv, 0.250, 0.0032);
    float center_axes = max(
        1.0 - step(0.0045, abs(display_uv.x - 0.5)),
        1.0 - step(0.0045, abs(display_uv.y - 0.5))
    );
    color = mix(color, vec3(0.090), clamp(minor_grid * domain * 0.45, 0.0, 1.0));
    color = mix(color, vec3(0.180), clamp(major_grid * domain * 0.60, 0.0, 1.0));
    color = mix(color, vec3(0.42), clamp(center_axes * domain * 0.55, 0.0, 1.0));

    blend_display_eye_uv_marker(display_uv, vec2(0.25, 0.25), vec3(0.0, 1.0, 1.0), color);
    blend_display_eye_uv_marker(display_uv, vec2(0.25, 0.50), vec3(1.0, 0.0, 0.0), color);
    blend_display_eye_uv_marker(display_uv, vec2(0.50, 0.25), vec3(1.0, 1.0, 0.0), color);
    blend_display_eye_uv_marker(display_uv, vec2(0.50, 0.50), vec3(0.0, 1.0, 0.0), color);
    blend_display_eye_uv_marker(display_uv, vec2(0.50, 0.75), vec3(1.0, 0.0, 1.0), color);
    blend_display_eye_uv_marker(display_uv, vec2(0.75, 0.50), vec3(0.0, 0.25, 1.0), color);

    return clamp01(color * domain);
}

float source_sampling_axis_mask(vec2 uv, float width) {
    float domain = display_eye_uv_domain(uv);
    float axes = max(
        1.0 - step(width, abs(uv.x - 0.5)),
        1.0 - step(width, abs(uv.y - 0.5))
    );
    return axes * domain;
}

float source_sampling_center_ring(vec2 uv, float radius, float width) {
    float domain = display_eye_uv_domain(uv);
    float distance_to_ring = abs(length(uv - vec2(0.5)) - radius);
    return (1.0 - step(width, distance_to_ring)) * domain;
}

vec3 resolve_source_sampling_witness(
    vec3 source_color,
    vec2 content_uv,
    vec2 source_sampler_uv,
    bool source_valid,
    int display_eye
) {
    vec3 color = source_valid
        ? source_color
        : mix(source_color * 0.15, vec3(0.36, 0.0, 0.0), 0.80);
    float source_luma_value = luma(color);
    color = mix(vec3(source_luma_value), color, 0.72);

    float content_domain = display_eye_uv_domain(content_uv);
    float sampler_domain = display_eye_uv_domain(source_sampler_uv);
    float content_minor = display_eye_uv_grid_line(content_uv, 0.125, 0.0016) * content_domain;
    float content_major = display_eye_uv_grid_line(content_uv, 0.250, 0.0030) * content_domain;
    float sampler_minor = display_eye_uv_grid_line(source_sampler_uv, 0.125, 0.0014) * sampler_domain;
    float sampler_major = display_eye_uv_grid_line(source_sampler_uv, 0.250, 0.0028) * sampler_domain;
    float content_axes = source_sampling_axis_mask(content_uv, 0.0042);
    float sampler_axes = source_sampling_axis_mask(source_sampler_uv, 0.0036);
    float content_ring = source_sampling_center_ring(content_uv, 0.070, 0.0060);
    float sampler_ring = source_sampling_center_ring(source_sampler_uv, 0.045, 0.0050);

    vec3 content_minor_color = display_eye == 0 ? vec3(0.78, 0.58, 0.16) : vec3(0.82, 0.50, 0.18);
    vec3 content_major_color = vec3(1.0, 0.74, 0.18);
    vec3 sampler_minor_color = display_eye == 0 ? vec3(0.15, 0.72, 0.92) : vec3(0.22, 0.65, 1.0);
    vec3 sampler_major_color = vec3(0.0, 0.94, 1.0);

    color = mix(color, content_minor_color, clamp(content_minor * 0.28, 0.0, 1.0));
    color = mix(color, sampler_minor_color, clamp(sampler_minor * 0.34, 0.0, 1.0));
    color = mix(color, content_major_color, clamp(content_major * 0.58, 0.0, 1.0));
    color = mix(color, sampler_major_color, clamp(sampler_major * 0.66, 0.0, 1.0));
    color = mix(color, vec3(1.0, 0.92, 0.18), clamp(content_axes * 0.84, 0.0, 1.0));
    color = mix(color, vec3(1.0, 0.0, 1.0), clamp(sampler_axes * 0.88, 0.0, 1.0));
    color = mix(color, vec3(1.0, 1.0, 1.0), clamp(content_ring * 0.88, 0.0, 1.0));
    color = mix(color, vec3(0.0, 0.0, 0.0), clamp(sampler_ring * 0.74, 0.0, 1.0));
    return clamp01(color);
}

float projection_alpha_mask(vec3 color) {
    vec3 rgb = clamp01(color);
    float luma = dot(rgb, vec3(0.2126, 0.7152, 0.0722));
    float max_channel = max(max(rgb.r, rgb.g), rgb.b);
    float min_channel = min(min(rgb.r, rgb.g), rgb.b);
    float saturation = max_channel - min_channel;
    int mode = int(floor(pc.alpha_params.x + 0.5));
    if (mode == 1) {
        return rgb.r;
    }
    if (mode == 2) {
        return rgb.g;
    }
    if (mode == 3) {
        return rgb.b;
    }
    if (mode == 4) {
        return luma;
    }
    if (mode == 5) {
        return 1.0 - rgb.r;
    }
    if (mode == 6) {
        return 1.0 - rgb.g;
    }
    if (mode == 7) {
        return 1.0 - rgb.b;
    }
    if (mode == 8) {
        return 1.0 - luma;
    }
    if (mode == 9) {
        return max(rgb.r - max(rgb.g, rgb.b), 0.0);
    }
    if (mode == 10) {
        return max(rgb.g - max(rgb.r, rgb.b), 0.0);
    }
    if (mode == 11) {
        return max(rgb.b - max(rgb.r, rgb.g), 0.0);
    }
    if (mode == 12) {
        return saturation;
    }
    if (mode == 13) {
        return 1.0 - saturation;
    }
    return 1.0;
}

float projection_color_alpha(vec3 color, float area_opacity) {
    float scaled_mask = projection_alpha_mask(color) * max(pc.alpha_params.y, 0.0) + pc.alpha_params.z;
    return clamp(area_opacity * clamp(scaled_mask, 0.0, 1.0), 0.0, 1.0);
}

void main() {
    int eye = clamp(v_eye_index, 0, 1);
    int packed_flags = int(floor(pc.params.w + 0.5));
    int source_eye = source_eye_for_display_eye(eye, packed_flags);
    int transform_flags = transform_flags_for_source_eye(source_eye, packed_flags);
    bool projected = pc.params.x < 0.0;
    bool world_canvas = pc.color_adjust.w > 1.5;
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
    bool target_local_raster_sampling =
        (packed_flags & CAMERA_FLAG_TARGET_LOCAL_RASTER_SAMPLING) != 0;
    bool target_footprint_from_metadata =
        (packed_flags & CAMERA_FLAG_TARGET_FOOTPRINT_FROM_METADATA) != 0;
    int diagnostic_mode = int(floor(pc.effect_params.w + 0.5));
    int peripheral_stretch_debug = int(floor(pc.alpha_params.w + 0.5));
    bool raw_projection_peripheral_stretch = diagnostic_mode == CAMERA_EFFECT_PERIPHERAL_STRETCH;
    bool camera_footprint_surface_mapping = diagnostic_mode == 4;
    bool full_frame_surface_mapping =
        target_local_raster_sampling && camera_footprint_surface_mapping;
    vec2 projection_screen_uv_base =
        (v_surface_uv - vec2(0.5)) * projection_area_scale + vec2(0.5);
    vec2 projection_area_domain_uv = projection_screen_uv_base - projection_area_offset;
    vec2 canonical_projection_area_domain_uv = projection_area_domain_uv;
    float projection_area_signed_distance_uv =
        target_footprint_signed_distance_uv(canonical_projection_area_domain_uv, eye);
    bool projection_area_inside = projection_area_signed_distance_uv <= 0.0;
    float stretch_weight = raw_projection_peripheral_stretch
        ? peripheral_stretch_blend_weight(projection_area_signed_distance_uv)
        : 0.0;
    bool stretch_exterior = raw_projection_peripheral_stretch && !projection_area_inside;
    bool target_transition_band = raw_projection_peripheral_stretch
        && projection_area_inside
        && stretch_weight > 0.0001;
    bool stretch_effect_sample_region = stretch_exterior || target_transition_band;
    if (stretch_effect_sample_region) {
        vec2 domain_min_uv =
            vec2(0.5) - vec2(0.5) * projection_area_scale - projection_area_offset;
        vec2 domain_max_uv =
            vec2(0.5) + vec2(0.5) * projection_area_scale - projection_area_offset;
        vec2 stretch_projection_area_domain_uv = projection_area_rect_edge_uv(
            canonical_projection_area_domain_uv,
            eye,
            domain_min_uv,
            domain_max_uv,
            stretch_weight > 0.0001
        );
        projection_area_domain_uv = mix(
            canonical_projection_area_domain_uv,
            stretch_projection_area_domain_uv,
            clamp(stretch_weight, 0.0, 1.0)
        );
        projection_screen_uv_base = projection_area_domain_uv + projection_area_offset;
    }
    // Metadata target footprints define the mask/effect boundary. They do not
    // change the screen-to-camera sampling domain for camera-homography runs.
    // Target-local raster sampling intentionally remaps the source into the
    // target-local footprint domain.
    bool target_local_source_mapping =
        target_local_raster_sampling && !full_frame_surface_mapping;
    vec2 projection_screen_uv = target_local_source_mapping
        ? projection_area_domain_uv
        : projection_screen_uv_base;

    vec2 local_uv = vec2(0.5) + ((v_surface_uv - vec2(0.5)) / overscan);
    bool content_surface_valid = true;
    vec2 projected_content_uv = content_uv_from_screen_uv(
            projection_screen_uv,
            eye,
            projected,
            content_surface_valid
        );
    vec2 content_uv = world_canvas
        ? projection_screen_uv
        : (projected
        ? projected_content_uv
        : (v_surface_uv - vec2(0.5)) * content_uv_scale + vec2(0.5));
    if (target_footprint_from_metadata && !full_frame_surface_mapping) {
        content_surface_valid = true;
    }
    vec2 full_frame_content_uv = full_frame_surface_mapping
        ? projected_content_uv
        : projection_area_content_uv(projection_area_domain_uv, eye);
    vec2 sample_content_uv = world_canvas
        ? (target_local_raster_sampling ? full_frame_content_uv : projection_screen_uv)
        : (target_local_raster_sampling
        ? full_frame_content_uv
        : (projected ? content_uv : clamp(local_uv, vec2(0.0), vec2(1.0))));
    vec2 projection_uv = world_canvas
        ? (target_local_raster_sampling ? full_frame_content_uv : projection_screen_uv)
        : (full_frame_surface_mapping
        ? projection_screen_uv
        : (target_local_raster_sampling
        ? full_frame_content_uv
        : (projected ? projection_screen_uv : sample_content_uv)));

    bool projection_valid = false;
    bool apply_projection_homography =
        world_canvas || (projected && (!target_local_raster_sampling || full_frame_surface_mapping));
    if (target_footprint_from_metadata && !full_frame_surface_mapping && !world_canvas) {
        apply_projection_homography = projected && !target_local_raster_sampling;
    }
    vec2 raw_projected_uv = projected_camera_uv(
        projection_uv,
        eye,
        source_eye,
        transform_flags,
        apply_projection_homography,
        projection_valid
    );
    projection_valid =
        projection_valid
        && ((target_local_raster_sampling && !full_frame_surface_mapping) || content_surface_valid);
    bool source_uv_stretchable =
        abs(raw_projected_uv.x) <= 65536.0 &&
        abs(raw_projected_uv.y) <= 65536.0;
    if (raw_projection_peripheral_stretch && stretch_effect_sample_region) {
        // Source-invalid transition/exterior pixels still belong to the
        // explicit stretch branch. Core/source-valid failures must stay
        // diagnostic and must not expand the effect footprint.
        vec2 source_edge_epsilon = max(camera_texel_size(source_eye) * 0.5, vec2(0.0001));
        raw_projected_uv = clamp(raw_projected_uv, source_edge_epsilon, vec2(1.0) - source_edge_epsilon);
        projection_valid = content_surface_valid && source_uv_stretchable;
    }
    float coverage = projection_coverage(raw_projected_uv, projection_valid, max(edge_fade, 0.012));
    bool projection_area_diagnostic = (packed_flags & CAMERA_FLAG_PROJECTION_AREA_DIAGNOSTIC) != 0;
    if (projection_area_diagnostic && diagnostic_mode == 1) {
        out_color = vec4(resolve_display_eye_uv_fiducial(projection_screen_uv_base, eye), 1.0);
        return;
    }
    if (projection_area_diagnostic && diagnostic_mode == 2) {
        out_color = vec4(resolve_display_eye_uv_fiducial(full_frame_content_uv, eye), 1.0);
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
    if (projection_area_diagnostic && diagnostic_mode == 3) {
        out_color = vec4(resolve_source_sampling_witness(
            center_color,
            full_frame_content_uv,
            raw_projected_uv,
            projection_valid,
            eye
        ), 1.0);
        return;
    }
    if (projection_area_diagnostic) {
        out_color = vec4(resolve_projection_area_diagnostic(
            projection_area_domain_uv,
            raw_projected_uv,
            projection_valid,
            content_surface_valid,
            eye
        ), 1.0);
        return;
    }

#ifdef RUSTY_XR_CAMERA_PROJECTION_DIRECT_ONLY
    bool direct_projection_border_solid_red =
        (packed_flags & CAMERA_FLAG_PROJECTION_BORDER_SOLID_RED) != 0;
    bool direct_passthrough_underlay_alpha =
        (packed_flags & CAMERA_FLAG_PASSTHROUGH_UNDERLAY_ALPHA) != 0;
    bool direct_projection_area_mask =
        direct_projection_border_solid_red || direct_passthrough_underlay_alpha;
    bool direct_projection_area_inside = projection_area_inside || stretch_exterior;
    bool direct_masked_projection_valid =
        projection_valid && (!direct_projection_area_mask || direct_projection_area_inside);
    float direct_surface_edge_distance = min(
        min(v_surface_uv.x, 1.0 - v_surface_uv.x),
        min(v_surface_uv.y, 1.0 - v_surface_uv.y)
    );
    float direct_surface_edge_dim = edge_fade > 0.0
        ? mix(0.90, 1.0, smoothstep(0.0, edge_fade, direct_surface_edge_distance))
        : 1.0;
    float direct_source_edge_dim = mix(0.94, 1.0, coverage);
    vec3 direct_border_fill_color = vec3(1.0, 0.0, 0.0);
    vec3 direct_source_invalid_color =
        raw_projection_peripheral_stretch && peripheral_stretch_debug == 1
        ? vec3(1.0, 0.0, 1.0)
        : direct_border_fill_color;
    bool direct_border_region =
        direct_projection_area_mask && !direct_projection_area_inside;
    bool direct_source_invalid_region =
        !projection_valid && (!direct_projection_area_mask || direct_projection_area_inside);
    vec3 direct_diagnostic_color = direct_source_invalid_region
        ? direct_source_invalid_color
        : direct_border_fill_color;
    vec3 direct_color =
        direct_projection_border_solid_red && (direct_border_region || direct_source_invalid_region)
        ? direct_diagnostic_color
        : center_color;
    float direct_out_alpha = 1.0;
    if (direct_projection_area_mask) {
        direct_out_alpha = direct_masked_projection_valid
            ? projection_color_alpha(direct_color, projection_area_opacity)
            : (direct_passthrough_underlay_alpha ? 0.0 : projection_border_opacity);
    }
    vec3 direct_final_color =
        direct_projection_border_solid_red && (direct_border_region || direct_source_invalid_region)
        ? direct_diagnostic_color
        : clamp01(direct_color * direct_surface_edge_dim * direct_source_edge_dim);
    if (raw_projection_peripheral_stretch && stretch_effect_sample_region && peripheral_stretch_debug == 2) {
        direct_final_color = vec3(
            clamp(raw_projected_uv, vec2(0.0), vec2(1.0)),
            target_transition_band ? 0.5 : 0.0
        );
        direct_out_alpha = 1.0;
    } else if (raw_projection_peripheral_stretch
        && target_transition_band
        && peripheral_stretch_debug == 1
        && !direct_source_invalid_region) {
        direct_final_color = mix(direct_final_color, vec3(1.0, 0.85, 0.0), 0.65);
        direct_out_alpha = 1.0;
    } else if (raw_projection_peripheral_stretch
        && stretch_exterior
        && peripheral_stretch_debug == 1
        && !direct_source_invalid_region) {
        direct_final_color = vec3(0.0, 1.0, 1.0);
        direct_out_alpha = 1.0;
    }
    bool direct_source_alpha_output =
        direct_projection_area_mask &&
        (direct_passthrough_underlay_alpha ||
            projection_area_opacity < 0.999 ||
            projection_border_opacity < 0.999 ||
            int(floor(pc.alpha_params.x + 0.5)) != 0);
    if (direct_source_alpha_output) {
        direct_final_color *= direct_out_alpha;
    }
    out_color = vec4(clamp01(direct_final_color), direct_out_alpha);
    return;
#endif

    bool raw_projection = (packed_flags & CAMERA_FLAG_RAW_PROJECTION) != 0;
    bool projection_border_solid_red =
        (packed_flags & CAMERA_FLAG_PROJECTION_BORDER_SOLID_RED) != 0;
    bool passthrough_underlay_alpha = (packed_flags & CAMERA_FLAG_PASSTHROUGH_UNDERLAY_ALPHA) != 0;
    bool raw_projection_blur = diagnostic_mode == CAMERA_EFFECT_RAW_PROJECTION_BLUR;
    bool raw_projection_area_mask = projection_border_solid_red || passthrough_underlay_alpha;
    bool effective_projection_area_inside = projection_area_inside || stretch_exterior;
    bool masked_projection_valid =
        projection_valid && (!raw_projection_area_mask || effective_projection_area_inside);
    vec3 raw_projection_border_fill_color = vec3(1.0, 0.0, 0.0);
    vec3 raw_projection_source_invalid_color =
        raw_projection_peripheral_stretch && peripheral_stretch_debug == 1
        ? vec3(1.0, 0.0, 1.0)
        : raw_projection_border_fill_color;
    bool raw_projection_border_region = raw_projection_area_mask && !effective_projection_area_inside;
    bool raw_projection_source_invalid_region =
        !projection_valid && (!raw_projection_area_mask || effective_projection_area_inside);
    vec3 raw_projection_diagnostic_color = raw_projection_source_invalid_region
        ? raw_projection_source_invalid_color
        : raw_projection_border_fill_color;
    vec3 color = center_color;
    if (raw_projection_blur) {
        color = masked_projection_valid
            ? sample_source_eye_blur_raw(source_eye, raw_projected_uv, pc.effect_params.x)
            : (projection_border_solid_red &&
                    (raw_projection_border_region || raw_projection_source_invalid_region)
                ? raw_projection_diagnostic_color
                : center_color);
    } else if (projection_border_solid_red) {
        color = masked_projection_valid
            ? center_color
            : raw_projection_diagnostic_color;
    } else if (raw_projection) {
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
            ? projection_color_alpha(color, projection_area_opacity)
            : (passthrough_underlay_alpha ? 0.0 : projection_border_opacity);
    }
    vec3 final_color = color * surface_edge_dim * source_edge_dim;
    if (projection_border_solid_red &&
            (raw_projection_border_region || raw_projection_source_invalid_region)) {
        final_color = raw_projection_diagnostic_color;
    }
    if (raw_projection_peripheral_stretch && stretch_effect_sample_region && peripheral_stretch_debug == 2) {
        final_color = vec3(
            clamp(raw_projected_uv, vec2(0.0), vec2(1.0)),
            target_transition_band ? 0.5 : 0.0
        );
        out_alpha = 1.0;
    } else if (raw_projection_peripheral_stretch
        && target_transition_band
        && peripheral_stretch_debug == 1
        && !raw_projection_source_invalid_region) {
        final_color = mix(final_color, vec3(1.0, 0.85, 0.0), 0.65);
        out_alpha = 1.0;
    } else if (raw_projection_peripheral_stretch
        && stretch_exterior
        && peripheral_stretch_debug == 1
        && !raw_projection_source_invalid_region) {
        final_color = vec3(0.0, 1.0, 1.0);
        out_alpha = 1.0;
    }
    bool source_alpha_output =
        raw_projection_area_mask &&
        (passthrough_underlay_alpha ||
            projection_area_opacity < 0.999 ||
            projection_border_opacity < 0.999 ||
            int(floor(pc.alpha_params.x + 0.5)) != 0);
    if (source_alpha_output) {
        final_color *= out_alpha;
    }
    out_color = vec4(clamp01(final_color), out_alpha);
}
