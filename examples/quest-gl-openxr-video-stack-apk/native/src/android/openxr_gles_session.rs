use openxr as xr;
use openxr::sys::Handle as _;
use rusty_xr_quest_diagnostics::{
    FrameRateSummary, OpenXrGlesFeasibilityState, OpenXrGlesFeasibilityStatus,
};
use std::{ffi::CString, os::raw::c_char, ptr, time::Instant};

use super::{ensure_xr_success, log_error, log_info, log_status, VIEW_TYPE};

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

pub(super) fn view_pose_is_submit_valid(view: &xr::View) -> bool {
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
