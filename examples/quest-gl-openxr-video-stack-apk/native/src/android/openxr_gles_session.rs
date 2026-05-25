use openxr as xr;
use openxr::sys::Handle as _;
use rusty_xr_quest_diagnostics::{
    FrameRateSummary, OpenXrGlesExtensionStatus, OpenXrGlesFeasibilityState,
    OpenXrGlesFeasibilityStatus,
};
use std::{ffi::CString, os::raw::c_char, ptr, time::Instant};

use super::egl_gles_context::EglContext;
use super::openxr_gles_passthrough::OpenXrGlesPassthroughUnderlay;
use super::{ensure_xr_success, log_error, log_info, log_status, VIEW_TYPE};

pub(super) enum OesLocatedViews {
    SubmitValid(Vec<xr::View>),
    NotSubmitValid(xr::ViewStateFlags),
}

pub(super) struct OesFrameRateTracker {
    frame_window_start: Instant,
    frame_window_count: u64,
}

impl OesFrameRateTracker {
    pub(super) fn new() -> Self {
        Self {
            frame_window_start: Instant::now(),
            frame_window_count: 0,
        }
    }

    pub(super) fn record_rendered_frame(
        &mut self,
        frame_count: u64,
        status: &mut OpenXrGlesFeasibilityStatus,
    ) {
        self.frame_window_count = self.frame_window_count.saturating_add(1);
        if status.state != OpenXrGlesFeasibilityState::Rendering && frame_count > 0 {
            status.state = OpenXrGlesFeasibilityState::Rendering;
        }
        if frame_count == 1 || frame_count.is_multiple_of(120) {
            let elapsed = self.frame_window_start.elapsed().as_secs_f32().max(0.001);
            let fps = self.frame_window_count as f32 / elapsed;
            status.frame_rate = Some(FrameRateSummary {
                sample_count: frame_count,
                average_fps: fps,
                min_fps: fps,
                max_fps: fps,
            });
            log_info(format!(
                "Rusty XR OpenXR GLES frame frame={} observedOpenXrFps={:.1} iteration2Ready={}",
                frame_count,
                fps,
                status.is_iteration2_ready()
            ));
            log_status(status);
            self.frame_window_start = Instant::now();
            self.frame_window_count = 0;
        }
    }
}

pub(super) fn poll_openxr_session_events(
    instance: &xr::Instance,
    session: &xr::Session<xr::OpenGlEs>,
    event_storage: &mut xr::EventDataBuffer,
    session_running: &mut bool,
) -> Result<bool, String> {
    while let Some(event) = instance
        .poll_event(event_storage)
        .map_err(|error| format!("poll OpenXR event: {error}"))?
    {
        match event {
            xr::Event::SessionStateChanged(event) => match event.state() {
                xr::SessionState::READY => {
                    session
                        .begin(VIEW_TYPE)
                        .map_err(|error| format!("begin OpenXR session: {error}"))?;
                    *session_running = true;
                    log_info("Rusty XR OpenXR GLES state READY -> running");
                }
                xr::SessionState::STOPPING => {
                    session
                        .end()
                        .map_err(|error| format!("end OpenXR session: {error}"))?;
                    *session_running = false;
                    log_info("Rusty XR OpenXR GLES state STOPPING -> ended");
                }
                xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                    return Ok(true);
                }
                state => {
                    log_info(format!("Rusty XR OpenXR GLES state {state:?}"));
                }
            },
            xr::Event::InstanceLossPending(_) => return Ok(true),
            xr::Event::EventsLost(event) => {
                log_error(format!(
                    "Rusty XR OpenXR GLES lost {} event(s)",
                    event.lost_event_count()
                ));
            }
            _ => {}
        }
    }
    Ok(false)
}

pub(super) fn request_session_exit_if_app_stopped(
    app_running: bool,
    session: &xr::Session<xr::OpenGlEs>,
) {
    if app_running {
        return;
    }
    match session.request_exit() {
        Ok(()) | Err(xr::sys::Result::ERROR_SESSION_NOT_RUNNING) => {}
        Err(error) => log_error(format!("Rusty XR OpenXR GLES request_exit failed: {error}")),
    }
}

pub(super) fn begin_openxr_frame(
    frame_wait: &mut xr::FrameWaiter,
    frame_stream: &mut xr::FrameStream<xr::OpenGlEs>,
) -> Result<xr::FrameState, String> {
    let frame_state = frame_wait
        .wait()
        .map_err(|error| format!("wait OpenXR frame: {error}"))?;
    frame_stream
        .begin()
        .map_err(|error| format!("begin OpenXR frame: {error}"))?;
    Ok(frame_state)
}

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

pub(super) fn end_empty_openxr_frame(
    frame_stream: &mut xr::FrameStream<xr::OpenGlEs>,
    predicted_display_time: xr::Time,
    environment_blend_mode: xr::EnvironmentBlendMode,
    operation: &str,
) -> Result<(), String> {
    frame_stream
        .end(predicted_display_time, environment_blend_mode, &[])
        .map_err(|error| format!("{operation}: {error}"))
}

pub(super) fn end_projection_openxr_frame(
    frame_stream: &mut xr::FrameStream<xr::OpenGlEs>,
    predicted_display_time: xr::Time,
    environment_blend_mode: xr::EnvironmentBlendMode,
    stage: &xr::Space,
    projection_views: &[xr::CompositionLayerProjectionView<'_, xr::OpenGlEs>],
    projection_uses_source_alpha: bool,
    native_passthrough_underlay: Option<&OpenXrGlesPassthroughUnderlay>,
) -> Result<(), String> {
    let layer = xr::CompositionLayerProjection::new()
        .layer_flags(if projection_uses_source_alpha {
            xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA
        } else {
            xr::CompositionLayerFlags::EMPTY
        })
        .space(stage)
        .views(projection_views);
    let passthrough_layer =
        native_passthrough_underlay.map(|underlay| xr::sys::CompositionLayerPassthroughFB {
            ty: xr::sys::CompositionLayerPassthroughFB::TYPE,
            next: ptr::null(),
            flags: xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
            space: xr::sys::Space::NULL,
            layer_handle: underlay.layer,
        });
    let mut layers: Vec<&xr::CompositionLayerBase<xr::OpenGlEs>> =
        Vec::with_capacity(1 + usize::from(passthrough_layer.is_some()));
    if let Some(passthrough_layer) = passthrough_layer.as_ref() {
        let layer_base: &xr::CompositionLayerBase<xr::OpenGlEs> = unsafe {
            &*(passthrough_layer as *const xr::sys::CompositionLayerPassthroughFB
                as *const xr::CompositionLayerBase<xr::OpenGlEs>)
        };
        layers.push(layer_base);
    }
    layers.push(&layer);
    frame_stream
        .end(predicted_display_time, environment_blend_mode, &layers)
        .map_err(|error| format!("end OpenXR frame: {error}"))
}

fn view_pose_is_submit_valid(view: &xr::View) -> bool {
    let pose = view.pose;
    let values = [
        pose.position.x,
        pose.position.y,
        pose.position.z,
        pose.orientation.x,
        pose.orientation.y,
        pose.orientation.z,
        pose.orientation.w,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let orientation_norm_squared = pose.orientation.x * pose.orientation.x
        + pose.orientation.y * pose.orientation.y
        + pose.orientation.z * pose.orientation.z
        + pose.orientation.w * pose.orientation.w;
    orientation_norm_squared.is_finite() && orientation_norm_squared > 0.0
}

pub(super) fn locate_submit_valid_views(
    session: &xr::Session<xr::OpenGlEs>,
    predicted_display_time: xr::Time,
    stage: &xr::Space,
) -> Result<OesLocatedViews, String> {
    let (view_state_flags, views) = session
        .locate_views(VIEW_TYPE, predicted_display_time, stage)
        .map_err(|error| format!("locate OpenXR views: {error}"))?;
    let views_valid = view_state_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
        && view_state_flags.contains(xr::ViewStateFlags::POSITION_VALID)
        && views.iter().all(view_pose_is_submit_valid);
    if views_valid {
        Ok(OesLocatedViews::SubmitValid(views))
    } else {
        Ok(OesLocatedViews::NotSubmitValid(view_state_flags))
    }
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
