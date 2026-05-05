#version 450
#extension GL_EXT_multiview : require

struct DepthParticle {
    vec4 position_depth;
    vec4 state;
};

layout(std430, set = 0, binding = 1) readonly buffer DepthParticles {
    DepthParticle particles[];
};

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

layout(location = 0) out vec2 v_particle_uv;
layout(location = 1) out float v_depth_meters;
layout(location = 2) out float v_confidence;

const uint PARTICLE_CAPACITY = 32768u;

vec3 rotate_by_quat(vec3 value, vec4 q) {
    return value + 2.0 * cross(q.xyz, cross(q.xyz, value) + q.w * value);
}

vec4 inverse_quat(vec4 q) {
    return vec4(-q.xyz, q.w);
}

vec3 stage_to_render_view_position(vec3 stage_position, int eye_index) {
    vec4 orientation = eye_index == 0
        ? pc.left_render_orientation
        : pc.right_render_orientation;
    vec3 position = (eye_index == 0
        ? pc.left_render_position
        : pc.right_render_position).xyz;
    return rotate_by_quat(stage_position - position, inverse_quat(orientation));
}

vec2 project_render_view_position(vec3 view_position, int eye_index) {
    vec4 fov = eye_index == 0
        ? pc.left_render_fov_tangents
        : pc.right_render_fov_tangents;
    float forward_z = max(-view_position.z, 0.001);
    float tangent_x = view_position.x / forward_z;
    float tangent_y = view_position.y / forward_z;
    float u = (tangent_x - fov.x) / max(fov.y - fov.x, 0.0001);
    float v = (tangent_y - fov.w) / max(fov.z - fov.w, 0.0001);
    return vec2(u, v) * 2.0 - vec2(1.0);
}

vec4 project_render_view_clip(vec3 view_position, int eye_index) {
    vec4 fov = eye_index == 0
        ? pc.left_render_fov_tangents
        : pc.right_render_fov_tangents;
    float near_z = max(pc.params.y, 0.001);
    float far_z = pc.params.z;
    bool infinite_far = pc.params.w > 0.5 || !(far_z > near_z);
    float tangent_width = max(fov.y - fov.x, 0.0001);
    float tangent_height = max(fov.z - fov.w, 0.0001);
    float clip_x = (2.0 / tangent_width) * view_position.x
        + ((fov.y + fov.x) / tangent_width) * view_position.z;
    float clip_y = (2.0 / tangent_height) * view_position.y
        + ((fov.z + fov.w) / tangent_height) * view_position.z;
    float clip_z = infinite_far
        ? -view_position.z - near_z
        : (far_z / (near_z - far_z)) * view_position.z
            + ((far_z * near_z) / (near_z - far_z));
    float clip_w = -view_position.z;
    return vec4(clip_x, clip_y, clip_z, clip_w);
}

void main() {
    uint particle_index = uint(gl_VertexIndex / 6);
    uint corner_index = uint(gl_VertexIndex % 6);
    vec2 corners[6] = vec2[](
        vec2(-1.0, -1.0),
        vec2(1.0, -1.0),
        vec2(-1.0, 1.0),
        vec2(1.0, -1.0),
        vec2(1.0, 1.0),
        vec2(-1.0, 1.0)
    );
    vec2 corner = corners[corner_index];
    if (particle_index >= PARTICLE_CAPACITY) {
        gl_Position = vec4(2.0, 2.0, 0.0, 1.0);
        v_particle_uv = corner;
        v_depth_meters = 0.0;
        v_confidence = 0.0;
        return;
    }

    DepthParticle particle = particles[particle_index];
    float valid = particle.state.x;
    int eye_index = int(gl_ViewIndex);
    vec3 view_position = stage_to_render_view_position(particle.position_depth.xyz, eye_index);
    bool behind_near_plane = -view_position.z <= max(pc.params.y, 0.001);
    vec2 ndc = project_render_view_position(view_position, eye_index);
    if (valid < 0.5 || behind_near_plane || any(lessThan(ndc, vec2(-1.35))) || any(greaterThan(ndc, vec2(1.35)))) {
        gl_Position = vec4(2.0, 2.0, 0.0, 1.0);
        v_particle_uv = corner;
        v_depth_meters = particle.position_depth.w;
        v_confidence = 0.0;
        return;
    }

    float half_size_meters = mix(0.008, 0.016, clamp(particle.state.y, 0.0, 1.0));
    vec3 corner_view_position = view_position
        + vec3(corner.x * half_size_meters, corner.y * half_size_meters, 0.0);
    gl_Position = project_render_view_clip(corner_view_position, eye_index);
    v_particle_uv = corner;
    v_depth_meters = particle.position_depth.w;
    v_confidence = particle.state.y;
}
