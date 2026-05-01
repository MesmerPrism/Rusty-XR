use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=shaders/camera_projection.vert.glsl");
    println!("cargo:rerun-if-changed=shaders/camera_projection.frag.glsl");
    println!("cargo:rerun-if-changed=shaders/environment_depth_visualization.frag.glsl");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }

    let glslc = find_glslc().unwrap_or_else(|| {
        panic!(
            "Android shader build needs glslc. Put glslc on PATH, set GLSLC, or set ANDROID_NDK_ROOT/ANDROID_NDK_HOME."
        )
    });
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));

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
