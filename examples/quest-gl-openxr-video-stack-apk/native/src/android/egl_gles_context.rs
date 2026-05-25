use super::log_info;
use rusty_xr_quest_diagnostics::EglGlesContextStatus;
use std::{
    ffi::CStr,
    os::raw::{c_char, c_void},
    ptr,
};

pub(super) type EGLDisplay = *mut c_void;
pub(super) type EGLConfig = *mut c_void;
pub(super) type EGLContext = *mut c_void;
type EGLSurface = *mut c_void;
type EGLint = i32;
type EGLBoolean = u32;

const EGL_FALSE: EGLBoolean = 0;
const EGL_NO_DISPLAY: EGLDisplay = ptr::null_mut();
const EGL_NO_CONTEXT: EGLContext = ptr::null_mut();
const EGL_NO_SURFACE: EGLSurface = ptr::null_mut();
const EGL_DEFAULT_DISPLAY: *mut c_void = ptr::null_mut();
const EGL_OPENGL_ES_API: u32 = 0x30A0;
const EGL_NONE: EGLint = 0x3038;
const EGL_RED_SIZE: EGLint = 0x3024;
const EGL_GREEN_SIZE: EGLint = 0x3023;
const EGL_BLUE_SIZE: EGLint = 0x3022;
const EGL_ALPHA_SIZE: EGLint = 0x3021;
const EGL_DEPTH_SIZE: EGLint = 0x3025;
const EGL_STENCIL_SIZE: EGLint = 0x3026;
const EGL_SAMPLES: EGLint = 0x3031;
const EGL_SURFACE_TYPE: EGLint = 0x3033;
const EGL_RENDERABLE_TYPE: EGLint = 0x3040;
const EGL_WIDTH: EGLint = 0x3057;
const EGL_HEIGHT: EGLint = 0x3056;
const EGL_PBUFFER_BIT: EGLint = 0x0001;
const EGL_OPENGL_ES3_BIT: EGLint = 0x0040;
const EGL_CONTEXT_CLIENT_VERSION: EGLint = 0x3098;
const EGL_VENDOR: EGLint = 0x3053;

const GL_VENDOR: u32 = 0x1F00;
const GL_RENDERER: u32 = 0x1F01;
const GL_VERSION: u32 = 0x1F02;
const GL_EXTENSIONS: u32 = 0x1F03;
const GL_SHADING_LANGUAGE_VERSION: u32 = 0x8B8C;

#[link(name = "EGL")]
unsafe extern "C" {
    fn eglGetDisplay(display_id: *mut c_void) -> EGLDisplay;
    fn eglInitialize(display: EGLDisplay, major: *mut EGLint, minor: *mut EGLint) -> EGLBoolean;
    fn eglTerminate(display: EGLDisplay) -> EGLBoolean;
    fn eglBindAPI(api: u32) -> EGLBoolean;
    fn eglChooseConfig(
        display: EGLDisplay,
        attrib_list: *const EGLint,
        configs: *mut EGLConfig,
        config_size: EGLint,
        num_config: *mut EGLint,
    ) -> EGLBoolean;
    fn eglCreateContext(
        display: EGLDisplay,
        config: EGLConfig,
        share_context: EGLContext,
        attrib_list: *const EGLint,
    ) -> EGLContext;
    fn eglDestroyContext(display: EGLDisplay, context: EGLContext) -> EGLBoolean;
    fn eglCreatePbufferSurface(
        display: EGLDisplay,
        config: EGLConfig,
        attrib_list: *const EGLint,
    ) -> EGLSurface;
    fn eglDestroySurface(display: EGLDisplay, surface: EGLSurface) -> EGLBoolean;
    fn eglMakeCurrent(
        display: EGLDisplay,
        draw: EGLSurface,
        read: EGLSurface,
        context: EGLContext,
    ) -> EGLBoolean;
    fn eglGetConfigAttrib(
        display: EGLDisplay,
        config: EGLConfig,
        attribute: EGLint,
        value: *mut EGLint,
    ) -> EGLBoolean;
    fn eglQueryString(display: EGLDisplay, name: EGLint) -> *const c_char;
}

#[link(name = "GLESv3")]
unsafe extern "C" {
    fn glGetString(name: u32) -> *const u8;
}

pub(super) struct EglContext {
    pub(super) display: EGLDisplay,
    pub(super) config: EGLConfig,
    pub(super) context: EGLContext,
    surface: EGLSurface,
    status: EglGlesContextStatus,
}

impl EglContext {
    pub(super) fn create() -> Result<Self, String> {
        unsafe {
            let display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
            if display == EGL_NO_DISPLAY {
                return Err("eglGetDisplay returned EGL_NO_DISPLAY".to_string());
            }
            let mut major = 0;
            let mut minor = 0;
            if eglInitialize(display, &mut major, &mut minor) == EGL_FALSE {
                return Err("eglInitialize failed".to_string());
            }
            if eglBindAPI(EGL_OPENGL_ES_API) == EGL_FALSE {
                return Err("eglBindAPI(EGL_OPENGL_ES_API) failed".to_string());
            }

            let config_attribs = [
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_DEPTH_SIZE,
                0,
                EGL_STENCIL_SIZE,
                0,
                EGL_SURFACE_TYPE,
                EGL_PBUFFER_BIT,
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES3_BIT,
                EGL_NONE,
            ];
            let mut config: EGLConfig = ptr::null_mut();
            let mut config_count = 0;
            if eglChooseConfig(
                display,
                config_attribs.as_ptr(),
                &mut config,
                1,
                &mut config_count,
            ) == EGL_FALSE
                || config_count == 0
                || config.is_null()
            {
                return Err("eglChooseConfig failed for GLES3 pbuffer config".to_string());
            }

            let surface_attribs = [EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE];
            let surface = eglCreatePbufferSurface(display, config, surface_attribs.as_ptr());
            if surface == EGL_NO_SURFACE {
                return Err("eglCreatePbufferSurface failed".to_string());
            }

            let context_attribs = [EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE];
            let context =
                eglCreateContext(display, config, EGL_NO_CONTEXT, context_attribs.as_ptr());
            if context == EGL_NO_CONTEXT {
                return Err("eglCreateContext failed for OpenGL ES 3".to_string());
            }
            if eglMakeCurrent(display, surface, surface, context) == EGL_FALSE {
                return Err("eglMakeCurrent failed".to_string());
            }

            let egl_version = Some(format!("{major}.{minor}"));
            let gles_version = gl_string(GL_VERSION);
            let glsl_version = gl_string(GL_SHADING_LANGUAGE_VERSION);
            let vendor = gl_string(GL_VENDOR).or_else(|| egl_string(display, EGL_VENDOR));
            let renderer = gl_string(GL_RENDERER);
            let extensions = gl_string(GL_EXTENSIONS).unwrap_or_default();
            let status = EglGlesContextStatus {
                egl_version,
                gles_version,
                glsl_version,
                vendor,
                renderer,
                config_red_bits: config_attrib(display, config, EGL_RED_SIZE),
                config_green_bits: config_attrib(display, config, EGL_GREEN_SIZE),
                config_blue_bits: config_attrib(display, config, EGL_BLUE_SIZE),
                config_alpha_bits: config_attrib(display, config, EGL_ALPHA_SIZE),
                config_depth_bits: config_attrib(display, config, EGL_DEPTH_SIZE),
                config_stencil_bits: config_attrib(display, config, EGL_STENCIL_SIZE),
                config_samples: config_attrib(display, config, EGL_SAMPLES),
                egl_context_current: true,
                external_oes_supported: extensions.contains("GL_OES_EGL_image_external"),
            };
            log_info(format!(
                "Rusty XR EGL/GLES context egl={:?} gles={:?} renderer={:?} externalOesSupported={}",
                status.egl_version,
                status.gles_version,
                status.renderer,
                status.external_oes_supported
            ));

            Ok(Self {
                display,
                config,
                context,
                surface,
                status,
            })
        }
    }

    pub(super) fn status(&self) -> EglGlesContextStatus {
        self.status.clone()
    }

    pub(super) fn make_current(&self) -> Result<(), String> {
        unsafe {
            if eglMakeCurrent(self.display, self.surface, self.surface, self.context) == EGL_FALSE {
                return Err("eglMakeCurrent failed before render".to_string());
            }
        }
        Ok(())
    }
}

impl Drop for EglContext {
    fn drop(&mut self) {
        unsafe {
            let _ = eglMakeCurrent(self.display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
            if self.context != EGL_NO_CONTEXT {
                let _ = eglDestroyContext(self.display, self.context);
            }
            if self.surface != EGL_NO_SURFACE {
                let _ = eglDestroySurface(self.display, self.surface);
            }
            if self.display != EGL_NO_DISPLAY {
                let _ = eglTerminate(self.display);
            }
        }
    }
}

fn gl_string(name: u32) -> Option<String> {
    unsafe {
        let value = glGetString(name);
        if value.is_null() {
            None
        } else {
            Some(
                CStr::from_ptr(value.cast::<c_char>())
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
}

fn egl_string(display: EGLDisplay, name: EGLint) -> Option<String> {
    unsafe {
        let value = eglQueryString(display, name);
        if value.is_null() {
            None
        } else {
            Some(CStr::from_ptr(value).to_string_lossy().into_owned())
        }
    }
}

fn config_attrib(display: EGLDisplay, config: EGLConfig, attribute: EGLint) -> Option<u8> {
    unsafe {
        let mut value = 0;
        if eglGetConfigAttrib(display, config, attribute, &mut value) == EGL_FALSE {
            None
        } else {
            u8::try_from(value).ok()
        }
    }
}
