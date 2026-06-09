use jni::{objects::JObject, sys::jobject, JavaVM};
use rusty_quest_projection_runtime_config as rxrc;

use super::openxr_gles_config::{
    activity_string_extra, android_system_property_value,
    OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_EXTRA,
    OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_PROP,
};

pub(super) fn oes_current_android_projection_property_values() -> Vec<(&'static str, String)> {
    rxrc::PROJECTION_RUNTIME_KEY_INPUTS
        .iter()
        .filter(|input| input.source == rxrc::RuntimeKeyInputSource::AndroidProperty)
        .filter_map(|input| {
            android_system_property_value(input.input).map(|value| (input.input, value))
        })
        .collect()
}

pub(super) fn oes_projection_runtime_resolution_enabled(
    app: &android_activity::AndroidApp,
) -> bool {
    if let Some(value) =
        android_system_property_value(OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_PROP)
            .and_then(|value| oes_projection_runtime_bool(&value))
    {
        return value;
    }
    activity_bool_extra(app, OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_EXTRA).unwrap_or(false)
}

fn activity_bool_extra(app: &android_activity::AndroidApp, key: &str) -> Option<bool> {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return None;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return None;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, key)
        .as_deref()
        .and_then(oes_projection_runtime_bool)
}

fn oes_projection_runtime_bool(value: &str) -> Option<bool> {
    rxrc::RuntimeValue::parse_typed(value).as_bool()
}
