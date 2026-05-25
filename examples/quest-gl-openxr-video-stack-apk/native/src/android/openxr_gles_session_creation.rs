use openxr as xr;
use openxr::sys::Handle as _;
use rusty_xr_quest_diagnostics::{
    OpenXrGlesExtensionStatus, OpenXrGlesFeasibilityState, OpenXrGlesFeasibilityStatus,
};
use std::{ffi::CString, os::raw::c_char, ptr};

use super::{egl_gles_context::EglContext, ensure_xr_success, log_error, log_info, log_status};

pub(super) fn select_openxr_gles_extensions(
    entry: &xr::Entry,
    native_passthrough_underlay_requested: bool,
    status: &mut OpenXrGlesFeasibilityStatus,
) -> Result<xr::ExtensionSet, String> {
    let available_extensions = entry
        .enumerate_extensions()
        .map_err(|error| format!("enumerate OpenXR extensions: {error}"))?;
    status.state = OpenXrGlesFeasibilityState::ExtensionsEnumerated;
    status.required_extensions[0].available = available_extensions.khr_opengl_es_enable;
    status.required_extensions.push(
        OpenXrGlesExtensionStatus::optional("XR_FB_passthrough")
            .with_available(available_extensions.fb_passthrough),
    );
    log_info(format!(
        "Rusty XR OpenXR GLES extensions androidCreateInstance={} openGles={} fbPassthrough={} displayRefresh={}",
        available_extensions.khr_android_create_instance,
        available_extensions.khr_opengl_es_enable,
        available_extensions.fb_passthrough,
        available_extensions.fb_display_refresh_rate
    ));
    log_status(status);

    if !available_extensions.khr_opengl_es_enable {
        status.state = OpenXrGlesFeasibilityState::Failed;
        status
            .issue_codes
            .push(String::from("missing.XR_KHR_opengl_es_enable"));
        log_status(status);
        return Err("OpenXR runtime does not expose XR_KHR_opengl_es_enable".to_string());
    }

    let mut enabled_extensions = xr::ExtensionSet::default();
    enabled_extensions.khr_android_create_instance = true;
    enabled_extensions.khr_opengl_es_enable = true;
    if native_passthrough_underlay_requested && available_extensions.fb_passthrough {
        enabled_extensions.fb_passthrough = true;
    } else if native_passthrough_underlay_requested {
        status
            .issue_codes
            .push(String::from("missing.XR_FB_passthrough"));
        status.notes.push(String::from(
            "native passthrough underlay was requested, but XR_FB_passthrough was not available before instance creation",
        ));
        log_error(
            "Rusty XR OpenXR GLES native passthrough underlay requested but XR_FB_passthrough is unavailable",
        );
    }

    Ok(enabled_extensions)
}

pub(super) fn record_openxr_runtime_properties(
    instance: &xr::Instance,
    status: &mut OpenXrGlesFeasibilityStatus,
) -> Result<(), String> {
    let properties = instance
        .properties()
        .map_err(|error| format!("read OpenXR properties: {error}"))?;
    status.runtime_name = Some(properties.runtime_name.clone());
    status.runtime_version = Some(properties.runtime_version.to_string());
    log_info(format!(
        "Rusty XR OpenXR GLES runtime name={} version={}",
        properties.runtime_name, properties.runtime_version
    ));
    Ok(())
}

pub(super) fn create_openxr_gles_session(
    instance: &xr::Instance,
    system: xr::SystemId,
    egl: &EglContext,
    status: &mut OpenXrGlesFeasibilityStatus,
) -> Result<
    (
        xr::Session<xr::OpenGlEs>,
        xr::FrameWaiter,
        xr::FrameStream<xr::OpenGlEs>,
    ),
    String,
> {
    let session_parts = unsafe {
        instance
            .create_session::<xr::OpenGlEs>(
                system,
                &xr::opengles::SessionCreateInfo::Android {
                    display: egl.display,
                    config: egl.config,
                    context: egl.context,
                },
            )
            .map_err(|error| format!("create OpenXR GLES session: {error}"))?
    };
    status.state = OpenXrGlesFeasibilityState::SessionReady;
    log_status(status);
    Ok(session_parts)
}

pub(super) fn initialize_android_loader(
    entry: &xr::Entry,
    app: &android_activity::AndroidApp,
) -> Result<(), String> {
    let loader_init = unsafe { xr::raw::LoaderInitKHR::load(entry, xr::sys::Instance::NULL) }
        .map_err(|error| format!("load Android OpenXR loader init: {error}"))?;
    let loader_info = xr::sys::LoaderInitInfoAndroidKHR {
        ty: xr::sys::LoaderInitInfoAndroidKHR::TYPE,
        next: ptr::null(),
        application_vm: app.vm_as_ptr(),
        application_context: app.activity_as_ptr(),
    };

    let result = unsafe { (loader_init.initialize_loader)(&loader_info as *const _ as _) };
    ensure_xr_success(result, "xrInitializeLoaderKHR")?;
    log_info("Rusty XR initialized Android OpenXR loader with Activity context");
    Ok(())
}

pub(super) unsafe fn create_android_instance(
    entry: &xr::Entry,
    app: &android_activity::AndroidApp,
    app_info: &xr::ApplicationInfo,
    required_extensions: &xr::ExtensionSet,
    layers: &[&str],
) -> Result<xr::Instance, String> {
    let extension_names = required_extensions.names();
    let extension_ptrs = extension_names
        .iter()
        .map(|name| name.as_ptr() as *const _)
        .collect::<Vec<_>>();
    let layer_names = layers
        .iter()
        .filter_map(|layer| CString::new(*layer).ok())
        .collect::<Vec<_>>();
    let layer_ptrs = layer_names
        .iter()
        .map(|layer| layer.as_ptr())
        .collect::<Vec<_>>();

    let android_info = xr::sys::InstanceCreateInfoAndroidKHR {
        ty: xr::sys::InstanceCreateInfoAndroidKHR::TYPE,
        next: ptr::null(),
        application_vm: app.vm_as_ptr(),
        application_activity: app.activity_as_ptr(),
    };
    let mut info = xr::sys::InstanceCreateInfo {
        ty: xr::sys::InstanceCreateInfo::TYPE,
        next: if required_extensions.khr_android_create_instance {
            &android_info as *const _ as _
        } else {
            ptr::null()
        },
        create_flags: Default::default(),
        application_info: xr::sys::ApplicationInfo {
            application_name: [0; xr::sys::MAX_APPLICATION_NAME_SIZE],
            application_version: app_info.application_version,
            engine_name: [0; xr::sys::MAX_ENGINE_NAME_SIZE],
            engine_version: app_info.engine_version,
            api_version: app_info.api_version,
        },
        enabled_api_layer_count: layer_ptrs.len() as _,
        enabled_api_layer_names: layer_ptrs.as_ptr(),
        enabled_extension_count: extension_ptrs.len() as _,
        enabled_extension_names: extension_ptrs.as_ptr(),
    };
    write_xr_string(
        &mut info.application_info.application_name,
        app_info.application_name,
    );
    write_xr_string(&mut info.application_info.engine_name, app_info.engine_name);

    let mut handle = xr::sys::Instance::NULL;
    let result = (entry.fp().create_instance)(&info, &mut handle);
    ensure_xr_success(result, "xrCreateInstance")?;

    let extensions = xr::InstanceExtensions::load(entry, handle, required_extensions)
        .map_err(|error| format!("load OpenXR instance extensions: {error}"))?;
    xr::Instance::from_raw(entry.clone(), handle, extensions)
        .map_err(|error| format!("wrap OpenXR instance: {error}"))
}

fn write_xr_string<const N: usize>(destination: &mut [c_char; N], value: &str) {
    for (slot, byte) in destination.iter_mut().zip(value.bytes()) {
        *slot = byte as _;
    }
}
