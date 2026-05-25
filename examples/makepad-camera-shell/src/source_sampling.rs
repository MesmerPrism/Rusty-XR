use crate::FrameOrientationDecision;
use rusty_xr_contracts::{
    SourceSamplerYAxis, SourceSamplingContract, SourceSamplingTransformStage, StereoSourceEyeMapping,
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
}

pub(crate) struct MakepadSourceSamplingHandoff<'a> {
    broker_h264_enabled: bool,
    explicit_top_left_broker_stimulus: bool,
    orientation_decision: &'a FrameOrientationDecision,
    projection_content_mapping_mode: f32,
    full_frame_diagnostic: bool,
    source_eye_mapping: &'a str,
    source_sample_transform: &'a str,
    content_geometry_fields: &'a str,
    source_color_contract_fields: &'a str,
}

impl<'a> MakepadSourceSamplingHandoff<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        broker_h264_enabled: bool,
        explicit_top_left_broker_stimulus: bool,
        orientation_decision: &'a FrameOrientationDecision,
        projection_content_mapping_mode: f32,
        full_frame_diagnostic: bool,
        source_eye_mapping: &'a str,
        source_sample_transform: &'a str,
        content_geometry_fields: &'a str,
        source_color_contract_fields: &'a str,
    ) -> Self {
        Self {
            broker_h264_enabled,
            explicit_top_left_broker_stimulus,
            orientation_decision,
            projection_content_mapping_mode,
            full_frame_diagnostic,
            source_eye_mapping,
            source_sample_transform,
            content_geometry_fields,
            source_color_contract_fields,
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
            "phase=source-sampling status=ok brokerH264Enabled={} explicitTopLeftBrokerStimulus={} orientationKind={} rasterOrientation={} uprightMarker={} orientationMetadataSource={} orientationDefault={} orientationFallbackReason={} sourceSampleYFlip={:.1} sourceSampleYFlipReason={} projectionContentMappingMode={} sourceEyeMapping={} sourceUvContract={} sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv=screen-to-camera-homography-output sourceSampleTransformStage={} sourceSampleTransform={} sourceSampleTransformOwner={} sourceSampleTransformApplied={} sourceSampleOutputUv={} sourceSamplerUvOrigin={} sourceSamplerYAxis={} sourceTextureTransformStage={} sourceTextureTransformOwner={} diagnosticUvTransform={} sourceRasterYMappingStage={} rendererSurfaceUvOrigin=makepad-renderer-surface-uv displayScreenUvOrigin=top-left-origin-y-down displayScreenUvNormalization=renderer-v-flip-to-display-screen-uv {} {}",
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
            contract.source_eye_mapping.stable_id(),
            MAKEPAD_SOURCE_UV_CONTRACT,
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
        )
    }

    fn projection_content_mapping_label(&self) -> &'static str {
        if self.projection_content_mapping_mode >= 0.5 {
            "full-frame-stimulus-to-projection-area"
        } else if self.full_frame_diagnostic {
            "full-frame-stimulus-to-surface-homography"
        } else {
            "camera-projection-homography"
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
        "RUSTY_XR_MAKEPAD_CADENCE schema=rusty.xr.makepad-cadence.v1 phase=sample status=ok elapsedMs={:.0} intervalMs={:.0} appFrameCount={} appFrameDelta={} appFrameRateHz={:.2} xrUpdateCount={} xrUpdateDelta={} xrUpdateRateHz={:.2} drawEventCount={} drawEventDelta={} drawEventRateHz={:.2} leftTextureUpdateCount={} rightTextureUpdateCount={} pairedTextureUpdateCount={} leftTextureUpdateDelta={} rightTextureUpdateDelta={} pairedTextureUpdateDelta={} leftTextureUpdateRateHz={:.2} rightTextureUpdateRateHz={:.2} pairedTextureUpdateRateHz={:.2} leftLastPositionMs={} rightLastPositionMs={} pairedLeftRightCameraFrames={} projectionMappingReady={} alignedProjection={} visibleCameraProjectionReady={} cpuUploadPath=makepad-camera-cpu-yuv-plane renderPath=makepad-xr appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated",
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
    )
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
        });

        assert_eq!(
            marker,
            "RUSTY_XR_MAKEPAD_CADENCE schema=rusty.xr.makepad-cadence.v1 phase=sample status=ok elapsedMs=4250 intervalMs=2000 appFrameCount=120 appFrameDelta=60 appFrameRateHz=30.00 xrUpdateCount=118 xrUpdateDelta=59 xrUpdateRateHz=29.50 drawEventCount=90 drawEventDelta=45 drawEventRateHz=22.50 leftTextureUpdateCount=32 rightTextureUpdateCount=31 pairedTextureUpdateCount=31 leftTextureUpdateDelta=16 rightTextureUpdateDelta=15 pairedTextureUpdateDelta=15 leftTextureUpdateRateHz=8.00 rightTextureUpdateRateHz=7.50 pairedTextureUpdateRateHz=7.50 leftLastPositionMs=1001 rightLastPositionMs=1003 pairedLeftRightCameraFrames=true projectionMappingReady=true alignedProjection=false visibleCameraProjectionReady=true cpuUploadPath=makepad-camera-cpu-yuv-plane renderPath=makepad-xr appFrameSource=makepad-next-frame cameraFrameSource=makepad-video-texture-updated"
        );
    }

    #[test]
    fn makepad_handoff_reports_direct_identity_contract() {
        let decision = direct_decision();
        let fields = MakepadSourceSamplingHandoff::new(
            false,
            false,
            &decision,
            0.0,
            false,
            "display-left-from-left-source",
            "identity-top-left-stimulus-raster",
            "projectionMetadataReady=true",
            "sourceColorTransformApplied=false",
        )
        .marker_fields();
        let contract = MakepadSourceSamplingHandoff::new(
            false,
            false,
            &decision,
            0.0,
            false,
            "display-left-from-left-source",
            "identity-top-left-stimulus-raster",
            "projectionMetadataReady=true",
            "sourceColorTransformApplied=false",
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
        assert!(fields.contains(
            "sourceUvContract=screen_to_camera_content_uv_to_makepad_video_sampler"
        ));
        assert!(fields.contains("sourceSampleTransformApplied=false"));
        assert!(fields.contains("sourceColorTransformApplied=false"));
        assert!(fields.contains("projectionContentMappingMode=camera-projection-homography"));
        assert!(fields.contains("sourceEyeMapping=display-left-from-left-source"));
    }

    #[test]
    fn makepad_handoff_reports_flipped_broker_stimulus() {
        let decision = flipped_decision();
        let fields = MakepadSourceSamplingHandoff::new(
            true,
            true,
            &decision,
            0.0,
            true,
            "display-left-from-right-source",
            "stimulus-raster-y-flip",
            "projectionMetadataReady=true",
            "sourceColorTransformApplied=true",
        )
        .marker_fields();
        let contract = MakepadSourceSamplingHandoff::new(
            true,
            true,
            &decision,
            0.0,
            true,
            "display-left-from-right-source",
            "stimulus-raster-y-flip",
            "projectionMetadataReady=true",
            "sourceColorTransformApplied=true",
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
        assert!(fields
            .contains("projectionContentMappingMode=full-frame-stimulus-to-surface-homography"));
    }

    #[test]
    fn texture_content_probe_markers_keep_source_sampling_shape() {
        let missing = makepad_texture_content_probe_missing_marker_fields(
            "left", true, false, 601.0, 90.0,
        );
        assert!(missing.starts_with(
            "phase=texture-content-probe status=missing side=left"
        ));
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
        assert!(ok.contains(
            "cpuPlaneContentPresent=true yReadable=true uReadable=true vReadable=true"
        ));
        assert!(ok.ends_with(
            "gpuSamplingStillVisual=full-frame-source-display-row-vertical-uv-yuv visualInspection=required visualReleaseAccepted=false"
        ));
    }
}
