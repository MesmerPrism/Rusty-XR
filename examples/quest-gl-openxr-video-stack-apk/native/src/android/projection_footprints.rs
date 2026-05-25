use rusty_xr_contracts::{
    Eye, InvalidProjectionFillPolicy, ProjectionFootprintRowSpan, ProjectionFootprintSummary,
    ProjectionGuideDomain,
};

use super::{openxr_gles_config::OesProjectionBorderPolicy, projection_geometry::OesEyeProjection};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_projection_footprint_log_wrapper_keeps_shape() {
        let footprint =
            projection_footprint_log_message(None, OesProjectionBorderPolicy::SolidRed, 12, 34)
                .expect("footprint should serialize");
        assert!(footprint.starts_with("Rusty XR OpenXR GLES projection footprint {"));
        assert!(footprint.contains("public_raw_oes_full_surface"));
        assert!(footprint.contains("Full-surface public raw OES copy"));
    }
}
