#version 450

layout(location = 0) in vec2 v_local_uv;
layout(location = 1) flat in vec4 v_color;
layout(location = 2) flat in vec4 v_glyph;
layout(location = 0) out vec4 out_color;

layout(std430, set = 0, binding = 1) readonly buffer OscDiagnosticsFontAtlas {
    uint pixels[];
} font_atlas;

const int FONT_FIRST_CODE = 32;
const int FONT_LAST_CODE = 126;
const int FONT_COLUMNS = 16;
const int FONT_CELL_WIDTH = 80;
const int FONT_CELL_HEIGHT = 112;
const int FONT_ATLAS_WIDTH = 1280;
const int FONT_ATLAS_HEIGHT = 672;

const float SDF_CENTER = 0.5;
const float SDF_SPREAD_PX = 16.0;
const float SDF_SHARPNESS = 1.25;
const float SDF_WEIGHT_DISTANCE_PX = 0.85;

int atlas_code(int code) {
    if (code < FONT_FIRST_CODE || code > FONT_LAST_CODE) {
        return 63;
    }
    return code;
}

float atlas_sdf_at(ivec2 pixel) {
    ivec2 p = clamp(pixel, ivec2(0), ivec2(FONT_ATLAS_WIDTH - 1, FONT_ATLAS_HEIGHT - 1));
    int index = p.y * FONT_ATLAS_WIDTH + p.x;
    return float(font_atlas.pixels[index] & 0xffu) * (1.0 / 255.0);
}

float font_sdf(vec2 local_uv, int code) {
    int slot = atlas_code(code) - FONT_FIRST_CODE;
    int cell_x = (slot % FONT_COLUMNS) * FONT_CELL_WIDTH;
    int cell_y = (slot / FONT_COLUMNS) * FONT_CELL_HEIGHT;
    vec2 atlas_pixel = vec2(cell_x, cell_y) +
        clamp(local_uv, vec2(0.0), vec2(1.0)) *
        vec2(float(FONT_CELL_WIDTH - 1), float(FONT_CELL_HEIGHT - 1));
    ivec2 cell_min = ivec2(cell_x, cell_y);
    ivec2 cell_max = cell_min + ivec2(FONT_CELL_WIDTH - 1, FONT_CELL_HEIGHT - 1);
    ivec2 p0 = clamp(ivec2(floor(atlas_pixel)), cell_min, cell_max);
    ivec2 p1 = clamp(p0 + ivec2(1, 1), cell_min, cell_max);
    vec2 t = fract(atlas_pixel);
    float a00 = atlas_sdf_at(p0);
    float a10 = atlas_sdf_at(ivec2(p1.x, p0.y));
    float a01 = atlas_sdf_at(ivec2(p0.x, p1.y));
    float a11 = atlas_sdf_at(p1);
    return mix(mix(a00, a10, t.x), mix(a01, a11, t.x), t.y);
}

float glyph_signed_distance_px(vec2 local_uv, int code, float weight) {
    return (font_sdf(local_uv, code) - SDF_CENTER) * (2.0 * SDF_SPREAD_PX) +
        weight * SDF_WEIGHT_DISTANCE_PX;
}

float glyph_texels_per_screen_pixel(vec2 local_uv) {
    vec2 cell_size = vec2(float(FONT_CELL_WIDTH), float(FONT_CELL_HEIGHT));
    vec2 dx = dFdx(local_uv * cell_size);
    vec2 dy = dFdy(local_uv * cell_size);
    return max((length(dx) + length(dy)) * 0.5, 0.0001);
}

void main() {
    int mode = int(floor(v_glyph.y + 0.5));
    if (mode == 0) {
        out_color = v_color;
        return;
    }

    int code = int(floor(v_glyph.x + 0.5));
    float weight = clamp(v_glyph.z, 0.0, 1.0);
    float distance_px = glyph_signed_distance_px(v_local_uv, code, weight) /
        glyph_texels_per_screen_pixel(v_local_uv);
    float alpha = clamp(distance_px * SDF_SHARPNESS + 0.5, 0.0, 1.0);
    out_color = vec4(v_color.rgb, v_color.a * alpha);
}
