use rusty_xr_contracts::{
    Eye, InvalidProjectionFillPolicy, ProjectionFootprintRowSpan, ProjectionFootprintSummary,
    ProjectionGuideDomain, ProjectionStageKind, ProjectionStageTokenRow,
};

use super::{
    gl_format_label, log_error, log_info,
    openxr_gles_config::{OesColorControls, OesProjectionBorderPolicy, OesSourceColorTransfer},
    projection_geometry::{identity_homography, OesEyeProjection},
    GL_SRGB8_ALPHA8, OES_COPY_RENDER_PATH, OES_PROJECTED_RENDER_PATH,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct OesSourceColorContract<'a> {
    pub(super) input_encoding: &'a str,
    pub(super) transform: &'a str,
    pub(super) transform_applied: bool,
    pub(super) output_encoding: &'a str,
    pub(super) swapchain_color_format: &'a str,
    pub(super) swapchain_color_encoding: &'a str,
}

pub(super) fn projected_footprint_summary(
    projection: &OesEyeProjection,
    projection_border_policy: OesProjectionBorderPolicy,
    frame_count: u64,
    source_sequence: u64,
) -> ProjectionFootprintSummary {
    let footprint_plan = &projection.geometry_plan.source_valid_screen_uv_footprint;
    let mut footprint = ProjectionFootprintSummary::new(
        "rusty_xr_gl_oes",
        format!("public_projected_camera_uv_{}", eye_label(projection.eye)),
    )
    .with_active_fraction(footprint_plan.active_fraction)
    .with_bbox_fraction(footprint_plan.bbox_ltrb())
    .with_invalid_fill_policy(projection_border_policy.invalid_source_uv_fill_policy())
    .with_guide_domain(ProjectionGuideDomain::ScreenCamera)
    .with_explicit_valid_mask(false)
    .with_note(format!(
        "Metadata-derived camera-UV valid footprint from broker stream-header and OpenXR view state at frame {frame_count}, source sequence {source_sequence}; contentMappingMode={}."
        , projection.content_mapping_mode.stable_id()
    ));

    for row in &footprint_plan.row_spans {
        footprint = if let Some((x0, x1)) = row.span {
            footprint.with_row_span(
                ProjectionFootprintRowSpan::new(row.row_y, row.active_fraction).with_span(x0, x1),
            )
        } else {
            footprint.with_row_span(ProjectionFootprintRowSpan::new(row.row_y, 0.0))
        };
    }
    footprint
}

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

pub(super) fn source_color_contract_fields(fields: OesSourceColorContract<'_>) -> String {
    format!(
        "sourceColorInputEncoding={} sourceColorTransformStage=post_oes_sample_pre_camera_color_controls sourceColorTransform={} sourceColorTransformOwner=gles-oes-copy-shader sourceColorTransformApplied={} sourceColorOutputEncoding={} cameraColorControlStage=post_source_color_transfer swapchainColorFormat={} swapchainColorEncoding={}",
        fields.input_encoding,
        fields.transform,
        fields.transform_applied,
        fields.output_encoding,
        fields.swapchain_color_format,
        fields.swapchain_color_encoding
    )
}

pub(super) fn source_color_contract(
    camera_color_controls: OesColorControls,
    swapchain_color_format: u32,
) -> OesSourceColorContract<'static> {
    let transfer = camera_color_controls.source_transfer;
    OesSourceColorContract {
        input_encoding: transfer.input_encoding(),
        transform: transfer.stable_id(),
        transform_applied: transfer != OesSourceColorTransfer::Identity,
        output_encoding: transfer.output_encoding(),
        swapchain_color_format: gl_format_label(swapchain_color_format),
        swapchain_color_encoding: if swapchain_color_format == GL_SRGB8_ALPHA8 {
            "srgb"
        } else {
            "linear-or-runtime-default"
        },
    }
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

pub(super) fn projection_footprint_log_message(
    projection: Option<&OesEyeProjection>,
    projection_border_policy: OesProjectionBorderPolicy,
    frame_count: u64,
    source_sequence: u64,
) -> Result<String, String> {
    let footprint = projection
        .map(|projection| {
            projected_footprint_summary(
                projection,
                projection_border_policy,
                frame_count,
                source_sequence,
            )
        })
        .unwrap_or_else(|| raw_copy_footprint_summary(frame_count));
    serde_json::to_string(&footprint)
        .map(|json| format!("Rusty XR OpenXR GLES projection footprint {json}"))
        .map_err(|error| {
            format!("Rusty XR OpenXR GLES projection footprint serialization failed: {error}")
        })
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

pub(super) fn raw_copy_footprint_summary(frame_count: u64) -> ProjectionFootprintSummary {
    ProjectionFootprintSummary::new("rusty_xr_gl_oes", "public_raw_oes_full_surface")
        .with_active_fraction(1.0)
        .with_bbox_fraction([0.0, 0.0, 1.0, 1.0])
        .with_row_span(ProjectionFootprintRowSpan::new(0.0, 1.0).with_span(0.0, 1.0))
        .with_row_span(ProjectionFootprintRowSpan::new(0.5, 1.0).with_span(0.0, 1.0))
        .with_row_span(ProjectionFootprintRowSpan::new(1.0, 1.0).with_span(0.0, 1.0))
        .with_invalid_fill_policy(InvalidProjectionFillPolicy::NotApplicable)
        .with_guide_domain(ProjectionGuideDomain::SubmittedSurface)
        .with_explicit_valid_mask(true)
        .with_note(format!(
            "Full-surface public raw OES copy into the OpenXR GLES swapchain at frame {frame_count}."
        ))
}

pub(super) fn expected_source_valid_footprint_fields(projection: &OesEyeProjection) -> String {
    let rect = screen_uv_rect_token(expected_source_valid_screen_uv_rect(projection));
    let eye_prefix = match projection.eye {
        Eye::Left => "left",
        Eye::Right => "right",
        Eye::Mono => "mono",
    };
    format!(
        "expectedSourceValidFootprintSource=renderer-authored expectedSourceValidFootprintStage=screen_to_camera_source_uv_bounds expectedSourceValidFootprintCoordinateSpace=display-eye-screen-uv expectedSourceValidFootprintMethod=renderer-grid-sampled-source-uv-validity expectedSourceValidFootprintRectSemantics=xywh {eye_prefix}ExpectedSourceValidScreenUvRect={rect} {}",
        projection.geometry_plan.marker_fields(eye_prefix)
    )
}

fn expected_source_valid_screen_uv_rect(projection: &OesEyeProjection) -> [f32; 4] {
    projection
        .geometry_plan
        .source_valid_screen_uv_footprint
        .bbox_xywh()
}

fn screen_uv_rect_token(rect: [f32; 4]) -> String {
    format!(
        "{:.6},{:.6},{:.6},{:.6}",
        rect[0], rect[1], rect[2], rect[3]
    )
}

fn eye_label(eye: Eye) -> &'static str {
    match eye {
        Eye::Left => "left",
        Eye::Right => "right",
        Eye::Mono => "mono",
    }
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

        let footprint =
            projection_footprint_log_message(None, OesProjectionBorderPolicy::SolidRed, 12, 34)
                .expect("footprint should serialize");
        assert!(footprint.starts_with("Rusty XR OpenXR GLES projection footprint {"));
        assert!(footprint.contains("public_raw_oes_full_surface"));
        assert!(footprint.contains("Full-surface public raw OES copy"));
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

    #[test]
    fn source_color_contract_fields_keep_marker_shape() {
        assert_eq!(
            source_color_contract_fields(OesSourceColorContract {
                input_encoding: "external-oes-srgb-nonlinear-rgb",
                transform: "srgb-to-linear",
                transform_applied: true,
                output_encoding: "linear-rgb",
                swapchain_color_format: "GL_SRGB8_ALPHA8",
                swapchain_color_encoding: "srgb",
            }),
            "sourceColorInputEncoding=external-oes-srgb-nonlinear-rgb sourceColorTransformStage=post_oes_sample_pre_camera_color_controls sourceColorTransform=srgb-to-linear sourceColorTransformOwner=gles-oes-copy-shader sourceColorTransformApplied=true sourceColorOutputEncoding=linear-rgb cameraColorControlStage=post_source_color_transfer swapchainColorFormat=GL_SRGB8_ALPHA8 swapchainColorEncoding=srgb"
        );
    }
}
