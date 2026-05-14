//! Native payload for the public Quest OpenXR/OpenGL ES video-stack example.
//!
//! This example proves an OpenXR session created through
//! `XR_KHR_opengl_es_enable`, renders static left/right diagnostic grids into
//! GL swapchains, and emits reusable public diagnostics payloads. It also
//! creates per-eye `SurfaceTexture` output surfaces backed by
//! `GL_TEXTURE_EXTERNAL_OES`, decodes broker-compatible H.264 streams into
//! those surfaces with Android MediaCodec, and updates the external textures
//! on the native GL render thread.

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use rusty_xr_quest_diagnostics::OpenXrGlesFeasibilityStatus;

pub fn status_json() -> String {
    let status = OpenXrGlesFeasibilityStatus::new();
    serde_json::to_string_pretty(&status).expect("OpenXR/GLES status should serialize")
}

#[cfg(target_os = "android")]
mod android {
    use super::*;
    use android_activity::{InputStatus, MainEvent, PollEvent};
    use jni::{
        objects::{GlobalRef, JClass, JObject, JString, JValue},
        sys::{jint, jlong, jobject},
        JNIEnv, JavaVM,
    };
    use openxr as xr;
    use openxr::sys::Handle as _;
    use rusty_xr_contracts::{
        Eye, InvalidProjectionFillPolicy, ProjectionFootprintRowSpan, ProjectionFootprintSummary,
        ProjectionGuideDomain, ProjectionStageKind, ProjectionStageTokenRow,
    };
    use rusty_xr_quest_diagnostics::{
        EglGlesContextStatus, FrameRateSummary, GlFramebufferCompleteness,
        OpenXrGlesFeasibilityState, OpenXrGlesGraphicsRequirements, OpenXrGlesSwapchainFormat,
        OpenXrGlesViewStatus, SurfaceTextureOesEyeStatus, SurfaceTextureOesIngestState,
        SurfaceTextureOesIngestStatus, OPENXR_GLES_EXTENSION,
    };
    use std::{
        ffi::{CStr, CString},
        mem,
        os::raw::{c_char, c_int, c_void},
        ptr,
        sync::{
            atomic::{AtomicI64, AtomicU64, Ordering},
            Mutex, OnceLock,
        },
        time::{Duration, Instant},
    };

    const VIEW_COUNT: usize = 2;
    const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;
    const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
    const GL_TRIANGLE_STRIP: u32 = 0x0005;
    const GL_SCISSOR_TEST: u32 = 0x0C11;
    const GL_TEXTURE_2D: u32 = 0x0DE1;
    const GL_FLOAT: u32 = 0x1406;
    const GL_VENDOR: u32 = 0x1F00;
    const GL_RENDERER: u32 = 0x1F01;
    const GL_VERSION: u32 = 0x1F02;
    const GL_EXTENSIONS: u32 = 0x1F03;
    const GL_SHADING_LANGUAGE_VERSION: u32 = 0x8B8C;
    const GL_RGBA: u32 = 0x1908;
    const GL_RGBA8: u32 = 0x8058;
    const GL_RGB10_A2: u32 = 0x8059;
    const GL_SRGB8_ALPHA8: u32 = 0x8C43;
    const GL_NO_ERROR: u32 = 0;
    const GL_TEXTURE_EXTERNAL_OES: u32 = 0x8D65;
    const GL_TEXTURE0: u32 = 0x84C0;
    const GL_ARRAY_BUFFER: u32 = 0x8892;
    const GL_STATIC_DRAW: u32 = 0x88E4;
    const GL_COMPILE_STATUS: u32 = 0x8B81;
    const GL_LINK_STATUS: u32 = 0x8B82;
    const GL_INFO_LOG_LENGTH: u32 = 0x8B84;
    const GL_VERTEX_SHADER: u32 = 0x8B31;
    const GL_FRAGMENT_SHADER: u32 = 0x8B30;
    const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
    const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
    const GL_TEXTURE_WRAP_S: u32 = 0x2802;
    const GL_TEXTURE_WRAP_T: u32 = 0x2803;
    const GL_LINEAR: u32 = 0x2601;
    const GL_CLAMP_TO_EDGE: u32 = 0x812F;
    const BROKER_H264_DEFAULT_HOST: &str = "127.0.0.1";
    const BROKER_H264_LEFT_STREAM_PORT: i32 = 8879;
    const BROKER_H264_RIGHT_STREAM_PORT: i32 = 8880;
    const BROKER_H264_MAX_PACKETS: i32 = 0;
    const BROKER_H264_CONNECT_TIMEOUT_MS: i32 = 5000;
    const BROKER_H264_DECODE_TIMEOUT_MS: i32 = 0;
    const GL_DEPTH_COMPONENT16: u32 = 0x81A5;
    const GL_DEPTH_COMPONENT24: u32 = 0x81A6;
    const GL_DEPTH24_STENCIL8: u32 = 0x88F0;
    const GL_FRAMEBUFFER: u32 = 0x8D40;
    const GL_COLOR_ATTACHMENT0: u32 = 0x8CE0;
    const GL_FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;
    const GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT: u32 = 0x8CD6;
    const GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT: u32 = 0x8CD7;
    const GL_FRAMEBUFFER_UNSUPPORTED: u32 = 0x8CDD;
    const GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE: u32 = 0x8D56;
    const DEFAULT_OES_SURFACE_WIDTH: i32 = 1280;
    const DEFAULT_OES_SURFACE_HEIGHT: i32 = 1280;
    const OES_COPY_RENDER_PATH: &str = "broker-h264-oes-full-surface-copy";

    static OES_DECODE_CALLBACKS: OnceLock<OesDecodeCallbackState> = OnceLock::new();

    struct OesDecodeCallbackState {
        frame_available_counts: [AtomicU64; VIEW_COUNT],
        latest_sequences: [AtomicU64; VIEW_COUNT],
        latest_queued_pts_us: [AtomicI64; VIEW_COUNT],
        report_sequence: AtomicU64,
        latest_report: Mutex<Option<String>>,
    }

    impl OesDecodeCallbackState {
        fn new() -> Self {
            Self {
                frame_available_counts: [AtomicU64::new(0), AtomicU64::new(0)],
                latest_sequences: [AtomicU64::new(0), AtomicU64::new(0)],
                latest_queued_pts_us: [AtomicI64::new(-1), AtomicI64::new(-1)],
                report_sequence: AtomicU64::new(0),
                latest_report: Mutex::new(None),
            }
        }

        fn reset(&self) {
            for index in 0..VIEW_COUNT {
                self.frame_available_counts[index].store(0, Ordering::Relaxed);
                self.latest_sequences[index].store(0, Ordering::Relaxed);
                self.latest_queued_pts_us[index].store(-1, Ordering::Relaxed);
            }
            self.report_sequence.store(0, Ordering::Relaxed);
            if let Ok(mut latest_report) = self.latest_report.lock() {
                *latest_report = None;
            }
        }

        fn mark_frame_available(&self, view_index: usize, sequence: u64, queued_pts_us: i64) {
            if view_index >= VIEW_COUNT {
                return;
            }
            self.latest_sequences[view_index].store(sequence, Ordering::Relaxed);
            self.latest_queued_pts_us[view_index].store(queued_pts_us, Ordering::Relaxed);
            self.frame_available_counts[view_index].fetch_add(1, Ordering::Relaxed);
        }

        fn frame_snapshot(&self, view_index: usize) -> (u64, u64, i64) {
            (
                self.frame_available_counts[view_index].load(Ordering::Relaxed),
                self.latest_sequences[view_index].load(Ordering::Relaxed),
                self.latest_queued_pts_us[view_index].load(Ordering::Relaxed),
            )
        }

        fn record_report(&self, report: String) {
            if let Ok(mut latest_report) = self.latest_report.lock() {
                *latest_report = Some(report);
                self.report_sequence.fetch_add(1, Ordering::Relaxed);
            }
        }

        fn latest_report_after(&self, last_seen_sequence: &mut u64) -> Option<String> {
            let sequence = self.report_sequence.load(Ordering::Relaxed);
            if sequence == *last_seen_sequence {
                return None;
            }
            *last_seen_sequence = sequence;
            self.latest_report
                .lock()
                .ok()
                .and_then(|report| report.clone())
        }
    }

    fn oes_decode_callbacks() -> &'static OesDecodeCallbackState {
        OES_DECODE_CALLBACKS.get_or_init(OesDecodeCallbackState::new)
    }

    type EGLDisplay = *mut c_void;
    type EGLConfig = *mut c_void;
    type EGLContext = *mut c_void;
    type EGLSurface = *mut c_void;
    type EGLBoolean = c_int;
    type EGLint = c_int;

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

    #[link(name = "EGL")]
    unsafe extern "C" {
        fn eglGetDisplay(display_id: *mut c_void) -> EGLDisplay;
        fn eglInitialize(display: EGLDisplay, major: *mut EGLint, minor: *mut EGLint)
            -> EGLBoolean;
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
        fn glViewport(x: c_int, y: c_int, width: c_int, height: c_int);
        fn glClearColor(red: f32, green: f32, blue: f32, alpha: f32);
        fn glClear(mask: u32);
        fn glEnable(cap: u32);
        fn glDisable(cap: u32);
        fn glScissor(x: c_int, y: c_int, width: c_int, height: c_int);
        fn glGenFramebuffers(n: c_int, framebuffers: *mut u32);
        fn glDeleteFramebuffers(n: c_int, framebuffers: *const u32);
        fn glBindFramebuffer(target: u32, framebuffer: u32);
        fn glFramebufferTexture2D(
            target: u32,
            attachment: u32,
            textarget: u32,
            texture: u32,
            level: c_int,
        );
        fn glCheckFramebufferStatus(target: u32) -> u32;
        fn glGenTextures(n: c_int, textures: *mut u32);
        fn glDeleteTextures(n: c_int, textures: *const u32);
        fn glBindTexture(target: u32, texture: u32);
        fn glTexParameteri(target: u32, pname: u32, param: c_int);
        fn glGetError() -> u32;
        fn glFlush();
        fn glActiveTexture(texture: u32);
        fn glCreateShader(shader_type: u32) -> u32;
        fn glShaderSource(
            shader: u32,
            count: c_int,
            string: *const *const c_char,
            length: *const c_int,
        );
        fn glCompileShader(shader: u32);
        fn glGetShaderiv(shader: u32, pname: u32, params: *mut c_int);
        fn glGetShaderInfoLog(
            shader: u32,
            buf_size: c_int,
            length: *mut c_int,
            info_log: *mut c_char,
        );
        fn glDeleteShader(shader: u32);
        fn glCreateProgram() -> u32;
        fn glAttachShader(program: u32, shader: u32);
        fn glLinkProgram(program: u32);
        fn glGetProgramiv(program: u32, pname: u32, params: *mut c_int);
        fn glGetProgramInfoLog(
            program: u32,
            buf_size: c_int,
            length: *mut c_int,
            info_log: *mut c_char,
        );
        fn glDeleteProgram(program: u32);
        fn glUseProgram(program: u32);
        fn glGetUniformLocation(program: u32, name: *const c_char) -> c_int;
        fn glUniform1i(location: c_int, v0: c_int);
        fn glGenBuffers(n: c_int, buffers: *mut u32);
        fn glDeleteBuffers(n: c_int, buffers: *const u32);
        fn glBindBuffer(target: u32, buffer: u32);
        fn glBufferData(target: u32, size: isize, data: *const c_void, usage: u32);
        fn glEnableVertexAttribArray(index: u32);
        fn glDisableVertexAttribArray(index: u32);
        fn glVertexAttribPointer(
            index: u32,
            size: c_int,
            type_: u32,
            normalized: u8,
            stride: c_int,
            pointer: *const c_void,
        );
        fn glDrawArrays(mode: u32, first: c_int, count: c_int);
    }

    #[no_mangle]
    fn android_on_create(_state: &android_activity::OnCreateState) {
        log_info("Rusty XR GLES NativeActivity created");
    }

    #[no_mangle]
    fn android_main(app: android_activity::AndroidApp) {
        log_info(format!("Rusty XR GLES source status: {}", status_json()));
        let app_for_error = app.clone();
        if let Err(error) = run(app) {
            log_error(format!("Rusty XR OpenXR GLES loop failed: {error}"));
            keep_activity_alive_after_error(app_for_error);
        }
    }

    fn run(app: android_activity::AndroidApp) -> Result<(), String> {
        let mut status = OpenXrGlesFeasibilityStatus::new();
        log_status(&status);

        let entry = unsafe { xr::Entry::load().map_err(|error| format!("load OpenXR: {error}"))? };
        initialize_android_loader(&entry, &app)?;
        let available_extensions = entry
            .enumerate_extensions()
            .map_err(|error| format!("enumerate OpenXR extensions: {error}"))?;
        status.state = OpenXrGlesFeasibilityState::ExtensionsEnumerated;
        status.required_extensions[0].available = available_extensions.khr_opengl_es_enable;
        log_info(format!(
            "Rusty XR OpenXR GLES extensions androidCreateInstance={} openGles={} displayRefresh={}",
            available_extensions.khr_android_create_instance,
            available_extensions.khr_opengl_es_enable,
            available_extensions.fb_display_refresh_rate
        ));
        log_status(&status);

        if !available_extensions.khr_opengl_es_enable {
            status.state = OpenXrGlesFeasibilityState::Failed;
            status
                .issue_codes
                .push(String::from("missing.XR_KHR_opengl_es_enable"));
            log_status(&status);
            return Err("OpenXR runtime does not expose XR_KHR_opengl_es_enable".to_string());
        }

        let mut enabled_extensions = xr::ExtensionSet::default();
        enabled_extensions.khr_android_create_instance = true;
        enabled_extensions.khr_opengl_es_enable = true;

        let xr_instance = unsafe {
            create_android_instance(
                &entry,
                &app,
                &xr::ApplicationInfo {
                    application_name: "Rusty XR GLES Stack",
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
        status.runtime_name = Some(properties.runtime_name.clone());
        status.runtime_version = Some(properties.runtime_version.to_string());
        log_info(format!(
            "Rusty XR OpenXR GLES runtime name={} version={}",
            properties.runtime_name, properties.runtime_version
        ));

        let system = xr_instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(|error| format!("get HMD system: {error}"))?;
        let environment_blend_mode =
            select_environment_blend_mode(&xr_instance, system, &mut status)?;
        let requirements = xr_instance
            .graphics_requirements::<xr::OpenGlEs>(system)
            .map_err(|error| format!("read OpenGL ES graphics requirements: {error}"))?;
        status.state = OpenXrGlesFeasibilityState::GraphicsRequirementsKnown;
        status.graphics_requirements = Some(OpenXrGlesGraphicsRequirements {
            min_api_version: Some(requirements.min_api_version_supported.to_string()),
            max_api_version: Some(requirements.max_api_version_supported.to_string()),
        });
        log_info(format!(
            "Rusty XR OpenXR GLES requirements min={} max={}",
            requirements.min_api_version_supported, requirements.max_api_version_supported
        ));

        let egl = EglContext::create()?;
        status.context = Some(egl.status());
        status.state = OpenXrGlesFeasibilityState::EglContextReady;
        log_status(&status);
        let mut surface_texture_oes_probe = probe_surface_texture_oes(&app, &egl);

        wait_for_android_foreground(&app)?;
        let (session, mut frame_wait, mut frame_stream) = unsafe {
            xr_instance
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
        log_status(&status);

        let stage = session
            .create_reference_space(xr::ReferenceSpaceType::LOCAL, xr::Posef::IDENTITY)
            .map_err(|error| format!("create LOCAL reference space: {error}"))?;
        let mut swapchains = create_eye_swapchains(&xr_instance, system, &session, &mut status)?;
        let mut fbo = GlFramebuffer::new();
        let mut oes_copy_renderer = match OesCopyRenderer::new() {
            Ok(renderer) => Some(renderer),
            Err(error) => {
                status
                    .issue_codes
                    .push(String::from("oes_copy_renderer_create_failed"));
                status.notes.push(format!(
                    "Could not create the public OES full-surface copy renderer: {error}"
                ));
                log_error(format!(
                    "Rusty XR OpenXR GLES OES copy renderer creation failed: {error}"
                ));
                None
            }
        };
        status.state = OpenXrGlesFeasibilityState::SwapchainsReady;
        log_status(&status);

        let mut event_storage = xr::EventDataBuffer::new();
        let mut app_running = true;
        let mut session_running = false;
        let mut frame_count = 0_u64;
        let mut frame_window_start = Instant::now();
        let mut frame_window_count = 0_u64;

        'main_loop: loop {
            pump_android_events(&app, &mut app_running);
            if !app_running {
                match session.request_exit() {
                    Ok(()) | Err(xr::sys::Result::ERROR_SESSION_NOT_RUNNING) => {}
                    Err(error) => {
                        log_error(format!("Rusty XR OpenXR GLES request_exit failed: {error}"))
                    }
                }
            }

            while let Some(event) = xr_instance
                .poll_event(&mut event_storage)
                .map_err(|error| format!("poll OpenXR event: {error}"))?
            {
                match event {
                    xr::Event::SessionStateChanged(event) => match event.state() {
                        xr::SessionState::READY => {
                            session
                                .begin(VIEW_TYPE)
                                .map_err(|error| format!("begin OpenXR session: {error}"))?;
                            session_running = true;
                            log_info("Rusty XR OpenXR GLES state READY -> running");
                        }
                        xr::SessionState::STOPPING => {
                            session
                                .end()
                                .map_err(|error| format!("end OpenXR session: {error}"))?;
                            session_running = false;
                            log_info("Rusty XR OpenXR GLES state STOPPING -> ended");
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            break 'main_loop;
                        }
                        state => {
                            log_info(format!("Rusty XR OpenXR GLES state {state:?}"));
                        }
                    },
                    xr::Event::InstanceLossPending(_) => break 'main_loop,
                    xr::Event::EventsLost(event) => {
                        log_error(format!(
                            "Rusty XR OpenXR GLES lost {} event(s)",
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

            let mut projection_views = Vec::new();
            if frame_state.should_render {
                let (_, views) = session
                    .locate_views(VIEW_TYPE, frame_state.predicted_display_time, &stage)
                    .map_err(|error| format!("locate OpenXR views: {error}"))?;
                if let Some(probe) = surface_texture_oes_probe.as_mut() {
                    probe.update_textures(&egl, frame_count);
                }
                render_eye_swapchains(
                    &egl,
                    &mut fbo,
                    &mut swapchains,
                    frame_count,
                    &mut status,
                    surface_texture_oes_probe.as_ref(),
                    &mut oes_copy_renderer,
                )?;

                for (index, eye) in swapchains.iter().enumerate() {
                    let Some(view) = views.get(index) else {
                        continue;
                    };
                    projection_views.push(
                        xr::CompositionLayerProjectionView::new()
                            .pose(view.pose)
                            .fov(view.fov)
                            .sub_image(
                                xr::SwapchainSubImage::new()
                                    .swapchain(&eye.handle)
                                    .image_array_index(0)
                                    .image_rect(xr::Rect2Di {
                                        offset: xr::Offset2Di { x: 0, y: 0 },
                                        extent: xr::Extent2Di {
                                            width: eye.width as i32,
                                            height: eye.height as i32,
                                        },
                                    }),
                            ),
                    );
                }
            }

            if projection_views.is_empty() {
                frame_stream
                    .end(
                        frame_state.predicted_display_time,
                        environment_blend_mode,
                        &[],
                    )
                    .map_err(|error| format!("end OpenXR frame without layers: {error}"))?;
            } else {
                let layer = xr::CompositionLayerProjection::new()
                    .space(&stage)
                    .views(&projection_views);
                frame_stream
                    .end(
                        frame_state.predicted_display_time,
                        environment_blend_mode,
                        &[&layer],
                    )
                    .map_err(|error| format!("end OpenXR frame: {error}"))?;
            }

            frame_count = frame_count.saturating_add(1);
            frame_window_count = frame_window_count.saturating_add(1);
            if status.state != OpenXrGlesFeasibilityState::Rendering && frame_count > 0 {
                status.state = OpenXrGlesFeasibilityState::Rendering;
            }
            if frame_count == 1 || frame_count.is_multiple_of(120) {
                let elapsed = frame_window_start.elapsed().as_secs_f32().max(0.001);
                let fps = frame_window_count as f32 / elapsed;
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
                log_status(&status);
                frame_window_start = Instant::now();
                frame_window_count = 0;
            }
        }

        log_info("Rusty XR OpenXR GLES loop exited cleanly");
        Ok(())
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

    fn ensure_xr_success(result: xr::sys::Result, operation: &str) -> Result<(), String> {
        if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
            return Err(format!("{operation} failed: {result:?}"));
        }
        Ok(())
    }

    fn select_environment_blend_mode(
        instance: &xr::Instance,
        system: xr::SystemId,
        status: &mut OpenXrGlesFeasibilityStatus,
    ) -> Result<xr::EnvironmentBlendMode, String> {
        let modes = instance
            .enumerate_environment_blend_modes(system, VIEW_TYPE)
            .map_err(|error| format!("enumerate environment blend modes: {error}"))?;
        let selected = modes
            .iter()
            .copied()
            .find(|mode| *mode == xr::EnvironmentBlendMode::OPAQUE)
            .or_else(|| modes.first().copied())
            .ok_or_else(|| "OpenXR runtime reported no environment blend modes".to_string())?;
        status.notes.push(format!(
            "environmentBlendModes={modes:?}; selected={selected:?}"
        ));
        log_info(format!(
            "Rusty XR OpenXR GLES environment blend modes available={modes:?} selected={selected:?}"
        ));
        Ok(selected)
    }

    fn create_eye_swapchains(
        instance: &xr::Instance,
        system: xr::SystemId,
        session: &xr::Session<xr::OpenGlEs>,
        status: &mut OpenXrGlesFeasibilityStatus,
    ) -> Result<Vec<EyeSwapchain>, String> {
        let view_configs = instance
            .enumerate_view_configuration_views(system, VIEW_TYPE)
            .map_err(|error| format!("enumerate view configuration views: {error}"))?;
        if view_configs.len() < VIEW_COUNT {
            return Err(format!(
                "OpenXR runtime reported {} view(s), expected at least {VIEW_COUNT}",
                view_configs.len()
            ));
        }

        let formats = session
            .enumerate_swapchain_formats()
            .map_err(|error| format!("enumerate OpenXR GLES swapchain formats: {error}"))?;
        let selected_format = select_color_format(&formats)
            .ok_or_else(|| "OpenXR runtime reported no GLES swapchain formats".to_string())?;
        status.swapchain_formats = formats
            .iter()
            .filter(|format| {
                **format == selected_format
                    || is_color_format(**format)
                    || is_depth_format(**format)
            })
            .map(|format| OpenXrGlesSwapchainFormat {
                format_id: *format as i64,
                label: gl_format_label(*format).to_string(),
                color_renderable: is_color_format(*format),
                depth_renderable: is_depth_format(*format),
                selected: *format == selected_format,
            })
            .collect();
        log_info(format!(
            "Rusty XR OpenXR GLES swapchain formats selected={} runtimeFormatCount={} trackedFormats={:?}",
            gl_format_label(selected_format),
            formats.len(),
            status
                .swapchain_formats
                .iter()
                .map(|format| format.label.as_str())
                .collect::<Vec<_>>()
        ));

        let mut swapchains = Vec::with_capacity(VIEW_COUNT);
        status.views.clear();
        for (index, view) in view_configs.iter().take(VIEW_COUNT).enumerate() {
            let width = view.recommended_image_rect_width;
            let height = view.recommended_image_rect_height;
            let handle = session
                .create_swapchain(&xr::SwapchainCreateInfo {
                    create_flags: xr::SwapchainCreateFlags::EMPTY,
                    usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                        | xr::SwapchainUsageFlags::SAMPLED,
                    format: selected_format,
                    sample_count: 1,
                    width,
                    height,
                    face_count: 1,
                    array_size: 1,
                    mip_count: 1,
                })
                .map_err(|error| {
                    format!("create OpenXR GLES swapchain for eye {index}: {error}")
                })?;
            let images = handle.enumerate_images().map_err(|error| {
                format!("enumerate GLES swapchain images for eye {index}: {error}")
            })?;
            let pattern = if index == 0 {
                "left-red-cyan-grid"
            } else {
                "right-blue-yellow-grid"
            };
            status.views.push(OpenXrGlesViewStatus::diagnostic_grid(
                index as u32,
                width,
                height,
                pattern,
            ));
            log_info(format!(
                "Rusty XR OpenXR GLES swapchain eye={} size={}x{} images={} sampleCountRecommended={}",
                index,
                width,
                height,
                images.len(),
                view.recommended_swapchain_sample_count
            ));
            swapchains.push(EyeSwapchain {
                handle,
                images,
                width,
                height,
                view_index: index,
                pattern,
            });
        }

        Ok(swapchains)
    }

    fn render_eye_swapchains(
        egl: &EglContext,
        fbo: &mut GlFramebuffer,
        swapchains: &mut [EyeSwapchain],
        frame_count: u64,
        status: &mut OpenXrGlesFeasibilityStatus,
        surface_texture_oes_probe: Option<&SurfaceTextureOesProbe>,
        oes_copy_renderer: &mut Option<OesCopyRenderer>,
    ) -> Result<(), String> {
        egl.make_current()?;
        for eye in swapchains {
            let image_index = eye.handle.acquire_image().map_err(|error| {
                format!(
                    "acquire GLES swapchain image eye {}: {error}",
                    eye.view_index
                )
            })?;
            eye.handle
                .wait_image(xr::Duration::INFINITE)
                .map_err(|error| {
                    format!("wait GLES swapchain image eye {}: {error}", eye.view_index)
                })?;
            let texture = eye
                .images
                .get(image_index as usize)
                .copied()
                .ok_or_else(|| format!("swapchain image index {image_index} is out of range"))?;

            let mut render_path = eye.pattern;
            let mut rendered_source_sequence = None;
            let fbo_status = if let (Some(probe), Some(renderer)) =
                (surface_texture_oes_probe, oes_copy_renderer.as_mut())
            {
                if let Some((source_texture, source_sequence)) =
                    probe.updated_eye_texture(eye.view_index)
                {
                    match fbo.render_external_oes(
                        texture,
                        source_texture,
                        eye.width,
                        eye.height,
                        renderer,
                    ) {
                        Ok(fbo_status) => {
                            render_path = OES_COPY_RENDER_PATH;
                            rendered_source_sequence = Some(source_sequence);
                            if frame_count == 0 || frame_count.is_multiple_of(120) {
                                log_projection_diagnostics(
                                    eye.view_index,
                                    frame_count,
                                    source_sequence,
                                );
                            }
                            fbo_status
                        }
                        Err(error) => {
                            status
                                .issue_codes
                                .push(String::from("oes_to_swapchain_copy_failed"));
                            log_error(format!(
                                "Rusty XR OpenXR GLES OES copy failed eye={} frame={}: {error}",
                                eye.view_index, frame_count
                            ));
                            fbo.render_grid(texture, eye.width, eye.height, eye.view_index)?
                        }
                    }
                } else {
                    fbo.render_grid(texture, eye.width, eye.height, eye.view_index)?
                }
            } else {
                fbo.render_grid(texture, eye.width, eye.height, eye.view_index)?
            };
            if let Some(view) = status.views.get_mut(eye.view_index) {
                view.acquired_image_index = Some(image_index);
                view.fbo_status = fbo_status;
                view.viewport_x = 0;
                view.viewport_y = 0;
                view.viewport_width = eye.width;
                view.viewport_height = eye.height;
                view.diagnostic_pattern = render_path.to_string();
                view.last_rendered_frame_index = Some(frame_count);
            }
            if frame_count == 0 || frame_count.is_multiple_of(120) {
                log_info(format!(
                    "Rusty XR OpenXR GLES rendered eye={} imageIndex={} texture={} viewport={}x{} fbo={:?} pattern={} sourceSequence={:?}",
                    eye.view_index,
                    image_index,
                    texture,
                    eye.width,
                    eye.height,
                    fbo_status,
                    render_path,
                    rendered_source_sequence
                ));
            }
            eye.handle.release_image().map_err(|error| {
                format!(
                    "release GLES swapchain image eye {}: {error}",
                    eye.view_index
                )
            })?;
        }
        unsafe {
            glFlush();
        }
        Ok(())
    }

    struct EyeSwapchain {
        handle: xr::Swapchain<xr::OpenGlEs>,
        images: Vec<u32>,
        width: u32,
        height: u32,
        view_index: usize,
        pattern: &'static str,
    }

    struct OesCopyRenderer {
        program: u32,
        vertex_buffer: u32,
        sampler_location: c_int,
    }

    impl OesCopyRenderer {
        fn new() -> Result<Self, String> {
            let vertex_shader = compile_shader(
                GL_VERTEX_SHADER,
                r#"#version 300 es
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_uv;
out vec2 v_uv;
void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_position, 0.0, 1.0);
}"#,
            )?;
            let fragment_shader = match compile_shader(
                GL_FRAGMENT_SHADER,
                r#"#version 300 es
#extension GL_OES_EGL_image_external_essl3 : require
precision mediump float;
uniform samplerExternalOES u_source;
in vec2 v_uv;
out vec4 out_color;
void main() {
    out_color = texture(u_source, v_uv);
}"#,
            ) {
                Ok(shader) => shader,
                Err(error) => {
                    delete_shader(vertex_shader);
                    return Err(error);
                }
            };
            let program = match link_program(vertex_shader, fragment_shader) {
                Ok(program) => program,
                Err(error) => {
                    delete_shader(vertex_shader);
                    delete_shader(fragment_shader);
                    return Err(error);
                }
            };
            delete_shader(vertex_shader);
            delete_shader(fragment_shader);

            let sampler_name =
                CString::new("u_source").map_err(|error| format!("sampler CString: {error}"))?;
            let sampler_location = unsafe { glGetUniformLocation(program, sampler_name.as_ptr()) };
            if sampler_location < 0 {
                unsafe {
                    glDeleteProgram(program);
                }
                return Err("OES copy shader did not expose u_source uniform".to_string());
            }

            let vertices: [f32; 16] = [
                -1.0, -1.0, 0.0, 0.0, //
                1.0, -1.0, 1.0, 0.0, //
                -1.0, 1.0, 0.0, 1.0, //
                1.0, 1.0, 1.0, 1.0,
            ];
            let mut vertex_buffer = 0;
            unsafe {
                glGenBuffers(1, &mut vertex_buffer);
                if vertex_buffer == 0 {
                    glDeleteProgram(program);
                    return Err("glGenBuffers returned 0 for OES copy quad".to_string());
                }
                glBindBuffer(GL_ARRAY_BUFFER, vertex_buffer);
                glBufferData(
                    GL_ARRAY_BUFFER,
                    (vertices.len() * mem::size_of::<f32>()) as isize,
                    vertices.as_ptr().cast(),
                    GL_STATIC_DRAW,
                );
                glBindBuffer(GL_ARRAY_BUFFER, 0);
            }

            Ok(Self {
                program,
                vertex_buffer,
                sampler_location,
            })
        }

        fn render(&mut self, source_oes_texture: u32) -> Result<(), String> {
            unsafe {
                glUseProgram(self.program);
                glActiveTexture(GL_TEXTURE0);
                glBindTexture(GL_TEXTURE_EXTERNAL_OES, source_oes_texture);
                glUniform1i(self.sampler_location, 0);
                glBindBuffer(GL_ARRAY_BUFFER, self.vertex_buffer);
                let stride = (4 * mem::size_of::<f32>()) as c_int;
                glEnableVertexAttribArray(0);
                glVertexAttribPointer(0, 2, GL_FLOAT, 0, stride, ptr::null());
                glEnableVertexAttribArray(1);
                glVertexAttribPointer(
                    1,
                    2,
                    GL_FLOAT,
                    0,
                    stride,
                    (2 * mem::size_of::<f32>()) as *const c_void,
                );
                glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
                glDisableVertexAttribArray(0);
                glDisableVertexAttribArray(1);
                glBindBuffer(GL_ARRAY_BUFFER, 0);
                glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0);
                glUseProgram(0);
                let error = glGetError();
                if error != GL_NO_ERROR {
                    return Err(format!(
                        "OES full-surface draw returned GL error 0x{error:04x}"
                    ));
                }
            }
            Ok(())
        }
    }

    impl Drop for OesCopyRenderer {
        fn drop(&mut self) {
            unsafe {
                if self.vertex_buffer != 0 {
                    glDeleteBuffers(1, &self.vertex_buffer);
                }
                if self.program != 0 {
                    glDeleteProgram(self.program);
                }
            }
        }
    }

    struct EglContext {
        display: EGLDisplay,
        config: EGLConfig,
        context: EGLContext,
        surface: EGLSurface,
        status: EglGlesContextStatus,
    }

    impl EglContext {
        fn create() -> Result<Self, String> {
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

        fn status(&self) -> EglGlesContextStatus {
            self.status.clone()
        }

        fn make_current(&self) -> Result<(), String> {
            unsafe {
                if eglMakeCurrent(self.display, self.surface, self.surface, self.context)
                    == EGL_FALSE
                {
                    return Err("eglMakeCurrent failed before render".to_string());
                }
            }
            Ok(())
        }
    }

    impl Drop for EglContext {
        fn drop(&mut self) {
            unsafe {
                let _ =
                    eglMakeCurrent(self.display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
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

    struct GlFramebuffer {
        id: u32,
    }

    impl GlFramebuffer {
        fn new() -> Self {
            let mut id = 0;
            unsafe {
                glGenFramebuffers(1, &mut id);
            }
            Self { id }
        }

        fn render_grid(
            &mut self,
            texture: u32,
            width: u32,
            height: u32,
            view_index: usize,
        ) -> Result<GlFramebufferCompleteness, String> {
            unsafe {
                glBindFramebuffer(GL_FRAMEBUFFER, self.id);
                glFramebufferTexture2D(
                    GL_FRAMEBUFFER,
                    GL_COLOR_ATTACHMENT0,
                    GL_TEXTURE_2D,
                    texture,
                    0,
                );
                let raw_status = glCheckFramebufferStatus(GL_FRAMEBUFFER);
                let fbo_status = framebuffer_status(raw_status);
                if !fbo_status.is_complete() {
                    glBindFramebuffer(GL_FRAMEBUFFER, 0);
                    return Ok(fbo_status);
                }

                glViewport(0, 0, width as c_int, height as c_int);
                let (background, grid) = if view_index == 0 {
                    ([0.12, 0.02, 0.02, 1.0], [0.0, 0.75, 0.85, 1.0])
                } else {
                    ([0.02, 0.04, 0.18, 1.0], [1.0, 0.85, 0.05, 1.0])
                };
                glClearColor(background[0], background[1], background[2], background[3]);
                glClear(GL_COLOR_BUFFER_BIT);
                glEnable(GL_SCISSOR_TEST);
                glClearColor(grid[0], grid[1], grid[2], grid[3]);

                let vertical_step = (width / 8).max(1);
                for x in (0..width).step_by(vertical_step as usize) {
                    glScissor(x as c_int, 0, 4, height as c_int);
                    glClear(GL_COLOR_BUFFER_BIT);
                }
                let horizontal_step = (height / 8).max(1);
                for y in (0..height).step_by(horizontal_step as usize) {
                    glScissor(0, y as c_int, width as c_int, 4);
                    glClear(GL_COLOR_BUFFER_BIT);
                }

                let marker_width = (width / 5).max(16);
                let marker_height = (height / 5).max(16);
                let marker_x = if view_index == 0 {
                    width / 12
                } else {
                    width.saturating_sub(marker_width + width / 12)
                };
                let marker_y = height / 2 - marker_height / 2;
                glClearColor(0.95, 0.95, 0.95, 1.0);
                glScissor(
                    marker_x as c_int,
                    marker_y as c_int,
                    marker_width as c_int,
                    marker_height as c_int,
                );
                glClear(GL_COLOR_BUFFER_BIT);
                glDisable(GL_SCISSOR_TEST);
                glBindFramebuffer(GL_FRAMEBUFFER, 0);
                Ok(fbo_status)
            }
        }

        fn render_external_oes(
            &mut self,
            target_texture: u32,
            source_oes_texture: u32,
            width: u32,
            height: u32,
            renderer: &mut OesCopyRenderer,
        ) -> Result<GlFramebufferCompleteness, String> {
            unsafe {
                glBindFramebuffer(GL_FRAMEBUFFER, self.id);
                glFramebufferTexture2D(
                    GL_FRAMEBUFFER,
                    GL_COLOR_ATTACHMENT0,
                    GL_TEXTURE_2D,
                    target_texture,
                    0,
                );
                let raw_status = glCheckFramebufferStatus(GL_FRAMEBUFFER);
                let fbo_status = framebuffer_status(raw_status);
                if !fbo_status.is_complete() {
                    glBindFramebuffer(GL_FRAMEBUFFER, 0);
                    return Ok(fbo_status);
                }

                glViewport(0, 0, width as c_int, height as c_int);
                glClearColor(0.0, 0.0, 0.0, 1.0);
                glClear(GL_COLOR_BUFFER_BIT);
                renderer.render(source_oes_texture)?;
                glBindFramebuffer(GL_FRAMEBUFFER, 0);
                Ok(fbo_status)
            }
        }
    }

    impl Drop for GlFramebuffer {
        fn drop(&mut self) {
            unsafe {
                if self.id != 0 {
                    glDeleteFramebuffers(1, &self.id);
                }
            }
        }
    }

    fn probe_surface_texture_oes(
        app: &android_activity::AndroidApp,
        egl: &EglContext,
    ) -> Option<SurfaceTextureOesProbe> {
        let mut failure = SurfaceTextureOesIngestStatus::new();
        failure.state = SurfaceTextureOesIngestState::Failed;

        let probe = match SurfaceTextureOesProbe::create(app, egl) {
            Ok(probe) => probe,
            Err(error) => {
                failure
                    .issue_codes
                    .push(String::from("surface_texture_oes_probe_failed"));
                failure.notes.push(error);
                log_surface_texture_oes_status(&failure);
                return None;
            }
        };

        log_surface_texture_oes_status(&probe.status);
        Some(probe)
    }

    struct SurfaceTextureOesProbe {
        status: SurfaceTextureOesIngestStatus,
        surface_textures: Vec<GlobalRef>,
        output_surfaces: Vec<GlobalRef>,
        decode_probe: Option<GlobalRef>,
        textures: Vec<u32>,
        java_vm: JavaVM,
        consumed_frame_available_counts: [u64; VIEW_COUNT],
        update_rate_start: Instant,
        last_report_sequence: u64,
    }

    impl SurfaceTextureOesProbe {
        fn create(app: &android_activity::AndroidApp, egl: &EglContext) -> Result<Self, String> {
            egl.make_current()?;
            oes_decode_callbacks().reset();
            let java_vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }
                .map_err(|error| format!("wrap Android JavaVM: {error}"))?;
            let mut textures = Vec::with_capacity(VIEW_COUNT);
            let mut status = SurfaceTextureOesIngestStatus::new();
            status.codec_mime = Some(String::from("video/avc"));
            status.notes.push(String::from(
                "Created SurfaceTexture-backed output surfaces for broker H.264 MediaCodec decode; updateTexImage runs on the native GL render thread.",
            ));

            let (surface_textures, output_surfaces) = {
                let mut env = java_vm.attach_current_thread().map_err(|error| {
                    format!("attach JNI thread for SurfaceTexture probe: {error}")
                })?;
                let mut surface_textures = Vec::with_capacity(VIEW_COUNT);
                let mut output_surfaces = Vec::with_capacity(VIEW_COUNT);

                for view_index in 0..VIEW_COUNT {
                    let texture = create_external_oes_texture()?;
                    let texture_name = i32::try_from(texture).map_err(|_| {
                        format!("external OES texture id {texture} does not fit JNI int")
                    })?;
                    let surface_texture = env
                        .new_object(
                            "android/graphics/SurfaceTexture",
                            "(I)V",
                            &[JValue::Int(texture_name)],
                        )
                        .map_err(|error| {
                            delete_gl_texture(texture);
                            format!("create Android SurfaceTexture for eye {view_index}: {error}")
                        })?;
                    env.call_method(
                        &surface_texture,
                        "setDefaultBufferSize",
                        "(II)V",
                        &[
                            JValue::Int(DEFAULT_OES_SURFACE_WIDTH),
                            JValue::Int(DEFAULT_OES_SURFACE_HEIGHT),
                        ],
                    )
                    .map_err(|error| {
                        delete_gl_texture(texture);
                        format!(
                            "set SurfaceTexture default buffer size for eye {view_index}: {error}"
                        )
                    })?;
                    let output_surface = env
                        .new_object(
                            "android/view/Surface",
                            "(Landroid/graphics/SurfaceTexture;)V",
                            &[JValue::Object(&surface_texture)],
                        )
                        .map_err(|error| {
                            delete_gl_texture(texture);
                            format!("create Android Surface for eye {view_index}: {error}")
                        })?;
                    let surface_texture_ref =
                        env.new_global_ref(&surface_texture).map_err(|error| {
                            delete_gl_texture(texture);
                            format!(
                                "promote SurfaceTexture global reference for eye {view_index}: {error}"
                            )
                        })?;
                    let output_surface_ref =
                        env.new_global_ref(&output_surface).map_err(|error| {
                            delete_gl_texture(texture);
                            format!(
                                "promote Surface global reference for eye {view_index}: {error}"
                            )
                        })?;

                    textures.push(texture);
                    surface_textures.push(surface_texture_ref);
                    output_surfaces.push(output_surface_ref);
                    let eye_name = if view_index == 0 { "left" } else { "right" };
                    let mut eye = SurfaceTextureOesEyeStatus::for_stream(
                        view_index as u32,
                        format!("public-synthetic:{eye_name}"),
                        eye_name,
                    )
                    .mark_surface_ready();
                    eye.source_width = Some(DEFAULT_OES_SURFACE_WIDTH as u32);
                    eye.source_height = Some(DEFAULT_OES_SURFACE_HEIGHT as u32);
                    status.eyes.push(eye);
                }

                (surface_textures, output_surfaces)
            };

            let decode_probe = {
                let mut env = java_vm.attach_current_thread().map_err(|error| {
                    format!("attach JNI thread for broker H.264 OES decode start: {error}")
                })?;
                match start_broker_h264_oes_decode_probe(
                    &mut env,
                    app,
                    &output_surfaces,
                    &surface_textures,
                ) {
                    Ok(probe) => {
                        for eye in &mut status.eyes {
                            eye.decoder_configured = true;
                            eye.decoder_started = true;
                        }
                        status.state = SurfaceTextureOesIngestState::DecoderStarted;
                        status.notes.push(format!(
                            "Started broker-compatible RXYRVID1 H.264 decode threads host={} leftPort={} rightPort={}.",
                            BROKER_H264_DEFAULT_HOST,
                            BROKER_H264_LEFT_STREAM_PORT,
                            BROKER_H264_RIGHT_STREAM_PORT
                        ));
                        Some(probe)
                    }
                    Err(error) => {
                        status.state = SurfaceTextureOesIngestState::OutputSurfaceReady;
                        status
                            .issue_codes
                            .push(String::from("broker_h264_oes_decode_start_failed"));
                        status.notes.push(error);
                        None
                    }
                }
            };

            log_info(format!(
                "Rusty XR SurfaceTexture OES output surfaces ready eyes={} size={}x{} decoderStarted={}",
                status.eyes.len(),
                DEFAULT_OES_SURFACE_WIDTH,
                DEFAULT_OES_SURFACE_HEIGHT,
                decode_probe.is_some()
            ));

            Ok(Self {
                status,
                surface_textures,
                output_surfaces,
                decode_probe,
                textures,
                java_vm,
                consumed_frame_available_counts: [0; VIEW_COUNT],
                update_rate_start: Instant::now(),
                last_report_sequence: 0,
            })
        }

        fn update_textures(&mut self, egl: &EglContext, frame_count: u64) {
            self.apply_latest_decode_report();
            let callbacks = oes_decode_callbacks();
            let mut updated_any = false;
            for view_index in 0..VIEW_COUNT {
                let (available_count, latest_sequence, latest_pts_us) =
                    callbacks.frame_snapshot(view_index);
                if available_count <= self.consumed_frame_available_counts[view_index] {
                    if let Some(eye) = self.status.eyes.get_mut(view_index) {
                        eye.frame_available_count = available_count;
                    }
                    continue;
                }

                let skipped = available_count
                    .saturating_sub(self.consumed_frame_available_counts[view_index])
                    .saturating_sub(1);
                match self.update_surface_texture(egl, view_index) {
                    Ok((timestamp_ns, transform_hash)) => {
                        if let Some(eye) = self.status.eyes.get_mut(view_index) {
                            eye.record_update(
                                frame_count,
                                latest_sequence,
                                latest_pts_us,
                                timestamp_ns,
                                transform_hash,
                            );
                            eye.frame_available_count = available_count;
                            eye.skipped_update_count =
                                eye.skipped_update_count.saturating_add(skipped);
                        }
                        self.consumed_frame_available_counts[view_index] = available_count;
                        updated_any = true;
                    }
                    Err(error) => {
                        if let Some(eye) = self.status.eyes.get_mut(view_index) {
                            eye.frame_available_count = available_count;
                            eye.decoder_error_count = eye.decoder_error_count.saturating_add(1);
                            eye.latest_decoder_error = Some(error.clone());
                        }
                        self.status
                            .issue_codes
                            .push(String::from("surface_texture_update_failed"));
                        log_error(format!(
                            "Rusty XR SurfaceTexture OES update failed eye={view_index}: {error}"
                        ));
                    }
                }
            }

            if updated_any {
                self.status.state = SurfaceTextureOesIngestState::TextureUpdated;
                self.refresh_texture_update_rate();
                log_surface_texture_oes_status(&self.status);
            }
        }

        fn update_surface_texture(
            &self,
            egl: &EglContext,
            view_index: usize,
        ) -> Result<(i64, String), String> {
            egl.make_current()?;
            let surface_texture = self
                .surface_textures
                .get(view_index)
                .ok_or_else(|| format!("SurfaceTexture eye index {view_index} is out of range"))?;
            let mut env = self
                .java_vm
                .attach_current_thread()
                .map_err(|error| format!("attach JNI thread for updateTexImage: {error}"))?;
            env.call_method(surface_texture.as_obj(), "updateTexImage", "()V", &[])
                .map_err(|error| format!("updateTexImage: {error}"))?;
            let timestamp_ns = env
                .call_method(surface_texture.as_obj(), "getTimestamp", "()J", &[])
                .and_then(|value| value.j())
                .map_err(|error| format!("get SurfaceTexture timestamp: {error}"))?;
            Ok((timestamp_ns, String::from("m44:not-sampled")))
        }

        fn updated_eye_texture(&self, view_index: usize) -> Option<(u32, u64)> {
            let eye = self.status.eyes.get(view_index)?;
            if eye.update_tex_image_count == 0 || eye.decoder_error_count > 0 {
                return None;
            }
            let texture = *self.textures.get(view_index)?;
            Some((texture, eye.latest_stream_sequence.unwrap_or_default()))
        }

        fn refresh_texture_update_rate(&mut self) {
            let elapsed = self.update_rate_start.elapsed().as_secs_f32();
            if elapsed <= 0.0 {
                return;
            }
            let sample_count = self
                .status
                .eyes
                .iter()
                .map(|eye| eye.update_tex_image_count)
                .sum::<u64>();
            let average_fps = sample_count as f32 / elapsed;
            self.status.texture_update_rate = Some(FrameRateSummary {
                sample_count,
                average_fps,
                min_fps: average_fps,
                max_fps: average_fps,
            });
        }

        fn apply_latest_decode_report(&mut self) {
            let Some(report_json) =
                oes_decode_callbacks().latest_report_after(&mut self.last_report_sequence)
            else {
                return;
            };
            let Ok(report) = serde_json::from_str::<serde_json::Value>(&report_json) else {
                return;
            };
            if let Some(decoder_name) = report.get("decoder_name").and_then(|value| value.as_str())
            {
                if !decoder_name.is_empty() {
                    self.status.codec_name = Some(decoder_name.to_string());
                }
            }
            if let Some(event) = report.get("event").and_then(|value| value.as_str()) {
                if event == "frame_available"
                    && self.status.state == SurfaceTextureOesIngestState::DecoderStarted
                {
                    self.status.state = SurfaceTextureOesIngestState::FrameAvailable;
                }
            }
            let Some(view_index) = report
                .get("view_index")
                .and_then(|value| value.as_u64())
                .and_then(|value| usize::try_from(value).ok())
            else {
                return;
            };
            let Some(eye) = self.status.eyes.get_mut(view_index) else {
                return;
            };
            if let Some(width) = report
                .get("width")
                .and_then(|value| value.as_u64())
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0)
            {
                eye.source_width = Some(width);
            }
            if let Some(height) = report
                .get("height")
                .and_then(|value| value.as_u64())
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0)
            {
                eye.source_height = Some(height);
            }
            if let Some(error) = report.get("error").and_then(|value| value.as_str()) {
                eye.decoder_error_count = eye.decoder_error_count.saturating_add(1);
                eye.latest_decoder_error = Some(error.to_string());
            } else if let Some(error) = report.get("last_error").and_then(|value| value.as_str()) {
                eye.latest_decoder_error = Some(error.to_string());
            }
        }
    }

    fn start_broker_h264_oes_decode_probe(
        env: &mut JNIEnv<'_>,
        app: &android_activity::AndroidApp,
        output_surfaces: &[GlobalRef],
        surface_textures: &[GlobalRef],
    ) -> Result<GlobalRef, String> {
        if output_surfaces.len() < VIEW_COUNT || surface_textures.len() < VIEW_COUNT {
            return Err(format!(
                "broker H.264 OES decode requires {VIEW_COUNT} output surfaces and SurfaceTextures"
            ));
        }
        let host = env
            .new_string(BROKER_H264_DEFAULT_HOST)
            .map_err(|error| jni_error(env, "create broker H.264 host string", error))?;
        let host_object = JObject::from(host);
        let class_name = env
            .new_string("com.example.rustyxr.opengles.BrokerH264OesDecodeProbe")
            .map_err(|error| jni_error(env, "create broker H.264 helper class string", error))?;
        let class_name_object = JObject::from(class_name);
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        let class_loader = env
            .call_method(
                &activity,
                "getClassLoader",
                "()Ljava/lang/ClassLoader;",
                &[],
            )
            .and_then(|value| value.l())
            .map_err(|error| jni_error(env, "read Activity class loader", error))?;
        let helper_class_object = env
            .call_method(
                &class_loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&class_name_object)],
            )
            .and_then(|value| value.l())
            .map_err(|error| jni_error(env, "load broker H.264 OES helper class", error))?;
        let helper_class = JClass::from(helper_class_object);
        let probe = env
            .call_static_method(
                &helper_class,
                "start",
                "(Ljava/lang/String;IILandroid/view/Surface;Landroid/view/Surface;Landroid/graphics/SurfaceTexture;Landroid/graphics/SurfaceTexture;III)Lcom/example/rustyxr/opengles/BrokerH264OesDecodeProbe;",
                &[
                    JValue::Object(&host_object),
                    JValue::Int(BROKER_H264_LEFT_STREAM_PORT),
                    JValue::Int(BROKER_H264_RIGHT_STREAM_PORT),
                    JValue::Object(output_surfaces[0].as_obj()),
                    JValue::Object(output_surfaces[1].as_obj()),
                    JValue::Object(surface_textures[0].as_obj()),
                    JValue::Object(surface_textures[1].as_obj()),
                    JValue::Int(BROKER_H264_MAX_PACKETS),
                    JValue::Int(BROKER_H264_CONNECT_TIMEOUT_MS),
                    JValue::Int(BROKER_H264_DECODE_TIMEOUT_MS),
                ],
            )
            .and_then(|value| value.l())
            .map_err(|error| jni_error(env, "start Java broker H.264 OES decode probe", error))?;
        if probe.is_null() {
            return Err("Java broker H.264 OES decode probe returned null".to_string());
        }
        env.new_global_ref(&probe).map_err(|error| {
            jni_error(
                env,
                "promote broker H.264 OES decode probe reference",
                error,
            )
        })
    }

    fn jni_error(env: &mut JNIEnv<'_>, context: &str, error: impl std::fmt::Display) -> String {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }
        format!("{context}: {error}")
    }

    #[allow(non_snake_case)]
    #[no_mangle]
    pub extern "system" fn Java_com_example_rustyxr_opengles_BrokerH264OesDecodeProbe_nativeBrokerH264FrameAvailable(
        _env: JNIEnv<'_>,
        _class: JClass<'_>,
        view_index: jint,
        sequence: jlong,
        queued_pts_us: jlong,
    ) {
        let Ok(view_index) = usize::try_from(view_index) else {
            return;
        };
        let sequence = u64::try_from(sequence).unwrap_or(0);
        oes_decode_callbacks().mark_frame_available(view_index, sequence, queued_pts_us);
    }

    #[allow(non_snake_case)]
    #[no_mangle]
    pub extern "system" fn Java_com_example_rustyxr_opengles_BrokerH264OesDecodeProbe_nativeBrokerH264DecodeReport(
        mut env: JNIEnv<'_>,
        _class: JClass<'_>,
        report_json: JString<'_>,
    ) {
        let report = env
            .get_string(&report_json)
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "{\"event\":\"invalidJniString\"}".to_string());
        log_info(format!("Rusty XR broker H.264 OES decode report {report}"));
        oes_decode_callbacks().record_report(report);
    }

    impl Drop for SurfaceTextureOesProbe {
        fn drop(&mut self) {
            if let Ok(mut env) = self.java_vm.attach_current_thread() {
                if let Some(decode_probe) = &self.decode_probe {
                    let _ = env.call_method(decode_probe.as_obj(), "stop", "()V", &[]);
                }
                for surface in &self.output_surfaces {
                    let _ = env.call_method(surface.as_obj(), "release", "()V", &[]);
                }
                for surface_texture in &self.surface_textures {
                    let _ = env.call_method(surface_texture.as_obj(), "release", "()V", &[]);
                }
            }
            for texture in &self.textures {
                delete_gl_texture(*texture);
            }
        }
    }

    fn create_external_oes_texture() -> Result<u32, String> {
        unsafe {
            while glGetError() != GL_NO_ERROR {}
            let mut texture = 0;
            glGenTextures(1, &mut texture);
            if texture == 0 {
                return Err("glGenTextures returned texture id 0 for external OES texture".into());
            }
            glBindTexture(GL_TEXTURE_EXTERNAL_OES, texture);
            glTexParameteri(
                GL_TEXTURE_EXTERNAL_OES,
                GL_TEXTURE_MIN_FILTER,
                GL_LINEAR as c_int,
            );
            glTexParameteri(
                GL_TEXTURE_EXTERNAL_OES,
                GL_TEXTURE_MAG_FILTER,
                GL_LINEAR as c_int,
            );
            glTexParameteri(
                GL_TEXTURE_EXTERNAL_OES,
                GL_TEXTURE_WRAP_S,
                GL_CLAMP_TO_EDGE as c_int,
            );
            glTexParameteri(
                GL_TEXTURE_EXTERNAL_OES,
                GL_TEXTURE_WRAP_T,
                GL_CLAMP_TO_EDGE as c_int,
            );
            glBindTexture(GL_TEXTURE_EXTERNAL_OES, 0);
            let error = glGetError();
            if error != GL_NO_ERROR {
                delete_gl_texture(texture);
                return Err(format!(
                    "external OES texture setup returned GL error 0x{error:04x}"
                ));
            }
            Ok(texture)
        }
    }

    fn delete_gl_texture(texture: u32) {
        if texture != 0 {
            unsafe {
                glDeleteTextures(1, &texture);
            }
        }
    }

    fn compile_shader(shader_type: u32, source: &str) -> Result<u32, String> {
        let source = CString::new(source).map_err(|error| format!("shader CString: {error}"))?;
        unsafe {
            let shader = glCreateShader(shader_type);
            if shader == 0 {
                return Err("glCreateShader returned 0".to_string());
            }
            let ptr = source.as_ptr();
            glShaderSource(shader, 1, &ptr, ptr::null());
            glCompileShader(shader);
            let mut compiled = 0;
            glGetShaderiv(shader, GL_COMPILE_STATUS, &mut compiled);
            if compiled == 0 {
                let info_log = shader_info_log(shader);
                glDeleteShader(shader);
                return Err(format!("OES copy shader compile failed: {info_log}"));
            }
            Ok(shader)
        }
    }

    fn link_program(vertex_shader: u32, fragment_shader: u32) -> Result<u32, String> {
        unsafe {
            let program = glCreateProgram();
            if program == 0 {
                return Err("glCreateProgram returned 0".to_string());
            }
            glAttachShader(program, vertex_shader);
            glAttachShader(program, fragment_shader);
            glLinkProgram(program);
            let mut linked = 0;
            glGetProgramiv(program, GL_LINK_STATUS, &mut linked);
            if linked == 0 {
                let info_log = program_info_log(program);
                glDeleteProgram(program);
                return Err(format!("OES copy program link failed: {info_log}"));
            }
            Ok(program)
        }
    }

    fn shader_info_log(shader: u32) -> String {
        unsafe {
            let mut length = 0;
            glGetShaderiv(shader, GL_INFO_LOG_LENGTH, &mut length);
            if length <= 1 {
                return String::from("no shader info log");
            }
            let mut buffer = vec![0_u8; length as usize];
            glGetShaderInfoLog(
                shader,
                length,
                ptr::null_mut(),
                buffer.as_mut_ptr().cast::<c_char>(),
            );
            CStr::from_ptr(buffer.as_ptr().cast::<c_char>())
                .to_string_lossy()
                .into_owned()
        }
    }

    fn program_info_log(program: u32) -> String {
        unsafe {
            let mut length = 0;
            glGetProgramiv(program, GL_INFO_LOG_LENGTH, &mut length);
            if length <= 1 {
                return String::from("no program info log");
            }
            let mut buffer = vec![0_u8; length as usize];
            glGetProgramInfoLog(
                program,
                length,
                ptr::null_mut(),
                buffer.as_mut_ptr().cast::<c_char>(),
            );
            CStr::from_ptr(buffer.as_ptr().cast::<c_char>())
                .to_string_lossy()
                .into_owned()
        }
    }

    fn delete_shader(shader: u32) {
        unsafe {
            if shader != 0 {
                glDeleteShader(shader);
            }
        }
    }

    fn log_projection_diagnostics(view_index: usize, frame_count: u64, source_sequence: u64) {
        let Some(eye) = eye_from_view_index(view_index) else {
            return;
        };
        let identity = identity_homography();
        for stage in [
            ProjectionStageKind::ScreenToSurface,
            ProjectionStageKind::SurfaceToCamera,
            ProjectionStageKind::ScreenToCamera,
        ] {
            let row = ProjectionStageTokenRow::new("rusty_xr_gl_oes", eye, stage)
                .with_rows(identity)
                .with_source(format!(
                    "{OES_COPY_RENDER_PATH}:frame={frame_count}:source_sequence={source_sequence}"
                ));
            match serde_json::to_string(&row) {
                Ok(json) => log_info(format!("Rusty XR OpenXR GLES projection stage row {json}")),
                Err(error) => log_error(format!(
                    "Rusty XR OpenXR GLES projection stage serialization failed: {error}"
                )),
            }
        }

        let footprint =
            ProjectionFootprintSummary::new("rusty_xr_gl_oes", "public_raw_oes_full_surface")
                .with_active_fraction(1.0)
                .with_bbox_fraction([0.0, 0.0, 1.0, 1.0])
                .with_row_span(ProjectionFootprintRowSpan::new(0.0, 1.0).with_span(0.0, 1.0))
                .with_row_span(ProjectionFootprintRowSpan::new(0.5, 1.0).with_span(0.0, 1.0))
                .with_row_span(ProjectionFootprintRowSpan::new(1.0, 1.0).with_span(0.0, 1.0))
                .with_invalid_fill_policy(InvalidProjectionFillPolicy::Black)
                .with_guide_domain(ProjectionGuideDomain::SubmittedSurface)
                .with_explicit_valid_mask(true)
                .with_note(format!(
                    "Full-surface public raw OES copy into the OpenXR GLES swapchain at frame {frame_count}."
                ));
        match serde_json::to_string(&footprint) {
            Ok(json) => log_info(format!("Rusty XR OpenXR GLES projection footprint {json}")),
            Err(error) => log_error(format!(
                "Rusty XR OpenXR GLES projection footprint serialization failed: {error}"
            )),
        }
    }

    fn eye_from_view_index(view_index: usize) -> Option<Eye> {
        match view_index {
            0 => Some(Eye::Left),
            1 => Some(Eye::Right),
            _ => None,
        }
    }

    const fn identity_homography() -> [[f32; 3]; 3] {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    fn framebuffer_status(raw: u32) -> GlFramebufferCompleteness {
        match raw {
            GL_FRAMEBUFFER_COMPLETE => GlFramebufferCompleteness::Complete,
            GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT => GlFramebufferCompleteness::IncompleteAttachment,
            GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => {
                GlFramebufferCompleteness::IncompleteMissingAttachment
            }
            GL_FRAMEBUFFER_UNSUPPORTED => GlFramebufferCompleteness::IncompleteUnsupported,
            GL_FRAMEBUFFER_INCOMPLETE_MULTISAMPLE => {
                GlFramebufferCompleteness::IncompleteMultisample
            }
            _ => GlFramebufferCompleteness::OtherIncomplete,
        }
    }

    fn select_color_format(formats: &[u32]) -> Option<u32> {
        [GL_SRGB8_ALPHA8, GL_RGBA8, GL_RGB10_A2, GL_RGBA]
            .into_iter()
            .find(|preferred| formats.contains(preferred))
            .or_else(|| formats.first().copied())
    }

    fn is_color_format(format: u32) -> bool {
        matches!(format, GL_SRGB8_ALPHA8 | GL_RGBA8 | GL_RGB10_A2 | GL_RGBA)
    }

    fn is_depth_format(format: u32) -> bool {
        matches!(
            format,
            GL_DEPTH_COMPONENT16 | GL_DEPTH_COMPONENT24 | GL_DEPTH24_STENCIL8
        )
    }

    fn gl_format_label(format: u32) -> &'static str {
        match format {
            GL_SRGB8_ALPHA8 => "GL_SRGB8_ALPHA8",
            GL_RGBA8 => "GL_RGBA8",
            GL_RGB10_A2 => "GL_RGB10_A2",
            GL_RGBA => "GL_RGBA",
            GL_DEPTH_COMPONENT16 => "GL_DEPTH_COMPONENT16",
            GL_DEPTH_COMPONENT24 => "GL_DEPTH_COMPONENT24",
            GL_DEPTH24_STENCIL8 => "GL_DEPTH24_STENCIL8",
            _ => "GL_UNKNOWN",
        }
    }

    fn gl_string(name: u32) -> Option<String> {
        unsafe {
            let value = glGetString(name);
            if value.is_null() {
                None
            } else {
                Some(
                    CStr::from_ptr(value as *const c_char)
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

    fn log_status(status: &OpenXrGlesFeasibilityStatus) {
        match serde_json::to_string(status) {
            Ok(json) => log_info(format!("Rusty XR OpenXR GLES feasibility status {json}")),
            Err(error) => log_error(format!(
                "Rusty XR OpenXR GLES status serialization failed: {error}"
            )),
        }
    }

    fn log_surface_texture_oes_status(status: &SurfaceTextureOesIngestStatus) {
        match serde_json::to_string(status) {
            Ok(json) => log_info(format!("Rusty XR SurfaceTexture OES ingest status {json}")),
            Err(error) => log_error(format!(
                "Rusty XR SurfaceTexture OES status serialization failed: {error}"
            )),
        }
    }

    fn wait_for_android_foreground(app: &android_activity::AndroidApp) -> Result<(), String> {
        let start = Instant::now();
        let mut state = AndroidForegroundState::default();
        log_info("Rusty XR GLES waiting for Android resume/focus before OpenXR session setup");

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
                log_info("Rusty XR GLES Android activity is foreground; continuing OpenXR setup");
                return Ok(());
            }
            if start.elapsed() >= Duration::from_secs(10) {
                log_error(
                    "Timed out waiting for Android focus before GLES/OpenXR setup; continuing best-effort",
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

    #[derive(Default)]
    struct AndroidForegroundState {
        resumed: bool,
        focused: bool,
        has_window: bool,
        destroyed: bool,
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
                log_info("Rusty XR GLES Android native window initialized");
                if let Some(state) = foreground.as_mut() {
                    state.has_window = true;
                }
            }
            MainEvent::TerminateWindow { .. } => {
                log_info("Rusty XR GLES Android native window terminated");
                if let Some(state) = foreground.as_mut() {
                    state.has_window = false;
                }
            }
            MainEvent::Destroy => {
                log_info("Rusty XR GLES Android activity destroy requested");
                if let Some(state) = foreground.as_mut() {
                    state.destroyed = true;
                }
                if let Some(running) = running {
                    *running = false;
                }
            }
            MainEvent::Pause => {
                log_info("Rusty XR GLES Android activity paused");
                if let Some(state) = foreground.as_mut() {
                    state.resumed = false;
                }
            }
            MainEvent::Resume { .. } => {
                log_info("Rusty XR GLES Android activity resumed");
                if let Some(state) = foreground.as_mut() {
                    state.resumed = true;
                }
            }
            MainEvent::GainedFocus => {
                log_info("Rusty XR GLES Android activity gained focus");
                if let Some(state) = foreground.as_mut() {
                    state.focused = true;
                }
            }
            MainEvent::LostFocus => {
                log_info("Rusty XR GLES Android activity lost focus");
                if let Some(state) = foreground.as_mut() {
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
                log_error(format!("Rusty XR GLES Android input drain failed: {error}"));
            }
        }
    }

    fn keep_activity_alive_after_error(app: android_activity::AndroidApp) {
        log_info("Rusty XR GLES keeping activity alive after setup failure");
        let mut running = true;
        while running {
            app.poll_events(Some(Duration::from_millis(250)), |event| {
                if let PollEvent::Main(MainEvent::Destroy) = event {
                    running = false;
                }
            });
        }
        log_info("Rusty XR GLES post-error keepalive exited");
    }

    fn log_info(message: impl AsRef<str>) {
        android_log(
            ndk_sys::android_LogPriority::ANDROID_LOG_INFO,
            message.as_ref(),
        );
    }

    fn log_error(message: impl AsRef<str>) {
        android_log(
            ndk_sys::android_LogPriority::ANDROID_LOG_ERROR,
            message.as_ref(),
        );
    }

    fn android_log(priority: ndk_sys::android_LogPriority, message: &str) {
        let tag = CString::new("RustyXrGles").expect("static Android log tag is valid");
        let safe_message = message.replace('\0', "\\0");
        if let Ok(message) = CString::new(safe_message) {
            unsafe {
                ndk_sys::__android_log_write(priority.0 as c_int, tag.as_ptr(), message.as_ptr());
            }
        }
    }

    #[allow(dead_code)]
    fn _extension_name_for_docs() -> &'static str {
        OPENXR_GLES_EXTENSION
    }
}

#[cfg(test)]
mod tests {
    use super::status_json;

    #[test]
    fn status_json_uses_public_schema() {
        let json = status_json();

        assert!(json.contains("rusty.xr.quest.openxr_gles_feasibility.v1"));
        assert!(json.contains("XR_KHR_opengl_es_enable"));
    }
}
