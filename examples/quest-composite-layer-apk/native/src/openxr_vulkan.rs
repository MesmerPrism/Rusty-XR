use std::{
    ffi::{CStr, CString},
    ptr,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    diagnostic_hud_snapshot, gpu_probe_counters, latest_headset_camera_frame,
    latest_headset_camera_gpu_frame, latest_headset_stereo_camera_gpu_frame, log_error, log_info,
    runtime_config, EnvironmentDepthMode, HandParticleMode, HeadsetCameraFrame,
    HeadsetCameraGpuFrame, OpenXrColorFormatMode, OpenXrPassthroughProbeMode,
    OpenXrPassthroughStyleMode, RuntimeConfig, StereoGpuCameraFrame,
};
use android_activity::{InputStatus, MainEvent, PollEvent};
use ash::vk::{self, Handle};
use openxr as xr;
use openxr::sys::Handle as _;
use rusty_xr_camera_model::{
    camera_basis_from_camera2_reference_pose_relative_to_center, full_view_content_uv_scale,
    head_anchored_preview_surface_corners, invert_homography, project_camera_point,
    scale_intrinsics_to_image, screen_to_camera_uv_homography,
    stereo_homography_projection_metrics, surface_to_camera_uv_homography,
    surface_to_eye_screen_uv_homography, CameraBasis, CameraCompositeTier, CameraPixelDomain,
    ImageSize, Quat, StereoHomographyProjection, TrackingBasis, Vec3,
};
use rusty_xr_debug_canvas::{
    CanvasBadge, CanvasDocument, CanvasDrawList, CanvasLayout, CanvasSection, CanvasTextRun,
    CanvasTheme, CanvasTone, DiagnosticHudUpdate,
};
use rusty_xr_particles::{
    ColorRgba, HandMeshSnapshot, Handedness, LiveHandMeshParticleSampler, LiveHandMeshUpdateStatus,
    MeshSurfaceCrossNeighborConfig, MeshSurfaceSampleConfig, ParticleRender,
};

const CAMERA_CPU_COPY_MAX_DIMENSION: u32 = 640;
const CAMERA_CPU_UPLOAD_MIN_INTERVAL_NS: i64 = 250_000_000;
const CAMERA_CPU_UPLOAD_HZ_LABEL: u32 = 4;
const XR_RENDER_SCALE_DEFAULT: f32 = 0.75;
const GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = crate::CAMERA_IMPORT_CACHE_LIMIT_MAX;
const GPU_CAMERA_PROJECTION_UNIFORM_SLOTS: u32 = 3;
const XR_FRAGMENT_DENSITY_MAP_FORMAT: vk::Format = vk::Format::R8G8_UNORM;
const XR_FOVEATION_DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
const XR_ENVIRONMENT_DEPTH_FORMAT: vk::Format = vk::Format::D16_UNORM;
const XR_ENVIRONMENT_DEPTH_VISUAL_MAX_METERS: f32 = 20.0;
const XR_ENVIRONMENT_DEPTH_MESH_DISTANCE_GRADIENT_MAX_METERS: f32 = 3.0;
const XR_ENVIRONMENT_DEPTH_MESH_CELL_METERS: f32 = 0.14;
const XR_ENVIRONMENT_DEPTH_MESH_DISCONTINUITY_METERS: f32 = 0.35;
const XR_ENVIRONMENT_DEPTH_MESH_GRID_STRIDE_PIXELS: u32 = 4;
const XR_ENVIRONMENT_DEPTH_MESH_HISTORY_FRAMES: usize = 1;
const XR_ENVIRONMENT_DEPTH_MESH_HISTORY_MAX_AGE_NS: i64 = 0;
const XR_ENVIRONMENT_DEPTH_MESH_HISTORY_MIN_ALPHA: f32 = 0.24;
const XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY: u32 = 32_768;
const XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS: u32 = 12;
const XR_ENVIRONMENT_DEPTH_PARTICLE_SOURCE_VIEW_COUNT: u32 = 1;
const XR_ENVIRONMENT_DEPTH_PARTICLE_DISCONTINUITY_METERS: f32 = 0.28;
const XR_ENVIRONMENT_DEPTH_PARTICLE_HALF_SIZE_MIN_METERS: f32 = 0.002;
const XR_ENVIRONMENT_DEPTH_PARTICLE_HALF_SIZE_MAX_METERS: f32 = 0.004;
const XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_CELL_METERS: f32 = 0.06;
const XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_PROBE_COUNT: u32 = 8;
const XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_FADE_START_FRAMES: u32 = 720;
const XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_RETIRE_FRAMES: u32 = 1440;
const XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_CONFIDENCE: f32 = 0.78;
const XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_STEP_METERS: f32 =
    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_CELL_METERS;
const XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_MAX_STEPS: u32 = 64;
const XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_SURFACE_KEEP_METERS: f32 = 0.18;
const XR_HAND_MESH_PARTICLE_COUNT_PER_HAND: usize = 480;
const XR_HAND_MESH_PARTICLE_CAPACITY: u32 = 1024;
const XR_HAND_MESH_PARTICLE_RADIUS_METERS: f32 = 0.00225;
const XR_HAND_MESH_PARTICLE_CROSS_NEIGHBORS_PER_POINT: usize = 2;
const XR_HAND_MESH_PARTICLE_CROSS_NEIGHBOR_MAX_METERS: f32 = 0.09;
const OPENXR_HAND_JOINT_PALM_INDEX: usize = 0;
const OPENXR_HAND_JOINT_WRIST_INDEX: usize = 1;
const XR_ENVIRONMENT_DEPTH_VISUAL_TEXTURE_TRANSFORM_FLAGS: u32 = 8;
const XR_ENVIRONMENT_DEPTH_VISUAL_TEXTURE_TRANSFORM_LABEL: &str = "rotate0+flipY";
const XR_ENVIRONMENT_DEPTH_DESCRIPTOR_LAYOUT: vk::ImageLayout =
    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
const OSC_OVERLAY_PROJECTION_INSET_X: f32 = 0.04;
const OSC_OVERLAY_PROJECTION_INSET_Y: f32 = 0.08;
const OSC_OVERLAY_MAX_INSTANCES: usize = 4096;
const OSC_OVERLAY_FONT_ATLAS_WIDTH: u32 = 1280;
const OSC_OVERLAY_FONT_ATLAS_HEIGHT: u32 = 672;
const OSC_OVERLAY_FONT_CELL_WIDTH: u32 = 80;
const OSC_OVERLAY_FONT_CELL_HEIGHT: u32 = 112;
const OSC_OVERLAY_FONT_ATLAS_BYTES: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/osc_diagnostics_font_atlas_u32.bin"
));

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
    let depth_requested = runtime_config().environment_depth_mode.enabled();
    if available_extensions.meta_environment_depth {
        enabled_extensions.meta_environment_depth = true;
    } else if depth_requested {
        log_info("Rusty XR OpenXR environment-depth extension unavailable".to_string());
    }
    let passthrough_probe_requested = runtime_config().openxr_passthrough_probe.enabled();
    if available_extensions.fb_passthrough && passthrough_probe_requested {
        enabled_extensions.fb_passthrough = true;
        if available_extensions.meta_passthrough_color_lut {
            enabled_extensions.meta_passthrough_color_lut = true;
        } else {
            log_info("Rusty XR OpenXR passthrough color LUT extension unavailable".to_string());
        }
    } else if passthrough_probe_requested {
        log_info("Rusty XR OpenXR passthrough extension unavailable".to_string());
    }
    let hand_mesh_particles_requested = runtime_config().hand_particle_mode.uses_openxr_hand_mesh();
    if hand_mesh_particles_requested
        && available_extensions.ext_hand_tracking
        && available_extensions.fb_hand_tracking_mesh
    {
        enabled_extensions.ext_hand_tracking = true;
        enabled_extensions.fb_hand_tracking_mesh = true;
    } else if hand_mesh_particles_requested {
        log_info(format!(
            "Rusty XR OpenXR hand mesh particle extensions unavailable extHandTracking={} fbHandTrackingMesh={}",
            available_extensions.ext_hand_tracking,
            available_extensions.fb_hand_tracking_mesh
        ));
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
    let passthrough_lut_max_resolution = query_passthrough_lut_max_resolution(&xr_instance, system);
    let environment_depth_properties = query_environment_depth_properties(&xr_instance, system);
    let environment_blend_modes = xr_instance
        .enumerate_environment_blend_modes(system, VIEW_TYPE)
        .map_err(|error| format!("enumerate environment blend modes: {error}"))?;
    let environment_blend_mode = environment_blend_modes
        .iter()
        .copied()
        .find(|mode| *mode == xr::EnvironmentBlendMode::OPAQUE)
        .or_else(|| environment_blend_modes.first().copied())
        .ok_or_else(|| "OpenXR runtime reported no environment blend modes".to_string())?;
    log_info(format!(
        "Rusty XR OpenXR environment blend modes available={:?} selected={:?}",
        environment_blend_modes, environment_blend_mode
    ));

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
            passthrough_lut_max_resolution,
            environment_depth_properties,
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

fn query_passthrough_lut_max_resolution(
    instance: &xr::Instance,
    system: xr::SystemId,
) -> Option<u32> {
    instance.exts().meta_passthrough_color_lut.as_ref()?;
    let mut lut_properties = xr::sys::SystemPassthroughColorLutPropertiesMETA {
        ty: xr::sys::SystemPassthroughColorLutPropertiesMETA::TYPE,
        next: ptr::null(),
        max_color_lut_resolution: 0,
    };
    let mut system_properties =
        xr::sys::SystemProperties::out(&mut lut_properties as *mut _ as *mut _);
    let result = unsafe {
        (instance.fp().get_system_properties)(
            instance.as_raw(),
            system,
            system_properties.as_mut_ptr(),
        )
    };
    if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
        log_error(format!(
            "Rusty XR could not query passthrough LUT properties: {result:?}"
        ));
        return None;
    }
    log_info(format!(
        "Rusty XR OpenXR passthrough LUT maxResolution={}",
        lut_properties.max_color_lut_resolution
    ));
    Some(lut_properties.max_color_lut_resolution)
}

#[derive(Clone, Copy, Debug, Default)]
struct EnvironmentDepthProperties {
    extension_available: bool,
    supports_environment_depth: bool,
    supports_hand_removal: bool,
}

fn query_environment_depth_properties(
    instance: &xr::Instance,
    system: xr::SystemId,
) -> EnvironmentDepthProperties {
    if instance.exts().meta_environment_depth.is_none() {
        return EnvironmentDepthProperties::default();
    }

    let mut depth_properties = xr::sys::SystemEnvironmentDepthPropertiesMETA {
        ty: xr::sys::SystemEnvironmentDepthPropertiesMETA::TYPE,
        next: ptr::null_mut(),
        supports_environment_depth: false.into(),
        supports_hand_removal: false.into(),
    };
    let mut system_properties =
        xr::sys::SystemProperties::out(&mut depth_properties as *mut _ as *mut _);
    let result = unsafe {
        (instance.fp().get_system_properties)(
            instance.as_raw(),
            system,
            system_properties.as_mut_ptr(),
        )
    };
    if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
        log_error(format!(
            "Rusty XR could not query environment-depth properties: {result:?}"
        ));
        return EnvironmentDepthProperties {
            extension_available: true,
            ..EnvironmentDepthProperties::default()
        };
    }

    let properties = EnvironmentDepthProperties {
        extension_available: true,
        supports_environment_depth: depth_properties.supports_environment_depth.into(),
        supports_hand_removal: depth_properties.supports_hand_removal.into(),
    };
    log_info(format!(
        "Rusty XR OpenXR environment-depth properties extensionAvailable={} supportsEnvironmentDepth={} supportsHandRemoval={}",
        properties.extension_available,
        properties.supports_environment_depth,
        properties.supports_hand_removal
    ));
    properties
}

fn sync_display_refresh_rate<G>(
    instance: &xr::Instance,
    session: &xr::Session<G>,
    target_display_refresh_hz: f32,
    last_requested_hz: &mut Option<f32>,
) {
    if instance.exts().fb_display_refresh_rate.is_none() {
        log_info("Rusty XR OpenXR display refresh extension unavailable; using runtime default");
        *last_requested_hz = Some(target_display_refresh_hz);
        return;
    }
    if last_requested_hz
        .map(|last| (last - target_display_refresh_hz).abs() <= 0.05)
        .unwrap_or(false)
    {
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
        .find(|rate| (*rate - target_display_refresh_hz).abs() <= 0.05);

    if let Some(target) = target {
        match session.request_display_refresh_rate(target) {
            Ok(()) => {
                *last_requested_hz = Some(target);
                log_info(format!(
                    "Rusty XR requested OpenXR display refresh {:.1}Hz current={} supported={}",
                    target,
                    refresh_rate_label(current),
                    refresh_rate_list_label(&supported)
                ));
            }
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
            target_display_refresh_hz,
            refresh_rate_label(current),
            refresh_rate_list_label(&supported)
        ));
        *last_requested_hz = Some(target_display_refresh_hz);
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

struct PassthroughLutResources {
    extension: xr::raw::PassthroughColorLutMETA,
    source: xr::sys::PassthroughColorLutMETA,
    target: xr::sys::PassthroughColorLutMETA,
    signature: String,
}

impl Drop for PassthroughLutResources {
    fn drop(&mut self) {
        for (label, handle) in [("source", self.source), ("target", self.target)] {
            let result = unsafe { (self.extension.destroy_passthrough_color_lut)(handle) };
            if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR passthrough LUT destroy failed {label}: {result:?}"
                ));
            }
        }
    }
}

#[derive(Default)]
struct PassthroughLutFlickerStats {
    target_bits: Option<u32>,
    start_predicted_ns: i64,
    last_half_cycle: i64,
    switches: u64,
    missed_half_cycles: u64,
    last_report_instant: Option<Instant>,
    last_report_frame: u64,
    last_report_switches: u64,
    last_report_missed: u64,
}

impl PassthroughLutFlickerStats {
    fn reset(&mut self) {
        if self.target_bits.is_some() {
            *self = Self::default();
        }
    }

    fn tick(
        &mut self,
        target_cycle_hz: f32,
        predicted_display_time: xr::Time,
        frame_count: u64,
    ) -> bool {
        let target_bits = target_cycle_hz.to_bits();
        let predicted_ns = predicted_display_time.as_nanos();
        if self.target_bits != Some(target_bits) || predicted_ns < self.start_predicted_ns {
            self.target_bits = Some(target_bits);
            self.start_predicted_ns = predicted_ns;
            self.last_half_cycle = 0;
            self.switches = 0;
            self.missed_half_cycles = 0;
            self.last_report_instant = Some(Instant::now());
            self.last_report_frame = frame_count;
            self.last_report_switches = 0;
            self.last_report_missed = 0;
        }

        let elapsed_seconds =
            (predicted_ns - self.start_predicted_ns).max(0) as f64 / 1_000_000_000.0;
        let half_cycle = (elapsed_seconds * target_cycle_hz as f64 * 2.0).floor() as i64;
        if half_cycle != self.last_half_cycle {
            let skipped = (half_cycle - self.last_half_cycle)
                .unsigned_abs()
                .saturating_sub(1);
            self.switches = self.switches.saturating_add(1);
            self.missed_half_cycles = self.missed_half_cycles.saturating_add(skipped);
            self.last_half_cycle = half_cycle;
        }

        self.report_if_due(target_cycle_hz, frame_count);
        (self.last_half_cycle & 1) != 0
    }

    fn report_if_due(&mut self, target_cycle_hz: f32, frame_count: u64) {
        let now = Instant::now();
        let Some(last_report) = self.last_report_instant else {
            self.last_report_instant = Some(now);
            return;
        };
        let elapsed = now.duration_since(last_report).as_secs_f64();
        if elapsed < 2.0 {
            return;
        }
        let delta_switches = self.switches.saturating_sub(self.last_report_switches);
        let delta_frames = frame_count.saturating_sub(self.last_report_frame);
        let delta_missed = self
            .missed_half_cycles
            .saturating_sub(self.last_report_missed);
        let observed_switch_hz = delta_switches as f64 / elapsed;
        let observed_cycle_hz = observed_switch_hz * 0.5;
        let observed_frame_hz = delta_frames as f64 / elapsed;
        log_info(format!(
            "Rusty XR passthrough LUT flicker stats targetCycleHz={:.2} targetSwitchHz={:.2} observedCycleHz={:.2} observedSwitchHz={:.2} observedFrameHz={:.2} frames={} switches={} missedHalfCycles={} totalSwitches={} totalMissedHalfCycles={}",
            target_cycle_hz,
            target_cycle_hz * 2.0,
            observed_cycle_hz,
            observed_switch_hz,
            observed_frame_hz,
            delta_frames,
            delta_switches,
            delta_missed,
            self.switches,
            self.missed_half_cycles
        ));
        self.last_report_instant = Some(now);
        self.last_report_frame = frame_count;
        self.last_report_switches = self.switches;
        self.last_report_missed = self.missed_half_cycles;
    }
}

#[derive(Default)]
struct FullFieldFlickerStats {
    target_bits: Option<u32>,
    start_predicted_ns: i64,
    last_half_cycle: i64,
    switches: u64,
    missed_half_cycles: u64,
    last_report_instant: Option<Instant>,
    last_report_frame: u64,
    last_report_switches: u64,
    last_report_missed: u64,
}

impl FullFieldFlickerStats {
    fn reset(&mut self) {
        if self.target_bits.is_some() {
            *self = Self::default();
        }
    }

    fn tick(
        &mut self,
        target_cycle_hz: f32,
        predicted_display_time: xr::Time,
        frame_count: u64,
    ) -> bool {
        if target_cycle_hz <= 0.0 {
            self.reset();
            return false;
        }
        let target_bits = target_cycle_hz.to_bits();
        let predicted_ns = predicted_display_time.as_nanos();
        if self.target_bits != Some(target_bits) || predicted_ns < self.start_predicted_ns {
            self.target_bits = Some(target_bits);
            self.start_predicted_ns = predicted_ns;
            self.last_half_cycle = 0;
            self.switches = 0;
            self.missed_half_cycles = 0;
            self.last_report_instant = Some(Instant::now());
            self.last_report_frame = frame_count;
            self.last_report_switches = 0;
            self.last_report_missed = 0;
        }

        let elapsed_seconds =
            (predicted_ns - self.start_predicted_ns).max(0) as f64 / 1_000_000_000.0;
        let half_cycle = (elapsed_seconds * target_cycle_hz as f64 * 2.0).floor() as i64;
        if half_cycle != self.last_half_cycle {
            let skipped = (half_cycle - self.last_half_cycle)
                .unsigned_abs()
                .saturating_sub(1);
            self.switches = self.switches.saturating_add(1);
            self.missed_half_cycles = self.missed_half_cycles.saturating_add(skipped);
            self.last_half_cycle = half_cycle;
        }

        self.report_if_due(target_cycle_hz, frame_count);
        (self.last_half_cycle & 1) == 0
    }

    fn report_if_due(&mut self, target_cycle_hz: f32, frame_count: u64) {
        let now = Instant::now();
        let Some(last_report) = self.last_report_instant else {
            self.last_report_instant = Some(now);
            return;
        };
        let elapsed = now.duration_since(last_report).as_secs_f64();
        if elapsed < 2.0 {
            return;
        }
        let delta_switches = self.switches.saturating_sub(self.last_report_switches);
        let delta_frames = frame_count.saturating_sub(self.last_report_frame);
        let delta_missed = self
            .missed_half_cycles
            .saturating_sub(self.last_report_missed);
        let observed_switch_hz = delta_switches as f64 / elapsed;
        let observed_cycle_hz = observed_switch_hz * 0.5;
        let observed_frame_hz = delta_frames as f64 / elapsed;
        log_info(format!(
            "Rusty XR full-field flicker stats targetCycleHz={:.2} targetSwitchHz={:.2} observedCycleHz={:.2} observedSwitchHz={:.2} observedFrameHz={:.2} frames={} switches={} missedHalfCycles={} totalSwitches={} totalMissedHalfCycles={}",
            target_cycle_hz,
            target_cycle_hz * 2.0,
            observed_cycle_hz,
            observed_switch_hz,
            observed_frame_hz,
            delta_frames,
            delta_switches,
            delta_missed,
            self.switches,
            self.missed_half_cycles
        ));
        self.last_report_instant = Some(now);
        self.last_report_frame = frame_count;
        self.last_report_switches = self.switches;
        self.last_report_missed = self.missed_half_cycles;
    }
}

struct OpenXrEnvironmentDepthProbe {
    mode: EnvironmentDepthMode,
    extension: xr::raw::EnvironmentDepthMETA,
    provider: xr::sys::EnvironmentDepthProviderMETA,
    swapchain: xr::sys::EnvironmentDepthSwapchainMETA,
    depth_images: Vec<u64>,
    width: u32,
    height: u32,
    supports_hand_removal: bool,
    hand_removal_enabled: bool,
    start_frame: u64,
    window_start: Instant,
    window_frame_count: u64,
    window_attempts: u64,
    window_acquired: u64,
    window_unavailable: u64,
    window_errors: u64,
    window_unique_capture_times: u64,
    window_acquire_cpu_ms: f64,
    total_attempts: u64,
    total_acquired: u64,
    total_unavailable: u64,
    total_errors: u64,
    total_unique_capture_times: u64,
    repeated_capture_time_count: u64,
    last_capture_time_ns: Option<i64>,
    last_acquired_frame: Option<u64>,
    last_swapchain_index: Option<u32>,
    last_near_z: f32,
    last_far_z: f32,
}

#[derive(Clone, Copy, Debug)]
struct EnvironmentDepthVisualFrame {
    frame_count: u64,
    swapchain_index: u32,
    depth_width: u32,
    depth_height: u32,
    near_z: f32,
    far_z: f32,
    capture_time_ns: i64,
    left_fov_tangents: [f32; 4],
    right_fov_tangents: [f32; 4],
    left_render_fov_tangents: [f32; 4],
    right_render_fov_tangents: [f32; 4],
    left_position: [f32; 4],
    right_position: [f32; 4],
    left_orientation: [f32; 4],
    right_orientation: [f32; 4],
    left_render_position: [f32; 4],
    right_render_position: [f32; 4],
    left_render_orientation: [f32; 4],
    right_render_orientation: [f32; 4],
}

fn fov_tangents(fov: xr::sys::Fovf) -> [f32; 4] {
    [
        fov.angle_left.tan(),
        fov.angle_right.tan(),
        fov.angle_up.tan(),
        fov.angle_down.tan(),
    ]
}

fn pose_position(pose: xr::sys::Posef) -> [f32; 4] {
    [pose.position.x, pose.position.y, pose.position.z, 1.0]
}

fn pose_orientation(pose: xr::sys::Posef) -> [f32; 4] {
    [
        pose.orientation.x,
        pose.orientation.y,
        pose.orientation.z,
        pose.orientation.w,
    ]
}

impl OpenXrEnvironmentDepthProbe {
    fn acquire(
        &mut self,
        acquire_space: &xr::Space,
        reference_from_acquire_space: xr::Posef,
        display_time: xr::Time,
        current_views: &[xr::View],
        frame_count: u64,
    ) -> Option<EnvironmentDepthVisualFrame> {
        self.window_frame_count = self.window_frame_count.saturating_add(1);
        self.window_attempts = self.window_attempts.saturating_add(1);
        self.total_attempts = self.total_attempts.saturating_add(1);

        let acquire_info = xr::sys::EnvironmentDepthImageAcquireInfoMETA {
            ty: xr::sys::EnvironmentDepthImageAcquireInfoMETA::TYPE,
            next: ptr::null(),
            space: acquire_space.as_raw(),
            display_time,
        };
        let mut timestamp = xr::sys::EnvironmentDepthImageTimestampMETA {
            ty: xr::sys::EnvironmentDepthImageTimestampMETA::TYPE,
            next: ptr::null(),
            capture_time: xr::Time::from_nanos(0),
        };
        let empty_view = xr::sys::EnvironmentDepthImageViewMETA {
            ty: xr::sys::EnvironmentDepthImageViewMETA::TYPE,
            next: ptr::null(),
            fov: xr::sys::Fovf::default(),
            pose: xr::sys::Posef::default(),
        };
        let mut image = xr::sys::EnvironmentDepthImageMETA {
            ty: xr::sys::EnvironmentDepthImageMETA::TYPE,
            next: &mut timestamp as *mut _ as *const _,
            swapchain_index: 0,
            near_z: 0.0,
            far_z: 0.0,
            views: [empty_view; 2],
        };

        let started = Instant::now();
        let result = unsafe {
            (self.extension.acquire_environment_depth_image)(
                self.provider,
                &acquire_info,
                &mut image,
            )
        };
        let acquire_ms = started.elapsed().as_secs_f64() * 1000.0;
        self.window_acquire_cpu_ms += acquire_ms;

        if result == xr::sys::Result::ENVIRONMENT_DEPTH_NOT_AVAILABLE_META {
            self.window_unavailable = self.window_unavailable.saturating_add(1);
            self.total_unavailable = self.total_unavailable.saturating_add(1);
            self.report_status_if_due(frame_count, acquire_ms, false);
            return None;
        }
        if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
            self.window_errors = self.window_errors.saturating_add(1);
            self.total_errors = self.total_errors.saturating_add(1);
            log_error(format!(
                "Rusty XR environment depth acquire failed frame={} result={result:?}",
                frame_count
            ));
            self.report_status_if_due(frame_count, acquire_ms, false);
            return None;
        }

        self.window_acquired = self.window_acquired.saturating_add(1);
        self.total_acquired = self.total_acquired.saturating_add(1);
        let capture_time_ns = timestamp.capture_time.as_nanos();
        if self.last_capture_time_ns == Some(capture_time_ns) {
            self.repeated_capture_time_count = self.repeated_capture_time_count.saturating_add(1);
        } else {
            self.window_unique_capture_times = self.window_unique_capture_times.saturating_add(1);
            self.total_unique_capture_times = self.total_unique_capture_times.saturating_add(1);
            self.last_capture_time_ns = Some(capture_time_ns);
        }
        self.last_acquired_frame = Some(frame_count);
        self.last_swapchain_index = Some(image.swapchain_index);
        self.last_near_z = image.near_z;
        self.last_far_z = image.far_z;

        if self.total_acquired == 1 {
            log_info(format!(
                "Rusty XR environment depth first frame swapchainIndex={} size={}x{} depthFormat=VK_FORMAT_D16_UNORM layerCount={} nearZ={} farZ={} captureTimeNs={} confidenceSource=none confidencePayload=false confidenceStatus=not-exposed-by-XR_META_environment_depth depthVisualEncoding=linear-d16-meters-infinity-white depthVisualMaxMeters={} depthVisualTextureTransform={} depthPoseSource=view-space-composed projectionYConvention=vulkan-positive-viewport-y-flipped-in-shader",
                image.swapchain_index,
                self.width,
                self.height,
                VIEW_COUNT,
                image.near_z,
                image.far_z,
                capture_time_ns,
                XR_ENVIRONMENT_DEPTH_VISUAL_MAX_METERS,
                XR_ENVIRONMENT_DEPTH_VISUAL_TEXTURE_TRANSFORM_LABEL
            ));
        }
        self.report_status_if_due(frame_count, acquire_ms, true);
        if self.mode.visualizes() {
            let left_depth_pose =
                multiply_openxr_pose(reference_from_acquire_space, image.views[0].pose);
            let right_depth_pose =
                multiply_openxr_pose(reference_from_acquire_space, image.views[1].pose);
            let left_render_fov = current_views
                .first()
                .map(|view| view.fov)
                .unwrap_or(image.views[0].fov);
            let right_render_fov = current_views
                .get(1)
                .map(|view| view.fov)
                .unwrap_or(image.views[1].fov);
            let left_render_pose = current_views
                .first()
                .map(|view| view.pose)
                .unwrap_or(image.views[0].pose);
            let right_render_pose = current_views
                .get(1)
                .map(|view| view.pose)
                .unwrap_or(image.views[1].pose);
            Some(EnvironmentDepthVisualFrame {
                frame_count,
                swapchain_index: image.swapchain_index,
                depth_width: self.width,
                depth_height: self.height,
                near_z: image.near_z,
                far_z: image.far_z,
                capture_time_ns,
                left_fov_tangents: fov_tangents(image.views[0].fov),
                right_fov_tangents: fov_tangents(image.views[1].fov),
                left_render_fov_tangents: fov_tangents(left_render_fov),
                right_render_fov_tangents: fov_tangents(right_render_fov),
                left_position: pose_position(left_depth_pose),
                right_position: pose_position(right_depth_pose),
                left_orientation: pose_orientation(left_depth_pose),
                right_orientation: pose_orientation(right_depth_pose),
                left_render_position: pose_position(left_render_pose),
                right_render_position: pose_position(right_render_pose),
                left_render_orientation: pose_orientation(left_render_pose),
                right_render_orientation: pose_orientation(right_render_pose),
            })
        } else {
            None
        }
    }

    fn depth_image_handles(&self) -> &[u64] {
        &self.depth_images
    }

    fn visual_clear_color(&self, frame_count: u64) -> Option<[f32; 4]> {
        if !self.mode.visualizes() {
            return None;
        }
        if self.mode.mesh_overlay() || self.mode.particle_overlay() {
            return Some([0.0, 0.0, 0.0, 0.0]);
        }
        let pulse = if frame_count % 60 < 30 { 0.05 } else { 0.0 };
        if self.total_errors > 0 && self.last_acquired_frame.is_none() {
            return Some([0.34 + pulse, 0.02, 0.03, 1.0]);
        }
        if self
            .last_acquired_frame
            .map(|last| frame_count.saturating_sub(last) <= 3)
            .unwrap_or(false)
        {
            return Some([0.02, 0.22 + pulse, 0.08, 1.0]);
        }
        if self.total_unavailable > 0 {
            return Some([0.28 + pulse, 0.17, 0.02, 1.0]);
        }
        Some([0.02, 0.08 + pulse, 0.18, 1.0])
    }

    fn report_status_if_due(&mut self, frame_count: u64, last_acquire_ms: f64, acquired: bool) {
        if frame_count != self.start_frame
            && frame_count % 120 != 0
            && !(acquired && self.total_acquired == 1)
        {
            return;
        }

        let elapsed = self.window_start.elapsed().as_secs_f64().max(0.001);
        let observed_openxr_fps = self.window_frame_count as f64 / elapsed;
        let observed_acquire_hz = self.window_attempts as f64 / elapsed;
        let observed_unique_depth_hz = self.window_unique_capture_times as f64 / elapsed;
        let avg_acquire_ms = if self.window_attempts > 0 {
            self.window_acquire_cpu_ms / self.window_attempts as f64
        } else {
            0.0
        };
        log_info(format!(
            "Rusty XR environment depth status frame={} depthEnabled=true mode={} extensionAvailable=true supported=true providerCreated=true providerRunning=true swapchainCreated=true size={}x{} depthFormat=VK_FORMAT_D16_UNORM layerCount={} swapchainIndex={} openXrFrameCount={} observedOpenXrFps={:.1} acquireAttempts={} acquiredFrames={} unavailableFrames={} acquireErrors={} uniqueCaptureTimes={} repeatedCaptureTimes={} observedAcquireHz={:.1} observedDepthHz={:.1} lastAcquireCpuMs={:.3} avgAcquireCpuMs={:.3} captureTimeNs={} nearZ={} farZ={} handRemovalSupported={} handRemovalEnabled={} confidenceSource=none confidencePayload=false confidenceStatus=not-exposed-by-XR_META_environment_depth visualizer={} depthVisualEncoding=linear-d16-meters-infinity-white depthVisualMaxMeters={} depthVisualTextureTransform={} depthVisualEyeMapping=left-layer-0-right-layer-1 depthPoseSource=view-space-composed projectionYConvention=vulkan-positive-viewport-y-flipped-in-shader depthMeshOverlay={} depthParticleOverlay={} depthMeshDistanceColorMaxMeters={} depthMeshCellMeters={} depthMeshDiscontinuityMeters={} depthMeshProjection=local-space-depth-surface depthMeshRasterization={} depthMeshGridStridePixels={} depthParticleCapacity={} depthParticleSampleStridePixels={} passthroughVisible={}",
            frame_count,
            self.mode.stable_id(),
            self.width,
            self.height,
            VIEW_COUNT,
            self.last_swapchain_index
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string()),
            frame_count,
            observed_openxr_fps,
            self.total_attempts,
            self.total_acquired,
            self.total_unavailable,
            self.total_errors,
            self.total_unique_capture_times,
            self.repeated_capture_time_count,
            observed_acquire_hz,
            observed_unique_depth_hz,
            last_acquire_ms,
            avg_acquire_ms,
            self.last_capture_time_ns
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.last_near_z,
            self.last_far_z,
            self.supports_hand_removal,
            self.hand_removal_enabled,
            self.mode.visualizes(),
            XR_ENVIRONMENT_DEPTH_VISUAL_MAX_METERS,
            XR_ENVIRONMENT_DEPTH_VISUAL_TEXTURE_TRANSFORM_LABEL,
            self.mode.mesh_overlay(),
            self.mode.particle_overlay(),
            XR_ENVIRONMENT_DEPTH_MESH_DISTANCE_GRADIENT_MAX_METERS,
            XR_ENVIRONMENT_DEPTH_MESH_CELL_METERS,
            XR_ENVIRONMENT_DEPTH_MESH_DISCONTINUITY_METERS,
            if self.mode.particle_overlay() {
                "retained-local-space-metric-billboard-particles"
            } else if self.mode.mesh_overlay() {
                "world-space-generated-grid"
            } else {
                "fullscreen-depth-visualizer"
            },
            XR_ENVIRONMENT_DEPTH_MESH_GRID_STRIDE_PIXELS,
            XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY,
            XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS,
            self.mode.mesh_overlay() || self.mode.particle_overlay()
        ));
        self.window_start = Instant::now();
        self.window_frame_count = 0;
        self.window_attempts = 0;
        self.window_acquired = 0;
        self.window_unavailable = 0;
        self.window_errors = 0;
        self.window_unique_capture_times = 0;
        self.window_acquire_cpu_ms = 0.0;
    }
}

impl Drop for OpenXrEnvironmentDepthProbe {
    fn drop(&mut self) {
        unsafe {
            let stop_result = (self.extension.stop_environment_depth_provider)(self.provider);
            if stop_result.into_raw() < xr::sys::Result::SUCCESS.into_raw()
                && stop_result != xr::sys::Result::ERROR_HANDLE_INVALID
            {
                log_error(format!(
                    "Rusty XR environment depth provider stop during drop failed result={stop_result:?}"
                ));
            }
            if self.swapchain != xr::sys::EnvironmentDepthSwapchainMETA::NULL {
                let destroy_swapchain_result =
                    (self.extension.destroy_environment_depth_swapchain)(self.swapchain);
                if destroy_swapchain_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR environment depth swapchain destroy failed result={destroy_swapchain_result:?}"
                    ));
                }
                self.swapchain = xr::sys::EnvironmentDepthSwapchainMETA::NULL;
            }
            if self.provider != xr::sys::EnvironmentDepthProviderMETA::NULL {
                let destroy_provider_result =
                    (self.extension.destroy_environment_depth_provider)(self.provider);
                if destroy_provider_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR environment depth provider destroy failed result={destroy_provider_result:?}"
                    ));
                }
                self.provider = xr::sys::EnvironmentDepthProviderMETA::NULL;
            }
        }
    }
}

fn sync_openxr_environment_depth_probe<G: xr::Graphics>(
    instance: &xr::Instance,
    session: &xr::Session<G>,
    existing: Option<OpenXrEnvironmentDepthProbe>,
    mode: EnvironmentDepthMode,
    hand_removal_enabled: bool,
    properties: EnvironmentDepthProperties,
    start_frame: u64,
) -> Option<OpenXrEnvironmentDepthProbe> {
    let effective_hand_removal_enabled = hand_removal_enabled && properties.supports_hand_removal;
    if let Some(existing) = existing {
        if existing.mode == mode && existing.hand_removal_enabled == effective_hand_removal_enabled
        {
            return Some(existing);
        }
        if mode.enabled() {
            log_info(format!(
                "Rusty XR environment depth probe switching from {} to {}",
                existing.mode.stable_id(),
                mode.stable_id()
            ));
        } else {
            log_info(format!(
                "Rusty XR environment depth probe disabled from {}",
                existing.mode.stable_id()
            ));
        }
    }
    if !mode.enabled() {
        return None;
    }
    if !properties.extension_available || instance.exts().meta_environment_depth.is_none() {
        log_info(format!(
            "Rusty XR environment depth requested mode={} but XR_META_environment_depth is unavailable",
            mode.stable_id()
        ));
        return None;
    }
    if !properties.supports_environment_depth {
        log_info(format!(
            "Rusty XR environment depth requested mode={} but system properties report unsupported",
            mode.stable_id()
        ));
        return None;
    }

    match create_openxr_environment_depth_probe(
        instance,
        session,
        mode,
        effective_hand_removal_enabled,
        properties,
        start_frame,
    ) {
        Ok(probe) => {
            log_info(format!(
                "Rusty XR environment depth probe active mode={} size={}x{} handRemovalEnabled={} visualizer={}",
                probe.mode.stable_id(),
                probe.width,
                probe.height,
                probe.hand_removal_enabled,
                probe.mode.visualizes()
            ));
            Some(probe)
        }
        Err(error) => {
            log_error(format!(
                "Rusty XR environment depth probe failed mode={} error={error}",
                mode.stable_id()
            ));
            None
        }
    }
}

fn openxr_environment_depth_probe_reuses_existing(
    existing: Option<&OpenXrEnvironmentDepthProbe>,
    mode: EnvironmentDepthMode,
    hand_removal_enabled: bool,
    properties: EnvironmentDepthProperties,
) -> bool {
    let effective_hand_removal_enabled = hand_removal_enabled && properties.supports_hand_removal;
    existing
        .map(|probe| {
            probe.mode == mode && probe.hand_removal_enabled == effective_hand_removal_enabled
        })
        .unwrap_or(false)
}

fn create_openxr_environment_depth_probe<G: xr::Graphics>(
    instance: &xr::Instance,
    session: &xr::Session<G>,
    mode: EnvironmentDepthMode,
    hand_removal_enabled: bool,
    properties: EnvironmentDepthProperties,
    start_frame: u64,
) -> Result<OpenXrEnvironmentDepthProbe, String> {
    let extension = *instance
        .exts()
        .meta_environment_depth
        .as_ref()
        .ok_or_else(|| "XR_META_environment_depth function table is unavailable".to_string())?;
    let provider_info = xr::sys::EnvironmentDepthProviderCreateInfoMETA {
        ty: xr::sys::EnvironmentDepthProviderCreateInfoMETA::TYPE,
        next: ptr::null(),
        create_flags: xr::sys::EnvironmentDepthProviderCreateFlagsMETA::EMPTY,
    };
    let mut provider = xr::sys::EnvironmentDepthProviderMETA::NULL;
    let result = unsafe {
        (extension.create_environment_depth_provider)(
            session.as_raw(),
            &provider_info,
            &mut provider,
        )
    };
    ensure_xr_success(result, "xrCreateEnvironmentDepthProviderMETA")?;

    let swapchain_info = xr::sys::EnvironmentDepthSwapchainCreateInfoMETA {
        ty: xr::sys::EnvironmentDepthSwapchainCreateInfoMETA::TYPE,
        next: ptr::null(),
        create_flags: xr::sys::EnvironmentDepthSwapchainCreateFlagsMETA::EMPTY,
    };
    let mut swapchain = xr::sys::EnvironmentDepthSwapchainMETA::NULL;
    let result = unsafe {
        (extension.create_environment_depth_swapchain)(provider, &swapchain_info, &mut swapchain)
    };
    if let Err(error) = ensure_xr_success(result, "xrCreateEnvironmentDepthSwapchainMETA") {
        unsafe {
            let destroy_result = (extension.destroy_environment_depth_provider)(provider);
            if destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR environment depth provider cleanup after swapchain create failed result={destroy_result:?}"
                ));
            }
        }
        return Err(error);
    }

    let mut swapchain_state = xr::sys::EnvironmentDepthSwapchainStateMETA {
        ty: xr::sys::EnvironmentDepthSwapchainStateMETA::TYPE,
        next: ptr::null_mut(),
        width: 0,
        height: 0,
    };
    let result = unsafe {
        (extension.get_environment_depth_swapchain_state)(swapchain, &mut swapchain_state)
    };
    if let Err(error) = ensure_xr_success(result, "xrGetEnvironmentDepthSwapchainStateMETA") {
        unsafe {
            let destroy_swapchain_result =
                (extension.destroy_environment_depth_swapchain)(swapchain);
            if destroy_swapchain_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR environment depth swapchain cleanup failed result={destroy_swapchain_result:?}"
                ));
            }
            let destroy_provider_result = (extension.destroy_environment_depth_provider)(provider);
            if destroy_provider_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR environment depth provider cleanup failed result={destroy_provider_result:?}"
                ));
            }
        }
        return Err(error);
    }

    let depth_images = match unsafe {
        enumerate_openxr_environment_depth_swapchain_images(&extension, swapchain)
    } {
        Ok(images) => images,
        Err(error) => {
            unsafe {
                let destroy_swapchain_result =
                    (extension.destroy_environment_depth_swapchain)(swapchain);
                if destroy_swapchain_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR environment depth swapchain cleanup after image enumeration failed result={destroy_swapchain_result:?}"
                    ));
                }
                let destroy_provider_result =
                    (extension.destroy_environment_depth_provider)(provider);
                if destroy_provider_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR environment depth provider cleanup after image enumeration failed result={destroy_provider_result:?}"
                    ));
                }
            }
            return Err(error);
        }
    };

    if hand_removal_enabled {
        let hand_removal_info = xr::sys::EnvironmentDepthHandRemovalSetInfoMETA {
            ty: xr::sys::EnvironmentDepthHandRemovalSetInfoMETA::TYPE,
            next: ptr::null(),
            enabled: true.into(),
        };
        let result =
            unsafe { (extension.set_environment_depth_hand_removal)(provider, &hand_removal_info) };
        if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
            log_error(format!(
                "Rusty XR environment depth hand removal request failed result={result:?}"
            ));
        }
    }

    let result = unsafe { (extension.start_environment_depth_provider)(provider) };
    if let Err(error) = ensure_xr_success(result, "xrStartEnvironmentDepthProviderMETA") {
        unsafe {
            let destroy_swapchain_result =
                (extension.destroy_environment_depth_swapchain)(swapchain);
            if destroy_swapchain_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR environment depth swapchain cleanup after start failed result={destroy_swapchain_result:?}"
                ));
            }
            let destroy_provider_result = (extension.destroy_environment_depth_provider)(provider);
            if destroy_provider_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR environment depth provider cleanup after start failed result={destroy_provider_result:?}"
                ));
            }
        }
        return Err(error);
    }

    Ok(OpenXrEnvironmentDepthProbe {
        mode,
        extension,
        provider,
        swapchain,
        depth_images,
        width: swapchain_state.width,
        height: swapchain_state.height,
        supports_hand_removal: properties.supports_hand_removal,
        hand_removal_enabled,
        start_frame,
        window_start: Instant::now(),
        window_frame_count: 0,
        window_attempts: 0,
        window_acquired: 0,
        window_unavailable: 0,
        window_errors: 0,
        window_unique_capture_times: 0,
        window_acquire_cpu_ms: 0.0,
        total_attempts: 0,
        total_acquired: 0,
        total_unavailable: 0,
        total_errors: 0,
        total_unique_capture_times: 0,
        repeated_capture_time_count: 0,
        last_capture_time_ns: None,
        last_acquired_frame: None,
        last_swapchain_index: None,
        last_near_z: 0.0,
        last_far_z: 0.0,
    })
}

unsafe fn enumerate_openxr_environment_depth_swapchain_images(
    extension: &xr::raw::EnvironmentDepthMETA,
    swapchain: xr::sys::EnvironmentDepthSwapchainMETA,
) -> Result<Vec<u64>, String> {
    let mut image_count = 0;
    ensure_xr_success(
        (extension.enumerate_environment_depth_swapchain_images)(
            swapchain,
            0,
            &mut image_count,
            ptr::null_mut(),
        ),
        "xrEnumerateEnvironmentDepthSwapchainImagesMETA(count)",
    )?;
    if image_count == 0 {
        return Err("environment depth swapchain returned no Vulkan images".to_string());
    }

    let mut images = vec![
        xr::sys::SwapchainImageVulkanKHR {
            ty: xr::sys::SwapchainImageVulkanKHR::TYPE,
            next: ptr::null_mut(),
            image: 0,
        };
        image_count as usize
    ];
    let mut enumerated = 0;
    ensure_xr_success(
        (extension.enumerate_environment_depth_swapchain_images)(
            swapchain,
            image_count,
            &mut enumerated,
            images.as_mut_ptr() as *mut xr::sys::SwapchainImageBaseHeader,
        ),
        "xrEnumerateEnvironmentDepthSwapchainImagesMETA",
    )?;
    images.truncate(enumerated as usize);
    if images.is_empty() {
        return Err(
            "environment depth swapchain image enumeration returned zero images".to_string(),
        );
    }
    for (index, image) in images.iter().enumerate() {
        if image.image == 0 {
            return Err(format!(
                "environment depth swapchain image {index} returned a null VkImage"
            ));
        }
        log_info(format!(
            "Rusty XR environment depth swapchain image index={} image=0x{:x} format=VK_FORMAT_D16_UNORM arrayLayers={}",
            index,
            image.image,
            VIEW_COUNT
        ));
    }
    Ok(images.into_iter().map(|image| image.image).collect())
}

struct OpenXrPassthroughProbe {
    mode: OpenXrPassthroughProbeMode,
    fb_passthrough: xr::raw::PassthroughFB,
    passthrough: xr::sys::PassthroughFB,
    layer: xr::sys::PassthroughLayerFB,
    start_frame: u64,
    paused: bool,
    last_style_signature: Option<String>,
    lut_max_resolution: Option<u32>,
    lut_resources: Option<PassthroughLutResources>,
    lut_flicker: PassthroughLutFlickerStats,
}

impl OpenXrPassthroughProbe {
    fn tick(&mut self, frame_count: u64) {
        if self.mode != OpenXrPassthroughProbeMode::Warmup || self.paused {
            return;
        }
        if frame_count.saturating_sub(self.start_frame) < 6 {
            return;
        }
        let result = unsafe { (self.fb_passthrough.passthrough_pause)(self.passthrough) };
        if result.into_raw() >= xr::sys::Result::SUCCESS.into_raw() {
            self.paused = true;
            log_info(format!(
                "Rusty XR OpenXR passthrough probe paused mode={} afterFrames={}",
                self.mode.stable_id(),
                frame_count.saturating_sub(self.start_frame)
            ));
        } else {
            self.paused = true;
            log_error(format!(
                "Rusty XR OpenXR passthrough probe pause failed mode={} result={result:?}",
                self.mode.stable_id()
            ));
        }
    }

    fn submits_composition_layer(&self) -> bool {
        self.mode.submits_composition_layer() && !self.paused
    }

    fn apply_style(
        &mut self,
        instance: &xr::Instance,
        config: &RuntimeConfig,
        predicted_display_time: xr::Time,
        frame_count: u64,
    ) {
        let flicker_state = self.lut_flicker_state(config, predicted_display_time, frame_count);
        let signature = passthrough_style_signature(config, flicker_state);
        if self.last_style_signature.as_deref() == Some(signature.as_str()) {
            return;
        }

        match apply_passthrough_layer_style(instance, self, config, flicker_state) {
            Ok(()) => {
                self.last_style_signature = Some(signature);
                if flicker_state.is_none()
                    || self.lut_flicker.switches <= 1
                    || frame_count % 120 == 0
                {
                    log_info(format!(
                        "Rusty XR OpenXR passthrough style applied mode={} opacity={} edge={:?} bcs=({},{},{}) colorPhase={} colorAmplitude={} lutResolution={} lutWeight={} lutFlickerHz={} flickerState={}",
                        config.passthrough_style_mode.stable_id(),
                        config.passthrough_opacity,
                        config.passthrough_edge_color,
                        config.passthrough_brightness,
                        config.passthrough_contrast,
                        config.passthrough_saturation,
                        config.passthrough_color_phase,
                        config.passthrough_color_amplitude,
                        config.passthrough_lut_resolution,
                        config.passthrough_lut_weight,
                        config.passthrough_lut_flicker_hz,
                        flicker_state
                            .map(|state| if state { "target" } else { "source" })
                            .unwrap_or("static")
                    ));
                }
            }
            Err(error) => {
                self.last_style_signature = Some(signature);
                log_error(format!(
                    "Rusty XR OpenXR passthrough style failed mode={} error={error}",
                    config.passthrough_style_mode.stable_id()
                ));
            }
        }
    }

    fn lut_flicker_state(
        &mut self,
        config: &RuntimeConfig,
        predicted_display_time: xr::Time,
        frame_count: u64,
    ) -> Option<bool> {
        if config.passthrough_style_mode != OpenXrPassthroughStyleMode::ColorLut
            || config.passthrough_lut_flicker_hz <= 0.0
        {
            self.lut_flicker.reset();
            return None;
        }
        Some(self.lut_flicker.tick(
            config.passthrough_lut_flicker_hz,
            predicted_display_time,
            frame_count,
        ))
    }
}

impl Drop for OpenXrPassthroughProbe {
    fn drop(&mut self) {
        self.lut_resources = None;
        unsafe {
            if self.layer != xr::sys::PassthroughLayerFB::NULL {
                let pause_result = (self.fb_passthrough.passthrough_layer_pause)(self.layer);
                if pause_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR OpenXR passthrough layer pause during drop failed result={pause_result:?}"
                    ));
                }
                let destroy_result = (self.fb_passthrough.destroy_passthrough_layer)(self.layer);
                if destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR OpenXR passthrough layer destroy failed result={destroy_result:?}"
                    ));
                }
                self.layer = xr::sys::PassthroughLayerFB::NULL;
            }
            if self.passthrough != xr::sys::PassthroughFB::NULL {
                let pause_result = (self.fb_passthrough.passthrough_pause)(self.passthrough);
                if pause_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR OpenXR passthrough pause during drop failed result={pause_result:?}"
                    ));
                }
                let destroy_result = (self.fb_passthrough.destroy_passthrough)(self.passthrough);
                if destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR OpenXR passthrough destroy failed result={destroy_result:?}"
                    ));
                }
                self.passthrough = xr::sys::PassthroughFB::NULL;
            }
        }
    }
}

fn sync_openxr_passthrough_probe<G: xr::Graphics>(
    instance: &xr::Instance,
    session: &xr::Session<G>,
    existing: Option<OpenXrPassthroughProbe>,
    mode: OpenXrPassthroughProbeMode,
    start_frame: u64,
    passthrough_lut_max_resolution: Option<u32>,
) -> Option<OpenXrPassthroughProbe> {
    if let Some(existing) = existing {
        if existing.mode == mode {
            return Some(existing);
        }
        if mode.enabled() {
            log_info(format!(
                "Rusty XR OpenXR passthrough probe switching from {} to {}",
                existing.mode.stable_id(),
                mode.stable_id()
            ));
        } else {
            log_info(format!(
                "Rusty XR OpenXR passthrough probe disabled from {}",
                existing.mode.stable_id()
            ));
        }
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

    match create_openxr_passthrough_probe(
        instance,
        session,
        mode,
        start_frame,
        passthrough_lut_max_resolution,
    ) {
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
    instance: &xr::Instance,
    session: &xr::Session<G>,
    mode: OpenXrPassthroughProbeMode,
    start_frame: u64,
    passthrough_lut_max_resolution: Option<u32>,
) -> Result<OpenXrPassthroughProbe, String> {
    let fb_passthrough = *instance
        .exts()
        .fb_passthrough
        .as_ref()
        .ok_or_else(|| "XR_FB_passthrough function table is unavailable".to_string())?;
    let flags = xr::PassthroughFlagsFB::EMPTY;
    let passthrough_info = xr::sys::PassthroughCreateInfoFB {
        ty: xr::sys::PassthroughCreateInfoFB::TYPE,
        next: ptr::null(),
        flags,
    };
    let mut passthrough = xr::sys::PassthroughFB::NULL;
    let result = unsafe {
        (fb_passthrough.create_passthrough)(session.as_raw(), &passthrough_info, &mut passthrough)
    };
    ensure_xr_success(result, "xrCreatePassthroughFB")?;

    let layer_info = xr::sys::PassthroughLayerCreateInfoFB {
        ty: xr::sys::PassthroughLayerCreateInfoFB::TYPE,
        next: ptr::null(),
        passthrough,
        flags,
        purpose: xr::PassthroughLayerPurposeFB::RECONSTRUCTION,
    };
    let mut layer = xr::sys::PassthroughLayerFB::NULL;
    let result = unsafe {
        (fb_passthrough.create_passthrough_layer)(session.as_raw(), &layer_info, &mut layer)
    };
    if let Err(error) = ensure_xr_success(result, "xrCreatePassthroughLayerFB") {
        let destroy_result = unsafe { (fb_passthrough.destroy_passthrough)(passthrough) };
        if destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
            log_error(format!(
                "Rusty XR OpenXR passthrough cleanup after layer create failed result={destroy_result:?}"
            ));
        }
        return Err(error);
    }

    let result = unsafe { (fb_passthrough.passthrough_start)(passthrough) };
    if let Err(error) = ensure_xr_success(result, "xrPassthroughStartFB") {
        unsafe {
            let layer_destroy_result = (fb_passthrough.destroy_passthrough_layer)(layer);
            if layer_destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR passthrough layer cleanup after start failed result={layer_destroy_result:?}"
                ));
            }
            let passthrough_destroy_result = (fb_passthrough.destroy_passthrough)(passthrough);
            if passthrough_destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR passthrough cleanup after start failed result={passthrough_destroy_result:?}"
                ));
            }
        }
        return Err(error);
    }

    let result = unsafe { (fb_passthrough.passthrough_layer_resume)(layer) };
    if let Err(error) = ensure_xr_success(result, "xrPassthroughLayerResumeFB") {
        unsafe {
            let passthrough_pause_result = (fb_passthrough.passthrough_pause)(passthrough);
            if passthrough_pause_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR passthrough pause cleanup after layer resume failed result={passthrough_pause_result:?}"
                ));
            }
            let layer_destroy_result = (fb_passthrough.destroy_passthrough_layer)(layer);
            if layer_destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR passthrough layer cleanup after resume failed result={layer_destroy_result:?}"
                ));
            }
            let passthrough_destroy_result = (fb_passthrough.destroy_passthrough)(passthrough);
            if passthrough_destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR passthrough cleanup after resume failed result={passthrough_destroy_result:?}"
                ));
            }
        }
        return Err(error);
    }

    log_info(format!(
        "Rusty XR OpenXR passthrough started mode={mode:?} purpose={:?}",
        xr::PassthroughLayerPurposeFB::RECONSTRUCTION
    ));

    Ok(OpenXrPassthroughProbe {
        mode,
        fb_passthrough,
        passthrough,
        layer,
        start_frame,
        paused: false,
        last_style_signature: None,
        lut_max_resolution: passthrough_lut_max_resolution,
        lut_resources: None,
        lut_flicker: PassthroughLutFlickerStats::default(),
    })
}

fn passthrough_style_signature(config: &RuntimeConfig, flicker_state: Option<bool>) -> String {
    format!(
        "{}:{}:{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{:?}",
        config.passthrough_style_mode.stable_id(),
        config.passthrough_opacity.to_bits(),
        config.passthrough_edge_color.map(f32::to_bits),
        config.passthrough_brightness.to_bits(),
        config.passthrough_contrast.to_bits(),
        config.passthrough_saturation.to_bits(),
        config.passthrough_color_phase.to_bits(),
        config.passthrough_color_amplitude.to_bits(),
        config.passthrough_lut_resolution,
        config.passthrough_lut_weight.to_bits(),
        config.passthrough_lut_flicker_hz.to_bits(),
        flicker_state
    )
}

fn apply_passthrough_layer_style(
    instance: &xr::Instance,
    probe: &mut OpenXrPassthroughProbe,
    config: &RuntimeConfig,
    flicker_state: Option<bool>,
) -> Result<(), String> {
    let Some(fb_passthrough) = instance.exts().fb_passthrough.as_ref() else {
        return Err("XR_FB_passthrough function table is unavailable".to_string());
    };
    let layer = probe.layer;

    let edge_color = xr::sys::Color4f {
        r: config.passthrough_edge_color[0],
        g: config.passthrough_edge_color[1],
        b: config.passthrough_edge_color[2],
        a: config.passthrough_edge_color[3],
    };
    match config.passthrough_style_mode {
        OpenXrPassthroughStyleMode::None => {
            let style = xr::sys::PassthroughStyleFB {
                ty: xr::sys::PassthroughStyleFB::TYPE,
                next: ptr::null(),
                texture_opacity_factor: config.passthrough_opacity,
                edge_color,
            };
            let result = unsafe { (fb_passthrough.passthrough_layer_set_style)(layer, &style) };
            ensure_xr_success(result, "xrPassthroughLayerSetStyleFB")
        }
        OpenXrPassthroughStyleMode::BrightnessContrastSaturation => {
            let bcs = xr::sys::PassthroughBrightnessContrastSaturationFB {
                ty: xr::sys::PassthroughBrightnessContrastSaturationFB::TYPE,
                next: ptr::null(),
                brightness: config.passthrough_brightness,
                contrast: config.passthrough_contrast,
                saturation: config.passthrough_saturation,
            };
            let style = xr::sys::PassthroughStyleFB {
                ty: xr::sys::PassthroughStyleFB::TYPE,
                next: &bcs as *const _ as *const _,
                texture_opacity_factor: config.passthrough_opacity,
                edge_color,
            };
            let result = unsafe { (fb_passthrough.passthrough_layer_set_style)(layer, &style) };
            ensure_xr_success(result, "xrPassthroughLayerSetStyleFB")
        }
        OpenXrPassthroughStyleMode::MonoToRgba => {
            let color_map = passthrough_rgba_color_map(config);
            let map = xr::sys::PassthroughColorMapMonoToRgbaFB {
                ty: xr::sys::PassthroughColorMapMonoToRgbaFB::TYPE,
                next: ptr::null(),
                texture_color_map: color_map,
            };
            let style = xr::sys::PassthroughStyleFB {
                ty: xr::sys::PassthroughStyleFB::TYPE,
                next: &map as *const _ as *const _,
                texture_opacity_factor: config.passthrough_opacity,
                edge_color,
            };
            let result = unsafe { (fb_passthrough.passthrough_layer_set_style)(layer, &style) };
            ensure_xr_success(result, "xrPassthroughLayerSetStyleFB")
        }
        OpenXrPassthroughStyleMode::ColorLut => {
            let Some(meta_lut) = instance.exts().meta_passthrough_color_lut.as_ref() else {
                return Err(
                    "XR_META_passthrough_color_lut function table is unavailable".to_string(),
                );
            };
            ensure_passthrough_lut_resources(probe, *meta_lut, config)?;
            let resources = probe
                .lut_resources
                .as_ref()
                .ok_or_else(|| "passthrough LUT resources were not created".to_string())?;
            if let Some(use_target) = flicker_state {
                let lut = xr::sys::PassthroughColorMapInterpolatedLutMETA {
                    ty: xr::sys::PassthroughColorMapInterpolatedLutMETA::TYPE,
                    next: ptr::null(),
                    source_color_lut: resources.source,
                    target_color_lut: resources.target,
                    weight: if use_target { 1.0 } else { 0.0 },
                };
                let style = xr::sys::PassthroughStyleFB {
                    ty: xr::sys::PassthroughStyleFB::TYPE,
                    next: &lut as *const _ as *const _,
                    texture_opacity_factor: config.passthrough_opacity,
                    edge_color,
                };
                let result = unsafe { (fb_passthrough.passthrough_layer_set_style)(layer, &style) };
                ensure_xr_success(result, "xrPassthroughLayerSetStyleFB")
            } else {
                let lut = xr::sys::PassthroughColorMapLutMETA {
                    ty: xr::sys::PassthroughColorMapLutMETA::TYPE,
                    next: ptr::null(),
                    color_lut: resources.source,
                    weight: config.passthrough_lut_weight,
                };
                let style = xr::sys::PassthroughStyleFB {
                    ty: xr::sys::PassthroughStyleFB::TYPE,
                    next: &lut as *const _ as *const _,
                    texture_opacity_factor: config.passthrough_opacity,
                    edge_color,
                };
                let result = unsafe { (fb_passthrough.passthrough_layer_set_style)(layer, &style) };
                ensure_xr_success(result, "xrPassthroughLayerSetStyleFB")
            }
        }
    }
}

fn ensure_passthrough_lut_resources(
    probe: &mut OpenXrPassthroughProbe,
    extension: xr::raw::PassthroughColorLutMETA,
    config: &RuntimeConfig,
) -> Result<(), String> {
    let resolution = effective_passthrough_lut_resolution(
        config.passthrough_lut_resolution,
        probe.lut_max_resolution,
    );
    let signature = format!(
        "{}:{}:{}",
        resolution,
        config.passthrough_color_phase.to_bits(),
        config.passthrough_color_amplitude.to_bits()
    );
    if probe
        .lut_resources
        .as_ref()
        .map(|resources| resources.signature.as_str() == signature.as_str())
        .unwrap_or(false)
    {
        return Ok(());
    }

    let source_data = passthrough_rgb_lut_data(
        resolution,
        config.passthrough_color_phase,
        config.passthrough_color_amplitude,
        false,
    );
    let target_data = passthrough_rgb_lut_data(
        resolution,
        config.passthrough_color_phase,
        config.passthrough_color_amplitude,
        true,
    );
    let source = create_passthrough_color_lut(
        extension,
        probe.passthrough,
        resolution,
        &source_data,
        "source",
    )?;
    let target = match create_passthrough_color_lut(
        extension,
        probe.passthrough,
        resolution,
        &target_data,
        "target",
    ) {
        Ok(target) => target,
        Err(error) => {
            let destroy_result = unsafe { (extension.destroy_passthrough_color_lut)(source) };
            if destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR passthrough LUT cleanup failed after target create error: {destroy_result:?}"
                ));
            }
            return Err(error);
        }
    };
    probe.lut_resources = Some(PassthroughLutResources {
        extension,
        source,
        target,
        signature,
    });
    log_info(format!(
        "Rusty XR OpenXR passthrough LUT resources created resolution={} bytesPerLut={} colorPhase={} colorAmplitude={}",
        resolution,
        source_data.len(),
        config.passthrough_color_phase,
        config.passthrough_color_amplitude
    ));
    Ok(())
}

fn create_passthrough_color_lut(
    extension: xr::raw::PassthroughColorLutMETA,
    passthrough: xr::sys::PassthroughFB,
    resolution: u32,
    data: &[u8],
    label: &str,
) -> Result<xr::sys::PassthroughColorLutMETA, String> {
    let create_info = xr::sys::PassthroughColorLutCreateInfoMETA {
        ty: xr::sys::PassthroughColorLutCreateInfoMETA::TYPE,
        next: ptr::null(),
        channels: xr::sys::PassthroughColorLutChannelsMETA::RGB,
        resolution,
        data: xr::sys::PassthroughColorLutDataMETA {
            buffer_size: data.len() as u32,
            buffer: data.as_ptr(),
        },
    };
    let mut color_lut = xr::sys::PassthroughColorLutMETA::NULL;
    let result = unsafe {
        (extension.create_passthrough_color_lut)(passthrough, &create_info, &mut color_lut)
    };
    ensure_xr_success(result, &format!("xrCreatePassthroughColorLutMETA({label})"))?;
    Ok(color_lut)
}

fn effective_passthrough_lut_resolution(requested: u32, max_resolution: Option<u32>) -> u32 {
    let capped = requested.clamp(2, max_resolution.unwrap_or(64).max(2));
    if capped.is_power_of_two() {
        capped
    } else if capped >= 32 {
        32
    } else if capped >= 16 {
        16
    } else if capped >= 8 {
        8
    } else if capped >= 4 {
        4
    } else {
        2
    }
}

fn passthrough_rgb_lut_data(
    resolution: u32,
    phase: f32,
    amplitude: f32,
    inverted: bool,
) -> Vec<u8> {
    let mut data = Vec::with_capacity((resolution * resolution * resolution * 3) as usize);
    let denominator = (resolution - 1).max(1) as f32;
    let phase_offset = if inverted { 0.5 } else { 0.0 };
    let amplitude = amplitude.clamp(0.0, 1.0);
    for b in 0..resolution {
        for g in 0..resolution {
            for r in 0..resolution {
                let input = [
                    r as f32 / denominator,
                    g as f32 / denominator,
                    b as f32 / denominator,
                ];
                let luminance =
                    (0.2126 * input[0] + 0.7152 * input[1] + 0.0722 * input[2]).clamp(0.0, 1.0);
                let palette =
                    opponent_cosine_palette(luminance + phase.rem_euclid(1.0) + phase_offset);
                for channel in 0..3 {
                    let value = input[channel] * (1.0 - amplitude) + palette[channel] * amplitude;
                    data.push(unit_float_to_u8(value));
                }
            }
        }
    }
    data
}

fn opponent_cosine_palette(position: f32) -> [f32; 3] {
    let angle = std::f32::consts::TAU * position.rem_euclid(1.0);
    [
        0.5 + 0.5 * angle.cos(),
        0.5 + 0.5 * (angle - (2.0 * std::f32::consts::PI / 3.0)).cos(),
        0.5 + 0.5 * (angle + (2.0 * std::f32::consts::PI / 3.0)).cos(),
    ]
}

fn unit_float_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn passthrough_rgba_color_map(config: &RuntimeConfig) -> [xr::sys::Color4f; 256] {
    let phase = config.passthrough_color_phase.rem_euclid(1.0);
    let amplitude = config.passthrough_color_amplitude.clamp(0.0, 1.0);
    std::array::from_fn(|index| {
        let luminance = index as f32 / 255.0;
        let position = (luminance + phase).rem_euclid(1.0);
        let gradient = public_passthrough_gradient(position);
        xr::sys::Color4f {
            r: luminance * (1.0 - amplitude) + gradient.r * amplitude,
            g: luminance * (1.0 - amplitude) + gradient.g * amplitude,
            b: luminance * (1.0 - amplitude) + gradient.b * amplitude,
            a: 1.0,
        }
    })
}

fn public_passthrough_gradient(position: f32) -> xr::sys::Color4f {
    let stops = [
        (
            0.00,
            xr::sys::Color4f {
                r: 0.03,
                g: 0.04,
                b: 0.10,
                a: 1.0,
            },
        ),
        (
            0.34,
            xr::sys::Color4f {
                r: 0.05,
                g: 0.62,
                b: 0.95,
                a: 1.0,
            },
        ),
        (
            0.68,
            xr::sys::Color4f {
                r: 0.88,
                g: 0.18,
                b: 0.58,
                a: 1.0,
            },
        ),
        (
            1.00,
            xr::sys::Color4f {
                r: 0.96,
                g: 0.86,
                b: 0.24,
                a: 1.0,
            },
        ),
    ];
    for pair in stops.windows(2) {
        let (left_position, left_color) = pair[0];
        let (right_position, right_color) = pair[1];
        if position <= right_position {
            let span = (right_position - left_position).max(f32::EPSILON);
            let amount = ((position - left_position) / span).clamp(0.0, 1.0);
            return mix_xr_color(left_color, right_color, amount);
        }
    }
    stops[stops.len() - 1].1
}

fn mix_xr_color(left: xr::sys::Color4f, right: xr::sys::Color4f, amount: f32) -> xr::sys::Color4f {
    let inverse = 1.0 - amount;
    xr::sys::Color4f {
        r: left.r * inverse + right.r * amount,
        g: left.g * inverse + right.g * amount,
        b: left.b * inverse + right.b * amount,
        a: left.a * inverse + right.a * amount,
    }
}

unsafe fn run_vulkan(
    app: android_activity::AndroidApp,
    xr_instance: xr::Instance,
    system: xr::SystemId,
    environment_blend_mode: xr::EnvironmentBlendMode,
    vk_target_version: u32,
    passthrough_lut_max_resolution: Option<u32>,
    environment_depth_properties: EnvironmentDepthProperties,
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
                .contains(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE)
                .then_some(index as u32)
        })
        .ok_or_else(|| "OpenXR-selected Vulkan device has no graphics+compute queue".to_string())?;

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
    let mut last_requested_display_refresh_hz = None;
    sync_display_refresh_rate(
        &xr_instance,
        &session,
        startup_config.xr_display_refresh_hz,
        &mut last_requested_display_refresh_hz,
    );

    let (reference_space, reference_space_label) =
        create_app_reference_space(&session, startup_config.hand_particle_mode)?;
    let view_reference_space = create_view_reference_space(&session)?;
    log_info(format!(
        "Rusty XR OpenXR reference space for projection, environment depth, and hand mesh={reference_space_label} viewPoseSource=view-space-composed"
    ));

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
    let mut environment_depth_visualizer = EnvironmentDepthVisualizer::new(render_pass);
    let mut openxr_hand_particle_source: Option<OpenXrHandMeshParticleSource> = None;
    let mut hand_particle_renderer = HandMeshParticleRenderer::new(render_pass);
    let mut osc_diagnostics_overlay = OscDiagnosticsOverlay::new(render_pass);
    let mut last_logged_gpu_frame_index: Option<u64> = None;
    let mut last_logged_prepared_gpu_frame_index: Option<u64> = None;
    let mut last_logged_prepared_stereo_frame_index: Option<u64> = None;
    let mut openxr_environment_depth_probe: Option<OpenXrEnvironmentDepthProbe> = None;
    let mut openxr_passthrough_probe: Option<OpenXrPassthroughProbe> = None;
    let mut temporal_projection_diagnostics = TemporalProjectionDiagnostics::default();
    let mut camera_render_cadence = CameraRenderCadenceStats::default();
    let mut full_field_flicker = FullFieldFlickerStats::default();
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
                            let config = runtime_config();
                            if openxr_environment_depth_probe.is_some()
                                && !openxr_environment_depth_probe_reuses_existing(
                                    openxr_environment_depth_probe.as_ref(),
                                    config.environment_depth_mode,
                                    config.environment_depth_hand_removal,
                                    environment_depth_properties,
                                )
                            {
                                environment_depth_visualizer.destroy(&vk_device);
                            }
                            openxr_environment_depth_probe = sync_openxr_environment_depth_probe(
                                &xr_instance,
                                &session,
                                openxr_environment_depth_probe,
                                config.environment_depth_mode,
                                config.environment_depth_hand_removal,
                                environment_depth_properties,
                                frame_count,
                            );
                            openxr_passthrough_probe = sync_openxr_passthrough_probe(
                                &xr_instance,
                                &session,
                                openxr_passthrough_probe,
                                config.openxr_passthrough_probe,
                                frame_count,
                                passthrough_lut_max_resolution,
                            );
                            openxr_hand_particle_source = sync_openxr_hand_particle_source(
                                &xr_instance,
                                system,
                                &session,
                                openxr_hand_particle_source,
                                config.hand_particle_mode,
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
                            vk_device.wait_for_fences(&fences, true, u64::MAX).map_err(
                                |error| {
                                    format!("wait Vulkan fences before depth shutdown: {error}")
                                },
                            )?;
                            environment_depth_visualizer.destroy(&vk_device);
                            hand_particle_renderer.destroy(&vk_device);
                            osc_diagnostics_overlay.destroy(&vk_device);
                            openxr_environment_depth_probe = None;
                            openxr_passthrough_probe = None;
                            openxr_hand_particle_source = None;
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
        let reference_from_view = view_reference_space
            .locate(&reference_space, frame_state.predicted_display_time)
            .map_err(|error| format!("locate OpenXR VIEW space from reference space: {error}"))?;
        let (view_state_flags, head_space_views) = session
            .locate_views(
                VIEW_TYPE,
                frame_state.predicted_display_time,
                &view_reference_space,
            )
            .map_err(|error| format!("locate OpenXR views: {error}"))?;
        let views = compose_head_space_views(reference_from_view.pose, &head_space_views);
        if views.len() != VIEW_COUNT as usize {
            return Err(format!(
                "expected {VIEW_COUNT} OpenXR views, got {}",
                views.len()
            ));
        }
        let view_space_valid = space_location_pose_valid(reference_from_view);
        let views_valid = view_space_valid
            && view_state_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
            && view_state_flags.contains(xr::ViewStateFlags::POSITION_VALID);
        if !views_valid {
            if frame_count == 0 || frame_count % 120 == 0 {
                log_info(format!(
                    "Rusty XR skipped composition frame {} because OpenXR view pose is not valid yet viewFlags={:?} referenceFromViewFlags={:?}",
                    frame_count, view_state_flags, reference_from_view.location_flags
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
        let config = runtime_config();
        if openxr_environment_depth_probe.is_some()
            && !openxr_environment_depth_probe_reuses_existing(
                openxr_environment_depth_probe.as_ref(),
                config.environment_depth_mode,
                config.environment_depth_hand_removal,
                environment_depth_properties,
            )
        {
            vk_device
                .wait_for_fences(&fences, true, u64::MAX)
                .map_err(|error| {
                    format!("wait Vulkan fences before depth visualizer reconfiguration: {error}")
                })?;
            environment_depth_visualizer.destroy(&vk_device);
        }
        vk_device
            .reset_fences(&[fences[frame]])
            .map_err(|error| format!("reset Vulkan fence: {error}"))?;

        sync_display_refresh_rate(
            &xr_instance,
            &session,
            config.xr_display_refresh_hz,
            &mut last_requested_display_refresh_hz,
        );
        openxr_passthrough_probe = sync_openxr_passthrough_probe(
            &xr_instance,
            &session,
            openxr_passthrough_probe,
            config.openxr_passthrough_probe,
            frame_count,
            passthrough_lut_max_resolution,
        );
        openxr_environment_depth_probe = sync_openxr_environment_depth_probe(
            &xr_instance,
            &session,
            openxr_environment_depth_probe,
            config.environment_depth_mode,
            config.environment_depth_hand_removal,
            environment_depth_properties,
            frame_count,
        );
        let mut depth_visual_frame: Option<EnvironmentDepthVisualFrame> = None;
        let depth_visual_clear = if let Some(probe) = openxr_environment_depth_probe.as_mut() {
            let acquired_depth_visual_frame = probe.acquire(
                &view_reference_space,
                reference_from_view.pose,
                frame_state.predicted_display_time,
                &views,
                frame_count,
            );
            if let Some(frame) = acquired_depth_visual_frame {
                match environment_depth_visualizer.prepare(
                    &vk_device,
                    &memory_properties,
                    probe.depth_image_handles(),
                    frame.depth_width,
                    frame.depth_height,
                ) {
                    Ok(true) => {
                        depth_visual_frame = Some(frame);
                    }
                    Ok(false) => {}
                    Err(error) => {
                        if frame_count == 0 || frame_count % 120 == 0 {
                            log_error(format!(
                                "Rusty XR environment depth visualizer prepare failed: {error}"
                            ));
                        }
                    }
                }
            }
            probe.visual_clear_color(frame_count)
        } else {
            None
        };
        if let Some(probe) = openxr_passthrough_probe.as_mut() {
            probe.apply_style(
                &xr_instance,
                &config,
                frame_state.predicted_display_time,
                frame_count,
            );
        }
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
                        let projection_homographies = CameraProjectionPush::from_stereo_frame(
                            &stereo_frame,
                            &config,
                            &controls,
                            &views,
                            swapchain.resolution,
                        )
                        .2;
                        let projection_active = projection_homographies.is_some();
                        let temporal_metrics = temporal_projection_diagnostics.update(
                            projection_homographies.as_ref(),
                            &stereo_frame,
                            frame_state.predicted_display_time,
                            swapchain.resolution,
                        );
                        let camera_cadence_metrics =
                            camera_render_cadence.record(stereo_frame.index);
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
                            let projection_homography_fields = projection_homographies
                                .as_ref()
                                .map(projected_homography_marker_fields)
                                .unwrap_or_else(|| {
                                    "projectionHomographyReady=false projectionAreaTransformStage=none projectionAreaWarpParity=reference_unwarped_screen_uv".to_string()
                                });
                            let orientation_accepted =
                                controls.left_texture_transform.is_explicit_visual_check()
                                    && controls.right_texture_transform.is_explicit_visual_check();
                            log_info(format!(
                                "Rusty XR final projection status frame={} openXrFrameCount={} openXrFocused={} activeTier=gpu-projected alignedProjection={} {} stereoLayout=Separate pairedLeftRightGpuBuffers=true poseSource={} poseReference={} poseConvention={} projectionMode={} cameraFeedMode={} cameraColorMode={} cameraColorShaderBit={} cameraColorContrast={} cameraColorBrightness={} cameraColorSaturation={} cameraImportImageLayout={} importCacheLimit={} sourceEyeMapping={} displayLeftCameraId={} displayRightCameraId={} leftCameraTextureTransform={} rightCameraTextureTransform={} cameraTextureTransformSource={} cameraTextureTransformReason={} orientationCheck=true orientationAccepted={} cpuUploadCount=0 projectionShaderPath=projected projectionSurface={} coordinateChain=camera2-sensor-reference-to-openxr-head-basis importCacheSize={} stereoDescriptorCacheSize={} noHardwareBufferLifetimeWarnings=true frameCadenceTargetHz={} visualInspection={} visualReleaseAccepted={} orientationDiagnosticMode={} orientationDiagnosticStep={} temporalProjectionMode=metrics-only cameraFrameAgeMsAvg={} cameraFrameAgeMsP95={} stereoPairDeltaMsAvg={:.3} targetProjectionMotionPxAvg={:.3} targetProjectionMotionPxP95={:.3} appliedProjectionMotionPxAvg={:.3} appliedProjectionMotionPxP95={:.3} projectionResidualPxAvg={:.3} projectionResidualPxP95={:.3} visualLagMsAvg={:.3} visualLagMsP95={:.3} heldFrameCount={} heldFrameDurationMsMax={:.3} frameCrossfadeCount={} invalidUvPxPercent={:.3} edgeFillPxPercent={:.3} aswEnabledFrameCount={} aswSkippedFrameCount={} motionVectorMaxPx={:.3} motionVectorClampedCount={} cameraProjectionRenderFrameCount={} cameraDistinctFrameCount={} cameraRepeatedRenderFrameCount={} cameraRendersPerCameraFrameAvg={:.3} cameraMaxConsecutiveRenderFramesPerCameraFrame={} cameraConsumedFrameHz={:.3} cameraProjectionRenderHz={:.3}",
                                stereo_frame.index,
                                frame_count,
                                session_focused,
                                aligned_projection,
                                projection_homography_fields,
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
                                config.xr_display_refresh_hz,
                                if config.visual_release_accepted { "accepted" } else { "required" },
                                config.visual_release_accepted,
                                controls.diagnostic_mode.stable_id(),
                                controls.diagnostic_step,
                                optional_ms_metric_label(temporal_metrics.camera_frame_age_ms),
                                optional_ms_metric_label(temporal_metrics.camera_frame_age_ms),
                                temporal_metrics.stereo_pair_delta_ms,
                                temporal_metrics.target_projection_motion_px_avg,
                                temporal_metrics.target_projection_motion_px_p95,
                                temporal_metrics.applied_projection_motion_px_avg,
                                temporal_metrics.applied_projection_motion_px_p95,
                                temporal_metrics.projection_residual_px_avg,
                                temporal_metrics.projection_residual_px_p95,
                                temporal_metrics.visual_lag_ms_avg,
                                temporal_metrics.visual_lag_ms_p95,
                                temporal_metrics.held_frame_count,
                                temporal_metrics.held_frame_duration_ms_max,
                                temporal_metrics.frame_crossfade_count,
                                temporal_metrics.invalid_uv_px_percent,
                                temporal_metrics.edge_fill_px_percent,
                                temporal_metrics.asw_enabled_frame_count,
                                temporal_metrics.asw_skipped_frame_count,
                                temporal_metrics.motion_vector_max_px,
                                temporal_metrics.motion_vector_clamped_count,
                                camera_cadence_metrics.render_frame_count,
                                camera_cadence_metrics.distinct_frame_count,
                                camera_cadence_metrics.repeated_render_frame_count,
                                camera_cadence_metrics.renders_per_camera_frame_avg,
                                camera_cadence_metrics.max_consecutive_render_frames_per_camera_frame,
                                camera_cadence_metrics.consumed_frame_hz,
                                camera_cadence_metrics.projection_render_hz
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
                        if last_logged_prepared_gpu_frame_index.is_none()
                            || last_logged_prepared_gpu_frame_index != Some(gpu_frame.index)
                                && gpu_frame.index % 120 == 0
                        {
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
                            last_logged_prepared_gpu_frame_index = Some(gpu_frame.index);
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

        let full_field_red = full_field_flicker.tick(
            config.full_field_flicker_hz,
            frame_state.predicted_display_time,
            frame_count,
        );
        let clear = if let Some(clear) = depth_visual_clear {
            clear
        } else if config.full_field_flicker_hz > 0.0 {
            if full_field_red {
                [1.0, 0.0, 0.0, 1.0]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            }
        } else if config.openxr_passthrough_probe.submits_composition_layer() {
            [0.0, 0.0, 0.0, 0.0]
        } else if config.camera_tier == CameraCompositeTier::GpuProjected {
            [0.0, 0.0, 0.0, 1.0]
        } else if frame_count % 120 < 60 {
            [0.02, 0.22, 0.26, 1.0]
        } else {
            [0.08, 0.12, 0.30, 1.0]
        };
        if let Some(frame) = depth_visual_frame {
            environment_depth_visualizer.record_particle_update(
                &vk_device,
                cmd,
                frame,
                config.environment_depth_mode,
            );
        }
        if config.hand_particle_mode.uses_openxr_hand_mesh()
            && openxr_hand_particle_source.is_none()
        {
            openxr_hand_particle_source = sync_openxr_hand_particle_source(
                &xr_instance,
                system,
                &session,
                openxr_hand_particle_source,
                config.hand_particle_mode,
                frame_count,
            );
        }
        let hand_particles = match config.hand_particle_mode {
            HandParticleMode::Meta => openxr_hand_particle_source
                .as_mut()
                .map(|source| {
                    source.update(
                        &reference_space,
                        frame_state.predicted_display_time,
                        frame_count,
                        &views,
                    )
                })
                .unwrap_or_else(|| {
                    if frame_count == 0 || frame_count.is_multiple_of(120) {
                        log_error(
                            "Rusty XR OpenXR hand mesh particle source unavailable mode=meta particles=0",
                        );
                    }
                    Vec::new()
                }),
            HandParticleMode::Off => Vec::new(),
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
        if let Some(frame) = depth_visual_frame {
            environment_depth_visualizer.record_draw(
                &vk_device,
                cmd,
                swapchain.resolution,
                frame,
                config.environment_depth_mode,
            );
        }
        hand_particle_renderer.record_draw(
            HandParticleDrawContext {
                device: &vk_device,
                memory_properties: &memory_properties,
                cmd,
                resolution: swapchain.resolution,
                views: &views,
                frame_count,
            },
            config.hand_particle_mode,
            &hand_particles,
        );
        osc_diagnostics_overlay.record_draw(
            &vk_device,
            &memory_properties,
            cmd,
            swapchain.resolution,
            &config,
            &views,
            frame_count,
        );
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
                layer_handle: probe.layer,
            });
        let projection_layer_visible =
            config.projection_layer_visible || passthrough_composition_layer.is_none();
        let mut layers: Vec<&xr::CompositionLayerBase<xr::Vulkan>> = Vec::with_capacity(
            (passthrough_composition_layer.is_some() as usize)
                + (projection_layer_visible as usize),
        );
        if let Some(layer) = passthrough_composition_layer.as_ref() {
            // The openxr crate does not re-export this FB layer builder, but the raw
            // struct has the standard composition-layer header prefix expected here.
            let layer_base: &xr::CompositionLayerBase<xr::Vulkan> = unsafe {
                &*(layer as *const xr::sys::CompositionLayerPassthroughFB
                    as *const xr::CompositionLayerBase<xr::Vulkan>)
            };
            layers.push(layer_base);
        }
        if projection_layer_visible {
            layers.push(&projection_layer);
        }
        let submitted_layer_count = layers.len();
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
                "Rusty XR OpenXR frame {} rendered {}x{} requestedTier={} cameraAcquisition={} cameraEnabled={} mediaProjection={} environmentDepthMode={} environmentDepthActive={} handParticleMode={} observedOpenXrFps={:.1} avgFrameMs={:.2} recordCpuMs={:.3} submitCpuMs={:.3} frameCadenceTargetHz={} activeDisplayRefreshHz={} renderScale={} fixedFoveationLevel={} fixedFoveationEnabled={} openxrPassthroughProbe={} projectionLayerVisible={} submittedLayerCount={} fenceSync=slot-reuse pipelineDepth={} gpuProbeSuccess={} gpuProbeFailure={} descriptorProbeCacheSize={} importCacheSize={} importCacheLimit={} stereoDescriptorCacheSize={} gpuImportSuccess={} gpuImportFailure={} gpuImportCacheHit={} gpuImportCacheMiss={} gpuImportCacheEvict={}",
                frame_count,
                swapchain.resolution.width,
                swapchain.resolution.height,
                config.camera_tier.stable_id(),
                config.camera_acquisition.as_str(),
                config.camera_enabled,
                config.media_projection_enabled,
                config.environment_depth_mode.stable_id(),
                openxr_environment_depth_probe.is_some(),
                config.hand_particle_mode.stable_id(),
                observed_openxr_fps,
                avg_frame_ms,
                record_ms,
                submit_ms,
                config.xr_display_refresh_hz,
                refresh_rate_label(active_display_refresh_hz),
                config.xr_render_scale,
                config.xr_fixed_foveation_level,
                swapchain.foveation_enabled,
                openxr_passthrough_probe
                    .as_ref()
                    .map(|probe| probe.mode.stable_id())
                    .unwrap_or("off"),
                projection_layer_visible,
                submitted_layer_count,
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

    vk_device
        .wait_for_fences(&fences, true, u64::MAX)
        .map_err(|error| format!("final Vulkan fence wait: {error}"))?;
    environment_depth_visualizer.destroy(&vk_device);
    hand_particle_renderer.destroy(&vk_device);
    osc_diagnostics_overlay.destroy(&vk_device);
    drop(openxr_environment_depth_probe.take());
    drop(openxr_passthrough_probe.take());
    drop(openxr_hand_particle_source.take());
    drop((
        session,
        frame_wait,
        frame_stream,
        reference_space,
        view_reference_space,
    ));
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
            "Rusty XR Vulkan imported camera hardware buffer size={}x{} nativeFormat={} externalFormat={} vkFormat={:?} samplerBindingMode={} importImageLayout={} allocationSize={} memoryTypeBits=0x{:x} suggestedYcbcrModel={:?} suggestedYcbcrRange={:?} samplerYcbcrComponents={:?} suggestedXChromaOffset={:?} suggestedYChromaOffset={:?} importCacheSize={} importCacheLimit={} importCacheMiss={} importCacheEvict={}",
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
        device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_for_config(config),
        );
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
        let (push, uniforms, projection_homographies) =
            CameraProjectionPush::from_stereo_frame(frame, config, &controls, views, resolution);
        let projection_active = projection_homographies.is_some();
        let uniforms = uniforms.with_border_cycle_phase(config, frame_count);
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
        device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_for_config(config),
        );
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
    fast_pipeline: vk::Pipeline,
    projection_uniform_buffer: vk::Buffer,
    projection_uniform_memory: vk::DeviceMemory,
    projection_uniform_stride: vk::DeviceSize,
    projection_uniform_slots: u32,
}

impl GpuCameraPipelineResources {
    unsafe fn destroy(self, device: &ash::Device) {
        device.destroy_pipeline(self.fast_pipeline, None);
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

    fn pipeline_for_config(&self, config: &crate::RuntimeConfig) -> vk::Pipeline {
        if config
            .camera_projection_effect_mode
            .uses_fast_projection_pipeline()
        {
            self.fast_pipeline
        } else {
            self.pipeline
        }
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

    fn with_border_cycle_phase(mut self, config: &crate::RuntimeConfig, frame_count: u64) -> Self {
        self.color_offset[3] = config.camera_border_cycle_phase(frame_count);
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
    ) -> (
        Self,
        CameraProjectionUniforms,
        Option<ProjectedStereoHomographies>,
    ) {
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
                None,
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
                Some(ProjectedStereoHomographies { left, right }),
            );
        }
        (
            push,
            CameraProjectionUniforms::identity().with_color_config(config),
            None,
        )
    }
}

#[derive(Clone, Copy)]
struct ProjectedStereoHomographies {
    left: DisplayEyeProjectionMapping,
    right: DisplayEyeProjectionMapping,
}

#[derive(Clone, Copy)]
struct DisplayEyeProjectionMapping {
    surface_to_camera: [[f32; 3]; 3],
    screen_to_camera: [[f32; 3]; 3],
    screen_to_surface: [[f32; 3]; 3],
    surface_to_screen: [[f32; 3]; 3],
}

#[derive(Clone, Copy, Default)]
struct TemporalProjectionMetricsFrame {
    camera_frame_age_ms: Option<f64>,
    stereo_pair_delta_ms: f64,
    target_projection_motion_px_avg: f64,
    target_projection_motion_px_p95: f64,
    applied_projection_motion_px_avg: f64,
    applied_projection_motion_px_p95: f64,
    projection_residual_px_avg: f64,
    projection_residual_px_p95: f64,
    visual_lag_ms_avg: f64,
    visual_lag_ms_p95: f64,
    held_frame_count: u64,
    held_frame_duration_ms_max: f64,
    frame_crossfade_count: u64,
    invalid_uv_px_percent: f64,
    edge_fill_px_percent: f64,
    asw_enabled_frame_count: u64,
    asw_skipped_frame_count: u64,
    motion_vector_max_px: f64,
    motion_vector_clamped_count: u64,
}

#[derive(Clone, Copy, Default)]
struct CameraRenderCadenceFrame {
    render_frame_count: u64,
    distinct_frame_count: u64,
    repeated_render_frame_count: u64,
    renders_per_camera_frame_avg: f64,
    max_consecutive_render_frames_per_camera_frame: u64,
    consumed_frame_hz: f64,
    projection_render_hz: f64,
}

struct CameraRenderCadenceStats {
    started: Option<Instant>,
    render_frame_count: u64,
    distinct_frame_count: u64,
    repeated_render_frame_count: u64,
    last_camera_frame_index: Option<u64>,
    current_consecutive_render_frames: u64,
    max_consecutive_render_frames_per_camera_frame: u64,
}

impl Default for CameraRenderCadenceStats {
    fn default() -> Self {
        Self {
            started: None,
            render_frame_count: 0,
            distinct_frame_count: 0,
            repeated_render_frame_count: 0,
            last_camera_frame_index: None,
            current_consecutive_render_frames: 0,
            max_consecutive_render_frames_per_camera_frame: 0,
        }
    }
}

impl CameraRenderCadenceStats {
    fn record(&mut self, camera_frame_index: u64) -> CameraRenderCadenceFrame {
        let started = *self.started.get_or_insert_with(Instant::now);
        self.render_frame_count = self.render_frame_count.saturating_add(1);

        if self.last_camera_frame_index == Some(camera_frame_index) {
            self.repeated_render_frame_count = self.repeated_render_frame_count.saturating_add(1);
            self.current_consecutive_render_frames =
                self.current_consecutive_render_frames.saturating_add(1);
        } else {
            self.distinct_frame_count = self.distinct_frame_count.saturating_add(1);
            self.last_camera_frame_index = Some(camera_frame_index);
            self.current_consecutive_render_frames = 1;
        }

        self.max_consecutive_render_frames_per_camera_frame = self
            .max_consecutive_render_frames_per_camera_frame
            .max(self.current_consecutive_render_frames);

        let elapsed_seconds = started.elapsed().as_secs_f64();
        let hz_divisor = if elapsed_seconds > 0.001 {
            elapsed_seconds
        } else {
            f64::INFINITY
        };
        let renders_per_camera_frame_avg = if self.distinct_frame_count > 0 {
            self.render_frame_count as f64 / self.distinct_frame_count as f64
        } else {
            0.0
        };

        CameraRenderCadenceFrame {
            render_frame_count: self.render_frame_count,
            distinct_frame_count: self.distinct_frame_count,
            repeated_render_frame_count: self.repeated_render_frame_count,
            renders_per_camera_frame_avg,
            max_consecutive_render_frames_per_camera_frame: self
                .max_consecutive_render_frames_per_camera_frame,
            consumed_frame_hz: self.distinct_frame_count as f64 / hz_divisor,
            projection_render_hz: self.render_frame_count as f64 / hz_divisor,
        }
    }
}

#[derive(Default)]
struct TemporalProjectionDiagnostics {
    previous: Option<StereoHomographyProjection>,
}

impl TemporalProjectionDiagnostics {
    fn update(
        &mut self,
        homographies: Option<&ProjectedStereoHomographies>,
        frame: &StereoGpuCameraFrame,
        predicted_display_time: xr::Time,
        resolution: vk::Extent2D,
    ) -> TemporalProjectionMetricsFrame {
        let Some(homographies) = homographies else {
            self.previous = None;
            return TemporalProjectionMetricsFrame {
                stereo_pair_delta_ms: ns_to_ms(frame.pair_delta_ns),
                ..TemporalProjectionMetricsFrame::default()
            };
        };

        let current = StereoHomographyProjection::new(
            homographies.left.screen_to_camera,
            homographies.right.screen_to_camera,
        );
        let metrics = stereo_homography_projection_metrics(
            self.previous,
            current,
            ImageSize::new(resolution.width, resolution.height),
        );
        self.previous = Some(current);

        TemporalProjectionMetricsFrame {
            camera_frame_age_ms: plausible_camera_frame_age_ms(
                predicted_display_time,
                frame.midpoint_timestamp_ns,
            ),
            stereo_pair_delta_ms: ns_to_ms(frame.pair_delta_ns),
            target_projection_motion_px_avg: metrics.average_motion_px as f64,
            target_projection_motion_px_p95: metrics.p95_motion_px as f64,
            applied_projection_motion_px_avg: metrics.average_motion_px as f64,
            applied_projection_motion_px_p95: metrics.p95_motion_px as f64,
            projection_residual_px_avg: 0.0,
            projection_residual_px_p95: 0.0,
            visual_lag_ms_avg: 0.0,
            visual_lag_ms_p95: 0.0,
            held_frame_count: 0,
            held_frame_duration_ms_max: 0.0,
            frame_crossfade_count: 0,
            invalid_uv_px_percent: metrics.invalid_uv_percent as f64,
            edge_fill_px_percent: 0.0,
            asw_enabled_frame_count: 0,
            asw_skipped_frame_count: 0,
            motion_vector_max_px: 0.0,
            motion_vector_clamped_count: 0,
        }
    }
}

fn plausible_camera_frame_age_ms(
    predicted_display_time: xr::Time,
    camera_midpoint_timestamp_ns: i64,
) -> Option<f64> {
    let age_ms = ns_to_ms_signed(
        predicted_display_time
            .as_nanos()
            .saturating_sub(camera_midpoint_timestamp_ns),
    );
    if (-25.0..=10_000.0).contains(&age_ms) {
        Some(age_ms.max(0.0))
    } else {
        None
    }
}

fn ns_to_ms(value: u64) -> f64 {
    value as f64 / 1_000_000.0
}

fn ns_to_ms_signed(value: i64) -> f64 {
    value as f64 / 1_000_000.0
}

fn optional_ms_metric_label(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "unavailable".to_string())
}

fn identity_homography() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

fn pack_homography_row(row: [f32; 3]) -> [f32; 4] {
    [row[0], row[1], row[2], 0.0]
}

fn homography_token(rows: [[f32; 3]; 3]) -> String {
    rows.iter()
        .flat_map(|row| row.iter())
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn projected_homography_marker_fields(homographies: &ProjectedStereoHomographies) -> String {
    format!(
        "projectionHomographyReady=true projectionAreaTransformStage=none projectionAreaWarpParity=reference_unwarped_screen_uv leftSurfaceToCameraH={} rightSurfaceToCameraH={} leftScreenToCameraH={} rightScreenToCameraH={} leftScreenToSurfaceH={} rightScreenToSurfaceH={} leftSurfaceToScreenH={} rightSurfaceToScreenH={}",
        homography_token(homographies.left.surface_to_camera),
        homography_token(homographies.right.surface_to_camera),
        homography_token(homographies.left.screen_to_camera),
        homography_token(homographies.right.screen_to_camera),
        homography_token(homographies.left.screen_to_surface),
        homography_token(homographies.right.screen_to_surface),
        homography_token(homographies.left.surface_to_screen),
        homography_token(homographies.right.surface_to_screen),
    )
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
        surface_to_camera,
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
        false,
    )?;
    let fast_pipeline = match create_gpu_camera_pipeline(
        device,
        render_pass,
        pipeline_layout,
        format_key.sampler_binding_mode,
        true,
    ) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            device.destroy_pipeline(pipeline, None);
            return Err(error);
        }
    };

    Ok(GpuCameraPipelineResources {
        format_key,
        sampler_ycbcr_conversion,
        sampler,
        descriptor_set_layout,
        descriptor_pool,
        pipeline_layout,
        pipeline,
        fast_pipeline,
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
    fast_raw_projection: bool,
) -> Result<vk::Pipeline, String> {
    let vertex_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/camera_projection.vert.spv"
    )))?;
    let fragment_words = match (sampler_binding_mode, fast_raw_projection) {
        (crate::CameraSamplerBindingMode::CombinedImmutableSampler, false) => spirv_words(
            include_bytes!(concat!(env!("OUT_DIR"), "/camera_projection.frag.spv")),
        )?,
        (crate::CameraSamplerBindingMode::CombinedImmutableSampler, true) => spirv_words(
            include_bytes!(concat!(env!("OUT_DIR"), "/camera_projection_fast.frag.spv")),
        )?,
        (crate::CameraSamplerBindingMode::SeparateImageSampler, false) => {
            spirv_words(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/camera_projection_separate_sampler.frag.spv"
            )))?
        }
        (crate::CameraSamplerBindingMode::SeparateImageSampler, true) => {
            spirv_words(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/camera_projection_fast_separate_sampler.frag.spv"
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

struct OscDiagnosticsOverlay {
    render_pass: vk::RenderPass,
    resources: Option<OscDiagnosticsOverlayResources>,
    last_draw_failure_frame: Option<u64>,
    last_stats: Option<OscDiagnosticsOverlayFrameStats>,
}

impl OscDiagnosticsOverlay {
    const fn new(render_pass: vk::RenderPass) -> Self {
        Self {
            render_pass,
            resources: None,
            last_draw_failure_frame: None,
            last_stats: None,
        }
    }

    unsafe fn record_draw(
        &mut self,
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        cmd: vk::CommandBuffer,
        resolution: vk::Extent2D,
        config: &RuntimeConfig,
        views: &[xr::View],
        frame_count: u64,
    ) {
        let total_started = Instant::now();
        let hud = diagnostic_hud_snapshot();
        if !hud.visible {
            self.last_stats = None;
            return;
        }
        let mut setup_ms = 0.0;
        if self.resources.is_none() {
            let setup_started = Instant::now();
            match create_osc_diagnostics_overlay_resources(
                device,
                memory_properties,
                self.render_pass,
            ) {
                Ok(resources) => {
                    setup_ms = elapsed_ms(setup_started);
                    self.resources = Some(resources);
                }
                Err(error) => {
                    if self
                        .last_draw_failure_frame
                        .map(|last| frame_count.saturating_sub(last) >= 120)
                        .unwrap_or(true)
                    {
                        log_error(format!(
                            "Rusty XR OSC diagnostics overlay setup failed: {error}"
                        ));
                        self.last_draw_failure_frame = Some(frame_count);
                    }
                    return;
                }
            }
        }

        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let surface_started = Instant::now();
        let Some(panel_surface) = osc_overlay_surface_for_projection(config, views, resolution)
        else {
            return;
        };
        let surface_ms = elapsed_ms(surface_started);
        let viewport = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: resolution.width as f32,
            height: resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissor = [overlay_surface_to_scissor(&panel_surface, resolution)];

        device.cmd_set_viewport(cmd, 0, &viewport);
        device.cmd_set_scissor(cmd, 0, &scissor);
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, resources.pipeline);

        let layout_started = Instant::now();
        let layout = osc_overlay_canvas_layout();
        let theme = CanvasTheme::default();
        let document = diagnostic_hud_document(config, frame_count, hud);
        let draw_list = layout.layout_document(&document, &theme);
        let layout_ms = elapsed_ms(layout_started);
        let cell_px = [
            scissor[0].extent.width as f32 / layout.columns.max(1) as f32,
            scissor[0].extent.height as f32 / layout.rows.max(1) as f32,
        ];
        match draw_osc_overlay_canvas(device, cmd, resources, &panel_surface, &draw_list) {
            Ok(draw_stats) => {
                let stats = OscDiagnosticsOverlayFrameStats {
                    total_ms: elapsed_ms(total_started),
                    setup_ms,
                    surface_ms,
                    layout_ms,
                    cell_px,
                    draw: draw_stats,
                };
                if frame_count == 1 || frame_count % 120 == 0 {
                    log_info(format!(
                        "Rusty XR diagnostic HUD CPU frame={} totalMs={:.3} setupMs={:.3} surfaceMs={:.3} layoutMs={:.3} buildProjectMs={:.3} uploadMs={:.3} cmdMs={:.3} instances={} rects={} textRuns={} glyphs={} approxCellPx={:.1}x{:.1}",
                        frame_count,
                        stats.total_ms,
                        stats.setup_ms,
                        stats.surface_ms,
                        stats.layout_ms,
                        stats.draw.build_ms,
                        stats.draw.upload_ms,
                        stats.draw.command_ms,
                        stats.draw.instances,
                        stats.draw.rects,
                        stats.draw.text_runs,
                        stats.draw.glyphs,
                        stats.cell_px[0],
                        stats.cell_px[1]
                    ));
                }
                self.last_stats = Some(stats);
            }
            Err(error) => {
                if self
                    .last_draw_failure_frame
                    .map(|last| frame_count.saturating_sub(last) >= 120)
                    .unwrap_or(true)
                {
                    log_error(format!(
                        "Rusty XR OSC diagnostics overlay draw failed: {error}"
                    ));
                    self.last_draw_failure_frame = Some(frame_count);
                }
            }
        }
    }

    unsafe fn destroy(&mut self, device: &ash::Device) {
        if let Some(resources) = self.resources.take() {
            resources.destroy(device);
        }
    }
}

#[derive(Clone, Copy)]
struct OscDiagnosticsOverlayFrameStats {
    total_ms: f64,
    setup_ms: f64,
    surface_ms: f64,
    layout_ms: f64,
    cell_px: [f32; 2],
    draw: OscDiagnosticsOverlayDrawStats,
}

struct OscDiagnosticsOverlayResources {
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    instance_buffer: vk::Buffer,
    instance_memory: vk::DeviceMemory,
    instance_capacity: usize,
    font_atlas_buffer: vk::Buffer,
    font_atlas_memory: vk::DeviceMemory,
}

impl OscDiagnosticsOverlayResources {
    unsafe fn destroy(self, device: &ash::Device) {
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_descriptor_pool(self.descriptor_pool, None);
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        device.destroy_buffer(self.instance_buffer, None);
        device.free_memory(self.instance_memory, None);
        device.destroy_buffer(self.font_atlas_buffer, None);
        device.free_memory(self.font_atlas_memory, None);
    }
}

#[derive(Clone, Copy)]
struct OscOverlayEyeProjection {
    eye: CameraBasis,
    tan_left: f32,
    tan_right: f32,
    tan_down: f32,
    tan_up: f32,
}

#[derive(Clone, Copy)]
struct OscOverlaySurface {
    corners: [Vec3; 4],
    left: OscOverlayEyeProjection,
    right: OscOverlayEyeProjection,
}

#[derive(Clone, Copy)]
struct OscOverlayClipQuad {
    left: [[f32; 4]; 4],
    right: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct OscDiagnosticsOverlayInstance {
    left_clip: [[f32; 4]; 4],
    right_clip: [[f32; 4]; 4],
    color: [f32; 4],
    glyph: [f32; 4],
}

#[derive(Clone, Copy)]
struct OscDiagnosticsOverlayDrawStats {
    instances: usize,
    rects: usize,
    text_runs: usize,
    glyphs: usize,
    build_ms: f64,
    upload_ms: f64,
    command_ms: f64,
}

unsafe fn draw_osc_overlay_canvas(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    resources: &OscDiagnosticsOverlayResources,
    panel_surface: &OscOverlaySurface,
    draw_list: &CanvasDrawList,
) -> Result<OscDiagnosticsOverlayDrawStats, String> {
    let build_started = Instant::now();
    let mut instances = Vec::with_capacity(
        OSC_OVERLAY_MAX_INSTANCES.min(
            draw_list
                .rects
                .len()
                .saturating_add(draw_list.text.iter().map(|run| run.text.len() * 2).sum()),
        ),
    );
    for rect in &draw_list.rects {
        push_osc_overlay_rect_instance(&mut instances, panel_surface, rect.rect, rect.color);
    }
    for text_run in &draw_list.text {
        push_osc_overlay_text_run_instances(
            &mut instances,
            panel_surface,
            text_run,
            draw_list.shadow_color,
        );
    }
    let build_ms = elapsed_ms(build_started);
    let glyphs = instances
        .iter()
        .filter(|instance| instance.glyph[1] > 0.5)
        .count();
    if instances.is_empty() {
        return Ok(OscDiagnosticsOverlayDrawStats {
            instances: 0,
            rects: draw_list.rects.len(),
            text_runs: draw_list.text.len(),
            glyphs,
            build_ms,
            upload_ms: 0.0,
            command_ms: 0.0,
        });
    }
    let upload_started = Instant::now();
    upload_osc_overlay_instances(device, resources, &instances)?;
    let upload_ms = elapsed_ms(upload_started);
    let command_started = Instant::now();
    device.cmd_bind_descriptor_sets(
        cmd,
        vk::PipelineBindPoint::GRAPHICS,
        resources.pipeline_layout,
        0,
        &[resources.descriptor_set],
        &[],
    );
    device.cmd_draw(cmd, 6, instances.len() as u32, 0, 0);
    Ok(OscDiagnosticsOverlayDrawStats {
        instances: instances.len(),
        rects: draw_list.rects.len(),
        text_runs: draw_list.text.len(),
        glyphs,
        build_ms,
        upload_ms,
        command_ms: elapsed_ms(command_started),
    })
}

fn push_osc_overlay_text_run_instances(
    instances: &mut Vec<OscDiagnosticsOverlayInstance>,
    panel_surface: &OscOverlaySurface,
    text_run: &CanvasTextRun,
    shadow_color: [f32; 4],
) {
    let columns = text_run.columns.max(1);
    let cell_w = text_run.rect[2] / columns as f32;
    let cell_h = text_run.rect[3];
    for (index, character) in text_run.text.chars().take(columns).enumerate() {
        let character = sanitize_overlay_char(character);
        if character == ' ' {
            continue;
        }
        let glyph_rect = [
            text_run.rect[0] + index as f32 * cell_w + cell_w * 0.020,
            text_run.rect[1] + cell_h * 0.015,
            cell_w * 0.960,
            cell_h * 0.960,
        ];
        let shadow_alpha = match text_run.role {
            rusty_xr_debug_canvas::CanvasTextRole::Title
            | rusty_xr_debug_canvas::CanvasTextRole::Section => shadow_color[3] * 0.0,
            _ => 0.0,
        };
        if shadow_alpha > 0.0 {
            let shadow_rect = [
                glyph_rect[0] + cell_w * 0.045,
                glyph_rect[1] + cell_h * 0.055,
                glyph_rect[2],
                glyph_rect[3],
            ];
            push_osc_overlay_glyph_instance(
                instances,
                panel_surface,
                shadow_rect,
                [
                    shadow_color[0],
                    shadow_color[1],
                    shadow_color[2],
                    shadow_alpha,
                ],
                character,
                0.0,
            );
        }
        push_osc_overlay_glyph_instance(
            instances,
            panel_surface,
            glyph_rect,
            text_run.color,
            character,
            osc_overlay_text_weight(text_run.role),
        );
    }
}

fn push_osc_overlay_rect_instance(
    instances: &mut Vec<OscDiagnosticsOverlayInstance>,
    panel_surface: &OscOverlaySurface,
    rect: [f32; 4],
    color: [f32; 4],
) {
    if instances.len() >= OSC_OVERLAY_MAX_INSTANCES {
        return;
    }
    let Some(clip) = osc_overlay_clip_quad_for_rect(panel_surface, rect) else {
        return;
    };
    instances.push(OscDiagnosticsOverlayInstance {
        left_clip: clip.left,
        right_clip: clip.right,
        color,
        glyph: [0.0, 0.0, 0.0, 0.0],
    });
}

fn push_osc_overlay_glyph_instance(
    instances: &mut Vec<OscDiagnosticsOverlayInstance>,
    panel_surface: &OscOverlaySurface,
    rect: [f32; 4],
    color: [f32; 4],
    character: char,
    weight: f32,
) {
    if instances.len() >= OSC_OVERLAY_MAX_INSTANCES {
        return;
    }
    let Some(clip) = osc_overlay_clip_quad_for_rect(panel_surface, rect) else {
        return;
    };
    instances.push(OscDiagnosticsOverlayInstance {
        left_clip: clip.left,
        right_clip: clip.right,
        color,
        glyph: [character as u32 as f32, 1.0, weight.clamp(0.0, 1.0), 0.0],
    });
}

fn osc_overlay_text_weight(role: rusty_xr_debug_canvas::CanvasTextRole) -> f32 {
    match role {
        rusty_xr_debug_canvas::CanvasTextRole::Title => 0.30,
        rusty_xr_debug_canvas::CanvasTextRole::Section => 0.18,
        rusty_xr_debug_canvas::CanvasTextRole::Label => 0.0,
        rusty_xr_debug_canvas::CanvasTextRole::Body
        | rusty_xr_debug_canvas::CanvasTextRole::Small => 0.0,
    }
}

unsafe fn upload_osc_overlay_instances(
    device: &ash::Device,
    resources: &OscDiagnosticsOverlayResources,
    instances: &[OscDiagnosticsOverlayInstance],
) -> Result<(), String> {
    if instances.len() > resources.instance_capacity {
        return Err(format!(
            "OSC diagnostics overlay instance count {} exceeds capacity {}",
            instances.len(),
            resources.instance_capacity
        ));
    }
    let byte_len = std::mem::size_of_val(instances) as vk::DeviceSize;
    let mapped = device
        .map_memory(
            resources.instance_memory,
            0,
            byte_len,
            vk::MemoryMapFlags::empty(),
        )
        .map_err(|error| format!("map OSC diagnostics overlay instance buffer: {error}"))?;
    ptr::copy_nonoverlapping(
        instances.as_ptr().cast::<u8>(),
        mapped.cast::<u8>(),
        byte_len as usize,
    );
    device.unmap_memory(resources.instance_memory);
    Ok(())
}

fn osc_overlay_canvas_layout() -> CanvasLayout {
    CanvasLayout {
        columns: 56,
        rows: 22,
        padding_columns: 3,
        header_rows: 3,
        key_columns: 12,
    }
}

fn diagnostic_hud_document(
    config: &RuntimeConfig,
    frame_count: u64,
    hud: DiagnosticHudUpdate,
) -> CanvasDocument {
    if config.osc_enabled {
        return osc_overlay_document(config, frame_count, hud);
    }

    CanvasDocument::new("DIAGNOSTIC HUD")
        .with_subtitle("HEADSET TEST PANEL")
        .with_section(CanvasSection::unnamed().with_badges(vec![
            CanvasBadge::new("STATUS", "VISIBLE", CanvasTone::Success),
            CanvasBadge::new(
                "PAGE",
                format!("{}/{}", hud.page_index.saturating_add(1), hud.page_count),
                CanvasTone::Accent,
            ),
            CanvasBadge::new(
                "SRC",
                diagnostic_hud_source_short_label(hud),
                CanvasTone::Muted,
            ),
        ]))
        .with_section(
            CanvasSection::new("RUNTIME")
                .with_key_value("TIER", config.camera_tier.stable_id(), CanvasTone::Text)
                .with_key_value(
                    "DEPTH",
                    config.environment_depth_mode.stable_id(),
                    CanvasTone::Text,
                )
                .with_key_value(
                    "SCALE",
                    format!("{:.2}", config.xr_render_scale),
                    CanvasTone::Text,
                )
                .with_key_value("FRAME", frame_count.to_string(), CanvasTone::Muted),
        )
        .with_section(
            CanvasSection::new("INPUTS")
                .with_key_value("ADB", "diagnosticHudCommand toggle", CanvasTone::Accent)
                .with_key_value("ADAPTERS", "controller lsl osc app", CanvasTone::Muted),
        )
        .with_footer("HUD COMMANDS SHOW HIDE TOGGLE NEXT PREVIOUS PAGE:N")
}

fn diagnostic_hud_source_short_label(hud: DiagnosticHudUpdate) -> &'static str {
    match hud.last_input_source {
        Some(rusty_xr_debug_canvas::DiagnosticHudInputSource::RuntimeConfig) => "CONFIG",
        Some(rusty_xr_debug_canvas::DiagnosticHudInputSource::AdbIntent) => "ADB",
        Some(rusty_xr_debug_canvas::DiagnosticHudInputSource::Controller) => "CTRL",
        Some(rusty_xr_debug_canvas::DiagnosticHudInputSource::Lsl) => "LSL",
        Some(rusty_xr_debug_canvas::DiagnosticHudInputSource::Osc) => "OSC",
        Some(rusty_xr_debug_canvas::DiagnosticHudInputSource::Application) => "APP",
        None => "NONE",
    }
}

fn osc_overlay_document(
    config: &RuntimeConfig,
    frame_count: u64,
    hud: DiagnosticHudUpdate,
) -> CanvasDocument {
    let snapshot = crate::osc_ingress::ingress_snapshot();
    let (status, status_tone) = if !config.osc_enabled {
        ("DISABLED", CanvasTone::Muted)
    } else if snapshot.listening {
        ("LISTENING", CanvasTone::Success)
    } else if snapshot.last_error.is_some() {
        ("ERROR", CanvasTone::Danger)
    } else {
        ("STARTING", CanvasTone::Warning)
    };
    let bind_addr = if snapshot.bind_addr.is_empty() {
        config.osc_listen_addr.as_str()
    } else {
        snapshot.bind_addr.as_str()
    };
    let local_addr = snapshot.local_addr.as_deref().unwrap_or(bind_addr);
    let max_packet_bytes = snapshot.max_packet_bytes.max(config.osc_max_packet_bytes);
    let packet_age = snapshot
        .last_received_unix_ms
        .map(|received_ms| duration_label_ms(now_unix_ms().saturating_sub(received_ms)))
        .unwrap_or_else(|| "NONE".to_string());
    let peer = snapshot.last_peer.as_deref().unwrap_or("NONE");
    let peer_tone = if snapshot.last_peer.is_some() {
        CanvasTone::Success
    } else {
        CanvasTone::Muted
    };
    let packet_summary = snapshot
        .last_packet_summary
        .as_deref()
        .unwrap_or("WAITING FOR COMPANION OSC PACKET");
    let (packet_address, packet_args, packet_types) = packet_summary_fields(packet_summary);
    let error = snapshot.last_error.as_deref().unwrap_or("NONE");
    let error_tone = if snapshot.last_error.is_some() {
        CanvasTone::Danger
    } else {
        CanvasTone::Muted
    };
    let port = udp_port_label(bind_addr);
    let packet_tone = if snapshot.packet_count > 0 {
        CanvasTone::Success
    } else {
        CanvasTone::Muted
    };
    let byte_tone = if snapshot.last_byte_len > 0 {
        CanvasTone::Accent
    } else {
        CanvasTone::Muted
    };

    CanvasDocument::new("OSC LIVE PROBE")
        .with_subtitle("DIAGNOSTIC HUD / UDP CONNECTOR")
        .with_section(CanvasSection::unnamed().with_badges(vec![
            CanvasBadge::new("STATUS", status, status_tone),
            CanvasBadge::new("PORT", port, CanvasTone::Accent),
            CanvasBadge::new("PKT", snapshot.packet_count.to_string(), packet_tone),
            CanvasBadge::new("AGE", packet_age, packet_tone),
            CanvasBadge::new(
                "SRC",
                diagnostic_hud_source_short_label(hud),
                CanvasTone::Muted,
            ),
        ]))
        .with_section(
            CanvasSection::new("ENDPOINT")
                .with_key_value("BIND", bind_addr, CanvasTone::Text)
                .with_key_value("LOCAL", local_addr, CanvasTone::Text)
                .with_key_value("PEER", peer, peer_tone),
        )
        .with_section(
            CanvasSection::new("LAST PACKET")
                .with_key_value("ADDRESS", non_empty(packet_address), packet_tone)
                .with_key_value("ARGS", non_empty(packet_args), packet_tone)
                .with_key_value("TYPES", non_empty(packet_types), CanvasTone::Muted)
                .with_key_value(
                    "BYTES",
                    format!("{} / MAX {} B", snapshot.last_byte_len, max_packet_bytes),
                    byte_tone,
                ),
        )
        .with_section(CanvasSection::new("RECEIVER").with_key_value("ERROR", error, error_tone))
        .with_footer(format!(
            "FRAME {frame_count}  SEND OSC TO QUEST LAN IP ON PORT {port}"
        ))
}

fn packet_summary_fields(summary: &str) -> (&str, &str, &str) {
    if summary.starts_with("message ") {
        (
            summary_field(summary, "address=").unwrap_or("?"),
            summary_field(summary, "args=").unwrap_or("?"),
            summary_field(summary, "types=").unwrap_or("?"),
        )
    } else {
        (summary, "", "")
    }
}

fn summary_field<'a>(summary: &'a str, key: &str) -> Option<&'a str> {
    let start = summary.find(key)? + key.len();
    let tail = &summary[start..];
    let end = tail.find(' ').unwrap_or(tail.len());
    Some(&tail[..end])
}

fn udp_port_label(addr: &str) -> &str {
    addr.rsplit_once(':')
        .map(|(_, port)| port)
        .filter(|port| !port.is_empty())
        .unwrap_or("?")
}

fn sanitize_overlay_char(character: char) -> char {
    if character == ' ' || character.is_ascii_graphic() {
        character
    } else {
        '?'
    }
}

fn non_empty(value: &str) -> &str {
    if value.is_empty() {
        "NONE"
    } else {
        value
    }
}

fn duration_label_ms(ms: u128) -> String {
    if ms < 1_000 {
        format!("{ms}MS")
    } else if ms < 60_000 {
        format!("{:.1}S", ms as f64 / 1_000.0)
    } else {
        format!("{:.1}M", ms as f64 / 60_000.0)
    }
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn osc_overlay_surface_for_projection(
    config: &RuntimeConfig,
    views: &[xr::View],
    resolution: vk::Extent2D,
) -> Option<OscOverlaySurface> {
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
    let surface_corners = inset_surface_corners(
        head_anchored_preview_surface_corners(
            tracking,
            config.camera_preview_fov_y_degrees,
            config.camera_projection_scale.max(0.05),
            aspect,
            config.camera_raw_overlay_overscan,
        )
        .ok()?,
        OSC_OVERLAY_PROJECTION_INSET_X,
        OSC_OVERLAY_PROJECTION_INSET_Y,
    );
    let left_view = views.first()?;
    let right_view = views.get(1).unwrap_or(left_view);
    Some(OscOverlaySurface {
        corners: surface_corners,
        left: osc_overlay_eye_projection(left_view)?,
        right: osc_overlay_eye_projection(right_view)?,
    })
}

fn osc_overlay_eye_projection(view: &xr::View) -> Option<OscOverlayEyeProjection> {
    let projection = OscOverlayEyeProjection {
        eye: eye_basis_from_view(view)?,
        tan_left: view.fov.angle_left.tan(),
        tan_right: view.fov.angle_right.tan(),
        tan_down: view.fov.angle_down.tan(),
        tan_up: view.fov.angle_up.tan(),
    };
    projection.is_valid().then_some(projection)
}

impl OscOverlayEyeProjection {
    fn is_valid(self) -> bool {
        self.eye.is_valid()
            && self.tan_left.is_finite()
            && self.tan_right.is_finite()
            && self.tan_down.is_finite()
            && self.tan_up.is_finite()
            && self.tan_right > self.tan_left
            && self.tan_up > self.tan_down
    }
}

fn inset_surface_corners(corners: [Vec3; 4], inset_x: f32, inset_y: f32) -> [Vec3; 4] {
    let inset_x = inset_x.clamp(0.0, 0.45);
    let inset_y = inset_y.clamp(0.0, 0.45);
    let x0 = inset_x;
    let y0 = inset_y;
    let x1 = 1.0 - inset_x;
    let y1 = 1.0 - inset_y;
    [
        surface_point(corners, x0, y0),
        surface_point(corners, x1, y0),
        surface_point(corners, x1, y1),
        surface_point(corners, x0, y1),
    ]
}

fn surface_point(corners: [Vec3; 4], u: f32, v: f32) -> Vec3 {
    let u = u.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let top = corners[0] * (1.0 - u) + corners[1] * u;
    let bottom = corners[3] * (1.0 - u) + corners[2] * u;
    top * (1.0 - v) + bottom * v
}

fn osc_overlay_clip_quad_for_rect(
    surface: &OscOverlaySurface,
    rect: [f32; 4],
) -> Option<OscOverlayClipQuad> {
    let [x0, y0, x1, y1] = clamped_rect_edges(rect)?;
    let points = [
        surface_point(surface.corners, x0, y0),
        surface_point(surface.corners, x1, y0),
        surface_point(surface.corners, x1, y1),
        surface_point(surface.corners, x0, y1),
    ];
    Some(OscOverlayClipQuad {
        left: project_points_to_eye_clip(surface.left, points)?,
        right: project_points_to_eye_clip(surface.right, points)?,
    })
}

fn clamped_rect_edges(rect: [f32; 4]) -> Option<[f32; 4]> {
    if !rect.iter().all(|value| value.is_finite()) || rect[2] <= 0.0 || rect[3] <= 0.0 {
        return None;
    }
    let x0 = rect[0].clamp(0.0, 1.0);
    let y0 = rect[1].clamp(0.0, 1.0);
    let x1 = (rect[0] + rect[2]).clamp(0.0, 1.0);
    let y1 = (rect[1] + rect[3]).clamp(0.0, 1.0);
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1, y1])
}

fn project_points_to_eye_clip(
    eye: OscOverlayEyeProjection,
    points: [Vec3; 4],
) -> Option<[[f32; 4]; 4]> {
    let mut projected = [[0.0; 4]; 4];
    for (index, point) in points.into_iter().enumerate() {
        projected[index] = project_tracking_point_to_eye_clip(eye, point)?;
    }
    Some(projected)
}

fn project_tracking_point_to_eye_clip(
    eye: OscOverlayEyeProjection,
    point: Vec3,
) -> Option<[f32; 4]> {
    if !eye.is_valid() || !point.is_finite() {
        return None;
    }

    let local = point - eye.eye.position;
    let z = local.dot(eye.eye.forward);
    if !z.is_finite() || z <= 0.0 {
        return None;
    }

    let tan_x = local.dot(eye.eye.right) / z;
    let tan_y = local.dot(eye.eye.up) / z;
    let width = eye.tan_right - eye.tan_left;
    let height = eye.tan_up - eye.tan_down;
    let uv_x = (tan_x - eye.tan_left) / width;
    let uv_y = (eye.tan_up - tan_y) / height;
    if !uv_x.is_finite() || !uv_y.is_finite() {
        return None;
    }

    Some([(uv_x * 2.0 - 1.0) * z, (uv_y * 2.0 - 1.0) * z, 0.0, z])
}

fn overlay_surface_to_scissor(surface: &OscOverlaySurface, resolution: vk::Extent2D) -> vk::Rect2D {
    let Some(left) = eye_screen_rect_for_surface(surface.corners, surface.left) else {
        return full_resolution_scissor(resolution);
    };
    let Some(right) = eye_screen_rect_for_surface(surface.corners, surface.right) else {
        return full_resolution_scissor(resolution);
    };
    screen_rect_to_scissor(union_screen_rects(left, right), resolution)
}

fn eye_screen_rect_for_surface(
    surface_corners: [Vec3; 4],
    eye: OscOverlayEyeProjection,
) -> Option<[f32; 4]> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for corner in surface_corners {
        let clip = project_tracking_point_to_eye_clip(eye, corner)?;
        let ndc_x = clip[0] / clip[3];
        let ndc_y = clip[1] / clip[3];
        let uv_x = (ndc_x + 1.0) * 0.5;
        let uv_y = (ndc_y + 1.0) * 0.5;
        if !uv_x.is_finite() || !uv_y.is_finite() {
            return None;
        }
        min_x = min_x.min(uv_x);
        min_y = min_y.min(uv_y);
        max_x = max_x.max(uv_x);
        max_y = max_y.max(uv_y);
    }

    let x0 = min_x.clamp(0.0, 1.0);
    let y0 = min_y.clamp(0.0, 1.0);
    let x1 = max_x.clamp(0.0, 1.0);
    let y1 = max_y.clamp(0.0, 1.0);
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1 - x0, y1 - y0])
}

fn union_screen_rects(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    let x0 = left[0].min(right[0]);
    let y0 = left[1].min(right[1]);
    let x1 = (left[0] + left[2]).max(right[0] + right[2]);
    let y1 = (left[1] + left[3]).max(right[1] + right[3]);
    [x0, y0, x1 - x0, y1 - y0]
}

fn screen_rect_to_scissor(rect: [f32; 4], resolution: vk::Extent2D) -> vk::Rect2D {
    let width = resolution.width.max(1) as f32;
    let height = resolution.height.max(1) as f32;
    let x0 = (rect[0].clamp(0.0, 1.0) * width).floor() as i32;
    let y0 = (rect[1].clamp(0.0, 1.0) * height).floor() as i32;
    let x1 = ((rect[0] + rect[2]).clamp(0.0, 1.0) * width).ceil() as i32;
    let y1 = ((rect[1] + rect[3]).clamp(0.0, 1.0) * height).ceil() as i32;
    vk::Rect2D {
        offset: vk::Offset2D { x: x0, y: y0 },
        extent: vk::Extent2D {
            width: (x1 - x0).max(1) as u32,
            height: (y1 - y0).max(1) as u32,
        },
    }
}

fn full_resolution_scissor(resolution: vk::Extent2D) -> vk::Rect2D {
    vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: vk::Extent2D {
            width: resolution.width.max(1),
            height: resolution.height.max(1),
        },
    }
}

unsafe fn create_osc_diagnostics_overlay_resources(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    render_pass: vk::RenderPass,
) -> Result<OscDiagnosticsOverlayResources, String> {
    let expected_font_atlas_bytes =
        (OSC_OVERLAY_FONT_ATLAS_WIDTH as usize) * (OSC_OVERLAY_FONT_ATLAS_HEIGHT as usize) * 4;
    if OSC_OVERLAY_FONT_ATLAS_BYTES.len() != expected_font_atlas_bytes {
        return Err(format!(
            "OSC diagnostics font atlas has {} bytes, expected {}",
            OSC_OVERLAY_FONT_ATLAS_BYTES.len(),
            expected_font_atlas_bytes
        ));
    }

    let descriptor_binding = [
        vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX),
        vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT),
    ];
    let descriptor_set_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_binding),
            None,
        )
        .map_err(|error| {
            format!("create OSC diagnostics overlay descriptor set layout: {error}")
        })?;
    let set_layouts = [descriptor_set_layout];
    let pipeline_layout = match device.create_pipeline_layout(
        &vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts),
        None,
    ) {
        Ok(layout) => layout,
        Err(error) => {
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(format!(
                "create OSC diagnostics overlay pipeline layout: {error}"
            ));
        }
    };
    let pipeline =
        match create_osc_diagnostics_overlay_pipeline(device, render_pass, pipeline_layout) {
            Ok(pipeline) => pipeline,
            Err(error) => {
                device.destroy_pipeline_layout(pipeline_layout, None);
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                return Err(error);
            }
        };

    let instance_capacity = OSC_OVERLAY_MAX_INSTANCES;
    let instance_buffer_size = (std::mem::size_of::<OscDiagnosticsOverlayInstance>()
        * instance_capacity) as vk::DeviceSize;
    let (instance_buffer, instance_memory) = match create_osc_overlay_storage_buffer(
        device,
        memory_properties,
        instance_buffer_size,
        "instance",
    ) {
        Ok(buffer) => buffer,
        Err(error) => {
            device.destroy_pipeline(pipeline, None);
            device.destroy_pipeline_layout(pipeline_layout, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(error);
        }
    };

    let font_atlas_buffer_size = OSC_OVERLAY_FONT_ATLAS_BYTES.len() as vk::DeviceSize;
    let (font_atlas_buffer, font_atlas_memory) = match create_osc_overlay_storage_buffer(
        device,
        memory_properties,
        font_atlas_buffer_size,
        "font atlas",
    ) {
        Ok(buffer) => buffer,
        Err(error) => {
            device.free_memory(instance_memory, None);
            device.destroy_buffer(instance_buffer, None);
            device.destroy_pipeline(pipeline, None);
            device.destroy_pipeline_layout(pipeline_layout, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(error);
        }
    };

    if let Err(error) = upload_osc_overlay_buffer_bytes(
        device,
        font_atlas_memory,
        OSC_OVERLAY_FONT_ATLAS_BYTES,
        "font atlas",
    ) {
        device.free_memory(font_atlas_memory, None);
        device.destroy_buffer(font_atlas_buffer, None);
        device.free_memory(instance_memory, None);
        device.destroy_buffer(instance_buffer, None);
        device.destroy_pipeline(pipeline, None);
        device.destroy_pipeline_layout(pipeline_layout, None);
        device.destroy_descriptor_set_layout(descriptor_set_layout, None);
        return Err(error);
    }

    let pool_sizes = [vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(2)];
    let descriptor_pool = match device.create_descriptor_pool(
        &vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(1),
        None,
    ) {
        Ok(pool) => pool,
        Err(error) => {
            device.free_memory(font_atlas_memory, None);
            device.destroy_buffer(font_atlas_buffer, None);
            device.free_memory(instance_memory, None);
            device.destroy_buffer(instance_buffer, None);
            device.destroy_pipeline(pipeline, None);
            device.destroy_pipeline_layout(pipeline_layout, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(format!(
                "create OSC diagnostics overlay descriptor pool: {error}"
            ));
        }
    };
    let descriptor_set_layouts = [descriptor_set_layout];
    let descriptor_set = match device.allocate_descriptor_sets(
        &vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts),
    ) {
        Ok(mut sets) => sets.pop().ok_or_else(|| {
            "OSC diagnostics overlay descriptor allocation returned no set".to_string()
        })?,
        Err(error) => {
            device.destroy_descriptor_pool(descriptor_pool, None);
            device.free_memory(font_atlas_memory, None);
            device.destroy_buffer(font_atlas_buffer, None);
            device.free_memory(instance_memory, None);
            device.destroy_buffer(instance_buffer, None);
            device.destroy_pipeline(pipeline, None);
            device.destroy_pipeline_layout(pipeline_layout, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(format!(
                "allocate OSC diagnostics overlay descriptor set: {error}"
            ));
        }
    };
    let instance_info = [vk::DescriptorBufferInfo::default()
        .buffer(instance_buffer)
        .offset(0)
        .range(instance_buffer_size)];
    let font_atlas_info = [vk::DescriptorBufferInfo::default()
        .buffer(font_atlas_buffer)
        .offset(0)
        .range(font_atlas_buffer_size)];
    let writes = [
        vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&instance_info),
        vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&font_atlas_info),
    ];
    device.update_descriptor_sets(&writes, &[]);

    let layout = osc_overlay_canvas_layout();
    log_info(format!(
        "Rusty XR diagnostic HUD overlay resources ready columns={} rows={} maxInstances={} projectionMode=head-anchored-stereo-surface textRenderer=sdf-font-atlas-scale-aware font=JetBrainsMono-Regular atlas={}x{} cell={}x{} inset={:.3},{:.3}",
        layout.columns,
        layout.rows,
        instance_capacity,
        OSC_OVERLAY_FONT_ATLAS_WIDTH,
        OSC_OVERLAY_FONT_ATLAS_HEIGHT,
        OSC_OVERLAY_FONT_CELL_WIDTH,
        OSC_OVERLAY_FONT_CELL_HEIGHT,
        OSC_OVERLAY_PROJECTION_INSET_X,
        OSC_OVERLAY_PROJECTION_INSET_Y
    ));

    Ok(OscDiagnosticsOverlayResources {
        pipeline_layout,
        pipeline,
        descriptor_set_layout,
        descriptor_pool,
        descriptor_set,
        instance_buffer,
        instance_memory,
        instance_capacity,
        font_atlas_buffer,
        font_atlas_memory,
    })
}

unsafe fn create_osc_overlay_storage_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    size: vk::DeviceSize,
    label: &str,
) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
    let buffer = device
        .create_buffer(
            &vk::BufferCreateInfo::default()
                .size(size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            None,
        )
        .map_err(|error| format!("create OSC diagnostics overlay {label} buffer: {error}"))?;
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
                "allocate OSC diagnostics overlay {label} memory: {error}"
            ));
        }
    };
    if let Err(error) = device.bind_buffer_memory(buffer, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_buffer(buffer, None);
        return Err(format!(
            "bind OSC diagnostics overlay {label} memory: {error}"
        ));
    }
    Ok((buffer, memory))
}

unsafe fn upload_osc_overlay_buffer_bytes(
    device: &ash::Device,
    memory: vk::DeviceMemory,
    bytes: &[u8],
    label: &str,
) -> Result<(), String> {
    let mapped = device
        .map_memory(
            memory,
            0,
            bytes.len() as vk::DeviceSize,
            vk::MemoryMapFlags::empty(),
        )
        .map_err(|error| format!("map OSC diagnostics overlay {label} buffer: {error}"))?;
    ptr::copy_nonoverlapping(bytes.as_ptr(), mapped.cast::<u8>(), bytes.len());
    device.unmap_memory(memory);
    Ok(())
}

unsafe fn create_osc_diagnostics_overlay_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
) -> Result<vk::Pipeline, String> {
    let vertex_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/osc_diagnostics_overlay.vert.spv"
    )))?;
    let fragment_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/osc_diagnostics_overlay.frag.spv"
    )))?;
    let vertex_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&vertex_words),
            None,
        )
        .map_err(|error| format!("create OSC diagnostics overlay vertex shader module: {error}"))?;
    let fragment_module = match device.create_shader_module(
        &vk::ShaderModuleCreateInfo::default().code(&fragment_words),
        None,
    ) {
        Ok(module) => module,
        Err(error) => {
            device.destroy_shader_module(vertex_module, None);
            return Err(format!(
                "create OSC diagnostics overlay fragment shader module: {error}"
            ));
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
        .map_err(|(_, error)| format!("create OSC diagnostics overlay graphics pipeline: {error}"))
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct GpuHandParticle {
    position_radius: [f32; 4],
    color_alpha: [f32; 4],
}

fn sync_openxr_hand_particle_source(
    instance: &xr::Instance,
    system: xr::SystemId,
    session: &xr::Session<xr::Vulkan>,
    existing: Option<OpenXrHandMeshParticleSource>,
    mode: HandParticleMode,
    frame_count: u64,
) -> Option<OpenXrHandMeshParticleSource> {
    if !mode.uses_openxr_hand_mesh() {
        return None;
    }
    if existing.is_some() {
        return existing;
    }
    if instance.exts().ext_hand_tracking.is_none()
        || instance.exts().fb_hand_tracking_mesh.is_none()
    {
        if frame_count == 0 || frame_count.is_multiple_of(120) {
            log_error(format!(
                "Rusty XR OpenXR hand mesh particle source unavailable extHandTracking={} fbHandTrackingMesh={}",
                instance.exts().ext_hand_tracking.is_some(),
                instance.exts().fb_hand_tracking_mesh.is_some()
            ));
        }
        return None;
    }
    match instance.supports_hand_tracking(system) {
        Ok(true) => {}
        Ok(false) => {
            log_error(
                "Rusty XR OpenXR hand mesh particle source unavailable supportsHandTracking=false",
            );
            return None;
        }
        Err(error) => {
            log_error(format!(
                "Rusty XR OpenXR hand mesh particle support query failed: {error}"
            ));
            return None;
        }
    }
    match OpenXrHandMeshParticleSource::new(instance, session) {
        Ok(source) => {
            log_info(format!(
                "Rusty XR OpenXR hand mesh particle source ready activeHands={} sampler=LiveHandMeshParticleSampler skinning=cpu-linear-blend extension=XR_FB_hand_tracking_mesh",
                source.active_hand_count()
            ));
            Some(source)
        }
        Err(error) => {
            log_error(format!(
                "Rusty XR OpenXR hand mesh particle source init failed: {error}"
            ));
            None
        }
    }
}

struct OpenXrHandMeshParticleSource {
    left: Option<OpenXrHandMeshRuntimeHand>,
    right: Option<OpenXrHandMeshRuntimeHand>,
    cross_config: MeshSurfaceCrossNeighborConfig,
    cross_neighborhood: Option<rusty_xr_particles::MeshSurfaceCrossNeighborhood>,
}

impl OpenXrHandMeshParticleSource {
    fn new(instance: &xr::Instance, session: &xr::Session<xr::Vulkan>) -> Result<Self, String> {
        let left = match OpenXrHandMeshRuntimeHand::new(
            instance,
            session,
            xr::Hand::LEFT,
            Handedness::Left,
            "left",
            52,
            ColorRgba::new(0.08, 0.9, 1.0, 0.62),
        ) {
            Ok(hand) => Some(hand),
            Err(error) => {
                log_error(format!(
                    "Rusty XR OpenXR left hand mesh particle source init failed: {error}"
                ));
                None
            }
        };
        let right = match OpenXrHandMeshRuntimeHand::new(
            instance,
            session,
            xr::Hand::RIGHT,
            Handedness::Right,
            "right",
            53,
            ColorRgba::new(1.0, 0.38, 0.84, 0.62),
        ) {
            Ok(hand) => Some(hand),
            Err(error) => {
                log_error(format!(
                    "Rusty XR OpenXR right hand mesh particle source init failed: {error}"
                ));
                None
            }
        };
        if left.is_none() && right.is_none() {
            return Err("no hand trackers produced a usable FB hand mesh".to_string());
        }
        Ok(Self {
            left,
            right,
            cross_config: MeshSurfaceCrossNeighborConfig {
                max_distance_meters: XR_HAND_MESH_PARTICLE_CROSS_NEIGHBOR_MAX_METERS,
                neighbors_per_point: XR_HAND_MESH_PARTICLE_CROSS_NEIGHBORS_PER_POINT,
            },
            cross_neighborhood: None,
        })
    }

    fn active_hand_count(&self) -> usize {
        usize::from(self.left.is_some()) + usize::from(self.right.is_some())
    }

    fn update(
        &mut self,
        reference_space: &xr::Space,
        predicted_display_time: xr::Time,
        frame_count: u64,
        views: &[xr::View],
    ) -> Vec<GpuHandParticle> {
        let left_status = self.left.as_mut().map(|hand| {
            hand.update(reference_space, predicted_display_time, frame_count)
                .unwrap_or_else(|error| {
                    if frame_count == 0 || frame_count.is_multiple_of(120) {
                        log_error(format!(
                            "Rusty XR OpenXR left hand mesh particle update failed: {error}"
                        ));
                    }
                    LiveHandMeshUpdateStatus::NoSnapshot
                })
        });
        let right_status = self.right.as_mut().map(|hand| {
            hand.update(reference_space, predicted_display_time, frame_count)
                .unwrap_or_else(|error| {
                    if frame_count == 0 || frame_count.is_multiple_of(120) {
                        log_error(format!(
                            "Rusty XR OpenXR right hand mesh particle update failed: {error}"
                        ));
                    }
                    LiveHandMeshUpdateStatus::NoSnapshot
                })
        });
        let left_renderable = hand_update_status_renders(left_status);
        let right_renderable = hand_update_status_renders(right_status);
        self.cross_neighborhood = match (&self.left, &self.right) {
            (Some(left), Some(right))
                if left_renderable
                    && right_renderable
                    && !left.sampler.samples().is_empty()
                    && !right.sampler.samples().is_empty() =>
            {
                Some(
                    left.sampler
                        .samples()
                        .cross_neighborhood_with(right.sampler.samples(), self.cross_config),
                )
            }
            _ => None,
        };

        let mut particles = Vec::with_capacity(XR_HAND_MESH_PARTICLE_COUNT_PER_HAND * 2);
        if let Some(left) = &self.left {
            if left_renderable {
                append_gpu_hand_particles(&mut particles, &left.sampler.render_particles());
            }
        }
        if let Some(right) = &self.right {
            if right_renderable {
                append_gpu_hand_particles(&mut particles, &right.sampler.render_particles());
            }
        }
        particles.truncate(XR_HAND_MESH_PARTICLE_CAPACITY as usize);

        if frame_count == 0 || frame_count.is_multiple_of(120) {
            let (first_tier_links, second_tier_links) = self.intra_hand_neighbor_link_counts();
            let cross_links = self.cross_neighbor_link_count();
            log_info(format!(
                "Rusty XR OpenXR hand mesh particles frame={} activeHands={} particles={} leftStatus={} rightStatus={} firstTierNeighborLinks={} secondTierNeighborLinks={} crossHandNeighborLinks={} crossHandMaxDistanceMeters={} crossHandNeighborsPerPoint={} skinning=cpu-linear-blend bindMesh=XR_FB_hand_tracking_mesh jointSource=xrLocateHandJointsEXT",
                frame_count,
                self.active_hand_count(),
                particles.len(),
                hand_update_status_label(left_status),
                hand_update_status_label(right_status),
                first_tier_links,
                second_tier_links,
                cross_links,
                self.cross_config.max_distance_meters,
                self.cross_config.neighbors_per_point
            ));
            if let Some(alignment) = hand_mesh_alignment_debug(
                self.left.as_ref(),
                self.right.as_ref(),
                views,
                cross_links,
            ) {
                log_info(alignment);
            }
        }

        particles
    }

    fn intra_hand_neighbor_link_counts(&self) -> (usize, usize) {
        [self.left.as_ref(), self.right.as_ref()]
            .into_iter()
            .flatten()
            .fold((0, 0), |(first_total, second_total), hand| {
                let samples = hand.sampler.samples();
                (
                    first_total
                        + samples
                            .first_tier_neighbors
                            .iter()
                            .map(Vec::len)
                            .sum::<usize>(),
                    second_total
                        + samples
                            .second_tier_neighbors
                            .iter()
                            .map(Vec::len)
                            .sum::<usize>(),
                )
            })
    }

    fn cross_neighbor_link_count(&self) -> usize {
        self.cross_neighborhood.as_ref().map_or(0, |cross| {
            cross
                .a_to_b_neighbors
                .iter()
                .chain(cross.b_to_a_neighbors.iter())
                .map(Vec::len)
                .sum()
        })
    }
}

fn hand_update_status_label(status: Option<LiveHandMeshUpdateStatus>) -> &'static str {
    match status {
        Some(LiveHandMeshUpdateStatus::NoSnapshot) => "no-snapshot",
        Some(LiveHandMeshUpdateStatus::Initialized) => "initialized",
        Some(LiveHandMeshUpdateStatus::Updated) => "updated",
        Some(LiveHandMeshUpdateStatus::ResampledTopology) => "resampled-topology",
        Some(LiveHandMeshUpdateStatus::InvalidSnapshot) => "invalid-snapshot",
        Some(LiveHandMeshUpdateStatus::InvalidSurface) => "invalid-surface",
        None => "not-created",
    }
}

fn hand_update_status_renders(status: Option<LiveHandMeshUpdateStatus>) -> bool {
    matches!(
        status,
        Some(
            LiveHandMeshUpdateStatus::Initialized
                | LiveHandMeshUpdateStatus::Updated
                | LiveHandMeshUpdateStatus::ResampledTopology
        )
    )
}

#[derive(Clone, Copy, Debug)]
struct OpenXrHandMeshSpatialDebug {
    sample_count: usize,
    center: Vec3,
    min: Vec3,
    max: Vec3,
    palm: Option<Vec3>,
    wrist: Option<Vec3>,
}

fn hand_mesh_spatial_debug_from_samples(
    samples: &rusty_xr_particles::MeshSurfaceSampleSet,
    joint_locations: &[xr::sys::HandJointLocationEXT],
) -> Option<OpenXrHandMeshSpatialDebug> {
    let first = samples.samples.first()?.position;
    let mut min = first;
    let mut max = first;
    let mut sum = Vec3::ZERO;
    for sample in &samples.samples {
        min = min.min(sample.position);
        max = max.max(sample.position);
        sum += sample.position;
    }
    let sample_count = samples.samples.len();
    Some(OpenXrHandMeshSpatialDebug {
        sample_count,
        center: sum * (1.0 / sample_count.max(1) as f32),
        min,
        max,
        palm: openxr_hand_joint_position_if_valid(joint_locations, OPENXR_HAND_JOINT_PALM_INDEX),
        wrist: openxr_hand_joint_position_if_valid(joint_locations, OPENXR_HAND_JOINT_WRIST_INDEX),
    })
}

fn openxr_hand_joint_position_if_valid(
    joint_locations: &[xr::sys::HandJointLocationEXT],
    index: usize,
) -> Option<Vec3> {
    let location = *joint_locations.get(index)?;
    openxr_hand_joint_pose_valid(location).then(|| xr_position_to_vec3(location.pose.position))
}

fn openxr_usable_hand_joint_count(joint_locations: &[xr::sys::HandJointLocationEXT]) -> usize {
    joint_locations
        .iter()
        .copied()
        .filter(|location| openxr_hand_joint_pose_valid(*location))
        .count()
}

fn hand_mesh_alignment_debug(
    left: Option<&OpenXrHandMeshRuntimeHand>,
    right: Option<&OpenXrHandMeshRuntimeHand>,
    views: &[xr::View],
    cross_links: usize,
) -> Option<String> {
    let left = left?.last_spatial_debug?;
    let right = right?.last_spatial_debug?;
    let render_view = views.first().copied().unwrap_or_else(default_xr_view);
    let head = head_pose_from_views(views).map(|(position, _)| position);
    let left_view_center = reference_to_view_position(left.center, render_view);
    let right_view_center = reference_to_view_position(right.center, render_view);
    let left_wrist_view = left
        .wrist
        .map(|position| reference_to_view_position(position, render_view));
    let right_wrist_view = right
        .wrist
        .map(|position| reference_to_view_position(position, render_view));
    let center_delta = right.center - left.center;
    let view_delta = right_view_center - left_view_center;
    Some(format!(
        "Rusty XR OpenXR hand mesh alignment samplesL={} samplesR={} centerLRef={} centerRRef={} deltaRef={} centerLView={} centerRView={} deltaView={} wristLRef={} wristRRef={} wristLView={} wristRView={} palmLRef={} palmRRef={} headRef={} boundsLRef={}..{} boundsRRef={}..{} centerDistanceMeters={:.3} crossHandNeighborLinks={}",
        left.sample_count,
        right.sample_count,
        format_vec3(left.center),
        format_vec3(right.center),
        format_vec3(center_delta),
        format_vec3(left_view_center),
        format_vec3(right_view_center),
        format_vec3(view_delta),
        format_option_vec3(left.wrist),
        format_option_vec3(right.wrist),
        format_option_vec3(left_wrist_view),
        format_option_vec3(right_wrist_view),
        format_option_vec3(left.palm),
        format_option_vec3(right.palm),
        format_option_vec3(head),
        format_vec3(left.min),
        format_vec3(left.max),
        format_vec3(right.min),
        format_vec3(right.max),
        center_delta.length(),
        cross_links
    ))
}

fn reference_to_view_position(reference_position: Vec3, view: xr::View) -> Vec3 {
    xr_orientation_to_quat(view.pose.orientation)
        .inverse()
        .rotate_vec3(reference_position - xr_position_to_vec3(view.pose.position))
}

fn format_option_vec3(value: Option<Vec3>) -> String {
    value.map(format_vec3).unwrap_or_else(|| "none".to_string())
}

fn format_vec3(value: Vec3) -> String {
    format!("({:.3},{:.3},{:.3})", value.x, value.y, value.z)
}

struct OpenXrHandMeshRuntimeHand {
    handedness: Handedness,
    label: &'static str,
    tracker: xr::HandTracker,
    bind_mesh: OpenXrFbHandMeshBindData,
    sampler: LiveHandMeshParticleSampler,
    last_spatial_debug: Option<OpenXrHandMeshSpatialDebug>,
}

impl OpenXrHandMeshRuntimeHand {
    fn new(
        instance: &xr::Instance,
        session: &xr::Session<xr::Vulkan>,
        xr_hand: xr::Hand,
        handedness: Handedness,
        label: &'static str,
        seed: u64,
        color: ColorRgba,
    ) -> Result<Self, String> {
        let tracker = session
            .create_hand_tracker(xr_hand)
            .map_err(|error| format!("create {label} hand tracker: {error}"))?;
        let bind_mesh = OpenXrFbHandMeshBindData::load(instance, &tracker, label)?;
        let sampler = LiveHandMeshParticleSampler::new(MeshSurfaceSampleConfig {
            point_count: XR_HAND_MESH_PARTICLE_COUNT_PER_HAND,
            first_tier_neighbor_count: 6,
            second_tier_neighbor_count: 12,
            seed,
        })
        .with_render_style(
            rusty_xr_particles::RenderCoordinateSpace::World,
            XR_HAND_MESH_PARTICLE_RADIUS_METERS * 2.0,
            color,
        );
        log_info(format!(
            "Rusty XR OpenXR {label} hand mesh bind data vertices={} triangles={} joints={} indices={} blendWeights=true",
            bind_mesh.vertex_positions.len(),
            bind_mesh.indices.len(),
            bind_mesh.joint_bind_poses.len(),
            bind_mesh.raw_index_count
        ));
        Ok(Self {
            handedness,
            label,
            tracker,
            bind_mesh,
            sampler,
            last_spatial_debug: None,
        })
    }

    fn update(
        &mut self,
        reference_space: &xr::Space,
        predicted_display_time: xr::Time,
        frame_count: u64,
    ) -> Result<LiveHandMeshUpdateStatus, String> {
        let Some((joint_locations, scale)) =
            locate_openxr_hand_joints(&self.tracker, reference_space, predicted_display_time)?
        else {
            self.last_spatial_debug = None;
            return Ok(LiveHandMeshUpdateStatus::NoSnapshot);
        };
        if openxr_hand_joint_position_if_valid(&joint_locations, OPENXR_HAND_JOINT_WRIST_INDEX)
            .is_none()
        {
            self.last_spatial_debug = None;
            if frame_count == 0 || frame_count.is_multiple_of(120) {
                log_info(format!(
                    "Rusty XR OpenXR {} hand mesh waiting for usable wrist pose frame={} usableJoints={} jointCount={}",
                    self.label,
                    frame_count,
                    openxr_usable_hand_joint_count(&joint_locations),
                    joint_locations.len()
                ));
            }
            return Ok(LiveHandMeshUpdateStatus::NoSnapshot);
        }
        let snapshot =
            self.bind_mesh
                .skinned_snapshot(self.handedness, frame_count, &joint_locations)?;
        let update = self.sampler.update_from_snapshot(&snapshot);
        self.last_spatial_debug =
            hand_mesh_spatial_debug_from_samples(self.sampler.samples(), &joint_locations);
        if frame_count == 0 || frame_count.is_multiple_of(120) {
            log_info(format!(
                "Rusty XR OpenXR {} hand mesh update frame={} status={} vertices={} triangles={} samples={} scale={:.4}",
                self.label,
                frame_count,
                hand_update_status_label(Some(update.status)),
                snapshot.vertices.len(),
                snapshot.indices.len(),
                update.sample_count,
                scale
            ));
        }
        Ok(update.status)
    }
}

struct OpenXrFbHandMeshBindData {
    joint_bind_poses: Vec<xr::sys::Posef>,
    vertex_positions: Vec<xr::sys::Vector3f>,
    vertex_normals: Vec<xr::sys::Vector3f>,
    vertex_blend_indices: Vec<xr::sys::Vector4sFB>,
    vertex_blend_weights: Vec<xr::sys::Vector4f>,
    indices: Vec<[u32; 3]>,
    raw_index_count: usize,
}

impl OpenXrFbHandMeshBindData {
    fn load(
        instance: &xr::Instance,
        tracker: &xr::HandTracker,
        label: &'static str,
    ) -> Result<Self, String> {
        let extension = instance
            .exts()
            .fb_hand_tracking_mesh
            .as_ref()
            .ok_or_else(|| "XR_FB_hand_tracking_mesh function table is unavailable".to_string())?;

        let mut probe = empty_openxr_hand_tracking_mesh();
        let probe_result = unsafe { (extension.get_hand_mesh)(tracker.as_raw(), &mut probe) };
        if probe_result.into_raw() < xr::sys::Result::SUCCESS.into_raw()
            && probe_result != xr::sys::Result::ERROR_SIZE_INSUFFICIENT
        {
            return Err(format!(
                "xrGetHandMeshFB({label}, probe) failed: {probe_result:?}"
            ));
        }

        let joint_count = probe.joint_count_output as usize;
        let vertex_count = probe.vertex_count_output as usize;
        let index_count = probe.index_count_output as usize;
        if joint_count == 0 || vertex_count == 0 || index_count < 3 {
            return Err(format!(
                "xrGetHandMeshFB({label}) returned empty mesh jointCount={} vertexCount={} indexCount={}",
                joint_count, vertex_count, index_count
            ));
        }
        if !index_count.is_multiple_of(3) {
            return Err(format!(
                "xrGetHandMeshFB({label}) returned non-triangle index count {index_count}"
            ));
        }

        let mut joint_bind_poses = vec![xr::sys::Posef::default(); joint_count];
        let mut joint_radii = vec![0.0_f32; joint_count];
        let mut joint_parents = vec![xr::sys::HandJointEXT::default(); joint_count];
        let mut vertex_positions = vec![xr::sys::Vector3f::default(); vertex_count];
        let mut vertex_normals = vec![xr::sys::Vector3f::default(); vertex_count];
        let mut vertex_uvs = vec![xr::sys::Vector2f::default(); vertex_count];
        let mut vertex_blend_indices = vec![xr::sys::Vector4sFB::default(); vertex_count];
        let mut vertex_blend_weights = vec![xr::sys::Vector4f::default(); vertex_count];
        let mut raw_indices = vec![0_i16; index_count];
        let mut mesh = xr::sys::HandTrackingMeshFB {
            ty: xr::sys::HandTrackingMeshFB::TYPE,
            next: ptr::null_mut(),
            joint_capacity_input: joint_count as u32,
            joint_count_output: 0,
            joint_bind_poses: joint_bind_poses.as_mut_ptr(),
            joint_radii: joint_radii.as_mut_ptr(),
            joint_parents: joint_parents.as_mut_ptr(),
            vertex_capacity_input: vertex_count as u32,
            vertex_count_output: 0,
            vertex_positions: vertex_positions.as_mut_ptr(),
            vertex_normals: vertex_normals.as_mut_ptr(),
            vertex_u_vs: vertex_uvs.as_mut_ptr(),
            vertex_blend_indices: vertex_blend_indices.as_mut_ptr(),
            vertex_blend_weights: vertex_blend_weights.as_mut_ptr(),
            index_capacity_input: index_count as u32,
            index_count_output: 0,
            indices: raw_indices.as_mut_ptr(),
        };
        let result = unsafe { (extension.get_hand_mesh)(tracker.as_raw(), &mut mesh) };
        ensure_xr_success(result, &format!("xrGetHandMeshFB({label})"))?;

        joint_bind_poses.truncate(mesh.joint_count_output as usize);
        vertex_positions.truncate(mesh.vertex_count_output as usize);
        vertex_normals.truncate(mesh.vertex_count_output as usize);
        vertex_blend_indices.truncate(mesh.vertex_count_output as usize);
        vertex_blend_weights.truncate(mesh.vertex_count_output as usize);
        raw_indices.truncate(mesh.index_count_output as usize);

        let mut indices = Vec::with_capacity(raw_indices.len() / 3);
        for triangle in raw_indices.chunks_exact(3) {
            let a = u32::try_from(triangle[0])
                .map_err(|_| format!("xrGetHandMeshFB({label}) returned negative index"))?;
            let b = u32::try_from(triangle[1])
                .map_err(|_| format!("xrGetHandMeshFB({label}) returned negative index"))?;
            let c = u32::try_from(triangle[2])
                .map_err(|_| format!("xrGetHandMeshFB({label}) returned negative index"))?;
            indices.push([a, b, c]);
        }

        Ok(Self {
            joint_bind_poses,
            vertex_positions,
            vertex_normals,
            vertex_blend_indices,
            vertex_blend_weights,
            indices,
            raw_index_count: raw_indices.len(),
        })
    }

    fn skinned_snapshot(
        &self,
        handedness: Handedness,
        version: u64,
        joint_locations: &[xr::sys::HandJointLocationEXT],
    ) -> Result<HandMeshSnapshot, String> {
        if joint_locations.len() < self.joint_bind_poses.len() {
            return Err(format!(
                "joint location count {} is smaller than bind pose count {}",
                joint_locations.len(),
                self.joint_bind_poses.len()
            ));
        }
        let mut vertices = Vec::with_capacity(self.vertex_positions.len());
        let mut normals = Vec::with_capacity(self.vertex_positions.len());
        for index in 0..self.vertex_positions.len() {
            let bind_position = xr_vector3_to_vec3(self.vertex_positions[index]);
            let blend_indices = self.vertex_blend_indices[index];
            let blend_weights = self.vertex_blend_weights[index];
            let skinned = skin_openxr_bind_point(
                bind_position,
                blend_indices,
                blend_weights,
                &self.joint_bind_poses,
                joint_locations,
            )
            .unwrap_or(bind_position);
            vertices.push(skinned);

            let bind_normal = self
                .vertex_normals
                .get(index)
                .copied()
                .map(xr_vector3_to_vec3)
                .unwrap_or(Vec3::UP);
            normals.push(
                skin_openxr_bind_vector(
                    bind_normal,
                    blend_indices,
                    blend_weights,
                    &self.joint_bind_poses,
                    joint_locations,
                )
                .unwrap_or(bind_normal)
                .normalized_or(Vec3::UP),
            );
        }
        let mut snapshot = HandMeshSnapshot::new(version, vertices, self.indices.clone())
            .with_handedness(handedness);
        snapshot.normals = normals;
        snapshot.joint_indices = self
            .vertex_blend_indices
            .iter()
            .copied()
            .map(|indices| {
                [
                    indices.x.max(0) as u16,
                    indices.y.max(0) as u16,
                    indices.z.max(0) as u16,
                    indices.w.max(0) as u16,
                ]
            })
            .collect();
        snapshot.joint_weights = self
            .vertex_blend_weights
            .iter()
            .copied()
            .map(|weights| [weights.x, weights.y, weights.z, weights.w])
            .collect();
        Ok(snapshot)
    }
}

fn empty_openxr_hand_tracking_mesh() -> xr::sys::HandTrackingMeshFB {
    xr::sys::HandTrackingMeshFB {
        ty: xr::sys::HandTrackingMeshFB::TYPE,
        next: ptr::null_mut(),
        joint_capacity_input: 0,
        joint_count_output: 0,
        joint_bind_poses: ptr::null_mut(),
        joint_radii: ptr::null_mut(),
        joint_parents: ptr::null_mut(),
        vertex_capacity_input: 0,
        vertex_count_output: 0,
        vertex_positions: ptr::null_mut(),
        vertex_normals: ptr::null_mut(),
        vertex_u_vs: ptr::null_mut(),
        vertex_blend_indices: ptr::null_mut(),
        vertex_blend_weights: ptr::null_mut(),
        index_capacity_input: 0,
        index_count_output: 0,
        indices: ptr::null_mut(),
    }
}

fn locate_openxr_hand_joints(
    tracker: &xr::HandTracker,
    reference_space: &xr::Space,
    predicted_display_time: xr::Time,
) -> Result<Option<(Vec<xr::sys::HandJointLocationEXT>, f32)>, String> {
    reference_space
        .locate_hand_joints(tracker, predicted_display_time)
        .map(|locations| locations.map(|joint_locations| (joint_locations.to_vec(), 1.0)))
        .map_err(|error| format!("xrLocateHandJointsEXT: {error}"))
}

fn skin_openxr_bind_point(
    bind_point: Vec3,
    blend_indices: xr::sys::Vector4sFB,
    blend_weights: xr::sys::Vector4f,
    joint_bind_poses: &[xr::sys::Posef],
    joint_locations: &[xr::sys::HandJointLocationEXT],
) -> Option<Vec3> {
    let mut skinned = Vec3::ZERO;
    let mut total_weight = 0.0_f32;
    for (joint_index, weight) in openxr_blend_pairs(blend_indices, blend_weights) {
        if weight <= 0.0
            || joint_index >= joint_bind_poses.len()
            || joint_index >= joint_locations.len()
        {
            continue;
        }
        let joint_location = joint_locations[joint_index];
        if !openxr_hand_joint_pose_valid(joint_location) {
            continue;
        }
        let bind_local =
            inverse_transform_openxr_pose_point(joint_bind_poses[joint_index], bind_point);
        let skinned_point = transform_openxr_pose_point(joint_location.pose, bind_local);
        skinned += skinned_point * weight;
        total_weight += weight;
    }
    (total_weight > 0.0).then_some(skinned * (1.0 / total_weight))
}

fn skin_openxr_bind_vector(
    bind_vector: Vec3,
    blend_indices: xr::sys::Vector4sFB,
    blend_weights: xr::sys::Vector4f,
    joint_bind_poses: &[xr::sys::Posef],
    joint_locations: &[xr::sys::HandJointLocationEXT],
) -> Option<Vec3> {
    let mut skinned = Vec3::ZERO;
    let mut total_weight = 0.0_f32;
    for (joint_index, weight) in openxr_blend_pairs(blend_indices, blend_weights) {
        if weight <= 0.0
            || joint_index >= joint_bind_poses.len()
            || joint_index >= joint_locations.len()
        {
            continue;
        }
        let joint_location = joint_locations[joint_index];
        if !openxr_hand_joint_pose_valid(joint_location) {
            continue;
        }
        let bind_orientation = xr_orientation_to_quat(joint_bind_poses[joint_index].orientation);
        let joint_orientation = xr_orientation_to_quat(joint_location.pose.orientation);
        let skinned_vector =
            (joint_orientation * bind_orientation.inverse()).rotate_vec3(bind_vector);
        skinned += skinned_vector * weight;
        total_weight += weight;
    }
    (total_weight > 0.0).then_some(skinned * (1.0 / total_weight))
}

fn openxr_blend_pairs(
    indices: xr::sys::Vector4sFB,
    weights: xr::sys::Vector4f,
) -> [(usize, f32); 4] {
    [
        (indices.x.max(0) as usize, weights.x),
        (indices.y.max(0) as usize, weights.y),
        (indices.z.max(0) as usize, weights.z),
        (indices.w.max(0) as usize, weights.w),
    ]
}

fn openxr_hand_joint_pose_valid(location: xr::sys::HandJointLocationEXT) -> bool {
    let flags = location.location_flags;
    let position_usable = flags.intersects(
        xr::sys::SpaceLocationFlags::POSITION_VALID | xr::sys::SpaceLocationFlags::POSITION_TRACKED,
    );
    let orientation_usable = flags.intersects(
        xr::sys::SpaceLocationFlags::ORIENTATION_VALID
            | xr::sys::SpaceLocationFlags::ORIENTATION_TRACKED,
    );
    position_usable && orientation_usable
}

fn transform_openxr_pose_point(pose: xr::sys::Posef, point: Vec3) -> Vec3 {
    xr_position_to_vec3(pose.position) + xr_orientation_to_quat(pose.orientation).rotate_vec3(point)
}

fn inverse_transform_openxr_pose_point(pose: xr::sys::Posef, point: Vec3) -> Vec3 {
    xr_orientation_to_quat(pose.orientation)
        .inverse()
        .rotate_vec3(point - xr_position_to_vec3(pose.position))
}

fn head_pose_from_views(views: &[xr::View]) -> Option<(Vec3, Quat)> {
    let left = views.first()?;
    let right = views.get(1).unwrap_or(left);
    let left_position = xr_position_to_vec3(left.pose.position);
    let right_position = xr_position_to_vec3(right.pose.position);
    let head_position = (left_position + right_position) * 0.5;
    let head_orientation = xr_orientation_to_quat(left.pose.orientation);
    Some((head_position, head_orientation))
}

fn xr_position_to_vec3(position: xr::sys::Vector3f) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

fn xr_vector3_to_vec3(vector: xr::sys::Vector3f) -> Vec3 {
    Vec3::new(vector.x, vector.y, vector.z)
}

fn xr_orientation_to_quat(orientation: xr::sys::Quaternionf) -> Quat {
    Quat::new(orientation.x, orientation.y, orientation.z, orientation.w)
        .normalized_or(Quat::IDENTITY)
}

fn append_gpu_hand_particles(output: &mut Vec<GpuHandParticle>, particles: &[ParticleRender]) {
    output.extend(particles.iter().map(|particle| GpuHandParticle {
        position_radius: [
            particle.position.x,
            particle.position.y,
            particle.position.z,
            (particle.size_meters * 0.5).max(0.001),
        ],
        color_alpha: [
            particle.color.r,
            particle.color.g,
            particle.color.b,
            particle.color.a,
        ],
    }));
}

struct EnvironmentDepthVisualizer {
    render_pass: vk::RenderPass,
    resources: Option<EnvironmentDepthVisualizerResources>,
    cached_depth_frames: Vec<EnvironmentDepthVisualFrame>,
    particle_write_cursor: u32,
    last_particle_capture_time_ns: Option<i64>,
    particles_initialized: bool,
    last_particle_scene_map: Option<bool>,
    last_draw_failure_frame: Option<u64>,
}

impl EnvironmentDepthVisualizer {
    fn new(render_pass: vk::RenderPass) -> Self {
        Self {
            render_pass,
            resources: None,
            cached_depth_frames: Vec::new(),
            particle_write_cursor: 0,
            last_particle_capture_time_ns: None,
            particles_initialized: false,
            last_particle_scene_map: None,
            last_draw_failure_frame: None,
        }
    }

    unsafe fn prepare(
        &mut self,
        device: &ash::Device,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        depth_image_handles: &[u64],
        depth_width: u32,
        depth_height: u32,
    ) -> Result<bool, String> {
        if depth_image_handles.is_empty() {
            self.destroy(device);
            return Ok(false);
        }
        if self
            .resources
            .as_ref()
            .map(|resources| resources.matches(depth_image_handles, depth_width, depth_height))
            .unwrap_or(false)
        {
            return Ok(true);
        }

        self.destroy(device);
        let resources = create_environment_depth_visualizer_resources(
            device,
            memory_properties,
            self.render_pass,
            depth_image_handles,
            depth_width,
            depth_height,
        )?;
        self.resources = Some(resources);
        Ok(true)
    }

    unsafe fn record_particle_update(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: EnvironmentDepthVisualFrame,
        mode: EnvironmentDepthMode,
    ) {
        if !mode.particle_overlay() {
            return;
        }
        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let scene_particle_map = mode.scene_particle_map();
        let particle_map_changed = self.last_particle_scene_map != Some(scene_particle_map);
        if !self.particles_initialized || particle_map_changed {
            device.cmd_fill_buffer(
                cmd,
                resources.particle_buffer,
                0,
                resources.particle_buffer_size,
                0,
            );
            let clear_barrier = [vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE)
                .buffer(resources.particle_buffer)
                .offset(0)
                .size(resources.particle_buffer_size)];
            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::VERTEX_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &clear_barrier,
                &[],
            );
            self.particles_initialized = true;
            self.last_particle_scene_map = Some(scene_particle_map);
            self.particle_write_cursor = 0;
            self.last_particle_capture_time_ns = None;
        }
        if self.last_particle_capture_time_ns == Some(frame.capture_time_ns) {
            return;
        }
        let Some(descriptor_set) = resources
            .descriptor_sets
            .get(frame.swapchain_index as usize)
            .copied()
        else {
            return;
        };

        let write_base = self.particle_write_cursor;
        let particle_writes =
            environment_depth_particle_samples_per_frame(frame.depth_width, frame.depth_height);
        let particle_update_marker = if scene_particle_map {
            frame.frame_count as f32
        } else {
            write_base as f32
        };
        let push = EnvironmentDepthVisualizationPush {
            params: [
                XR_ENVIRONMENT_DEPTH_VISUAL_MAX_METERS,
                frame.near_z,
                if frame.far_z.is_finite() {
                    frame.far_z
                } else {
                    -1.0
                },
                if frame.far_z.is_finite() { 0.0 } else { 1.0 },
            ],
            transform: [
                XR_ENVIRONMENT_DEPTH_VISUAL_TEXTURE_TRANSFORM_FLAGS as f32,
                particle_update_marker,
                if scene_particle_map { 1.0 } else { 0.0 },
                XR_ENVIRONMENT_DEPTH_PARTICLE_DISCONTINUITY_METERS,
            ],
            left_fov_tangents: frame.left_fov_tangents,
            right_fov_tangents: frame.right_fov_tangents,
            left_render_fov_tangents: frame.left_render_fov_tangents,
            right_render_fov_tangents: frame.right_render_fov_tangents,
            left_position: frame.left_position,
            right_position: frame.right_position,
            left_orientation: frame.left_orientation,
            right_orientation: frame.right_orientation,
            left_render_position: frame.left_render_position,
            right_render_position: frame.right_render_position,
            left_render_orientation: frame.left_render_orientation,
            right_render_orientation: frame.right_render_orientation,
        };
        device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            resources.particle_update_pipeline,
        );
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            resources.pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );
        let push_bytes = std::slice::from_raw_parts(
            (&push as *const EnvironmentDepthVisualizationPush).cast::<u8>(),
            std::mem::size_of::<EnvironmentDepthVisualizationPush>(),
        );
        device.cmd_push_constants(
            cmd,
            resources.pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            push_bytes,
        );
        let grid_width = environment_depth_particle_grid_width(frame.depth_width);
        let grid_height = environment_depth_particle_grid_height(frame.depth_height);
        device.cmd_dispatch(
            cmd,
            grid_width.div_ceil(8).max(1),
            grid_height.div_ceil(8).max(1),
            XR_ENVIRONMENT_DEPTH_PARTICLE_SOURCE_VIEW_COUNT,
        );
        let update_barrier = [vk::BufferMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .buffer(resources.particle_buffer)
            .offset(0)
            .size(resources.particle_buffer_size)];
        device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::COMPUTE_SHADER,
            vk::PipelineStageFlags::VERTEX_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &update_barrier,
            &[],
        );

        if !scene_particle_map {
            self.particle_write_cursor = (self.particle_write_cursor + particle_writes)
                % XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY;
        }
        self.last_particle_capture_time_ns = Some(frame.capture_time_ns);

        if frame.frame_count == 0 || frame.frame_count % 120 == 0 {
            if scene_particle_map {
                log_info(format!(
                    "Rusty XR environment depth scene particle map update frame={} captureTimeNs={} candidateSamples={} particleCapacity={} sampleStridePixels={} cellMeters={} hashProbeCount={} staleFadeStartFrames={} staleRetireFrames={} confidenceSource=depth-discontinuity confidenceThresholdMeters={} mergePolicy=spatial-cell-confidence-weighted invalidSamplePolicy=preserve-existing-cells activeCorrectionPolicy=visible-free-space-ray-clear activeCorrectionConfidenceThreshold={} activeCorrectionStepMeters={} activeCorrectionMaxSteps={} activeCorrectionSurfaceKeepMeters={} occlusionPolicy=preserve-behind-current-depth depthColorMaxMeters={}",
                    frame.frame_count,
                    frame.capture_time_ns,
                    particle_writes,
                    XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY,
                    XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS,
                    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_CELL_METERS,
                    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_PROBE_COUNT,
                    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_FADE_START_FRAMES,
                    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_RETIRE_FRAMES,
                    XR_ENVIRONMENT_DEPTH_PARTICLE_DISCONTINUITY_METERS,
                    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_CONFIDENCE,
                    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_STEP_METERS,
                    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_MAX_STEPS,
                    XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_SURFACE_KEEP_METERS,
                    XR_ENVIRONMENT_DEPTH_MESH_DISTANCE_GRADIENT_MAX_METERS
                ));
            } else {
                log_info(format!(
                    "Rusty XR environment depth particle update frame={} captureTimeNs={} writeBase={} samplesPerFrame={} particleCapacity={} sampleStridePixels={} confidenceSource=depth-discontinuity confidenceThresholdMeters={} depthColorMaxMeters={}",
                    frame.frame_count,
                    frame.capture_time_ns,
                    write_base,
                    particle_writes,
                    XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY,
                    XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS,
                    XR_ENVIRONMENT_DEPTH_PARTICLE_DISCONTINUITY_METERS,
                    XR_ENVIRONMENT_DEPTH_MESH_DISTANCE_GRADIENT_MAX_METERS
                ));
            }
        }
    }

    unsafe fn record_draw(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        resolution: vk::Extent2D,
        frame: EnvironmentDepthVisualFrame,
        mode: EnvironmentDepthMode,
    ) {
        let mesh_overlay = mode.mesh_overlay();
        let particle_overlay = mode.particle_overlay();
        let scene_particle_map = mode.scene_particle_map();
        if mesh_overlay {
            self.remember_depth_frame(frame);
        }
        let draw_frames = if mesh_overlay {
            self.mesh_draw_frames(frame)
        } else {
            vec![(frame, if particle_overlay { 1.0 } else { 0.0 })]
        };
        let Some(resources) = self.resources.as_ref() else {
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

        device.cmd_set_viewport(cmd, 0, &viewport);
        device.cmd_set_scissor(cmd, 0, &scissor);
        let pipeline = if particle_overlay {
            resources.particle_pipeline
        } else if mesh_overlay {
            resources.mesh_pipeline
        } else {
            resources.visualization_pipeline
        };
        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
        let mut drawn_frames = 0_usize;
        let mut last_vertex_count = 0_u32;
        for (draw_frame, history_alpha) in &draw_frames {
            let Some(descriptor_set) = resources
                .descriptor_sets
                .get(draw_frame.swapchain_index as usize)
                .copied()
            else {
                if self.last_draw_failure_frame != Some(frame.frame_count) {
                    log_error(format!(
                        "Rusty XR environment depth visualizer missing descriptor for swapchainIndex={} descriptorCount={}",
                        draw_frame.swapchain_index,
                        resources.descriptor_sets.len()
                    ));
                    self.last_draw_failure_frame = Some(frame.frame_count);
                }
                continue;
            };
            let push = EnvironmentDepthVisualizationPush {
                params: [
                    XR_ENVIRONMENT_DEPTH_VISUAL_MAX_METERS,
                    draw_frame.near_z,
                    if draw_frame.far_z.is_finite() {
                        draw_frame.far_z
                    } else {
                        -1.0
                    },
                    if draw_frame.far_z.is_finite() {
                        0.0
                    } else {
                        1.0
                    },
                ],
                transform: [
                    XR_ENVIRONMENT_DEPTH_VISUAL_TEXTURE_TRANSFORM_FLAGS as f32,
                    if scene_particle_map {
                        frame.frame_count as f32
                    } else {
                        *history_alpha
                    },
                    if scene_particle_map {
                        1.0
                    } else {
                        XR_ENVIRONMENT_DEPTH_MESH_CELL_METERS
                    },
                    XR_ENVIRONMENT_DEPTH_MESH_DISCONTINUITY_METERS,
                ],
                left_fov_tangents: draw_frame.left_fov_tangents,
                right_fov_tangents: draw_frame.right_fov_tangents,
                left_render_fov_tangents: draw_frame.left_render_fov_tangents,
                right_render_fov_tangents: draw_frame.right_render_fov_tangents,
                left_position: draw_frame.left_position,
                right_position: draw_frame.right_position,
                left_orientation: draw_frame.left_orientation,
                right_orientation: draw_frame.right_orientation,
                left_render_position: draw_frame.left_render_position,
                right_render_position: draw_frame.right_render_position,
                left_render_orientation: draw_frame.left_render_orientation,
                right_render_orientation: draw_frame.right_render_orientation,
            };
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                resources.pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
            let push_bytes = std::slice::from_raw_parts(
                (&push as *const EnvironmentDepthVisualizationPush).cast::<u8>(),
                std::mem::size_of::<EnvironmentDepthVisualizationPush>(),
            );
            device.cmd_push_constants(
                cmd,
                resources.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_bytes,
            );
            let vertex_count = if particle_overlay {
                XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY.saturating_mul(6)
            } else if mesh_overlay {
                environment_depth_mesh_vertex_count(draw_frame.depth_width, draw_frame.depth_height)
            } else {
                3
            };
            device.cmd_draw(cmd, vertex_count, 1, 0, 0);
            drawn_frames = drawn_frames.saturating_add(1);
            last_vertex_count = vertex_count;
        }
        if frame.frame_count == 0 || frame.frame_count % 120 == 0 {
            log_info(format!(
                "Rusty XR environment depth visualizer draw frame={} swapchainIndex={} captureTimeNs={} renderTarget={}x{} depthTexture={}x{} depthTextureFormat=VK_FORMAT_D16_UNORM depthTextureLayers={} grayscale=linear-d16-meters-infinity-white depthVisualMaxMeters={} depthVisualTextureTransform={} depthPoseSource=view-space-composed projectionYConvention=vulkan-positive-viewport-y-flipped-in-shader depthMeshOverlay={} depthMeshDistanceColorMaxMeters={} depthMeshCellMeters={} depthMeshDiscontinuityMeters={} depthMeshProjection=local-space-depth-surface depthMeshRasterization={} depthMeshGridStridePixels={} depthMeshVertexCount={} depthMeshHistoryFramesDrawn={} depthMeshHistoryMaxAgeMs={} passthroughVisible={} confidenceSource=none confidencePayload=false confidenceStatus=not-exposed-by-XR_META_environment_depth",
                frame.frame_count,
                frame.swapchain_index,
                frame.capture_time_ns,
                resolution.width,
                resolution.height,
                frame.depth_width,
                frame.depth_height,
                VIEW_COUNT,
                XR_ENVIRONMENT_DEPTH_VISUAL_MAX_METERS,
                XR_ENVIRONMENT_DEPTH_VISUAL_TEXTURE_TRANSFORM_LABEL,
                mesh_overlay,
                XR_ENVIRONMENT_DEPTH_MESH_DISTANCE_GRADIENT_MAX_METERS,
                XR_ENVIRONMENT_DEPTH_MESH_CELL_METERS,
                XR_ENVIRONMENT_DEPTH_MESH_DISCONTINUITY_METERS,
                if particle_overlay {
                    if scene_particle_map {
                        "scene-owned-spatial-particle-map"
                    } else {
                        "retained-local-space-metric-billboard-particles"
                    }
                } else if mesh_overlay {
                    "world-space-generated-grid"
                } else {
                    "fullscreen-depth-visualizer"
                },
                XR_ENVIRONMENT_DEPTH_MESH_GRID_STRIDE_PIXELS,
                last_vertex_count,
                drawn_frames,
                XR_ENVIRONMENT_DEPTH_MESH_HISTORY_MAX_AGE_NS / 1_000_000,
                mesh_overlay || particle_overlay
            ));
        }
        if mesh_overlay && (frame.frame_count == 0 || frame.frame_count % 120 == 0) {
            log_info(format!(
                "Rusty XR environment depth mesh overlay draw frame={} swapchainIndex={} cellMeters={} discontinuityMeters={} distanceColorMaxMeters={} distanceColorSource=environment-depth-meters captureTimeNs={} renderTarget={}x{} depthTexture={}x{} depthTextureFormat=VK_FORMAT_D16_UNORM depthTextureLayers={} depthVisualTextureTransform={} depthPoseSource=view-space-composed projectionYConvention=vulkan-positive-viewport-y-flipped-in-shader projection=local-space-depth-surface rasterization=world-space-generated-grid gridStridePixels={} generatedVertexCount={} historyFramesDrawn={} historyMaxAgeMs={} dominantSurfaceGrid=true screenUvGrid=false passthroughVisible=true",
                frame.frame_count,
                frame.swapchain_index,
                XR_ENVIRONMENT_DEPTH_MESH_CELL_METERS,
                XR_ENVIRONMENT_DEPTH_MESH_DISCONTINUITY_METERS,
                XR_ENVIRONMENT_DEPTH_MESH_DISTANCE_GRADIENT_MAX_METERS,
                frame.capture_time_ns,
                resolution.width,
                resolution.height,
                frame.depth_width,
                frame.depth_height,
                VIEW_COUNT,
                XR_ENVIRONMENT_DEPTH_VISUAL_TEXTURE_TRANSFORM_LABEL,
                XR_ENVIRONMENT_DEPTH_MESH_GRID_STRIDE_PIXELS,
                last_vertex_count,
                drawn_frames,
                XR_ENVIRONMENT_DEPTH_MESH_HISTORY_MAX_AGE_NS / 1_000_000
            ));
        }
        if scene_particle_map && (frame.frame_count == 0 || frame.frame_count % 120 == 0) {
            log_info(format!(
                "Rusty XR environment depth scene particle map draw frame={} swapchainIndex={} distanceColorMaxMeters={} distanceColorSource=environment-depth-meters captureTimeNs={} renderTarget={}x{} depthTexture={}x{} depthTextureFormat=VK_FORMAT_D16_UNORM depthTextureLayers={} projection=local-space-scene-particle-map rasterization=metric-billboard-particles depthPoseSource=view-space-composed projectionYConvention=vulkan-positive-viewport-y-flipped-in-shader particleCapacity={} particleVertexCount={} sampleStridePixels={} particleHalfSizeMeters={}..{} particleMask=default-disc particleOpacity=alpha-clipped-opaque cellMeters={} hashProbeCount={} staleFadeStartFrames={} staleRetireFrames={} confidenceSource=depth-discontinuity confidenceThresholdMeters={} mapPolicy=spatial-hash-local-cells invalidSamplePolicy=preserve-existing-cells activeCorrectionPolicy=visible-free-space-ray-clear activeCorrectionConfidenceThreshold={} activeCorrectionStepMeters={} activeCorrectionMaxSteps={} activeCorrectionSurfaceKeepMeters={} occlusionPolicy=preserve-behind-current-depth passthroughVisible=true",
                frame.frame_count,
                frame.swapchain_index,
                XR_ENVIRONMENT_DEPTH_MESH_DISTANCE_GRADIENT_MAX_METERS,
                frame.capture_time_ns,
                resolution.width,
                resolution.height,
                frame.depth_width,
                frame.depth_height,
                VIEW_COUNT,
                XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY,
                last_vertex_count,
                XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS,
                XR_ENVIRONMENT_DEPTH_PARTICLE_HALF_SIZE_MIN_METERS,
                XR_ENVIRONMENT_DEPTH_PARTICLE_HALF_SIZE_MAX_METERS,
                XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_CELL_METERS,
                XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_PROBE_COUNT,
                XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_FADE_START_FRAMES,
                XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_RETIRE_FRAMES,
                XR_ENVIRONMENT_DEPTH_PARTICLE_DISCONTINUITY_METERS,
                XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_CONFIDENCE,
                XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_STEP_METERS,
                XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_MAX_STEPS,
                XR_ENVIRONMENT_DEPTH_SCENE_PARTICLE_ACTIVE_CORRECTION_SURFACE_KEEP_METERS
            ));
        } else if particle_overlay && (frame.frame_count == 0 || frame.frame_count % 120 == 0) {
            log_info(format!(
                "Rusty XR environment depth particle overlay draw frame={} swapchainIndex={} distanceColorMaxMeters={} distanceColorSource=environment-depth-meters captureTimeNs={} renderTarget={}x{} depthTexture={}x{} depthTextureFormat=VK_FORMAT_D16_UNORM depthTextureLayers={} projection=local-space-retained-particles rasterization=metric-billboard-particles depthPoseSource=view-space-composed projectionYConvention=vulkan-positive-viewport-y-flipped-in-shader particleCapacity={} particleVertexCount={} sampleStridePixels={} particleHalfSizeMeters={}..{} particleMask=default-disc particleOpacity=alpha-clipped-opaque confidenceSource=depth-discontinuity confidenceThresholdMeters={} passthroughVisible=true",
                frame.frame_count,
                frame.swapchain_index,
                XR_ENVIRONMENT_DEPTH_MESH_DISTANCE_GRADIENT_MAX_METERS,
                frame.capture_time_ns,
                resolution.width,
                resolution.height,
                frame.depth_width,
                frame.depth_height,
                VIEW_COUNT,
                XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY,
                last_vertex_count,
                XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS,
                XR_ENVIRONMENT_DEPTH_PARTICLE_HALF_SIZE_MIN_METERS,
                XR_ENVIRONMENT_DEPTH_PARTICLE_HALF_SIZE_MAX_METERS,
                XR_ENVIRONMENT_DEPTH_PARTICLE_DISCONTINUITY_METERS
            ));
        }
    }

    fn remember_depth_frame(&mut self, frame: EnvironmentDepthVisualFrame) {
        if let Some(existing) = self
            .cached_depth_frames
            .iter_mut()
            .find(|cached| cached.swapchain_index == frame.swapchain_index)
        {
            *existing = frame;
        } else {
            self.cached_depth_frames.push(frame);
        }
        self.cached_depth_frames
            .sort_by_key(|cached| cached.capture_time_ns);
        while self.cached_depth_frames.len() > XR_ENVIRONMENT_DEPTH_MESH_HISTORY_FRAMES {
            self.cached_depth_frames.remove(0);
        }
    }

    fn mesh_draw_frames(
        &self,
        current_frame: EnvironmentDepthVisualFrame,
    ) -> Vec<(EnvironmentDepthVisualFrame, f32)> {
        let mut frames = self
            .cached_depth_frames
            .iter()
            .copied()
            .filter_map(|source_frame| {
                let age_ns = current_frame
                    .capture_time_ns
                    .saturating_sub(source_frame.capture_time_ns);
                if age_ns < 0 || age_ns > XR_ENVIRONMENT_DEPTH_MESH_HISTORY_MAX_AGE_NS {
                    return None;
                }
                let age_t = (age_ns as f32 / XR_ENVIRONMENT_DEPTH_MESH_HISTORY_MAX_AGE_NS as f32)
                    .clamp(0.0, 1.0);
                let alpha = if source_frame.capture_time_ns == current_frame.capture_time_ns {
                    1.0
                } else {
                    (1.0 - age_t).max(XR_ENVIRONMENT_DEPTH_MESH_HISTORY_MIN_ALPHA)
                };
                Some((
                    environment_depth_frame_with_current_render_view(source_frame, current_frame),
                    alpha,
                ))
            })
            .collect::<Vec<_>>();
        if frames.is_empty() {
            frames.push((current_frame, 1.0));
        }
        frames.sort_by_key(|(draw_frame, _)| draw_frame.capture_time_ns);
        frames
    }

    unsafe fn destroy(&mut self, device: &ash::Device) {
        if let Some(resources) = self.resources.take() {
            resources.destroy(device);
        }
        self.cached_depth_frames.clear();
        self.particle_write_cursor = 0;
        self.last_particle_capture_time_ns = None;
        self.particles_initialized = false;
        self.last_particle_scene_map = None;
    }
}

struct EnvironmentDepthVisualizerResources {
    image_handles: Vec<u64>,
    depth_width: u32,
    depth_height: u32,
    image_views: Vec<vk::ImageView>,
    sampler: vk::Sampler,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    pipeline_layout: vk::PipelineLayout,
    visualization_pipeline: vk::Pipeline,
    mesh_pipeline: vk::Pipeline,
    particle_update_pipeline: vk::Pipeline,
    particle_pipeline: vk::Pipeline,
    particle_buffer: vk::Buffer,
    particle_memory: vk::DeviceMemory,
    particle_buffer_size: vk::DeviceSize,
}

impl EnvironmentDepthVisualizerResources {
    fn matches(&self, depth_image_handles: &[u64], depth_width: u32, depth_height: u32) -> bool {
        self.image_handles == depth_image_handles
            && self.depth_width == depth_width
            && self.depth_height == depth_height
    }

    unsafe fn destroy(self, device: &ash::Device) {
        device.destroy_buffer(self.particle_buffer, None);
        device.free_memory(self.particle_memory, None);
        device.destroy_pipeline(self.particle_pipeline, None);
        device.destroy_pipeline(self.particle_update_pipeline, None);
        device.destroy_pipeline(self.mesh_pipeline, None);
        device.destroy_pipeline(self.visualization_pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_descriptor_pool(self.descriptor_pool, None);
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        for image_view in self.image_views {
            device.destroy_image_view(image_view, None);
        }
        device.destroy_sampler(self.sampler, None);
    }
}

fn environment_depth_mesh_vertex_count(depth_width: u32, depth_height: u32) -> u32 {
    let grid_width = (depth_width / XR_ENVIRONMENT_DEPTH_MESH_GRID_STRIDE_PIXELS).max(2);
    let grid_height = (depth_height / XR_ENVIRONMENT_DEPTH_MESH_GRID_STRIDE_PIXELS).max(2);
    grid_width
        .saturating_sub(1)
        .saturating_mul(grid_height.saturating_sub(1))
        .saturating_mul(6)
}

fn environment_depth_particle_grid_width(depth_width: u32) -> u32 {
    (depth_width / XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS).max(1)
}

fn environment_depth_particle_grid_height(depth_height: u32) -> u32 {
    (depth_height / XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS).max(1)
}

fn environment_depth_particle_samples_per_frame(depth_width: u32, depth_height: u32) -> u32 {
    environment_depth_particle_grid_width(depth_width)
        .saturating_mul(environment_depth_particle_grid_height(depth_height))
        .saturating_mul(XR_ENVIRONMENT_DEPTH_PARTICLE_SOURCE_VIEW_COUNT)
        .min(XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY)
}

fn environment_depth_frame_with_current_render_view(
    mut source_frame: EnvironmentDepthVisualFrame,
    current_frame: EnvironmentDepthVisualFrame,
) -> EnvironmentDepthVisualFrame {
    source_frame.frame_count = current_frame.frame_count;
    source_frame.left_render_fov_tangents = current_frame.left_render_fov_tangents;
    source_frame.right_render_fov_tangents = current_frame.right_render_fov_tangents;
    source_frame.left_render_position = current_frame.left_render_position;
    source_frame.right_render_position = current_frame.right_render_position;
    source_frame.left_render_orientation = current_frame.left_render_orientation;
    source_frame.right_render_orientation = current_frame.right_render_orientation;
    source_frame
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EnvironmentDepthVisualizationPush {
    params: [f32; 4],
    transform: [f32; 4],
    left_fov_tangents: [f32; 4],
    right_fov_tangents: [f32; 4],
    left_render_fov_tangents: [f32; 4],
    right_render_fov_tangents: [f32; 4],
    left_position: [f32; 4],
    right_position: [f32; 4],
    left_orientation: [f32; 4],
    right_orientation: [f32; 4],
    left_render_position: [f32; 4],
    right_render_position: [f32; 4],
    left_render_orientation: [f32; 4],
    right_render_orientation: [f32; 4],
}

struct HandMeshParticleRenderer {
    render_pass: vk::RenderPass,
    resources: Option<HandMeshParticleResources>,
    last_draw_failure_frame: Option<u64>,
}

struct HandParticleDrawContext<'a> {
    device: &'a ash::Device,
    memory_properties: &'a vk::PhysicalDeviceMemoryProperties,
    cmd: vk::CommandBuffer,
    resolution: vk::Extent2D,
    views: &'a [xr::View],
    frame_count: u64,
}

impl HandMeshParticleRenderer {
    fn new(render_pass: vk::RenderPass) -> Self {
        Self {
            render_pass,
            resources: None,
            last_draw_failure_frame: None,
        }
    }

    unsafe fn record_draw(
        &mut self,
        context: HandParticleDrawContext<'_>,
        mode: HandParticleMode,
        particles: &[GpuHandParticle],
    ) {
        if !mode.enabled() || particles.is_empty() {
            return;
        }
        if self.resources.is_none() {
            match create_hand_mesh_particle_resources(
                context.device,
                context.memory_properties,
                self.render_pass,
            ) {
                Ok(resources) => {
                    log_info(format!(
                        "Rusty XR hand mesh particle resources particleCapacity={} particleBufferBytes={} mode={}",
                        XR_HAND_MESH_PARTICLE_CAPACITY,
                        resources.particle_buffer_size,
                        mode.stable_id()
                    ));
                    self.resources = Some(resources);
                }
                Err(error) => {
                    if self.last_draw_failure_frame != Some(context.frame_count) {
                        log_error(format!(
                            "Rusty XR hand mesh particle renderer init failed: {error}"
                        ));
                        self.last_draw_failure_frame = Some(context.frame_count);
                    }
                    return;
                }
            }
        }
        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let particle_count = particles.len().min(XR_HAND_MESH_PARTICLE_CAPACITY as usize);
        if let Err(error) = upload_hand_mesh_particles(
            context.device,
            resources.particle_memory,
            &particles[..particle_count],
        ) {
            if self.last_draw_failure_frame != Some(context.frame_count) {
                log_error(format!(
                    "Rusty XR hand mesh particle upload failed: {error}"
                ));
                self.last_draw_failure_frame = Some(context.frame_count);
            }
            return;
        }

        let viewport = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: context.resolution.width as f32,
            height: context.resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissor = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: context.resolution,
        }];
        context.device.cmd_set_viewport(context.cmd, 0, &viewport);
        context.device.cmd_set_scissor(context.cmd, 0, &scissor);
        context.device.cmd_bind_pipeline(
            context.cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline,
        );
        context.device.cmd_bind_descriptor_sets(
            context.cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_layout,
            0,
            &[resources.descriptor_set],
            &[],
        );
        let push = hand_particle_push_from_views(context.views);
        let push_bytes = std::slice::from_raw_parts(
            (&push as *const EnvironmentDepthVisualizationPush).cast::<u8>(),
            std::mem::size_of::<EnvironmentDepthVisualizationPush>(),
        );
        context.device.cmd_push_constants(
            context.cmd,
            resources.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            push_bytes,
        );
        let vertex_count = (particle_count as u32).saturating_mul(6);
        context.device.cmd_draw(context.cmd, vertex_count, 1, 0, 0);

        if context.frame_count.is_multiple_of(120) {
            let projection = match mode {
                HandParticleMode::Off => "off",
                HandParticleMode::Meta => "openxr-reference-space-skinned-fb-hand-mesh",
            };
            log_info(format!(
                "Rusty XR hand mesh particle draw frame={} mode={} particles={} vertexCount={} projection={} sampler=LiveHandMeshParticleSampler passthroughVisible={}",
                context.frame_count,
                mode.stable_id(),
                particle_count,
                vertex_count,
                projection,
                true
            ));
        }
    }

    unsafe fn destroy(&mut self, device: &ash::Device) {
        if let Some(resources) = self.resources.take() {
            resources.destroy(device);
        }
    }
}

struct HandMeshParticleResources {
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    particle_buffer: vk::Buffer,
    particle_memory: vk::DeviceMemory,
    particle_buffer_size: vk::DeviceSize,
}

impl HandMeshParticleResources {
    unsafe fn destroy(self, device: &ash::Device) {
        device.destroy_buffer(self.particle_buffer, None);
        device.free_memory(self.particle_memory, None);
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_descriptor_pool(self.descriptor_pool, None);
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
    }
}

fn hand_particle_push_from_views(views: &[xr::View]) -> EnvironmentDepthVisualizationPush {
    let left = views.first().copied().unwrap_or_else(default_xr_view);
    let right = views.get(1).copied().unwrap_or(left);
    EnvironmentDepthVisualizationPush {
        params: [0.0, 0.02, 100.0, 0.0],
        transform: [0.0; 4],
        left_fov_tangents: fov_tangents(left.fov),
        right_fov_tangents: fov_tangents(right.fov),
        left_render_fov_tangents: fov_tangents(left.fov),
        right_render_fov_tangents: fov_tangents(right.fov),
        left_position: pose_position(left.pose),
        right_position: pose_position(right.pose),
        left_orientation: pose_orientation(left.pose),
        right_orientation: pose_orientation(right.pose),
        left_render_position: pose_position(left.pose),
        right_render_position: pose_position(right.pose),
        left_render_orientation: pose_orientation(left.pose),
        right_render_orientation: pose_orientation(right.pose),
    }
}

fn default_xr_view() -> xr::View {
    xr::View {
        pose: xr::sys::Posef {
            orientation: xr::sys::Quaternionf {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            position: xr::sys::Vector3f {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
        fov: xr::sys::Fovf {
            angle_left: -0.75,
            angle_right: 0.75,
            angle_up: 0.75,
            angle_down: -0.75,
        },
    }
}

fn create_app_reference_space(
    session: &xr::Session<xr::Vulkan>,
    hand_particle_mode: HandParticleMode,
) -> Result<(xr::Space, &'static str), String> {
    let (primary_type, primary_label, fallback_type, fallback_label) =
        if hand_particle_mode.uses_openxr_hand_mesh() {
            (
                xr::ReferenceSpaceType::STAGE,
                "STAGE",
                xr::ReferenceSpaceType::LOCAL,
                "LOCAL",
            )
        } else {
            (
                xr::ReferenceSpaceType::LOCAL,
                "LOCAL",
                xr::ReferenceSpaceType::STAGE,
                "STAGE",
            )
        };
    match session.create_reference_space(primary_type, xr::Posef::IDENTITY) {
        Ok(space) => Ok((space, primary_label)),
        Err(primary_error) => {
            log_info(format!(
                "Rusty XR OpenXR reference space {primary_label} unavailable for handParticleMode={}, trying {fallback_label}: {primary_error}",
                hand_particle_mode.stable_id()
            ));
            session
                .create_reference_space(fallback_type, xr::Posef::IDENTITY)
                .map(|space| (space, fallback_label))
                .map_err(|fallback_error| {
                    format!(
                        "create OpenXR reference space: {primary_label} failed with {primary_error}; {fallback_label} failed with {fallback_error}"
                    )
                })
        }
    }
}

fn create_view_reference_space(session: &xr::Session<xr::Vulkan>) -> Result<xr::Space, String> {
    session
        .create_reference_space(xr::ReferenceSpaceType::VIEW, xr::Posef::IDENTITY)
        .map_err(|error| format!("create OpenXR VIEW reference space: {error}"))
}

fn space_location_pose_valid(location: xr::SpaceLocation) -> bool {
    location
        .location_flags
        .contains(xr::sys::SpaceLocationFlags::ORIENTATION_VALID)
        && location
            .location_flags
            .contains(xr::sys::SpaceLocationFlags::POSITION_VALID)
}

fn compose_head_space_views(
    reference_from_head: xr::Posef,
    head_space_views: &[xr::View],
) -> Vec<xr::View> {
    head_space_views
        .iter()
        .copied()
        .map(|view| xr::View {
            pose: multiply_openxr_pose(reference_from_head, view.pose),
            fov: view.fov,
        })
        .collect()
}

fn multiply_openxr_pose(a: xr::Posef, b: xr::Posef) -> xr::Posef {
    let rotated_b_position = rotate_openxr_vec3(a.orientation, b.position);
    xr::Posef {
        orientation: normalize_openxr_quat(multiply_openxr_quat(a.orientation, b.orientation)),
        position: xr::Vector3f {
            x: a.position.x + rotated_b_position.x,
            y: a.position.y + rotated_b_position.y,
            z: a.position.z + rotated_b_position.z,
        },
    }
}

fn multiply_openxr_quat(a: xr::Quaternionf, b: xr::Quaternionf) -> xr::Quaternionf {
    xr::Quaternionf {
        x: (a.w * b.x) + (a.x * b.w) + (a.y * b.z) - (a.z * b.y),
        y: (a.w * b.y) - (a.x * b.z) + (a.y * b.w) + (a.z * b.x),
        z: (a.w * b.z) + (a.x * b.y) - (a.y * b.x) + (a.z * b.w),
        w: (a.w * b.w) - (a.x * b.x) - (a.y * b.y) - (a.z * b.z),
    }
}

fn normalize_openxr_quat(q: xr::Quaternionf) -> xr::Quaternionf {
    let len_sq = (q.x * q.x) + (q.y * q.y) + (q.z * q.z) + (q.w * q.w);
    if !len_sq.is_finite() || len_sq <= f32::EPSILON {
        return xr::Quaternionf::IDENTITY;
    }
    let inv_len = len_sq.sqrt().recip();
    xr::Quaternionf {
        x: q.x * inv_len,
        y: q.y * inv_len,
        z: q.z * inv_len,
        w: q.w * inv_len,
    }
}

fn rotate_openxr_vec3(q: xr::Quaternionf, v: xr::Vector3f) -> xr::Vector3f {
    let qv = [q.x, q.y, q.z];
    let value = [v.x, v.y, v.z];
    let inner = add3(cross3(qv, value), scale3(value, q.w));
    let rotated = add3(value, scale3(cross3(qv, inner), 2.0));
    xr::Vector3f {
        x: rotated[0],
        y: rotated[1],
        z: rotated[2],
    }
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(v: [f32; 3], scale: f32) -> [f32; 3] {
    [v[0] * scale, v[1] * scale, v[2] * scale]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        (a[1] * b[2]) - (a[2] * b[1]),
        (a[2] * b[0]) - (a[0] * b[2]),
        (a[0] * b[1]) - (a[1] * b[0]),
    ]
}

unsafe fn create_hand_mesh_particle_resources(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    render_pass: vk::RenderPass,
) -> Result<HandMeshParticleResources, String> {
    let descriptor_binding = [vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX)];
    let descriptor_set_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_binding),
            None,
        )
        .map_err(|error| format!("create hand mesh particle descriptor set layout: {error}"))?;
    let descriptor_pool_size = [vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(1)];
    let descriptor_pool = match device.create_descriptor_pool(
        &vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_pool_size)
            .max_sets(1),
        None,
    ) {
        Ok(pool) => pool,
        Err(error) => {
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(format!(
                "create hand mesh particle descriptor pool: {error}"
            ));
        }
    };
    let set_layouts = [descriptor_set_layout];
    let descriptor_set = match device.allocate_descriptor_sets(
        &vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts),
    ) {
        Ok(mut sets) => sets.remove(0),
        Err(error) => {
            device.destroy_descriptor_pool(descriptor_pool, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(format!(
                "allocate hand mesh particle descriptor set: {error}"
            ));
        }
    };
    let push_ranges = [vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(std::mem::size_of::<EnvironmentDepthVisualizationPush>() as u32)];
    let pipeline_layout = match device.create_pipeline_layout(
        &vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_ranges),
        None,
    ) {
        Ok(layout) => layout,
        Err(error) => {
            device.destroy_descriptor_pool(descriptor_pool, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(format!(
                "create hand mesh particle pipeline layout: {error}"
            ));
        }
    };
    let pipeline = match create_hand_mesh_particle_pipeline(device, render_pass, pipeline_layout) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            device.destroy_pipeline_layout(pipeline_layout, None);
            device.destroy_descriptor_pool(descriptor_pool, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            return Err(error);
        }
    };
    let (particle_buffer, particle_memory, particle_buffer_size) =
        match create_hand_mesh_particle_buffer(device, memory_properties) {
            Ok(buffer) => buffer,
            Err(error) => {
                device.destroy_pipeline(pipeline, None);
                device.destroy_pipeline_layout(pipeline_layout, None);
                device.destroy_descriptor_pool(descriptor_pool, None);
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                return Err(error);
            }
        };
    let particle_info = [vk::DescriptorBufferInfo::default()
        .buffer(particle_buffer)
        .offset(0)
        .range(particle_buffer_size)];
    let writes = [vk::WriteDescriptorSet::default()
        .dst_set(descriptor_set)
        .dst_binding(0)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .buffer_info(&particle_info)];
    device.update_descriptor_sets(&writes, &[]);

    Ok(HandMeshParticleResources {
        descriptor_set_layout,
        descriptor_pool,
        descriptor_set,
        pipeline_layout,
        pipeline,
        particle_buffer,
        particle_memory,
        particle_buffer_size,
    })
}

unsafe fn create_hand_mesh_particle_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::DeviceSize), String> {
    let size = (XR_HAND_MESH_PARTICLE_CAPACITY as vk::DeviceSize)
        * std::mem::size_of::<GpuHandParticle>() as vk::DeviceSize;
    let buffer = device
        .create_buffer(
            &vk::BufferCreateInfo::default()
                .size(size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            None,
        )
        .map_err(|error| format!("create hand mesh particle buffer: {error}"))?;
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
            return Err(format!("allocate hand mesh particle memory: {error}"));
        }
    };
    if let Err(error) = device.bind_buffer_memory(buffer, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_buffer(buffer, None);
        return Err(format!("bind hand mesh particle memory: {error}"));
    }
    Ok((buffer, memory, size))
}

unsafe fn upload_hand_mesh_particles(
    device: &ash::Device,
    memory: vk::DeviceMemory,
    particles: &[GpuHandParticle],
) -> Result<(), String> {
    let byte_len = std::mem::size_of_val(particles) as vk::DeviceSize;
    let mapped = device
        .map_memory(memory, 0, byte_len, vk::MemoryMapFlags::empty())
        .map_err(|error| format!("map hand mesh particle memory: {error}"))?;
    ptr::copy_nonoverlapping(
        particles.as_ptr().cast::<u8>(),
        mapped.cast::<u8>(),
        byte_len as usize,
    );
    device.unmap_memory(memory);
    Ok(())
}

unsafe fn create_hand_mesh_particle_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
) -> Result<vk::Pipeline, String> {
    let vertex_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/hand_mesh_particles.vert.spv"
    )))?;
    let fragment_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/hand_mesh_particles.frag.spv"
    )))?;
    let vertex_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&vertex_words),
            None,
        )
        .map_err(|error| format!("create hand mesh particles vertex shader module: {error}"))?;
    let fragment_module = match device.create_shader_module(
        &vk::ShaderModuleCreateInfo::default().code(&fragment_words),
        None,
    ) {
        Ok(module) => module,
        Err(error) => {
            device.destroy_shader_module(vertex_module, None);
            return Err(format!(
                "create hand mesh particles fragment shader module: {error}"
            ));
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
        .src_color_blend_factor(vk::BlendFactor::ONE)
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
        .map_err(|(_, error)| format!("create hand mesh particles graphics pipeline: {error}"))
}

unsafe fn create_environment_depth_visualizer_resources(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    render_pass: vk::RenderPass,
    depth_image_handles: &[u64],
    depth_width: u32,
    depth_height: u32,
) -> Result<EnvironmentDepthVisualizerResources, String> {
    let sampler = device
        .create_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::NEAREST)
                .min_filter(vk::Filter::NEAREST)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK),
            None,
        )
        .map_err(|error| format!("create environment depth sampler: {error}"))?;

    let descriptor_binding = [
        vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(
                vk::ShaderStageFlags::VERTEX
                    | vk::ShaderStageFlags::FRAGMENT
                    | vk::ShaderStageFlags::COMPUTE,
            ),
        vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::COMPUTE),
    ];
    let descriptor_set_layout = match device.create_descriptor_set_layout(
        &vk::DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_binding),
        None,
    ) {
        Ok(layout) => layout,
        Err(error) => {
            device.destroy_sampler(sampler, None);
            return Err(format!(
                "create environment depth descriptor set layout: {error}"
            ));
        }
    };

    let descriptor_count = depth_image_handles.len() as u32;
    let pool_sizes = [
        vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(descriptor_count),
        vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(descriptor_count),
    ];
    let descriptor_pool = match device.create_descriptor_pool(
        &vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(descriptor_count),
        None,
    ) {
        Ok(pool) => pool,
        Err(error) => {
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            device.destroy_sampler(sampler, None);
            return Err(format!("create environment depth descriptor pool: {error}"));
        }
    };

    let push_ranges = [vk::PushConstantRange::default()
        .stage_flags(
            vk::ShaderStageFlags::VERTEX
                | vk::ShaderStageFlags::FRAGMENT
                | vk::ShaderStageFlags::COMPUTE,
        )
        .offset(0)
        .size(std::mem::size_of::<EnvironmentDepthVisualizationPush>() as u32)];
    let set_layouts = [descriptor_set_layout];
    let pipeline_layout = match device.create_pipeline_layout(
        &vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_ranges),
        None,
    ) {
        Ok(layout) => layout,
        Err(error) => {
            device.destroy_descriptor_pool(descriptor_pool, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            device.destroy_sampler(sampler, None);
            return Err(format!("create environment depth pipeline layout: {error}"));
        }
    };

    let visualization_pipeline =
        match create_environment_depth_visualization_pipeline(device, render_pass, pipeline_layout)
        {
            Ok(pipeline) => pipeline,
            Err(error) => {
                device.destroy_pipeline_layout(pipeline_layout, None);
                device.destroy_descriptor_pool(descriptor_pool, None);
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                device.destroy_sampler(sampler, None);
                return Err(error);
            }
        };
    let mesh_pipeline =
        match create_environment_depth_mesh_pipeline(device, render_pass, pipeline_layout) {
            Ok(pipeline) => pipeline,
            Err(error) => {
                device.destroy_pipeline(visualization_pipeline, None);
                device.destroy_pipeline_layout(pipeline_layout, None);
                device.destroy_descriptor_pool(descriptor_pool, None);
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                device.destroy_sampler(sampler, None);
                return Err(error);
            }
        };
    let particle_update_pipeline =
        match create_environment_depth_particle_update_pipeline(device, pipeline_layout) {
            Ok(pipeline) => pipeline,
            Err(error) => {
                device.destroy_pipeline(mesh_pipeline, None);
                device.destroy_pipeline(visualization_pipeline, None);
                device.destroy_pipeline_layout(pipeline_layout, None);
                device.destroy_descriptor_pool(descriptor_pool, None);
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                device.destroy_sampler(sampler, None);
                return Err(error);
            }
        };
    let particle_pipeline =
        match create_environment_depth_particle_pipeline(device, render_pass, pipeline_layout) {
            Ok(pipeline) => pipeline,
            Err(error) => {
                device.destroy_pipeline(particle_update_pipeline, None);
                device.destroy_pipeline(mesh_pipeline, None);
                device.destroy_pipeline(visualization_pipeline, None);
                device.destroy_pipeline_layout(pipeline_layout, None);
                device.destroy_descriptor_pool(descriptor_pool, None);
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                device.destroy_sampler(sampler, None);
                return Err(error);
            }
        };
    let (particle_buffer, particle_memory, particle_buffer_size) =
        match create_environment_depth_particle_buffer(device, memory_properties) {
            Ok(buffer) => buffer,
            Err(error) => {
                device.destroy_pipeline(particle_pipeline, None);
                device.destroy_pipeline(particle_update_pipeline, None);
                device.destroy_pipeline(mesh_pipeline, None);
                device.destroy_pipeline(visualization_pipeline, None);
                device.destroy_pipeline_layout(pipeline_layout, None);
                device.destroy_descriptor_pool(descriptor_pool, None);
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                device.destroy_sampler(sampler, None);
                return Err(error);
            }
        };

    let mut image_views = Vec::with_capacity(depth_image_handles.len());
    for (index, image_handle) in depth_image_handles.iter().copied().enumerate() {
        let image = vk::Image::from_raw(image_handle);
        match device.create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
                .format(XR_ENVIRONMENT_DEPTH_FORMAT)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: VIEW_COUNT,
                }),
            None,
        ) {
            Ok(view) => image_views.push(view),
            Err(error) => {
                for view in image_views {
                    device.destroy_image_view(view, None);
                }
                device.destroy_buffer(particle_buffer, None);
                device.free_memory(particle_memory, None);
                device.destroy_pipeline(particle_pipeline, None);
                device.destroy_pipeline(particle_update_pipeline, None);
                device.destroy_pipeline(mesh_pipeline, None);
                device.destroy_pipeline(visualization_pipeline, None);
                device.destroy_pipeline_layout(pipeline_layout, None);
                device.destroy_descriptor_pool(descriptor_pool, None);
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                device.destroy_sampler(sampler, None);
                return Err(format!(
                    "create environment depth image view index={index}: {error}"
                ));
            }
        }
    }

    let descriptor_set_layouts = vec![descriptor_set_layout; depth_image_handles.len()];
    let descriptor_sets = match device.allocate_descriptor_sets(
        &vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts),
    ) {
        Ok(sets) => sets,
        Err(error) => {
            for view in image_views {
                device.destroy_image_view(view, None);
            }
            device.destroy_buffer(particle_buffer, None);
            device.free_memory(particle_memory, None);
            device.destroy_pipeline(particle_pipeline, None);
            device.destroy_pipeline(particle_update_pipeline, None);
            device.destroy_pipeline(mesh_pipeline, None);
            device.destroy_pipeline(visualization_pipeline, None);
            device.destroy_pipeline_layout(pipeline_layout, None);
            device.destroy_descriptor_pool(descriptor_pool, None);
            device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            device.destroy_sampler(sampler, None);
            return Err(format!(
                "allocate environment depth descriptor sets: {error}"
            ));
        }
    };

    for (descriptor_set, image_view) in descriptor_sets.iter().copied().zip(image_views.iter()) {
        let image_info = [vk::DescriptorImageInfo::default()
            .sampler(sampler)
            .image_view(*image_view)
            .image_layout(XR_ENVIRONMENT_DEPTH_DESCRIPTOR_LAYOUT)];
        let particle_info = [vk::DescriptorBufferInfo::default()
            .buffer(particle_buffer)
            .offset(0)
            .range(particle_buffer_size)];
        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&particle_info),
        ];
        device.update_descriptor_sets(&writes, &[]);
    }

    log_info(format!(
        "Rusty XR environment depth visualizer resources images={} size={}x{} format=VK_FORMAT_D16_UNORM imageViewType=TYPE_2D_ARRAY layers={} descriptorLayout={:?} visualMaxMeters={} meshRasterization=world-space-generated-grid meshGridStridePixels={} meshVertexCount={} particleRasterization=retained-local-space-metric-billboard-particles particleCapacity={} particleSampleStridePixels={} particleBufferBytes={}",
        depth_image_handles.len(),
        depth_width,
        depth_height,
        VIEW_COUNT,
        XR_ENVIRONMENT_DEPTH_DESCRIPTOR_LAYOUT,
        XR_ENVIRONMENT_DEPTH_VISUAL_MAX_METERS,
        XR_ENVIRONMENT_DEPTH_MESH_GRID_STRIDE_PIXELS,
        environment_depth_mesh_vertex_count(depth_width, depth_height),
        XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY,
        XR_ENVIRONMENT_DEPTH_PARTICLE_SAMPLE_STRIDE_PIXELS,
        particle_buffer_size
    ));

    Ok(EnvironmentDepthVisualizerResources {
        image_handles: depth_image_handles.to_vec(),
        depth_width,
        depth_height,
        image_views,
        sampler,
        descriptor_set_layout,
        descriptor_pool,
        descriptor_sets,
        pipeline_layout,
        visualization_pipeline,
        mesh_pipeline,
        particle_update_pipeline,
        particle_pipeline,
        particle_buffer,
        particle_memory,
        particle_buffer_size,
    })
}

unsafe fn create_environment_depth_particle_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::DeviceSize), String> {
    const PARTICLE_BYTES: vk::DeviceSize = 32;
    let size = (XR_ENVIRONMENT_DEPTH_PARTICLE_CAPACITY as vk::DeviceSize) * PARTICLE_BYTES;
    let buffer = device
        .create_buffer(
            &vk::BufferCreateInfo::default()
                .size(size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            None,
        )
        .map_err(|error| format!("create environment depth particle buffer: {error}"))?;
    let requirements = device.get_buffer_memory_requirements(buffer);
    let memory_type_index = match find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
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
                "allocate environment depth particle memory: {error}"
            ));
        }
    };
    if let Err(error) = device.bind_buffer_memory(buffer, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_buffer(buffer, None);
        return Err(format!("bind environment depth particle memory: {error}"));
    }
    Ok((buffer, memory, size))
}

unsafe fn create_environment_depth_visualization_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
) -> Result<vk::Pipeline, String> {
    let vertex_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/camera_projection.vert.spv"
    )))?;
    let fragment_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/environment_depth_visualization.frag.spv"
    )))?;
    let vertex_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&vertex_words),
            None,
        )
        .map_err(|error| format!("create environment depth vertex shader module: {error}"))?;
    let fragment_module = match device.create_shader_module(
        &vk::ShaderModuleCreateInfo::default().code(&fragment_words),
        None,
    ) {
        Ok(module) => module,
        Err(error) => {
            device.destroy_shader_module(vertex_module, None);
            return Err(format!(
                "create environment depth fragment shader module: {error}"
            ));
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
        .blend_enable(false)
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
        .map_err(|(_, error)| format!("create environment depth graphics pipeline: {error}"))
}

unsafe fn create_environment_depth_particle_update_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
) -> Result<vk::Pipeline, String> {
    let compute_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/environment_depth_particle_update.comp.spv"
    )))?;
    let compute_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&compute_words),
            None,
        )
        .map_err(|error| {
            format!("create environment depth particle update shader module: {error}")
        })?;
    let entry = CString::new("main").expect("static shader entry point is valid");
    let stage = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(compute_module)
        .name(&entry);
    let create_info = [vk::ComputePipelineCreateInfo::default()
        .stage(stage)
        .layout(pipeline_layout)];
    let pipeline_result =
        device.create_compute_pipelines(vk::PipelineCache::null(), &create_info, None);
    device.destroy_shader_module(compute_module, None);
    pipeline_result
        .map(|mut pipelines| pipelines.remove(0))
        .map_err(|(_, error)| {
            format!("create environment depth particle update compute pipeline: {error}")
        })
}

unsafe fn create_environment_depth_particle_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
) -> Result<vk::Pipeline, String> {
    let vertex_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/environment_depth_particles.vert.spv"
    )))?;
    let fragment_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/environment_depth_particles.frag.spv"
    )))?;
    let vertex_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&vertex_words),
            None,
        )
        .map_err(|error| {
            format!("create environment depth particles vertex shader module: {error}")
        })?;
    let fragment_module = match device.create_shader_module(
        &vk::ShaderModuleCreateInfo::default().code(&fragment_words),
        None,
    ) {
        Ok(module) => module,
        Err(error) => {
            device.destroy_shader_module(vertex_module, None);
            return Err(format!(
                "create environment depth particles fragment shader module: {error}"
            ));
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
        .src_color_blend_factor(vk::BlendFactor::ONE)
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
        .map_err(|(_, error)| {
            format!("create environment depth particles graphics pipeline: {error}")
        })
}

unsafe fn create_environment_depth_mesh_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
) -> Result<vk::Pipeline, String> {
    let vertex_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/environment_depth_mesh.vert.spv"
    )))?;
    let fragment_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/environment_depth_mesh.frag.spv"
    )))?;
    let vertex_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&vertex_words),
            None,
        )
        .map_err(|error| format!("create environment depth mesh vertex shader module: {error}"))?;
    let fragment_module = match device.create_shader_module(
        &vk::ShaderModuleCreateInfo::default().code(&fragment_words),
        None,
    ) {
        Ok(module) => module,
        Err(error) => {
            device.destroy_shader_module(vertex_module, None);
            return Err(format!(
                "create environment depth mesh fragment shader module: {error}"
            ));
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
        .blend_enable(false)
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
        .map_err(|(_, error)| format!("create environment depth mesh graphics pipeline: {error}"))
}
