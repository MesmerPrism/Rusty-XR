#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int, c_uint, c_void};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACameraManager {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACameraIdList {
    pub numCameras: c_int,
    pub cameraIds: *mut *const c_char,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACameraMetadata {
    _unused: [u8; 0],
}

pub const ACAMERA_LENS_FACING: u32 = 524_293;
pub const ACAMERA_LENS_POSE_ROTATION: u32 = 524_294;
pub const ACAMERA_LENS_POSE_TRANSLATION: u32 = 524_295;
pub const ACAMERA_LENS_INTRINSIC_CALIBRATION: u32 = 524_298;
pub const ACAMERA_LENS_POSE_REFERENCE: u32 = 524_300;
pub const ACAMERA_SENSOR_ORIENTATION: u32 = 393_217;
pub const ACAMERA_REQUEST_AVAILABLE_CAPABILITIES: u32 = 786_444;
pub const ACAMERA_SENSOR_INFO_ACTIVE_ARRAY_SIZE: u32 = 983_040;
pub const ACAMERA_SENSOR_INFO_PIXEL_ARRAY_SIZE: u32 = 983_046;
pub const ACAMERA_SCALER_AVAILABLE_STREAM_CONFIGURATIONS: u32 = 851_978;
pub const ACAMERA_LOGICAL_MULTI_CAMERA_PHYSICAL_IDS: u32 = 1_703_936;
pub const ACAMERA_LOGICAL_MULTI_CAMERA_SENSOR_SYNC_TYPE: u32 = 1_703_937;

pub const ACAMERA_LENS_FACING_FRONT: u8 = 0;
pub const ACAMERA_LENS_FACING_BACK: u8 = 1;
pub const ACAMERA_LENS_FACING_EXTERNAL: u8 = 2;
pub const ACAMERA_REQUEST_AVAILABLE_CAPABILITIES_LOGICAL_MULTI_CAMERA: u8 = 11;

pub const AIMAGE_FORMAT_PRIVATE: u32 = 0x22;
pub const AHARDWAREBUFFER_USAGE_GPU_SAMPLED_IMAGE: u64 = 1 << 8;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ACameraMetadata_const_entry {
    pub tag: u32,
    pub type_: u8,
    pub count: u32,
    pub data: ACameraMetadata_const_entry__bindgen_ty_1,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union ACameraMetadata_const_entry__bindgen_ty_1 {
    pub u8_: *const u8,
    pub i32_: *const i32,
    pub f: *const f32,
    pub i64_: *const i64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACameraDevice {
    _unused: [u8; 0],
}

pub type ACameraDevice_StateCallback =
    Option<unsafe extern "C" fn(context: *mut c_void, device: *mut ACameraDevice)>;

pub type ACameraDevice_ErrorStateCallback =
    Option<unsafe extern "C" fn(context: *mut c_void, device: *mut ACameraDevice, error: c_int)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACameraDevice_StateCallbacks {
    pub context: *mut c_void,
    pub onDisconnected: ACameraDevice_StateCallback,
    pub onError: ACameraDevice_ErrorStateCallback,
}

pub type ACameraDevice_request_template = c_uint;
pub const TEMPLATE_PREVIEW: ACameraDevice_request_template = 1;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACaptureRequest {
    _unused: [u8; 0],
}

pub type ANativeWindow = ndk_sys::ANativeWindow;
pub type AHardwareBuffer = ndk_sys::AHardwareBuffer;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACaptureSessionOutput {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACaptureSessionOutputContainer {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AImageReader {
    _unused: [u8; 0],
}

pub type camera_status_t = c_int;
pub type media_status_t = c_int;
pub type ACameraWindowType = ANativeWindow;

pub type AImageReader_ImageCallback =
    Option<unsafe extern "C" fn(context: *mut c_void, reader: *mut AImageReader)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AImageReader_ImageListener {
    pub context: *mut c_void,
    pub onImageAvailable: AImageReader_ImageCallback,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACameraOutputTarget {
    _unused: [u8; 0],
}

pub type ACameraCaptureSession_stateCallback =
    Option<unsafe extern "C" fn(context: *mut c_void, session: *mut ACameraCaptureSession)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACameraCaptureSession_stateCallbacks {
    pub context: *mut c_void,
    pub onClosed: ACameraCaptureSession_stateCallback,
    pub onReady: ACameraCaptureSession_stateCallback,
    pub onActive: ACameraCaptureSession_stateCallback,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ACameraCaptureSession {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct AImage {
    _unused: [u8; 0],
}

#[link(name = "nativewindow")]
extern "C" {
    pub fn ANativeWindow_acquire(window: *mut ANativeWindow);
    pub fn ANativeWindow_release(window: *mut ANativeWindow);
}

#[link(name = "mediandk")]
extern "C" {
    pub fn AImageReader_newWithUsage(
        width: i32,
        height: i32,
        format: u32,
        usage: u64,
        maxImages: i32,
        reader: *mut *mut AImageReader,
    ) -> media_status_t;

    pub fn AImageReader_setImageListener(
        reader: *mut AImageReader,
        listener: *mut AImageReader_ImageListener,
    ) -> media_status_t;

    pub fn AImageReader_getWindow(
        reader: *mut AImageReader,
        window: *mut *mut ANativeWindow,
    ) -> media_status_t;

    pub fn AImageReader_delete(reader: *mut AImageReader);

    pub fn AImageReader_acquireLatestImage(
        reader: *mut AImageReader,
        image: *mut *mut AImage,
    ) -> media_status_t;

    pub fn AImage_getTimestamp(image: *const AImage, timestampNs: *mut i64) -> media_status_t;

    pub fn AImage_getHardwareBuffer(
        image: *const AImage,
        buffer: *mut *mut AHardwareBuffer,
    ) -> media_status_t;

    pub fn AImage_delete(image: *mut AImage);
}

#[link(name = "camera2ndk")]
extern "C" {
    pub fn ACameraManager_create() -> *mut ACameraManager;
    pub fn ACameraManager_delete(manager: *mut ACameraManager);
    pub fn ACameraManager_getCameraIdList(
        manager: *mut ACameraManager,
        cameraIdList: *mut *mut ACameraIdList,
    ) -> camera_status_t;
    pub fn ACameraManager_deleteCameraIdList(cameraIdList: *mut ACameraIdList);
    pub fn ACameraManager_getCameraCharacteristics(
        manager: *mut ACameraManager,
        cameraId: *const c_char,
        characteristics: *mut *mut ACameraMetadata,
    ) -> camera_status_t;
    pub fn ACameraMetadata_free(metadata: *mut ACameraMetadata);
    pub fn ACameraMetadata_getConstEntry(
        metadata: *const ACameraMetadata,
        tag: u32,
        entry: *mut ACameraMetadata_const_entry,
    ) -> camera_status_t;

    pub fn ACameraManager_openCamera(
        manager: *mut ACameraManager,
        cameraId: *const c_char,
        callback: *mut ACameraDevice_StateCallbacks,
        device: *mut *mut ACameraDevice,
    ) -> camera_status_t;

    pub fn ACameraDevice_createCaptureRequest(
        device: *const ACameraDevice,
        templateId: ACameraDevice_request_template,
        request: *mut *mut ACaptureRequest,
    ) -> camera_status_t;

    pub fn ACaptureSessionOutput_create(
        anw: *mut ACameraWindowType,
        output: *mut *mut ACaptureSessionOutput,
    ) -> camera_status_t;
    pub fn ACaptureSessionOutputContainer_create(
        container: *mut *mut ACaptureSessionOutputContainer,
    ) -> camera_status_t;
    pub fn ACaptureSessionOutputContainer_add(
        container: *mut ACaptureSessionOutputContainer,
        output: *const ACaptureSessionOutput,
    ) -> camera_status_t;
    pub fn ACaptureSessionOutputContainer_free(container: *mut ACaptureSessionOutputContainer);
    pub fn ACaptureSessionOutput_free(output: *mut ACaptureSessionOutput);

    pub fn ACameraOutputTarget_create(
        window: *mut ACameraWindowType,
        output: *mut *mut ACameraOutputTarget,
    ) -> camera_status_t;
    pub fn ACameraOutputTarget_free(output: *mut ACameraOutputTarget);
    pub fn ACaptureRequest_addTarget(
        request: *mut ACaptureRequest,
        output: *const ACameraOutputTarget,
    ) -> camera_status_t;
    pub fn ACaptureRequest_removeTarget(
        request: *mut ACaptureRequest,
        output: *const ACameraOutputTarget,
    ) -> camera_status_t;

    pub fn ACameraDevice_createCaptureSession(
        device: *mut ACameraDevice,
        outputs: *const ACaptureSessionOutputContainer,
        callbacks: *const ACameraCaptureSession_stateCallbacks,
        session: *mut *mut ACameraCaptureSession,
    ) -> camera_status_t;
    pub fn ACameraCaptureSession_setRepeatingRequest(
        session: *mut ACameraCaptureSession,
        callbacks: *mut c_void,
        numRequests: c_int,
        requests: *mut *mut ACaptureRequest,
        captureSequenceId: *mut c_int,
    ) -> camera_status_t;
    pub fn ACameraCaptureSession_stopRepeating(
        session: *mut ACameraCaptureSession,
    ) -> camera_status_t;
    pub fn ACameraCaptureSession_close(session: *mut ACameraCaptureSession);
    pub fn ACameraDevice_close(device: *mut ACameraDevice) -> camera_status_t;
    pub fn ACaptureRequest_free(request: *mut ACaptureRequest);
}
