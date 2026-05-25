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
    use android_activity::{InputStatus, MainEvent, PollEvent};
    use jni::{
        objects::{JObject, JString, JValue},
        sys::jobject,
        JNIEnv, JavaVM,
    };
    use openxr as xr;
    use openxr::sys::Handle as _;
    use rusty_xr_camera_model::{
        ColorRgba, ProjectionBorderDescriptor, ProjectionBorderFillPolicy,
    };
    use rusty_xr_contracts::InvalidProjectionFillPolicy;
    use rusty_xr_quest_diagnostics::{
        FrameRateSummary, OpenXrGlesExtensionStatus, OpenXrGlesFeasibilityState,
        OpenXrGlesGraphicsRequirements, OPENXR_GLES_EXTENSION,
    };
    use std::{
        ffi::{CStr, CString},
        os::raw::{c_char, c_int, c_void},
        ptr,
        time::{Duration, Instant},
    };

    mod egl_gles_context;
    mod oes_copy_renderer;
    mod openxr_gles_passthrough;
    mod openxr_gles_renderer;
    mod openxr_gles_resources;
    mod projection_geometry;
    mod projection_runtime;
    mod source_metadata;
    mod surface_texture_oes_probe;
    use egl_gles_context::EglContext;
    use oes_copy_renderer::{GlFramebuffer, OesColorControls, OesCopyRenderer};
    use openxr_gles_passthrough::create_openxr_gles_passthrough_underlay;
    use openxr_gles_renderer::{render_eye_swapchains, OesRenderFrameInputs, OesRenderTuning};
    use openxr_gles_resources::{
        create_eye_swapchains, gl_format_label, select_environment_blend_mode,
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
    const DIAGNOSTIC_BLUR_SOURCE_WIDTH_PX: f32 = 1280.0;
    const DIAGNOSTIC_BLUR_SOURCE_HEIGHT_PX: f32 = 1280.0;
    const OES_COPY_RENDER_PATH: &str = "oes-full-surface-copy";
    const OES_PROJECTED_RENDER_PATH: &str = "oes-projected-camera-uv";
    const DIRECT_CAMERA2_OES_SOURCE: &str = "app.camera2_oes_surface_texture";
    const DEFAULT_PROJECTION_TARGET_DEPTH_METERS: f32 = 1.0;
    const PROJECTION_PREVIEW_FOV_Y_DEGREES: f32 = 60.0;
    const PROJECTION_RAW_OVERSCAN: f32 = 1.06;
    const PROJECTION_SOURCE_ASPECT: f32 = 1.0;

    fn diagnostic_blur_source_texel_size() -> [f32; 2] {
        [
            1.0 / DIAGNOSTIC_BLUR_SOURCE_WIDTH_PX.max(1.0),
            1.0 / DIAGNOSTIC_BLUR_SOURCE_HEIGHT_PX.max(1.0),
        ]
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    enum OesProjectionBorderPolicy {
        #[default]
        SolidRed,
        PassthroughUnderlay,
    }

    impl OesProjectionBorderPolicy {
        fn parse(value: &str) -> Option<Self> {
            match value.trim().to_ascii_lowercase().as_str() {
                "solid-red" => Some(Self::SolidRed),
                "passthrough-underlay" => Some(Self::PassthroughUnderlay),
                _ => None,
            }
        }

        const fn stable_id(self) -> &'static str {
            match self {
                Self::SolidRed => "solid-red",
                Self::PassthroughUnderlay => "passthrough-underlay",
            }
        }

        const fn shader_id(self) -> c_int {
            match self {
                Self::SolidRed => 0,
                Self::PassthroughUnderlay => 1,
            }
        }

        const fn uses_source_alpha(self) -> bool {
            matches!(self, Self::PassthroughUnderlay)
        }

        fn needs_source_alpha(
            self,
            projection_area_opacity: f32,
            projection_border_opacity: f32,
            projection_alpha_mode: OesProjectionAlphaMode,
        ) -> bool {
            self.uses_source_alpha()
                || projection_area_opacity < 0.999
                || projection_border_opacity < 0.999
                || projection_alpha_mode.uses_dynamic_alpha()
        }

        const fn clear_color(self) -> (f32, f32, f32, f32) {
            match self {
                Self::SolidRed => (1.0, 0.0, 0.0, 1.0),
                Self::PassthroughUnderlay => (0.0, 0.0, 0.0, 0.0),
            }
        }

        const fn shared_fill_policy(self) -> ProjectionBorderFillPolicy {
            match self {
                Self::SolidRed => ProjectionBorderFillPolicy::SolidColor,
                Self::PassthroughUnderlay => ProjectionBorderFillPolicy::PassthroughUnderlay,
            }
        }

        fn shared_descriptor(self, opacity: f32) -> ProjectionBorderDescriptor {
            let (r, g, b, a) = self.clear_color();
            ProjectionBorderDescriptor::new(
                self.shared_fill_policy(),
                ColorRgba::new(r, g, b, a),
                opacity.clamp(0.0, 1.0),
            )
        }

        const fn invalid_source_uv_fill_policy(self) -> InvalidProjectionFillPolicy {
            match self {
                Self::SolidRed => InvalidProjectionFillPolicy::SolidRed,
                Self::PassthroughUnderlay => InvalidProjectionFillPolicy::Transparent,
            }
        }
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    enum OesSourceColorTransfer {
        Identity,
        #[default]
        SrgbToLinear,
    }

    impl OesSourceColorTransfer {
        fn parse(value: &str) -> Option<Self> {
            match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
                "identity" => Some(Self::Identity),
                "srgb-to-linear" => Some(Self::SrgbToLinear),
                _ => None,
            }
        }

        const fn stable_id(self) -> &'static str {
            match self {
                Self::Identity => "identity",
                Self::SrgbToLinear => "srgb-to-linear",
            }
        }

        const fn shader_id(self) -> c_int {
            match self {
                Self::Identity => 0,
                Self::SrgbToLinear => 1,
            }
        }

        const fn input_encoding(self) -> &'static str {
            match self {
                Self::Identity => "linear-or-renderer-native-rgb",
                Self::SrgbToLinear => "external-oes-srgb-nonlinear-rgb",
            }
        }

        const fn output_encoding(self) -> &'static str {
            match self {
                Self::Identity => "unchanged-rgb",
                Self::SrgbToLinear => "linear-rgb",
            }
        }
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    enum OesProjectionAlphaMode {
        #[default]
        Fixed,
        Red,
        Green,
        Blue,
        Luma,
        InverseRed,
        InverseGreen,
        InverseBlue,
        InverseLuma,
        RedDominance,
        GreenDominance,
        BlueDominance,
        Saturation,
        InverseSaturation,
    }

    impl OesProjectionAlphaMode {
        fn parse(value: &str) -> Option<Self> {
            match value.trim().to_ascii_lowercase().as_str() {
                "" | "fixed" | "none" | "constant" | "area-opacity" | "opacity" => {
                    Some(Self::Fixed)
                }
                "red" | "r" | "channel-r" => Some(Self::Red),
                "green" | "g" | "channel-g" => Some(Self::Green),
                "blue" | "b" | "channel-b" => Some(Self::Blue),
                "luma" | "luminance" | "brightness" | "value" => Some(Self::Luma),
                "inverse-red" | "red-inverse" | "inv-red" | "one-minus-red" | "1-red" | "1-r" => {
                    Some(Self::InverseRed)
                }
                "inverse-green" | "green-inverse" | "inv-green" | "one-minus-green" | "1-green"
                | "1-g" => Some(Self::InverseGreen),
                "inverse-blue" | "blue-inverse" | "inv-blue" | "one-minus-blue" | "1-blue"
                | "1-b" => Some(Self::InverseBlue),
                "inverse-luma" | "luma-inverse" | "inv-luma" | "inverse-brightness"
                | "one-minus-luma" | "1-luma" | "1-brightness" => Some(Self::InverseLuma),
                "red-dominance" | "dominant-red" | "red-key" | "red-chroma" | "red-minus-max" => {
                    Some(Self::RedDominance)
                }
                "green-dominance" | "dominant-green" | "green-key" | "green-chroma"
                | "green-minus-max" | "screen-green" => Some(Self::GreenDominance),
                "blue-dominance" | "dominant-blue" | "blue-key" | "blue-chroma"
                | "blue-minus-max" => Some(Self::BlueDominance),
                "saturation" | "chroma" | "max-min" | "colorfulness" => Some(Self::Saturation),
                "inverse-saturation"
                | "saturation-inverse"
                | "inverse-chroma"
                | "inv-chroma"
                | "one-minus-saturation"
                | "1-saturation" => Some(Self::InverseSaturation),
                _ => None,
            }
        }

        const fn stable_id(self) -> &'static str {
            match self {
                Self::Fixed => "fixed",
                Self::Red => "red",
                Self::Green => "green",
                Self::Blue => "blue",
                Self::Luma => "luma",
                Self::InverseRed => "inverse-red",
                Self::InverseGreen => "inverse-green",
                Self::InverseBlue => "inverse-blue",
                Self::InverseLuma => "inverse-luma",
                Self::RedDominance => "red-dominance",
                Self::GreenDominance => "green-dominance",
                Self::BlueDominance => "blue-dominance",
                Self::Saturation => "saturation",
                Self::InverseSaturation => "inverse-saturation",
            }
        }

        const fn shader_id(self) -> c_int {
            match self {
                Self::Fixed => 0,
                Self::Red => 1,
                Self::Green => 2,
                Self::Blue => 3,
                Self::Luma => 4,
                Self::InverseRed => 5,
                Self::InverseGreen => 6,
                Self::InverseBlue => 7,
                Self::InverseLuma => 8,
                Self::RedDominance => 9,
                Self::GreenDominance => 10,
                Self::BlueDominance => 11,
                Self::Saturation => 12,
                Self::InverseSaturation => 13,
            }
        }

        const fn uses_dynamic_alpha(self) -> bool {
            !matches!(self, Self::Fixed)
        }
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    enum OesCameraProjectionMode {
        #[default]
        DisplayScreenHomography,
        WorldCanvas,
    }

    impl OesCameraProjectionMode {
        fn parse(value: &str) -> Option<Self> {
            match value.trim() {
                ""
                | "display-screen-homography"
                | "screen-homography"
                | "display-eye-homography"
                | "fullscreen"
                | "custom"
                | "default" => Some(Self::DisplayScreenHomography),
                "world-canvas" | "worldCanvas" | "world-space-canvas" | "world-space-quad"
                | "mesh-quad" | "actual-quad" | "canvas" => Some(Self::WorldCanvas),
                _ => None,
            }
        }

        const fn stable_id(self) -> &'static str {
            match self {
                Self::DisplayScreenHomography => "display-screen-homography",
                Self::WorldCanvas => "world-canvas",
            }
        }

        const fn uses_world_canvas(self) -> bool {
            matches!(self, Self::WorldCanvas)
        }
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    enum OesContentMappingMode {
        #[default]
        CameraProjection,
        #[allow(dead_code)]
        FullFrameStimulusToProjectionArea,
        FullFrameStimulusToSurfaceHomography,
    }

    impl OesContentMappingMode {
        const fn shader_id(self) -> c_int {
            match self {
                Self::CameraProjection => 0,
                Self::FullFrameStimulusToProjectionArea => 1,
                Self::FullFrameStimulusToSurfaceHomography => 0,
            }
        }

        const fn stable_id(self) -> &'static str {
            match self {
                Self::CameraProjection => "camera-projection-homography",
                Self::FullFrameStimulusToProjectionArea => "full-frame-stimulus-to-projection-area",
                Self::FullFrameStimulusToSurfaceHomography => {
                    "full-frame-stimulus-to-surface-homography"
                }
            }
        }
    }

    const OES_TUNING_PROP_PROJECTION_DEPTH_METERS: &str = "debug.rustyxr.projection.depth.meters";
    const OES_TUNING_PROP_CAMERA_PREVIEW_FOV_Y_DEGREES: &str =
        "debug.rustyxr.camera.preview.fov.y.degrees";
    const OES_TUNING_PROP_CAMERA_PREVIEW_OFFSET_Y_METERS: &str =
        "debug.rustyxr.camera.preview.offset.y.meters";
    const OES_TUNING_PROP_CAMERA_RAW_OVERLAY_OVERSCAN: &str =
        "debug.rustyxr.camera.raw.overlay.overscan";
    const OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_PROP: &str =
        "debug.rustyxr.oes.projection.runtime.resolution.enabled";
    const OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_EXTRA: &str =
        "rustyxr.projectionRuntimeResolutionEnabled";

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct OesProjectionTuning {
        projection_depth_meters: f32,
        camera_preview_fov_y_degrees: f32,
        camera_preview_offset_y_meters: f32,
        camera_raw_overlay_overscan: f32,
    }

    impl OesProjectionTuning {
        fn from_activity(app: &android_activity::AndroidApp) -> Self {
            Self {
                projection_depth_meters: projection_depth_meters_from_activity(app),
                camera_preview_fov_y_degrees: projection_preview_fov_y_degrees_from_activity(app),
                camera_preview_offset_y_meters: projection_preview_offset_y_meters_from_activity(
                    app,
                ),
                camera_raw_overlay_overscan: projection_raw_overscan_from_activity(app),
            }
        }

        fn with_system_properties(self) -> Self {
            Self {
                projection_depth_meters: android_system_property_f32(
                    OES_TUNING_PROP_PROJECTION_DEPTH_METERS,
                    self.projection_depth_meters,
                    0.05,
                    10.0,
                ),
                camera_preview_fov_y_degrees: android_system_property_f32(
                    OES_TUNING_PROP_CAMERA_PREVIEW_FOV_Y_DEGREES,
                    self.camera_preview_fov_y_degrees,
                    1.0,
                    175.0,
                ),
                camera_preview_offset_y_meters: android_system_property_f32(
                    OES_TUNING_PROP_CAMERA_PREVIEW_OFFSET_Y_METERS,
                    self.camera_preview_offset_y_meters,
                    -2.0,
                    2.0,
                ),
                camera_raw_overlay_overscan: android_system_property_f32(
                    OES_TUNING_PROP_CAMERA_RAW_OVERLAY_OVERSCAN,
                    self.camera_raw_overlay_overscan,
                    1.0,
                    16.0,
                ),
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct OesProjectionRuntimeState {
        tuning: OesProjectionTuning,
        projection_area_offset_uv: [f32; 2],
        projection_area_eye_offset_uv: [[f32; 2]; 2],
        projection_area_scale: [f32; 2],
        projection_area_radius: [f32; 2],
        projection_area_corner_radius_uv: f32,
        projection_area_opacity: f32,
        projection_border_opacity: f32,
        projection_alpha_mode: OesProjectionAlphaMode,
        projection_alpha_scale: f32,
        projection_alpha_bias: f32,
        camera_projection_mode: OesCameraProjectionMode,
        projection_border_policy: OesProjectionBorderPolicy,
    }

    impl OesProjectionRuntimeState {
        fn with_legacy_system_properties(self) -> Self {
            Self {
                tuning: self.tuning.with_system_properties(),
                ..self
            }
        }
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    enum OesProcessingLayer {
        #[default]
        Raw,
        Blur,
    }

    impl OesProcessingLayer {
        fn parse(value: &str) -> Option<Self> {
            match value.trim().to_ascii_lowercase().as_str() {
                "raw" => Some(Self::Raw),
                "blur" => Some(Self::Blur),
                _ => None,
            }
        }

        const fn stable_id(self) -> &'static str {
            match self {
                Self::Raw => "raw",
                Self::Blur => "blur",
            }
        }

        const fn shader_id(self) -> c_int {
            match self {
                Self::Raw => 0,
                Self::Blur => 1,
            }
        }
    }

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
        let processing_layer = processing_layer_from_activity(&app);
        let blur_radius_px = blur_radius_px_from_activity(&app);
        let base_projection_tuning = OesProjectionTuning::from_activity(&app);
        let projection_area_offset_x_uv = projection_area_offset_x_uv_from_activity(&app);
        let projection_area_offset_y_uv = projection_area_offset_y_uv_from_activity(&app);
        let projection_area_offset_uv = [projection_area_offset_x_uv, projection_area_offset_y_uv];
        let activity_projection_state = OesProjectionRuntimeState {
            tuning: base_projection_tuning,
            projection_area_offset_uv,
            projection_area_eye_offset_uv: projection_area_eye_offset_uv_from_activity(
                &app,
                projection_area_offset_uv,
            ),
            projection_area_scale: projection_area_scale_from_activity(&app),
            projection_area_radius: projection_area_radius_from_activity(&app),
            projection_area_corner_radius_uv: projection_area_corner_radius_uv_from_activity(&app),
            projection_area_opacity: projection_area_opacity_from_activity(&app),
            projection_border_opacity: projection_border_opacity_from_activity(&app),
            projection_alpha_mode: projection_alpha_mode_from_activity(&app),
            projection_alpha_scale: projection_alpha_scale_from_activity(&app),
            projection_alpha_bias: projection_alpha_bias_from_activity(&app),
            camera_projection_mode: camera_projection_mode_from_activity(&app),
            projection_border_policy: projection_border_policy_from_activity(&app),
        };
        let camera_color_controls = camera_color_controls_from_activity(&app);
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
                let (view_state_flags, views) = session
                    .locate_views(VIEW_TYPE, frame_state.predicted_display_time, &stage)
                    .map_err(|error| format!("locate OpenXR views: {error}"))?;
                let views_valid = view_state_flags.contains(xr::ViewStateFlags::ORIENTATION_VALID)
                    && view_state_flags.contains(xr::ViewStateFlags::POSITION_VALID)
                    && views.iter().all(view_pose_is_submit_valid);
                if !views_valid {
                    if frame_count.is_multiple_of(120) {
                        log_info(format!(
                            "Rusty XR OpenXR GLES skipped composition frame {} because OpenXR view pose is not valid yet viewFlags={:?}",
                            frame_count, view_state_flags
                        ));
                    }
                    frame_stream
                        .end(
                            frame_state.predicted_display_time,
                            environment_blend_mode,
                            &[],
                        )
                        .map_err(|error| {
                            format!("end OpenXR frame without valid view pose: {error}")
                        })?;
                    frame_count = frame_count.saturating_add(1);
                    continue;
                }
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
                    probe.projection_plan_from_xr_views(
                        &views,
                        projection_state.camera_projection_mode,
                        projection_state.projection_area_eye_offset_uv,
                        projection_state.projection_area_scale,
                        projection_state.projection_area_radius,
                        projection_state.projection_area_opacity,
                        projection_state.projection_border_policy,
                        projection_state.projection_border_opacity,
                        projection_state.tuning.projection_depth_meters,
                        projection_state.tuning.camera_preview_fov_y_degrees,
                        projection_state.tuning.camera_preview_offset_y_meters,
                        projection_state.tuning.camera_raw_overlay_overscan,
                    )
                });
                let openxr_projection_fields = openxr_projection_contract_fields(
                    "LOCAL",
                    frame_state.predicted_display_time,
                    &views,
                );
                render_eye_swapchains(
                    OesRenderFrameInputs {
                        egl: &egl,
                        fbo: &mut fbo,
                        swapchains: &mut swapchains,
                        frame_count,
                        status: &mut status,
                        surface_texture_oes_probe: surface_texture_oes_probe.as_ref(),
                        projection_plan: projection_plan.as_ref(),
                        oes_copy_renderer: &mut oes_copy_renderer,
                        openxr_projection_fields: &openxr_projection_fields,
                        projection_area_target_fields: &projection_area_target_fields,
                    },
                    OesRenderTuning {
                        projection_border_policy: projection_state.projection_border_policy,
                        processing_layer,
                        blur_radius_px,
                        projection_area_eye_offset_uv: projection_state
                            .projection_area_eye_offset_uv,
                        projection_area_scale: projection_state.projection_area_scale,
                        projection_area_radius: projection_state.projection_area_radius,
                        projection_area_corner_radius_uv: projection_state
                            .projection_area_corner_radius_uv,
                        projection_area_opacity: projection_state.projection_area_opacity,
                        projection_border_opacity: projection_state.projection_border_opacity,
                        projection_alpha_mode: projection_state.projection_alpha_mode,
                        projection_alpha_scale: projection_state.projection_alpha_scale,
                        projection_alpha_bias: projection_state.projection_alpha_bias,
                        camera_color_controls,
                    },
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

    pub(super) fn ensure_xr_success(
        result: xr::sys::Result,
        operation: &str,
    ) -> Result<(), String> {
        if result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
            return Err(format!("{operation} failed: {result:?}"));
        }
        Ok(())
    }

    fn activity_string_extra(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        key: &str,
    ) -> Option<String> {
        let intent = env
            .call_method(activity, "getIntent", "()Landroid/content/Intent;", &[])
            .and_then(|value| value.l())
            .ok()?;
        if intent.is_null() {
            return None;
        }
        let key = env.new_string(key).ok()?;
        let key_object = JObject::from(key);
        let extras = env
            .call_method(&intent, "getExtras", "()Landroid/os/Bundle;", &[])
            .and_then(|value| value.l())
            .ok()?;
        if extras.is_null() {
            return None;
        }
        let value = env
            .call_method(
                &extras,
                "get",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&key_object)],
            )
            .and_then(|value| value.l())
            .ok()?;
        if value.is_null() {
            return None;
        }
        let value_string = env
            .call_method(&value, "toString", "()Ljava/lang/String;", &[])
            .and_then(|value| value.l())
            .ok()?;
        if value_string.is_null() {
            return None;
        }
        env.get_string(&JString::from(value_string))
            .map(|value| value.to_string_lossy().into_owned())
            .ok()
    }

    fn projection_border_policy_from_activity(
        app: &android_activity::AndroidApp,
    ) -> OesProjectionBorderPolicy {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return OesProjectionBorderPolicy::default();
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return OesProjectionBorderPolicy::default();
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        let requested =
            activity_string_extra(&mut env, &activity, "rustyxr.projectionBorderPolicy");
        requested
            .as_deref()
            .and_then(OesProjectionBorderPolicy::parse)
            .unwrap_or_default()
    }

    fn processing_layer_from_activity(app: &android_activity::AndroidApp) -> OesProcessingLayer {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return OesProcessingLayer::default();
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return OesProcessingLayer::default();
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        let requested = activity_string_extra(&mut env, &activity, "rustyxr.processingLayer");
        requested
            .as_deref()
            .and_then(OesProcessingLayer::parse)
            .unwrap_or_default()
    }

    fn camera_projection_mode_from_activity(
        app: &android_activity::AndroidApp,
    ) -> OesCameraProjectionMode {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return OesCameraProjectionMode::default();
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return OesCameraProjectionMode::default();
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        let requested = activity_string_extra(&mut env, &activity, "rustyxr.cameraProjectionMode");
        requested
            .as_deref()
            .and_then(OesCameraProjectionMode::parse)
            .unwrap_or_default()
    }

    fn blur_radius_px_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 2.0;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 2.0;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.cameraBlurRadiusPx")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(2.0)
            .clamp(0.0, 16.0)
    }

    fn android_system_property_value(name: &str) -> Option<String> {
        #[link(name = "c")]
        unsafe extern "C" {
            fn __system_property_get(name: *const c_char, value: *mut c_char) -> c_int;
        }

        let name = CString::new(name).ok()?;
        let mut value = [0 as c_char; 128];
        let len = unsafe { __system_property_get(name.as_ptr(), value.as_mut_ptr()) };
        if len <= 0 {
            return None;
        }
        let value = unsafe { CStr::from_ptr(value.as_ptr()) }
            .to_string_lossy()
            .trim()
            .to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }

    fn android_system_property_f32(name: &str, default: f32, min: f32, max: f32) -> f32 {
        android_system_property_value(name)
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(min, max))
            .unwrap_or(default)
    }

    fn projection_depth_meters_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return DEFAULT_PROJECTION_TARGET_DEPTH_METERS;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return DEFAULT_PROJECTION_TARGET_DEPTH_METERS;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionDepthMeters")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(DEFAULT_PROJECTION_TARGET_DEPTH_METERS)
            .clamp(0.05, 10.0)
    }

    fn projection_preview_fov_y_degrees_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return PROJECTION_PREVIEW_FOV_Y_DEGREES;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return PROJECTION_PREVIEW_FOV_Y_DEGREES;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.cameraPreviewFovYDegrees")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(PROJECTION_PREVIEW_FOV_Y_DEGREES)
            .clamp(1.0, 175.0)
    }

    fn projection_preview_offset_y_meters_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 0.0;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 0.0;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.cameraPreviewOffsetYMeters")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-2.0, 2.0)
    }

    fn projection_raw_overscan_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return PROJECTION_RAW_OVERSCAN;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return PROJECTION_RAW_OVERSCAN;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.cameraRawOverlayOverscan")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(PROJECTION_RAW_OVERSCAN)
            .max(1.0)
    }

    fn projection_area_offset_x_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 0.0;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 0.0;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaOffsetXUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-0.5, 0.5)
    }

    fn projection_area_offset_y_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 0.0;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 0.0;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaOffsetYUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-0.5, 0.5)
    }

    fn activity_float_extra(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        keys: &[&str],
    ) -> Option<f32> {
        keys.iter()
            .find_map(|key| activity_string_extra(env, activity, key))
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
    }

    fn projection_area_eye_offset_uv_from_activity(
        app: &android_activity::AndroidApp,
        base_offset_uv: [f32; 2],
    ) -> [[f32; 2]; 2] {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return [base_offset_uv, base_offset_uv];
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return [base_offset_uv, base_offset_uv];
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        let left_x = activity_float_extra(
            &mut env,
            &activity,
            &["rustyxr.projectionAreaLeftOffsetXUv"],
        )
        .unwrap_or(base_offset_uv[0])
        .clamp(-0.5, 0.5);
        let left_y = activity_float_extra(
            &mut env,
            &activity,
            &["rustyxr.projectionAreaLeftOffsetYUv"],
        )
        .unwrap_or(base_offset_uv[1])
        .clamp(-0.5, 0.5);
        let right_x = activity_float_extra(
            &mut env,
            &activity,
            &["rustyxr.projectionAreaRightOffsetXUv"],
        )
        .unwrap_or(base_offset_uv[0])
        .clamp(-0.5, 0.5);
        let right_y = activity_float_extra(
            &mut env,
            &activity,
            &["rustyxr.projectionAreaRightOffsetYUv"],
        )
        .unwrap_or(base_offset_uv[1])
        .clamp(-0.5, 0.5);
        [[left_x, left_y], [right_x, right_y]]
    }

    fn projection_area_scale_from_activity(app: &android_activity::AndroidApp) -> [f32; 2] {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return [1.0, 1.0];
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return [1.0, 1.0];
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        let uniform_scale =
            activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaScaleUv")
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite())
                .unwrap_or(1.0)
                .clamp(0.05, 4.0);
        let scale_x = activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaScaleX")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(uniform_scale)
            .clamp(0.05, 4.0);
        let scale_y = activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaScaleY")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(uniform_scale)
            .clamp(0.05, 4.0);
        [scale_x, scale_y]
    }

    fn projection_area_radius_from_activity(app: &android_activity::AndroidApp) -> [f32; 2] {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return [0.47, 0.36];
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return [0.47, 0.36];
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        let radius_x =
            activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaRadiusXUv")
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite())
                .unwrap_or(0.47)
                .clamp(0.05, 0.5);
        let radius_y =
            activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaRadiusYUv")
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite())
                .unwrap_or(0.36)
                .clamp(0.05, 0.5);
        [radius_x, radius_y]
    }

    fn projection_area_corner_radius_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 0.08;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 0.08;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaCornerRadiusUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.08)
            .clamp(0.0, 0.5)
    }

    fn projection_area_opacity_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 1.0;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 1.0;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaOpacity")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    }

    fn projection_border_opacity_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 1.0;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 1.0;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionBorderOpacity")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    }

    fn projection_alpha_mode_from_activity(
        app: &android_activity::AndroidApp,
    ) -> OesProjectionAlphaMode {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return OesProjectionAlphaMode::default();
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return OesProjectionAlphaMode::default();
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionAlphaMode")
            .as_deref()
            .and_then(OesProjectionAlphaMode::parse)
            .unwrap_or_default()
    }

    fn projection_alpha_scale_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 1.0;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 1.0;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionAlphaScale")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 4.0)
    }

    fn projection_alpha_bias_from_activity(app: &android_activity::AndroidApp) -> f32 {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return 0.0;
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return 0.0;
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        activity_string_extra(&mut env, &activity, "rustyxr.projectionAlphaBias")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-1.0, 1.0)
    }

    fn camera_color_controls_from_activity(app: &android_activity::AndroidApp) -> OesColorControls {
        let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return OesColorControls::default();
        };
        let Ok(mut env) = java_vm.attach_current_thread() else {
            return OesColorControls::default();
        };
        let activity = unsafe {
            JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject)
        };
        let defaults = OesColorControls::default();
        let matrix = activity_string_extra(&mut env, &activity, "rustyxr.cameraColorMatrix")
            .as_deref()
            .map(parse_color_matrix)
            .unwrap_or(defaults.matrix);
        let offset = activity_string_extra(&mut env, &activity, "rustyxr.cameraColorOffset")
            .as_deref()
            .map(parse_color_offset)
            .unwrap_or(defaults.offset);
        let contrast = activity_string_extra(&mut env, &activity, "rustyxr.cameraColorContrast")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.contrast)
            .clamp(0.0, 4.0);
        let brightness =
            activity_string_extra(&mut env, &activity, "rustyxr.cameraColorBrightness")
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite())
                .unwrap_or(defaults.brightness)
                .clamp(-1.0, 1.0);
        let saturation =
            activity_string_extra(&mut env, &activity, "rustyxr.cameraColorSaturation")
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite())
                .unwrap_or(defaults.saturation)
                .clamp(0.0, 4.0);
        let source_transfer =
            activity_string_extra(&mut env, &activity, "rustyxr.oesSourceColorTransfer")
                .as_deref()
                .and_then(OesSourceColorTransfer::parse)
                .unwrap_or(defaults.source_transfer);
        OesColorControls {
            matrix,
            offset,
            contrast,
            brightness,
            saturation,
            source_transfer,
        }
    }

    fn parse_color_components(value: &str) -> Vec<f32> {
        value
            .split([';', ',', ' '])
            .filter(|item| !item.trim().is_empty())
            .filter_map(|item| item.trim().parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .collect()
    }

    fn parse_color_matrix(value: &str) -> [[f32; 3]; 3] {
        let values = parse_color_components(value);
        if values.len() != 9 {
            return OesColorControls::default().matrix;
        }
        [
            [values[0], values[1], values[2]],
            [values[3], values[4], values[5]],
            [values[6], values[7], values[8]],
        ]
    }

    fn parse_color_offset(value: &str) -> [f32; 3] {
        let values = parse_color_components(value);
        if values.len() != 3 {
            return OesColorControls::default().offset;
        }
        [
            values[0].clamp(-1.0, 1.0),
            values[1].clamp(-1.0, 1.0),
            values[2].clamp(-1.0, 1.0),
        ]
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

    fn uniform_location(program: u32, name: &str) -> Result<c_int, String> {
        let name_cstring =
            CString::new(name).map_err(|error| format!("uniform name CString: {error}"))?;
        let location = unsafe { glGetUniformLocation(program, name_cstring.as_ptr()) };
        if location < 0 {
            Err(format!("shader did not expose uniform {name}"))
        } else {
            Ok(location)
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

    fn view_pose_is_submit_valid(view: &xr::View) -> bool {
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

    fn log_status(status: &OpenXrGlesFeasibilityStatus) {
        match serde_json::to_string(status) {
            Ok(json) => log_info(format!("Rusty XR OpenXR GLES feasibility status {json}")),
            Err(error) => log_error(format!(
                "Rusty XR OpenXR GLES status serialization failed: {error}"
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
