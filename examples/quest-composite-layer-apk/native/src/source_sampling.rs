use crate::{
    HeadsetCameraFrameDiagnostics, RuntimeConfig, StereoGpuCameraFrame, StereoProjectionControls,
};
use rusty_xr_contracts::{
    SourceSamplerYAxis, SourceSamplingContract, SourceSamplingTransformStage, SourceUvRect, Vec2,
};

pub(crate) const HWB_SOURCE_UV_CONTRACT: &str =
    "screen_to_camera_content_uv_to_hardware_buffer_sampler";
const HWB_SOURCE_SAMPLING_BACKEND: &str = "hwb";
const HWB_SOURCE_SAMPLING_MODE: &str = "hwb-runtime";
const HWB_SAMPLE_TRANSFORM: &str = "sourceVisibleUvRect+cameraTextureTransformFlags";
const HWB_SAMPLE_TRANSFORM_OWNER: &str =
    "android-media-image-crop-rect+vulkan-hwb-camera_projection_shader";
const HWB_OUTPUT_UV_LABEL: &str = "hardware-buffer-sampler-uv";
const HWB_SAMPLER_UV_ORIGIN: &str = "hardware-buffer-import-convention";
const HWB_TEXTURE_TRANSFORM_OWNER: &str = "vulkan-hwb-camera_projection_shader";

pub(crate) struct HwbSourceSamplingHandoff<'a> {
    frame: &'a StereoGpuCameraFrame,
    controls: &'a StereoProjectionControls,
    config: &'a RuntimeConfig,
}

impl<'a> HwbSourceSamplingHandoff<'a> {
    pub(crate) const fn new(
        frame: &'a StereoGpuCameraFrame,
        controls: &'a StereoProjectionControls,
        config: &'a RuntimeConfig,
    ) -> Self {
        Self {
            frame,
            controls,
            config,
        }
    }

    pub(crate) fn contract(&self) -> SourceSamplingContract {
        let left_source_uv_rect = source_uv_rect_ltrb_for_diagnostics(&self.frame.left.diagnostics);
        let right_source_uv_rect =
            source_uv_rect_ltrb_for_diagnostics(&self.frame.right.diagnostics);
        let content_uv_rect = if left_source_uv_rect == right_source_uv_rect {
            left_source_uv_rect
        } else {
            full_source_uv_rect_ltrb()
        };
        SourceSamplingContract::new(
            HWB_SOURCE_SAMPLING_BACKEND,
            HWB_SOURCE_SAMPLING_MODE,
            self.controls.source_eye_mapping,
            SourceSamplingTransformStage::PostHomographyPreSourceVisibleRectThenTextureSample,
        )
        .with_content_uv_rect(source_uv_rect_from_ltrb(content_uv_rect))
        .with_source_visible_uv_rect(source_uv_rect_from_ltrb(content_uv_rect))
        .with_transform(
            HWB_SAMPLE_TRANSFORM,
            HWB_SAMPLE_TRANSFORM_OWNER,
            source_uv_rect_transform_applied(self.frame)
                || self.controls.left_texture_transform.shader_flags() != 0
                || self.controls.right_texture_transform.shader_flags() != 0,
        )
        .with_sampler(
            HWB_OUTPUT_UV_LABEL,
            HWB_SAMPLER_UV_ORIGIN,
            SourceSamplerYAxis::RendererDefined,
        )
        .with_texture_transform(
            SourceSamplingTransformStage::PostHomographyPreTextureSample,
            HWB_TEXTURE_TRANSFORM_OWNER,
        )
    }

    pub(crate) fn marker_fields(&self) -> String {
        let contract = self.contract();
        let left_source_uv_rect = source_uv_rect_ltrb_for_diagnostics(&self.frame.left.diagnostics);
        let right_source_uv_rect =
            source_uv_rect_ltrb_for_diagnostics(&self.frame.right.diagnostics);
        let source_crop_rect_state = marker_token(
            self.frame
                .left
                .diagnostics
                .source_crop_rect_state
                .as_deref()
                .or(self
                    .frame
                    .right
                    .diagnostics
                    .source_crop_rect_state
                    .as_deref()),
            "not-logged",
        );
        let source_crop_rect_owner = marker_token(
            self.frame
                .left
                .diagnostics
                .source_crop_rect_owner
                .as_deref()
                .or(self
                    .frame
                    .right
                    .diagnostics
                    .source_crop_rect_owner
                    .as_deref()),
            "not-logged",
        );
        let source_mode = source_mode_for_frame(self.frame);
        let geometry_profile = projection_geometry_profile_for_frame(self.frame, self.config);
        format!(
            "schema=rusty.xr.hwb-source-sampling.v1 phase=source-sampling status=ok sourceMode={} projectionGeometryProfile={} geometry_profile={} sourceEyeMapping={} sourceUvContract={} sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv=screen-to-camera-homography-output sourceSampleTransformStage={} sourceSampleTransform={} sourceSampleTransformOwner={} sourceSampleTransformApplied={} sourceSampleOutputUv={} sourceSamplerUvOrigin={} sourceSamplerYAxis={} sourceTextureTransformStage={} sourceTextureTransformOwner={} contentUvRect={} sourceVisibleUvRect={} sourceCropRectState={} sourceCropRectOwner={} leftSourceVisibleUvRect={} rightSourceVisibleUvRect={} leftSourceCropRectPx={} rightSourceCropRectPx={} leftCameraTextureTransform={} rightCameraTextureTransform={} leftCameraTextureTransformFlags={} rightCameraTextureTransformFlags={} cameraTextureTransformSource={} cameraTextureTransformReason={} {}",
            source_mode,
            geometry_profile,
            geometry_profile,
            contract.source_eye_mapping.stable_id(),
            HWB_SOURCE_UV_CONTRACT,
            legacy_transform_stage_token(contract.transform_stage),
            contract.transform_label,
            contract.transform_owner,
            contract.transform_applied,
            contract.output_uv_label,
            contract.sampler_uv_origin,
            contract.sampler_y_axis.stable_id(),
            legacy_transform_stage_token(contract.texture_transform_stage),
            contract.texture_transform_owner,
            uv_rect_token(source_uv_rect_to_ltrb(contract.content_uv_rect)),
            uv_rect_token(source_uv_rect_to_ltrb(contract.source_visible_uv_rect)),
            source_crop_rect_state,
            source_crop_rect_owner,
            uv_rect_token(left_source_uv_rect),
            uv_rect_token(right_source_uv_rect),
            pixel_rect_token(self.frame.left.diagnostics.source_crop_rect_px),
            pixel_rect_token(self.frame.right.diagnostics.source_crop_rect_px),
            self.controls.left_label(),
            self.controls.right_label(),
            self.controls.left_texture_transform.shader_flags(),
            self.controls.right_texture_transform.shader_flags(),
            self.config.camera_texture_transform.source_label.as_str(),
            self.config.camera_texture_transform.reason.as_str(),
            projection_hardware_buffer_marker_fields(self.frame),
        )
    }
}

pub(crate) fn hwb_stereo_draw_prepared_log_message(
    frame: &StereoGpuCameraFrame,
    controls: &StereoProjectionControls,
    config: &RuntimeConfig,
    projection_active: bool,
    display_left_camera_id: &str,
    display_right_camera_id: &str,
    import_cache_size: usize,
    stereo_descriptor_cache_size: usize,
) -> String {
    let explicit_visual_check = controls.left_texture_transform.is_explicit_visual_check()
        && controls.right_texture_transform.is_explicit_visual_check();
    let accepted_flat_visual_check =
        !projection_active && config.visual_release_accepted && explicit_visual_check;
    let orientation_accepted =
        explicit_visual_check && (projection_active || accepted_flat_visual_check);
    let pose_source = frame
        .left
        .diagnostics
        .pose_source
        .as_deref()
        .unwrap_or("missing");
    let pose_reference = frame
        .left
        .diagnostics
        .lens_pose_reference_label
        .as_deref()
        .unwrap_or("unknown");
    let pose_convention = frame
        .left
        .diagnostics
        .pose_coordinate_convention
        .as_deref()
        .unwrap_or("unknown");
    let source_sample_transform_applied = source_uv_rect_transform_applied(frame)
        || controls.left_texture_transform.shader_flags() != 0
        || controls.right_texture_transform.shader_flags() != 0;
    let active_tier = if projection_active {
        "gpu-projected"
    } else if accepted_flat_visual_check {
        "gpu-flat-visual-check"
    } else {
        "gpu-buffer-probe"
    };
    let projection_shader_path = if projection_active {
        "projected"
    } else if accepted_flat_visual_check {
        "flat-visual-check"
    } else {
        "flat-probe"
    };
    let fallback_reason = if projection_active {
        if config.visual_release_accepted {
            "projected shader path active with manual visual acceptance"
        } else {
            "projected shader path active; visual orientation/alignment acceptance still required"
        }
    } else if accepted_flat_visual_check {
        "missing per-eye projection metadata; drawing accepted flat stereo visual-check path"
    } else {
        "missing per-eye projection metadata or explicit texture orientation"
    };

    format!(
        "Rusty XR GPU stereo camera draw prepared frame {} requestedTier={} activeTier={} alignedProjection={} stereoLayout=Separate pairedLeftRightGpuBuffers=true cpuUploadCount=0 poseSource={} poseReference={} poseConvention={} projectionMode={} cameraFeedMode={} cameraColorMode={} cameraColorShaderBit={} {} cameraColorContrast={} cameraColorBrightness={} cameraColorSaturation={} cameraImportImageLayout={} importCacheLimit={} sourceEyeMapping={} displayLeftCameraId={} displayRightCameraId={} leftCameraTextureTransform={} rightCameraTextureTransform={} leftCameraTextureTransformFlags={} rightCameraTextureTransformFlags={} cameraTextureTransformSource={} cameraTextureTransformReason={} sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv=screen-to-camera-homography-output sourceSampleTransformStage=post_homography_pre_source_visible_rect_then_texture_sample sourceSampleTransform=sourceVisibleUvRect+cameraTextureTransformFlags sourceSampleTransformOwner=android-media-image-crop-rect+vulkan-hwb-camera_projection_shader sourceSampleTransformApplied={} sourceSampleOutputUv=hardware-buffer-sampler-uv sourceSamplerUvOrigin=hardware-buffer-import-convention sourceSamplerYAxis=renderer-defined sourceTextureTransformStage=post_homography_pre_texture_sample sourceTextureTransformOwner=vulkan-hwb-camera_projection_shader orientationCheck={} orientationAccepted={} visualReleaseAccepted={} orientationDiagnosticMode={} orientationDiagnosticStep={} importCacheSize={} stereoDescriptorCacheSize={} projectionShaderPath={} projectionMetadataReady={} fallbackReason={}",
        frame.index,
        config.camera_tier.stable_id(),
        active_tier,
        projection_active,
        pose_source,
        pose_reference,
        pose_convention,
        config.camera_projection_mode.stable_id(),
        config.camera_feed_pipeline_mode.stable_id(),
        config.camera_color_mode.stable_id(),
        config.camera_color_mode.shader_bit(),
        config.hwb_source_color_contract_fields(),
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
        controls.left_texture_transform.shader_flags(),
        controls.right_texture_transform.shader_flags(),
        config.camera_texture_transform.source_label.as_str(),
        config.camera_texture_transform.reason.as_str(),
        source_sample_transform_applied,
        config.camera_texture_transform.is_explicit_visual_check(),
        orientation_accepted,
        config.visual_release_accepted,
        controls.diagnostic_mode.stable_id(),
        controls.diagnostic_step,
        import_cache_size,
        stereo_descriptor_cache_size,
        projection_shader_path,
        stereo_projection_metadata_ready(frame),
        fallback_reason
    )
}

fn source_mode_for_frame(frame: &StereoGpuCameraFrame) -> &'static str {
    let source = frame
        .left
        .diagnostics
        .source
        .as_deref()
        .or(frame.right.diagnostics.source.as_deref())
        .unwrap_or("direct-camera2")
        .to_ascii_lowercase();
    if source.contains("synthetic") {
        "broker-synthetic"
    } else if source.contains("broker") {
        "broker-h264"
    } else {
        "direct-camera2"
    }
}

fn projection_geometry_profile_for_frame(
    frame: &StereoGpuCameraFrame,
    config: &RuntimeConfig,
) -> String {
    let fallback = if config.camera_projection_mode.uses_world_canvas() {
        "full-frame-diagnostic"
    } else {
        "camera-projection"
    };
    marker_token(
        frame
            .left
            .diagnostics
            .projection_geometry_profile
            .as_deref()
            .or(frame
                .left
                .diagnostics
                .synthetic_projection_profile
                .as_deref())
            .or(frame
                .right
                .diagnostics
                .projection_geometry_profile
                .as_deref())
            .or(frame
                .right
                .diagnostics
                .synthetic_projection_profile
                .as_deref()),
        fallback,
    )
}

fn marker_token(value: Option<&str>, fallback: &str) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .replace(char::is_whitespace, "_")
}

fn full_source_uv_rect_ltrb() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

fn source_uv_rect_from_ltrb(rect: [f32; 4]) -> SourceUvRect {
    SourceUvRect::new(
        Vec2::new(rect[0], rect[1]),
        Vec2::new(rect[2] - rect[0], rect[3] - rect[1]),
    )
}

fn source_uv_rect_to_ltrb(rect: SourceUvRect) -> [f32; 4] {
    [
        rect.origin_uv.x,
        rect.origin_uv.y,
        rect.origin_uv.x + rect.size_uv.x,
        rect.origin_uv.y + rect.size_uv.y,
    ]
}

fn source_uv_rect_ltrb_for_diagnostics(diagnostics: &HeadsetCameraFrameDiagnostics) -> [f32; 4] {
    diagnostics
        .source_visible_uv_rect
        .or(diagnostics.content_uv_rect)
        .unwrap_or_else(full_source_uv_rect_ltrb)
}

fn source_uv_rect_is_full(rect: [f32; 4]) -> bool {
    const EPSILON: f32 = 0.0005;
    (rect[0]).abs() <= EPSILON
        && (rect[1]).abs() <= EPSILON
        && (rect[2] - 1.0).abs() <= EPSILON
        && (rect[3] - 1.0).abs() <= EPSILON
}

fn source_uv_rect_transform_applied(frame: &StereoGpuCameraFrame) -> bool {
    !source_uv_rect_is_full(source_uv_rect_ltrb_for_diagnostics(&frame.left.diagnostics))
        || !source_uv_rect_is_full(source_uv_rect_ltrb_for_diagnostics(
            &frame.right.diagnostics,
        ))
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

fn uv_rect_token(rect: [f32; 4]) -> String {
    format!(
        "{:.6},{:.6},{:.6},{:.6}",
        rect[0], rect[1], rect[2], rect[3]
    )
}

fn pixel_rect_token(rect: Option<[u32; 4]>) -> String {
    rect.map(|[left, top, right, bottom]| format!("{left},{top},{right},{bottom}"))
        .unwrap_or_else(|| "not-logged".to_string())
}

fn optional_u64_token(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not-logged".to_string())
}

fn optional_u32_token(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not-logged".to_string())
}

fn legacy_transform_stage_token(stage: SourceSamplingTransformStage) -> &'static str {
    match stage {
        SourceSamplingTransformStage::None => "none",
        SourceSamplingTransformStage::PostHomographyPreTextureSample => {
            "post_homography_pre_texture_sample"
        }
        SourceSamplingTransformStage::PostHomographyPreOesSample => {
            "post_homography_pre_oes_sample"
        }
        SourceSamplingTransformStage::PostHomographyPreYuvSample => {
            "post_homography_pre_yuv_sample"
        }
        SourceSamplingTransformStage::PostHomographyPreSourceVisibleRectThenTextureSample => {
            "post_homography_pre_source_visible_rect_then_texture_sample"
        }
        SourceSamplingTransformStage::Other => "other",
    }
}

fn projection_hardware_buffer_marker_fields(frame: &StereoGpuCameraFrame) -> String {
    format!(
        "leftHardwareBufferWidth={} leftHardwareBufferHeight={} leftHardwareBufferNativeFormat={} leftHardwareBufferUsage={} leftHardwareBufferLayers={} leftHardwareBufferStridePx={} leftHardwareBufferId={} rightHardwareBufferWidth={} rightHardwareBufferHeight={} rightHardwareBufferNativeFormat={} rightHardwareBufferUsage={} rightHardwareBufferLayers={} rightHardwareBufferStridePx={} rightHardwareBufferId={}",
        frame.left.width,
        frame.left.height,
        optional_u64_token(frame.left.descriptor.native_format),
        optional_u64_token(frame.left.descriptor.usage_flags),
        optional_u32_token(frame.left.descriptor.layer_count),
        optional_u32_token(frame.left.descriptor.stride_px),
        optional_u64_token(frame.left.descriptor.buffer_id),
        frame.right.width,
        frame.right.height,
        optional_u64_token(frame.right.descriptor.native_format),
        optional_u64_token(frame.right.descriptor.usage_flags),
        optional_u32_token(frame.right.descriptor.layer_count),
        optional_u32_token(frame.right.descriptor.stride_px),
        optional_u64_token(frame.right.descriptor.buffer_id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CameraOrientationDiagnosticMode, CameraProjectionMode, HeadsetCameraGpuFrame,
        RuntimeConfig, StereoGpuCameraFrame, StereoProjectionControls,
    };
    use rusty_xr_contracts::{
        CameraCompositeTier, CameraFrameMetadata, CameraGpuBufferDescriptor, CameraSourceId,
        CameraTextureTransform, ImageSize, StereoMediaLayout, StereoSourceEyeMapping,
    };

    fn diagnostics(source_visible_uv_rect: Option<[f32; 4]>) -> HeadsetCameraFrameDiagnostics {
        HeadsetCameraFrameDiagnostics {
            source: Some("direct-cam".to_string()),
            camera_id: Some("50".to_string()),
            lens_facing: Some("external".to_string()),
            lens_facing_rank: Some(0),
            selection_score: Some(1),
            requested_tier: CameraCompositeTier::GpuProjected,
            active_tier_label: "gpu-projected".to_string(),
            transport: "hardware-buffer".to_string(),
            pose_source: None,
            pose_coordinate_convention: None,
            lens_pose_reference_label: None,
            diagnostic_source: None,
            synthetic_projection_profile: None,
            projection_geometry_profile: None,
            synthetic_pattern: None,
            orientation_kind: None,
            raster_orientation: None,
            upright_marker: None,
            orientation_metadata_source: None,
            orientation_default: None,
            stimulus_raster_orientation: None,
            stimulus_upright_marker: None,
            stimulus_orientation_default: None,
            content_kind: Some("camera".to_string()),
            content_width: Some(1280),
            content_height: Some(1280),
            content_aspect_ratio: Some(1.0),
            desired_display_aspect_ratio: Some(1.0),
            desired_projection_aspect_ratio: Some(1.0),
            content_coordinate_space: Some("content-normalized-top-left-y-down".to_string()),
            content_origin: Some("top-left".to_string()),
            content_x_axis: Some("right".to_string()),
            content_y_axis: Some("down".to_string()),
            content_uv_rect: None,
            source_visible_uv_rect,
            source_crop_rect_px: Some([16, 32, 1200, 1240]),
            source_crop_rect_state: Some("metadata ready".to_string()),
            source_crop_rect_owner: Some("android media image".to_string()),
            content_mapping_intent: None,
            content_geometry_metadata_source: None,
            content_geometry_default: None,
            requested_stereo_layout: None,
            stereo_layout: StereoMediaLayout::Separate,
            mono_fallback: false,
            fallback_reason: String::new(),
        }
    }

    fn gpu_frame(index: u64, source_visible_uv_rect: Option<[f32; 4]>) -> HeadsetCameraGpuFrame {
        let size = ImageSize::new(1280, 1280);
        HeadsetCameraGpuFrame {
            width: size.width,
            height: size.height,
            timestamp_ns: 1000 + index as i64,
            index,
            metadata: CameraFrameMetadata::without_intrinsics(
                CameraSourceId::new("test-camera"),
                index,
                size,
            ),
            diagnostics: diagnostics(source_visible_uv_rect),
            descriptor: CameraGpuBufferDescriptor::new("test-camera", size, "hardware-buffer")
                .with_native_format(35)
                .with_usage_flags(0x100)
                .with_layer_count(1)
                .with_stride_px(1280)
                .with_buffer_id(9000 + index),
        }
    }

    fn stereo_frame(source_visible_uv_rect: Option<[f32; 4]>) -> StereoGpuCameraFrame {
        StereoGpuCameraFrame {
            index: 1,
            left: gpu_frame(1, source_visible_uv_rect),
            right: gpu_frame(2, source_visible_uv_rect),
            pair_delta_ns: 10,
            midpoint_timestamp_ns: 1005,
        }
    }

    fn controls() -> StereoProjectionControls {
        StereoProjectionControls {
            source_eye_mapping: StereoSourceEyeMapping::DisplayLeftFromLeftSource,
            left_texture_transform: CameraTextureTransform::default(),
            right_texture_transform: CameraTextureTransform::default(),
            diagnostic_mode: CameraOrientationDiagnosticMode::Off,
            diagnostic_step: 0,
        }
    }

    #[test]
    fn hwb_handoff_reports_identity_full_rect_contract() {
        let frame = stereo_frame(None);
        let controls = controls();
        let config = RuntimeConfig::default();
        let handoff = HwbSourceSamplingHandoff::new(&frame, &controls, &config);
        let contract = handoff.contract();
        let fields = handoff.marker_fields();
        assert!(contract.is_valid());
        assert_eq!(contract.backend, "hwb");
        assert_eq!(
            contract.transform_stage,
            SourceSamplingTransformStage::PostHomographyPreSourceVisibleRectThenTextureSample
        );
        assert_eq!(
            contract.texture_transform_stage,
            SourceSamplingTransformStage::PostHomographyPreTextureSample
        );
        assert_eq!(contract.sampler_y_axis, SourceSamplerYAxis::RendererDefined);
        assert!(fields.contains("schema=rusty.xr.hwb-source-sampling.v1"));
        assert!(fields.contains("sourceMode=direct-camera2"));
        assert!(fields.contains("projectionGeometryProfile=camera-projection"));
        assert!(fields
            .contains("sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler"));
        assert!(fields.contains("sourceEyeMapping=display-left-from-left-source"));
        assert!(fields.contains("sourceSampleTransformApplied=false"));
        assert!(fields.contains("sourceTextureTransformStage=post_homography_pre_texture_sample"));
        assert!(fields.contains("contentUvRect=0.000000,0.000000,1.000000,1.000000"));
        assert!(fields.contains("leftHardwareBufferWidth=1280"));
    }

    #[test]
    fn hwb_handoff_reports_world_canvas_geometry_profile() {
        let frame = stereo_frame(None);
        let controls = controls();
        let mut config = RuntimeConfig::default();
        config.camera_projection_mode = CameraProjectionMode::WorldCanvas;
        let fields = HwbSourceSamplingHandoff::new(&frame, &controls, &config).marker_fields();
        assert!(fields.contains("projectionGeometryProfile=full-frame-diagnostic"));
        assert!(fields.contains("geometry_profile=full-frame-diagnostic"));
    }

    #[test]
    fn hwb_handoff_reports_source_visible_rect_transform() {
        let frame = stereo_frame(Some([0.1, 0.2, 0.9, 0.8]));
        let controls = controls();
        let config = RuntimeConfig::default();
        let handoff = HwbSourceSamplingHandoff::new(&frame, &controls, &config);
        let contract = handoff.contract();
        let fields = handoff.marker_fields();
        let visible_rect = source_uv_rect_to_ltrb(contract.source_visible_uv_rect);
        assert!((visible_rect[0] - 0.1).abs() < 0.0001);
        assert!((visible_rect[1] - 0.2).abs() < 0.0001);
        assert!((visible_rect[2] - 0.9).abs() < 0.0001);
        assert!((visible_rect[3] - 0.8).abs() < 0.0001);
        assert!(contract.transform_applied);
        assert!(fields.contains("sourceSampleTransformApplied=true"));
        assert!(fields.contains("sourceVisibleUvRect=0.100000,0.200000,0.900000,0.800000"));
        assert!(fields.contains("sourceCropRectState=metadata_ready"));
        assert!(fields.contains("sourceCropRectOwner=android_media_image"));
    }

    #[test]
    fn hwb_draw_prepared_marker_uses_source_sampling_record() {
        let frame = stereo_frame(Some([0.1, 0.2, 0.9, 0.8]));
        let controls = controls();
        let config = RuntimeConfig::default();

        let message = hwb_stereo_draw_prepared_log_message(
            &frame, &controls, &config, true, "50", "51", 3, 2,
        );

        assert!(message.starts_with("Rusty XR GPU stereo camera draw prepared frame 1"));
        assert!(message.contains("activeTier=gpu-projected alignedProjection=true"));
        assert!(message
            .contains("sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler"));
        assert!(message.contains("sourceSampleTransformApplied=true"));
        assert!(message.contains("displayLeftCameraId=50 displayRightCameraId=51"));
        assert!(message.contains("importCacheSize=3 stereoDescriptorCacheSize=2"));
        assert!(message.contains("projectionShaderPath=projected"));
        assert!(message.contains(
            "fallbackReason=projected shader path active; visual orientation/alignment acceptance still required"
        ));
    }
}
