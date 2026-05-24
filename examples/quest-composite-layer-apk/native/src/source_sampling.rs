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
        format!(
            "schema=rusty.xr.hwb-source-sampling.v1 phase=source-sampling status=ok sourceEyeMapping={} sourceUvContract={} sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv=screen-to-camera-homography-output sourceSampleTransformStage={} sourceSampleTransform={} sourceSampleTransformOwner={} sourceSampleTransformApplied={} sourceSampleOutputUv={} sourceSamplerUvOrigin={} sourceSamplerYAxis={} sourceTextureTransformStage={} sourceTextureTransformOwner={} contentUvRect={} sourceVisibleUvRect={} sourceCropRectState={} sourceCropRectOwner={} leftSourceVisibleUvRect={} rightSourceVisibleUvRect={} leftSourceCropRectPx={} rightSourceCropRectPx={} leftCameraTextureTransform={} rightCameraTextureTransform={} leftCameraTextureTransformFlags={} rightCameraTextureTransformFlags={} cameraTextureTransformSource={} cameraTextureTransformReason={} {}",
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
        CameraOrientationDiagnosticMode, HeadsetCameraGpuFrame, RuntimeConfig,
        StereoGpuCameraFrame, StereoProjectionControls,
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
        assert!(fields
            .contains("sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler"));
        assert!(fields.contains("sourceEyeMapping=display-left-from-left-source"));
        assert!(fields.contains("sourceSampleTransformApplied=false"));
        assert!(fields.contains("sourceTextureTransformStage=post_homography_pre_texture_sample"));
        assert!(fields.contains("contentUvRect=0.000000,0.000000,1.000000,1.000000"));
        assert!(fields.contains("leftHardwareBufferWidth=1280"));
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
}
