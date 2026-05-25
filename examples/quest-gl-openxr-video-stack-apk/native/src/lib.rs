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

mod source_sampling;

pub fn status_json() -> String {
    let status = OpenXrGlesFeasibilityStatus::new();
    serde_json::to_string_pretty(&status).expect("OpenXR/GLES status should serialize")
}

fn current_android_projection_property_config<'a>(
    pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> rusty_xr_runtime_config::RuntimeConfigAliasParse {
    use rusty_xr_runtime_config as rxrc;

    let mut config = rxrc::RuntimeConfig::new();
    let mut aliases = Vec::new();
    for (key, value) in pairs {
        let Ok(alias) = rxrc::resolve_projection_runtime_key(key) else {
            continue;
        };
        if alias.source != rxrc::RuntimeKeyAliasSource::AndroidProperty
            || alias.status != rxrc::RuntimeKeyAliasStatus::Current
        {
            continue;
        }
        let Ok(parsed) = rxrc::parse_projection_runtime_pairs(
            rxrc::RuntimeConfigSource::AndroidProperty,
            [(key, value)],
        ) else {
            continue;
        };
        for setting in parsed.config.iter() {
            config.insert(setting.clone());
        }
        aliases.extend(parsed.aliases);
    }

    rxrc::RuntimeConfigAliasParse { config, aliases }
}

#[cfg(target_os = "android")]
mod android {
    use super::*;
    use openxr as xr;
    use rusty_xr_quest_diagnostics::{OpenXrGlesFeasibilityState, OPENXR_GLES_EXTENSION};
    use std::{
        ffi::CString,
        os::raw::{c_char, c_int, c_void},
        time::Duration,
    };

    mod android_activity_events;
    mod egl_gles_context;
    mod oes_copy_renderer;
    mod openxr_gles_config;
    mod openxr_gles_passthrough;
    mod openxr_gles_renderer;
    mod openxr_gles_resources;
    mod openxr_gles_session;
    mod projection_geometry;
    mod projection_runtime;
    mod source_metadata;
    mod surface_texture_oes_callbacks;
    mod surface_texture_oes_probe;
    mod surface_texture_oes_sources;
    use android_activity_events::{
        keep_activity_alive_after_error, pump_android_events, wait_for_android_foreground,
    };
    use egl_gles_context::{create_recorded_egl_context, EglContext};
    use openxr_gles_config::OesActivityConfig;
    use openxr_gles_passthrough::create_requested_openxr_gles_passthrough_underlay;
    use openxr_gles_renderer::{
        projection_views_from_swapchains, OesRenderFrameInputs, OesRenderResources, OesRenderTuning,
    };
    use openxr_gles_resources::{
        create_eye_swapchains, gl_format_label, record_graphics_requirements,
        select_environment_blend_mode,
    };
    use openxr_gles_session::{
        begin_openxr_frame, create_android_instance, create_openxr_gles_session,
        end_empty_openxr_frame, end_projection_openxr_frame, initialize_android_loader,
        locate_submit_valid_views, poll_openxr_session_events, record_openxr_runtime_properties,
        request_session_exit_if_app_stopped, select_openxr_gles_extensions, OesFrameRateTracker,
        OesLocatedViews,
    };
    use projection_geometry::projection_frame_context_from_state;
    use projection_runtime::{log_oes_projection_startup_summary, OesProjectionRuntimeController};
    use surface_texture_oes_probe::probe_surface_texture_oes;

    const VIEW_COUNT: usize = 2;
    const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;
    const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
    const GL_TRIANGLE_STRIP: u32 = 0x0005;
    const GL_SCISSOR_TEST: u32 = 0x0C11;
    const GL_TEXTURE_2D: u32 = 0x0DE1;
    const GL_FLOAT: u32 = 0x1406;
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
    const OES_COPY_RENDER_PATH: &str = "oes-full-surface-copy";
    const OES_PROJECTED_RENDER_PATH: &str = "oes-projected-camera-uv";
    const DIRECT_CAMERA2_OES_SOURCE: &str = "app.camera2_oes_surface_texture";
    const PROJECTION_SOURCE_ASPECT: f32 = 1.0;

    #[link(name = "GLESv3")]
    unsafe extern "C" {
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
        fn glBindTexture(target: u32, texture: u32);
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
        fn glUniform1f(location: c_int, v0: f32);
        fn glUniform2f(location: c_int, v0: f32, v1: f32);
        fn glUniform3f(location: c_int, v0: f32, v1: f32, v2: f32);
        fn glUniform4f(location: c_int, v0: f32, v1: f32, v2: f32, v3: f32);
        fn glUniformMatrix4fv(location: c_int, count: c_int, transpose: u8, value: *const f32);
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
        let activity_config = OesActivityConfig::from_activity(&app);
        let processing_layer = activity_config.processing_layer;
        let blur_radius_px = activity_config.blur_radius_px;
        let base_projection_tuning = activity_config.base_projection_tuning;
        let activity_projection_state = activity_config.projection_state;
        let camera_color_controls = activity_config.camera_color_controls;
        let mut projection_runtime_controller =
            OesProjectionRuntimeController::from_activity(&app, activity_projection_state);
        let mut projection_state = projection_runtime_controller.current_state();
        projection_runtime_controller.log_manifest("startup");
        let projection_uses_source_alpha = projection_state
            .projection_border_policy
            .needs_source_alpha(
                projection_state.projection_area_opacity,
                projection_state.projection_border_opacity,
                projection_state.projection_alpha_mode,
            );
        let native_passthrough_underlay_requested = projection_uses_source_alpha;
        projection_runtime_controller.log_initial_tuning_if_changed(base_projection_tuning);

        let entry = unsafe { xr::Entry::load().map_err(|error| format!("load OpenXR: {error}"))? };
        initialize_android_loader(&entry, &app)?;
        let enabled_extensions = select_openxr_gles_extensions(
            &entry,
            native_passthrough_underlay_requested,
            &mut status,
        )?;

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

        record_openxr_runtime_properties(&xr_instance, &mut status)?;
        // Repeat the resolved runtime manifest at lifecycle boundaries where
        // OpenXR state has changed. The validation harness owns log capture
        // timing; renderer cadence must not be changed just to satisfy a tail.
        projection_runtime_controller.log_manifest("openxr-runtime");

        let system = xr_instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(|error| format!("get HMD system: {error}"))?;
        log_oes_projection_startup_summary(
            projection_state,
            processing_layer,
            blur_radius_px,
            native_passthrough_underlay_requested,
            enabled_extensions.fb_passthrough,
            camera_color_controls,
        );
        let environment_blend_mode = select_environment_blend_mode(
            &xr_instance,
            system,
            &mut status,
            projection_state.projection_border_policy.stable_id(),
            projection_uses_source_alpha,
        )?;
        record_graphics_requirements(&xr_instance, system, &mut status)?;

        let egl = create_recorded_egl_context(&mut status)?;
        let mut surface_texture_oes_probe = probe_surface_texture_oes(&app, &egl);

        wait_for_android_foreground(&app)?;
        let (session, mut frame_wait, mut frame_stream) =
            create_openxr_gles_session(&xr_instance, system, &egl, &mut status)?;
        projection_runtime_controller.log_manifest("session-ready");
        let native_passthrough_underlay = create_requested_openxr_gles_passthrough_underlay(
            native_passthrough_underlay_requested,
            &xr_instance,
            &session,
            &mut status,
        );

        let stage = session
            .create_reference_space(xr::ReferenceSpaceType::LOCAL, xr::Posef::IDENTITY)
            .map_err(|error| format!("create LOCAL reference space: {error}"))?;
        let mut swapchains = create_eye_swapchains(&xr_instance, system, &session, &mut status)?;
        let mut render_resources = OesRenderResources::new(&mut status);
        status.state = OpenXrGlesFeasibilityState::SwapchainsReady;
        log_status(&status);

        let mut event_storage = xr::EventDataBuffer::new();
        let mut app_running = true;
        let mut session_running = false;
        let mut frame_count = 0_u64;
        let mut frame_rate_tracker = OesFrameRateTracker::new();

        'main_loop: loop {
            pump_android_events(&app, &mut app_running);
            request_session_exit_if_app_stopped(app_running, &session);

            if poll_openxr_session_events(
                &xr_instance,
                &session,
                &mut event_storage,
                &mut session_running,
            )? {
                break 'main_loop;
            }

            if !session_running {
                if !app_running {
                    break 'main_loop;
                }
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }

            let frame_state = begin_openxr_frame(&mut frame_wait, &mut frame_stream)?;

            let mut projection_views = Vec::new();
            if frame_state.should_render {
                let views = match locate_submit_valid_views(
                    &session,
                    frame_state.predicted_display_time,
                    &stage,
                )? {
                    OesLocatedViews::SubmitValid(views) => views,
                    OesLocatedViews::NotSubmitValid(view_state_flags) => {
                        if frame_count.is_multiple_of(120) {
                            log_info(format!(
                                "Rusty XR OpenXR GLES skipped composition frame {} because OpenXR view pose is not valid yet viewFlags={:?}",
                                frame_count, view_state_flags
                            ));
                        }
                        end_empty_openxr_frame(
                            &mut frame_stream,
                            frame_state.predicted_display_time,
                            environment_blend_mode,
                            "end OpenXR frame without valid view pose",
                        )?;
                        frame_count = frame_count.saturating_add(1);
                        continue;
                    }
                };
                if let Some(probe) = surface_texture_oes_probe.as_mut() {
                    probe.update_textures(&egl, frame_count);
                }
                projection_state = projection_runtime_controller.refresh_state(frame_count);
                let projection_context = projection_frame_context_from_state(
                    "LOCAL",
                    frame_state.predicted_display_time,
                    &views,
                    projection_state,
                    surface_texture_oes_probe
                        .as_ref()
                        .and_then(|probe| probe.projection_metadata_pair()),
                );
                render_resources.render_eye_swapchains(
                    OesRenderFrameInputs {
                        egl: &egl,
                        swapchains: &mut swapchains,
                        frame_count,
                        status: &mut status,
                        surface_texture_oes_probe: surface_texture_oes_probe.as_ref(),
                        projection_plan: projection_context.projection_plan.as_ref(),
                        openxr_projection_fields: &projection_context.openxr_projection_fields,
                        projection_area_target_fields: &projection_context
                            .projection_area_target_fields,
                    },
                    OesRenderTuning::from_projection_state(
                        projection_state,
                        processing_layer,
                        blur_radius_px,
                        camera_color_controls,
                    ),
                )?;

                projection_views = projection_views_from_swapchains(&views, &swapchains);
            }

            if projection_views.is_empty() {
                end_empty_openxr_frame(
                    &mut frame_stream,
                    frame_state.predicted_display_time,
                    environment_blend_mode,
                    "end OpenXR frame without layers",
                )?;
            } else {
                end_projection_openxr_frame(
                    &mut frame_stream,
                    frame_state.predicted_display_time,
                    environment_blend_mode,
                    &stage,
                    &projection_views,
                    projection_uses_source_alpha,
                    native_passthrough_underlay.as_ref(),
                )?;
            }

            frame_count = frame_count.saturating_add(1);
            frame_rate_tracker.record_rendered_frame(frame_count, &mut status);
        }

        log_info("Rusty XR OpenXR GLES loop exited cleanly");
        Ok(())
    }

    pub(super) fn ensure_xr_success(
        result: xr::sys::Result,
        operation: &str,
    ) -> Result<(), String> {
        if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
            return Err(format!("{operation} failed: {result:?}"));
        }
        Ok(())
    }

    pub(super) fn log_status(status: &OpenXrGlesFeasibilityStatus) {
        match serde_json::to_string(status) {
            Ok(json) => log_info(format!("Rusty XR OpenXR GLES feasibility status {json}")),
            Err(error) => log_error(format!(
                "Rusty XR OpenXR GLES status serialization failed: {error}"
            )),
        }
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
    use super::{current_android_projection_property_config, status_json};
    use rusty_xr_runtime_config as rxrc;

    #[test]
    fn status_json_uses_public_schema() {
        let json = status_json();

        assert!(json.contains("rusty.xr.quest.openxr_gles_feasibility.v1"));
        assert!(json.contains("XR_KHR_opengl_es_enable"));
    }

    #[test]
    fn current_android_projection_properties_cover_canonical_surface() {
        let parsed = current_android_projection_property_config([
            ("debug.rustyxr.projection.depth.meters", "1.75"),
            ("debug.rustyxr.projection.area.radius.x.uv", "0.42"),
            ("debug.rustyxr.projection.border.policy", "solid-red"),
            ("debug.rustyxr.projection.alpha.mode", "source-alpha"),
            ("debug.rustyxr.source.visible.rect.width.uv", "0.875"),
            ("debug.rustyxr.source.eye.mapping", "right-left"),
            ("debug.rustyxr.source.texture.transform.source", "metadata"),
        ]);

        assert_eq!(
            parsed.config.get(rxrc::KEY_PROJECTION_DEPTH_METERS),
            Some(&rxrc::RuntimeValue::Float(1.75))
        );
        assert_eq!(
            parsed.config.get(rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV),
            Some(&rxrc::RuntimeValue::Float(0.42))
        );
        assert_eq!(
            parsed.config.get(rxrc::KEY_PROJECTION_BORDER_POLICY),
            Some(&rxrc::RuntimeValue::Text("solid-red".to_string()))
        );
        assert_eq!(
            parsed.config.get(rxrc::KEY_PROJECTION_ALPHA_MODE),
            Some(&rxrc::RuntimeValue::Text("source-alpha".to_string()))
        );
        assert_eq!(
            parsed.config.get(rxrc::KEY_SOURCE_VISIBLE_RECT_WIDTH_UV),
            Some(&rxrc::RuntimeValue::Float(0.875))
        );
        assert_eq!(
            parsed.config.get(rxrc::KEY_SOURCE_EYE_MAPPING),
            Some(&rxrc::RuntimeValue::Text("right-left".to_string()))
        );
        assert_eq!(
            parsed.config.get(rxrc::KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE),
            Some(&rxrc::RuntimeValue::Text("metadata".to_string()))
        );
        assert_eq!(parsed.aliases.len(), 7);
    }

    #[test]
    fn current_android_projection_properties_reject_legacy_makepad_aliases() {
        let parsed = current_android_projection_property_config([
            (
                "debug.rustyxr.makepad.projection.area.offset.left.uv",
                "0.125",
            ),
            ("debug.rustyxr.projection.area.left.offset.x.uv", "0.25"),
        ]);

        assert_eq!(
            parsed
                .config
                .get(rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV),
            Some(&rxrc::RuntimeValue::Float(0.25))
        );
        assert_eq!(parsed.aliases.len(), 1);
        assert_eq!(
            parsed.aliases[0].status,
            rxrc::RuntimeKeyAliasStatus::Current
        );
    }

    #[test]
    fn current_android_projection_properties_ignore_invalid_values() {
        let parsed = current_android_projection_property_config([
            ("debug.rustyxr.projection.depth.meters", "not-a-number"),
            ("debug.rustyxr.projection.area.radius.x.uv", "0.42"),
        ]);

        assert_eq!(parsed.config.get(rxrc::KEY_PROJECTION_DEPTH_METERS), None);
        assert_eq!(
            parsed.config.get(rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV),
            Some(&rxrc::RuntimeValue::Float(0.42))
        );
        assert_eq!(parsed.aliases.len(), 1);
    }
}
