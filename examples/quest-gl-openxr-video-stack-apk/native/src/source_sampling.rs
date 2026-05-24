pub(crate) const OES_SOURCE_UV_CONTRACT: &str =
    "screen_to_camera_content_uv_to_oes_external_sampler";

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

    pub(crate) fn marker_fields(self) -> String {
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
            "surface-texture-transform-defined"
        } else {
            "content-top-left-y-down"
        };
        format!(
            "sourceEyeMapping=display-left-from-left-source:sourceUvContract={}:sourceHomographyOutputUv=content-normalized-top-left-y-down:sourceSampleInputUv=screen-to-camera-homography-output:sourceSampleTransformStage=post_homography_pre_oes_sample:sourceSampleTransform={}:sourceSampleTransformOwner={}:sourceSampleTransformApplied={}:sourceSampleOutputUv=oes-external-sampler-uv:sourceSamplerUvOrigin=android-surface-texture:sourceSamplerYAxis={}:sourceTextureTransformStage=post_homography_pre_oes_sample:sourceTextureTransformOwner=android-surface-texture:contentUvRect=0,0,1,1",
            OES_SOURCE_UV_CONTRACT,
            source_sample_transform,
            source_sample_transform_owner,
            self.use_surface_texture_transform,
            source_sampler_y_axis
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oes_handoff_reports_identity_metadata_transform_without_surface_texture() {
        let fields = OesSourceSamplingHandoff::new(false).marker_fields();
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
        let fields = OesSourceSamplingHandoff::new(true).marker_fields();
        assert!(fields.contains("sourceSampleTransform=surfaceTextureTransformMatrix"));
        assert!(fields.contains("sourceSampleTransformOwner=android-surface-texture"));
        assert!(fields.contains("sourceSampleTransformApplied=true"));
        assert!(fields.contains("sourceSamplerYAxis=surface-texture-transform-defined"));
    }
}
