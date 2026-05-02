#version 450
#extension GL_EXT_multiview : require

struct OscDiagnosticsOverlayInstance {
    vec4 left_clip[4];
    vec4 right_clip[4];
    vec4 color;
    vec4 glyph;
};

layout(std430, set = 0, binding = 0) readonly buffer OscDiagnosticsOverlayInstances {
    OscDiagnosticsOverlayInstance instances[];
} overlay;

layout(location = 0) out vec2 v_local_uv;
layout(location = 1) flat out vec4 v_color;
layout(location = 2) flat out vec4 v_glyph;

const vec2 CORNERS[6] = vec2[6](
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(0.0, 1.0),
    vec2(0.0, 1.0),
    vec2(1.0, 0.0),
    vec2(1.0, 1.0)
);

void main() {
    OscDiagnosticsOverlayInstance instance = overlay.instances[gl_InstanceIndex];
    vec4 clip0 = gl_ViewIndex == 0 ? instance.left_clip[0] : instance.right_clip[0];
    vec4 clip1 = gl_ViewIndex == 0 ? instance.left_clip[1] : instance.right_clip[1];
    vec4 clip2 = gl_ViewIndex == 0 ? instance.left_clip[2] : instance.right_clip[2];
    vec4 clip3 = gl_ViewIndex == 0 ? instance.left_clip[3] : instance.right_clip[3];
    vec2 local = CORNERS[gl_VertexIndex % 6];
    vec4 top = mix(clip0, clip1, local.x);
    vec4 bottom = mix(clip3, clip2, local.x);

    gl_Position = mix(top, bottom, local.y);
    v_local_uv = local;
    v_color = instance.color;
    v_glyph = instance.glyph;
}
