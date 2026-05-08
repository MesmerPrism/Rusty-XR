#version 450

layout(location = 0) in vec2 v_particle_uv;
layout(location = 1) in float v_depth_meters;
layout(location = 2) in float v_confidence;
layout(location = 0) out vec4 out_color;

const float PARTICLE_DISTANCE_GRADIENT_MAX_METERS = 3.0;
const float DEFAULT_PARTICLE_DISC_CLIP = 0.5;

vec3 depth_distance_gradient(float depth_meters) {
    float t = clamp(depth_meters / PARTICLE_DISTANCE_GRADIENT_MAX_METERS, 0.0, 1.0);
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
    float radius = length(v_particle_uv);
    float disc = 1.0 - smoothstep(0.72, 1.0, radius);
    float core = 1.0 - smoothstep(0.0, 0.45, radius);
    if (disc < DEFAULT_PARTICLE_DISC_CLIP || v_confidence <= 0.002) {
        discard;
    }

    vec3 color = depth_distance_gradient(v_depth_meters);
    color = mix(color, vec3(1.0), core * 0.16);
    out_color = vec4(color, 1.0);
}
