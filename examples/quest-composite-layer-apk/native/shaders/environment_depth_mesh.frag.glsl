#version 450

layout(set = 0, binding = 0) uniform sampler2DArray u_environment_depth;

layout(push_constant) uniform EnvironmentDepthVisualizationPush {
    vec4 params;
    vec4 transform;
    vec4 left_fov_tangents;
    vec4 right_fov_tangents;
    vec4 left_render_fov_tangents;
    vec4 right_render_fov_tangents;
    vec4 left_position;
    vec4 right_position;
    vec4 left_orientation;
    vec4 right_orientation;
    vec4 left_render_position;
    vec4 right_render_position;
    vec4 left_render_orientation;
    vec4 right_render_orientation;
} pc;

layout(location = 0) in vec2 v_depth_uv;
layout(location = 1) flat in int v_eye_index;
layout(location = 2) in vec3 v_stage_position;
layout(location = 3) in float v_depth_meters;
layout(location = 4) in float v_valid_depth;
layout(location = 0) out vec4 out_color;

const float MESH_DISTANCE_GRADIENT_MAX_METERS = 3.0;

float linear_depth_meters(float raw_depth) {
    float near_z = max(pc.params.y, 0.001);
    float far_z = pc.params.z;
    bool infinite_far = pc.params.w > 0.5 || !(far_z > near_z);
    raw_depth = clamp(raw_depth, 0.0, 1.0);

    if (infinite_far) {
        return near_z / max(1.0 - raw_depth, 1.0 / 65535.0);
    }

    return (near_z * far_z) / max(far_z - raw_depth * (far_z - near_z), 0.0001);
}

float sample_depth_meters(vec2 uv, int eye_index) {
    float raw_depth = texture(
        u_environment_depth,
        vec3(clamp(uv, vec2(0.0), vec2(1.0)), float(eye_index))).r;
    if (!(raw_depth >= 0.0)) {
        raw_depth = 0.0;
    }
    bool infinity_cutoff = raw_depth >= 1.0 - (0.5 / 65535.0);
    if (infinity_cutoff) {
        return max(pc.params.x, max(pc.params.y, 0.001) + 0.1);
    }
    return min(
        linear_depth_meters(raw_depth),
        max(pc.params.x, max(pc.params.y, 0.001) + 0.1));
}

float triangle_wire_line(vec2 plane_coord) {
    vec2 local = fract(plane_coord);
    float edge_distance = min(
        min(local.x, 1.0 - local.x),
        min(local.y, 1.0 - local.y));
    float diagonal_distance = abs(local.x - local.y) * 0.70710678;
    float width = clamp(
        max(fwidth(plane_coord.x), fwidth(plane_coord.y)) * 0.42,
        0.0025,
        0.014);
    float edge_line = 1.0 - smoothstep(width, width * 2.2, edge_distance);
    float diagonal_line = 1.0 - smoothstep(width * 0.75, width * 1.8, diagonal_distance);
    return max(edge_line, diagonal_line * 0.45);
}

float dominant_surface_wire(vec3 world_position, vec3 world_normal, float cell_meters) {
    vec3 axis_weight = abs(normalize(world_normal));
    float line_xy = triangle_wire_line(world_position.xy / cell_meters);
    float line_xz = triangle_wire_line(world_position.xz / cell_meters);
    float line_yz = triangle_wire_line(world_position.yz / cell_meters);
    if (axis_weight.z >= axis_weight.x && axis_weight.z >= axis_weight.y) {
        return line_xy;
    }
    if (axis_weight.y >= axis_weight.x) {
        return line_xz;
    }
    return line_yz;
}

vec3 depth_distance_gradient(float depth_meters) {
    float t = clamp(depth_meters / MESH_DISTANCE_GRADIENT_MAX_METERS, 0.0, 1.0);
    vec3 near_color = vec3(1.0, 0.18, 0.08);
    vec3 near_mid_color = vec3(1.0, 0.72, 0.10);
    vec3 mid_color = vec3(0.16, 0.86, 0.34);
    vec3 far_mid_color = vec3(0.0, 0.72, 1.0);
    vec3 far_color = vec3(0.32, 0.24, 1.0);
    if (t < 0.25) {
        return mix(near_color, near_mid_color, t / 0.25);
    }
    if (t < 0.5) {
        return mix(near_mid_color, mid_color, (t - 0.25) / 0.25);
    }
    if (t < 0.75) {
        return mix(mid_color, far_mid_color, (t - 0.5) / 0.25);
    }
    return mix(far_mid_color, far_color, (t - 0.75) / 0.25);
}

void main() {
    if (v_valid_depth < 0.5) {
        discard;
    }

    float cell_meters = max(pc.transform.z, 0.02);
    float discontinuity_meters = max(pc.transform.w, 0.01);
    float history_alpha = clamp(pc.transform.y, 0.0, 1.0);
    vec3 dx = dFdx(v_stage_position);
    vec3 dy = dFdy(v_stage_position);
    vec3 normal = cross(dx, dy);
    if (dot(normal, normal) < 1.0e-8) {
        normal = vec3(0.0, 1.0, 0.0);
    } else {
        normal = normalize(normal);
    }

    float line = dominant_surface_wire(v_stage_position, normal, cell_meters);
    vec2 depth_size = vec2(textureSize(u_environment_depth, 0).xy);
    vec2 sample_step = 1.0 / max(depth_size, vec2(1.0));
    float right_depth = sample_depth_meters(v_depth_uv + vec2(sample_step.x, 0.0), v_eye_index);
    float up_depth = sample_depth_meters(v_depth_uv + vec2(0.0, sample_step.y), v_eye_index);
    float discontinuity = max(
        abs(v_depth_meters - right_depth),
        abs(v_depth_meters - up_depth));
    float stable = 1.0 - smoothstep(
        discontinuity_meters,
        discontinuity_meters * 2.0,
        discontinuity);
    float break_line = 1.0 - stable;

    float depth_t = clamp(v_depth_meters / MESH_DISTANCE_GRADIENT_MAX_METERS, 0.0, 1.0);
    float surface_gradient = clamp(
        length(vec2(dFdx(v_depth_meters), dFdy(v_depth_meters))) * 4.0,
        0.0,
        1.0);
    float normal_light = 0.55 + 0.45 * abs(normal.y);
    vec3 stable_color = depth_distance_gradient(v_depth_meters);
    stable_color *= normal_light;
    stable_color += surface_gradient * vec3(0.06, 0.06, 0.06);
    vec3 break_color = vec3(1.0, 0.34, 0.06);

    float surface_alpha = history_alpha * stable * (0.02 + (1.0 - depth_t) * 0.025);
    float stable_alpha = history_alpha * line * (0.34 + (1.0 - depth_t) * 0.14);
    float break_alpha = history_alpha * break_line * 0.58;
    float alpha = max(max(surface_alpha, stable_alpha), break_alpha);
    if (alpha <= 0.001) {
        discard;
    }

    vec3 premultiplied = stable_color * max(surface_alpha, stable_alpha);
    premultiplied = mix(premultiplied, break_color * break_alpha, break_alpha);
    vec3 color = premultiplied / max(alpha, 0.001);
    out_color = vec4(color * alpha, alpha);
}
