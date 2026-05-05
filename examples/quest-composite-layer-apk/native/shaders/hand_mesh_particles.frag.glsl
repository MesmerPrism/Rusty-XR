#version 450

layout(location = 0) in vec2 v_particle_uv;
layout(location = 1) in vec4 v_color;
layout(location = 0) out vec4 out_color;

void main() {
    float radius = length(v_particle_uv);
    float disc = 1.0 - smoothstep(0.72, 1.0, radius);
    float core = 1.0 - smoothstep(0.0, 0.45, radius);
    float alpha = disc * clamp(v_color.a, 0.0, 1.0);
    if (alpha <= 0.002) {
        discard;
    }

    vec3 color = mix(v_color.rgb, vec3(1.0), core * 0.18);
    out_color = vec4(color * alpha, alpha);
}
