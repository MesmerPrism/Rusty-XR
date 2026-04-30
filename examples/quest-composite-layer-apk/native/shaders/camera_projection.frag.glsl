#version 450

layout(set = 0, binding = 0) uniform sampler2D u_camera_left;
layout(set = 0, binding = 1) uniform sampler2D u_camera_right;

layout(push_constant) uniform CameraProjectionPush {
    vec4 params;
    vec4 border_color;
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
const float BORDER_RADIUS_X = 0.47;
const float BORDER_RADIUS_Y = 0.36;
const float BORDER_FEATHER = 0.10;
const float BORDER_CORNER_RADIUS = 0.08;
const float BORDER_BRIGHTNESS_INSET = 0.16;
const float BORDER_BRIGHTNESS_CUTOFF = 0.25;
const float BORDER_BRIGHTNESS_FEATHER = 0.14;

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
    return source_eye == 0
        ? texture(u_camera_left, uv)
        : texture(u_camera_right, uv);
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
    vec2 half_size = vec2(max(BORDER_RADIUS_X, 0.05), max(BORDER_RADIUS_Y, 0.05));
    float corner_radius = clamp(
        BORDER_CORNER_RADIUS,
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
    ivec2 size = source_eye == 0 ? textureSize(u_camera_left, 0) : textureSize(u_camera_right, 0);
    vec2 dims = vec2(float(max(size.x, 1)), float(max(size.y, 1)));
    return 1.0 / dims;
}

float source_luma(vec2 sample_uv, int source_eye, int transform_flags) {
    return luma(sample_source_eye_oriented(source_eye, sample_uv, transform_flags).rgb);
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

float source_edge_strength(vec2 sample_uv, int source_eye, int transform_flags) {
    vec2 step_size = max(camera_texel_size(source_eye) * 2.0, vec2(1.0 / 2048.0));
    float left = source_luma(sample_uv - vec2(step_size.x, 0.0), source_eye, transform_flags);
    float right = source_luma(sample_uv + vec2(step_size.x, 0.0), source_eye, transform_flags);
    float up = source_luma(sample_uv - vec2(0.0, step_size.y), source_eye, transform_flags);
    float down = source_luma(sample_uv + vec2(0.0, step_size.y), source_eye, transform_flags);
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
    int source_eye,
    int transform_flags,
    vec2 raw_projected_uv,
    float coverage
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
    vec2 guide_uv = clamp_border_seed_uv(
        center + screen_dir * clamp(screen_radius * 0.50, 0.04, 0.36)
    );
    float guide_brightness = guide_luma(guide_uv, source_eye, transform_flags);
    float guide_edge = source_edge_strength(guide_uv, source_eye, transform_flags);
    float dark_signal = 1.0 - smoothstep(0.10, 0.55, guide_brightness);
    float geometry_signal = clamp(max(guide_edge, dark_signal * 0.72), 0.0, 1.0);
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

    vec3 base = sample_source_eye_oriented(source_eye, trail_a_uv, transform_flags).rgb;
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
    int source_eye,
    int transform_flags,
    vec3 center_color,
    vec2 raw_projected_uv,
    float coverage,
    bool projection_valid,
    float edge_fade
) {
    vec2 center = vec2(0.5);
    vec2 screen_delta = content_uv - center;
    float screen_radius = max(length(screen_delta), 0.0001);
    vec2 guide_uv = clamp_border_seed_uv(
        center + (screen_delta / screen_radius) * clamp(screen_radius * 0.50, 0.04, 0.36)
    );
    float guide_brightness = guide_luma(guide_uv, source_eye, transform_flags);
    float raw_feedback_signal = max(
        source_edge_strength(guide_uv, source_eye, transform_flags),
        1.0 - smoothstep(0.10, 0.55, guide_brightness)
    );
    float border_mix = resolve_fov_border_composite_mix(
        content_uv,
        guide_brightness,
        raw_feedback_signal
    );
    float oval_distance = resolve_camera_oval_distance(content_uv);
    float noise = bleed_noise(content_uv);
    float spatial_gate = resolve_brightness_bleed_spatial_gate(oval_distance, noise);
    float projection_gap_mix = (1.0 - coverage)
        * resolve_camera_oval_border_mix_from_distance(oval_distance)
        * spatial_gate;
    float coverage_mix = resolve_fov_border_mix(coverage) * (1.0 - smoothstep(0.0, max(edge_fade, 0.0001), coverage));
    float invalid_boost = projection_valid ? 0.0 : projection_gap_mix;
    float resolved_border_mix = max(max(border_mix, projection_gap_mix), max(coverage_mix, invalid_boost));
    vec3 border_color = resolve_fov_border_color(
        content_uv,
        source_eye,
        transform_flags,
        raw_projected_uv,
        min(coverage, 1.0 - resolved_border_mix)
    );
    return mix(center_color, border_color, clamp(resolved_border_mix, 0.0, pc.border_color.a));
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

    vec2 local_uv = vec2(0.5) + ((v_surface_uv - vec2(0.5)) / overscan);
    vec2 content_uv = (v_surface_uv - vec2(0.5)) * content_uv_scale + vec2(0.5);
    vec2 sample_content_uv = projected ? content_uv : clamp(local_uv, vec2(0.0), vec2(1.0));
    vec2 projection_uv = projected ? v_surface_uv : sample_content_uv;

    bool projection_valid = false;
    vec2 raw_projected_uv = projected_camera_uv(
        projection_uv,
        eye,
        transform_flags,
        projected,
        projection_valid
    );
    float coverage = projection_coverage(raw_projected_uv, projection_valid, max(edge_fade, 0.012));

    vec3 center_color = projection_valid
        ? sample_source_eye_raw(source_eye, raw_projected_uv).rgb
        : sample_source_eye_oriented(
            source_eye,
            clamp_border_seed_uv(clamp(sample_content_uv, vec2(0.0), vec2(1.0))),
            transform_flags
        ).rgb;
    if (!projection_valid && projected) {
        center_color *= 0.12;
    }

    vec3 color = resolve_fov_border_composite(
        sample_content_uv,
        source_eye,
        transform_flags,
        center_color,
        raw_projected_uv,
        coverage,
        projection_valid,
        edge_fade
    );

    float surface_edge_distance = min(
        min(v_surface_uv.x, 1.0 - v_surface_uv.x),
        min(v_surface_uv.y, 1.0 - v_surface_uv.y)
    );
    float surface_edge_dim = edge_fade > 0.0
        ? mix(0.90, 1.0, smoothstep(0.0, edge_fade, surface_edge_distance))
        : 1.0;
    float source_edge_dim = mix(0.94, 1.0, coverage);
    out_color = vec4(clamp01(color * surface_edge_dim * source_edge_dim), 1.0);
}
