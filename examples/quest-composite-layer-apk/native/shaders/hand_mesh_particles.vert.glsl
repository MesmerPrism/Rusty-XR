#version 450
#extension GL_EXT_multiview : require

struct HandParticle {
    vec4 position_radius;
    vec4 color_alpha;
};

layout(std430, set = 0, binding = 0) readonly buffer HandParticles {
    HandParticle particles[];
};

layout(push_constant) uniform HandParticlePush {
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
layout(location = 1) out vec4 v_color;

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
    HandParticle particle = particles[particle_index];
    int eye_index = int(gl_ViewIndex);
    vec3 view_position = stage_to_render_view_position(particle.position_radius.xyz, eye_index);
    bool behind_near_plane = -view_position.z <= max(pc.params.y, 0.001);
    vec2 ndc = project_render_view_position(view_position, eye_index);
    if (particle.color_alpha.a <= 0.002 || behind_near_plane || any(lessThan(ndc, vec2(-1.35))) || any(greaterThan(ndc, vec2(1.35)))) {
        gl_Position = vec4(2.0, 2.0, 0.0, 1.0);
        v_particle_uv = corner;
        v_color = vec4(0.0);
        return;
    }

    float half_size_meters = max(particle.position_radius.w, 0.001);
    vec3 corner_view_position = view_position
        + vec3(corner.x * half_size_meters, corner.y * half_size_meters, 0.0);
    gl_Position = project_render_view_clip(corner_view_position, eye_index);
    v_particle_uv = corner;
    v_color = particle.color_alpha;
}
