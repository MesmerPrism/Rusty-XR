use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const OSC_DIAGNOSTICS_FONT_FIRST_CODE: u32 = 32;
const OSC_DIAGNOSTICS_FONT_LAST_CODE: u32 = 126;
const OSC_DIAGNOSTICS_FONT_COLUMNS: usize = 16;
const OSC_DIAGNOSTICS_FONT_CELL_WIDTH: usize = 80;
const OSC_DIAGNOSTICS_FONT_CELL_HEIGHT: usize = 112;
const OSC_DIAGNOSTICS_FONT_ATLAS_WIDTH: usize =
    OSC_DIAGNOSTICS_FONT_COLUMNS * OSC_DIAGNOSTICS_FONT_CELL_WIDTH;
const OSC_DIAGNOSTICS_FONT_ROWS: usize =
    ((OSC_DIAGNOSTICS_FONT_LAST_CODE - OSC_DIAGNOSTICS_FONT_FIRST_CODE + 1) as usize)
        .div_ceil(OSC_DIAGNOSTICS_FONT_COLUMNS);
const OSC_DIAGNOSTICS_FONT_ATLAS_HEIGHT: usize =
    OSC_DIAGNOSTICS_FONT_ROWS * OSC_DIAGNOSTICS_FONT_CELL_HEIGHT;
const OSC_DIAGNOSTICS_FONT_SIZE_PX: f32 = 84.0;

fn main() {
    println!("cargo:rerun-if-changed=shaders/camera_projection.vert.glsl");
    println!("cargo:rerun-if-changed=shaders/camera_projection.frag.glsl");
    println!("cargo:rerun-if-changed=shaders/environment_depth_visualization.frag.glsl");
    println!("cargo:rerun-if-changed=shaders/osc_diagnostics_overlay.vert.glsl");
    println!("cargo:rerun-if-changed=shaders/osc_diagnostics_overlay.frag.glsl");
    println!("cargo:rerun-if-changed=assets/fonts/JetBrainsMono-Regular.ttf");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }

    let glslc = find_glslc().unwrap_or_else(|| {
        panic!(
            "Android shader build needs glslc. Put glslc on PATH, set GLSLC, or set ANDROID_NDK_ROOT/ANDROID_NDK_HOME."
        )
    });
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    generate_osc_diagnostics_font_atlas(&out_dir);

    compile_shader(
        &glslc,
        "vertex",
        Path::new("shaders/camera_projection.vert.glsl"),
        &out_dir.join("camera_projection.vert.spv"),
        &[],
    );
    compile_shader(
        &glslc,
        "fragment",
        Path::new("shaders/camera_projection.frag.glsl"),
        &out_dir.join("camera_projection.frag.spv"),
        &[],
    );
    compile_shader(
        &glslc,
        "fragment",
        Path::new("shaders/camera_projection.frag.glsl"),
        &out_dir.join("camera_projection_separate_sampler.frag.spv"),
        &["RUSTY_XR_SEPARATE_CAMERA_SAMPLER=1"],
    );
    compile_shader(
        &glslc,
        "fragment",
        Path::new("shaders/environment_depth_visualization.frag.glsl"),
        &out_dir.join("environment_depth_visualization.frag.spv"),
        &[],
    );
    compile_shader(
        &glslc,
        "vertex",
        Path::new("shaders/osc_diagnostics_overlay.vert.glsl"),
        &out_dir.join("osc_diagnostics_overlay.vert.spv"),
        &[],
    );
    compile_shader(
        &glslc,
        "fragment",
        Path::new("shaders/osc_diagnostics_overlay.frag.glsl"),
        &out_dir.join("osc_diagnostics_overlay.frag.spv"),
        &[],
    );
}

fn generate_osc_diagnostics_font_atlas(out_dir: &Path) {
    let font_bytes = include_bytes!("assets/fonts/JetBrainsMono-Regular.ttf");
    let font = fontdue::Font::from_bytes(
        font_bytes.as_slice(),
        fontdue::FontSettings {
            scale: OSC_DIAGNOSTICS_FONT_SIZE_PX,
            ..fontdue::FontSettings::default()
        },
    )
    .expect("JetBrains Mono regular font asset is parseable");
    let line = font
        .horizontal_line_metrics(OSC_DIAGNOSTICS_FONT_SIZE_PX)
        .expect("JetBrains Mono regular has horizontal line metrics");
    let line_height = line.ascent - line.descent;
    let baseline_y =
        ((OSC_DIAGNOSTICS_FONT_CELL_HEIGHT as f32 - line_height) * 0.5 + line.ascent + 0.5).round()
            as i32;

    let mut atlas = vec![0u8; OSC_DIAGNOSTICS_FONT_ATLAS_WIDTH * OSC_DIAGNOSTICS_FONT_ATLAS_HEIGHT];
    for code in OSC_DIAGNOSTICS_FONT_FIRST_CODE..=OSC_DIAGNOSTICS_FONT_LAST_CODE {
        let character = char::from_u32(code).expect("ASCII diagnostic HUD codepoint is valid");
        let slot = (code - OSC_DIAGNOSTICS_FONT_FIRST_CODE) as usize;
        let cell_x = (slot % OSC_DIAGNOSTICS_FONT_COLUMNS) * OSC_DIAGNOSTICS_FONT_CELL_WIDTH;
        let cell_y = (slot / OSC_DIAGNOSTICS_FONT_COLUMNS) * OSC_DIAGNOSTICS_FONT_CELL_HEIGHT;
        let (metrics, bitmap) = font.rasterize(character, OSC_DIAGNOSTICS_FONT_SIZE_PX);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }
        let pen_x = cell_x as f32
            + ((OSC_DIAGNOSTICS_FONT_CELL_WIDTH as f32 - metrics.advance_width) * 0.5);
        let glyph_x = (pen_x + metrics.xmin as f32).round() as i32;
        let glyph_y = cell_y as i32 + baseline_y - metrics.ymin - metrics.height as i32;
        copy_glyph_bitmap_to_atlas(&mut atlas, glyph_x, glyph_y, metrics.width, &bitmap);
    }

    let sdf = alpha_atlas_to_sdf(&atlas);
    let mut atlas_u32 = Vec::with_capacity(sdf.len() * std::mem::size_of::<u32>());
    for distance in sdf {
        atlas_u32.extend_from_slice(&(distance as u32).to_ne_bytes());
    }
    fs::write(
        out_dir.join("osc_diagnostics_font_atlas_u32.bin"),
        atlas_u32,
    )
    .expect("write generated OSC diagnostics font atlas");
}

fn alpha_atlas_to_sdf(alpha: &[u8]) -> Vec<u8> {
    let mut sdf = vec![0u8; alpha.len()];
    for slot in 0..=((OSC_DIAGNOSTICS_FONT_LAST_CODE - OSC_DIAGNOSTICS_FONT_FIRST_CODE) as usize) {
        let cell_x = (slot % OSC_DIAGNOSTICS_FONT_COLUMNS) * OSC_DIAGNOSTICS_FONT_CELL_WIDTH;
        let cell_y = (slot / OSC_DIAGNOSTICS_FONT_COLUMNS) * OSC_DIAGNOSTICS_FONT_CELL_HEIGHT;
        alpha_cell_to_sdf(alpha, &mut sdf, cell_x, cell_y);
    }
    sdf
}

fn alpha_cell_to_sdf(alpha: &[u8], sdf: &mut [u8], cell_x: usize, cell_y: usize) {
    const EDGE_THRESHOLD: u8 = 64;
    const SDF_SPREAD_PX: f32 = 16.0;

    let mut edge_pixels = Vec::new();
    for y in 0..OSC_DIAGNOSTICS_FONT_CELL_HEIGHT {
        for x in 0..OSC_DIAGNOSTICS_FONT_CELL_WIDTH {
            let inside = cell_alpha(alpha, cell_x, cell_y, x, y) >= EDGE_THRESHOLD;
            let edge = (x > 0
                && (cell_alpha(alpha, cell_x, cell_y, x - 1, y) >= EDGE_THRESHOLD) != inside)
                || (x + 1 < OSC_DIAGNOSTICS_FONT_CELL_WIDTH
                    && (cell_alpha(alpha, cell_x, cell_y, x + 1, y) >= EDGE_THRESHOLD) != inside)
                || (y > 0
                    && (cell_alpha(alpha, cell_x, cell_y, x, y - 1) >= EDGE_THRESHOLD) != inside)
                || (y + 1 < OSC_DIAGNOSTICS_FONT_CELL_HEIGHT
                    && (cell_alpha(alpha, cell_x, cell_y, x, y + 1) >= EDGE_THRESHOLD) != inside);
            if edge {
                edge_pixels.push((x as f32 + 0.5, y as f32 + 0.5));
            }
        }
    }

    for y in 0..OSC_DIAGNOSTICS_FONT_CELL_HEIGHT {
        for x in 0..OSC_DIAGNOSTICS_FONT_CELL_WIDTH {
            let atlas_index = (cell_y + y) * OSC_DIAGNOSTICS_FONT_ATLAS_WIDTH + cell_x + x;
            if edge_pixels.is_empty() {
                sdf[atlas_index] = if alpha[atlas_index] >= EDGE_THRESHOLD {
                    255
                } else {
                    0
                };
                continue;
            }
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let mut nearest_sq = f32::INFINITY;
            for (edge_x, edge_y) in &edge_pixels {
                let dx = px - edge_x;
                let dy = py - edge_y;
                nearest_sq = nearest_sq.min(dx * dx + dy * dy);
            }
            let signed_distance = nearest_sq.sqrt().min(SDF_SPREAD_PX)
                * if alpha[atlas_index] >= EDGE_THRESHOLD {
                    1.0
                } else {
                    -1.0
                };
            let encoded = 127.5 + signed_distance * (127.5 / SDF_SPREAD_PX);
            sdf[atlas_index] = encoded.round().clamp(0.0, 255.0) as u8;
        }
    }
}

fn cell_alpha(alpha: &[u8], cell_x: usize, cell_y: usize, x: usize, y: usize) -> u8 {
    alpha[(cell_y + y) * OSC_DIAGNOSTICS_FONT_ATLAS_WIDTH + cell_x + x]
}

fn copy_glyph_bitmap_to_atlas(
    atlas: &mut [u8],
    glyph_x: i32,
    glyph_y: i32,
    glyph_width: usize,
    bitmap: &[u8],
) {
    if glyph_width == 0 {
        return;
    }
    for (index, alpha) in bitmap.iter().copied().enumerate() {
        if alpha == 0 {
            continue;
        }
        let src_x = index % glyph_width;
        let src_y = index / glyph_width;
        let dst_x = glyph_x + src_x as i32;
        let dst_y = glyph_y + src_y as i32;
        if dst_x < 0
            || dst_y < 0
            || dst_x >= OSC_DIAGNOSTICS_FONT_ATLAS_WIDTH as i32
            || dst_y >= OSC_DIAGNOSTICS_FONT_ATLAS_HEIGHT as i32
        {
            continue;
        }
        let dst = dst_y as usize * OSC_DIAGNOSTICS_FONT_ATLAS_WIDTH + dst_x as usize;
        atlas[dst] = atlas[dst].max(alpha);
    }
}

fn find_glslc() -> Option<PathBuf> {
    if let Ok(path) = env::var("GLSLC") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(path) = find_on_path("glslc") {
        return Some(path);
    }
    if let Ok(path) = find_on_path("glslc.exe") {
        return Some(path);
    }

    for env_name in ["ANDROID_NDK_ROOT", "ANDROID_NDK_HOME"] {
        if let Ok(root) = env::var(env_name) {
            let candidate = PathBuf::from(root).join("shader-tools/windows-x86_64/glslc.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn find_on_path(file_name: &str) -> Result<PathBuf, ()> {
    let path = env::var_os("PATH").ok_or(())?;
    for entry in env::split_paths(&path) {
        let candidate = entry.join(file_name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(())
}

fn compile_shader(glslc: &Path, stage: &str, source: &Path, output: &Path, defines: &[&str]) {
    let mut command = Command::new(glslc);
    command
        .arg("--target-env=vulkan1.1")
        .arg(format!("-fshader-stage={stage}"));
    for define in defines {
        command.arg(format!("-D{define}"));
    }
    let status = command
        .arg(source)
        .arg("-o")
        .arg(output)
        .status()
        .unwrap_or_else(|error| panic!("failed to run glslc at {}: {error}", glslc.display()));

    if !status.success() {
        panic!(
            "glslc failed for {} with status {}",
            source.display(),
            status
        );
    }
}
