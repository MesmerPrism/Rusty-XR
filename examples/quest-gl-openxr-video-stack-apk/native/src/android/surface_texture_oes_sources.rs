use jni::{
    objects::{GlobalRef, JClass, JObject, JValue},
    sys::jobject,
    JNIEnv,
};

use super::VIEW_COUNT;

pub(super) const BROKER_H264_DEFAULT_HOST: &str = "127.0.0.1";
pub(super) const BROKER_H264_LEFT_STREAM_PORT: i32 = 8879;
pub(super) const BROKER_H264_RIGHT_STREAM_PORT: i32 = 8880;
pub(super) const BROKER_H264_MAX_PACKETS: i32 = 0;
pub(super) const BROKER_H264_CONNECT_TIMEOUT_MS: i32 = 5000;
pub(super) const BROKER_H264_DECODE_TIMEOUT_MS: i32 = 0;

pub(super) fn start_broker_h264_oes_decode_probe(
    env: &mut JNIEnv<'_>,
    app: &android_activity::AndroidApp,
    output_surfaces: &[GlobalRef],
    surface_textures: &[GlobalRef],
) -> Result<GlobalRef, String> {
    if output_surfaces.len() < VIEW_COUNT || surface_textures.len() < VIEW_COUNT {
        return Err(format!(
            "broker H.264 OES decode requires {VIEW_COUNT} output surfaces and SurfaceTextures"
        ));
    }
    let host = env
        .new_string(BROKER_H264_DEFAULT_HOST)
        .map_err(|error| jni_error(env, "create broker H.264 host string", error))?;
    let host_object = JObject::from(host);
    let class_name = env
        .new_string("com.example.rustyxr.opengles.BrokerH264OesDecodeProbe")
        .map_err(|error| jni_error(env, "create broker H.264 helper class string", error))?;
    let class_name_object = JObject::from(class_name);
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let class_loader = env
        .call_method(
            &activity,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "read Activity class loader", error))?;
    let helper_class_object = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name_object)],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "load broker H.264 OES helper class", error))?;
    let helper_class = JClass::from(helper_class_object);
    let probe = env
        .call_static_method(
            &helper_class,
            "start",
            "(Landroid/app/Activity;Ljava/lang/String;IILandroid/view/Surface;Landroid/view/Surface;Landroid/graphics/SurfaceTexture;Landroid/graphics/SurfaceTexture;III)Lcom/example/rustyxr/opengles/BrokerH264OesDecodeProbe;",
            &[
                JValue::Object(&activity),
                JValue::Object(&host_object),
                JValue::Int(BROKER_H264_LEFT_STREAM_PORT),
                JValue::Int(BROKER_H264_RIGHT_STREAM_PORT),
                JValue::Object(output_surfaces[0].as_obj()),
                JValue::Object(output_surfaces[1].as_obj()),
                JValue::Object(surface_textures[0].as_obj()),
                JValue::Object(surface_textures[1].as_obj()),
                JValue::Int(BROKER_H264_MAX_PACKETS),
                JValue::Int(BROKER_H264_CONNECT_TIMEOUT_MS),
                JValue::Int(BROKER_H264_DECODE_TIMEOUT_MS),
            ],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "start Java broker H.264 OES decode probe", error))?;
    if probe.is_null() {
        return Err("Java broker H.264 OES decode probe returned null".to_string());
    }
    env.new_global_ref(&probe).map_err(|error| {
        jni_error(
            env,
            "promote broker H.264 OES decode probe reference",
            error,
        )
    })
}

pub(super) fn start_direct_camera2_oes_probe(
    env: &mut JNIEnv<'_>,
    app: &android_activity::AndroidApp,
    output_surfaces: &[GlobalRef],
    surface_textures: &[GlobalRef],
    surface_width: i32,
    surface_height: i32,
) -> Result<GlobalRef, String> {
    if output_surfaces.len() < VIEW_COUNT || surface_textures.len() < VIEW_COUNT {
        return Err(format!(
            "direct Camera2 OES probe requires {VIEW_COUNT} output surfaces and SurfaceTextures"
        ));
    }
    let class_name = env
        .new_string("com.example.rustyxr.opengles.DirectCamera2OesProbe")
        .map_err(|error| jni_error(env, "create direct Camera2 OES helper class string", error))?;
    let class_name_object = JObject::from(class_name);
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let class_loader = env
        .call_method(
            &activity,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "read Activity class loader", error))?;
    let helper_class_object = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name_object)],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "load direct Camera2 OES helper class", error))?;
    let helper_class = JClass::from(helper_class_object);
    let probe = env
        .call_static_method(
            &helper_class,
            "start",
            "(Landroid/app/Activity;Landroid/view/Surface;Landroid/view/Surface;Landroid/graphics/SurfaceTexture;Landroid/graphics/SurfaceTexture;III)Lcom/example/rustyxr/opengles/DirectCamera2OesProbe;",
            &[
                JValue::Object(&activity),
                JValue::Object(output_surfaces[0].as_obj()),
                JValue::Object(output_surfaces[1].as_obj()),
                JValue::Object(surface_textures[0].as_obj()),
                JValue::Object(surface_textures[1].as_obj()),
                JValue::Int(surface_width),
                JValue::Int(surface_height),
                JValue::Int(50),
            ],
        )
        .and_then(|value| value.l())
        .map_err(|error| jni_error(env, "start Java direct Camera2 OES probe", error))?;
    if probe.is_null() {
        return Err("Java direct Camera2 OES probe returned null".to_string());
    }
    env.new_global_ref(&probe)
        .map_err(|error| jni_error(env, "promote direct Camera2 OES probe reference", error))
}

fn jni_error(env: &mut JNIEnv<'_>, context: &str, error: impl std::fmt::Display) -> String {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    format!("{context}: {error}")
}
