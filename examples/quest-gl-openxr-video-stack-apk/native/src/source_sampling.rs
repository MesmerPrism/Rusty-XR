use rusty_xr_contracts::{
    SourceSamplerYAxis, SourceSamplingContract, SourceSamplingTransformStage,
    StereoSourceEyeMapping,
};

pub(crate) const OES_SOURCE_UV_CONTRACT: &str =
    "screen_to_camera_content_uv_to_oes_external_sampler";
const OES_SOURCE_SAMPLING_BACKEND: &str = "oes";
const OES_SOURCE_SAMPLING_MODE: &str = "oes-runtime";
const OES_OUTPUT_UV_LABEL: &str = "oes-external-sampler-uv";
const OES_SAMPLER_UV_ORIGIN: &str = "android-surface-texture";
const OES_TEXTURE_TRANSFORM_OWNER: &str = "android-surface-texture";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct OesSourceSamplingHandoff {
    use_surface_texture_transform: bool,
}

impl OesSourceSamplingHandoff {
    pub(crate) const fn new(use_surface_texture_transform: bool) -> Self {
        Self {
            use_surface_texture_transform,
        }
    }

    pub(crate) fn contract(self) -> SourceSamplingContract {
        let source_sample_transform = if self.use_surface_texture_transform {
            "surfaceTextureTransformMatrix"
        } else {
            "identity"
        };
        let source_sample_transform_owner = if self.use_surface_texture_transform {
            "android-surface-texture"
        } else {
            "stimulus-orientation-metadata"
        };
        let source_sampler_y_axis = if self.use_surface_texture_transform {
            SourceSamplerYAxis::SurfaceTextureTransformDefined
        } else {
            SourceSamplerYAxis::ContentTopLeftYDown
        };
        SourceSamplingContract::new(
            OES_SOURCE_SAMPLING_BACKEND,
            OES_SOURCE_SAMPLING_MODE,
            StereoSourceEyeMapping::DisplayLeftFromLeftSource,
            SourceSamplingTransformStage::PostHomographyPreOesSample,
        )
        .with_transform(
            source_sample_transform,
            source_sample_transform_owner,
            self.use_surface_texture_transform,
        )
        .with_sampler(
            OES_OUTPUT_UV_LABEL,
            OES_SAMPLER_UV_ORIGIN,
            source_sampler_y_axis,
        )
        .with_texture_transform(
            SourceSamplingTransformStage::PostHomographyPreOesSample,
            OES_TEXTURE_TRANSFORM_OWNER,
        )
    }

    pub(crate) fn marker_fields(self) -> String {
        let contract = self.contract();
        format!(
            "sourceEyeMapping={}:sourceUvContract={}:sourceHomographyOutputUv=content-normalized-top-left-y-down:sourceSampleInputUv=screen-to-camera-homography-output:sourceSampleTransformStage={}:sourceSampleTransform={}:sourceSampleTransformOwner={}:sourceSampleTransformApplied={}:sourceSampleOutputUv={}:sourceSamplerUvOrigin={}:sourceSamplerYAxis={}:sourceTextureTransformStage={}:sourceTextureTransformOwner={}:contentUvRect=0,0,1,1",
            contract.source_eye_mapping.stable_id(),
            OES_SOURCE_UV_CONTRACT,
            legacy_transform_stage_token(contract.transform_stage),
            contract.transform_label,
            contract.transform_owner,
            contract.transform_applied,
            contract.output_uv_label,
            contract.sampler_uv_origin,
            contract.sampler_y_axis.stable_id(),
            legacy_transform_stage_token(contract.texture_transform_stage),
            contract.texture_transform_owner
        )
    }
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

    #[test]
    fn oes_handoff_reports_identity_metadata_transform_without_surface_texture() {
        let handoff = OesSourceSamplingHandoff::new(false);
        let contract = handoff.contract();
        let fields = handoff.marker_fields();
        assert!(contract.is_valid());
        assert_eq!(contract.backend, "oes");
        assert_eq!(
            contract.transform_stage,
            SourceSamplingTransformStage::PostHomographyPreOesSample
        );
        assert_eq!(
            contract.sampler_y_axis,
            SourceSamplerYAxis::ContentTopLeftYDown
        );
        assert!(
            fields.contains("sourceUvContract=screen_to_camera_content_uv_to_oes_external_sampler")
        );
        assert!(fields.contains("sourceSampleTransform=identity"));
        assert!(fields.contains("sourceSampleTransformOwner=stimulus-orientation-metadata"));
        assert!(fields.contains("sourceSampleTransformApplied=false"));
        assert!(fields.contains("sourceSamplerYAxis=content-top-left-y-down"));
    }

    #[test]
    fn oes_handoff_reports_surface_texture_transform_when_used() {
        let handoff = OesSourceSamplingHandoff::new(true);
        let contract = handoff.contract();
        let fields = handoff.marker_fields();
        assert_eq!(
            contract.sampler_y_axis,
            SourceSamplerYAxis::SurfaceTextureTransformDefined
        );
        assert!(contract.transform_applied);
        assert!(fields.contains("sourceSampleTransform=surfaceTextureTransformMatrix"));
        assert!(fields.contains("sourceSampleTransformOwner=android-surface-texture"));
        assert!(fields.contains("sourceSampleTransformApplied=true"));
        assert!(fields.contains("sourceSamplerYAxis=surface-texture-transform-defined"));
    }
}
