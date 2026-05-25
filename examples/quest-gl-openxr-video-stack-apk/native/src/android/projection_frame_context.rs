use openxr as xr;

use super::{
    openxr_gles_config::OesProjectionRuntimeState,
    projection_contract_markers::{
        openxr_projection_contract_fields, projection_area_target_marker_fields_from_state,
    },
    projection_geometry::OesProjectionPlan,
    projection_plan_builders::projection_plan_from_metadata_and_state,
    source_metadata::OesProjectionMetadata,
};

pub(super) struct OesProjectionFrameContext {
    pub(super) projection_plan: Option<OesProjectionPlan>,
    pub(super) openxr_projection_fields: String,
    pub(super) projection_area_target_fields: String,
}

pub(super) fn projection_frame_context_from_state(
    reference_space_label: &str,
    predicted_display_time: xr::Time,
    views: &[xr::View],
    projection_state: OesProjectionRuntimeState,
    metadata_pair: Option<(&OesProjectionMetadata, &OesProjectionMetadata)>,
) -> OesProjectionFrameContext {
    OesProjectionFrameContext {
        projection_plan: metadata_pair.and_then(|(left, right)| {
            projection_plan_from_metadata_and_state(left, right, views, projection_state)
        }),
        openxr_projection_fields: openxr_projection_contract_fields(
            reference_space_label,
            predicted_display_time,
            views,
        ),
        projection_area_target_fields: projection_area_target_marker_fields_from_state(
            projection_state,
        ),
    }
}
