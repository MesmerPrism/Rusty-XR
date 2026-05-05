#version 450
#extension GL_EXT_multiview : require

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

layout(location = 0) out vec2 v_depth_uv;
layout(location = 1) flat out int v_eye_index;
layout(location = 2) out vec3 v_stage_position;
layout(location = 3) out float v_depth_meters;
layout(location = 4) out float v_valid_depth;

const int MESH_GRID_STRIDE_PIXELS = 4;

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

vec3 rotate_by_quat(vec3 value, vec4 q) {
    return value + 2.0 * cross(q.xyz, cross(q.xyz, value) + q.w * value);
}

vec4 inverse_quat(vec4 q) {
    return vec4(-q.xyz, q.w);
}

vec3 reconstruct_stage_position(vec2 depth_uv, int eye_index, float depth_meters) {
    vec4 fov = eye_index == 0 ? pc.left_fov_tangents : pc.right_fov_tangents;
    vec4 orientation = eye_index == 0 ? pc.left_orientation : pc.right_orientation;
    vec3 position = (eye_index == 0 ? pc.left_position : pc.right_position).xyz;
    float tangent_x = mix(fov.x, fov.y, depth_uv.x);
    float tangent_y = mix(fov.w, fov.z, depth_uv.y);
    vec3 view_position = vec3(tangent_x * depth_meters, tangent_y * depth_meters, -depth_meters);
    return position + rotate_by_quat(view_position, orientation);
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
    int eye_index = int(gl_ViewIndex);
    ivec2 depth_size = textureSize(u_environment_depth, 0).xy;
    ivec2 grid_size = max(depth_size / MESH_GRID_STRIDE_PIXELS, ivec2(2));
    int cells_x = max(grid_size.x - 1, 1);
    int cells_y = max(grid_size.y - 1, 1);
    int vertex_in_cell = gl_VertexIndex % 6;
    int cell_index = gl_VertexIndex / 6;
    int cell_x = cell_index % cells_x;
    int cell_y = min(cell_index / cells_x, cells_y - 1);
    ivec2 corner_offsets[6] = ivec2[](
        ivec2(0, 0),
        ivec2(1, 0),
        ivec2(0, 1),
        ivec2(1, 0),
        ivec2(1, 1),
        ivec2(0, 1)
    );
    ivec2 depth_pixel = min(
        (ivec2(cell_x, cell_y) + corner_offsets[vertex_in_cell]) * MESH_GRID_STRIDE_PIXELS,
        depth_size - ivec2(1));
    vec2 surface_uv = (vec2(depth_pixel) + vec2(0.5)) / max(vec2(depth_size), vec2(1.0));
    int transform_flags = int(floor(pc.transform.x + 0.5));
    vec2 depth_sample_uv = clamp(
        apply_depth_texture_transform(surface_uv, transform_flags),
        vec2(0.0),
        vec2(1.0));
    float raw_depth = textureLod(
        u_environment_depth,
        vec3(depth_sample_uv, float(eye_index)),
        0.0).r;
    if (!(raw_depth >= 0.0)) {
        raw_depth = 0.0;
    }

    float near_depth_meters = max(pc.params.y, 0.001);
    float max_depth_meters = max(pc.params.x, near_depth_meters + 0.1);
    bool infinity_cutoff = raw_depth >= 1.0 - (0.5 / 65535.0);
    float depth_meters = min(linear_depth_meters(raw_depth), max_depth_meters);
    float valid_depth = infinity_cutoff ? 0.0 : 1.0;
    if (infinity_cutoff) {
        depth_meters = max_depth_meters;
    }

    vec3 stage_position = reconstruct_stage_position(surface_uv, eye_index, depth_meters);
    vec3 render_view_position = stage_to_render_view_position(stage_position, eye_index);
    bool behind_near_plane = -render_view_position.z <= near_depth_meters;
    vec2 ndc = clamp(
        project_render_view_position(render_view_position, eye_index),
        vec2(-2.0),
        vec2(2.0));
    if (behind_near_plane) {
        ndc = vec2(2.0);
        valid_depth = 0.0;
    }

    gl_Position = behind_near_plane
        ? vec4(2.0, 2.0, 0.0, 1.0)
        : project_render_view_clip(render_view_position, eye_index);
    v_depth_uv = depth_sample_uv;
    v_eye_index = eye_index;
    v_stage_position = stage_position;
    v_depth_meters = depth_meters;
    v_valid_depth = valid_depth;
}
