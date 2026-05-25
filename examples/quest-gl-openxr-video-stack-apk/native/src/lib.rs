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
    use openxr::sys::Handle as _;
    use rusty_xr_quest_diagnostics::{
        OpenXrGlesExtensionStatus, OpenXrGlesFeasibilityState, OpenXrGlesGraphicsRequirements,
        OPENXR_GLES_EXTENSION,
    };
    use std::{
        ffi::CString,
        os::raw::{c_char, c_int, c_void},
        ptr,
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
    use egl_gles_context::EglContext;
    use openxr_gles_config::OesActivityConfig;
    use openxr_gles_passthrough::create_openxr_gles_passthrough_underlay;
    use openxr_gles_renderer::{
        projection_views_from_swapchains, OesRenderFrameInputs, OesRenderResources, OesRenderTuning,
    };
    use openxr_gles_resources::{
        create_eye_swapchains, gl_format_label, select_environment_blend_mode,
    };
    use openxr_gles_session::{
        begin_openxr_frame, create_android_instance, end_empty_openxr_frame,
        initialize_android_loader, locate_submit_valid_views, poll_openxr_session_events,
        request_session_exit_if_app_stopped, OesFrameRateTracker, OesLocatedViews,
    };
    use projection_geometry::{
        openxr_projection_contract_fields, projection_area_target_marker_fields_from_state,
    };
    use projection_runtime::{
        log_oes_projection_runtime_manifest, oes_projection_runtime_hotload_log_message,
        oes_projection_runtime_resolution_enabled, oes_projection_runtime_resolution_from_state,
        oes_projection_runtime_state_from_resolution, oes_projection_tuning_hotload_log_message,
    };
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
        let projection_runtime =
            oes_projection_runtime_resolution_from_state(activity_projection_state);
        let projection_runtime_resolution_enabled = oes_projection_runtime_resolution_enabled(&app);
        let mut projection_state = if projection_runtime_resolution_enabled {
            oes_projection_runtime_state_from_resolution(
                activity_projection_state,
                &projection_runtime.resolution,
            )
        } else {
            activity_projection_state.with_legacy_system_properties()
        };
        log_oes_projection_runtime_manifest(
            "startup",
            &projection_runtime,
            projection_runtime_resolution_enabled,
        );
        let projection_depth_meters = projection_state.tuning.projection_depth_meters;
        let projection_preview_fov_y_degrees = projection_state.tuning.camera_preview_fov_y_degrees;
        let projection_preview_offset_y_meters =
            projection_state.tuning.camera_preview_offset_y_meters;
        let projection_raw_overscan = projection_state.tuning.camera_raw_overlay_overscan;
        let projection_area_offset_x_uv = projection_state.projection_area_offset_uv[0];
        let projection_area_offset_y_uv = projection_state.projection_area_offset_uv[1];
        let projection_uses_source_alpha = projection_state
            .projection_border_policy
            .needs_source_alpha(
                projection_state.projection_area_opacity,
                projection_state.projection_border_opacity,
                projection_state.projection_alpha_mode,
            );
        let native_passthrough_underlay_requested = projection_uses_source_alpha;
        if projection_state.tuning != base_projection_tuning {
            let tuning_source = if projection_runtime_resolution_enabled {
                "resolved-projection-runtime"
            } else {
                "android-system-property"
            };
            log_info(oes_projection_tuning_hotload_log_message(
                tuning_source,
                0,
                projection_state.tuning,
            ));
        }

        let entry = unsafe { xr::Entry::load().map_err(|error| format!("load OpenXR: {error}"))? };
        initialize_android_loader(&entry, &app)?;
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
        // Repeat the resolved runtime manifest at lifecycle boundaries where
        // OpenXR state has changed. The validation harness owns log capture
        // timing; renderer cadence must not be changed just to satisfy a tail.
        log_oes_projection_runtime_manifest(
            "openxr-runtime",
            &projection_runtime,
            projection_runtime_resolution_enabled,
        );

        let system = xr_instance
            .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
            .map_err(|error| format!("get HMD system: {error}"))?;
        let projection_area_target_fields =
            projection_area_target_marker_fields_from_state(projection_state);
        log_info(format!(
            "Rusty XR OpenXR GLES projection border policy={} processingLayer={} cameraProjectionMode={} cameraBlurRadiusPx={:.3} projectionDepthMeters={:.3} cameraPreviewFovYDegrees={:.3} cameraPreviewOffsetYMeters={:.3} cameraRawOverlayOverscan={:.3} projectionAreaOffsetXUv={:.6} projectionAreaOffsetYUv={:.6} projectionAreaLeftOffsetXUv={:.6} projectionAreaLeftOffsetYUv={:.6} projectionAreaRightOffsetXUv={:.6} projectionAreaRightOffsetYUv={:.6} projectionAreaScale={:.6},{:.6} projectionAreaRadiusUv={:.6},{:.6} projectionAreaCornerRadiusUv={:.6} projectionAreaOpacity={:.3} projectionBorderOpacity={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} {} nativePassthroughUnderlayRequested={} nativePassthroughExtensionEnabled={} oesSourceColorTransfer={} sourceColorInputEncoding={} sourceColorOutputEncoding={} cameraColorMatrix={:?} cameraColorOffset={:?} cameraColorContrast={:.3} cameraColorBrightness={:.3} cameraColorSaturation={:.3}",
            projection_state.projection_border_policy.stable_id(),
            processing_layer.stable_id(),
            projection_state.camera_projection_mode.stable_id(),
            blur_radius_px,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
            projection_area_offset_x_uv,
            projection_area_offset_y_uv,
            projection_state.projection_area_eye_offset_uv[0][0],
            projection_state.projection_area_eye_offset_uv[0][1],
            projection_state.projection_area_eye_offset_uv[1][0],
            projection_state.projection_area_eye_offset_uv[1][1],
            projection_state.projection_area_scale[0],
            projection_state.projection_area_scale[1],
            projection_state.projection_area_radius[0],
            projection_state.projection_area_radius[1],
            projection_state.projection_area_corner_radius_uv,
            projection_state.projection_area_opacity,
            projection_state.projection_border_opacity,
            projection_state.projection_alpha_mode.stable_id(),
            projection_state.projection_alpha_scale,
            projection_state.projection_alpha_bias,
            projection_area_target_fields,
            native_passthrough_underlay_requested,
            enabled_extensions.fb_passthrough,
            camera_color_controls.source_transfer.stable_id(),
            camera_color_controls.source_transfer.input_encoding(),
            camera_color_controls.source_transfer.output_encoding(),
            camera_color_controls.matrix,
            camera_color_controls.offset,
            camera_color_controls.contrast,
            camera_color_controls.brightness,
            camera_color_controls.saturation
        ));
        let environment_blend_mode = select_environment_blend_mode(
            &xr_instance,
            system,
            &mut status,
            projection_state.projection_border_policy.stable_id(),
            projection_uses_source_alpha,
        )?;
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
        log_oes_projection_runtime_manifest(
            "session-ready",
            &projection_runtime,
            projection_runtime_resolution_enabled,
        );
        let native_passthrough_underlay = if native_passthrough_underlay_requested {
            match create_openxr_gles_passthrough_underlay(&xr_instance, &session) {
                Ok(underlay) => {
                    status.notes.push(String::from(
                        "nativePassthroughUnderlay=true; passthrough is submitted as XR_FB_passthrough below the projection layer with OPAQUE environment blend",
                    ));
                    log_info(
                        "Rusty XR OpenXR GLES native passthrough underlay active via XR_FB_passthrough",
                    );
                    Some(underlay)
                }
                Err(error) => {
                    status
                        .issue_codes
                        .push(String::from("create.XR_FB_passthrough.failed"));
                    status.notes.push(format!(
                        "nativePassthroughUnderlay=false; XR_FB_passthrough create/start failed: {error}"
                    ));
                    log_error(format!(
                        "Rusty XR OpenXR GLES native passthrough underlay failed: {error}"
                    ));
                    None
                }
            }
        } else {
            None
        };

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
                let next_projection_state = if projection_runtime_resolution_enabled {
                    let runtime =
                        oes_projection_runtime_resolution_from_state(activity_projection_state);
                    oes_projection_runtime_state_from_resolution(
                        activity_projection_state,
                        &runtime.resolution,
                    )
                } else {
                    activity_projection_state.with_legacy_system_properties()
                };
                if next_projection_state != projection_state {
                    let tuning_source = if projection_runtime_resolution_enabled {
                        "resolved-projection-runtime"
                    } else {
                        "android-system-property"
                    };
                    projection_state = next_projection_state;
                    log_info(oes_projection_runtime_hotload_log_message(
                        tuning_source,
                        frame_count,
                        projection_state,
                    ));
                }
                let projection_area_target_fields =
                    projection_area_target_marker_fields_from_state(projection_state);
                let projection_plan = surface_texture_oes_probe.as_ref().and_then(|probe| {
                    probe.projection_plan_from_xr_views(&views, projection_state)
                });
                let openxr_projection_fields = openxr_projection_contract_fields(
                    "LOCAL",
                    frame_state.predicted_display_time,
                    &views,
                );
                render_resources.render_eye_swapchains(
                    OesRenderFrameInputs {
                        egl: &egl,
                        swapchains: &mut swapchains,
                        frame_count,
                        status: &mut status,
                        surface_texture_oes_probe: surface_texture_oes_probe.as_ref(),
                        projection_plan: projection_plan.as_ref(),
                        openxr_projection_fields: &openxr_projection_fields,
                        projection_area_target_fields: &projection_area_target_fields,
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
                let layer = xr::CompositionLayerProjection::new()
                    .layer_flags(if projection_uses_source_alpha {
                        xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA
                    } else {
                        xr::CompositionLayerFlags::EMPTY
                    })
                    .space(&stage)
                    .views(&projection_views);
                let passthrough_layer = native_passthrough_underlay.as_ref().map(|underlay| {
                    xr::sys::CompositionLayerPassthroughFB {
                        ty: xr::sys::CompositionLayerPassthroughFB::TYPE,
                        next: ptr::null(),
                        flags: xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
                        space: xr::sys::Space::NULL,
                        layer_handle: underlay.layer,
                    }
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
                    .end(
                        frame_state.predicted_display_time,
                        environment_blend_mode,
                        &layers,
                    )
                    .map_err(|error| format!("end OpenXR frame: {error}"))?;
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
