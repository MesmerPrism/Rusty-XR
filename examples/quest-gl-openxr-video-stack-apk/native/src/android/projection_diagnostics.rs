use rusty_xr_contracts::{Eye, ProjectionStageKind, ProjectionStageTokenRow};

use super::{
    log_error, log_info,
    openxr_gles_config::{OesColorControls, OesProjectionBorderPolicy},
    projection_footprints::{
        expected_source_valid_footprint_fields, projection_footprint_log_message,
    },
    projection_geometry::{identity_homography, OesEyeProjection},
    projection_source_color::{source_color_contract, source_color_contract_fields},
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

pub(super) fn projection_source_contract_fields(
    projection: Option<&OesEyeProjection>,
    frame_count: u64,
    source_sequence: u64,
) -> String {
    projection
        .map(|projection| {
            format!(
                "{} {} source_eye={}:content_mapping={}:frame={frame_count}:source_sequence={source_sequence}",
                projection.source_label,
                expected_source_valid_footprint_fields(projection),
                projection.source_eye,
                projection.content_mapping_mode.stable_id()
            )
        })
        .unwrap_or_else(|| {
            format!("{OES_COPY_RENDER_PATH}:frame={frame_count}:source_sequence={source_sequence}")
        })
}

pub(super) fn projection_stage_source_label(
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

pub(super) fn source_sampling_projection_contract_log_message(
    source_contract_fields: &str,
) -> String {
    format!(
        "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase=source-sampling status=ready {} projectionHomographyReady=true projectionMappingReady=true visibleCameraProjectionReady=true",
        source_contract_fields.replace(':', " ")
    )
}

pub(super) fn projection_coordinate_contract_log_message(phase: &str, fields: &str) -> String {
    format!(
        "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase={} status=ready {}",
        phase, fields
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn log_projection_diagnostics(
    view_index: usize,
    frame_count: u64,
    source_sequence: u64,
    projection: Option<&OesEyeProjection>,
    projection_border_policy: OesProjectionBorderPolicy,
    camera_color_controls: OesColorControls,
    swapchain_color_format: u32,
    openxr_projection_fields: &str,
    projection_area_target_fields: &str,
) {
    let source_color_fields = source_color_contract_fields(source_color_contract(
        camera_color_controls,
        swapchain_color_format,
    ));
    for message in projection_diagnostic_log_messages(
        view_index,
        frame_count,
        source_sequence,
        projection,
        projection_border_policy,
        &source_color_fields,
        openxr_projection_fields,
        projection_area_target_fields,
    ) {
        match message {
            Ok(line) => log_info(line),
            Err(error) => log_error(error),
        }
    }
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

#[allow(clippy::too_many_arguments)]
pub(super) fn projection_diagnostic_log_messages(
    view_index: usize,
    frame_count: u64,
    source_sequence: u64,
    projection: Option<&OesEyeProjection>,
    projection_border_policy: OesProjectionBorderPolicy,
    source_color_fields: &str,
    openxr_projection_fields: &str,
    projection_area_target_fields: &str,
) -> Vec<Result<String, String>> {
    let Some(eye) = eye_from_view_index(view_index) else {
        return Vec::new();
    };
    let source_contract_fields =
        projection_source_contract_fields(projection, frame_count, source_sequence);
    let mut messages = vec![
        Ok(source_sampling_projection_contract_log_message(
            &source_contract_fields,
        )),
        Ok(projection_coordinate_contract_log_message(
            "source-color",
            source_color_fields,
        )),
        Ok(projection_coordinate_contract_log_message(
            "projection-plan",
            openxr_projection_fields,
        )),
        Ok(projection_coordinate_contract_log_message(
            "draw-vars-bound",
            projection_area_target_fields,
        )),
    ];
    messages.extend(projection_stage_row_log_messages(
        eye,
        projection,
        frame_count,
        source_sequence,
    ));
    messages.push(projection_footprint_log_message(
        projection,
        projection_border_policy,
        frame_count,
        source_sequence,
    ));
    messages
}

fn eye_from_view_index(view_index: usize) -> Option<Eye> {
    match view_index {
        0 => Some(Eye::Left),
        1 => Some(Eye::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_projection_diagnostic_log_wrappers_keep_shape() {
        let rows = projection_stage_row_log_messages(Eye::Left, None, 12, 34);
        assert_eq!(rows.len(), 4);
        let first = rows[0].as_ref().expect("stage row should serialize");
        assert!(first.starts_with("Rusty XR OpenXR GLES projection stage row {"));
        assert!(first.contains("\"backend\":\"rusty_xr_gl_oes\""));
        assert!(first.contains("rusty_xr_gl_oes:frame=12:source_sequence=34"));
    }

    #[test]
    fn projection_diagnostic_log_messages_keep_contract_order() {
        let messages = projection_diagnostic_log_messages(
            0,
            12,
            34,
            None,
            OesProjectionBorderPolicy::SolidRed,
            "sourceColorTransform=identity",
            "referenceSpace=app-reference-space",
            "projectionAreaTargetSource=renderer-authored",
        );
        assert_eq!(messages.len(), 9);
        let rendered: Vec<String> = messages
            .into_iter()
            .map(|message| message.expect("diagnostic message should serialize"))
            .collect();
        assert!(rendered[0].contains("phase=source-sampling status=ready"));
        assert!(rendered[1].contains("phase=source-color status=ready"));
        assert!(rendered[2].contains("phase=projection-plan status=ready"));
        assert!(rendered[3].contains("phase=draw-vars-bound status=ready"));
        assert!(rendered[4].starts_with("Rusty XR OpenXR GLES projection stage row {"));
        assert!(rendered[8].starts_with("Rusty XR OpenXR GLES projection footprint {"));
    }
}
