use crate::camera_texture_path::MakepadCameraTexturePath;
use crate::FrameOrientationDecision;
use rusty_xr_camera_model::SourceSamplingMode;
use rusty_xr_contracts::{
    SourceSamplerYAxis, SourceSamplingContract, SourceSamplingTransformStage,
    StereoSourceEyeMapping,
};

pub(crate) const MAKEPAD_SOURCE_UV_CONTRACT: &str =
    "screen_to_camera_content_uv_to_makepad_video_sampler";
const MAKEPAD_SOURCE_SAMPLING_BACKEND: &str = "makepad";
const MAKEPAD_SOURCE_SAMPLING_MODE: &str = "makepad-runtime";
const MAKEPAD_SAMPLE_TRANSFORM_OWNER: &str = "makepad-shader-source_sample_uv";
const MAKEPAD_OUTPUT_UV_LABEL: &str = "makepad-video-sampler-uv";
const MAKEPAD_SAMPLER_UV_ORIGIN: &str = "makepad-video-sampler";

pub(crate) struct MakepadCadenceSampleMarker {
    pub(crate) elapsed_seconds: f64,
    pub(crate) interval_seconds: f64,
    pub(crate) app_frame_count: u64,
    pub(crate) app_frame_delta: u64,
    pub(crate) app_frame_rate_hz: f64,
    pub(crate) xr_update_count: u64,
    pub(crate) xr_update_delta: u64,
    pub(crate) xr_update_rate_hz: f64,
    pub(crate) draw_event_count: u64,
    pub(crate) draw_event_delta: u64,
    pub(crate) draw_event_rate_hz: f64,
    pub(crate) left_texture_update_count: u64,
    pub(crate) right_texture_update_count: u64,
    pub(crate) paired_texture_update_count: u64,
    pub(crate) left_texture_update_delta: u64,
    pub(crate) right_texture_update_delta: u64,
    pub(crate) paired_texture_update_delta: u64,
    pub(crate) left_texture_update_rate_hz: f64,
    pub(crate) right_texture_update_rate_hz: f64,
    pub(crate) paired_texture_update_rate_hz: f64,
    pub(crate) left_last_position_ms: u128,
    pub(crate) right_last_position_ms: u128,
    pub(crate) paired_left_right_camera_frames: bool,
    pub(crate) projection_mapping_ready: bool,
    pub(crate) aligned_projection: bool,
    pub(crate) visible_camera_projection_ready: bool,
    pub(crate) xr_display_refresh_rate_hz: Option<f64>,
    pub(crate) xr_effective_frame_rate_hz: Option<f64>,
    pub(crate) xr_frame_cpu_ms: Option<f64>,
    pub(crate) xr_should_render: Option<bool>,
    pub(crate) xr_skipped_should_render_count: Option<u64>,
    pub(crate) xr_pre_frame_events_ms: Option<f64>,
    pub(crate) xr_post_frame_media_events_ms: Option<f64>,
    pub(crate) xr_wait_frame_ms: Option<f64>,
    pub(crate) xr_begin_frame_ms: Option<f64>,
    pub(crate) xr_locate_space_ms: Option<f64>,
    pub(crate) xr_locate_views_ms: Option<f64>,
    pub(crate) xr_acquire_swapchain_ms: Option<f64>,
    pub(crate) xr_wait_swapchain_ms: Option<f64>,
    pub(crate) xr_acquire_depth_ms: Option<f64>,
    pub(crate) xr_update_prepare_ms: Option<f64>,
    pub(crate) xr_update_dispatch_ms: Option<f64>,
    pub(crate) xr_next_frame_ms: Option<f64>,
    pub(crate) xr_draw_event_ms: Option<f64>,
    pub(crate) xr_compile_shaders_ms: Option<f64>,
    pub(crate) xr_repaint_ms: Option<f64>,
    pub(crate) xr_repaint_gpu_ms: Option<f64>,
    pub(crate) xr_repaint_wait_inflight_ms: Option<f64>,
    pub(crate) xr_repaint_prepare_textures_ms: Option<f64>,
    pub(crate) xr_repaint_record_draw_ms: Option<f64>,
    pub(crate) xr_repaint_submit_ms: Option<f64>,
    pub(crate) xr_repaint_texture_upload_count: Option<u32>,
    pub(crate) xr_repaint_texture_upload_bytes: Option<u64>,
    pub(crate) xr_repaint_packet_buffer_count: Option<u32>,
    pub(crate) xr_repaint_packet_buffer_bytes: Option<u64>,
    pub(crate) xr_repaint_geometry_upload_bytes: Option<u64>,
    pub(crate) xr_repaint_descriptor_set_count: Option<u32>,
    pub(crate) xr_repaint_draw_items: Option<u64>,
    pub(crate) xr_repaint_draw_calls: Option<u64>,
    pub(crate) xr_repaint_packets: Option<u64>,
    pub(crate) xr_repaint_instances: Option<u64>,
    pub(crate) xr_repaint_indices: Option<u64>,
    pub(crate) xr_depth_readback_ms: Option<f64>,
    pub(crate) xr_end_frame_ms: Option<f64>,
    pub(crate) xr_resize_projection_ms: Option<f64>,
    pub(crate) texture_path: MakepadCameraTexturePath,
}

pub(crate) struct MakepadSourceSamplingHandoff<'a> {
    broker_h264_enabled: bool,
    explicit_top_left_broker_stimulus: bool,
    orientation_decision: &'a FrameOrientationDecision,
    source_sampling_mode: SourceSamplingMode,
    projection_content_mapping_mode: f32,
    full_frame_diagnostic: bool,
    source_eye_mapping: &'a str,
    source_sample_transform: &'a str,
    content_geometry_fields: &'a str,
    source_color_contract_fields: &'a str,
    texture_path: MakepadCameraTexturePath,
}

impl<'a> MakepadSourceSamplingHandoff<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        broker_h264_enabled: bool,
        explicit_top_left_broker_stimulus: bool,
        orientation_decision: &'a FrameOrientationDecision,
        source_sampling_mode: SourceSamplingMode,
        projection_content_mapping_mode: f32,
        full_frame_diagnostic: bool,
        source_eye_mapping: &'a str,
        source_sample_transform: &'a str,
        content_geometry_fields: &'a str,
        source_color_contract_fields: &'a str,
        texture_path: MakepadCameraTexturePath,
    ) -> Self {
        Self {
            broker_h264_enabled,
            explicit_top_left_broker_stimulus,
            orientation_decision,
            source_sampling_mode,
            projection_content_mapping_mode,
            full_frame_diagnostic,
            source_eye_mapping,
            source_sample_transform,
            content_geometry_fields,
            source_color_contract_fields,
            texture_path,
        }
    }

    pub(crate) fn contract(&self) -> SourceSamplingContract {
        let source_eye_mapping =
            StereoSourceEyeMapping::parse(self.source_eye_mapping).unwrap_or_default();
        SourceSamplingContract::new(
            MAKEPAD_SOURCE_SAMPLING_BACKEND,
            MAKEPAD_SOURCE_SAMPLING_MODE,
            source_eye_mapping,
            SourceSamplingTransformStage::PostHomographyPreYuvSample,
        )
        .with_transform(
            self.source_sample_transform,
            MAKEPAD_SAMPLE_TRANSFORM_OWNER,
            self.orientation_decision.source_sample_y_flip >= 0.5,
        )
        .with_sampler(
            MAKEPAD_OUTPUT_UV_LABEL,
            MAKEPAD_SAMPLER_UV_ORIGIN,
            SourceSamplerYAxis::MakepadSamplerOriginConvention,
        )
        .with_texture_transform(
            SourceSamplingTransformStage::PostHomographyPreYuvSample,
            MAKEPAD_SAMPLE_TRANSFORM_OWNER,
        )
    }

    pub(crate) fn marker_fields(&self) -> String {
        let contract = self.contract();
        format!(
            "phase=source-sampling status=ok brokerH264Enabled={} explicitTopLeftBrokerStimulus={} orientationKind={} rasterOrientation={} uprightMarker={} orientationMetadataSource={} orientationDefault={} orientationFallbackReason={} sourceSampleYFlip={:.1} sourceSampleYFlipReason={} projectionContentMappingMode={} sourceSamplingMode={} sourceEyeMapping={} sourceUvContract={} sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv={} sourceSampleTransformStage={} sourceSampleTransform={} sourceSampleTransformOwner={} sourceSampleTransformApplied={} sourceSampleOutputUv={} sourceSamplerUvOrigin={} sourceSamplerYAxis={} sourceTextureTransformStage={} sourceTextureTransformOwner={} diagnosticUvTransform={} sourceRasterYMappingStage={} rendererSurfaceUvOrigin=makepad-renderer-surface-uv displayScreenUvOrigin=top-left-origin-y-down displayScreenUvNormalization=renderer-v-flip-to-display-screen-uv {} {} {}",
            self.broker_h264_enabled,
            self.explicit_top_left_broker_stimulus,
            marker_token(&self.orientation_decision.orientation_kind),
            marker_token(&self.orientation_decision.raster_orientation),
            marker_token(&self.orientation_decision.upright_marker),
            marker_token(&self.orientation_decision.metadata_source),
            self.orientation_decision.orientation_default,
            marker_token(&self.orientation_decision.fallback_reason),
            self.orientation_decision.source_sample_y_flip,
            marker_token(&self.orientation_decision.source_sample_y_flip_reason),
            self.projection_content_mapping_label(),
            self.source_sampling_mode.stable_id(),
            contract.source_eye_mapping.stable_id(),
            self.source_uv_contract_label(),
            self.source_sample_input_uv_label(),
            legacy_transform_stage_token(contract.transform_stage),
            contract.transform_label,
            contract.transform_owner,
            contract.transform_applied,
            contract.output_uv_label,
            contract.sampler_uv_origin,
            contract.sampler_y_axis.stable_id(),
            legacy_transform_stage_token(contract.texture_transform_stage),
            contract.texture_transform_owner,
            contract.transform_label,
            contract.transform_label,
            self.content_geometry_fields,
            self.source_color_contract_fields,
            self.texture_path.marker_fields(),
        )
    }

    fn projection_content_mapping_label(&self) -> &'static str {
        if self.source_sampling_mode.uses_target_local_raster()
            || self.projection_content_mapping_mode >= 0.5
        {
            "target-local-raster"
        } else if self.full_frame_diagnostic {
            "full-frame-stimulus-to-surface-homography"
        } else {
            "screen-to-camera-homography"
        }
    }

    fn source_uv_contract_label(&self) -> &'static str {
        if self.source_sampling_mode.uses_target_local_raster() {
            "target_local_raster_uv_to_makepad_video_sampler"
        } else {
            MAKEPAD_SOURCE_UV_CONTRACT
        }
    }

    fn source_sample_input_uv_label(&self) -> &'static str {
        if self.source_sampling_mode.uses_target_local_raster() {
            "target-local-raster-uv"
        } else {
            "screen-to-camera-homography-output"
        }
    }
}

pub(crate) fn makepad_cadence_start_marker_line(sample_period_seconds: f64) -> String {
    format!(
        "RUSTY_XR_MAKEPAD_CADENCE schema=rusty.xr.makepad-cadence.v1 phase=start status=started samplePeriodSeconds={:.1} appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated",
        sample_period_seconds,
    )
}

pub(crate) fn makepad_cadence_sample_marker_line(sample: MakepadCadenceSampleMarker) -> String {
    format!(
        "RUSTY_XR_MAKEPAD_CADENCE schema=rusty.xr.makepad-cadence.v1 phase=sample status=ok elapsedMs={:.0} intervalMs={:.0} appFrameCount={} appFrameDelta={} appFrameRateHz={:.2} xrUpdateCount={} xrUpdateDelta={} xrUpdateRateHz={:.2} drawEventCount={} drawEventDelta={} drawEventRateHz={:.2} leftTextureUpdateCount={} rightTextureUpdateCount={} pairedTextureUpdateCount={} leftTextureUpdateDelta={} rightTextureUpdateDelta={} pairedTextureUpdateDelta={} leftTextureUpdateRateHz={:.2} rightTextureUpdateRateHz={:.2} pairedTextureUpdateRateHz={:.2} leftLastPositionMs={} rightLastPositionMs={} pairedLeftRightCameraFrames={} projectionMappingReady={} alignedProjection={} visibleCameraProjectionReady={} renderPath=makepad-xr appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated xrDisplayRefreshRateHz={} xrEffectiveFrameRateHz={} xrFrameCpuMs={} xrShouldRender={} xrSkippedShouldRenderCount={} xrPreFrameEventsMs={} xrPostFrameMediaEventsMs={} xrWaitFrameMs={} xrBeginFrameMs={} xrLocateSpaceMs={} xrLocateViewsMs={} xrAcquireSwapchainMs={} xrWaitSwapchainMs={} xrAcquireDepthMs={} xrUpdatePrepareMs={} xrUpdateDispatchMs={} xrNextFrameMs={} xrDrawEventMs={} xrCompileShadersMs={} xrRepaintMs={} xrRepaintGpuMs={} xrRepaintWaitInflightMs={} xrRepaintPrepareTexturesMs={} xrRepaintRecordDrawMs={} xrRepaintSubmitMs={} xrRepaintTextureUploadCount={} xrRepaintTextureUploadBytes={} xrRepaintPacketBufferCount={} xrRepaintPacketBufferBytes={} xrRepaintGeometryUploadBytes={} xrRepaintDescriptorSetCount={} xrRepaintDrawItems={} xrRepaintDrawCalls={} xrRepaintPackets={} xrRepaintInstances={} xrRepaintIndices={} xrDepthReadbackMs={} xrEndFrameMs={} xrResizeProjectionMs={} {}",
        sample.elapsed_seconds * 1000.0,
        sample.interval_seconds * 1000.0,
        sample.app_frame_count,
        sample.app_frame_delta,
        sample.app_frame_rate_hz,
        sample.xr_update_count,
        sample.xr_update_delta,
        sample.xr_update_rate_hz,
        sample.draw_event_count,
        sample.draw_event_delta,
        sample.draw_event_rate_hz,
        sample.left_texture_update_count,
        sample.right_texture_update_count,
        sample.paired_texture_update_count,
        sample.left_texture_update_delta,
        sample.right_texture_update_delta,
        sample.paired_texture_update_delta,
        sample.left_texture_update_rate_hz,
        sample.right_texture_update_rate_hz,
        sample.paired_texture_update_rate_hz,
        sample.left_last_position_ms,
        sample.right_last_position_ms,
        sample.paired_left_right_camera_frames,
        sample.projection_mapping_ready,
        sample.aligned_projection,
        sample.visible_camera_projection_ready,
        optional_f64_marker(sample.xr_display_refresh_rate_hz),
        optional_f64_marker(sample.xr_effective_frame_rate_hz),
        optional_f64_marker(sample.xr_frame_cpu_ms),
        optional_bool_marker(sample.xr_should_render),
        optional_u64_marker(sample.xr_skipped_should_render_count),
        optional_f64_marker(sample.xr_pre_frame_events_ms),
        optional_f64_marker(sample.xr_post_frame_media_events_ms),
        optional_f64_marker(sample.xr_wait_frame_ms),
        optional_f64_marker(sample.xr_begin_frame_ms),
        optional_f64_marker(sample.xr_locate_space_ms),
        optional_f64_marker(sample.xr_locate_views_ms),
        optional_f64_marker(sample.xr_acquire_swapchain_ms),
        optional_f64_marker(sample.xr_wait_swapchain_ms),
        optional_f64_marker(sample.xr_acquire_depth_ms),
        optional_f64_marker(sample.xr_update_prepare_ms),
        optional_f64_marker(sample.xr_update_dispatch_ms),
        optional_f64_marker(sample.xr_next_frame_ms),
        optional_f64_marker(sample.xr_draw_event_ms),
        optional_f64_marker(sample.xr_compile_shaders_ms),
        optional_f64_marker(sample.xr_repaint_ms),
        optional_f64_marker(sample.xr_repaint_gpu_ms),
        optional_f64_marker(sample.xr_repaint_wait_inflight_ms),
        optional_f64_marker(sample.xr_repaint_prepare_textures_ms),
        optional_f64_marker(sample.xr_repaint_record_draw_ms),
        optional_f64_marker(sample.xr_repaint_submit_ms),
        optional_u32_marker(sample.xr_repaint_texture_upload_count),
        optional_u64_marker(sample.xr_repaint_texture_upload_bytes),
        optional_u32_marker(sample.xr_repaint_packet_buffer_count),
        optional_u64_marker(sample.xr_repaint_packet_buffer_bytes),
        optional_u64_marker(sample.xr_repaint_geometry_upload_bytes),
        optional_u32_marker(sample.xr_repaint_descriptor_set_count),
        optional_u64_marker(sample.xr_repaint_draw_items),
        optional_u64_marker(sample.xr_repaint_draw_calls),
        optional_u64_marker(sample.xr_repaint_packets),
        optional_u64_marker(sample.xr_repaint_instances),
        optional_u64_marker(sample.xr_repaint_indices),
        optional_f64_marker(sample.xr_depth_readback_ms),
        optional_f64_marker(sample.xr_end_frame_ms),
        optional_f64_marker(sample.xr_resize_projection_ms),
        sample.texture_path.marker_fields(),
    )
}

fn optional_f64_marker(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{:.2}", value))
        .unwrap_or_else(|| "unavailable".to_string())
}

fn optional_u32_marker(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

fn optional_u64_marker(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

fn optional_bool_marker(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_string())
}

pub(crate) fn makepad_texture_content_probe_missing_marker_fields(
    side: &str,
    yuv_enabled: bool,
    yuv_biplanar: bool,
    yuv_matrix: f32,
    rotation_steps: f32,
) -> String {
    format!(
        "phase=texture-content-probe status=missing side={} {} yuvEnabled={} yuvBiplanar={} yuvMatrix={:.1} rotationSteps={:.0} cpuPlaneContentPresent=false visualInspection=required visualReleaseAccepted=false",
        side,
        texture_content_probe_contract_fields(),
        yuv_enabled,
        yuv_biplanar,
        yuv_matrix,
        rotation_steps,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn makepad_texture_content_probe_ok_marker_fields(
    side: &str,
    yuv_enabled: bool,
    yuv_biplanar: bool,
    yuv_matrix: f32,
    rotation_steps: f32,
    cpu_content_present: bool,
    y_stats_fields: &str,
    u_stats_fields: &str,
    v_stats_fields: &str,
) -> String {
    format!(
        "phase=texture-content-probe status=ok side={} {} yuvEnabled={} yuvBiplanar={} yuvMatrix={:.1} rotationSteps={:.0} cpuPlaneContentPresent={} {} {} {} gpuSamplingStillVisual=full-frame-source-display-row-vertical-uv-yuv visualInspection=required visualReleaseAccepted=false",
        side,
        texture_content_probe_contract_fields(),
        yuv_enabled,
        yuv_biplanar,
        yuv_matrix,
        rotation_steps,
        cpu_content_present,
        y_stats_fields,
        u_stats_fields,
        v_stats_fields,
    )
}

fn texture_content_probe_contract_fields() -> &'static str {
    "textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040"
}

fn marker_token(value: &str) -> String {
    value.replace(char::is_whitespace, "_")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn direct_decision() -> FrameOrientationDecision {
        FrameOrientationDecision::direct_camera2()
    }

    fn flipped_decision() -> FrameOrientationDecision {
        FrameOrientationDecision {
            source_sample_y_flip: 1.0,
            source_sample_y_flip_reason: "bottom-left raster needs flip".to_string(),
            orientation_kind: "broker stimulus".to_string(),
            raster_orientation: "bottom-left-origin-y-up".to_string(),
            upright_marker: "upright".to_string(),
            metadata_source: "broker metadata".to_string(),
            orientation_default: false,
            fallback_reason: "none".to_string(),
        }
    }

    #[test]
    fn cadence_start_marker_keeps_source_sampling_shape() {
        assert_eq!(
            makepad_cadence_start_marker_line(2.0),
            "RUSTY_XR_MAKEPAD_CADENCE schema=rusty.xr.makepad-cadence.v1 phase=start status=started samplePeriodSeconds=2.0 appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated"
        );
    }

    #[test]
    fn cadence_sample_marker_keeps_source_sampling_shape() {
        let marker = makepad_cadence_sample_marker_line(MakepadCadenceSampleMarker {
            elapsed_seconds: 4.25,
            interval_seconds: 2.0,
            app_frame_count: 120,
            app_frame_delta: 60,
            app_frame_rate_hz: 30.0,
            xr_update_count: 118,
            xr_update_delta: 59,
            xr_update_rate_hz: 29.5,
            draw_event_count: 90,
            draw_event_delta: 45,
            draw_event_rate_hz: 22.5,
            left_texture_update_count: 32,
            right_texture_update_count: 31,
            paired_texture_update_count: 31,
            left_texture_update_delta: 16,
            right_texture_update_delta: 15,
            paired_texture_update_delta: 15,
            left_texture_update_rate_hz: 8.0,
            right_texture_update_rate_hz: 7.5,
            paired_texture_update_rate_hz: 7.5,
            left_last_position_ms: 1001,
            right_last_position_ms: 1003,
            paired_left_right_camera_frames: true,
            projection_mapping_ready: true,
            aligned_projection: false,
            visible_camera_projection_ready: true,
            xr_display_refresh_rate_hz: Some(90.0),
            xr_effective_frame_rate_hz: Some(30.0),
            xr_frame_cpu_ms: Some(12.34),
            xr_should_render: Some(true),
            xr_skipped_should_render_count: Some(3),
            xr_pre_frame_events_ms: Some(0.25),
            xr_post_frame_media_events_ms: Some(1.25),
            xr_wait_frame_ms: Some(8.0),
            xr_begin_frame_ms: Some(0.1),
            xr_locate_space_ms: Some(0.11),
            xr_locate_views_ms: Some(0.12),
            xr_acquire_swapchain_ms: Some(0.2),
            xr_wait_swapchain_ms: Some(0.3),
            xr_acquire_depth_ms: Some(0.35),
            xr_update_prepare_ms: Some(0.36),
            xr_update_dispatch_ms: Some(0.4),
            xr_next_frame_ms: Some(0.5),
            xr_draw_event_ms: Some(0.6),
            xr_compile_shaders_ms: Some(0.65),
            xr_repaint_ms: Some(1.7),
            xr_repaint_gpu_ms: Some(2.1),
            xr_repaint_wait_inflight_ms: Some(0.8),
            xr_repaint_prepare_textures_ms: Some(0.9),
            xr_repaint_record_draw_ms: Some(1.0),
            xr_repaint_submit_ms: Some(1.1),
            xr_repaint_texture_upload_count: Some(2),
            xr_repaint_texture_upload_bytes: Some(3456),
            xr_repaint_packet_buffer_count: Some(4),
            xr_repaint_packet_buffer_bytes: Some(5678),
            xr_repaint_geometry_upload_bytes: Some(6789),
            xr_repaint_descriptor_set_count: Some(8),
            xr_repaint_draw_items: Some(9),
            xr_repaint_draw_calls: Some(10),
            xr_repaint_packets: Some(11),
            xr_repaint_instances: Some(12),
            xr_repaint_indices: Some(13),
            xr_depth_readback_ms: Some(1.2),
            xr_end_frame_ms: Some(0.7),
            xr_resize_projection_ms: Some(1.3),
            texture_path: MakepadCameraTexturePath::DirectCpuYuvPlane,
        });

        assert_eq!(
            marker,
            "RUSTY_XR_MAKEPAD_CADENCE schema=rusty.xr.makepad-cadence.v1 phase=sample status=ok elapsedMs=4250 intervalMs=2000 appFrameCount=120 appFrameDelta=60 appFrameRateHz=30.00 xrUpdateCount=118 xrUpdateDelta=59 xrUpdateRateHz=29.50 drawEventCount=90 drawEventDelta=45 drawEventRateHz=22.50 leftTextureUpdateCount=32 rightTextureUpdateCount=31 pairedTextureUpdateCount=31 leftTextureUpdateDelta=16 rightTextureUpdateDelta=15 pairedTextureUpdateDelta=15 leftTextureUpdateRateHz=8.00 rightTextureUpdateRateHz=7.50 pairedTextureUpdateRateHz=7.50 leftLastPositionMs=1001 rightLastPositionMs=1003 pairedLeftRightCameraFrames=true projectionMappingReady=true alignedProjection=false visibleCameraProjectionReady=true renderPath=makepad-xr appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated xrDisplayRefreshRateHz=90.00 xrEffectiveFrameRateHz=30.00 xrFrameCpuMs=12.34 xrShouldRender=true xrSkippedShouldRenderCount=3 xrPreFrameEventsMs=0.25 xrPostFrameMediaEventsMs=1.25 xrWaitFrameMs=8.00 xrBeginFrameMs=0.10 xrLocateSpaceMs=0.11 xrLocateViewsMs=0.12 xrAcquireSwapchainMs=0.20 xrWaitSwapchainMs=0.30 xrAcquireDepthMs=0.35 xrUpdatePrepareMs=0.36 xrUpdateDispatchMs=0.40 xrNextFrameMs=0.50 xrDrawEventMs=0.60 xrCompileShadersMs=0.65 xrRepaintMs=1.70 xrRepaintGpuMs=2.10 xrRepaintWaitInflightMs=0.80 xrRepaintPrepareTexturesMs=0.90 xrRepaintRecordDrawMs=1.00 xrRepaintSubmitMs=1.10 xrRepaintTextureUploadCount=2 xrRepaintTextureUploadBytes=3456 xrRepaintPacketBufferCount=4 xrRepaintPacketBufferBytes=5678 xrRepaintGeometryUploadBytes=6789 xrRepaintDescriptorSetCount=8 xrRepaintDrawItems=9 xrRepaintDrawCalls=10 xrRepaintPackets=11 xrRepaintInstances=12 xrRepaintIndices=13 xrDepthReadbackMs=1.20 xrEndFrameMs=0.70 xrResizeProjectionMs=1.30 cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane visualColorStatus=accepted-cpu-yuv-reference"
        );
    }

    #[test]
    fn makepad_handoff_reports_direct_identity_contract() {
        let decision = direct_decision();
        let fields = MakepadSourceSamplingHandoff::new(
            false,
            false,
            &decision,
            SourceSamplingMode::ScreenToCameraHomography,
            0.0,
            false,
            "display-left-from-left-source",
            "identity-top-left-stimulus-raster",
            "projectionMetadataReady=true",
            "sourceColorTransformApplied=false",
            MakepadCameraTexturePath::direct_default(),
        )
        .marker_fields();
        let contract = MakepadSourceSamplingHandoff::new(
            false,
            false,
            &decision,
            SourceSamplingMode::ScreenToCameraHomography,
            0.0,
            false,
            "display-left-from-left-source",
            "identity-top-left-stimulus-raster",
            "projectionMetadataReady=true",
            "sourceColorTransformApplied=false",
            MakepadCameraTexturePath::direct_default(),
        )
        .contract();
        assert!(contract.is_valid());
        assert_eq!(contract.backend, "makepad");
        assert_eq!(
            contract.transform_stage,
            SourceSamplingTransformStage::PostHomographyPreYuvSample
        );
        assert_eq!(
            contract.sampler_y_axis,
            SourceSamplerYAxis::MakepadSamplerOriginConvention
        );
        assert!(fields.contains("phase=source-sampling status=ok"));
        assert!(fields
            .contains("sourceUvContract=screen_to_camera_content_uv_to_makepad_video_sampler"));
        assert!(fields.contains("sourceSamplingMode=screen-to-camera-homography"));
        assert!(fields.contains("sourceSampleTransformApplied=false"));
        assert!(fields.contains("sourceColorTransformApplied=false"));
        assert!(fields.contains("projectionContentMappingMode=screen-to-camera-homography"));
        assert!(fields.contains("sourceEyeMapping=display-left-from-left-source"));
    }

    #[test]
    fn makepad_handoff_reports_flipped_broker_stimulus() {
        let decision = flipped_decision();
        let fields = MakepadSourceSamplingHandoff::new(
            true,
            true,
            &decision,
            SourceSamplingMode::TargetLocalRaster,
            0.0,
            true,
            "display-left-from-right-source",
            "stimulus-raster-y-flip",
            "projectionMetadataReady=true",
            "sourceColorTransformApplied=true",
            MakepadCameraTexturePath::BrokerH264CpuYuv,
        )
        .marker_fields();
        let contract = MakepadSourceSamplingHandoff::new(
            true,
            true,
            &decision,
            SourceSamplingMode::TargetLocalRaster,
            0.0,
            true,
            "display-left-from-right-source",
            "stimulus-raster-y-flip",
            "projectionMetadataReady=true",
            "sourceColorTransformApplied=true",
            MakepadCameraTexturePath::BrokerH264CpuYuv,
        )
        .contract();
        assert_eq!(
            contract.source_eye_mapping,
            StereoSourceEyeMapping::DisplayLeftFromRightSource
        );
        assert!(contract.transform_applied);
        assert!(fields.contains("brokerH264Enabled=true"));
        assert!(fields.contains("explicitTopLeftBrokerStimulus=true"));
        assert!(fields.contains("orientationKind=broker_stimulus"));
        assert!(fields.contains("sourceSampleTransformApplied=true"));
        assert!(fields.contains("sourceColorTransformApplied=true"));
        assert!(fields.contains("projectionContentMappingMode=target-local-raster"));
        assert!(fields.contains("sourceSamplingMode=target-local-raster"));
        assert!(fields.contains("sourceSampleInputUv=target-local-raster-uv"));
    }

    #[test]
    fn texture_content_probe_markers_keep_source_sampling_shape() {
        let missing =
            makepad_texture_content_probe_missing_marker_fields("left", true, false, 601.0, 90.0);
        assert!(missing.starts_with("phase=texture-content-probe status=missing side=left"));
        assert!(missing.contains("textureProbeMode=single-quad-target-screen-uv"));
        assert!(missing.contains("yuvEnabled=true yuvBiplanar=false yuvMatrix=601.0"));
        assert!(missing.ends_with(
            "cpuPlaneContentPresent=false visualInspection=required visualReleaseAccepted=false"
        ));

        let ok = makepad_texture_content_probe_ok_marker_fields(
            "right",
            true,
            true,
            601.0,
            270.0,
            true,
            "yReadable=true",
            "uReadable=true",
            "vReadable=true",
        );
        assert!(ok.starts_with("phase=texture-content-probe status=ok side=right"));
        assert!(
            ok.contains("cpuPlaneContentPresent=true yReadable=true uReadable=true vReadable=true")
        );
        assert!(ok.ends_with(
            "gpuSamplingStillVisual=full-frame-source-display-row-vertical-uv-yuv visualInspection=required visualReleaseAccepted=false"
        ));
    }
}
