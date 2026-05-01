#version 450

layout(set = 0, binding = 0) uniform sampler2DArray u_environment_depth;

layout(push_constant) uniform EnvironmentDepthVisualizationPush {
    vec4 params;
    vec4 transform;
} pc;

layout(location = 0) in vec2 v_surface_uv;
layout(location = 1) flat in int v_eye_index;
layout(location = 0) out vec4 out_color;

vec2 apply_depth_texture_transform(vec2 uv, int flags) {
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

void main() {
    int transform_flags = int(floor(pc.transform.x + 0.5));
    vec2 uv = clamp(
        apply_depth_texture_transform(v_surface_uv, transform_flags),
        vec2(0.0),
        vec2(1.0));
    float raw_depth = texture(u_environment_depth, vec3(uv, float(v_eye_index))).r;
    float near_depth_meters = max(pc.params.y, 0.001);
    float max_depth_meters = max(pc.params.x, near_depth_meters + 0.1);

    if (!(raw_depth >= 0.0)) {
        raw_depth = 0.0;
    }

    bool infinity_cutoff = raw_depth >= 1.0 - (0.5 / 65535.0);
    float depth_meters = min(linear_depth_meters(raw_depth), max_depth_meters);
    float grayscale = clamp(depth_meters / max_depth_meters, 0.0, 1.0);

    if (infinity_cutoff) {
        grayscale = 1.0;
    }

    out_color = vec4(vec3(grayscale), 1.0);
}
