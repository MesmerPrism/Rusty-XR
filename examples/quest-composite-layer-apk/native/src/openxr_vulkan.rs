use std::{
    ffi::{CStr, CString},
    ptr,
    time::{Duration, Instant},
};

use crate::{
    gpu_probe_counters, latest_headset_camera_frame, latest_headset_camera_gpu_frame,
    latest_headset_stereo_camera_gpu_frame, log_error, log_info, runtime_config,
    HeadsetCameraFrame, HeadsetCameraGpuFrame, OpenXrColorFormatMode, OpenXrPassthroughProbeMode,
    StereoGpuCameraFrame,
};
use android_activity::{InputStatus, MainEvent, PollEvent};
use ash::vk::{self, Handle};
use openxr as xr;
use openxr::sys::Handle as _;
use rusty_xr_camera_model::{
    camera_basis_from_camera2_reference_pose_relative_to_center, full_view_content_uv_scale,
    head_anchored_preview_surface_corners, invert_homography, project_camera_point,
    scale_intrinsics_to_image, screen_to_camera_uv_homography, surface_to_camera_uv_homography,
    surface_to_eye_screen_uv_homography, CameraBasis, CameraCompositeTier, CameraPixelDomain,
    ImageSize, Quat, TrackingBasis, Vec3,
};

const CAMERA_CPU_COPY_MAX_DIMENSION: u32 = 640;
const CAMERA_CPU_UPLOAD_MIN_INTERVAL_NS: i64 = 250_000_000;
const CAMERA_CPU_UPLOAD_HZ_LABEL: u32 = 4;
const XR_RENDER_SCALE_DEFAULT: f32 = 0.75;
const GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = crate::CAMERA_IMPORT_CACHE_LIMIT_MAX;
const GPU_CAMERA_PROJECTION_UNIFORM_SLOTS: u32 = 3;
const XR_FRAGMENT_DENSITY_MAP_FORMAT: vk::Format = vk::Format::R8G8_UNORM;
const XR_FOVEATION_DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

fn effective_camera_import_cache_limit(limit: usize) -> usize {
    limit.clamp(2, GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX)
}

pub fn run(app: android_activity::AndroidApp) -> Result<(), String> {
    let entry = unsafe { xr::Entry::load().map_err(|error| format!("load OpenXR: {error}"))? };
    initialize_android_loader(&entry, &app)?;

    let available_extensions = entry
        .enumerate_extensions()
        .map_err(|error| format!("enumerate OpenXR extensions: {error}"))?;
    if !available_extensions.khr_vulkan_enable2 {
        return Err("OpenXR runtime does not expose XR_KHR_vulkan_enable2".to_string());
    }

    let mut enabled_extensions = xr::ExtensionSet::default();
    enabled_extensions.khr_android_create_instance = true;
    enabled_extensions.khr_vulkan_enable2 = true;
    if available_extensions.fb_display_refresh_rate {
        enabled_extensions.fb_display_refresh_rate = true;
    }
    let passthrough_probe_requested = runtime_config().openxr_passthrough_probe.enabled();
    if available_extensions.fb_passthrough && passthrough_probe_requested {
        enabled_extensions.fb_passthrough = true;
    } else if passthrough_probe_requested {
        log_info("Rusty XR OpenXR passthrough extension unavailable".to_string());
    }
    if available_extensions.fb_swapchain_update_state
        && available_extensions.fb_foveation
        && available_extensions.fb_foveation_configuration
        && available_extensions.fb_foveation_vulkan
    {
        enabled_extensions.fb_swapchain_update_state = true;
        enabled_extensions.fb_foveation = true;
        enabled_extensions.fb_foveation_configuration = true;
        enabled_extensions.fb_foveation_vulkan = true;
    } else {
        log_info(format!(
            "Rusty XR OpenXR fixed foveation extensions unavailable swapchainUpdate={} foveation={} foveationConfig={} foveationVulkan={}",
            available_extensions.fb_swapchain_update_state,
            available_extensions.fb_foveation,
            available_extensions.fb_foveation_configuration,
            available_extensions.fb_foveation_vulkan
        ));
    }

    let xr_instance = unsafe {
        create_android_instance(
            &entry,
            &app,
            &xr::ApplicationInfo {
                application_name: "Rusty XR Composite Layer",
                application_version: 1,
                engine_name: "Rusty XR",
                engine_version: 1,
                api_version: xr::Version::new(1, 0, 0),
            },
            &enabled_extensions,
            &[],
        )
    }?;

    let properties = xr_instance
        .properties()
        .map_err(|error| format!("read OpenXR properties: {error}"))?;
    log_info(format!(
        "Rusty XR OpenXR runtime: {} {}",
        properties.runtime_name, properties.runtime_version
    ));

    let system = xr_instance
        .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
        .map_err(|error| format!("get HMD system: {error}"))?;
    let environment_blend_mode = xr_instance
        .enumerate_environment_blend_modes(system, VIEW_TYPE)
        .map_err(|error| format!("enumerate environment blend modes: {error}"))?
        .into_iter()
        .next()
        .ok_or_else(|| "OpenXR runtime reported no environment blend modes".to_string())?;

    let vk_target_version = vk::make_api_version(0, 1, 1, 0);
    let vk_target_version_xr = xr::Version::new(1, 1, 0);
    let requirements = xr_instance
        .graphics_requirements::<xr::Vulkan>(system)
        .map_err(|error| format!("read Vulkan graphics requirements: {error}"))?;
    if vk_target_version_xr < requirements.min_api_version_supported
        || vk_target_version_xr.major() > requirements.max_api_version_supported.major()
    {
        return Err(format!(
            "OpenXR runtime requires Vulkan >= {} and < {}.0.0",
            requirements.min_api_version_supported,
            requirements.max_api_version_supported.major() + 1
        ));
    }

    unsafe {
        wait_for_android_foreground(&app)?;
        run_vulkan(
            app,
            xr_instance,
            system,
            environment_blend_mode,
            vk_target_version,
        )
    }
}

fn initialize_android_loader(
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

unsafe fn create_android_instance(
    entry: &xr::Entry,
    app: &android_activity::AndroidApp,
    app_info: &xr::ApplicationInfo,
    required_extensions: &xr::ExtensionSet,
    layers: &[&str],
) -> Result<xr::Instance, String> {
    if app_info.application_name.len() >= xr::sys::MAX_APPLICATION_NAME_SIZE {
        return Err(format!(
            "OpenXR application name must be shorter than {} bytes",
            xr::sys::MAX_APPLICATION_NAME_SIZE
        ));
    }
    if app_info.engine_name.len() >= xr::sys::MAX_ENGINE_NAME_SIZE {
        return Err(format!(
            "OpenXR engine name must be shorter than {} bytes",
            xr::sys::MAX_ENGINE_NAME_SIZE
        ));
    }

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

fn write_xr_string<const N: usize>(destination: &mut [std::os::raw::c_char; N], value: &str) {
    for (slot, byte) in destination.iter_mut().zip(value.bytes()) {
        *slot = byte as _;
    }
}

fn ensure_xr_success(result: xr::sys::Result, operation: &str) -> Result<(), String> {
    if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
        return Err(format!("{operation} failed: {result:?}"));
    }

    Ok(())
}

fn configure_display_refresh_rate<G>(instance: &xr::Instance, session: &xr::Session<G>) {
    const TARGET_DISPLAY_REFRESH_HZ: f32 = 72.0;

    if instance.exts().fb_display_refresh_rate.is_none() {
        log_info("Rusty XR OpenXR display refresh extension unavailable; using runtime default");
        return;
    }

    let supported = match session.enumerate_display_refresh_rates() {
        Ok(rates) => rates,
        Err(error) => {
            log_error(format!(
                "Rusty XR could not enumerate OpenXR display refresh rates: {error}"
            ));
            return;
        }
    };
    let current = session.get_display_refresh_rate().ok();
    let target = supported
        .iter()
        .copied()
        .find(|rate| (*rate - TARGET_DISPLAY_REFRESH_HZ).abs() <= 0.05);

    if let Some(target) = target {
        match session.request_display_refresh_rate(target) {
            Ok(()) => log_info(format!(
                "Rusty XR requested OpenXR display refresh {:.1}Hz current={} supported={}",
                target,
                refresh_rate_label(current),
                refresh_rate_list_label(&supported)
            )),
            Err(error) => log_error(format!(
                "Rusty XR could not request OpenXR display refresh {:.1}Hz current={} supported={} error={error}",
                target,
                refresh_rate_label(current),
                refresh_rate_list_label(&supported)
            )),
        }
    } else {
        log_error(format!(
            "Rusty XR OpenXR display refresh target {:.1}Hz is not in supported rates current={} supported={}",
            TARGET_DISPLAY_REFRESH_HZ,
            refresh_rate_label(current),
            refresh_rate_list_label(&supported)
        ));
    }
}

fn refresh_rate_label(value: Option<f32>) -> String {
    value
        .map(|rate| format!("{rate:.1}"))
        .unwrap_or_else(|| "unavailable".to_string())
}

fn refresh_rate_list_label(values: &[f32]) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }

    let mut label = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            label.push(',');
        }
        label.push_str(&format!("{value:.1}"));
    }
    label.push(']');
    label
}

struct OpenXrPassthroughProbe {
    mode: OpenXrPassthroughProbeMode,
    passthrough: xr::Passthrough,
    layer: xr::PassthroughLayerFB,
    start_frame: u64,
    paused: bool,
}

impl OpenXrPassthroughProbe {
    fn tick(&mut self, frame_count: u64) {
        if self.mode != OpenXrPassthroughProbeMode::Warmup || self.paused {
            return;
        }
        if frame_count.saturating_sub(self.start_frame) < 6 {
            return;
        }
        match self.passthrough.pause() {
            Ok(()) => {
                self.paused = true;
                log_info(format!(
                    "Rusty XR OpenXR passthrough probe paused mode={} afterFrames={}",
                    self.mode.stable_id(),
                    frame_count.saturating_sub(self.start_frame)
                ));
            }
            Err(error) => {
                self.paused = true;
                log_error(format!(
                    "Rusty XR OpenXR passthrough probe pause failed mode={} error={error}",
                    self.mode.stable_id()
                ));
            }
        }
    }

    fn submits_composition_layer(&self) -> bool {
        self.mode.submits_composition_layer() && !self.paused
    }
}

fn ensure_openxr_passthrough_probe<G: xr::Graphics>(
    instance: &xr::Instance,
    session: &xr::Session<G>,
    existing: Option<OpenXrPassthroughProbe>,
    mode: OpenXrPassthroughProbeMode,
    start_frame: u64,
) -> Option<OpenXrPassthroughProbe> {
    if let Some(existing) = existing {
        return Some(existing);
    }
    if !mode.enabled() {
        return None;
    }
    if instance.exts().fb_passthrough.is_none() {
        log_info(format!(
            "Rusty XR OpenXR passthrough probe requested mode={} but XR_FB_passthrough is unavailable",
            mode.stable_id()
        ));
        return None;
    }

    match create_openxr_passthrough_probe(session, mode, start_frame) {
        Ok(probe) => {
            log_info(format!(
                "Rusty XR OpenXR passthrough probe active mode={}",
                probe.mode.stable_id()
            ));
            Some(probe)
        }
        Err(error) => {
            log_error(format!(
                "Rusty XR OpenXR passthrough probe failed mode={} error={error}",
                mode.stable_id()
            ));
            None
        }
    }
}

fn create_openxr_passthrough_probe<G: xr::Graphics>(
    session: &xr::Session<G>,
    mode: OpenXrPassthroughProbeMode,
    start_frame: u64,
) -> Result<OpenXrPassthroughProbe, String> {
    let flags = xr::PassthroughFlagsFB::IS_RUNNING_AT_CREATION;
    let passthrough = session
        .create_passthrough(flags)
        .map_err(|error| format!("xrCreatePassthroughFB: {error}"))?;
    let layer = session
        .create_passthrough_layer(
            &passthrough,
            flags,
            xr::PassthroughLayerPurposeFB::RECONSTRUCTION,
        )
        .map_err(|error| format!("xrCreatePassthroughLayerFB: {error}"))?;
    layer
        .resume()
        .map_err(|error| format!("xrPassthroughLayerResumeFB: {error}"))?;
    Ok(OpenXrPassthroughProbe {
        mode,
        passthrough,
        layer,
        start_frame,
        paused: false,
    })
}

unsafe fn run_vulkan(
    app: android_activity::AndroidApp,
    xr_instance: xr::Instance,
    system: xr::SystemId,
    environment_blend_mode: xr::EnvironmentBlendMode,
    vk_target_version: u32,
) -> Result<(), String> {
    let vk_entry = ash::Entry::load().map_err(|error| format!("load Vulkan: {error}"))?;
    let vk_app_info = vk::ApplicationInfo::default()
        .application_version(1)
        .engine_version(1)
        .api_version(vk_target_version);

    let vk_instance = {
        let raw = xr_instance
            .create_vulkan_instance(
                system,
                std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                &vk::InstanceCreateInfo::default().application_info(&vk_app_info) as *const _
                    as *const _,
            )
            .map_err(|error| format!("OpenXR create Vulkan instance: {error}"))?
            .map_err(vk::Result::from_raw)
            .map_err(|error| format!("Vulkan create instance: {error}"))?;
        ash::Instance::load(vk_entry.static_fn(), vk::Instance::from_raw(raw as _))
    };

    let vk_physical_device = vk::PhysicalDevice::from_raw(
        xr_instance
            .vulkan_graphics_device(system, vk_instance.handle().as_raw() as _)
            .map_err(|error| format!("get OpenXR Vulkan graphics device: {error}"))? as _,
    );

    let properties = vk_instance.get_physical_device_properties(vk_physical_device);
    if properties.api_version < vk_target_version {
        vk_instance.destroy_instance(None);
        return Err("OpenXR-selected Vulkan device does not support Vulkan 1.1".to_string());
    }
    let memory_properties = vk_instance.get_physical_device_memory_properties(vk_physical_device);
    let ahb_extension_supported = physical_device_supports_extension(
        &vk_instance,
        vk_physical_device,
        ash::android::external_memory_android_hardware_buffer::NAME,
    )?;
    let sampler_ycbcr_extension_supported = physical_device_supports_extension(
        &vk_instance,
        vk_physical_device,
        ash::khr::sampler_ycbcr_conversion::NAME,
    )?;
    let fragment_density_map_supported =
        query_fragment_density_map_support(&vk_instance, vk_physical_device)?;
    let mut sampler_ycbcr_features = vk::PhysicalDeviceSamplerYcbcrConversionFeatures::default();
    let mut feature_query =
        vk::PhysicalDeviceFeatures2::default().push_next(&mut sampler_ycbcr_features);
    vk_instance.get_physical_device_features2(vk_physical_device, &mut feature_query);
    let sampler_ycbcr_supported = sampler_ycbcr_features.sampler_ycbcr_conversion == vk::TRUE;
    let gpu_camera_import_supported = ahb_extension_supported && sampler_ycbcr_supported;
    log_info(format!(
        "Rusty XR Vulkan camera import support androidHardwareBuffer={} samplerYcbcrFeature={} samplerYcbcrExtension={}",
        ahb_extension_supported,
        sampler_ycbcr_supported,
        sampler_ycbcr_extension_supported
    ));
    log_info(format!(
        "Rusty XR Vulkan fixed foveation support fragmentDensityMap={}",
        fragment_density_map_supported
    ));

    let queue_family_index = vk_instance
        .get_physical_device_queue_family_properties(vk_physical_device)
        .into_iter()
        .enumerate()
        .find_map(|(index, info)| {
            info.queue_flags
                .contains(vk::QueueFlags::GRAPHICS)
                .then_some(index as u32)
        })
        .ok_or_else(|| "OpenXR-selected Vulkan device has no graphics queue".to_string())?;

    let mut multiview_features = vk::PhysicalDeviceMultiviewFeatures {
        multiview: vk::TRUE,
        ..Default::default()
    };
    let mut sampler_ycbcr_enable = vk::PhysicalDeviceSamplerYcbcrConversionFeatures::default()
        .sampler_ycbcr_conversion(gpu_camera_import_supported);
    let mut device_extension_ptrs = Vec::new();
    if ahb_extension_supported {
        device_extension_ptrs
            .push(ash::android::external_memory_android_hardware_buffer::NAME.as_ptr());
    }
    if sampler_ycbcr_extension_supported {
        device_extension_ptrs.push(ash::khr::sampler_ycbcr_conversion::NAME.as_ptr());
    }
    if fragment_density_map_supported {
        device_extension_ptrs.push(ash::ext::fragment_density_map::NAME.as_ptr());
    }
    let queue_priorities = [1.0_f32];
    let queue_infos = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&queue_priorities)];
    let mut fragment_density_map_features =
        vk::PhysicalDeviceFragmentDensityMapFeaturesEXT::default()
            .fragment_density_map(fragment_density_map_supported);
    let mut device_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_extension_ptrs)
        .push_next(&mut sampler_ycbcr_enable)
        .push_next(&mut multiview_features);
    if fragment_density_map_supported {
        device_info = device_info.push_next(&mut fragment_density_map_features);
    }

    let vk_device = {
        let raw = xr_instance
            .create_vulkan_device(
                system,
                std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                vk_physical_device.as_raw() as _,
                &device_info as *const _ as *const _,
            )
            .map_err(|error| format!("OpenXR create Vulkan device: {error}"))?
            .map_err(vk::Result::from_raw)
            .map_err(|error| format!("Vulkan create device: {error}"))?;
        ash::Device::load(vk_instance.fp_v1_0(), vk::Device::from_raw(raw as _))
    };
    let queue = vk_device.get_device_queue(queue_family_index, 0);

    let startup_config = runtime_config();
    let fixed_foveation_render_path = startup_config.xr_fixed_foveation_level > 0
        && fragment_density_map_supported
        && xr_instance.exts().fb_swapchain_update_state.is_some()
        && xr_instance.exts().fb_foveation.is_some()
        && xr_instance.exts().fb_foveation_configuration.is_some()
        && xr_instance.exts().fb_foveation_vulkan.is_some();
    let xr_color_format_mode = startup_config.xr_color_format_mode;
    let xr_color_format = xr_color_format_mode.vk_format();
    let render_pass =
        create_openxr_render_pass(&vk_device, fixed_foveation_render_path, xr_color_format)?;
    log_info(format!(
        "Rusty XR OpenXR render pass fragmentDensityMap={} requestedFixedFoveationLevel={} xrColorFormat={} vkFormat={:?}",
        fixed_foveation_render_path,
        startup_config.xr_fixed_foveation_level,
        xr_color_format_mode.stable_id(),
        xr_color_format
    ));

    let (session, mut frame_wait, mut frame_stream) = xr_instance
        .create_session::<xr::Vulkan>(
            system,
            &xr::vulkan::SessionCreateInfo {
                instance: vk_instance.handle().as_raw() as _,
                physical_device: vk_physical_device.as_raw() as _,
                device: vk_device.handle().as_raw() as _,
                queue_family_index,
                queue_index: 0,
            },
        )
        .map_err(|error| format!("create OpenXR Vulkan session: {error}"))?;
    configure_display_refresh_rate(&xr_instance, &session);

    let reference_space = session
        .create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)
        .or_else(|_| {
            session.create_reference_space(xr::ReferenceSpaceType::LOCAL, xr::Posef::IDENTITY)
        })
        .map_err(|error| format!("create OpenXR reference space: {error}"))?;

    let cmd_pool = vk_device
        .create_command_pool(
            &vk::CommandPoolCreateInfo::default()
                .queue_family_index(queue_family_index)
                .flags(
                    vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                        | vk::CommandPoolCreateFlags::TRANSIENT,
                ),
            None,
        )
        .map_err(|error| format!("create Vulkan command pool: {error}"))?;
    let cmds = vk_device
        .allocate_command_buffers(
            &vk::CommandBufferAllocateInfo::default()
                .command_pool(cmd_pool)
                .command_buffer_count(PIPELINE_DEPTH),
        )
        .map_err(|error| format!("allocate Vulkan command buffers: {error}"))?;
    let fences = (0..PIPELINE_DEPTH)
        .map(|_| {
            vk_device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("create Vulkan fences: {error}"))?;

    let mut swapchain: Option<Swapchain> = None;
    let mut event_storage = xr::EventDataBuffer::new();
    let mut session_running = false;
    let mut session_focused = false;
    let mut app_running = true;
    let mut frame = 0_usize;
    let mut frame_count = 0_u64;
    let mut camera_upload: Option<CameraUpload> = None;
    let mut last_uploaded_camera_index: Option<u64> = None;
    let mut last_uploaded_camera_timestamp_ns: Option<i64> = None;
    let mut active_camera_copy: Option<CameraCopy> = None;
    let mut gpu_camera_renderer = GpuCameraRenderer::new(
        &vk_instance,
        &vk_device,
        memory_properties,
        properties.limits.min_uniform_buffer_offset_alignment,
        render_pass,
        gpu_camera_import_supported,
    );
    let mut last_logged_gpu_frame_index: Option<u64> = None;
    let mut last_logged_prepared_stereo_frame_index: Option<u64> = None;
    let mut openxr_passthrough_probe: Option<OpenXrPassthroughProbe> = None;
    let mut frame_pacing_window_start = Instant::now();
    let mut frame_pacing_window_frames = 0_u64;

    'main_loop: loop {
        pump_android_events(&app, &mut app_running);
        if !app_running {
            match session.request_exit() {
                Ok(()) | Err(xr::sys::Result::ERROR_SESSION_NOT_RUNNING) => {}
                Err(error) => log_error(format!("Rusty XR OpenXR request_exit failed: {error}")),
            }
        }

        while let Some(event) = xr_instance
            .poll_event(&mut event_storage)
            .map_err(|error| format!("poll OpenXR event: {error}"))?
        {
            match event {
                xr::Event::SessionStateChanged(event) => {
                    log_info(format!("Rusty XR OpenXR state {:?}", event.state()));
                    session_focused = event.state() == xr::SessionState::FOCUSED;
                    match event.state() {
                        xr::SessionState::READY => {
                            session
                                .begin(VIEW_TYPE)
                                .map_err(|error| format!("begin OpenXR session: {error}"))?;
                            session_running = true;
                            openxr_passthrough_probe = ensure_openxr_passthrough_probe(
                                &xr_instance,
                                &session,
                                openxr_passthrough_probe,
                                runtime_config().openxr_passthrough_probe,
                                frame_count,
                            );
                            frame_pacing_window_start = Instant::now();
                            frame_pacing_window_frames = 0;
                        }
                        xr::SessionState::STOPPING => {
                            session
                                .end()
                                .map_err(|error| format!("end OpenXR session: {error}"))?;
                            session_running = false;
                            openxr_passthrough_probe = None;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            break 'main_loop;
                        }
                        _ => {}
                    }
                }
                xr::Event::DisplayRefreshRateChangedFB(event) => {
                    log_info(format!(
                        "Rusty XR OpenXR display refresh changed from {:.1}Hz to {:.1}Hz",
                        event.from_display_refresh_rate(),
                        event.to_display_refresh_rate()
                    ));
                }
                xr::Event::InstanceLossPending(_) => break 'main_loop,
                xr::Event::EventsLost(event) => {
                    log_error(format!(
                        "Rusty XR OpenXR lost {} event(s)",
                        event.lost_event_count()
                    ));
                }
                _ => {}
            }
        }

        if !session_running {
            if !app_running {
                break 'main_loop;
            }
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        let frame_state = frame_wait
            .wait()
            .map_err(|error| format!("wait OpenXR frame: {error}"))?;
        frame_stream
            .begin()
            .map_err(|error| format!("begin OpenXR frame: {error}"))?;

        if !frame_state.should_render {
            frame_stream
                .end(
                    frame_state.predicted_display_time,
                    environment_blend_mode,
                    &[],
                )
                .map_err(|error| format!("end skipped OpenXR frame: {error}"))?;
            continue;
        }

        let swapchain = ensure_swapchain(
            &xr_instance,
            &session,
            system,
            &vk_device,
            &memory_properties,
            render_pass,
            xr_color_format_mode,
            xr_color_format,
            fixed_foveation_render_path,
            &mut swapchain,
        )?;
        let (view_state_flags, views) = session
            .locate_views(
                VIEW_TYPE,
                frame_state.predicted_display_time,
                &reference_space,
            )
            .map_err(|error| format!("locate OpenXR views: {error}"))?;
        if views.len() != VIEW_COUNT as usize {
            return Err(format!(
                "expected {VIEW_COUNT} OpenXR views, got {}",
                views.len()
            ));
        }
        let views_valid = view_state_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
            && view_state_flags.contains(xr::ViewStateFlags::POSITION_VALID);
        if !views_valid {
            if frame_count == 0 || frame_count % 120 == 0 {
                log_info(format!(
                    "Rusty XR skipped composition frame {} because OpenXR view pose is not valid yet flags={:?}",
                    frame_count, view_state_flags
                ));
            }
            frame_stream
                .end(
                    frame_state.predicted_display_time,
                    environment_blend_mode,
                    &[],
                )
                .map_err(|error| format!("end OpenXR frame without valid view pose: {error}"))?;
            frame_count += 1;
            continue;
        }
        let image_index = swapchain
            .handle
            .acquire_image()
            .map_err(|error| format!("acquire OpenXR swapchain image: {error}"))?;

        vk_device
            .wait_for_fences(&[fences[frame]], true, u64::MAX)
            .map_err(|error| format!("wait Vulkan fence: {error}"))?;
        vk_device
            .reset_fences(&[fences[frame]])
            .map_err(|error| format!("reset Vulkan fence: {error}"))?;

        let config = runtime_config();
        let record_started = Instant::now();
        if config.camera_tier == CameraCompositeTier::GpuProjected {
            if let Some(stereo_frame) = latest_headset_stereo_camera_gpu_frame() {
                if last_logged_gpu_frame_index != Some(stereo_frame.index)
                    && (stereo_frame.index == 0 || stereo_frame.index % 120 == 0)
                {
                    let pose_source = stereo_frame
                        .left
                        .diagnostics
                        .pose_source
                        .as_deref()
                        .unwrap_or("missing");
                    let pose_reference = stereo_frame
                        .left
                        .diagnostics
                        .lens_pose_reference_label
                        .as_deref()
                        .unwrap_or("unknown");
                    let pose_convention = stereo_frame
                        .left
                        .diagnostics
                        .pose_coordinate_convention
                        .as_deref()
                        .unwrap_or("unknown");
                    let projection_ready = stereo_frame.left.metadata.has_projection_metadata()
                        && stereo_frame.right.metadata.has_projection_metadata()
                        && matches!(pose_source, "platform" | "estimated-profile");
                    log_info(format!(
                        "Rusty XR stereo GPU projection candidate frame {} requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false stereoLayout=Separate pairedLeftRightGpuBuffers=true poseSource={} poseReference={} poseConvention={} leftCameraId={} rightCameraId={} pairDeltaNs={} midpointTs={} projectionMetadataReady={} fallbackReason={}",
                        stereo_frame.index,
                        config.camera_tier.stable_id(),
                        pose_source,
                        pose_reference,
                        pose_convention,
                        stereo_frame
                            .left
                            .metadata
                            .source
                            .physical_id
                            .as_deref()
                            .unwrap_or("unknown"),
                        stereo_frame
                            .right
                            .metadata
                            .source
                            .physical_id
                            .as_deref()
                            .unwrap_or("unknown"),
                        stereo_frame.pair_delta_ns,
                        stereo_frame.midpoint_timestamp_ns,
                        projection_ready,
                        if projection_ready {
                            "projection metadata present; waiting for projected shader status"
                        } else {
                            "paired stereo buffers are present but per-eye projection metadata is missing"
                        }
                    ));
                    last_logged_gpu_frame_index = Some(stereo_frame.index);
                }
            } else if frame_count == 0 || frame_count % 120 == 0 {
                let (success, failure, cache_size) = gpu_probe_counters();
                log_info(format!(
                    "Rusty XR GPU projected camera path requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false waiting for paired left/right Camera2 PRIVATE buffers; success={} failure={} descriptorProbeCacheSize={} allowCpuFallback=false",
                    config.camera_tier.stable_id(),
                    success,
                    failure,
                    cache_size
                ));
            }
        } else if config.camera_tier == CameraCompositeTier::GpuBufferProbe {
            if let Some(gpu_frame) = latest_headset_camera_gpu_frame() {
                if last_logged_gpu_frame_index != Some(gpu_frame.index)
                    && (gpu_frame.index == 0 || gpu_frame.index % 120 == 0)
                {
                    let (_, _, cache_size) = gpu_probe_counters();
                    log_info(format!(
                        "Rusty XR GPU camera buffer probe frame {} requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false source={} cameraId={} delivered={}x{} ts={} descriptor={} nativeFormat={} usage={} bufferId={} cacheSize={} stereoLayout={:?} requestedStereoLayout={} intrinsics={} pose={} poseSource={} fallbackReason={}",
                        gpu_frame.index,
                        config.camera_tier.stable_id(),
                        gpu_frame.metadata.source.label.as_str(),
                        gpu_frame
                            .metadata
                            .source
                            .physical_id
                            .as_deref()
                            .unwrap_or("unknown"),
                        gpu_frame.width,
                        gpu_frame.height,
                        gpu_frame.timestamp_ns,
                        gpu_frame.descriptor.format_label.as_str(),
                        gpu_frame
                            .descriptor
                            .native_format
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        gpu_frame
                            .descriptor
                            .usage_flags
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        gpu_frame
                            .descriptor
                            .buffer_id
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "unknown".to_string()),
                        cache_size,
                        gpu_frame.diagnostics.stereo_layout,
                        gpu_frame
                            .diagnostics
                            .requested_stereo_layout
                            .as_deref()
                            .unwrap_or("unknown"),
                        if gpu_frame.metadata.has_intrinsics() {
                            "available"
                        } else {
                            "missing"
                        },
                        if gpu_frame.metadata.has_pose() {
                            "available"
                        } else {
                            "missing"
                        },
                        gpu_frame
                            .diagnostics
                            .pose_source
                            .as_deref()
                            .unwrap_or("missing"),
                        gpu_frame.diagnostics.fallback_reason
                    ));
                    last_logged_gpu_frame_index = Some(gpu_frame.index);
                }
            } else if frame_count == 0 || frame_count % 120 == 0 {
                let (success, failure, cache_size) = gpu_probe_counters();
                log_info(format!(
                    "Rusty XR GPU camera path requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false waiting for Camera2 PRIVATE buffer; success={} failure={} descriptorProbeCacheSize={} allowCpuFallback={}",
                    config.camera_tier.stable_id(),
                    success,
                    failure,
                    cache_size,
                    config.allow_cpu_fallback
                ));
            }
        }

        let cpu_diagnostic_visible = config.camera_tier
            == CameraCompositeTier::CpuDiagnosticFlatCopy
            || (config.camera_tier == CameraCompositeTier::GpuBufferProbe
                && config.allow_cpu_fallback);

        if cpu_diagnostic_visible {
            if let Some(camera_frame) = latest_headset_camera_frame() {
                if last_uploaded_camera_index != Some(camera_frame.index) {
                    if should_throttle_camera_upload(
                        last_uploaded_camera_timestamp_ns,
                        camera_frame.timestamp_ns,
                    ) {
                        last_uploaded_camera_index = Some(camera_frame.index);
                    } else {
                        let copy_resolution = bounded_diagnostic_camera_copy_extent(
                            &camera_frame,
                            swapchain.resolution,
                        );
                        if let Some(flat_rgba) =
                            build_diagnostic_flat_camera_rgba(&camera_frame, copy_resolution)
                        {
                            let upload = ensure_camera_upload(
                                &vk_device,
                                &memory_properties,
                                &mut camera_upload,
                                flat_rgba.len() as vk::DeviceSize,
                            )?;
                            upload_headset_camera_rgba(&vk_device, upload, &flat_rgba)?;
                            active_camera_copy = Some(CameraCopy {
                                buffer: upload.buffer,
                                width: copy_resolution.width,
                                height: copy_resolution.height,
                            });
                            if camera_frame.index == 0 || camera_frame.index % 30 == 0 {
                                let projection = projection_readiness(&camera_frame);
                                log_info(format!(
                                "Rusty XR uploaded diagnostic flat camera copy frame {} requestedTier={} activeTier=cpu-diagnostic-flat-copy source={} cameraId={} lensFacing={} score={} delivered={}x{} copy={}x{} centeredIn={}x{} ts={} uploadCadenceHz~{} metadataIntrinsics={} intrinsicsDomain={} deliveredDomain={} metadataPose={} stereoLayout={:?} monoFallback={} fallbackReason={} projectionCheck={}",
                                camera_frame.index,
                                config.camera_tier.stable_id(),
                                camera_frame.metadata.source.label.as_str(),
                                camera_frame
                                    .metadata
                                    .source
                                    .physical_id
                                    .as_deref()
                                    .unwrap_or("unknown"),
                                camera_frame
                                    .diagnostics
                                    .lens_facing
                                    .as_deref()
                                    .unwrap_or("unknown"),
                                camera_frame
                                    .diagnostics
                                    .selection_score
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "unknown".to_string()),
                                camera_frame.width,
                                camera_frame.height,
                                copy_resolution.width,
                                copy_resolution.height,
                                swapchain.resolution.width,
                                swapchain.resolution.height,
                                camera_frame.timestamp_ns,
                                CAMERA_CPU_UPLOAD_HZ_LABEL,
                                if camera_frame.metadata.has_intrinsics() {
                                    "available"
                                } else {
                                    "missing"
                                },
                                pixel_domain_label(camera_frame.metadata.intrinsics_domain),
                                image_size_label(camera_frame.metadata.delivered_size),
                                if camera_frame.metadata.has_pose() {
                                    "available"
                                } else {
                                    "missing"
                                },
                                camera_frame.diagnostics.stereo_layout,
                                camera_frame.diagnostics.mono_fallback,
                                projection.fallback_reason,
                                projection.check_label
                            ));
                            }
                            last_uploaded_camera_timestamp_ns = Some(camera_frame.timestamp_ns);
                        } else {
                            log_error(format!(
                            "Rusty XR could not build diagnostic flat camera copy for frame {} {}x{} to swapchain {}x{}",
                            camera_frame.index,
                            camera_frame.width,
                            camera_frame.height,
                            swapchain.resolution.width,
                            swapchain.resolution.height
                        ));
                        }
                        last_uploaded_camera_index = Some(camera_frame.index);
                    }
                }
            }
        } else if config.camera_tier == CameraCompositeTier::GpuBufferProbe
            && !config.allow_cpu_fallback
            && active_camera_copy.is_some()
        {
            active_camera_copy = None;
            log_info("Rusty XR cleared diagnostic CPU camera copy because GPU camera tier requested CPU fallback is disabled");
        }

        let cmd = cmds[frame];
        vk_device
            .begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .map_err(|error| format!("begin Vulkan command buffer: {error}"))?;

        let mut prepared_gpu_camera: Option<(HeadsetCameraGpuFrame, usize)> = None;
        let mut prepared_stereo_camera: Option<(StereoGpuCameraFrame, usize)> = None;
        if config.camera_tier == CameraCompositeTier::GpuProjected {
            if let Some(stereo_frame) = latest_headset_stereo_camera_gpu_frame() {
                match gpu_camera_renderer.prepare_stereo_frame(
                    &vk_device,
                    cmd,
                    &stereo_frame,
                    config.camera_sampler_binding_mode,
                    config.camera_import_image_layout_mode,
                    config.camera_import_cache_limit,
                ) {
                    Ok(Some(descriptor_index)) => {
                        let controls = config.stereo_projection_controls(frame_count);
                        let projection_active = CameraProjectionPush::from_stereo_frame(
                            &stereo_frame,
                            &config,
                            &controls,
                            &views,
                            swapchain.resolution,
                        )
                        .2;
                        if last_logged_prepared_stereo_frame_index != Some(stereo_frame.index)
                            && (stereo_frame.index == 0 || stereo_frame.index % 120 == 0)
                        {
                            let orientation_accepted = projection_active
                                && controls.left_texture_transform.is_explicit_visual_check()
                                && controls.right_texture_transform.is_explicit_visual_check();
                            let pose_source = stereo_frame
                                .left
                                .diagnostics
                                .pose_source
                                .as_deref()
                                .unwrap_or("missing");
                            let pose_reference = stereo_frame
                                .left
                                .diagnostics
                                .lens_pose_reference_label
                                .as_deref()
                                .unwrap_or("unknown");
                            let pose_convention = stereo_frame
                                .left
                                .diagnostics
                                .pose_coordinate_convention
                                .as_deref()
                                .unwrap_or("unknown");
                            let (display_left_camera_id, display_right_camera_id) =
                                mapped_display_camera_ids(
                                    &stereo_frame,
                                    controls.source_eye_mapping,
                                );
                            log_info(format!(
                                "Rusty XR GPU stereo camera draw prepared frame {} requestedTier={} activeTier={} alignedProjection={} stereoLayout=Separate pairedLeftRightGpuBuffers=true cpuUploadCount=0 poseSource={} poseReference={} poseConvention={} projectionMode={} cameraFeedMode={} cameraColorMode={} cameraColorShaderBit={} cameraColorContrast={} cameraColorBrightness={} cameraColorSaturation={} cameraImportImageLayout={} importCacheLimit={} sourceEyeMapping={} displayLeftCameraId={} displayRightCameraId={} leftCameraTextureTransform={} rightCameraTextureTransform={} cameraTextureTransformSource={} cameraTextureTransformReason={} orientationCheck={} orientationAccepted={} visualReleaseAccepted={} orientationDiagnosticMode={} orientationDiagnosticStep={} importCacheSize={} stereoDescriptorCacheSize={} projectionShaderPath={} projectionMetadataReady={} fallbackReason={}",
                                stereo_frame.index,
                                config.camera_tier.stable_id(),
                                if projection_active { "gpu-projected" } else { "gpu-buffer-probe" },
                                projection_active,
                                pose_source,
                                pose_reference,
                                pose_convention,
                                config.camera_projection_mode.stable_id(),
                                config.camera_feed_pipeline_mode.stable_id(),
                                config.camera_color_mode.stable_id(),
                                config.camera_color_mode.shader_bit(),
                                config.camera_color_contrast,
                                config.camera_color_brightness,
                                config.camera_color_saturation,
                                config.camera_import_image_layout_mode.stable_id(),
                                config.camera_import_cache_limit,
                                controls.source_eye_mapping.stable_id(),
                                display_left_camera_id,
                                display_right_camera_id,
                                controls.left_label(),
                                controls.right_label(),
                                config.camera_texture_transform.source_label.as_str(),
                                config.camera_texture_transform.reason.as_str(),
                                config.camera_texture_transform.is_explicit_visual_check(),
                                orientation_accepted,
                                config.visual_release_accepted,
                                controls.diagnostic_mode.stable_id(),
                                controls.diagnostic_step,
                                gpu_camera_renderer.imports.len(),
                                gpu_camera_renderer.stereo_descriptors.len(),
                                if projection_active { "projected" } else { "flat-probe" },
                                stereo_projection_metadata_ready(&stereo_frame),
                                if projection_active {
                                    if config.visual_release_accepted {
                                        "projected shader path active with manual visual acceptance"
                                    } else {
                                        "projected shader path active; visual orientation/alignment acceptance still required"
                                    }
                                } else {
                                    "missing per-eye projection metadata or explicit texture orientation"
                                }
                            ));
                            last_logged_prepared_stereo_frame_index = Some(stereo_frame.index);
                        }
                        if projection_active && (frame_count == 0 || frame_count % 120 == 0) {
                            let pose_source = stereo_frame
                                .left
                                .diagnostics
                                .pose_source
                                .as_deref()
                                .unwrap_or("missing");
                            let pose_reference = stereo_frame
                                .left
                                .diagnostics
                                .lens_pose_reference_label
                                .as_deref()
                                .unwrap_or("unknown");
                            let pose_convention = stereo_frame
                                .left
                                .diagnostics
                                .pose_coordinate_convention
                                .as_deref()
                                .unwrap_or("unknown");
                            let (display_left_camera_id, display_right_camera_id) =
                                mapped_display_camera_ids(
                                    &stereo_frame,
                                    controls.source_eye_mapping,
                                );
                            let aligned_projection = projection_active;
                            let orientation_accepted =
                                controls.left_texture_transform.is_explicit_visual_check()
                                    && controls.right_texture_transform.is_explicit_visual_check();
                            log_info(format!(
                                "Rusty XR final projection status frame={} openXrFrameCount={} openXrFocused={} activeTier=gpu-projected alignedProjection={} stereoLayout=Separate pairedLeftRightGpuBuffers=true poseSource={} poseReference={} poseConvention={} projectionMode={} cameraFeedMode={} cameraColorMode={} cameraColorShaderBit={} cameraColorContrast={} cameraColorBrightness={} cameraColorSaturation={} cameraImportImageLayout={} importCacheLimit={} sourceEyeMapping={} displayLeftCameraId={} displayRightCameraId={} leftCameraTextureTransform={} rightCameraTextureTransform={} cameraTextureTransformSource={} cameraTextureTransformReason={} orientationCheck=true orientationAccepted={} cpuUploadCount=0 projectionShaderPath=projected projectionSurface={} coordinateChain=camera2-sensor-reference-to-openxr-head-basis importCacheSize={} stereoDescriptorCacheSize={} noHardwareBufferLifetimeWarnings=true frameCadenceTargetHz=72 visualInspection={} visualReleaseAccepted={} orientationDiagnosticMode={} orientationDiagnosticStep={}",
                                stereo_frame.index,
                                frame_count,
                                session_focused,
                                aligned_projection,
                                pose_source,
                                pose_reference,
                                pose_convention,
                                config.camera_projection_mode.stable_id(),
                                config.camera_feed_pipeline_mode.stable_id(),
                                config.camera_color_mode.stable_id(),
                                config.camera_color_mode.shader_bit(),
                                config.camera_color_contrast,
                                config.camera_color_brightness,
                                config.camera_color_saturation,
                                config.camera_import_image_layout_mode.stable_id(),
                                config.camera_import_cache_limit,
                                controls.source_eye_mapping.stable_id(),
                                display_left_camera_id,
                                display_right_camera_id,
                                controls.left_label(),
                                controls.right_label(),
                                config.camera_texture_transform.source_label.as_str(),
                                config.camera_texture_transform.reason.as_str(),
                                orientation_accepted,
                                config.camera_projection_mode.projection_surface_label(),
                                gpu_camera_renderer.imports.len(),
                                gpu_camera_renderer.stereo_descriptors.len(),
                                if config.visual_release_accepted { "accepted" } else { "required" },
                                config.visual_release_accepted,
                                controls.diagnostic_mode.stable_id(),
                                controls.diagnostic_step
                            ));
                        }
                        prepared_stereo_camera = Some((stereo_frame, descriptor_index));
                    }
                    Ok(None) => {
                        if frame_count == 0 || frame_count % 120 == 0 {
                            log_error(format!(
                                "Rusty XR GPU stereo camera import unavailable requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false fallbackReason={}",
                                config.camera_tier.stable_id(),
                                gpu_camera_renderer
                                    .last_failure
                                    .as_deref()
                                    .unwrap_or("Vulkan hardware-buffer import unavailable")
                            ));
                        }
                    }
                    Err(error) => {
                        if frame_count == 0 || frame_count % 120 == 0 {
                            log_error(format!(
                                "Rusty XR GPU stereo camera import failed requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false importFailure={} fallbackReason={}",
                                config.camera_tier.stable_id(),
                                gpu_camera_renderer.import_failure_count,
                                error
                            ));
                        }
                    }
                }
            }
        } else if config.camera_tier == CameraCompositeTier::GpuBufferProbe {
            if let Some(gpu_frame) = latest_headset_camera_gpu_frame() {
                match gpu_camera_renderer.prepare_frame(
                    &vk_device,
                    cmd,
                    &gpu_frame,
                    config.camera_sampler_binding_mode,
                    config.camera_import_image_layout_mode,
                    config.camera_import_cache_limit,
                ) {
                    Ok(Some(import_index)) => {
                        if gpu_frame.index == 0 || gpu_frame.index % 120 == 0 {
                            let projection = gpu_projection_readiness(&gpu_frame);
                            log_info(format!(
                                "Rusty XR GPU-sampled diagnostic camera surface frame {} requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false importCacheSize={} importSuccess={} importFailure={} stereoLayout={:?} poseSource={} projectionCheck={} fallbackReason={}",
                                gpu_frame.index,
                                config.camera_tier.stable_id(),
                                gpu_camera_renderer.imports.len(),
                                gpu_camera_renderer.import_success_count,
                                gpu_camera_renderer.import_failure_count,
                                gpu_frame.diagnostics.stereo_layout,
                                gpu_frame
                                    .diagnostics
                                    .pose_source
                                    .as_deref()
                                    .unwrap_or("missing"),
                                projection.check_label,
                                projection.fallback_reason
                            ));
                        }
                        prepared_gpu_camera = Some((gpu_frame, import_index));
                    }
                    Ok(None) => {
                        if frame_count == 0 || frame_count % 120 == 0 {
                            log_error(format!(
                                "Rusty XR GPU camera import unavailable requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false fallbackReason={}",
                                config.camera_tier.stable_id(),
                                gpu_camera_renderer
                                    .last_failure
                                    .as_deref()
                                    .unwrap_or("Vulkan hardware-buffer import unavailable")
                            ));
                        }
                    }
                    Err(error) => {
                        if frame_count == 0 || frame_count % 120 == 0 {
                            log_error(format!(
                                "Rusty XR GPU camera import failed requestedTier={} activeTier=gpu-buffer-probe alignedProjection=false importFailure={} fallbackReason={}",
                                config.camera_tier.stable_id(),
                                gpu_camera_renderer.import_failure_count,
                                error
                            ));
                        }
                    }
                }
            }
        }

        let clear = if config.camera_tier == CameraCompositeTier::GpuProjected {
            if config.openxr_passthrough_probe.submits_composition_layer() {
                [0.0, 0.0, 0.0, 0.0]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            }
        } else if frame_count % 120 < 60 {
            [0.02, 0.22, 0.26, 1.0]
        } else {
            [0.08, 0.12, 0.30, 1.0]
        };
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue { float32: clear },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
        ];
        let clear_values = if swapchain.foveation_enabled {
            &clear_values[..2]
        } else {
            &clear_values[..1]
        };
        vk_device.cmd_begin_render_pass(
            cmd,
            &vk::RenderPassBeginInfo::default()
                .render_pass(render_pass)
                .framebuffer(swapchain.buffers[image_index as usize].framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D::default(),
                    extent: swapchain.resolution,
                })
                .clear_values(clear_values),
            vk::SubpassContents::INLINE,
        );
        if let Some((ref stereo_frame, descriptor_index)) = prepared_stereo_camera {
            gpu_camera_renderer.record_draw_stereo(
                &vk_device,
                cmd,
                swapchain.resolution,
                descriptor_index,
                stereo_frame,
                &config,
                &views,
                frame_count,
            );
        }
        if let Some((ref gpu_frame, import_index)) = prepared_gpu_camera {
            gpu_camera_renderer.record_draw(
                &vk_device,
                cmd,
                swapchain.resolution,
                import_index,
                gpu_frame,
                &config,
            );
        }
        vk_device.cmd_end_render_pass(cmd);

        if let Some(camera_copy) = active_camera_copy {
            copy_diagnostic_camera_to_swapchain(
                &vk_device,
                cmd,
                &swapchain.buffers[image_index as usize],
                swapchain.resolution,
                camera_copy,
            );
        }

        vk_device
            .end_command_buffer(cmd)
            .map_err(|error| format!("end Vulkan command buffer: {error}"))?;
        let record_ms = record_started.elapsed().as_secs_f64() * 1000.0;

        swapchain
            .handle
            .wait_image(xr::Duration::INFINITE)
            .map_err(|error| format!("wait OpenXR swapchain image: {error}"))?;
        let submit_started = Instant::now();
        vk_device
            .queue_submit(
                queue,
                &[vk::SubmitInfo::default().command_buffers(&[cmd])],
                fences[frame],
            )
            .map_err(|error| format!("submit Vulkan queue: {error}"))?;
        let submit_ms = submit_started.elapsed().as_secs_f64() * 1000.0;
        swapchain
            .handle
            .release_image()
            .map_err(|error| format!("release OpenXR swapchain image: {error}"))?;

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: swapchain.resolution.width as _,
                height: swapchain.resolution.height as _,
            },
        };
        let projection_views = [
            xr::CompositionLayerProjectionView::new()
                .pose(views[0].pose)
                .fov(views[0].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&swapchain.handle)
                        .image_array_index(0)
                        .image_rect(rect),
                ),
            xr::CompositionLayerProjectionView::new()
                .pose(views[1].pose)
                .fov(views[1].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&swapchain.handle)
                        .image_array_index(1)
                        .image_rect(rect),
                ),
        ];
        let projection_layer_flags = if config.openxr_passthrough_probe.submits_composition_layer()
        {
            xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA
                | xr::CompositionLayerFlags::UNPREMULTIPLIED_ALPHA
        } else {
            xr::CompositionLayerFlags::EMPTY
        };
        let projection_layer = xr::CompositionLayerProjection::new()
            .layer_flags(projection_layer_flags)
            .space(&reference_space)
            .views(&projection_views);
        let passthrough_composition_layer = openxr_passthrough_probe
            .as_ref()
            .filter(|probe| probe.submits_composition_layer())
            .map(|probe| xr::sys::CompositionLayerPassthroughFB {
                ty: xr::sys::CompositionLayerPassthroughFB::TYPE,
                next: ptr::null(),
                flags: xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
                space: xr::sys::Space::NULL,
                layer_handle: probe.layer.as_raw(),
            });
        let mut layers: Vec<&xr::CompositionLayerBase<xr::Vulkan>> =
            Vec::with_capacity(if passthrough_composition_layer.is_some() {
                2
            } else {
                1
            });
        if let Some(layer) = passthrough_composition_layer.as_ref() {
            // The openxr crate does not re-export this FB layer builder, but the raw
            // struct has the standard composition-layer header prefix expected here.
            let layer_base: &xr::CompositionLayerBase<xr::Vulkan> = unsafe {
                &*(layer as *const xr::sys::CompositionLayerPassthroughFB
                    as *const xr::CompositionLayerBase<xr::Vulkan>)
            };
            layers.push(layer_base);
        }
        layers.push(&projection_layer);
        frame_stream
            .end(
                frame_state.predicted_display_time,
                environment_blend_mode,
                &layers,
            )
            .map_err(|error| format!("end OpenXR frame: {error}"))?;

        frame_count += 1;
        if let Some(probe) = openxr_passthrough_probe.as_mut() {
            probe.tick(frame_count);
        }
        frame_pacing_window_frames += 1;
        if frame_count == 1 || frame_count % 120 == 0 {
            let config = runtime_config();
            let (gpu_success, gpu_failure, gpu_cache_size) = gpu_probe_counters();
            let active_display_refresh_hz = if xr_instance.exts().fb_display_refresh_rate.is_some()
            {
                session.get_display_refresh_rate().ok()
            } else {
                None
            };
            let window_elapsed = frame_pacing_window_start.elapsed();
            let window_secs = window_elapsed.as_secs_f64().max(0.001);
            let observed_openxr_fps = frame_pacing_window_frames as f64 / window_secs;
            let avg_frame_ms = window_secs * 1000.0 / frame_pacing_window_frames.max(1) as f64;
            log_info(format!(
                "Rusty XR OpenXR frame {} rendered {}x{} requestedTier={} cameraAcquisition={} cameraEnabled={} mediaProjection={} observedOpenXrFps={:.1} avgFrameMs={:.2} recordCpuMs={:.3} submitCpuMs={:.3} frameCadenceTargetHz=72 activeDisplayRefreshHz={} renderScale={} fixedFoveationLevel={} fixedFoveationEnabled={} openxrPassthroughProbe={} fenceSync=slot-reuse pipelineDepth={} gpuProbeSuccess={} gpuProbeFailure={} descriptorProbeCacheSize={} importCacheSize={} importCacheLimit={} stereoDescriptorCacheSize={} gpuImportSuccess={} gpuImportFailure={} gpuImportCacheHit={} gpuImportCacheMiss={} gpuImportCacheEvict={}",
                frame_count,
                swapchain.resolution.width,
                swapchain.resolution.height,
                config.camera_tier.stable_id(),
                config.camera_acquisition.as_str(),
                config.camera_enabled,
                config.media_projection_enabled,
                observed_openxr_fps,
                avg_frame_ms,
                record_ms,
                submit_ms,
                refresh_rate_label(active_display_refresh_hz),
                config.xr_render_scale,
                config.xr_fixed_foveation_level,
                swapchain.foveation_enabled,
                openxr_passthrough_probe
                    .as_ref()
                    .map(|probe| probe.mode.stable_id())
                    .unwrap_or("off"),
                PIPELINE_DEPTH,
                gpu_success,
                gpu_failure,
                gpu_cache_size,
                gpu_camera_renderer.imports.len(),
                config.camera_import_cache_limit,
                gpu_camera_renderer.stereo_descriptors.len(),
                gpu_camera_renderer.import_success_count,
                gpu_camera_renderer.import_failure_count,
                gpu_camera_renderer.import_cache_hit_count,
                gpu_camera_renderer.import_cache_miss_count,
                gpu_camera_renderer.import_cache_evict_count
            ));
            frame_pacing_window_start = Instant::now();
            frame_pacing_window_frames = 0;
        }
        frame = (frame + 1) % PIPELINE_DEPTH as usize;
    }

    drop((session, frame_wait, frame_stream, reference_space));
    vk_device
        .wait_for_fences(&fences, true, u64::MAX)
        .map_err(|error| format!("final Vulkan fence wait: {error}"))?;
    for fence in fences {
        vk_device.destroy_fence(fence, None);
    }
    if let Some(swapchain) = swapchain {
        for buffer in swapchain.buffers {
            vk_device.destroy_framebuffer(buffer.framebuffer, None);
            if buffer.fragment_density != vk::ImageView::null() {
                vk_device.destroy_image_view(buffer.fragment_density, None);
            }
            if let Some(depth) = buffer.depth {
                vk_device.destroy_image_view(depth.view, None);
                vk_device.destroy_image(depth.image, None);
                vk_device.free_memory(depth.memory, None);
            }
            vk_device.destroy_image_view(buffer.color, None);
        }
    }
    if let Some(upload) = camera_upload {
        vk_device.destroy_buffer(upload.buffer, None);
        vk_device.free_memory(upload.memory, None);
    }
    gpu_camera_renderer.destroy(&vk_device);
    vk_device.destroy_command_pool(cmd_pool, None);
    vk_device.destroy_render_pass(render_pass, None);
    vk_device.destroy_device(None);
    vk_instance.destroy_instance(None);

    log_info("Rusty XR OpenXR loop exited cleanly");
    Ok(())
}

unsafe fn ensure_swapchain<'a>(
    xr_instance: &xr::Instance,
    session: &xr::Session<xr::Vulkan>,
    system: xr::SystemId,
    vk_device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    render_pass: vk::RenderPass,
    color_format_mode: OpenXrColorFormatMode,
    color_format: vk::Format,
    fixed_foveation_render_path: bool,
    swapchain: &'a mut Option<Swapchain>,
) -> Result<&'a mut Swapchain, String> {
    if swapchain.is_none() {
        let views = xr_instance
            .enumerate_view_configuration_views(system, VIEW_TYPE)
            .map_err(|error| format!("enumerate OpenXR view configuration: {error}"))?;
        if views.len() != VIEW_COUNT as usize {
            return Err(format!(
                "expected {VIEW_COUNT} OpenXR views, got {}",
                views.len()
            ));
        }
        if views[0] != views[1] {
            return Err(
                "this minimal multiview example requires matching eye dimensions".to_string(),
            );
        }

        let recommended_resolution = vk::Extent2D {
            width: views[0].recommended_image_rect_width,
            height: views[0].recommended_image_rect_height,
        };
        let config = runtime_config();
        let render_scale = sanitized_render_scale(config.xr_render_scale);
        let fixed_foveation_level = config.xr_fixed_foveation_level;
        let resolution = scaled_extent(recommended_resolution, render_scale);
        let use_fixed_foveation = fixed_foveation_level > 0 && fixed_foveation_render_path;
        if fixed_foveation_level > 0 && !use_fixed_foveation {
            log_error(format!(
                "Rusty XR fixed foveation requested level={} but required OpenXR/Vulkan fragment-density path is unavailable",
                fixed_foveation_level
            ));
        }
        let created_swapchain = create_openxr_swapchain(
            xr_instance,
            session,
            resolution,
            color_format_mode,
            color_format,
            fixed_foveation_level,
            use_fixed_foveation,
        )?;
        let mut buffers = Vec::with_capacity(created_swapchain.color_images.len());
        for (index, color_image) in created_swapchain.color_images.iter().copied().enumerate() {
            let color_image = vk::Image::from_raw(color_image);
            let color = vk_device
                .create_image_view(
                    &vk::ImageViewCreateInfo::default()
                        .image(color_image)
                        .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
                        .format(color_format)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: VIEW_COUNT,
                        }),
                    None,
                )
                .map_err(|error| format!("create Vulkan swapchain image view: {error}"))?;
            let fragment_density = if created_swapchain.fixed_foveation_enabled {
                let fragment_density_image = created_swapchain
                    .fragment_density_images
                    .get(index)
                    .copied()
                    .ok_or_else(|| {
                        "OpenXR foveation image count did not match swapchain image count"
                            .to_string()
                    })?;
                if fragment_density_image == 0 {
                    return Err(format!(
                        "OpenXR foveation image handle was null for swapchain image {index}"
                    ));
                }
                create_fragment_density_image_view(
                    vk_device,
                    vk::Image::from_raw(fragment_density_image),
                )?
            } else {
                vk::ImageView::null()
            };
            let depth = if created_swapchain.fixed_foveation_enabled {
                Some(create_foveation_depth_attachment(
                    vk_device,
                    memory_properties,
                    resolution,
                )?)
            } else {
                None
            };
            let mut attachments = vec![color];
            if let Some(depth) = &depth {
                attachments.push(depth.view);
            }
            if fragment_density != vk::ImageView::null() {
                attachments.push(fragment_density);
            }
            if created_swapchain.fixed_foveation_enabled {
                log_info(format!(
                    "Rusty XR OpenXR foveation framebuffer plan index={} colorImage=0x{:x} colorView=0x{:x} depthView=0x{:x} fragmentDensityView=0x{:x} attachments={}",
                    index,
                    color_image.as_raw(),
                    color.as_raw(),
                    depth.as_ref().map(|value| value.view.as_raw()).unwrap_or_default(),
                    fragment_density.as_raw(),
                    attachments.len()
                ));
            }
            let framebuffer = vk_device
                .create_framebuffer(
                    &vk::FramebufferCreateInfo::default()
                        .render_pass(render_pass)
                        .width(resolution.width)
                        .height(resolution.height)
                        .attachments(&attachments)
                        .layers(1),
                    None,
                )
                .map_err(|error| format!("create Vulkan framebuffer: {error}"))?;
            buffers.push(Framebuffer {
                framebuffer,
                color,
                depth,
                fragment_density,
                image: color_image,
            });
        }

        log_info(format!(
            "Rusty XR OpenXR swapchain created {}x{} from recommended {}x{} scale={} xrColorFormat={} vkFormat={:?} fixedFoveationLevel={} fixedFoveationEnabled={} fragmentDensityMapImages={} with {} image(s)",
            resolution.width,
            resolution.height,
            recommended_resolution.width,
            recommended_resolution.height,
            render_scale,
            color_format_mode.stable_id(),
            color_format,
            fixed_foveation_level,
            created_swapchain.fixed_foveation_enabled,
            created_swapchain.fragment_density_images.len(),
            buffers.len()
        ));
        *swapchain = Some(Swapchain {
            handle: created_swapchain.handle,
            buffers,
            resolution,
            foveation_enabled: created_swapchain.fixed_foveation_enabled,
        });
    }

    swapchain
        .as_mut()
        .ok_or_else(|| "swapchain was not initialized".to_string())
}

struct OpenXrSwapchainImages {
    handle: xr::Swapchain<xr::Vulkan>,
    color_images: Vec<u64>,
    fragment_density_images: Vec<u64>,
    fixed_foveation_enabled: bool,
}

unsafe fn create_openxr_swapchain(
    xr_instance: &xr::Instance,
    session: &xr::Session<xr::Vulkan>,
    resolution: vk::Extent2D,
    color_format_mode: OpenXrColorFormatMode,
    color_format: vk::Format,
    fixed_foveation_level: u8,
    use_fixed_foveation: bool,
) -> Result<OpenXrSwapchainImages, String> {
    let mut foveation_create_info = xr::sys::SwapchainCreateInfoFoveationFB {
        ty: xr::sys::SwapchainCreateInfoFoveationFB::TYPE,
        next: ptr::null_mut(),
        flags: xr::sys::SwapchainCreateFoveationFlagsFB::FRAGMENT_DENSITY_MAP,
    };
    let create_info = xr::sys::SwapchainCreateInfo {
        ty: xr::sys::SwapchainCreateInfo::TYPE,
        next: if use_fixed_foveation {
            &mut foveation_create_info as *mut _ as *const _
        } else {
            ptr::null()
        },
        create_flags: xr::sys::SwapchainCreateFlags::EMPTY,
        usage_flags: xr::sys::SwapchainUsageFlags::COLOR_ATTACHMENT
            | xr::sys::SwapchainUsageFlags::SAMPLED
            | xr::sys::SwapchainUsageFlags::TRANSFER_DST,
        format: color_format.as_raw() as _,
        sample_count: 1,
        width: resolution.width,
        height: resolution.height,
        face_count: 1,
        array_size: VIEW_COUNT,
        mip_count: 1,
    };
    log_info(format!(
        "Rusty XR OpenXR swapchain request {}x{} xrColorFormat={} vkFormat={:?} fixedFoveationLevel={} fixedFoveationRequested={}",
        resolution.width,
        resolution.height,
        color_format_mode.stable_id(),
        color_format,
        fixed_foveation_level,
        use_fixed_foveation
    ));
    let mut raw_handle = xr::sys::Swapchain::NULL;
    ensure_xr_success(
        (xr_instance.fp().create_swapchain)(session.as_raw(), &create_info, &mut raw_handle),
        "xrCreateSwapchain",
    )?;

    let handle = xr::Swapchain::from_raw(session.clone(), raw_handle);
    let mut fixed_foveation_enabled = false;
    if use_fixed_foveation {
        let level = desired_fixed_foveation_level(fixed_foveation_level)
            .ok_or_else(|| "invalid fixed foveation level".to_string())?;
        match enable_openxr_fixed_foveation(xr_instance, session, &handle, level) {
            Ok(()) => {
                fixed_foveation_enabled = true;
            }
            Err(error) => {
                log_error(format!(
                    "Rusty XR OpenXR fixed foveation enable failed; continuing without foveation: {error}"
                ));
            }
        }
    }

    let (color_images, fragment_density_images) = if fixed_foveation_enabled {
        enumerate_openxr_foveation_swapchain_images(xr_instance, &handle)?
    } else {
        (
            handle
                .enumerate_images()
                .map_err(|error| format!("enumerate OpenXR swapchain images: {error}"))?,
            Vec::new(),
        )
    };
    log_info(format!(
        "Rusty XR OpenXR swapchain image enumeration fixedFoveationEnabled={} colorImages={} fragmentDensityImages={}",
        fixed_foveation_enabled,
        color_images.len(),
        fragment_density_images.len()
    ));

    Ok(OpenXrSwapchainImages {
        handle,
        color_images,
        fragment_density_images,
        fixed_foveation_enabled,
    })
}

fn desired_fixed_foveation_level(level: u8) -> Option<xr::FoveationLevelFB> {
    match level {
        0 => None,
        1 => Some(xr::FoveationLevelFB::LOW),
        2 => Some(xr::FoveationLevelFB::MEDIUM),
        _ => Some(xr::FoveationLevelFB::HIGH),
    }
}

unsafe fn enable_openxr_fixed_foveation(
    xr_instance: &xr::Instance,
    session: &xr::Session<xr::Vulkan>,
    swapchain: &xr::Swapchain<xr::Vulkan>,
    level: xr::FoveationLevelFB,
) -> Result<(), String> {
    let update_swapchain = xr_instance
        .exts()
        .fb_swapchain_update_state
        .as_ref()
        .ok_or_else(|| "XR_FB_swapchain_update_state is unavailable".to_string())?;
    let profile = session
        .create_foveation_profile(Some(xr::FoveationLevelProfile {
            level,
            vertical_offset: 0.0,
            dynamic: xr::FoveationDynamicFB::DISABLED,
        }))
        .map_err(|error| format!("xrCreateFoveationProfileFB: {error}"))?;
    let state = xr::sys::SwapchainStateFoveationFB {
        ty: xr::sys::SwapchainStateFoveationFB::TYPE,
        next: ptr::null_mut(),
        flags: xr::sys::SwapchainStateFoveationFlagsFB::EMPTY,
        profile: profile.as_raw(),
    };
    ensure_xr_success(
        (update_swapchain.update_swapchain)(
            swapchain.as_raw(),
            &state as *const _ as *const xr::sys::SwapchainStateBaseHeaderFB,
        ),
        "xrUpdateSwapchainFB",
    )?;
    Ok(())
}

unsafe fn enumerate_openxr_foveation_swapchain_images(
    xr_instance: &xr::Instance,
    swapchain: &xr::Swapchain<xr::Vulkan>,
) -> Result<(Vec<u64>, Vec<u64>), String> {
    let mut image_count = 0;
    ensure_xr_success(
        (xr_instance.fp().enumerate_swapchain_images)(
            swapchain.as_raw(),
            0,
            &mut image_count,
            ptr::null_mut(),
        ),
        "xrEnumerateSwapchainImages(count)",
    )?;
    let mut color_images = vec![
        xr::sys::SwapchainImageVulkanKHR {
            ty: xr::sys::SwapchainImageVulkanKHR::TYPE,
            next: ptr::null_mut(),
            image: 0,
        };
        image_count as usize
    ];
    let mut fragment_density_images = vec![
        xr::sys::SwapchainImageFoveationVulkanFB {
            ty: xr::sys::SwapchainImageFoveationVulkanFB::TYPE,
            next: ptr::null_mut(),
            image: 0,
            width: 0,
            height: 0,
        };
        image_count as usize
    ];
    for (color, fragment_density) in color_images
        .iter_mut()
        .zip(fragment_density_images.iter_mut())
    {
        color.next = fragment_density as *mut _ as *mut _;
    }
    let mut enumerated = 0;
    ensure_xr_success(
        (xr_instance.fp().enumerate_swapchain_images)(
            swapchain.as_raw(),
            image_count,
            &mut enumerated,
            color_images.as_mut_ptr() as *mut xr::sys::SwapchainImageBaseHeader,
        ),
        "xrEnumerateSwapchainImages",
    )?;
    color_images.truncate(enumerated as usize);
    fragment_density_images.truncate(enumerated as usize);
    for (index, (color, fragment_density)) in color_images
        .iter()
        .zip(fragment_density_images.iter())
        .enumerate()
    {
        log_info(format!(
            "Rusty XR OpenXR foveation image index={} colorImage=0x{:x} fragmentDensityImage=0x{:x} fragmentDensitySize={}x{}",
            index,
            color.image,
            fragment_density.image,
            fragment_density.width,
            fragment_density.height
        ));
    }
    Ok((
        color_images.into_iter().map(|image| image.image).collect(),
        fragment_density_images
            .into_iter()
            .map(|image| image.image)
            .collect(),
    ))
}

unsafe fn create_fragment_density_image_view(
    device: &ash::Device,
    image: vk::Image,
) -> Result<vk::ImageView, String> {
    device
        .create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(XR_FRAGMENT_DENSITY_MAP_FORMAT)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }),
            None,
        )
        .map_err(|error| format!("create fragment density image view: {error}"))
}

unsafe fn create_foveation_depth_attachment(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    resolution: vk::Extent2D,
) -> Result<DepthAttachment, String> {
    let image = device
        .create_image(
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(XR_FOVEATION_DEPTH_FORMAT)
                .extent(vk::Extent3D {
                    width: resolution.width.max(1),
                    height: resolution.height.max(1),
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(VIEW_COUNT)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            None,
        )
        .map_err(|error| format!("create foveation depth image: {error}"))?;
    let requirements = device.get_image_memory_requirements(image);
    let memory_type_index = match find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    ) {
        Ok(index) => index,
        Err(error) => {
            device.destroy_image(image, None);
            return Err(error);
        }
    };
    let memory = match device.allocate_memory(
        &vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index),
        None,
    ) {
        Ok(memory) => memory,
        Err(error) => {
            device.destroy_image(image, None);
            return Err(format!("allocate foveation depth memory: {error}"));
        }
    };
    if let Err(error) = device.bind_image_memory(image, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_image(image, None);
        return Err(format!("bind foveation depth memory: {error}"));
    }
    let view = match device.create_image_view(
        &vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(XR_FOVEATION_DEPTH_FORMAT)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: VIEW_COUNT,
            }),
        None,
    ) {
        Ok(view) => view,
        Err(error) => {
            device.free_memory(memory, None);
            device.destroy_image(image, None);
            return Err(format!("create foveation depth image view: {error}"));
        }
    };
    Ok(DepthAttachment {
        image,
        view,
        memory,
    })
}

unsafe fn ensure_camera_upload<'a>(
    vk_device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    upload: &'a mut Option<CameraUpload>,
    byte_len: vk::DeviceSize,
) -> Result<&'a mut CameraUpload, String> {
    let needs_new = upload
        .as_ref()
        .map(|upload| upload.capacity < byte_len)
        .unwrap_or(true);

    if needs_new {
        if let Some(old) = upload.take() {
            vk_device.destroy_buffer(old.buffer, None);
            vk_device.free_memory(old.memory, None);
        }

        let buffer = vk_device
            .create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(byte_len)
                    .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )
            .map_err(|error| format!("create headset camera upload buffer: {error}"))?;
        let requirements = vk_device.get_buffer_memory_requirements(buffer);
        let memory_type_index = find_memory_type(
            memory_properties,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let memory = vk_device
            .allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(memory_type_index),
                None,
            )
            .map_err(|error| format!("allocate headset camera upload memory: {error}"))?;
        vk_device
            .bind_buffer_memory(buffer, memory, 0)
            .map_err(|error| format!("bind headset camera upload memory: {error}"))?;

        *upload = Some(CameraUpload {
            buffer,
            memory,
            capacity: byte_len,
        });
    }

    upload
        .as_mut()
        .ok_or_else(|| "headset camera upload buffer was not initialized".to_string())
}

unsafe fn upload_headset_camera_rgba(
    vk_device: &ash::Device,
    upload: &CameraUpload,
    rgba: &[u8],
) -> Result<(), String> {
    let byte_len = rgba.len() as vk::DeviceSize;
    let mapped = vk_device
        .map_memory(upload.memory, 0, byte_len, vk::MemoryMapFlags::empty())
        .map_err(|error| format!("map headset camera upload memory: {error}"))?;
    std::ptr::copy_nonoverlapping(rgba.as_ptr(), mapped.cast::<u8>(), rgba.len());
    vk_device.unmap_memory(upload.memory);
    Ok(())
}

fn build_diagnostic_flat_camera_rgba(
    frame: &HeadsetCameraFrame,
    target: vk::Extent2D,
) -> Option<Vec<u8>> {
    let source_size = ImageSize::new(frame.width, frame.height);
    let target_size = ImageSize::new(target.width, target.height);
    if !source_size.is_non_empty() || !target_size.is_non_empty() {
        return None;
    }

    let expected_len = frame.width as usize * frame.height as usize * 4;
    if frame.rgba.len() != expected_len {
        return None;
    }

    let target_width = target.width as usize;
    let target_height = target.height as usize;
    let mut output = vec![0_u8; target_width * target_height * 4];

    for y in 0..target_height {
        let v = normalized_pixel_coord(y, target_height);
        let source_y = (frame.height.saturating_sub(1) as f32) * v;
        for x in 0..target_width {
            let u = normalized_pixel_coord(x, target_width);
            let source_x = (frame.width.saturating_sub(1) as f32) * u;
            let sampled =
                sample_bilinear_rgba(&frame.rgba, frame.width, frame.height, source_x, source_y);
            let dst = (y * target_width + x) * 4;
            output[dst..dst + 4].copy_from_slice(&sampled);
        }
    }

    Some(output)
}

fn bounded_diagnostic_camera_copy_extent(
    frame: &HeadsetCameraFrame,
    swapchain: vk::Extent2D,
) -> vk::Extent2D {
    if frame.width == 0 || frame.height == 0 || swapchain.width == 0 || swapchain.height == 0 {
        return vk::Extent2D {
            width: 0,
            height: 0,
        };
    }

    let max_width = CAMERA_CPU_COPY_MAX_DIMENSION
        .min(frame.width.max(frame.height))
        .min(swapchain.width)
        .max(1);
    let max_height = CAMERA_CPU_COPY_MAX_DIMENSION
        .min(frame.width.max(frame.height))
        .min(swapchain.height)
        .max(1);
    let source_aspect = frame.width as f32 / frame.height as f32;
    let container_aspect = max_width as f32 / max_height as f32;
    let (width, height) = if container_aspect > source_aspect {
        let height = max_height;
        let width = (height as f32 * source_aspect).round().max(1.0) as u32;
        (width, height)
    } else {
        let width = max_width;
        let height = (width as f32 / source_aspect).round().max(1.0) as u32;
        (width, height)
    };

    vk::Extent2D {
        width: width.min(swapchain.width).max(1),
        height: height.min(swapchain.height).max(1),
    }
}

fn projection_readiness(frame: &HeadsetCameraFrame) -> ProjectionReadiness {
    let Some(intrinsics) = frame.metadata.intrinsics else {
        return ProjectionReadiness::fallback("missing intrinsics");
    };
    let Some(source_domain) = frame.metadata.intrinsics_domain else {
        return ProjectionReadiness::fallback("missing intrinsics source domain");
    };
    let scaled = match scale_intrinsics_to_image(
        intrinsics,
        source_domain.size,
        frame.metadata.delivered_size,
    ) {
        Ok(value) => value,
        Err(error) => {
            return ProjectionReadiness::fallback(format!("could not scale intrinsics: {error}"));
        }
    };
    let center = match project_camera_point(scaled, Vec3::new(0.0, 0.0, 1.0)) {
        Ok(value) => value,
        Err(error) => {
            return ProjectionReadiness::fallback(format!(
                "could not project center point: {error}"
            ));
        }
    };

    if !frame.metadata.has_pose() {
        return ProjectionReadiness {
            fallback_reason: "missing camera pose; diagnostic flat camera copy".to_string(),
            check_label: format!("scaledPrincipal=({:.1},{:.1})", center.x, center.y),
        };
    }

    ProjectionReadiness {
        fallback_reason:
            "metadata-backed projection inputs available, but CPU diagnostic copy path is active"
                .to_string(),
        check_label: format!("scaledPrincipal=({:.1},{:.1})", center.x, center.y),
    }
}

fn gpu_projection_readiness(frame: &HeadsetCameraGpuFrame) -> ProjectionReadiness {
    let Some(intrinsics) = frame.metadata.intrinsics else {
        return ProjectionReadiness::fallback("missing intrinsics");
    };
    let Some(source_domain) = frame.metadata.intrinsics_domain else {
        return ProjectionReadiness::fallback("missing intrinsics source domain");
    };
    let scaled = match scale_intrinsics_to_image(
        intrinsics,
        source_domain.size,
        frame.metadata.delivered_size,
    ) {
        Ok(value) => value,
        Err(error) => {
            return ProjectionReadiness::fallback(format!("could not scale intrinsics: {error}"));
        }
    };
    let center = match project_camera_point(scaled, Vec3::new(0.0, 0.0, 1.0)) {
        Ok(value) => value,
        Err(error) => {
            return ProjectionReadiness::fallback(format!(
                "could not project center point: {error}"
            ));
        }
    };

    if !frame.metadata.has_pose() {
        return ProjectionReadiness {
            fallback_reason: "missing camera pose; GPU-sampled diagnostic surface is flat"
                .to_string(),
            check_label: format!("scaledPrincipal=({:.1},{:.1})", center.x, center.y),
        };
    }

    ProjectionReadiness {
        fallback_reason:
            "projection metadata is available, but this renderer is still the flat GPU-buffer diagnostic path"
                .to_string(),
        check_label: format!("scaledPrincipal=({:.1},{:.1})", center.x, center.y),
    }
}

fn stereo_projection_metadata_ready(frame: &StereoGpuCameraFrame) -> bool {
    let left_pose = frame
        .left
        .diagnostics
        .pose_source
        .as_deref()
        .map(|value| matches!(value, "platform" | "estimated-profile"))
        .unwrap_or(false);
    let right_pose = frame
        .right
        .diagnostics
        .pose_source
        .as_deref()
        .map(|value| matches!(value, "platform" | "estimated-profile"))
        .unwrap_or(false);
    frame.left.metadata.has_projection_metadata()
        && frame.right.metadata.has_projection_metadata()
        && left_pose
        && right_pose
}

fn mapped_display_camera_ids(
    frame: &StereoGpuCameraFrame,
    mapping: crate::StereoSourceEyeMapping,
) -> (&str, &str) {
    let left = frame
        .left
        .metadata
        .source
        .physical_id
        .as_deref()
        .unwrap_or("left-source");
    let right = frame
        .right
        .metadata
        .source
        .physical_id
        .as_deref()
        .unwrap_or("right-source");
    match mapping {
        crate::StereoSourceEyeMapping::DisplayLeftFromLeftSource => (left, right),
        crate::StereoSourceEyeMapping::DisplayLeftFromRightSource => (right, left),
    }
}

fn pixel_domain_label(domain: Option<CameraPixelDomain>) -> String {
    domain
        .map(|domain| {
            format!(
                "{:?}:{}x{}",
                domain.kind, domain.size.width, domain.size.height
            )
        })
        .unwrap_or_else(|| "missing".to_string())
}

fn image_size_label(size: ImageSize) -> String {
    format!("{}x{}", size.width, size.height)
}

unsafe fn create_openxr_render_pass(
    device: &ash::Device,
    use_fragment_density_map: bool,
    color_format: vk::Format,
) -> Result<vk::RenderPass, String> {
    let color_attachment = vk::AttachmentDescription {
        format: color_format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        ..Default::default()
    };
    let depth_attachment = vk::AttachmentDescription {
        format: XR_FOVEATION_DEPTH_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::DONT_CARE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        ..Default::default()
    };
    let fragment_density_attachment = vk::AttachmentDescription {
        format: XR_FRAGMENT_DENSITY_MAP_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::DONT_CARE,
        store_op: vk::AttachmentStoreOp::DONT_CARE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::FRAGMENT_DENSITY_MAP_OPTIMAL_EXT,
        final_layout: vk::ImageLayout::FRAGMENT_DENSITY_MAP_OPTIMAL_EXT,
        ..Default::default()
    };
    let attachments = if use_fragment_density_map {
        vec![
            color_attachment,
            depth_attachment,
            fragment_density_attachment,
        ]
    } else {
        vec![color_attachment]
    };
    let color_refs = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
    let fragment_density_ref = vk::AttachmentReference {
        attachment: 2,
        layout: vk::ImageLayout::FRAGMENT_DENSITY_MAP_OPTIMAL_EXT,
    };
    let depth_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };
    let mut subpass = vk::SubpassDescription::default()
        .color_attachments(&color_refs)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);
    if use_fragment_density_map {
        subpass = subpass.depth_stencil_attachment(&depth_ref);
    }
    let subpasses = [subpass];
    let depth_stage = if use_fragment_density_map {
        vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
    } else {
        vk::PipelineStageFlags::empty()
    };
    let depth_access = if use_fragment_density_map {
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
    } else {
        vk::AccessFlags::empty()
    };
    let fdm_stage = if use_fragment_density_map {
        vk::PipelineStageFlags::FRAGMENT_DENSITY_PROCESS_EXT
    } else {
        vk::PipelineStageFlags::empty()
    };
    let fdm_access = if use_fragment_density_map {
        vk::AccessFlags::FRAGMENT_DENSITY_MAP_READ_EXT
    } else {
        vk::AccessFlags::empty()
    };
    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        dst_subpass: 0,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | depth_stage | fdm_stage,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | depth_stage | fdm_stage,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE | depth_access | fdm_access,
        ..Default::default()
    }];
    let view_mask = !(!0 << VIEW_COUNT);
    let view_masks = [view_mask];
    let correlation_masks = [view_mask];
    let mut multiview = vk::RenderPassMultiviewCreateInfo::default()
        .view_masks(&view_masks)
        .correlation_masks(&correlation_masks);
    let mut fragment_density_info = vk::RenderPassFragmentDensityMapCreateInfoEXT::default()
        .fragment_density_map_attachment(fragment_density_ref);
    let mut render_pass_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);
    if use_fragment_density_map {
        render_pass_info = render_pass_info.push_next(&mut fragment_density_info);
    }
    render_pass_info = render_pass_info.push_next(&mut multiview);
    device
        .create_render_pass(&render_pass_info, None)
        .map_err(|error| format!("create render pass: {error}"))
}

unsafe fn query_fragment_density_map_support(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<bool, String> {
    if !physical_device_supports_extension(
        instance,
        physical_device,
        ash::ext::fragment_density_map::NAME,
    )? {
        return Ok(false);
    }

    let mut features = vk::PhysicalDeviceFragmentDensityMapFeaturesEXT::default();
    let mut features2 = vk::PhysicalDeviceFeatures2::default().push_next(&mut features);
    instance.get_physical_device_features2(physical_device, &mut features2);
    if features.fragment_density_map != vk::TRUE {
        return Ok(false);
    }

    let format_props = instance
        .get_physical_device_format_properties(physical_device, XR_FRAGMENT_DENSITY_MAP_FORMAT);
    Ok(format_props
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::FRAGMENT_DENSITY_MAP_EXT))
}

unsafe fn physical_device_supports_extension(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    extension_name: &CStr,
) -> Result<bool, String> {
    let extensions = instance
        .enumerate_device_extension_properties(physical_device)
        .map_err(|error| format!("enumerate Vulkan device extensions: {error}"))?;
    Ok(extensions
        .iter()
        .any(|extension| CStr::from_ptr(extension.extension_name.as_ptr()) == extension_name))
}

struct ProjectionReadiness {
    fallback_reason: String,
    check_label: String,
}

impl ProjectionReadiness {
    fn fallback(reason: impl Into<String>) -> Self {
        Self {
            fallback_reason: format!("{}; diagnostic flat camera copy", reason.into()),
            check_label: "unavailable".to_string(),
        }
    }
}

fn scaled_extent(recommended: vk::Extent2D, scale: f32) -> vk::Extent2D {
    let scale = sanitized_render_scale(scale);
    vk::Extent2D {
        width: ((recommended.width.max(1) as f32) * scale).round().max(1.0) as u32,
        height: ((recommended.height.max(1) as f32) * scale)
            .round()
            .max(1.0) as u32,
    }
}

fn sanitized_render_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale.clamp(0.25, 1.5)
    } else {
        XR_RENDER_SCALE_DEFAULT
    }
}

fn should_throttle_camera_upload(last_timestamp_ns: Option<i64>, timestamp_ns: i64) -> bool {
    last_timestamp_ns
        .and_then(|last_timestamp_ns| timestamp_ns.checked_sub(last_timestamp_ns))
        .map(|delta_ns| delta_ns >= 0 && delta_ns < CAMERA_CPU_UPLOAD_MIN_INTERVAL_NS)
        .unwrap_or(false)
}

fn normalized_pixel_coord(index: usize, count: usize) -> f32 {
    if count <= 1 {
        0.5
    } else {
        index as f32 / (count - 1) as f32
    }
}

fn sample_bilinear_rgba(source: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 4] {
    let max_x = width.saturating_sub(1) as f32;
    let max_y = height.saturating_sub(1) as f32;
    let x = x.clamp(0.0, max_x);
    let y = y.clamp(0.0, max_y);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width.saturating_sub(1));
    let y1 = (y0 + 1).min(height.saturating_sub(1));
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;

    let c00 = sample_rgba(source, width, x0, y0);
    let c10 = sample_rgba(source, width, x1, y0);
    let c01 = sample_rgba(source, width, x0, y1);
    let c11 = sample_rgba(source, width, x1, y1);
    let mut out = [0_u8; 4];
    for channel in 0..4 {
        let top = lerp(c00[channel] as f32, c10[channel] as f32, tx);
        let bottom = lerp(c01[channel] as f32, c11[channel] as f32, tx);
        out[channel] = lerp(top, bottom, ty).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn sample_rgba(source: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * width as usize + x as usize) * 4;
    [
        source[index],
        source[index + 1],
        source[index + 2],
        source[index + 3],
    ]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + ((b - a) * t)
}

unsafe fn copy_diagnostic_camera_to_swapchain(
    vk_device: &ash::Device,
    cmd: vk::CommandBuffer,
    target: &Framebuffer,
    resolution: vk::Extent2D,
    camera: CameraCopy,
) {
    let copy_width = camera.width.min(resolution.width);
    let copy_height = camera.height.min(resolution.height);
    if copy_width == 0 || copy_height == 0 {
        return;
    }

    let range = vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: VIEW_COUNT,
    };
    let to_transfer = [vk::ImageMemoryBarrier::default()
        .image(target.image)
        .subresource_range(range)
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)];
    vk_device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &to_transfer,
    );

    let offset = vk::Offset3D {
        x: ((resolution.width - copy_width) / 2) as i32,
        y: ((resolution.height - copy_height) / 2) as i32,
        z: 0,
    };
    let extent = vk::Extent3D {
        width: copy_width,
        height: copy_height,
        depth: 1,
    };
    for layer in 0..VIEW_COUNT {
        let region = [vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: camera.width,
            buffer_image_height: camera.height,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: layer,
                layer_count: 1,
            },
            image_offset: offset,
            image_extent: extent,
        }];
        vk_device.cmd_copy_buffer_to_image(
            cmd,
            camera.buffer,
            target.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &region,
        );
    }

    let to_color = [vk::ImageMemoryBarrier::default()
        .image(target.image)
        .subresource_range(range)
        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ)];
    vk_device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &to_color,
    );
}

struct GpuCameraRenderer {
    ahb: Option<ash::android::external_memory_android_hardware_buffer::Device>,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    projection_uniform_alignment: vk::DeviceSize,
    render_pass: vk::RenderPass,
    resources: Option<GpuCameraPipelineResources>,
    imports: Vec<GpuCameraImport>,
    stereo_descriptors: Vec<GpuCameraStereoDescriptor>,
    import_success_count: u64,
    import_failure_count: u64,
    import_cache_hit_count: u64,
    import_cache_miss_count: u64,
    import_cache_evict_count: u64,
    last_failure: Option<String>,
}

impl GpuCameraRenderer {
    unsafe fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        memory_properties: vk::PhysicalDeviceMemoryProperties,
        projection_uniform_alignment: vk::DeviceSize,
        render_pass: vk::RenderPass,
        import_supported: bool,
    ) -> Self {
        let ahb = import_supported.then(|| {
            ash::android::external_memory_android_hardware_buffer::Device::new(instance, device)
        });
        Self {
            ahb,
            memory_properties,
            projection_uniform_alignment,
            render_pass,
            resources: None,
            imports: Vec::new(),
            stereo_descriptors: Vec::new(),
            import_success_count: 0,
            import_failure_count: 0,
            import_cache_hit_count: 0,
            import_cache_miss_count: 0,
            import_cache_evict_count: 0,
            last_failure: None,
        }
    }

    unsafe fn prepare_frame(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &HeadsetCameraGpuFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<Option<usize>, String> {
        if self.ahb.is_none() {
            self.last_failure = Some(
                "Vulkan Android hardware-buffer import or sampler YCbCr support missing"
                    .to_string(),
            );
            return Ok(None);
        }

        match self.prepare_frame_inner(
            device,
            cmd,
            frame,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        ) {
            Ok(index) => {
                self.import_success_count = self.import_success_count.saturating_add(1);
                self.last_failure = None;
                Ok(Some(index))
            }
            Err(error) => {
                self.import_failure_count = self.import_failure_count.saturating_add(1);
                self.last_failure = Some(error.clone());
                Err(error)
            }
        }
    }

    unsafe fn prepare_stereo_frame(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &StereoGpuCameraFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<Option<usize>, String> {
        if self.ahb.is_none() {
            self.last_failure = Some(
                "Vulkan Android hardware-buffer import or sampler YCbCr support missing"
                    .to_string(),
            );
            return Ok(None);
        }

        match self.prepare_stereo_frame_inner(
            device,
            cmd,
            frame,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        ) {
            Ok(index) => {
                self.import_success_count = self.import_success_count.saturating_add(1);
                self.last_failure = None;
                Ok(Some(index))
            }
            Err(error) => {
                self.import_failure_count = self.import_failure_count.saturating_add(1);
                self.last_failure = Some(error.clone());
                Err(error)
            }
        }
    }

    unsafe fn prepare_stereo_frame_inner(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &StereoGpuCameraFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<usize, String> {
        let import_cache_limit = effective_camera_import_cache_limit(import_cache_limit);
        let left_key = GpuCameraImportKey::from_frame(&frame.left);
        let right_key = GpuCameraImportKey::from_frame(&frame.right);
        let _left_index = self.prepare_frame_inner(
            device,
            cmd,
            &frame.left,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        )?;
        let _right_index = self.prepare_frame_inner(
            device,
            cmd,
            &frame.right,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        )?;

        if let Some(index) = self.stereo_descriptors.iter().position(|descriptor| {
            descriptor.left_key == left_key && descriptor.right_key == right_key
        }) {
            return Ok(index);
        }

        let left_import = self
            .imports
            .iter()
            .find(|import| import.key == left_key)
            .ok_or_else(|| {
                "left stereo camera import was evicted before descriptor binding".to_string()
            })?;
        let right_import = self
            .imports
            .iter()
            .find(|import| import.key == right_key)
            .ok_or_else(|| {
                "right stereo camera import was evicted before descriptor binding".to_string()
            })?;
        let resources = self
            .resources
            .as_ref()
            .ok_or_else(|| "GPU camera pipeline resources were not initialized".to_string())?;

        while self.stereo_descriptors.len() >= import_cache_limit {
            let old = self.stereo_descriptors.remove(0);
            old.destroy(device);
        }

        let descriptor_set = allocate_camera_descriptor_set(
            device,
            resources,
            left_import.image_view,
            right_import.image_view,
        )?;
        self.stereo_descriptors.push(GpuCameraStereoDescriptor {
            left_key,
            right_key,
            descriptor_set,
            descriptor_pool: resources.descriptor_pool,
        });
        Ok(self.stereo_descriptors.len() - 1)
    }

    unsafe fn prepare_frame_inner(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &HeadsetCameraGpuFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<usize, String> {
        let import_cache_limit = effective_camera_import_cache_limit(import_cache_limit);
        let key = GpuCameraImportKey::from_frame(frame);
        if let Some(index) = self.imports.iter().position(|import| import.key == key) {
            self.import_cache_hit_count = self.import_cache_hit_count.saturating_add(1);
            if self.imports[index].needs_layout_transition
                && self.resources.as_ref().is_some_and(|resources| {
                    resources
                        .format_key
                        .import_image_layout_mode
                        .needs_transition()
                })
            {
                transition_imported_camera_image(device, cmd, self.imports[index].image);
                self.imports[index].needs_layout_transition = false;
            }
            return Ok(index);
        }
        self.import_cache_miss_count = self.import_cache_miss_count.saturating_add(1);

        let mut format_props = vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
        let mut properties =
            vk::AndroidHardwareBufferPropertiesANDROID::default().push_next(&mut format_props);
        let ahb = self
            .ahb
            .as_ref()
            .ok_or_else(|| "Android hardware-buffer Vulkan extension is unavailable".to_string())?;
        ahb.get_android_hardware_buffer_properties(
            frame.hardware_buffer.as_ptr().cast(),
            &mut properties,
        )
        .map_err(|error| format!("query AHardwareBuffer Vulkan properties: {error}"))?;
        let allocation_size = properties.allocation_size;
        let memory_type_bits = properties.memory_type_bits;

        let format_key = GpuCameraFormatKey {
            format: if format_props.external_format != 0 {
                vk::Format::UNDEFINED
            } else {
                format_props.format
            },
            external_format: format_props.external_format,
            sampler_binding_mode,
            import_image_layout_mode,
        };
        if self
            .resources
            .as_ref()
            .map(|resources| resources.format_key != format_key)
            .unwrap_or(true)
        {
            self.destroy_stereo_descriptors(device);
            self.destroy_imports(device);
            self.destroy_resources(device);
            self.resources = Some(create_gpu_camera_pipeline_resources(
                device,
                &self.memory_properties,
                self.projection_uniform_alignment,
                self.render_pass,
                format_key,
                &format_props,
            )?);
        }

        while self.imports.len() >= import_cache_limit {
            let old = self.imports.remove(0);
            self.destroy_stereo_descriptors_for_key(device, old.key);
            old.destroy(device);
            self.import_cache_evict_count = self.import_cache_evict_count.saturating_add(1);
        }

        let resources = self
            .resources
            .as_ref()
            .ok_or_else(|| "GPU camera pipeline resources were not initialized".to_string())?;
        let import = import_camera_hardware_buffer(
            device,
            &self.memory_properties,
            resources,
            frame,
            key,
            format_key,
            allocation_size,
            memory_type_bits,
        )?;
        self.imports.push(import);
        let index = self.imports.len() - 1;
        if format_key.import_image_layout_mode.needs_transition() {
            transition_imported_camera_image(device, cmd, self.imports[index].image);
        }
        self.imports[index].needs_layout_transition = false;
        log_info(format!(
            "Rusty XR Vulkan imported Camera2 hardware buffer size={}x{} nativeFormat={} externalFormat={} vkFormat={:?} samplerBindingMode={} importImageLayout={} allocationSize={} memoryTypeBits=0x{:x} suggestedYcbcrModel={:?} suggestedYcbcrRange={:?} samplerYcbcrComponents={:?} suggestedXChromaOffset={:?} suggestedYChromaOffset={:?} importCacheSize={} importCacheLimit={} importCacheMiss={} importCacheEvict={}",
            frame.width,
            frame.height,
            frame.descriptor.native_format.unwrap_or_default(),
            format_key.external_format,
            format_key.format,
            format_key.sampler_binding_mode.stable_id(),
            format_key.import_image_layout_mode.stable_id(),
            allocation_size,
            memory_type_bits,
            format_props.suggested_ycbcr_model,
            format_props.suggested_ycbcr_range,
            format_props.sampler_ycbcr_conversion_components,
            format_props.suggested_x_chroma_offset,
            format_props.suggested_y_chroma_offset,
            self.imports.len(),
            import_cache_limit,
            self.import_cache_miss_count,
            self.import_cache_evict_count
        ));
        Ok(index)
    }

    unsafe fn record_draw(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        resolution: vk::Extent2D,
        import_index: usize,
        frame: &HeadsetCameraGpuFrame,
        config: &crate::RuntimeConfig,
    ) {
        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let Some(import) = self.imports.get(import_index) else {
            return;
        };

        let viewport = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: resolution.width as f32,
            height: resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissor = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: resolution,
        }];
        let push = CameraProjectionPush::from_frame(frame, config);
        let uniforms = CameraProjectionUniforms::identity().with_color_config(config);
        let uniform_offset = resources.projection_uniform_offset(0);
        if let Err(error) =
            update_camera_projection_uniforms(device, resources, uniform_offset, &uniforms)
        {
            log_error(format!(
                "Rusty XR update mono camera projection uniforms failed: {error}"
            ));
            return;
        }
        device.cmd_set_viewport(cmd, 0, &viewport);
        device.cmd_set_scissor(cmd, 0, &scissor);
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, resources.pipeline);
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_layout,
            0,
            &[import.descriptor_set],
            &[uniform_offset],
        );
        let push_bytes = std::slice::from_raw_parts(
            (&push as *const CameraProjectionPush).cast::<u8>(),
            std::mem::size_of::<CameraProjectionPush>(),
        );
        device.cmd_push_constants(
            cmd,
            resources.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            push_bytes,
        );
        device.cmd_draw(cmd, 3, 1, 0, 0);
    }

    unsafe fn record_draw_stereo(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        resolution: vk::Extent2D,
        descriptor_index: usize,
        frame: &StereoGpuCameraFrame,
        config: &crate::RuntimeConfig,
        views: &[xr::View],
        frame_count: u64,
    ) {
        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let Some(descriptor) = self.stereo_descriptors.get(descriptor_index) else {
            return;
        };

        let viewport = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: resolution.width as f32,
            height: resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissor = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: resolution,
        }];
        let controls = config.stereo_projection_controls(frame_count);
        let (push, uniforms, projection_active) =
            CameraProjectionPush::from_stereo_frame(frame, config, &controls, views, resolution);
        if config.camera_tier == CameraCompositeTier::GpuProjected && !projection_active {
            return;
        }
        let uniform_offset = resources.projection_uniform_offset(frame_count);
        if let Err(error) =
            update_camera_projection_uniforms(device, resources, uniform_offset, &uniforms)
        {
            log_error(format!(
                "Rusty XR update stereo camera projection uniforms failed: {error}"
            ));
            return;
        }
        device.cmd_set_viewport(cmd, 0, &viewport);
        device.cmd_set_scissor(cmd, 0, &scissor);
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, resources.pipeline);
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_layout,
            0,
            &[descriptor.descriptor_set],
            &[uniform_offset],
        );
        let push_bytes = std::slice::from_raw_parts(
            (&push as *const CameraProjectionPush).cast::<u8>(),
            std::mem::size_of::<CameraProjectionPush>(),
        );
        device.cmd_push_constants(
            cmd,
            resources.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            push_bytes,
        );
        device.cmd_draw(cmd, 3, 1, 0, 0);
    }

    unsafe fn destroy(&mut self, device: &ash::Device) {
        self.destroy_imports(device);
        self.destroy_resources(device);
    }

    unsafe fn destroy_imports(&mut self, device: &ash::Device) {
        self.destroy_stereo_descriptors(device);
        for import in self.imports.drain(..) {
            import.destroy(device);
        }
    }

    unsafe fn destroy_stereo_descriptors_for_key(
        &mut self,
        device: &ash::Device,
        key: GpuCameraImportKey,
    ) {
        let mut index = 0;
        while index < self.stereo_descriptors.len() {
            if self.stereo_descriptors[index].left_key == key
                || self.stereo_descriptors[index].right_key == key
            {
                let old = self.stereo_descriptors.remove(index);
                old.destroy(device);
            } else {
                index += 1;
            }
        }
    }

    unsafe fn destroy_stereo_descriptors(&mut self, device: &ash::Device) {
        for descriptor in self.stereo_descriptors.drain(..) {
            descriptor.destroy(device);
        }
    }

    unsafe fn destroy_resources(&mut self, device: &ash::Device) {
        if let Some(resources) = self.resources.take() {
            resources.destroy(device);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GpuCameraFormatKey {
    format: vk::Format,
    external_format: u64,
    sampler_binding_mode: crate::CameraSamplerBindingMode,
    import_image_layout_mode: crate::CameraImportImageLayoutMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GpuCameraImportKey {
    buffer_id: u64,
    width: u32,
    height: u32,
    native_format: u64,
}

impl GpuCameraImportKey {
    fn from_frame(frame: &HeadsetCameraGpuFrame) -> Self {
        Self {
            buffer_id: frame
                .descriptor
                .buffer_id
                .unwrap_or(frame.timestamp_ns as u64),
            width: frame.width,
            height: frame.height,
            native_format: frame.descriptor.native_format.unwrap_or_default(),
        }
    }
}

struct GpuCameraPipelineResources {
    format_key: GpuCameraFormatKey,
    sampler_ycbcr_conversion: vk::SamplerYcbcrConversion,
    sampler: vk::Sampler,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    projection_uniform_buffer: vk::Buffer,
    projection_uniform_memory: vk::DeviceMemory,
    projection_uniform_stride: vk::DeviceSize,
    projection_uniform_slots: u32,
}

impl GpuCameraPipelineResources {
    unsafe fn destroy(self, device: &ash::Device) {
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_descriptor_pool(self.descriptor_pool, None);
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        device.destroy_buffer(self.projection_uniform_buffer, None);
        device.free_memory(self.projection_uniform_memory, None);
        device.destroy_sampler(self.sampler, None);
        device.destroy_sampler_ycbcr_conversion(self.sampler_ycbcr_conversion, None);
    }

    fn projection_uniform_offset(&self, frame_count: u64) -> u32 {
        let slot = frame_count % self.projection_uniform_slots.max(1) as u64;
        (slot * self.projection_uniform_stride) as u32
    }
}

struct GpuCameraImport {
    key: GpuCameraImportKey,
    image: vk::Image,
    memory: vk::DeviceMemory,
    image_view: vk::ImageView,
    descriptor_set: vk::DescriptorSet,
    descriptor_pool: vk::DescriptorPool,
    needs_layout_transition: bool,
    _hardware_buffer: crate::AndroidHardwareBufferHandle,
}

struct GpuCameraStereoDescriptor {
    left_key: GpuCameraImportKey,
    right_key: GpuCameraImportKey,
    descriptor_set: vk::DescriptorSet,
    descriptor_pool: vk::DescriptorPool,
}

impl GpuCameraImport {
    unsafe fn destroy(self, device: &ash::Device) {
        let _ = device.free_descriptor_sets(self.descriptor_pool, &[self.descriptor_set]);
        device.destroy_image_view(self.image_view, None);
        device.destroy_image(self.image, None);
        device.free_memory(self.memory, None);
    }
}

impl GpuCameraStereoDescriptor {
    unsafe fn destroy(self, device: &ash::Device) {
        let _ = device.free_descriptor_sets(self.descriptor_pool, &[self.descriptor_set]);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CameraProjectionPush {
    params: [f32; 4],
    color_adjust: [f32; 4],
    left_h0: [f32; 4],
    left_h1: [f32; 4],
    left_h2: [f32; 4],
    right_h0: [f32; 4],
    right_h1: [f32; 4],
    right_h2: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CameraProjectionUniforms {
    left_screen_to_surface_h0: [f32; 4],
    left_screen_to_surface_h1: [f32; 4],
    left_screen_to_surface_h2: [f32; 4],
    right_screen_to_surface_h0: [f32; 4],
    right_screen_to_surface_h1: [f32; 4],
    right_screen_to_surface_h2: [f32; 4],
    left_surface_to_screen_h0: [f32; 4],
    left_surface_to_screen_h1: [f32; 4],
    left_surface_to_screen_h2: [f32; 4],
    right_surface_to_screen_h0: [f32; 4],
    right_surface_to_screen_h1: [f32; 4],
    right_surface_to_screen_h2: [f32; 4],
    color_matrix_r0: [f32; 4],
    color_matrix_r1: [f32; 4],
    color_matrix_r2: [f32; 4],
    color_offset: [f32; 4],
}

impl CameraProjectionUniforms {
    fn identity() -> Self {
        let h = identity_homography();
        Self::from_rows(&h, &h, &h, &h)
    }

    fn from_mappings(
        left: &DisplayEyeProjectionMapping,
        right: &DisplayEyeProjectionMapping,
    ) -> Self {
        Self::from_rows(
            &left.screen_to_surface,
            &right.screen_to_surface,
            &left.surface_to_screen,
            &right.surface_to_screen,
        )
    }

    fn from_rows(
        left_screen_to_surface: &[[f32; 3]; 3],
        right_screen_to_surface: &[[f32; 3]; 3],
        left_surface_to_screen: &[[f32; 3]; 3],
        right_surface_to_screen: &[[f32; 3]; 3],
    ) -> Self {
        Self {
            left_screen_to_surface_h0: pack_homography_row(left_screen_to_surface[0]),
            left_screen_to_surface_h1: pack_homography_row(left_screen_to_surface[1]),
            left_screen_to_surface_h2: pack_homography_row(left_screen_to_surface[2]),
            right_screen_to_surface_h0: pack_homography_row(right_screen_to_surface[0]),
            right_screen_to_surface_h1: pack_homography_row(right_screen_to_surface[1]),
            right_screen_to_surface_h2: pack_homography_row(right_screen_to_surface[2]),
            left_surface_to_screen_h0: pack_homography_row(left_surface_to_screen[0]),
            left_surface_to_screen_h1: pack_homography_row(left_surface_to_screen[1]),
            left_surface_to_screen_h2: pack_homography_row(left_surface_to_screen[2]),
            right_surface_to_screen_h0: pack_homography_row(right_surface_to_screen[0]),
            right_surface_to_screen_h1: pack_homography_row(right_surface_to_screen[1]),
            right_surface_to_screen_h2: pack_homography_row(right_surface_to_screen[2]),
            color_matrix_r0: [1.0, 0.0, 0.0, 0.0],
            color_matrix_r1: [0.0, 1.0, 0.0, 0.0],
            color_matrix_r2: [0.0, 0.0, 1.0, 0.0],
            color_offset: [0.0, 0.0, 0.0, 0.0],
        }
    }

    fn with_color_config(mut self, config: &crate::RuntimeConfig) -> Self {
        self.color_matrix_r0 = [
            config.camera_color_matrix[0][0],
            config.camera_color_matrix[0][1],
            config.camera_color_matrix[0][2],
            0.0,
        ];
        self.color_matrix_r1 = [
            config.camera_color_matrix[1][0],
            config.camera_color_matrix[1][1],
            config.camera_color_matrix[1][2],
            0.0,
        ];
        self.color_matrix_r2 = [
            config.camera_color_matrix[2][0],
            config.camera_color_matrix[2][1],
            config.camera_color_matrix[2][2],
            0.0,
        ];
        self.color_offset = [
            config.camera_color_offset[0],
            config.camera_color_offset[1],
            config.camera_color_offset[2],
            0.0,
        ];
        self
    }
}

impl CameraProjectionPush {
    fn from_frame(_frame: &HeadsetCameraGpuFrame, config: &crate::RuntimeConfig) -> Self {
        let mono_flags = config.camera_texture_transform.shader_flags() & 0x1f;
        let packed_flags = (mono_flags | (mono_flags << 5))
            | config.camera_color_mode.shader_bit()
            | config.camera_feed_pipeline_mode.shader_bit()
            | config.camera_projection_effect_mode.shader_bit();
        let content_uv_scale = full_view_content_uv_scale(
            config.camera_full_view_overlay_overscan,
            config.camera_raw_overlay_overscan,
        )
        .unwrap_or(1.0);
        Self {
            params: [
                config.camera_raw_overlay_overscan.max(1.0),
                config.camera_edge_fade.clamp(0.0, 0.5),
                content_uv_scale,
                packed_flags as f32,
            ],
            color_adjust: config.camera_color_adjust_push(),
            left_h0: [1.0, 0.0, 0.0, 0.0],
            left_h1: [0.0, 1.0, 0.0, 0.0],
            left_h2: [0.0, 0.0, 1.0, 0.0],
            right_h0: [1.0, 0.0, 0.0, 0.0],
            right_h1: [0.0, 1.0, 0.0, 0.0],
            right_h2: [0.0, 0.0, 1.0, 0.0],
        }
    }

    fn from_stereo_frame(
        frame: &StereoGpuCameraFrame,
        config: &crate::RuntimeConfig,
        controls: &crate::StereoProjectionControls,
        views: &[xr::View],
        resolution: vk::Extent2D,
    ) -> (Self, CameraProjectionUniforms, bool) {
        let content_uv_scale = full_view_content_uv_scale(
            config.camera_full_view_overlay_overscan,
            config.camera_raw_overlay_overscan,
        )
        .unwrap_or(1.0);
        let mut push = Self {
            params: [
                config.camera_raw_overlay_overscan.max(1.0),
                config.camera_edge_fade.clamp(0.0, 0.5),
                content_uv_scale,
                (controls.packed_shader_flags()
                    | config.camera_color_mode.shader_bit()
                    | config.camera_feed_pipeline_mode.shader_bit()
                    | config.camera_projection_effect_mode.shader_bit()) as f32,
            ],
            color_adjust: config.camera_color_adjust_push(),
            left_h0: [1.0, 0.0, 0.0, 0.0],
            left_h1: [0.0, 1.0, 0.0, 0.0],
            left_h2: [0.0, 0.0, 1.0, 0.0],
            right_h0: [1.0, 0.0, 0.0, 0.0],
            right_h1: [0.0, 1.0, 0.0, 0.0],
            right_h2: [0.0, 0.0, 1.0, 0.0],
        };
        if !controls.left_texture_transform.is_explicit_visual_check()
            || !controls.right_texture_transform.is_explicit_visual_check()
        {
            return (
                push,
                CameraProjectionUniforms::identity().with_color_config(config),
                false,
            );
        }

        if let Some((left, right)) =
            projected_stereo_homographies(frame, config, controls, views, resolution)
        {
            push.params[0] = -config.camera_raw_overlay_overscan.max(1.0);
            push.left_h0 = pack_homography_row(left.screen_to_camera[0]);
            push.left_h1 = pack_homography_row(left.screen_to_camera[1]);
            push.left_h2 = pack_homography_row(left.screen_to_camera[2]);
            push.right_h0 = pack_homography_row(right.screen_to_camera[0]);
            push.right_h1 = pack_homography_row(right.screen_to_camera[1]);
            push.right_h2 = pack_homography_row(right.screen_to_camera[2]);
            return (
                push,
                CameraProjectionUniforms::from_mappings(&left, &right).with_color_config(config),
                true,
            );
        }
        (
            push,
            CameraProjectionUniforms::identity().with_color_config(config),
            false,
        )
    }
}

#[derive(Clone, Copy)]
struct DisplayEyeProjectionMapping {
    screen_to_camera: [[f32; 3]; 3],
    screen_to_surface: [[f32; 3]; 3],
    surface_to_screen: [[f32; 3]; 3],
}

fn identity_homography() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

fn pack_homography_row(row: [f32; 3]) -> [f32; 4] {
    [row[0], row[1], row[2], 0.0]
}

fn projected_stereo_homographies(
    frame: &StereoGpuCameraFrame,
    config: &crate::RuntimeConfig,
    controls: &crate::StereoProjectionControls,
    views: &[xr::View],
    resolution: vk::Extent2D,
) -> Option<(DisplayEyeProjectionMapping, DisplayEyeProjectionMapping)> {
    let left_extrinsics = frame.left.metadata.extrinsics?;
    let right_extrinsics = frame.right.metadata.extrinsics?;
    if !left_extrinsics.is_valid() || !right_extrinsics.is_valid() {
        return None;
    }
    let reference_center = (left_extrinsics.world_from_camera.position
        + right_extrinsics.world_from_camera.position)
        * 0.5;
    let left_view = views.first()?;
    let right_view = views.get(1).unwrap_or(left_view);
    let (display_left_source, display_right_source) = match controls.source_eye_mapping {
        crate::StereoSourceEyeMapping::DisplayLeftFromLeftSource => (&frame.left, &frame.right),
        crate::StereoSourceEyeMapping::DisplayLeftFromRightSource => (&frame.right, &frame.left),
    };
    let left = projected_display_eye_homography(
        display_left_source,
        config,
        views,
        left_view,
        resolution,
        reference_center,
    )?;
    let right = projected_display_eye_homography(
        display_right_source,
        config,
        views,
        right_view,
        resolution,
        reference_center,
    )?;
    Some((left, right))
}

fn projected_display_eye_homography(
    frame: &HeadsetCameraGpuFrame,
    config: &crate::RuntimeConfig,
    views: &[xr::View],
    display_view: &xr::View,
    resolution: vk::Extent2D,
    reference_center: Vec3,
) -> Option<DisplayEyeProjectionMapping> {
    let intrinsics = frame.metadata.intrinsics?;
    let source_domain = frame.metadata.intrinsics_domain?;
    let scaled = scale_intrinsics_to_image(
        intrinsics,
        source_domain.size,
        frame.metadata.delivered_size,
    )
    .ok()?;
    let width = frame.metadata.delivered_size.width as f32;
    let height = frame.metadata.delivered_size.height as f32;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let extrinsics = frame.metadata.extrinsics?;
    if !extrinsics.is_valid() {
        return None;
    }
    let tracking = tracking_basis_from_views(views)?;
    let aspect = views
        .first()
        .and_then(|view| fov_aspect(view.fov))
        .unwrap_or_else(|| {
            if resolution.height == 0 {
                1.0
            } else {
                resolution.width as f32 / resolution.height as f32
            }
        })
        .clamp(0.25, 4.0);
    // Build the homography over the camera-content surface, not the larger
    // visible full-view surface. The fragment shader expands full-view UVs
    // into content UVs before applying this homography, matching a real
    // head-anchored overlay whose border may extend beyond the camera-covered
    // content region.
    let surface_corners = head_anchored_preview_surface_corners(
        tracking,
        config.camera_preview_fov_y_degrees,
        config.camera_projection_scale.max(0.05),
        aspect,
        config.camera_raw_overlay_overscan,
    )
    .ok()?;
    let camera_basis = camera_basis_from_camera2_reference_pose_relative_to_center(
        tracking,
        extrinsics,
        reference_center,
    )
    .ok()?;
    let eye_basis = eye_basis_from_view(display_view)?;
    let surface_to_screen = surface_to_eye_screen_uv_homography(
        surface_corners,
        eye_basis,
        display_view.fov.angle_left.tan(),
        display_view.fov.angle_right.tan(),
        display_view.fov.angle_down.tan(),
        display_view.fov.angle_up.tan(),
    )
    .ok()?;
    let surface_to_camera =
        surface_to_camera_uv_homography(surface_corners, camera_basis, scaled).ok()?;
    // Both public projection modes render through the same fullscreen
    // multiview pass today. Reconstruct the head-anchored content-surface UV
    // from the current display-eye geometry so the shader samples the camera
    // feed as if a real quad had supplied rasterized surface coordinates.
    // The mode remains visible in logs/catalogs so a future mesh-quad backend
    // can be A/B tested without changing launch profiles.
    let screen_to_surface = invert_homography(surface_to_screen)?;
    let screen_to_camera =
        screen_to_camera_uv_homography(surface_to_screen, surface_to_camera).ok()?;
    Some(DisplayEyeProjectionMapping {
        screen_to_camera,
        screen_to_surface,
        surface_to_screen,
    })
}

fn eye_basis_from_view(view: &xr::View) -> Option<CameraBasis> {
    let orientation = Quat::new(
        view.pose.orientation.x,
        view.pose.orientation.y,
        view.pose.orientation.z,
        view.pose.orientation.w,
    )
    .normalized_or(Quat::IDENTITY);
    CameraBasis::new(
        Vec3::new(
            view.pose.position.x,
            view.pose.position.y,
            view.pose.position.z,
        ),
        orientation.rotate_vec3(Vec3::RIGHT),
        orientation.rotate_vec3(Vec3::UP),
        orientation.rotate_vec3(Vec3::FORWARD_NEG_Z),
    )
}

fn tracking_basis_from_views(views: &[xr::View]) -> Option<TrackingBasis> {
    let first = views.first()?;
    let position = if views.len() >= 2 {
        let left = views[0].pose.position;
        let right = views[1].pose.position;
        Vec3::new(
            (left.x + right.x) * 0.5,
            (left.y + right.y) * 0.5,
            (left.z + right.z) * 0.5,
        )
    } else {
        Vec3::new(
            first.pose.position.x,
            first.pose.position.y,
            first.pose.position.z,
        )
    };
    let orientation = Quat::new(
        first.pose.orientation.x,
        first.pose.orientation.y,
        first.pose.orientation.z,
        first.pose.orientation.w,
    )
    .normalized_or(Quat::IDENTITY);
    TrackingBasis::new(
        position,
        orientation.rotate_vec3(Vec3::RIGHT),
        orientation.rotate_vec3(Vec3::UP),
        orientation.rotate_vec3(Vec3::FORWARD_NEG_Z),
    )
}

fn fov_aspect(fov: xr::Fovf) -> Option<f32> {
    let width = fov.angle_right.tan() - fov.angle_left.tan();
    let height = fov.angle_up.tan() - fov.angle_down.tan();
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        Some(width / height)
    } else {
        None
    }
}

unsafe fn create_gpu_camera_pipeline_resources(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    projection_uniform_alignment: vk::DeviceSize,
    render_pass: vk::RenderPass,
    format_key: GpuCameraFormatKey,
    format_props: &vk::AndroidHardwareBufferFormatPropertiesANDROID<'_>,
) -> Result<GpuCameraPipelineResources, String> {
    let mut external_format =
        vk::ExternalFormatANDROID::default().external_format(format_key.external_format);
    let mut conversion_info = vk::SamplerYcbcrConversionCreateInfo::default()
        .format(format_key.format)
        .ycbcr_model(format_props.suggested_ycbcr_model)
        .ycbcr_range(format_props.suggested_ycbcr_range)
        .components(format_props.sampler_ycbcr_conversion_components)
        .x_chroma_offset(format_props.suggested_x_chroma_offset)
        .y_chroma_offset(format_props.suggested_y_chroma_offset)
        .chroma_filter(vk::Filter::LINEAR);
    if format_key.external_format != 0 {
        conversion_info = conversion_info.push_next(&mut external_format);
    }
    let sampler_ycbcr_conversion = device
        .create_sampler_ycbcr_conversion(&conversion_info, None)
        .map_err(|error| format!("create camera sampler YCbCr conversion: {error}"))?;

    let mut sampler_conversion_info =
        vk::SamplerYcbcrConversionInfo::default().conversion(sampler_ycbcr_conversion);
    let sampler = device
        .create_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .push_next(&mut sampler_conversion_info),
            None,
        )
        .map_err(|error| format!("create camera sampler: {error}"))?;

    let immutable_samplers = [sampler];
    let descriptor_binding = match format_key.sampler_binding_mode {
        crate::CameraSamplerBindingMode::CombinedImmutableSampler => vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .immutable_samplers(&immutable_samplers),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .immutable_samplers(&immutable_samplers),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ],
        crate::CameraSamplerBindingMode::SeparateImageSampler => vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(3)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ],
    };
    let descriptor_set_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_binding),
            None,
        )
        .map_err(|error| format!("create camera descriptor set layout: {error}"))?;
    let max_descriptor_sets = (GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX as u32) * 2;
    let pool_sizes = match format_key.sampler_binding_mode {
        crate::CameraSamplerBindingMode::CombinedImmutableSampler => vec![
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count((GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX as u32) * 4),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(max_descriptor_sets),
        ],
        crate::CameraSamplerBindingMode::SeparateImageSampler => vec![
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count((GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX as u32) * 4),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(max_descriptor_sets),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(max_descriptor_sets),
        ],
    };
    let descriptor_pool = device
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .pool_sizes(&pool_sizes)
                .max_sets(max_descriptor_sets),
            None,
        )
        .map_err(|error| format!("create camera descriptor pool: {error}"))?;
    let (projection_uniform_buffer, projection_uniform_memory, projection_uniform_stride) =
        create_camera_projection_uniform_buffer(
            device,
            memory_properties,
            projection_uniform_alignment,
        )?;

    let push_ranges = [vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(std::mem::size_of::<CameraProjectionPush>() as u32)];
    let set_layouts = [descriptor_set_layout];
    let pipeline_layout = device
        .create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&set_layouts)
                .push_constant_ranges(&push_ranges),
            None,
        )
        .map_err(|error| format!("create camera pipeline layout: {error}"))?;
    let pipeline = create_gpu_camera_pipeline(
        device,
        render_pass,
        pipeline_layout,
        format_key.sampler_binding_mode,
    )?;

    Ok(GpuCameraPipelineResources {
        format_key,
        sampler_ycbcr_conversion,
        sampler,
        descriptor_set_layout,
        descriptor_pool,
        pipeline_layout,
        pipeline,
        projection_uniform_buffer,
        projection_uniform_memory,
        projection_uniform_stride,
        projection_uniform_slots: GPU_CAMERA_PROJECTION_UNIFORM_SLOTS,
    })
}

unsafe fn allocate_camera_descriptor_set(
    device: &ash::Device,
    resources: &GpuCameraPipelineResources,
    left_image_view: vk::ImageView,
    right_image_view: vk::ImageView,
) -> Result<vk::DescriptorSet, String> {
    let descriptor_set_layouts = [resources.descriptor_set_layout];
    let descriptor_set = match device.allocate_descriptor_sets(
        &vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(resources.descriptor_pool)
            .set_layouts(&descriptor_set_layouts),
    ) {
        Ok(mut sets) => sets
            .pop()
            .ok_or_else(|| "camera descriptor allocation returned no set".to_string())?,
        Err(error) => {
            return Err(format!("allocate camera descriptor set: {error}"));
        }
    };
    let image_layout =
        camera_import_descriptor_layout(resources.format_key.import_image_layout_mode);
    let left_info = [vk::DescriptorImageInfo::default()
        .sampler(resources.sampler)
        .image_view(left_image_view)
        .image_layout(image_layout)];
    let right_info = [vk::DescriptorImageInfo::default()
        .sampler(resources.sampler)
        .image_view(right_image_view)
        .image_layout(image_layout)];
    let projection_info = [vk::DescriptorBufferInfo::default()
        .buffer(resources.projection_uniform_buffer)
        .offset(0)
        .range(std::mem::size_of::<CameraProjectionUniforms>() as vk::DeviceSize)];
    match resources.format_key.sampler_binding_mode {
        crate::CameraSamplerBindingMode::CombinedImmutableSampler => {
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&left_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&right_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                    .buffer_info(&projection_info),
            ];
            device.update_descriptor_sets(&writes, &[]);
        }
        crate::CameraSamplerBindingMode::SeparateImageSampler => {
            let left_sampled_image = [vk::DescriptorImageInfo::default()
                .image_view(left_image_view)
                .image_layout(image_layout)];
            let right_sampled_image = [vk::DescriptorImageInfo::default()
                .image_view(right_image_view)
                .image_layout(image_layout)];
            let sampler_info = [vk::DescriptorImageInfo::default().sampler(resources.sampler)];
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&left_sampled_image),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&right_sampled_image),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                    .buffer_info(&projection_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(3)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&sampler_info),
            ];
            device.update_descriptor_sets(&writes, &[]);
        }
    }
    Ok(descriptor_set)
}

unsafe fn create_camera_projection_uniform_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    min_uniform_alignment: vk::DeviceSize,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::DeviceSize), String> {
    let uniform_size = std::mem::size_of::<CameraProjectionUniforms>() as vk::DeviceSize;
    let stride = align_uniform_stride(uniform_size, min_uniform_alignment.max(16));
    let total_size = stride * GPU_CAMERA_PROJECTION_UNIFORM_SLOTS.max(1) as vk::DeviceSize;
    let buffer = device
        .create_buffer(
            &vk::BufferCreateInfo::default()
                .size(total_size)
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            None,
        )
        .map_err(|error| format!("create camera projection uniform buffer: {error}"))?;
    let requirements = device.get_buffer_memory_requirements(buffer);
    let memory_type_index = match find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    ) {
        Ok(index) => index,
        Err(error) => {
            device.destroy_buffer(buffer, None);
            return Err(error);
        }
    };
    let memory = match device.allocate_memory(
        &vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index),
        None,
    ) {
        Ok(memory) => memory,
        Err(error) => {
            device.destroy_buffer(buffer, None);
            return Err(format!(
                "allocate camera projection uniform memory: {error}"
            ));
        }
    };
    if let Err(error) = device.bind_buffer_memory(buffer, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_buffer(buffer, None);
        return Err(format!("bind camera projection uniform memory: {error}"));
    }
    Ok((buffer, memory, stride))
}

fn align_uniform_stride(value: vk::DeviceSize, alignment: vk::DeviceSize) -> vk::DeviceSize {
    if alignment <= 1 {
        value
    } else {
        ((value + alignment - 1) / alignment) * alignment
    }
}

unsafe fn update_camera_projection_uniforms(
    device: &ash::Device,
    resources: &GpuCameraPipelineResources,
    offset: u32,
    uniforms: &CameraProjectionUniforms,
) -> Result<(), String> {
    let byte_len = std::mem::size_of::<CameraProjectionUniforms>() as vk::DeviceSize;
    let mapped = device
        .map_memory(
            resources.projection_uniform_memory,
            offset as vk::DeviceSize,
            byte_len,
            vk::MemoryMapFlags::empty(),
        )
        .map_err(|error| format!("map camera projection uniform memory: {error}"))?;
    std::ptr::copy_nonoverlapping(
        (uniforms as *const CameraProjectionUniforms).cast::<u8>(),
        mapped.cast::<u8>(),
        byte_len as usize,
    );
    device.unmap_memory(resources.projection_uniform_memory);
    Ok(())
}

unsafe fn import_camera_hardware_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    resources: &GpuCameraPipelineResources,
    frame: &HeadsetCameraGpuFrame,
    key: GpuCameraImportKey,
    format_key: GpuCameraFormatKey,
    allocation_size: vk::DeviceSize,
    memory_type_bits: u32,
) -> Result<GpuCameraImport, String> {
    let mut external_memory = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);
    let mut external_format =
        vk::ExternalFormatANDROID::default().external_format(format_key.external_format);
    let mut image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format_key.format)
        .extent(vk::Extent3D {
            width: frame.width,
            height: frame.height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .push_next(&mut external_memory);
    if format_key.external_format != 0 {
        image_info = image_info.push_next(&mut external_format);
    }
    let image = device
        .create_image(&image_info, None)
        .map_err(|error| format!("create imported camera image: {error}"))?;

    let memory_type_index = match find_memory_type_relaxed(memory_properties, memory_type_bits) {
        Ok(index) => index,
        Err(error) => {
            device.destroy_image(image, None);
            return Err(error);
        }
    };
    let mut import_info = vk::ImportAndroidHardwareBufferInfoANDROID::default()
        .buffer(frame.hardware_buffer.as_ptr().cast());
    let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
    let memory = match device.allocate_memory(
        &vk::MemoryAllocateInfo::default()
            .allocation_size(allocation_size)
            .memory_type_index(memory_type_index)
            .push_next(&mut import_info)
            .push_next(&mut dedicated),
        None,
    ) {
        Ok(memory) => memory,
        Err(error) => {
            device.destroy_image(image, None);
            return Err(format!("allocate imported camera memory: {error}"));
        }
    };
    if let Err(error) = device.bind_image_memory(image, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_image(image, None);
        return Err(format!("bind imported camera memory: {error}"));
    }

    let mut view_conversion =
        vk::SamplerYcbcrConversionInfo::default().conversion(resources.sampler_ycbcr_conversion);
    let image_view = match device.create_image_view(
        &vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format_key.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .push_next(&mut view_conversion),
        None,
    ) {
        Ok(image_view) => image_view,
        Err(error) => {
            device.free_memory(memory, None);
            device.destroy_image(image, None);
            return Err(format!("create imported camera image view: {error}"));
        }
    };

    let descriptor_set =
        match allocate_camera_descriptor_set(device, resources, image_view, image_view) {
            Ok(descriptor_set) => descriptor_set,
            Err(error) => {
                device.destroy_image_view(image_view, None);
                device.free_memory(memory, None);
                device.destroy_image(image, None);
                return Err(error);
            }
        };

    Ok(GpuCameraImport {
        key,
        image,
        memory,
        image_view,
        descriptor_set,
        descriptor_pool: resources.descriptor_pool,
        needs_layout_transition: format_key.import_image_layout_mode.needs_transition(),
        _hardware_buffer: frame.hardware_buffer.clone(),
    })
}

unsafe fn create_gpu_camera_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    sampler_binding_mode: crate::CameraSamplerBindingMode,
) -> Result<vk::Pipeline, String> {
    let vertex_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/camera_projection.vert.spv"
    )))?;
    let fragment_words = match sampler_binding_mode {
        crate::CameraSamplerBindingMode::CombinedImmutableSampler => spirv_words(include_bytes!(
            concat!(env!("OUT_DIR"), "/camera_projection.frag.spv")
        ))?,
        crate::CameraSamplerBindingMode::SeparateImageSampler => {
            spirv_words(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/camera_projection_separate_sampler.frag.spv"
            )))?
        }
    };
    let vertex_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&vertex_words),
            None,
        )
        .map_err(|error| format!("create camera vertex shader module: {error}"))?;
    let fragment_module = match device.create_shader_module(
        &vk::ShaderModuleCreateInfo::default().code(&fragment_words),
        None,
    ) {
        Ok(module) => module,
        Err(error) => {
            device.destroy_shader_module(vertex_module, None);
            return Err(format!("create camera fragment shader module: {error}"));
        }
    };
    let entry = CString::new("main").expect("static shader entry point is valid");
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_module)
            .name(&entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_module)
            .name(&entry),
    ];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .line_width(1.0);
    let multisample = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let color_blend_attachment = [vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA)];
    let color_blend =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(&color_blend_attachment);
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::ALWAYS)
        .stencil_test_enable(false);
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let create_info = [vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisample)
        .color_blend_state(&color_blend)
        .depth_stencil_state(&depth_stencil)
        .dynamic_state(&dynamic)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)];
    let pipeline_result =
        device.create_graphics_pipelines(vk::PipelineCache::null(), &create_info, None);
    device.destroy_shader_module(fragment_module, None);
    device.destroy_shader_module(vertex_module, None);
    pipeline_result
        .map(|mut pipelines| pipelines.remove(0))
        .map_err(|(_, error)| format!("create camera graphics pipeline: {error}"))
}

fn spirv_words(bytes: &[u8]) -> Result<Vec<u32>, String> {
    if bytes.len() % 4 != 0 {
        return Err("SPIR-V bytecode length is not word-aligned".to_string());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn camera_import_descriptor_layout(mode: crate::CameraImportImageLayoutMode) -> vk::ImageLayout {
    match mode {
        crate::CameraImportImageLayoutMode::ShaderReadOnlyTransition => {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        }
        crate::CameraImportImageLayoutMode::GeneralNoTransition => vk::ImageLayout::GENERAL,
    }
}

unsafe fn transition_imported_camera_image(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
) {
    let barrier = [vk::ImageMemoryBarrier::default()
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        })
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::SHADER_READ)];
    device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &barrier,
    );
}

fn find_memory_type_relaxed(
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    memory_type_bits: u32,
) -> Result<u32, String> {
    find_memory_type(
        memory_properties,
        memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
    .or_else(|_| {
        for index in 0..memory_properties.memory_type_count {
            if (memory_type_bits & (1 << index)) != 0 {
                return Ok(index);
            }
        }
        Err(format!(
            "no Vulkan memory type supports imported Android hardware buffer bits 0x{memory_type_bits:x}"
        ))
    })
}

fn find_memory_type(
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    memory_type_bits: u32,
    required: vk::MemoryPropertyFlags,
) -> Result<u32, String> {
    for index in 0..memory_properties.memory_type_count {
        let supported = (memory_type_bits & (1 << index)) != 0;
        let flags = memory_properties.memory_types[index as usize].property_flags;
        if supported && flags.contains(required) {
            return Ok(index);
        }
    }

    Err(format!(
        "no Vulkan memory type supports {required:?} for headset camera upload"
    ))
}

fn wait_for_android_foreground(app: &android_activity::AndroidApp) -> Result<(), String> {
    let start = Instant::now();
    let mut state = AndroidForegroundState::default();
    log_info("Rusty XR waiting for Android resume/focus before OpenXR session setup");

    loop {
        app.poll_events(Some(Duration::from_millis(50)), |event| {
            if let PollEvent::Main(main_event) = event {
                handle_android_main_event(app, main_event, Some(&mut state), None);
            }
        });

        if state.destroyed {
            return Err("Android activity was destroyed before OpenXR setup".to_string());
        }
        if state.resumed && state.focused && state.has_window {
            log_info("Rusty XR Android activity is foreground; continuing OpenXR setup");
            return Ok(());
        }
        if start.elapsed() >= Duration::from_secs(10) {
            log_error(
                "Timed out waiting for Android focus before OpenXR setup; continuing best-effort",
            );
            return Ok(());
        }
    }
}

fn pump_android_events(app: &android_activity::AndroidApp, running: &mut bool) {
    app.poll_events(Some(Duration::from_millis(0)), |event| {
        if let PollEvent::Main(main_event) = event {
            handle_android_main_event(app, main_event, None, Some(running));
        }
    });
}

fn handle_android_main_event(
    app: &android_activity::AndroidApp,
    event: MainEvent<'_>,
    mut foreground: Option<&mut AndroidForegroundState>,
    running: Option<&mut bool>,
) {
    match event {
        MainEvent::InputAvailable => {
            drain_input_events(app);
        }
        MainEvent::InitWindow { .. } => {
            log_info("Rusty XR Android native window initialized");
            if let Some(state) = foreground.as_deref_mut() {
                state.has_window = true;
            }
        }
        MainEvent::TerminateWindow { .. } => {
            log_info("Rusty XR Android native window terminated");
            if let Some(state) = foreground.as_deref_mut() {
                state.has_window = false;
            }
        }
        MainEvent::Destroy => {
            log_info("Rusty XR Android activity destroy requested");
            if let Some(state) = foreground.as_deref_mut() {
                state.destroyed = true;
            }
            if let Some(running) = running {
                *running = false;
            }
        }
        MainEvent::Pause => {
            log_info("Rusty XR Android activity paused");
            if let Some(state) = foreground.as_deref_mut() {
                state.resumed = false;
            }
        }
        MainEvent::Resume { .. } => {
            log_info("Rusty XR Android activity resumed");
            if let Some(state) = foreground.as_deref_mut() {
                state.resumed = true;
            }
        }
        MainEvent::GainedFocus => {
            log_info("Rusty XR Android activity gained focus");
            if let Some(state) = foreground.as_deref_mut() {
                state.focused = true;
            }
        }
        MainEvent::LostFocus => {
            log_info("Rusty XR Android activity lost focus");
            if let Some(state) = foreground.as_deref_mut() {
                state.focused = false;
            }
        }
        _ => {}
    }
}

fn drain_input_events(app: &android_activity::AndroidApp) {
    match app.input_events_iter() {
        Ok(mut events) => loop {
            if !events.next(|_| InputStatus::Handled) {
                break;
            }
        },
        Err(error) => {
            log_error(format!("Rusty XR Android input drain failed: {error}"));
        }
    }
}

const VIEW_COUNT: u32 = 2;
const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;
const PIPELINE_DEPTH: u32 = 2;

#[derive(Default)]
struct AndroidForegroundState {
    resumed: bool,
    focused: bool,
    has_window: bool,
    destroyed: bool,
}

struct Swapchain {
    handle: xr::Swapchain<xr::Vulkan>,
    buffers: Vec<Framebuffer>,
    resolution: vk::Extent2D,
    foveation_enabled: bool,
}

struct Framebuffer {
    framebuffer: vk::Framebuffer,
    color: vk::ImageView,
    depth: Option<DepthAttachment>,
    fragment_density: vk::ImageView,
    image: vk::Image,
}

struct DepthAttachment {
    image: vk::Image,
    view: vk::ImageView,
    memory: vk::DeviceMemory,
}

struct CameraUpload {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    capacity: vk::DeviceSize,
}

#[derive(Clone, Copy)]
struct CameraCopy {
    buffer: vk::Buffer,
    width: u32,
    height: u32,
}
