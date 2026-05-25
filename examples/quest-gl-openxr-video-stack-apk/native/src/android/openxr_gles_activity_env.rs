use jni::{objects::JObject, sys::jobject, JNIEnv, JavaVM};

use super::openxr_gles_config::activity_string_extra;

pub(super) fn with_activity_env<R>(
    app: &android_activity::AndroidApp,
    fallback: R,
    read: impl FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> R,
) -> R {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return fallback;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return fallback;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    read(&mut env, &activity)
}

pub(super) fn activity_float_extra(
    env: &mut JNIEnv<'_>,
    activity: &JObject<'_>,
    keys: &[&str],
) -> Option<f32> {
    keys.iter()
        .find_map(|key| activity_string_extra(env, activity, key))
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
}
