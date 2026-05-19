use crate::{
    acamera_sys::*, log_error, log_info, store_headset_stereo_camera_gpu_frame,
    AndroidHardwareBufferHandle, HeadsetCameraGpuBufferImport,
};
use rusty_xr_contracts::{CameraGpuBufferDescriptor, ImageSize};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    ffi::{CStr, CString},
    os::raw::c_void,
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

const DEFAULT_PAIR_MAX_DELTA_NS: u64 = 5_000_000;

static NATIVE_CAMERA_SESSION: OnceLock<Mutex<Option<NativeCameraSession>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeCameraConfig {
    width: Option<u32>,
    height: Option<u32>,
    max_dimension: Option<u32>,
    preferred_square: Option<u32>,
    reader_max_images: Option<i32>,
    stereo_pair_max_delta_ns: Option<u64>,
    requested_tier: Option<String>,
    requested_stereo_layout: Option<String>,
    source_mode: Option<String>,
    left_camera_id: Option<String>,
    right_camera_id: Option<String>,
}

impl NativeCameraConfig {
    fn requested_width(&self) -> u32 {
        self.width.unwrap_or(1280).max(1)
    }

    fn requested_height(&self) -> u32 {
        self.height.unwrap_or(1280).max(1)
    }

    fn max_dimension(&self) -> u32 {
        self.max_dimension.unwrap_or(1920).max(1)
    }

    fn pair_max_delta_ns(&self) -> u64 {
        self.stereo_pair_max_delta_ns
            .unwrap_or(DEFAULT_PAIR_MAX_DELTA_NS)
    }

    fn reader_max_images(&self) -> i32 {
        self.reader_max_images.unwrap_or(3).clamp(2, 16)
    }

    fn requested_tier(&self) -> &str {
        self.requested_tier.as_deref().unwrap_or("gpu-projected")
    }

    fn requested_stereo_layout(&self) -> &str {
        self.requested_stereo_layout
            .as_deref()
            .unwrap_or("separate")
    }

    fn source_mode(&self) -> &str {
        self.source_mode.as_deref().unwrap_or("auto")
    }

    fn single_camera_mirror(&self) -> bool {
        let mode = self.source_mode().to_ascii_lowercase();
        mode == "single-back-mirror"
            || mode == "mono-mirror"
            || mode == "single-camera-mirror"
            || mode == "mirror-left"
    }

    fn deferred_dual_repeating(&self) -> bool {
        let mode = self.source_mode().to_ascii_lowercase();
        mode == "dual-back-deferred-repeat"
            || mode == "deferred-dual-back"
            || mode == "prepare-then-repeat-dual-back"
            || mode == "synthetic-dual-back-deferred-repeat"
    }
}

pub fn start_from_json(config_json: &str) -> Result<(), String> {
    let config: NativeCameraConfig = serde_json::from_str(config_json)
        .map_err(|error| format!("parse native camera config: {error}"))?;
    let mut guard = native_session_state()
        .lock()
        .map_err(|_| "native camera session mutex poisoned".to_string())?;
    if let Some(session) = guard.take() {
        session.stop();
    }
    let session = NativeCameraSession::start(config)?;
    *guard = Some(session);
    Ok(())
}

pub fn stop() {
    if let Ok(mut guard) = native_session_state().lock() {
        if let Some(session) = guard.take() {
            session.stop();
        }
    }
}

fn native_session_state() -> &'static Mutex<Option<NativeCameraSession>> {
    NATIVE_CAMERA_SESSION.get_or_init(|| Mutex::new(None))
}

struct NativeCameraSession {
    manager: *mut ACameraManager,
    left: NativeCameraSideSession,
    right: Option<NativeCameraSideSession>,
    context: Arc<NativeStereoContext>,
    publish_stop: Arc<AtomicBool>,
    publish_thread: Option<thread::JoinHandle<()>>,
    left_reader_context: *mut NativeReaderContext,
    right_reader_context: Option<*mut NativeReaderContext>,
}

unsafe impl Send for NativeCameraSession {}

impl NativeCameraSession {
    fn start(config: NativeCameraConfig) -> Result<Self, String> {
        unsafe {
            let manager = ACameraManager_create();
            if manager.is_null() {
                return Err("ACameraManager_create returned null".to_string());
            }

            let sources = enumerate_camera_sources(manager)?;
            let single_camera_mirror = config.single_camera_mirror();
            let (left_source, right_source, selected_size) = if single_camera_mirror {
                let (source, selected_size) =
                    select_single_mirror_source(manager, &sources, &config)?;
                (source.clone(), source, selected_size)
            } else {
                select_stereo_sources(manager, &sources, &config)?
            };
            let width = selected_size.0;
            let height = selected_size.1;

            let context = Arc::new(NativeStereoContext {
                alive: AtomicBool::new(true),
                left_info: NativeCameraSideInfo::from_source(&left_source, width, height, true),
                right_info: NativeCameraSideInfo::from_source(&right_source, width, height, false),
                pair_max_delta_ns: config.pair_max_delta_ns(),
                reader_max_images: config.reader_max_images(),
                requested_tier: config.requested_tier().to_string(),
                requested_stereo_layout: config.requested_stereo_layout().to_string(),
                source_mode: config.source_mode().to_string(),
                single_camera_mirror,
                pair_state: Mutex::new(NativePairState::default()),
                pair_index: AtomicU64::new(0),
                left_received_count: AtomicU64::new(0),
                right_received_count: AtomicU64::new(0),
                logged_pair_delta: AtomicBool::new(false),
                logged_first_pair: AtomicBool::new(false),
                publish_queue: NativePublishQueue::default(),
            });

            let publish_stop = Arc::new(AtomicBool::new(false));
            let publish_thread = {
                let worker_context = context.clone();
                let worker_stop = publish_stop.clone();
                thread::Builder::new()
                    .name("RustyXrNativeCameraPublish".to_string())
                    .spawn(move || worker_context.run_publish_worker(worker_stop))
                    .map_err(|error| format!("spawn native camera publish worker: {error}"))?
            };

            let left_reader_context = Box::into_raw(Box::new(NativeReaderContext {
                context: context.clone(),
                side: NativeCameraSide::Left,
            }));
            let right_reader_context = if single_camera_mirror {
                None
            } else {
                Some(Box::into_raw(Box::new(NativeReaderContext {
                    context: context.clone(),
                    side: NativeCameraSide::Right,
                })))
            };

            let deferred_dual_repeating = !single_camera_mirror && config.deferred_dual_repeating();
            let (left, right) = if deferred_dual_repeating {
                let Some(right_reader_context) = right_reader_context else {
                    unreachable!("deferred dual repeating requires a right reader context");
                };
                let left = match NativeCameraSideSession::prepare(
                    manager,
                    &left_source.camera_id_c,
                    width,
                    height,
                    context.reader_max_images,
                    left_reader_context,
                ) {
                    Ok(session) => session,
                    Err(error) => {
                        drop(Box::from_raw(left_reader_context));
                        drop(Box::from_raw(right_reader_context));
                        publish_stop.store(true, Ordering::Release);
                        context.publish_queue.notify();
                        let _ = publish_thread.join();
                        ACameraManager_delete(manager);
                        return Err(error);
                    }
                };
                let right = match NativeCameraSideSession::prepare(
                    manager,
                    &right_source.camera_id_c,
                    width,
                    height,
                    context.reader_max_images,
                    right_reader_context,
                ) {
                    Ok(session) => session,
                    Err(error) => {
                        left.stop();
                        drop(Box::from_raw(left_reader_context));
                        drop(Box::from_raw(right_reader_context));
                        publish_stop.store(true, Ordering::Release);
                        context.publish_queue.notify();
                        let _ = publish_thread.join();
                        ACameraManager_delete(manager);
                        return Err(error);
                    }
                };
                log_info(format!(
                    "Rusty XR native ACamera deferred dual-side start prepared both sessions before repeating leftId={} rightId={} size={}x{}",
                    context.left_info.camera_id, context.right_info.camera_id, width, height
                ));
                if let Err(error) = left.start_repeating(&left_source.camera_id_c) {
                    left.stop();
                    right.stop();
                    drop(Box::from_raw(left_reader_context));
                    drop(Box::from_raw(right_reader_context));
                    publish_stop.store(true, Ordering::Release);
                    context.publish_queue.notify();
                    let _ = publish_thread.join();
                    ACameraManager_delete(manager);
                    return Err(error);
                }
                if let Err(error) = right.start_repeating(&right_source.camera_id_c) {
                    left.stop();
                    right.stop();
                    drop(Box::from_raw(left_reader_context));
                    drop(Box::from_raw(right_reader_context));
                    publish_stop.store(true, Ordering::Release);
                    context.publish_queue.notify();
                    let _ = publish_thread.join();
                    ACameraManager_delete(manager);
                    return Err(error);
                }
                (left, Some(right))
            } else {
                let left = match NativeCameraSideSession::start(
                    manager,
                    &left_source.camera_id_c,
                    width,
                    height,
                    context.reader_max_images,
                    left_reader_context,
                ) {
                    Ok(session) => session,
                    Err(error) => {
                        drop(Box::from_raw(left_reader_context));
                        if let Some(right_reader_context) = right_reader_context {
                            drop(Box::from_raw(right_reader_context));
                        }
                        publish_stop.store(true, Ordering::Release);
                        context.publish_queue.notify();
                        let _ = publish_thread.join();
                        ACameraManager_delete(manager);
                        return Err(error);
                    }
                };

                let right = if let Some(right_reader_context) = right_reader_context {
                    match NativeCameraSideSession::start(
                        manager,
                        &right_source.camera_id_c,
                        width,
                        height,
                        context.reader_max_images,
                        right_reader_context,
                    ) {
                        Ok(session) => Some(session),
                        Err(error) => {
                            left.stop();
                            drop(Box::from_raw(left_reader_context));
                            drop(Box::from_raw(right_reader_context));
                            publish_stop.store(true, Ordering::Release);
                            context.publish_queue.notify();
                            let _ = publish_thread.join();
                            ACameraManager_delete(manager);
                            return Err(error);
                        }
                    }
                } else {
                    None
                };
                (left, right)
            };

            log_info(format!(
                "Rusty XR native ACamera stereo acquisition running leftId={} rightId={} size={}x{} readerMaxImages={} requestedTier={} stereoLayout={} sourceMode={} singleCameraMirror={} deferredDualRepeating={} aeFps=unset",
                context.left_info.camera_id,
                context.right_info.camera_id,
                width,
                height,
                context.reader_max_images,
                context.requested_tier,
                context.requested_stereo_layout,
                context.source_mode,
                context.single_camera_mirror,
                deferred_dual_repeating,
            ));

            Ok(Self {
                manager,
                left,
                right,
                context,
                publish_stop,
                publish_thread: Some(publish_thread),
                left_reader_context,
                right_reader_context,
            })
        }
    }

    fn stop(mut self) {
        self.context.alive.store(false, Ordering::Release);
        self.publish_stop.store(true, Ordering::Release);
        self.context.publish_queue.notify();
        unsafe {
            self.left.stop();
            if let Some(right) = self.right {
                right.stop();
            }
            drop(Box::from_raw(self.left_reader_context));
            if let Some(right_reader_context) = self.right_reader_context {
                drop(Box::from_raw(right_reader_context));
            }
            ACameraManager_delete(self.manager);
        }
        if let Some(thread) = self.publish_thread.take() {
            let _ = thread.join();
        }
        log_info("Rusty XR native ACamera stereo acquisition stopped");
    }
}

struct NativeCameraSideSession {
    capture_session: *mut ACameraCaptureSession,
    output_container: *mut ACaptureSessionOutputContainer,
    output: *mut ACaptureSessionOutput,
    camera_device: *mut ACameraDevice,
    target: *mut ACameraOutputTarget,
    window: *mut ANativeWindow,
    reader: *mut AImageReader,
    capture_request: *mut ACaptureRequest,
}

unsafe impl Send for NativeCameraSideSession {}

impl NativeCameraSideSession {
    unsafe fn start(
        manager: *mut ACameraManager,
        camera_id: &CString,
        width: u32,
        height: u32,
        reader_max_images: i32,
        reader_context: *mut NativeReaderContext,
    ) -> Result<Self, String> {
        let session = Self::prepare(
            manager,
            camera_id,
            width,
            height,
            reader_max_images,
            reader_context,
        )?;
        if let Err(error) = session.start_repeating(camera_id) {
            session.stop();
            return Err(error);
        }
        log_info(format!(
            "Rusty XR native ACamera side running cameraId={} size={}x{} readerMaxImages={} aeFps=unset",
            camera_id.to_string_lossy(),
            width,
            height,
            reader_max_images
        ));
        Ok(session)
    }

    unsafe fn prepare(
        manager: *mut ACameraManager,
        camera_id: &CString,
        width: u32,
        height: u32,
        reader_max_images: i32,
        reader_context: *mut NativeReaderContext,
    ) -> Result<Self, String> {
        let mut device_callbacks = ACameraDevice_StateCallbacks {
            context: ptr::null_mut(),
            onDisconnected: Some(device_on_disconnected),
            onError: Some(device_on_error),
        };
        let mut camera_device = ptr::null_mut();
        if ACameraManager_openCamera(
            manager,
            camera_id.as_ptr(),
            &mut device_callbacks,
            &mut camera_device,
        ) != 0
            || camera_device.is_null()
        {
            return Err(format!(
                "native ACamera open failed cameraId={}",
                camera_id.to_string_lossy()
            ));
        }

        let mut capture_request = ptr::null_mut();
        if ACameraDevice_createCaptureRequest(camera_device, TEMPLATE_PREVIEW, &mut capture_request)
            != 0
            || capture_request.is_null()
        {
            ACameraDevice_close(camera_device);
            return Err(format!(
                "native ACamera create request failed cameraId={}",
                camera_id.to_string_lossy()
            ));
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
            return Err(format!(
                "native AImageReader create failed cameraId={} result={} size={}x{}",
                camera_id.to_string_lossy(),
                reader_result,
                width,
                height
            ));
        }

        let mut listener = AImageReader_ImageListener {
            context: reader_context.cast(),
            onImageAvailable: Some(image_on_image_available),
        };
        let _ = AImageReader_setImageListener(reader, &mut listener);

        let mut window = ptr::null_mut();
        if AImageReader_getWindow(reader, &mut window) != 0 || window.is_null() {
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!(
                "native AImageReader get window failed cameraId={}",
                camera_id.to_string_lossy()
            ));
        }
        ANativeWindow_acquire(window);

        let mut target = ptr::null_mut();
        if ACameraOutputTarget_create(window, &mut target) != 0 || target.is_null() {
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!(
                "native ACamera output target failed cameraId={}",
                camera_id.to_string_lossy()
            ));
        }
        let _ = ACaptureRequest_addTarget(capture_request, target);

        let mut output = ptr::null_mut();
        if ACaptureSessionOutput_create(window, &mut output) != 0 || output.is_null() {
            ACaptureRequest_removeTarget(capture_request, target);
            ACameraOutputTarget_free(target);
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!(
                "native ACamera session output failed cameraId={}",
                camera_id.to_string_lossy()
            ));
        }

        let mut output_container = ptr::null_mut();
        ACaptureSessionOutputContainer_create(&mut output_container);
        if output_container.is_null() {
            ACaptureSessionOutput_free(output);
            ACaptureRequest_removeTarget(capture_request, target);
            ACameraOutputTarget_free(target);
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!(
                "native ACamera output container failed cameraId={}",
                camera_id.to_string_lossy()
            ));
        }
        let _ = ACaptureSessionOutputContainer_add(output_container, output);

        let session_callbacks = ACameraCaptureSession_stateCallbacks {
            context: ptr::null_mut(),
            onClosed: Some(session_on_closed),
            onReady: Some(session_on_ready),
            onActive: Some(session_on_active),
        };
        let mut capture_session = ptr::null_mut();
        if ACameraDevice_createCaptureSession(
            camera_device,
            output_container,
            &session_callbacks,
            &mut capture_session,
        ) != 0
            || capture_session.is_null()
        {
            ACaptureSessionOutputContainer_free(output_container);
            ACaptureSessionOutput_free(output);
            ACaptureRequest_removeTarget(capture_request, target);
            ACameraOutputTarget_free(target);
            ANativeWindow_release(window);
            AImageReader_delete(reader);
            ACaptureRequest_free(capture_request);
            ACameraDevice_close(camera_device);
            return Err(format!(
                "native ACamera capture session failed cameraId={}",
                camera_id.to_string_lossy()
            ));
        }

        log_info(format!(
            "Rusty XR native ACamera side prepared cameraId={} size={}x{} readerMaxImages={} repeating=not-started",
            camera_id.to_string_lossy(),
            width,
            height,
            reader_max_images
        ));

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

    unsafe fn start_repeating(&self, camera_id: &CString) -> Result<(), String> {
        let mut capture_request = self.capture_request;
        if ACameraCaptureSession_setRepeatingRequest(
            self.capture_session,
            ptr::null_mut(),
            1,
            &mut capture_request,
            ptr::null_mut(),
        ) != 0
        {
            return Err(format!(
                "native ACamera repeating request failed cameraId={}",
                camera_id.to_string_lossy()
            ));
        }
        log_info(format!(
            "Rusty XR native ACamera side repeating cameraId={}",
            camera_id.to_string_lossy()
        ));
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

#[derive(Clone, Copy)]
enum NativeCameraSide {
    Left,
    Right,
}

struct NativeReaderContext {
    context: Arc<NativeStereoContext>,
    side: NativeCameraSide,
}

struct NativeStereoContext {
    alive: AtomicBool,
    left_info: NativeCameraSideInfo,
    right_info: NativeCameraSideInfo,
    pair_max_delta_ns: u64,
    reader_max_images: i32,
    requested_tier: String,
    requested_stereo_layout: String,
    source_mode: String,
    single_camera_mirror: bool,
    pair_state: Mutex<NativePairState>,
    pair_index: AtomicU64,
    left_received_count: AtomicU64,
    right_received_count: AtomicU64,
    logged_pair_delta: AtomicBool,
    logged_first_pair: AtomicBool,
    publish_queue: NativePublishQueue,
}

impl NativeStereoContext {
    fn publish_frame(
        &self,
        side: NativeCameraSide,
        timestamp_ns: i64,
        hardware_buffer: AndroidHardwareBufferHandle,
    ) -> Result<(), String> {
        let info = match side {
            NativeCameraSide::Left => &self.left_info,
            NativeCameraSide::Right => &self.right_info,
        };
        let side_count = match side {
            NativeCameraSide::Left => self.left_received_count.fetch_add(1, Ordering::Relaxed) + 1,
            NativeCameraSide::Right => {
                self.right_received_count.fetch_add(1, Ordering::Relaxed) + 1
            }
        };
        if side_count <= 5 || side_count.is_multiple_of(30) {
            log_info(format!(
                "Rusty XR native ACamera side frame side={} count={} ts={} cameraId={} readerMaxImages={}",
                match side {
                    NativeCameraSide::Left => "left",
                    NativeCameraSide::Right => "right",
                },
                side_count,
                timestamp_ns,
                info.camera_id,
                self.reader_max_images
            ));
        }
        let frame = NativeCameraFrame {
            width: info.width,
            height: info.height,
            timestamp_ns,
            hardware_buffer,
        };

        if self.single_camera_mirror {
            let right = NativeCameraFrame {
                width: frame.width,
                height: frame.height,
                timestamp_ns: frame.timestamp_ns,
                hardware_buffer: frame.hardware_buffer.clone(),
            };
            self.publish_queue.replace(NativePendingPair {
                left: frame,
                right,
                pair_delta_ns: 0,
            });
            return Ok(());
        }

        let pair = {
            let mut state = self
                .pair_state
                .lock()
                .map_err(|_| "native pair state mutex poisoned".to_string())?;
            match side {
                NativeCameraSide::Left => state.left = Some(frame),
                NativeCameraSide::Right => state.right = Some(frame),
            }

            let (Some(left), Some(right)) = (state.left.as_ref(), state.right.as_ref()) else {
                return Ok(());
            };
            let pair_key = (left.timestamp_ns, right.timestamp_ns);
            if state.last_published_pair == Some(pair_key) {
                return Ok(());
            }

            let pair_delta = left.timestamp_ns.abs_diff(right.timestamp_ns);
            if pair_delta > self.pair_max_delta_ns
                && !self.logged_pair_delta.swap(true, Ordering::Relaxed)
            {
                log_info(format!(
                    "Rusty XR native ACamera pair exceeded soft timestamp target deltaNs={} softTargetNs={} publishingLatestPair=true",
                    pair_delta,
                    self.pair_max_delta_ns
                ));
            }

            state.last_published_pair = Some(pair_key);
            let left = state.left.take().unwrap();
            let right = state.right.take().unwrap();
            (left, right, pair_delta)
        };

        self.publish_queue.replace(NativePendingPair {
            left: pair.0,
            right: pair.1,
            pair_delta_ns: pair.2,
        });
        Ok(())
    }

    fn run_publish_worker(self: Arc<Self>, stop: Arc<AtomicBool>) {
        while self.alive.load(Ordering::Acquire) || !stop.load(Ordering::Acquire) {
            let Some(pair) = self.publish_queue.take_blocking(&stop) else {
                if stop.load(Ordering::Acquire) {
                    break;
                }
                continue;
            };
            if let Err(error) = self.store_pair(pair) {
                log_error(format!("Rusty XR native ACamera frame rejected: {error}"));
            }
        }

        while let Some(pair) = self.publish_queue.take_nowait() {
            if let Err(error) = self.store_pair(pair) {
                log_error(format!("Rusty XR native ACamera frame rejected: {error}"));
            }
        }
    }

    fn store_pair(&self, pair: NativePendingPair) -> Result<(), String> {
        let left_import =
            import_hardware_buffer(pair.left.width, pair.left.height, pair.left.hardware_buffer)?;
        let right_import = import_hardware_buffer(
            pair.right.width,
            pair.right.height,
            pair.right.hardware_buffer,
        )?;
        let left_metadata_json = self.metadata_json(&self.left_info, pair.left.timestamp_ns);
        let right_metadata_json = self.metadata_json(&self.right_info, pair.right.timestamp_ns);
        let pair_index = self.pair_index.fetch_add(1, Ordering::Relaxed);
        if !self.logged_first_pair.swap(true, Ordering::Relaxed)
            || pair_index < 5
            || pair_index.is_multiple_of(30)
        {
            log_info(format!(
                "Rusty XR native ACamera publishing stereo pair index={} leftTs={} rightTs={} deltaNs={} leftId={} rightId={} size={}x{} leftReceived={} rightReceived={} readerMaxImages={}",
                pair_index,
                pair.left.timestamp_ns,
                pair.right.timestamp_ns,
                pair.pair_delta_ns,
                self.left_info.camera_id,
                self.right_info.camera_id,
                pair.left.width,
                pair.left.height,
                self.left_received_count.load(Ordering::Relaxed),
                self.right_received_count.load(Ordering::Relaxed),
                self.reader_max_images
            ));
        }

        let accepted = store_headset_stereo_camera_gpu_frame(
            pair.left.width,
            pair.left.height,
            pair.left.timestamp_ns,
            Some(left_metadata_json),
            left_import,
            pair.right.width,
            pair.right.height,
            pair.right.timestamp_ns,
            Some(right_metadata_json),
            right_import,
            pair.pair_delta_ns,
            pair_index,
        );
        if !accepted {
            return Err("native stereo pair was rejected by renderer state".to_string());
        }
        Ok(())
    }

    fn metadata_json(&self, info: &NativeCameraSideInfo, timestamp_ns: i64) -> String {
        let has_intrinsics = info.intrinsics.is_some() && info.intrinsics_domain.is_some();
        let has_pose = info.pose_translation.is_some()
            && info.pose_rotation.is_some()
            && info
                .pose_reference
                .map(is_accepted_pose_reference)
                .unwrap_or(false);
        let metadata = NativeFrameMetadata {
            source_label: format!(
                "Native ACamera {} {}",
                info.camera_id,
                if info.left_eye { "left" } else { "right" }
            ),
            camera_id: info.camera_id.clone(),
            eye: if info.left_eye { "left" } else { "right" },
            lens_facing: info.lens_facing_label(),
            lens_facing_rank: info.lens_facing_rank(),
            selection_score: info.selection_score,
            delivered_width: info.width,
            delivered_height: info.height,
            timestamp_ns,
            sensor_orientation_degrees: info.sensor_orientation_degrees,
            stereo_layout: "separate",
            requested_stereo_layout: self.requested_stereo_layout.as_str(),
            transport: "ndk-ahardwarebuffer",
            requested_tier: self.requested_tier.as_str(),
            active_tier: "gpu-projected",
            gpu_import_requested: true,
            missing_intrinsics: !has_intrinsics,
            missing_pose: !has_pose,
            pose_source: if has_pose { "platform" } else { "missing" },
            pose_coordinate_convention: "android-camera2-lens-pose-reference-from-camera",
            lens_pose_reference_label: info.pose_reference.map(pose_reference_label),
            extrinsics: if has_pose {
                info.pose_translation
                    .zip(info.pose_rotation)
                    .map(|(p, q)| NativeExtrinsics {
                        px: p[0],
                        py: p[1],
                        pz: p[2],
                        qx: q[0],
                        qy: q[1],
                        qz: q[2],
                        qw: q[3],
                    })
            } else {
                None
            },
            mono_fallback: false,
            fallback_reason: if has_pose {
                "native-ndk-platform-metadata"
            } else {
                "native-ndk-missing-projection-metadata"
            },
            intrinsics: info.intrinsics,
            intrinsics_domain: info.intrinsics_domain,
            active_array_domain: info.active_array_domain,
            sensor_pixel_domain: info.sensor_pixel_domain,
        };
        serde_json::to_string(&metadata).unwrap_or_else(|_| "{}".to_string())
    }
}

#[derive(Default)]
struct NativePairState {
    left: Option<NativeCameraFrame>,
    right: Option<NativeCameraFrame>,
    last_published_pair: Option<(i64, i64)>,
}

struct NativeCameraFrame {
    width: u32,
    height: u32,
    timestamp_ns: i64,
    hardware_buffer: AndroidHardwareBufferHandle,
}

struct NativePendingPair {
    left: NativeCameraFrame,
    right: NativeCameraFrame,
    pair_delta_ns: u64,
}

#[derive(Default)]
struct NativePublishQueue {
    pending: Mutex<Option<NativePendingPair>>,
    condition: Condvar,
}

impl NativePublishQueue {
    fn replace(&self, pair: NativePendingPair) {
        if let Ok(mut pending) = self.pending.lock() {
            *pending = Some(pair);
            self.condition.notify_one();
        }
    }

    fn take_nowait(&self) -> Option<NativePendingPair> {
        self.pending.lock().ok()?.take()
    }

    fn take_blocking(&self, stop: &AtomicBool) -> Option<NativePendingPair> {
        let mut pending = self.pending.lock().ok()?;
        loop {
            if pending.is_some() {
                return pending.take();
            }
            if stop.load(Ordering::Acquire) {
                return None;
            }
            let result = self
                .condition
                .wait_timeout(pending, Duration::from_millis(50))
                .ok()?;
            pending = result.0;
        }
    }

    fn notify(&self) {
        self.condition.notify_all();
    }
}

#[derive(Clone)]
struct NativeCameraSource {
    camera_id: String,
    camera_id_c: CString,
    lens_facing: u8,
    logical_multi_camera: bool,
    physical_camera_ids: Vec<String>,
    sensor_sync_type: Option<u8>,
    sensor_orientation_degrees: Option<i32>,
    private_sizes: Vec<(u32, u32)>,
    active_array_domain: Option<NativePixelDomain>,
    sensor_pixel_domain: Option<NativePixelDomain>,
    intrinsics: Option<NativeIntrinsics>,
    intrinsics_domain: Option<NativePixelDomain>,
    pose_translation: Option<[f32; 3]>,
    pose_rotation: Option<[f32; 4]>,
    pose_reference: Option<u8>,
}

#[derive(Clone)]
struct NativeCameraSideInfo {
    camera_id: String,
    lens_facing: u8,
    sensor_orientation_degrees: Option<i32>,
    width: u32,
    height: u32,
    active_array_domain: Option<NativePixelDomain>,
    sensor_pixel_domain: Option<NativePixelDomain>,
    intrinsics: Option<NativeIntrinsics>,
    intrinsics_domain: Option<NativePixelDomain>,
    pose_translation: Option<[f32; 3]>,
    pose_rotation: Option<[f32; 4]>,
    pose_reference: Option<u8>,
    selection_score: i64,
    left_eye: bool,
}

impl NativeCameraSideInfo {
    fn from_source(source: &NativeCameraSource, width: u32, height: u32, left_eye: bool) -> Self {
        Self {
            camera_id: source.camera_id.clone(),
            lens_facing: source.lens_facing,
            sensor_orientation_degrees: source.sensor_orientation_degrees,
            width,
            height,
            active_array_domain: source.active_array_domain,
            sensor_pixel_domain: source.sensor_pixel_domain,
            intrinsics: source.intrinsics,
            intrinsics_domain: source.intrinsics_domain,
            pose_translation: source.pose_translation,
            pose_rotation: source.pose_rotation,
            pose_reference: source.pose_reference,
            selection_score: score_size(
                width,
                height,
                &NativeCameraConfig {
                    width: Some(width),
                    height: Some(height),
                    max_dimension: Some(width.max(height)),
                    preferred_square: None,
                    reader_max_images: None,
                    stereo_pair_max_delta_ns: None,
                    requested_tier: None,
                    requested_stereo_layout: None,
                    source_mode: None,
                    left_camera_id: None,
                    right_camera_id: None,
                },
            ),
            left_eye,
        }
    }

    fn lens_facing_label(&self) -> &'static str {
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
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeIntrinsics {
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
    skew: f32,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePixelDomain {
    kind: &'static str,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeExtrinsics {
    px: f32,
    py: f32,
    pz: f32,
    qx: f32,
    qy: f32,
    qz: f32,
    qw: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeFrameMetadata<'a> {
    source_label: String,
    camera_id: String,
    eye: &'a str,
    lens_facing: &'a str,
    lens_facing_rank: i32,
    selection_score: i64,
    delivered_width: u32,
    delivered_height: u32,
    timestamp_ns: i64,
    sensor_orientation_degrees: Option<i32>,
    stereo_layout: &'a str,
    requested_stereo_layout: &'a str,
    transport: &'a str,
    requested_tier: &'a str,
    active_tier: &'a str,
    gpu_import_requested: bool,
    missing_intrinsics: bool,
    missing_pose: bool,
    pose_source: &'a str,
    pose_coordinate_convention: &'a str,
    lens_pose_reference_label: Option<&'static str>,
    extrinsics: Option<NativeExtrinsics>,
    mono_fallback: bool,
    fallback_reason: &'a str,
    intrinsics: Option<NativeIntrinsics>,
    intrinsics_domain: Option<NativePixelDomain>,
    active_array_domain: Option<NativePixelDomain>,
    sensor_pixel_domain: Option<NativePixelDomain>,
}

unsafe extern "C" fn image_on_image_available(context: *mut c_void, reader: *mut AImageReader) {
    let reader_context = &*(context as *mut NativeReaderContext);
    if !reader_context.context.alive.load(Ordering::Acquire) {
        return;
    }

    let mut image = ptr::null_mut();
    if AImageReader_acquireLatestImage(reader, &mut image) != 0 || image.is_null() {
        return;
    }

    let mut timestamp_ns = 0i64;
    let _ = AImage_getTimestamp(image, &mut timestamp_ns);
    let mut hardware_buffer = ptr::null_mut();
    if AImage_getHardwareBuffer(image, &mut hardware_buffer) == 0 && !hardware_buffer.is_null() {
        let hardware_buffer = match AndroidHardwareBufferHandle::acquire(hardware_buffer) {
            Ok(handle) => handle,
            Err(error) => {
                AImage_delete(image);
                log_error(format!(
                    "Rusty XR native ACamera buffer acquire failed: {error}"
                ));
                return;
            }
        };
        AImage_delete(image);
        if let Err(error) =
            reader_context
                .context
                .publish_frame(reader_context.side, timestamp_ns, hardware_buffer)
        {
            log_error(format!("Rusty XR native ACamera frame rejected: {error}"));
        }
    } else {
        AImage_delete(image);
        log_error("Rusty XR native ACamera frame did not expose AHardwareBuffer");
    }
}

unsafe extern "C" fn device_on_disconnected(_context: *mut c_void, _device: *mut ACameraDevice) {
    log_info("Rusty XR native ACamera device disconnected");
}

unsafe extern "C" fn device_on_error(
    _context: *mut c_void,
    _device: *mut ACameraDevice,
    error: i32,
) {
    log_error(format!("Rusty XR native ACamera device error {error}"));
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
    log_info("Rusty XR native ACamera capture session active");
}

unsafe fn select_stereo_sources(
    manager: *mut ACameraManager,
    sources: &[NativeCameraSource],
    config: &NativeCameraConfig,
) -> Result<(NativeCameraSource, NativeCameraSource, (u32, u32)), String> {
    let left = if let Some(id) = config
        .left_camera_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        select_explicit_source(manager, sources, id, "left")?
    } else {
        select_default_side(sources, true)?
    };
    let right = if let Some(id) = config
        .right_camera_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        select_explicit_source(manager, sources, id, "right")?
    } else {
        select_default_side(sources, false)?
    };

    if left.camera_id == right.camera_id {
        return Err("native stereo source selection resolved to the same camera twice".to_string());
    }

    let shared_size = select_shared_size(&left.private_sizes, &right.private_sizes, config)
        .ok_or_else(|| {
            format!(
                "native stereo sources have no shared PRIVATE size left={} right={}",
                left.camera_id, right.camera_id
            )
        })?;
    let baseline_meters = stereo_baseline_meters(left.pose_translation, right.pose_translation);
    let selection_kind = if config
        .left_camera_id
        .as_deref()
        .map(|value| !value.is_empty())
        .unwrap_or(false)
        || config
            .right_camera_id
            .as_deref()
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    {
        "explicit-dual-side"
    } else {
        "synthetic-dual-back"
    };
    log_info(format!(
        "Rusty XR native ACamera selected stereo side sources selectionKind={} sourceMode={} left={} right={} size={}x{} sensorSync=approximate baselineMeters={}",
        selection_kind,
        config.source_mode(),
        left.camera_id,
        right.camera_id,
        shared_size.0,
        shared_size.1,
        baseline_meters
            .map(|value| format!("{value:.5}"))
            .unwrap_or_else(|| "missing".to_string())
    ));
    Ok((left, right, shared_size))
}

unsafe fn select_single_mirror_source(
    manager: *mut ACameraManager,
    sources: &[NativeCameraSource],
    config: &NativeCameraConfig,
) -> Result<(NativeCameraSource, (u32, u32)), String> {
    let source = if let Some(id) = config
        .left_camera_id
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        select_explicit_source(manager, sources, id, "mirror")?
    } else {
        select_default_side(sources, true)?
    };
    let selected_size = source
        .private_sizes
        .iter()
        .copied()
        .max_by_key(|(width, height)| score_size(*width, *height, config))
        .ok_or_else(|| {
            format!(
                "native mirror camera source has no PRIVATE size cameraId={}",
                source.camera_id
            )
        })?;
    log_info(format!(
        "Rusty XR native ACamera selected single mirror source selectionKind=single-back-mirror sourceMode={} cameraId={} size={}x{}",
        config.source_mode(),
        source.camera_id,
        selected_size.0,
        selected_size.1,
    ));
    Ok((source, selected_size))
}

unsafe fn select_explicit_source(
    manager: *mut ACameraManager,
    sources: &[NativeCameraSource],
    camera_id: &str,
    role: &str,
) -> Result<NativeCameraSource, String> {
    if let Some(source) = sources
        .iter()
        .find(|source| source.camera_id == camera_id)
        .cloned()
    {
        return Ok(source);
    }

    log_info(format!(
        "Rusty XR native ACamera explicit {} cameraId={} not returned by camera id list; probing characteristics directly",
        role, camera_id
    ));
    let source = load_camera_source_by_id(manager, camera_id)
        .map_err(|error| {
            format!(
                "native {} camera id not found: {}; explicit unlisted probe failed: {}",
                role, camera_id, error
            )
        })?
        .ok_or_else(|| {
            format!(
                "native {} camera id not found: {}; explicit unlisted probe had incomplete metadata",
                role, camera_id
            )
        })?;
    log_info(format!(
        "Rusty XR native ACamera accepted explicit unlisted {} cameraId={}",
        role, camera_id
    ));
    log_native_camera_source(&source);
    Ok(source)
}

fn select_default_side(
    sources: &[NativeCameraSource],
    left: bool,
) -> Result<NativeCameraSource, String> {
    let mut back = sources
        .iter()
        .filter(|source| source.lens_facing == ACAMERA_LENS_FACING_BACK)
        .filter(|source| !source.private_sizes.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    back.sort_by(|a, b| {
        let ax = a.pose_translation.map(|pose| pose[0]).unwrap_or(0.0);
        let bx = b.pose_translation.map(|pose| pose[0]).unwrap_or(0.0);
        ax.partial_cmp(&bx)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.camera_id.cmp(&b.camera_id))
    });
    if back.len() < 2 {
        return Err(
            "native ACamera could not find two back-facing PRIVATE camera sources".to_string(),
        );
    }
    if left {
        Ok(back.first().cloned().unwrap())
    } else {
        Ok(back.last().cloned().unwrap())
    }
}

fn select_shared_size(
    left_sizes: &[(u32, u32)],
    right_sizes: &[(u32, u32)],
    config: &NativeCameraConfig,
) -> Option<(u32, u32)> {
    let right_set = right_sizes.iter().copied().collect::<BTreeSet<_>>();
    left_sizes
        .iter()
        .copied()
        .filter(|size| right_set.contains(size))
        .max_by_key(|(width, height)| score_size(*width, *height, config))
}

fn score_size(width: u32, height: u32, config: &NativeCameraConfig) -> i64 {
    let target_width = config.requested_width() as i64;
    let target_height = config.requested_height() as i64;
    let mut score = -((width as i64 - target_width).abs() * 100_000
        + (height as i64 - target_height).abs() * 100_000);
    if width == config.requested_width() && height == config.requested_height() {
        score += 10_000_000_000;
    }
    if let Some(preferred_square) = config.preferred_square {
        if preferred_square > 0 && width == preferred_square && height == preferred_square {
            score += 1_000_000_000;
        }
    }
    if width == height && config.requested_width() == config.requested_height() {
        score += 100_000_000;
    }
    if width > config.max_dimension() || height > config.max_dimension() {
        score -= 2_000_000_000;
    }
    score + (width as i64 * height as i64)
}

unsafe fn enumerate_camera_sources(
    manager: *mut ACameraManager,
) -> Result<Vec<NativeCameraSource>, String> {
    let mut camera_ids_ptr = ptr::null_mut();
    if ACameraManager_getCameraIdList(manager, &mut camera_ids_ptr) != 0 || camera_ids_ptr.is_null()
    {
        return Err("ACameraManager_getCameraIdList failed".to_string());
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
        match load_camera_source_by_id(manager, &camera_id) {
            Ok(Some(source)) => {
                log_native_camera_source(&source);
                sources.push(source);
            }
            Ok(None) => {}
            Err(_) => {}
        }
    }

    ACameraManager_deleteCameraIdList(camera_ids_ptr);
    if sources.is_empty() {
        return Err("native ACamera enumeration returned no usable sources".to_string());
    }
    Ok(sources)
}

unsafe fn load_camera_source_by_id(
    manager: *mut ACameraManager,
    camera_id: &str,
) -> Result<Option<NativeCameraSource>, String> {
    let camera_id_c = CString::new(camera_id)
        .map_err(|_| format!("camera id contains interior null: {camera_id:?}"))?;
    let mut metadata = ptr::null_mut();
    let result =
        ACameraManager_getCameraCharacteristics(manager, camera_id_c.as_ptr(), &mut metadata);
    if result != 0 || metadata.is_null() {
        return Err(format!(
            "ACameraManager_getCameraCharacteristics failed cameraId={} result={} metadataNull={}",
            camera_id,
            result,
            metadata.is_null()
        ));
    }
    let source = camera_source_from_metadata(camera_id.to_string(), camera_id_c, metadata);
    ACameraMetadata_free(metadata);
    Ok(source)
}

unsafe fn camera_source_from_metadata(
    camera_id: String,
    camera_id_c: CString,
    metadata: *const ACameraMetadata,
) -> Option<NativeCameraSource> {
    let lens_facing = metadata_u8(metadata, ACAMERA_LENS_FACING)?;
    let capabilities = metadata_u8_vec(metadata, ACAMERA_REQUEST_AVAILABLE_CAPABILITIES);
    let private_sizes = metadata_private_output_sizes(metadata);
    let physical_camera_ids =
        metadata_string_list(metadata, ACAMERA_LOGICAL_MULTI_CAMERA_PHYSICAL_IDS);
    let sensor_sync_type = metadata_u8(metadata, ACAMERA_LOGICAL_MULTI_CAMERA_SENSOR_SYNC_TYPE);
    let active_array_domain = metadata_rect_domain(
        metadata,
        ACAMERA_SENSOR_INFO_ACTIVE_ARRAY_SIZE,
        "activeArray",
    );
    let sensor_pixel_domain = metadata_size_domain(
        metadata,
        ACAMERA_SENSOR_INFO_PIXEL_ARRAY_SIZE,
        "sensorPixelArray",
    );
    let intrinsics = metadata_intrinsics(metadata);
    let intrinsics_domain = active_array_domain.or(sensor_pixel_domain);
    let pose_translation = metadata_vec3(metadata, ACAMERA_LENS_POSE_TRANSLATION);
    let pose_rotation = metadata_quat(metadata, ACAMERA_LENS_POSE_ROTATION);
    let pose_reference = metadata_u8(metadata, ACAMERA_LENS_POSE_REFERENCE);

    Some(NativeCameraSource {
        camera_id,
        camera_id_c,
        lens_facing,
        logical_multi_camera: capabilities.iter().any(|capability| {
            *capability == ACAMERA_REQUEST_AVAILABLE_CAPABILITIES_LOGICAL_MULTI_CAMERA
        }),
        physical_camera_ids,
        sensor_sync_type,
        sensor_orientation_degrees: metadata_i32(metadata, ACAMERA_SENSOR_ORIENTATION),
        private_sizes,
        active_array_domain,
        sensor_pixel_domain,
        intrinsics,
        intrinsics_domain,
        pose_translation,
        pose_rotation,
        pose_reference,
    })
}

fn log_native_camera_source(source: &NativeCameraSource) {
    log_info(format!(
        "Rusty XR native ACamera source cameraId={} facing={} logicalMultiCamera={} physicalIds={} sensorSync={} privateSizes={} poseX={} poseReference={}",
        source.camera_id,
        lens_facing_label(source.lens_facing),
        source.logical_multi_camera,
        format_string_list(&source.physical_camera_ids),
        source
            .sensor_sync_type
            .map(sensor_sync_type_label)
            .unwrap_or("missing"),
        format_sizes(&source.private_sizes),
        source
            .pose_translation
            .map(|pose| format!("{:.4}", pose[0]))
            .unwrap_or_else(|| "missing".to_string()),
        source
            .pose_reference
            .map(pose_reference_label)
            .unwrap_or("missing"),
    ));
}

fn format_string_list(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join("|")
    }
}

fn format_sizes(sizes: &[(u32, u32)]) -> String {
    if sizes.is_empty() {
        return "-".to_string();
    }
    let mut values = sizes
        .iter()
        .take(8)
        .map(|(width, height)| format!("{width}x{height}"))
        .collect::<Vec<_>>();
    if sizes.len() > values.len() {
        values.push(format!("...+{}", sizes.len() - values.len()));
    }
    values.join("|")
}

fn stereo_baseline_meters(
    left_pose: Option<[f32; 3]>,
    right_pose: Option<[f32; 3]>,
) -> Option<f32> {
    let left_pose = left_pose?;
    let right_pose = right_pose?;
    let dx = right_pose[0] - left_pose[0];
    let dy = right_pose[1] - left_pose[1];
    let dz = right_pose[2] - left_pose[2];
    Some((dx * dx + dy * dy + dz * dz).sqrt())
}

fn sensor_sync_type_label(value: u8) -> &'static str {
    match value {
        ACAMERA_LOGICAL_MULTI_CAMERA_SENSOR_SYNC_TYPE_APPROXIMATE => "approximate",
        ACAMERA_LOGICAL_MULTI_CAMERA_SENSOR_SYNC_TYPE_CALIBRATED => "calibrated",
        _ => "unknown",
    }
}

fn lens_facing_label(value: u8) -> &'static str {
    match value {
        ACAMERA_LENS_FACING_FRONT => "front",
        ACAMERA_LENS_FACING_BACK => "back",
        ACAMERA_LENS_FACING_EXTERNAL => "external",
        _ => "unknown",
    }
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

unsafe fn metadata_i32(metadata: *const ACameraMetadata, tag: u32) -> Option<i32> {
    let entry = metadata_entry(metadata, tag)?;
    (!entry.data.i32_.is_null()).then(|| *entry.data.i32_)
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

unsafe fn metadata_rect_domain(
    metadata: *const ACameraMetadata,
    tag: u32,
    kind: &'static str,
) -> Option<NativePixelDomain> {
    let entry = metadata_entry(metadata, tag)?;
    if entry.count < 4 || entry.data.i32_.is_null() {
        return None;
    }
    let values = std::slice::from_raw_parts(entry.data.i32_, entry.count as usize);
    let width = values[2].saturating_sub(values[0]) as u32;
    let height = values[3].saturating_sub(values[1]) as u32;
    (width > 0 && height > 0).then_some(NativePixelDomain {
        kind,
        width,
        height,
    })
}

unsafe fn metadata_size_domain(
    metadata: *const ACameraMetadata,
    tag: u32,
    kind: &'static str,
) -> Option<NativePixelDomain> {
    let entry = metadata_entry(metadata, tag)?;
    if entry.count < 2 || entry.data.i32_.is_null() {
        return None;
    }
    let values = std::slice::from_raw_parts(entry.data.i32_, entry.count as usize);
    let width = values[0].max(0) as u32;
    let height = values[1].max(0) as u32;
    (width > 0 && height > 0).then_some(NativePixelDomain {
        kind,
        width,
        height,
    })
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

fn import_hardware_buffer(
    width: u32,
    height: u32,
    hardware_buffer: AndroidHardwareBufferHandle,
) -> Result<HeadsetCameraGpuBufferImport, String> {
    let buffer = hardware_buffer.as_ptr();
    let mut desc = std::mem::MaybeUninit::<ndk_sys::AHardwareBuffer_Desc>::zeroed();
    unsafe {
        ndk_sys::AHardwareBuffer_describe(buffer, desc.as_mut_ptr());
    }
    let desc = unsafe { desc.assume_init() };
    let mut descriptor = CameraGpuBufferDescriptor::new(
        "Native ACamera PRIVATE AHardwareBuffer",
        ImageSize::new(width, height),
        "AHardwareBuffer",
    )
    .with_native_format(desc.format as u64)
    .with_usage_flags(desc.usage)
    .with_layer_count(desc.layers)
    .with_stride_px(desc.stride);
    let mut native_id = 0u64;
    let id_result = unsafe { ndk_sys::AHardwareBuffer_getId(buffer, &mut native_id) };
    if id_result == 0 && native_id != 0 {
        descriptor = descriptor.with_buffer_id(native_id);
    }
    Ok(HeadsetCameraGpuBufferImport {
        descriptor,
        hardware_buffer,
    })
}

fn is_accepted_pose_reference(value: u8) -> bool {
    matches!(
        value,
        ACAMERA_LENS_POSE_REFERENCE_PRIMARY_CAMERA | ACAMERA_LENS_POSE_REFERENCE_GYROSCOPE
    )
}

fn pose_reference_label(value: u8) -> &'static str {
    match value {
        ACAMERA_LENS_POSE_REFERENCE_PRIMARY_CAMERA => "PRIMARY_CAMERA",
        ACAMERA_LENS_POSE_REFERENCE_GYROSCOPE => "GYROSCOPE",
        ACAMERA_LENS_POSE_REFERENCE_UNDEFINED => "UNDEFINED",
        _ => "UNKNOWN",
    }
}
