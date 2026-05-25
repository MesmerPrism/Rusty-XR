use rusty_xr_contracts::{Eye, ProjectionStageKind, ProjectionStageTokenRow};

use super::{
    projection_geometry::{identity_homography, OesEyeProjection},
    OES_COPY_RENDER_PATH, OES_PROJECTED_RENDER_PATH,
};

pub(super) fn projection_stage_rows(
    projection: Option<&OesEyeProjection>,
) -> [(ProjectionStageKind, [[f32; 3]; 3]); 4] {
    projection
        .map(|projection| {
            [
                (
                    ProjectionStageKind::SurfaceToScreen,
                    projection.surface_to_screen_h,
                ),
                (
                    ProjectionStageKind::ScreenToSurface,
                    projection.screen_to_surface_h,
                ),
                (
                    ProjectionStageKind::SurfaceToCamera,
                    projection.surface_to_camera_h,
                ),
                (
                    ProjectionStageKind::ScreenToCamera,
                    projection.screen_to_camera_h,
                ),
            ]
        })
        .unwrap_or([
            (ProjectionStageKind::SurfaceToScreen, identity_homography()),
            (ProjectionStageKind::ScreenToSurface, identity_homography()),
            (ProjectionStageKind::SurfaceToCamera, identity_homography()),
            (ProjectionStageKind::ScreenToCamera, identity_homography()),
        ])
}

pub(super) fn projection_stage_row_log_messages(
    eye: Eye,
    projection: Option<&OesEyeProjection>,
    frame_count: u64,
    source_sequence: u64,
) -> Vec<Result<String, String>> {
    let compact_source_label =
        projection_stage_source_label(projection, frame_count, source_sequence);
    projection_stage_rows(projection)
        .into_iter()
        .map(|(stage, rows)| {
            let row = ProjectionStageTokenRow::new("rusty_xr_gl_oes", eye, stage)
                .with_rows(rows)
                .with_source(compact_source_label.clone());
            serde_json::to_string(&row)
                .map(|json| format!("Rusty XR OpenXR GLES projection stage row {json}"))
                .map_err(|error| {
                    format!("Rusty XR OpenXR GLES projection stage serialization failed: {error}")
                })
        })
        .collect()
}

fn projection_stage_source_label(
    projection: Option<&OesEyeProjection>,
    frame_count: u64,
    source_sequence: u64,
) -> String {
    projection
        .map(|projection| {
            format!(
                "{OES_PROJECTED_RENDER_PATH}:source_eye={}:frame={frame_count}:source_sequence={source_sequence}",
                projection.source_eye
            )
        })
        .unwrap_or_else(|| {
            format!("{OES_COPY_RENDER_PATH}:frame={frame_count}:source_sequence={source_sequence}")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_projection_stage_row_log_wrapper_keeps_shape() {
        let rows = projection_stage_row_log_messages(Eye::Left, None, 12, 34);
        assert_eq!(rows.len(), 4);
        let first = rows[0].as_ref().expect("stage row should serialize");
        assert!(first.starts_with("Rusty XR OpenXR GLES projection stage row {"));
        assert!(first.contains("\"backend\":\"rusty_xr_gl_oes\""));
        assert!(first.contains("rusty_xr_gl_oes:frame=12:source_sequence=34"));
    }
}
