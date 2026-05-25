pub(super) use super::projection_homography_markers::projected_homography_status_marker_fields;
pub(super) use super::projection_openxr_contract_markers::{
    display_eye_uv_fiducial_contract_log_message, display_eye_uv_fiducial_marker_fields,
    projection_openxr_contract_log_message,
};

#[cfg(test)]
mod tests {
    use super::{
        display_eye_uv_fiducial_contract_log_message, display_eye_uv_fiducial_marker_fields,
        projected_homography_status_marker_fields,
    };
    use crate::{camera_color_pipeline::CameraProjectionEffectMode, RuntimeConfig};

    #[test]
    fn display_eye_uv_fiducial_marker_fields_keep_contract_shape() {
        let mut config = RuntimeConfig::default();

        config.camera_projection_effect_mode = CameraProjectionEffectMode::DisplayEyeUvFiducial;
        let display_eye = display_eye_uv_fiducial_marker_fields(&config);
        assert!(display_eye.contains("displayEyeUvFiducialActive=true"));
        assert!(display_eye.contains("displayEyeUvFiducialCoordinateSpace=display-eye-screen-uv"));
        assert!(display_eye.contains("displayEyeUvFiducialShaderFormula=displayEyeUv="));

        config.camera_projection_effect_mode = CameraProjectionEffectMode::SourceSamplingWitness;
        let source_sampling = display_eye_uv_fiducial_marker_fields(&config);
        assert!(source_sampling.contains("schema=rusty.xr.source_sampling_witness.v1"));
        assert!(
            source_sampling.contains("displayEyeUvFiducialCoordinateSpace=source-sampling-witness")
        );

        config.camera_projection_effect_mode = CameraProjectionEffectMode::BorderComposite;
        assert_eq!(
            display_eye_uv_fiducial_marker_fields(&config),
            "displayEyeUvFiducialActive=false"
        );
    }

    #[test]
    fn display_eye_uv_fiducial_contract_log_message_keeps_prefix_shape() {
        assert_eq!(
            display_eye_uv_fiducial_contract_log_message(
                7,
                42,
                "displayEyeUvFiducialActive=true"
            ),
            "Rusty XR display-eye UV fiducial contract frame=7 openXrFrameCount=42 displayEyeUvFiducialActive=true"
        );
    }

    #[test]
    fn projected_homography_status_marker_fields_keeps_missing_shape() {
        assert_eq!(
            projected_homography_status_marker_fields(None, None, &RuntimeConfig::default()),
            "projectionHomographyReady=false projectionAreaTransformStage=none projectionAreaWarpParity=reference_unwarped_screen_uv"
        );
    }
}
