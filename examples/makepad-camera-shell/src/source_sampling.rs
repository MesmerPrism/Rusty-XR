use crate::FrameOrientationDecision;

pub(crate) const MAKEPAD_SOURCE_UV_CONTRACT: &str =
    "screen_to_camera_content_uv_to_makepad_video_sampler";

pub(crate) struct MakepadSourceSamplingHandoff<'a> {
    broker_h264_enabled: bool,
    explicit_top_left_broker_stimulus: bool,
    orientation_decision: &'a FrameOrientationDecision,
    projection_content_mapping_mode: f32,
    full_frame_diagnostic: bool,
    source_eye_mapping: &'a str,
    source_sample_transform: &'a str,
    content_geometry_fields: &'a str,
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
        }
    }

    pub(crate) fn marker_fields(&self) -> String {
        format!(
            "phase=source-sampling status=ok brokerH264Enabled={} explicitTopLeftBrokerStimulus={} orientationKind={} rasterOrientation={} uprightMarker={} orientationMetadataSource={} orientationDefault={} orientationFallbackReason={} sourceSampleYFlip={:.1} sourceSampleYFlipReason={} projectionContentMappingMode={} sourceEyeMapping={} sourceUvContract={} sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv=screen-to-camera-homography-output sourceSampleTransformStage=post_homography_pre_yuv_sample sourceSampleTransform={} sourceSampleTransformOwner=makepad-shader-source_sample_uv sourceSampleTransformApplied={} sourceSampleOutputUv=makepad-video-sampler-uv sourceSamplerUvOrigin=makepad-video-sampler sourceSamplerYAxis=makepad-sampler-origin-convention sourceTextureTransformStage=post_homography_pre_yuv_sample sourceTextureTransformOwner=makepad-shader-source_sample_uv diagnosticUvTransform={} sourceRasterYMappingStage={} rendererSurfaceUvOrigin=makepad-renderer-surface-uv displayScreenUvOrigin=top-left-origin-y-down displayScreenUvNormalization=renderer-v-flip-to-display-screen-uv {}",
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
            marker_token(self.source_eye_mapping),
            MAKEPAD_SOURCE_UV_CONTRACT,
            self.source_sample_transform,
            self.orientation_decision.source_sample_y_flip >= 0.5,
            self.source_sample_transform,
            self.source_sample_transform,
            self.content_geometry_fields,
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

fn marker_token(value: &str) -> String {
    value.replace(char::is_whitespace, "_")
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
        )
        .marker_fields();
        assert!(fields.contains("phase=source-sampling status=ok"));
        assert!(fields.contains(
            "sourceUvContract=screen_to_camera_content_uv_to_makepad_video_sampler"
        ));
        assert!(fields.contains("sourceSampleTransformApplied=false"));
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
        )
        .marker_fields();
        assert!(fields.contains("brokerH264Enabled=true"));
        assert!(fields.contains("explicitTopLeftBrokerStimulus=true"));
        assert!(fields.contains("orientationKind=broker_stimulus"));
        assert!(fields.contains("sourceSampleTransformApplied=true"));
        assert!(fields
            .contains("projectionContentMappingMode=full-frame-stimulus-to-surface-homography"));
    }
}
