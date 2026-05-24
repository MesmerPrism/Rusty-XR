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
