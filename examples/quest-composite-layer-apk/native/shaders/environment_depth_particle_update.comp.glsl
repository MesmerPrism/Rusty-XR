#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0) uniform sampler2DArray u_environment_depth;

struct DepthParticle {
    vec4 position_depth;
    vec4 state;
};

layout(std430, set = 0, binding = 1) buffer DepthParticles {
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

const uint PARTICLE_CAPACITY = 32768u;
const uint PARTICLE_SAMPLE_STRIDE_PIXELS = 12u;
const uint PARTICLE_SOURCE_VIEW_COUNT = 1u;
const float SCENE_PARTICLE_CELL_METERS = 0.06;
const uint SCENE_PARTICLE_PROBE_COUNT = 8u;
const float SCENE_PARTICLE_STALE_REPLACE_FRAMES = 1440.0;
const float SCENE_PARTICLE_MERGE_WEIGHT = 0.18;
const float SCENE_PARTICLE_ACTIVE_CORRECTION_CONFIDENCE = 0.78;
const float SCENE_PARTICLE_ACTIVE_CORRECTION_STEP_METERS = SCENE_PARTICLE_CELL_METERS;
const uint SCENE_PARTICLE_ACTIVE_CORRECTION_MAX_STEPS = 64u;
const float SCENE_PARTICLE_ACTIVE_CORRECTION_SURFACE_KEEP_METERS = 0.18;

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

float sample_depth_meters(vec2 uv, int eye_index) {
    float raw_depth = textureLod(
        u_environment_depth,
        vec3(clamp(uv, vec2(0.0), vec2(1.0)), float(eye_index)),
        0.0).r;
    bool infinity_cutoff = raw_depth >= 1.0 - (0.5 / 65535.0);
    if (!(raw_depth >= 0.0) || infinity_cutoff) {
        return pc.params.x + 1.0;
    }
    return min(linear_depth_meters(raw_depth), pc.params.x + 1.0);
}

vec3 rotate_by_quat(vec3 value, vec4 q) {
    return value + 2.0 * cross(q.xyz, cross(q.xyz, value) + q.w * value);
}

vec3 reconstruct_stage_position(vec2 depth_uv, int eye_index, float depth_meters) {
    vec4 fov = eye_index == 0 ? pc.left_fov_tangents : pc.right_fov_tangents;
    vec4 orientation = eye_index == 0 ? pc.left_orientation : pc.right_orientation;
    vec3 position = (eye_index == 0 ? pc.left_position : pc.right_position).xyz;
    float tangent_x = mix(fov.x, fov.y, depth_uv.x);
    float tangent_y = mix(fov.z, fov.w, depth_uv.y);
    vec3 view_position = vec3(tangent_x * depth_meters, tangent_y * depth_meters, -depth_meters);
    return position + rotate_by_quat(view_position, orientation);
}

uint hash_scene_cell(ivec3 cell) {
    uint h = (uint(cell.x) * 73856093u)
        ^ (uint(cell.y) * 19349663u)
        ^ (uint(cell.z) * 83492791u);
    h ^= h >> 16;
    h *= 0x7feb352du;
    h ^= h >> 15;
    h *= 0x846ca68bu;
    h ^= h >> 16;
    return h;
}

float compact_scene_cell_key(uint hash_value) {
    return float((hash_value & 0x00ffffffu) + 1u);
}

ivec3 scene_cell_for_stage_position(vec3 stage_position) {
    return ivec3(floor(stage_position / SCENE_PARTICLE_CELL_METERS));
}

void retire_scene_cell(ivec3 cell, float frame_marker) {
    uint hash_value = hash_scene_cell(cell);
    float cell_key = compact_scene_cell_key(hash_value);
    uint base_slot = hash_value % PARTICLE_CAPACITY;

    for (uint probe = 0u; probe < SCENE_PARTICLE_PROBE_COUNT; probe++) {
        uint slot = (base_slot + probe) % PARTICLE_CAPACITY;
        DepthParticle existing = particles[slot];
        bool occupied = existing.state.x >= 0.5 && existing.state.z >= 0.5;
        bool same_cell = abs(existing.state.z - cell_key) < 0.5;

        if (occupied && same_cell) {
            particles[slot].state = vec4(0.0, 0.0, existing.state.z, frame_marker);
            return;
        }
    }
}

void active_correct_visible_free_space(vec2 surface_uv, int eye_index, float observed_depth_meters) {
    float near_z = max(pc.params.y, 0.001);
    float start_depth = near_z + SCENE_PARTICLE_ACTIVE_CORRECTION_STEP_METERS;
    float active_range = SCENE_PARTICLE_ACTIVE_CORRECTION_STEP_METERS
        * float(SCENE_PARTICLE_ACTIVE_CORRECTION_MAX_STEPS);
    float stop_depth = min(
        observed_depth_meters - SCENE_PARTICLE_ACTIVE_CORRECTION_SURFACE_KEEP_METERS,
        active_range);

    if (!(stop_depth > start_depth)) {
        return;
    }

    for (uint step_index = 0u; step_index < SCENE_PARTICLE_ACTIVE_CORRECTION_MAX_STEPS; step_index++) {
        float depth_meters = start_depth
            + (float(step_index) + 0.5) * SCENE_PARTICLE_ACTIVE_CORRECTION_STEP_METERS;
        if (depth_meters >= stop_depth) {
            return;
        }

        vec3 stage_position = reconstruct_stage_position(surface_uv, eye_index, depth_meters);
        retire_scene_cell(scene_cell_for_stage_position(stage_position), max(pc.transform.y, 0.0));
    }
}

void write_scene_particle(vec3 stage_position, float depth_meters, float confidence) {
    float frame_marker = max(pc.transform.y, 0.0);
    ivec3 cell = scene_cell_for_stage_position(stage_position);
    uint hash_value = hash_scene_cell(cell);
    float cell_key = compact_scene_cell_key(hash_value);
    uint base_slot = hash_value % PARTICLE_CAPACITY;

    for (uint probe = 0u; probe < SCENE_PARTICLE_PROBE_COUNT; probe++) {
        uint slot = (base_slot + probe) % PARTICLE_CAPACITY;
        DepthParticle existing = particles[slot];
        bool empty = existing.state.x < 0.5 || existing.state.z < 0.5;
        bool same_cell = abs(existing.state.z - cell_key) < 0.5;
        float age_frames = max(frame_marker - existing.state.w, 0.0);
        bool stale = age_frames > SCENE_PARTICLE_STALE_REPLACE_FRAMES;

        if (empty || same_cell || stale) {
            float merge_weight = same_cell && !empty && !stale
                ? SCENE_PARTICLE_MERGE_WEIGHT * clamp(confidence, 0.0, 1.0)
                : 1.0;
            vec3 merged_position = same_cell && !empty && !stale
                ? mix(existing.position_depth.xyz, stage_position, merge_weight)
                : stage_position;
            float merged_depth = same_cell && !empty && !stale
                ? mix(existing.position_depth.w, depth_meters, merge_weight)
                : depth_meters;
            float merged_confidence = same_cell && !empty && !stale
                ? clamp(max(existing.state.y * 0.995, mix(existing.state.y, confidence, 0.22) + (confidence * 0.035)), 0.0, 1.0)
                : confidence;

            particles[slot].position_depth = vec4(merged_position, merged_depth);
            particles[slot].state = vec4(1.0, merged_confidence, cell_key, frame_marker);
            return;
        }
    }
}

void main() {
    ivec2 depth_size = textureSize(u_environment_depth, 0).xy;
    uvec2 grid_size = max(
        uvec2(depth_size) / uvec2(PARTICLE_SAMPLE_STRIDE_PIXELS),
        uvec2(1u));
    uint eye_index = gl_GlobalInvocationID.z;
    uint gx = gl_GlobalInvocationID.x;
    uint gy = gl_GlobalInvocationID.y;
    if (eye_index >= PARTICLE_SOURCE_VIEW_COUNT || gx >= grid_size.x || gy >= grid_size.y) {
        return;
    }

    uint sample_index = ((eye_index * grid_size.y) + gy) * grid_size.x + gx;
    uint write_base = uint(max(pc.transform.y, 0.0));
    uint slot = (write_base + sample_index) % PARTICLE_CAPACITY;
    ivec2 pixel = min(
        ivec2(gx, gy) * int(PARTICLE_SAMPLE_STRIDE_PIXELS)
            + ivec2(int(PARTICLE_SAMPLE_STRIDE_PIXELS / 2u)),
        depth_size - ivec2(1));
    vec2 surface_uv = (vec2(pixel) + vec2(0.5)) / max(vec2(depth_size), vec2(1.0));
    int transform_flags = int(floor(pc.transform.x + 0.5));
    vec2 depth_uv = clamp(
        apply_depth_texture_transform(surface_uv, transform_flags),
        vec2(0.0),
        vec2(1.0));

    float depth_meters = sample_depth_meters(depth_uv, int(eye_index));
    vec2 sample_step = 1.0 / max(vec2(depth_size), vec2(1.0));
    float right_depth = sample_depth_meters(depth_uv + vec2(sample_step.x, 0.0), int(eye_index));
    float up_depth = sample_depth_meters(depth_uv + vec2(0.0, sample_step.y), int(eye_index));
    float discontinuity = max(abs(depth_meters - right_depth), abs(depth_meters - up_depth));
    float threshold = max(pc.transform.w, 0.01);
    float confidence = 1.0 - smoothstep(threshold, threshold * 2.0, discontinuity);
    bool valid = depth_meters <= pc.params.x && confidence >= 0.58;

    bool scene_particle_map = pc.transform.z > 0.5;
    if (!valid && scene_particle_map) {
        return;
    }
    if (!valid) {
        particles[slot].state = vec4(0.0);
        return;
    }

    vec3 stage_position = reconstruct_stage_position(surface_uv, int(eye_index), depth_meters);
    if (scene_particle_map) {
        if (confidence >= SCENE_PARTICLE_ACTIVE_CORRECTION_CONFIDENCE) {
            active_correct_visible_free_space(surface_uv, int(eye_index), depth_meters);
        }
        write_scene_particle(stage_position, depth_meters, confidence);
        return;
    }

    particles[slot].position_depth = vec4(stage_position, depth_meters);
    particles[slot].state = vec4(1.0, confidence, float(eye_index), 0.0);
}
