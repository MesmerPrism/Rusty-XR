use super::projection_geometry::StereoProjectionSources;
pub use super::projection_geometry::{
    broker_full_frame_projection_plan_from_xr_views, broker_physical_projection_plan_from_xr_views,
    broker_synthetic_projection_plan_from_xr_views, BrokerProjectionSource, StereoProjectionPlan,
    XrDisplayEyeView, XrDisplayViews, XrProjectionContract,
};
use crate::acamera_sys::*;
use crate::emit_marker_line;
use std::cmp::Ordering as CmpOrdering;
use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

static CAMERA_PROBE_STARTED: AtomicBool = AtomicBool::new(false);
static STEREO_PROJECTION_PLAN: Mutex<Option<StereoProjectionPlan>> = Mutex::new(None);
static STEREO_PROJECTION_SOURCES: Mutex<Option<StereoProjectionSources>> = Mutex::new(None);
static XR_VIEW_PROJECTION_MARKER_EMITTED: AtomicBool = AtomicBool::new(false);

const READER_MAX_IMAGES: i32 = 3;
const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(5);
const PREFERRED_DIMENSION: u32 = 1280;
const MAX_CAPTURE_DIMENSION: u32 = 1920;

pub fn latest_stereo_projection_plan() -> Option<StereoProjectionPlan> {
    STEREO_PROJECTION_PLAN
        .lock()
        .ok()
        .and_then(|plan| plan.clone())
}

pub fn update_stereo_projection_from_xr_views(views: XrDisplayViews) -> bool {
    let Some(sources) = STEREO_PROJECTION_SOURCES
        .lock()
        .ok()
        .and_then(|sources| sources.clone())
    else {
        return false;
    };
    let Some(plan) = StereoProjectionPlan::from_sources_and_xr_views(&sources, views) else {
        return false;
    };
    if let Ok(mut current) = STEREO_PROJECTION_PLAN.lock() {
        *current = Some(plan.clone());
    }
    if !XR_VIEW_PROJECTION_MARKER_EMITTED.swap(true, Ordering::AcqRel) {
        emit_stereo_projection_marker(&format!(
            "phase=xr-view-projection status=ok runtimeXrViewStateReady=true projectionMappingReady={} alignedProjection=false poseSource={} sourceEyeMapping={} coordinateChain={} displayLeftCameraId={} displayRightCameraId={} projectionHomographyReady={} {} leftSurfaceToScreenH={} rightSurfaceToScreenH={} leftScreenToCameraH={} rightScreenToCameraH={} leftScreenToSurfaceH={} rightScreenToSurfaceH={} projectionUvCorrection=runtime_openxr_view_screen_to_camera_homography fallbackReason={}",
            plan.projection_homography_ready,
            plan.pose_source,
            plan.source_eye_mapping,
            plan.coordinate_chain,
            marker_token(&plan.left_camera_id),
            marker_token(&plan.right_camera_id),
            plan.projection_homography_ready,
            openxr_contract_marker_fields(plan.openxr_contract),
            homography_token(plan.left_surface_to_screen_h),
            homography_token(plan.right_surface_to_screen_h),
            homography_token(plan.left_screen_to_camera_h),
            homography_token(plan.right_screen_to_camera_h),
            homography_token(plan.left_screen_to_surface_h),
            homography_token(plan.right_screen_to_surface_h),
            marker_token(plan.fallback_reason)
        ));
    }
    true
}

pub fn start_camera_probe_once() {
    if CAMERA_PROBE_STARTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    if let Err(error) = std::thread::Builder::new()
        .name("rusty-xr-camera2-probe".to_string())
        .spawn(run_camera_probe_thread)
    {
        emit_acquisition_marker(&format!(
            "phase=thread-start status=error errorKind=thread_spawn_failed message={}",
            marker_token(&error.to_string())
        ));
    }
}

fn run_camera_probe_thread() {
    let started_at = Instant::now();
    emit_acquisition_marker(
        "phase=start status=started acquisition=bounded-camera2-private-probe import=none",
    );

    let result = unsafe { run_camera_probe(started_at) };
    if let Err(error) = result {
        emit_acquisition_marker(&format!(
            "phase=complete status=error errorKind={} elapsedMs={}",
            marker_token(&error),
            started_at.elapsed().as_millis()
        ));
    }
}

unsafe fn run_camera_probe(started_at: Instant) -> Result<(), String> {
    let manager = ACameraManager_create();
    if manager.is_null() {
        return Err("camera_manager_create_failed".to_string());
    }

    let result = run_camera_probe_with_manager(manager, started_at);
    ACameraManager_delete(manager);
    result
}

unsafe fn run_camera_probe_with_manager(
    manager: *mut ACameraManager,
    started_at: Instant,
) -> Result<(), String> {
    let sources = enumerate_camera_sources(manager)?;
    let private_source_count = sources
        .iter()
        .filter(|source| !source.private_sizes.is_empty())
        .count();
    let selected = select_camera_source(&sources);
    let stereo_sources = select_stereo_projection_sources(&sources);
    if let Ok(mut stored_sources) = STEREO_PROJECTION_SOURCES.lock() {
        *stored_sources = stereo_sources.clone();
    }
    let stereo_plan = stereo_sources
        .as_ref()
        .map(StereoProjectionPlan::from_sources);
    if let Ok(mut plan) = STEREO_PROJECTION_PLAN.lock() {
        *plan = stereo_plan.clone();
    }

    emit_metadata_marker(&metadata_marker_line(
        &sources,
        private_source_count,
        selected,
        stereo_plan.as_ref(),
    ));
    emit_stereo_projection_marker(&stereo_projection_metadata_line(
        &sources,
        stereo_plan.as_ref(),
    ));

    let Some(selected_index) = selected else {
        return Err("no_private_camera_source".to_string());
    };
    let selected_source = &sources[selected_index];
    let Some((width, height)) = select_capture_size(selected_source) else {
        return Err("selected_source_missing_private_size".to_string());
    };

    let state = Arc::new(ProbeState::new());
    let reader_context = Box::into_raw(Box::new(ReaderContext {
        state: state.clone(),
        selected_index,
        width,
        height,
    }));

    let session = match CameraProbeSession::start(
        manager,
        &selected_source.camera_id_c,
        width,
        height,
        READER_MAX_IMAGES,
        reader_context,
    ) {
        Ok(session) => session,
        Err(error) => {
            drop(Box::from_raw(reader_context));
            return Err(error);
        }
    };

    let first_frame_seen = state.wait_for_first_frame(FIRST_FRAME_TIMEOUT);
    state.alive.store(false, Ordering::Release);
    session.stop();
    drop(Box::from_raw(reader_context));

    let status = if first_frame_seen { "ok" } else { "timeout" };
    emit_acquisition_marker(&format!(
        "phase=complete status={} selectedIndex={} width={} height={} frameCount={} hardwareBufferFrames={} acquireErrors={} elapsedMs={} import=none",
        status,
        selected_index,
        width,
        height,
        state.frame_count.load(Ordering::Acquire),
        state.hardware_buffer_frames.load(Ordering::Acquire),
        state.acquire_errors.load(Ordering::Acquire),
        started_at.elapsed().as_millis()
    ));

    Ok(())
}

fn emit_metadata_marker(body: &str) {
    emit_marker_line(&format!(
        "RUSTY_XR_MAKEPAD_CAMERA2_METADATA schema=rusty.xr.makepad-camera2.metadata.v1 {}",
        body
    ));
}

fn emit_acquisition_marker(body: &str) {
    emit_marker_line(&format!(
        "RUSTY_XR_MAKEPAD_CAMERA2_ACQUISITION schema=rusty.xr.makepad-camera2.acquisition.v1 {}",
        body
    ));
}

fn emit_stereo_projection_marker(body: &str) {
    emit_marker_line(&format!(
        "RUSTY_XR_MAKEPAD_STEREO_PROJECTION schema=rusty.xr.makepad-stereo-projection.v1 {}",
        body
    ));
}

fn metadata_marker_line(
    sources: &[CameraSource],
    private_source_count: usize,
    selected: Option<usize>,
    stereo_plan: Option<&StereoProjectionPlan>,
) -> String {
    let selected_source = selected.and_then(|index| sources.get(index));
    let selected_size = selected_source.and_then(select_capture_size);
    format!(
        "phase=enumerated status=ok sourceCount={} privateSourceCount={} selected={} selectedIndex={} selectedFacing={} selectedWidth={} selectedHeight={} selectedHasIntrinsics={} selectedHasPose={} selectedLogicalMultiCamera={} selectedPhysicalCount={} selectedSensorSync={} stereoPairSelected={} stereoLeftIndex={} stereoRightIndex={} stereoProjectionMetadataReady={} stereoProjectionHomographyReady={} stereoLeftSurfaceToCameraH={} stereoRightSurfaceToCameraH={} stereoLeftScreenToCameraH={} stereoRightScreenToCameraH={} stereoLeftScreenToSurfaceH={} stereoRightScreenToSurfaceH={} acquisitionPlan=bounded-single-private import=none",
        sources.len(),
        private_source_count,
        selected.is_some(),
        selected.map(index_token).unwrap_or_else(|| "none".to_string()),
        selected_source
            .map(CameraSource::lens_facing_label)
            .unwrap_or("none"),
        selected_size.map(|(width, _)| width).unwrap_or(0),
        selected_size.map(|(_, height)| height).unwrap_or(0),
        selected_source
            .map(|source| source.intrinsics.is_some())
            .unwrap_or(false),
        selected_source
            .map(|source| source.pose_translation.is_some() && source.pose_rotation.is_some())
            .unwrap_or(false),
        selected_source
            .map(|source| source.logical_multi_camera)
            .unwrap_or(false),
        selected_source
            .map(|source| source.physical_camera_ids.len())
            .unwrap_or(0),
        selected_source
            .and_then(|source| source.sensor_sync_type)
            .map(index_token)
            .unwrap_or_else(|| "none".to_string()),
        stereo_plan.is_some(),
        stereo_plan
            .map(|plan| plan.left_source_index.to_string())
            .unwrap_or_else(|| "none".to_string()),
        stereo_plan
            .map(|plan| plan.right_source_index.to_string())
            .unwrap_or_else(|| "none".to_string()),
        stereo_plan
            .map(|plan| plan.projection_metadata_ready)
            .unwrap_or(false),
        stereo_plan
            .map(|plan| plan.projection_homography_ready)
            .unwrap_or(false),
        stereo_plan
            .map(|plan| homography_token(plan.left_surface_to_camera_h))
            .unwrap_or_else(|| "identity".to_string()),
        stereo_plan
            .map(|plan| homography_token(plan.right_surface_to_camera_h))
            .unwrap_or_else(|| "identity".to_string()),
        stereo_plan
            .map(|plan| homography_token(plan.left_screen_to_camera_h))
            .unwrap_or_else(|| "identity".to_string()),
        stereo_plan
            .map(|plan| homography_token(plan.right_screen_to_camera_h))
            .unwrap_or_else(|| "identity".to_string()),
        stereo_plan
            .map(|plan| homography_token(plan.left_screen_to_surface_h))
            .unwrap_or_else(|| "identity".to_string()),
        stereo_plan
            .map(|plan| homography_token(plan.right_screen_to_surface_h))
            .unwrap_or_else(|| "identity".to_string())
    )
}

fn stereo_projection_metadata_line(
    sources: &[CameraSource],
    stereo_plan: Option<&StereoProjectionPlan>,
) -> String {
    match stereo_plan {
        Some(plan) => format!(
            "phase=metadata status=ok sourceCount={} leftSourceIndex={} rightSourceIndex={} leftCameraId={} rightCameraId={} leftFacing={} rightFacing={} width={} height={} pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} projectionHomographyReady={} leftSurfaceToCameraH={} rightSurfaceToCameraH={} leftSurfaceToScreenH={} rightSurfaceToScreenH={} leftScreenToCameraH={} rightScreenToCameraH={} leftScreenToSurfaceH={} rightScreenToSurfaceH={} projectionUvCorrection=screen_to_camera_homography_camera2_intrinsics_pose_display_eye sourceSelection=pose-x-ordered-camera2 fallbackReason={}",
            sources.len(),
            plan.left_source_index,
            plan.right_source_index,
            marker_token(&plan.left_camera_id),
            marker_token(&plan.right_camera_id),
            plan.left_facing,
            plan.right_facing,
            plan.width,
            plan.height,
            plan.projection_metadata_ready,
            plan.projection_metadata_ready,
            plan.pose_source,
            plan.source_eye_mapping,
            plan.coordinate_chain,
            plan.projection_homography_ready,
            homography_token(plan.left_surface_to_camera_h),
            homography_token(plan.right_surface_to_camera_h),
            homography_token(plan.left_surface_to_screen_h),
            homography_token(plan.right_surface_to_screen_h),
            homography_token(plan.left_screen_to_camera_h),
            homography_token(plan.right_screen_to_camera_h),
            homography_token(plan.left_screen_to_surface_h),
            homography_token(plan.right_screen_to_surface_h),
            marker_token(plan.fallback_reason)
        ),
        None => format!(
            "phase=metadata status=error sourceCount={} pairedLeftRightGpuBuffers=false projectionMappingReady=false alignedProjection=false projectionMetadataReady=false poseSource=missing fallbackReason=no_camera2_stereo_pair",
            sources.len()
        ),
    }
}

#[derive(Clone)]
pub(super) struct CameraSource {
    pub(super) camera_id_c: CString,
    pub(super) lens_facing: u8,
    pub(super) logical_multi_camera: bool,
    pub(super) physical_camera_ids: Vec<String>,
    pub(super) sensor_sync_type: Option<u8>,
    pub(super) private_sizes: Vec<(u32, u32)>,
    pub(super) intrinsics: Option<NativeIntrinsics>,
    pub(super) active_array_size: Option<(u32, u32)>,
    pub(super) pose_translation: Option<[f32; 3]>,
    pub(super) pose_rotation: Option<[f32; 4]>,
}

impl CameraSource {
    pub(super) fn camera_id_label(&self) -> String {
        self.camera_id_c.as_c_str().to_string_lossy().into_owned()
    }

    pub(super) fn lens_facing_label(&self) -> &'static str {
        match self.lens_facing {
            ACAMERA_LENS_FACING_FRONT => "front",
            ACAMERA_LENS_FACING_BACK => "back",
            ACAMERA_LENS_FACING_EXTERNAL => "external",
            _ => "unknown",
        }
    }

    fn lens_facing_rank(&self) -> i32 {
        match self.lens_facing {
            ACAMERA_LENS_FACING_BACK => 3,
            ACAMERA_LENS_FACING_EXTERNAL => 2,
            ACAMERA_LENS_FACING_FRONT => 1,
            _ => 0,
        }
    }

    pub(super) fn has_projection_metadata(&self) -> bool {
        self.intrinsics.is_some() && self.pose_translation.is_some() && self.pose_rotation.is_some()
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(super) struct NativeIntrinsics {
    pub(super) fx: f32,
    pub(super) fy: f32,
    pub(super) cx: f32,
    pub(super) cy: f32,
    pub(super) skew: f32,
}

#[derive(Default)]
struct ProbeState {
    alive: AtomicBool,
    frame_count: AtomicU32,
    hardware_buffer_frames: AtomicU32,
    acquire_errors: AtomicU32,
    first_frame_logged: AtomicBool,
    frame_ready: Mutex<bool>,
    frame_condition: Condvar,
}

impl ProbeState {
    fn new() -> Self {
        Self {
            alive: AtomicBool::new(true),
            ..Self::default()
        }
    }

    fn wait_for_first_frame(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let Ok(mut ready) = self.frame_ready.lock() else {
            return false;
        };

        while !*ready {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            let Ok(result) = self.frame_condition.wait_timeout(ready, remaining) else {
                return false;
            };
            ready = result.0;
            if result.1.timed_out() && !*ready {
                return false;
            }
        }
        true
    }

    fn notify_first_frame(&self) {
        if let Ok(mut ready) = self.frame_ready.lock() {
            *ready = true;
            self.frame_condition.notify_all();
        }
    }
}

struct ReaderContext {
    state: Arc<ProbeState>,
    selected_index: usize,
    width: u32,
    height: u32,
}

struct CameraProbeSession {
    capture_session: *mut ACameraCaptureSession,
    output_container: *mut ACaptureSessionOutputContainer,
    output: *mut ACaptureSessionOutput,
    camera_device: *mut ACameraDevice,
    target: *mut ACameraOutputTarget,
    window: *mut ANativeWindow,
    reader: *mut AImageReader,
    capture_request: *mut ACaptureRequest,
}

impl CameraProbeSession {
    unsafe fn start(
        manager: *mut ACameraManager,
        camera_id: &CString,
        width: u32,
        height: u32,
        reader_max_images: i32,
        reader_context: *mut ReaderContext,
    ) -> Result<Self, String> {
        let session = Self::prepare(
            manager,
            camera_id,
            width,
            height,
            reader_max_images,
            reader_context,
        )?;
        if let Err(error) = session.start_repeating() {
            session.stop();
            return Err(error);
        }
        Ok(session)
    }

    unsafe fn prepare(
        manager: *mut ACameraManager,
        camera_id: &CString,
        width: u32,
        height: u32,
        reader_max_images: i32,
        reader_context: *mut ReaderContext,
    ) -> Result<Self, String> {
        let mut device_callbacks = ACameraDevice_StateCallbacks {
            context: ptr::null_mut(),
            onDisconnected: Some(device_on_disconnected),
            onError: Some(device_on_error),
        };
        let mut camera_device = ptr::null_mut();
        let open_result = ACameraManager_openCamera(
            manager,
            camera_id.as_ptr(),
            &mut device_callbacks,
            &mut camera_device,
        );
        if open_result != 0 || camera_device.is_null() {
            return Err(format!("open_camera_failed_{open_result}"));
        }

        let mut capture_request = ptr::null_mut();
        let request_result = ACameraDevice_createCaptureRequest(
            camera_device,
            TEMPLATE_PREVIEW,
            &mut capture_request,
        );
        if request_result != 0 || capture_request.is_null() {
            ACameraDevice_close(camera_device);
            return Err(format!("create_request_failed_{request_result}"));
        }

        let mut reader = ptr::null_mut();
        let reader_result = AImageReader_newWithUsage(
            width as i32,
            height as i32,
            AIMAGE_FORMAT_PRIVATE,
            AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE,
            reader_max_images,
            &mut reader,
        );
        if reader_result != 0 || reader.is_null() {
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!("create_image_reader_failed_{reader_result}"));
        }

        let mut listener = AImageReader_ImageListener {
            context: reader_context.cast::<c_void>(),
            onImageAvailable: Some(image_on_image_available),
        };
        let listener_result = AImageReader_setImageListener(reader, &mut listener);
        if listener_result != 0 {
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!("set_image_listener_failed_{listener_result}"));
        }

        let mut window = ptr::null_mut();
        let window_result = AImageReader_getWindow(reader, &mut window);
        if window_result != 0 || window.is_null() {
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!("get_reader_window_failed_{window_result}"));
        }
        ANativeWindow_acquire(window);

        let mut target = ptr::null_mut();
        let target_result = ACameraOutputTarget_create(window, &mut target);
        if target_result != 0 || target.is_null() {
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!("create_output_target_failed_{target_result}"));
        }
        let add_target_result = ACaptureRequest_addTarget(capture_request, target);
        if add_target_result != 0 {
            ACameraOutputTarget_free(target);
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!("add_target_failed_{add_target_result}"));
        }

        let mut output = ptr::null_mut();
        let output_result = ACaptureSessionOutput_create(window, &mut output);
        if output_result != 0 || output.is_null() {
            ACaptureRequest_removeTarget(capture_request, target);
            ACameraOutputTarget_free(target);
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!("create_session_output_failed_{output_result}"));
        }

        let mut output_container = ptr::null_mut();
        let container_result = ACaptureSessionOutputContainer_create(&mut output_container);
        if container_result != 0 || output_container.is_null() {
            ACaptureSessionOutput_free(output);
            ACaptureRequest_removeTarget(capture_request, target);
            ACameraOutputTarget_free(target);
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!("create_output_container_failed_{container_result}"));
        }
        let container_add_result = ACaptureSessionOutputContainer_add(output_container, output);
        if container_add_result != 0 {
            ACaptureSessionOutputContainer_free(output_container);
            ACaptureSessionOutput_free(output);
            ACaptureRequest_removeTarget(capture_request, target);
            ACameraOutputTarget_free(target);
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!(
                "add_output_container_failed_{container_add_result}"
            ));
        }

        let session_callbacks = ACameraCaptureSession_stateCallbacks {
            context: ptr::null_mut(),
            onClosed: Some(session_on_closed),
            onReady: Some(session_on_ready),
            onActive: Some(session_on_active),
        };
        let mut capture_session = ptr::null_mut();
        let session_result = ACameraDevice_createCaptureSession(
            camera_device,
            output_container,
            &session_callbacks,
            &mut capture_session,
        );
        if session_result != 0 || capture_session.is_null() {
            ACaptureSessionOutputContainer_free(output_container);
            ACaptureSessionOutput_free(output);
            ACaptureRequest_removeTarget(capture_request, target);
            ACameraOutputTarget_free(target);
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!("create_capture_session_failed_{session_result}"));
        }

        Ok(Self {
            capture_session,
            output_container,
            output,
            camera_device,
            target,
            window,
            reader,
            capture_request,
        })
    }

    unsafe fn start_repeating(&self) -> Result<(), String> {
        let mut capture_request = self.capture_request;
        let repeat_result = ACameraCaptureSession_setRepeatingRequest(
            self.capture_session,
            ptr::null_mut(),
            1,
            &mut capture_request,
            ptr::null_mut(),
        );
        if repeat_result != 0 {
            return Err(format!("set_repeating_request_failed_{repeat_result}"));
        }
        Ok(())
    }

    unsafe fn stop(self) {
        let mut listener = AImageReader_ImageListener {
            context: ptr::null_mut(),
            onImageAvailable: None,
        };
        let _ = AImageReader_setImageListener(self.reader, &mut listener);
        ACameraCaptureSession_stopRepeating(self.capture_session);
        ACameraCaptureSession_close(self.capture_session);
        ACaptureSessionOutputContainer_free(self.output_container);
        ACaptureSessionOutput_free(self.output);
        ACaptureRequest_removeTarget(self.capture_request, self.target);
        ACameraOutputTarget_free(self.target);
        ANativeWindow_release(self.window);
        AImageReader_delete(self.reader);
        ACaptureRequest_free(self.capture_request);
        ACameraDevice_close(self.camera_device);
    }
}

unsafe extern "C" fn image_on_image_available(context: *mut c_void, reader: *mut AImageReader) {
    if context.is_null() || reader.is_null() {
        return;
    }
    let reader_context = &*(context as *mut ReaderContext);
    if !reader_context.state.alive.load(Ordering::Acquire) {
        return;
    }

    let mut image = ptr::null_mut();
    if AImageReader_acquireLatestImage(reader, &mut image) != 0 || image.is_null() {
        reader_context
            .state
            .acquire_errors
            .fetch_add(1, Ordering::AcqRel);
        return;
    }

    reader_context
        .state
        .frame_count
        .fetch_add(1, Ordering::AcqRel);

    let mut timestamp_ns = 0i64;
    let _ = AImage_getTimestamp(image, &mut timestamp_ns);
    let mut hardware_buffer = ptr::null_mut();
    let hardware_buffer_result = AImage_getHardwareBuffer(image, &mut hardware_buffer);
    if hardware_buffer_result == 0 && !hardware_buffer.is_null() {
        reader_context
            .state
            .hardware_buffer_frames
            .fetch_add(1, Ordering::AcqRel);

        if reader_context
            .state
            .first_frame_logged
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            emit_first_frame_marker(reader_context, timestamp_ns, hardware_buffer);
            reader_context.state.notify_first_frame();
        }
    } else {
        reader_context
            .state
            .acquire_errors
            .fetch_add(1, Ordering::AcqRel);
    }

    AImage_delete(image);
}

unsafe fn emit_first_frame_marker(
    reader_context: &ReaderContext,
    timestamp_ns: i64,
    hardware_buffer: *mut AHardwareBuffer,
) {
    let mut desc = std::mem::MaybeUninit::<ndk_sys::AHardwareBuffer_Desc>::zeroed();
    ndk_sys::AHardwareBuffer_describe(hardware_buffer, desc.as_mut_ptr());
    let desc = desc.assume_init();
    let mut buffer_id = 0u64;
    let id_result = ndk_sys::AHardwareBuffer_getId(hardware_buffer, &mut buffer_id);
    let buffer_id_present = id_result == 0 && buffer_id != 0;

    emit_acquisition_marker(&format!(
        "phase=first-frame status=frame selectedIndex={} width={} height={} timestampNs={} hardwareBufferPresent=true nativeFormat={} usage={} layers={} stride={} bufferIdPresent={} import=none",
        reader_context.selected_index,
        reader_context.width,
        reader_context.height,
        timestamp_ns,
        desc.format,
        desc.usage,
        desc.layers,
        desc.stride,
        buffer_id_present
    ));
}

unsafe extern "C" fn device_on_disconnected(_context: *mut c_void, _device: *mut ACameraDevice) {
    emit_acquisition_marker("phase=device-callback status=disconnected");
}

unsafe extern "C" fn device_on_error(
    _context: *mut c_void,
    _device: *mut ACameraDevice,
    error: i32,
) {
    emit_acquisition_marker(&format!(
        "phase=device-callback status=error errorCode={error}"
    ));
}

unsafe extern "C" fn session_on_closed(
    _context: *mut c_void,
    _session: *mut ACameraCaptureSession,
) {
}

unsafe extern "C" fn session_on_ready(_context: *mut c_void, _session: *mut ACameraCaptureSession) {
}

unsafe extern "C" fn session_on_active(
    _context: *mut c_void,
    _session: *mut ACameraCaptureSession,
) {
}

unsafe fn enumerate_camera_sources(
    manager: *mut ACameraManager,
) -> Result<Vec<CameraSource>, String> {
    let mut camera_ids_ptr = ptr::null_mut();
    let result = ACameraManager_getCameraIdList(manager, &mut camera_ids_ptr);
    if result != 0 || camera_ids_ptr.is_null() {
        return Err(format!("get_camera_id_list_failed_{result}"));
    }

    let camera_ids = std::slice::from_raw_parts(
        (*camera_ids_ptr).cameraIds,
        (*camera_ids_ptr).numCameras.max(0) as usize,
    );
    let mut sources = Vec::new();

    for &camera_id_ptr in camera_ids {
        if camera_id_ptr.is_null() {
            continue;
        }
        let camera_id = CStr::from_ptr(camera_id_ptr).to_string_lossy().into_owned();
        if let Ok(Some(source)) = load_camera_source_by_id(manager, &camera_id) {
            sources.push(source);
        }
    }

    ACameraManager_deleteCameraIdList(camera_ids_ptr);
    if sources.is_empty() {
        return Err("camera_enumeration_no_usable_sources".to_string());
    }
    Ok(sources)
}

unsafe fn load_camera_source_by_id(
    manager: *mut ACameraManager,
    camera_id: &str,
) -> Result<Option<CameraSource>, String> {
    let camera_id_c = CString::new(camera_id).map_err(|_| "camera_id_contains_null".to_string())?;
    let mut metadata = ptr::null_mut();
    let result =
        ACameraManager_getCameraCharacteristics(manager, camera_id_c.as_ptr(), &mut metadata);
    if result != 0 || metadata.is_null() {
        return Err(format!("get_camera_characteristics_failed_{result}"));
    }

    let source = camera_source_from_metadata(camera_id_c, metadata);
    ACameraMetadata_free(metadata);
    Ok(source)
}

unsafe fn camera_source_from_metadata(
    camera_id_c: CString,
    metadata: *const ACameraMetadata,
) -> Option<CameraSource> {
    let lens_facing = metadata_u8(metadata, ACAMERA_LENS_FACING)?;
    let capabilities = metadata_u8_vec(metadata, ACAMERA_REQUEST_AVAILABLE_CAPABILITIES);
    let physical_camera_ids =
        metadata_string_list(metadata, ACAMERA_LOGICAL_MULTI_CAMERA_PHYSICAL_IDS);

    Some(CameraSource {
        camera_id_c,
        lens_facing,
        logical_multi_camera: capabilities.iter().any(|capability| {
            *capability == ACAMERA_REQUEST_AVAILABLE_CAPABILITIES_LOGICAL_MULTI_CAMERA
        }),
        physical_camera_ids,
        sensor_sync_type: metadata_u8(metadata, ACAMERA_LOGICAL_MULTI_CAMERA_SENSOR_SYNC_TYPE),
        private_sizes: metadata_private_output_sizes(metadata),
        intrinsics: metadata_intrinsics(metadata),
        active_array_size: metadata_active_array_size(metadata),
        pose_translation: metadata_vec3(metadata, ACAMERA_LENS_POSE_TRANSLATION),
        pose_rotation: metadata_quat(metadata, ACAMERA_LENS_POSE_ROTATION),
    })
}

fn select_camera_source(sources: &[CameraSource]) -> Option<usize> {
    sources
        .iter()
        .enumerate()
        .filter(|(_, source)| !source.private_sizes.is_empty())
        .max_by_key(|(_, source)| {
            (
                source.lens_facing_rank(),
                source.logical_multi_camera as i32,
                source.physical_camera_ids.len() as i32,
                select_capture_size(source)
                    .map(|(width, height)| score_size(width, height))
                    .unwrap_or(i64::MIN),
            )
        })
        .map(|(index, _)| index)
}

fn select_stereo_projection_sources(sources: &[CameraSource]) -> Option<StereoProjectionSources> {
    select_pose_ordered_stereo_projection_sources(sources)
        .or_else(|| select_best_index_ordered_stereo_projection_sources(sources))
}

fn select_pose_ordered_stereo_projection_sources(
    sources: &[CameraSource],
) -> Option<StereoProjectionSources> {
    let mut back_sources = sources
        .iter()
        .enumerate()
        .filter(|(_, source)| source.lens_facing == ACAMERA_LENS_FACING_BACK)
        .filter(|(_, source)| !source.private_sizes.is_empty())
        .collect::<Vec<_>>();
    if back_sources.len() < 2 {
        return None;
    }

    back_sources.sort_by(|(_, left), (_, right)| {
        let left_x = left.pose_translation.map(|pose| pose[0]).unwrap_or(0.0);
        let right_x = right.pose_translation.map(|pose| pose[0]).unwrap_or(0.0);
        left_x
            .partial_cmp(&right_x)
            .unwrap_or(CmpOrdering::Equal)
            .then_with(|| left.camera_id_label().cmp(&right.camera_id_label()))
    });

    let left_index = back_sources.first()?.0;
    let right_index = back_sources.last()?.0;
    if left_index == right_index {
        return None;
    }
    let left = &sources[left_index];
    let right = &sources[right_index];
    let (width, height) = select_stereo_capture_size(left, right)?;
    Some(StereoProjectionSources {
        left_source_index: left_index,
        left: left.clone(),
        right_source_index: right_index,
        right: right.clone(),
        width,
        height,
    })
}

fn select_best_index_ordered_stereo_projection_sources(
    sources: &[CameraSource],
) -> Option<StereoProjectionSources> {
    let mut best: Option<(usize, usize, (u8, i64, i64, i64), (u32, u32))> = None;

    for left_index in 0..sources.len() {
        for right_index in 0..sources.len() {
            if left_index == right_index {
                continue;
            }

            let left = &sources[left_index];
            let right = &sources[right_index];
            if left.private_sizes.is_empty() || right.private_sizes.is_empty() {
                continue;
            }

            let Some((width, height)) = select_stereo_capture_size(left, right) else {
                continue;
            };

            let metadata_rank =
                (left.has_projection_metadata() as u8) + (right.has_projection_metadata() as u8);
            let source_rank = (left.lens_facing_rank() as i64) + (right.lens_facing_rank() as i64);
            let sync_rank = (left.sensor_sync_type == right.sensor_sync_type) as i64
                + left.logical_multi_camera as i64
                + right.logical_multi_camera as i64;
            let index_spacing = left_index.abs_diff(right_index) as i64;
            let score = (
                metadata_rank,
                source_rank,
                score_size(width, height) + sync_rank * 1_000_000,
                -index_spacing,
            );

            if best
                .as_ref()
                .map(|(_, _, best_score, _)| score > *best_score)
                .unwrap_or(true)
            {
                best = Some((left_index, right_index, score, (width, height)));
            }
        }
    }

    let (left_index, right_index, _, (width, height)) = best?;
    Some(StereoProjectionSources {
        left_source_index: left_index,
        left: sources[left_index].clone(),
        right_source_index: right_index,
        right: sources[right_index].clone(),
        width,
        height,
    })
}

fn select_stereo_capture_size(left: &CameraSource, right: &CameraSource) -> Option<(u32, u32)> {
    left.private_sizes
        .iter()
        .copied()
        .filter(|size| right.private_sizes.contains(size))
        .filter(|(width, height)| {
            *width <= MAX_CAPTURE_DIMENSION && *height <= MAX_CAPTURE_DIMENSION
        })
        .max_by_key(|(width, height)| score_size(*width, *height))
        .or_else(|| {
            left.private_sizes
                .iter()
                .copied()
                .filter(|size| right.private_sizes.contains(size))
                .min_by_key(|(width, height)| (*width as u64) * (*height as u64))
        })
}

fn select_capture_size(source: &CameraSource) -> Option<(u32, u32)> {
    source
        .private_sizes
        .iter()
        .copied()
        .filter(|(width, height)| {
            *width <= MAX_CAPTURE_DIMENSION && *height <= MAX_CAPTURE_DIMENSION
        })
        .max_by_key(|(width, height)| score_size(*width, *height))
        .or_else(|| {
            source
                .private_sizes
                .iter()
                .copied()
                .min_by_key(|(width, height)| (*width as u64) * (*height as u64))
        })
}

fn score_size(width: u32, height: u32) -> i64 {
    let target_penalty =
        (width.abs_diff(PREFERRED_DIMENSION) + height.abs_diff(PREFERRED_DIMENSION)) as i64;
    let square_penalty = width.abs_diff(height) as i64;
    let area = (width as i64) * (height as i64);
    area - target_penalty * 2048 - square_penalty * 4096
}

unsafe fn metadata_entry(
    metadata: *const ACameraMetadata,
    tag: u32,
) -> Option<ACameraMetadata_const_entry> {
    let mut entry = std::mem::MaybeUninit::<ACameraMetadata_const_entry>::zeroed();
    if ACameraMetadata_getConstEntry(metadata, tag, entry.as_mut_ptr()) != 0 {
        return None;
    }
    let entry = entry.assume_init();
    (entry.count > 0).then_some(entry)
}

unsafe fn metadata_u8(metadata: *const ACameraMetadata, tag: u32) -> Option<u8> {
    let entry = metadata_entry(metadata, tag)?;
    (!entry.data.u8_.is_null()).then(|| *entry.data.u8_)
}

unsafe fn metadata_u8_vec(metadata: *const ACameraMetadata, tag: u32) -> Vec<u8> {
    let Some(entry) = metadata_entry(metadata, tag) else {
        return Vec::new();
    };
    if entry.data.u8_.is_null() {
        return Vec::new();
    }
    std::slice::from_raw_parts(entry.data.u8_, entry.count as usize).to_vec()
}

unsafe fn metadata_string_list(metadata: *const ACameraMetadata, tag: u32) -> Vec<String> {
    let Some(entry) = metadata_entry(metadata, tag) else {
        return Vec::new();
    };
    if entry.data.u8_.is_null() || entry.count == 0 {
        return Vec::new();
    }

    std::slice::from_raw_parts(entry.data.u8_, entry.count as usize)
        .split(|byte| *byte == 0)
        .filter_map(|chunk| {
            if chunk.is_empty() {
                None
            } else {
                std::str::from_utf8(chunk)
                    .ok()
                    .map(|value| value.to_string())
            }
        })
        .collect()
}

unsafe fn metadata_intrinsics(metadata: *const ACameraMetadata) -> Option<NativeIntrinsics> {
    let entry = metadata_entry(metadata, ACAMERA_LENS_INTRINSIC_CALIBRATION)?;
    if entry.count < 4 || entry.data.f.is_null() {
        return None;
    }
    let values = std::slice::from_raw_parts(entry.data.f, entry.count as usize);
    Some(NativeIntrinsics {
        fx: values[0],
        fy: values[1],
        cx: values[2],
        cy: values[3],
        skew: values.get(4).copied().unwrap_or(0.0),
    })
}

unsafe fn metadata_vec3(metadata: *const ACameraMetadata, tag: u32) -> Option<[f32; 3]> {
    let entry = metadata_entry(metadata, tag)?;
    if entry.count < 3 || entry.data.f.is_null() {
        return None;
    }
    let values = std::slice::from_raw_parts(entry.data.f, entry.count as usize);
    Some([values[0], values[1], values[2]])
}

unsafe fn metadata_quat(metadata: *const ACameraMetadata, tag: u32) -> Option<[f32; 4]> {
    let entry = metadata_entry(metadata, tag)?;
    if entry.count < 4 || entry.data.f.is_null() {
        return None;
    }
    let values = std::slice::from_raw_parts(entry.data.f, entry.count as usize);
    Some([values[0], values[1], values[2], values[3]])
}

unsafe fn metadata_active_array_size(metadata: *const ACameraMetadata) -> Option<(u32, u32)> {
    let entry = metadata_entry(metadata, ACAMERA_SENSOR_INFO_ACTIVE_ARRAY_SIZE)?;
    if entry.count < 4 || entry.data.i32_.is_null() {
        return None;
    }
    let values = std::slice::from_raw_parts(entry.data.i32_, entry.count as usize);
    let width = values[2].checked_sub(values[0])?;
    let height = values[3].checked_sub(values[1])?;
    (width > 0 && height > 0).then_some((width as u32, height as u32))
}

unsafe fn metadata_private_output_sizes(metadata: *const ACameraMetadata) -> Vec<(u32, u32)> {
    let Some(entry) = metadata_entry(metadata, ACAMERA_SCALER_AVAILABLE_STREAM_CONFIGURATIONS)
    else {
        return Vec::new();
    };
    if entry.count < 4 || entry.data.i32_.is_null() {
        return Vec::new();
    }
    let values = std::slice::from_raw_parts(entry.data.i32_, entry.count as usize);
    let mut sizes = BTreeSet::new();
    for chunk in values.chunks_exact(4) {
        let format = chunk[0] as u32;
        let width = chunk[1];
        let height = chunk[2];
        let input = chunk[3];
        if format == AIMAGE_FORMAT_PRIVATE && input == 0 && width > 0 && height > 0 {
            sizes.insert((width as u32, height as u32));
        }
    }
    sizes.into_iter().collect()
}

fn index_token<T: std::fmt::Display>(value: T) -> String {
    value.to_string()
}

fn homography_token(rows: [[f32; 3]; 3]) -> String {
    rows.iter()
        .flat_map(|row| row.iter())
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn vec4_token(values: [f32; 4]) -> String {
    format!(
        "[{:.6},{:.6},{:.6},{:.6}]",
        values[0], values[1], values[2], values[3]
    )
}

fn optional_vec4_token(values: Option<[f32; 4]>) -> String {
    values
        .map(vec4_token)
        .unwrap_or_else(|| "not-logged".to_string())
}

fn optional_i64_token(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not-logged".to_string())
}

fn optional_f32_token(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "not-logged".to_string())
}

fn openxr_contract_marker_fields(contract: XrProjectionContract) -> String {
    format!(
        "referenceSpace={} openxrReferenceSpace={} displayTimeSource={} predictedDisplayTimeSource={} predictedDisplayTimeNs={} viewPoseFovSource={} projectionDepthMeters={} cameraPreviewFovYDegrees={} cameraPreviewOffsetYMeters={} cameraRawOverlayOverscan={} leftRenderFovTangents={} rightRenderFovTangents={} leftRenderPosition={} rightRenderPosition={} leftRenderOrientation={} rightRenderOrientation={}",
        marker_token(contract.reference_space),
        marker_token(contract.openxr_reference_space),
        marker_token(contract.display_time_source),
        marker_token(contract.display_time_source),
        optional_i64_token(contract.predicted_display_time_ns),
        marker_token(contract.view_pose_fov_source),
        optional_f32_token(contract.projection_depth_meters),
        optional_f32_token(contract.projection_preview_fov_y_degrees),
        optional_f32_token(contract.projection_preview_offset_y_meters),
        optional_f32_token(contract.projection_raw_overscan),
        optional_vec4_token(contract.left_render_fov_tangents),
        optional_vec4_token(contract.right_render_fov_tangents),
        optional_vec4_token(contract.left_render_position),
        optional_vec4_token(contract.right_render_position),
        optional_vec4_token(contract.left_render_orientation),
        optional_vec4_token(contract.right_render_orientation),
    )
}

fn marker_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}
