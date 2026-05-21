#version 450
#extension GL_EXT_multiview : require

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
    vec4 alpha_params;
    vec4 area_params;
    vec4 area_offset_params;
    vec4 left_h0;
    vec4 left_h1;
    vec4 left_h2;
    vec4 right_h0;
    vec4 right_h1;
    vec4 right_h2;
} pc;

layout(location = 0) out vec2 v_surface_uv;
layout(location = 1) flat out int v_eye_index;

vec4 canvas_clip_for_corner(int eye, int corner) {
    if (eye == 0) {
        if (corner == 0) {
            return surface_map.left_canvas_clip0;
        }
        if (corner == 1) {
            return surface_map.left_canvas_clip1;
        }
        if (corner == 2) {
            return surface_map.left_canvas_clip2;
        }
        return surface_map.left_canvas_clip3;
    }
    if (corner == 0) {
        return surface_map.right_canvas_clip0;
    }
    if (corner == 1) {
        return surface_map.right_canvas_clip1;
    }
    if (corner == 2) {
        return surface_map.right_canvas_clip2;
    }
    return surface_map.right_canvas_clip3;
}

void main() {
    vec2 positions[3] = vec2[](
        vec2(-1.0, -1.0),
        vec2(3.0, -1.0),
        vec2(-1.0, 3.0)
    );
    vec2 canvas_uvs[4] = vec2[](
        vec2(0.0, 0.0),
        vec2(1.0, 0.0),
        vec2(1.0, 1.0),
        vec2(0.0, 1.0)
    );
    int canvas_indices[6] = int[](0, 1, 2, 2, 3, 0);

    v_eye_index = int(gl_ViewIndex);
    if (pc.color_adjust.w > 1.5) {
        int corner = canvas_indices[gl_VertexIndex % 6];
        gl_Position = canvas_clip_for_corner(v_eye_index, corner);
        v_surface_uv = canvas_uvs[corner];
        return;
    }

    vec2 position = positions[gl_VertexIndex];
    gl_Position = vec4(position, 0.0, 1.0);
    v_surface_uv = position * 0.5 + vec2(0.5);
}
