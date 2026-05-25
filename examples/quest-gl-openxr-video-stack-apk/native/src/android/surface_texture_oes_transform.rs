use super::log_info;
use jni::{
    objects::{JObject, JValue},
    JNIEnv, JavaVM,
};

pub(super) fn sample_surface_texture_transform_matrix(
    env: &mut JNIEnv<'_>,
    surface_texture: &JObject<'_>,
) -> Result<[f32; 16], String> {
    let transform_array = env
        .new_float_array(16)
        .map_err(|error| format!("allocate SurfaceTexture transform matrix array: {error}"))?;
    env.call_method(
        surface_texture,
        "getTransformMatrix",
        "([F)V",
        &[JValue::Object(&transform_array)],
    )
    .map_err(|error| format!("get SurfaceTexture transform matrix: {error}"))?;
    let mut transform = [0.0_f32; 16];
    env.get_float_array_region(&transform_array, 0, &mut transform)
        .map_err(|error| format!("read SurfaceTexture transform matrix: {error}"))?;
    Ok(transform)
}

pub(super) fn transform_matrix_hash(transform: &[f32; 16]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in transform {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("m44:fnv1a64:{hash:016x}")
}

pub(super) fn android_elapsed_realtime_nanos(java_vm: &JavaVM) -> Option<i64> {
    let mut env = java_vm.attach_current_thread().ok()?;
    env.call_static_method("android/os/SystemClock", "elapsedRealtimeNanos", "()J", &[])
        .ok()?
        .j()
        .ok()
}

pub(super) fn log_surface_texture_transform_matrix(
    view_index: usize,
    source_eye: Option<&str>,
    update_tex_image_count: u64,
    timestamp_ns: i64,
    transform_hash: &str,
    transform_matrix: &[f32; 16],
) {
    let payload = serde_json::json!({
        "schema": "rusty.xr.quest.surface_texture_oes_transform_matrix.v1",
        "view_index": view_index,
        "source_eye": source_eye,
        "update_tex_image_count": update_tex_image_count,
        "surface_texture_timestamp_ns": timestamp_ns,
        "transform_matrix_hash": transform_hash,
        "transform_matrix": transform_matrix,
    });
    log_info(format!(
        "Rusty XR SurfaceTexture OES transform matrix {payload}"
    ));
}
