use openxr as xr;

use super::{
    openxr_gles_config::OesProjectionRuntimeState,
    projection_contract_markers::openxr_projection_contract_fields,
    projection_geometry::OesProjectionPlan,
    projection_plan_builders::projection_plan_from_metadata_and_state,
    projection_target_footprint::{
        target_footprints_from_metadata_pair_or_state, target_footprints_from_state,
        OesTargetFootprintSource,
    },
    source_metadata::OesProjectionMetadata,
};

pub(super) struct OesProjectionFrameContext {
    pub(super) projection_plan: Option<OesProjectionPlan>,
    pub(super) target_projection_state: OesProjectionRuntimeState,
    pub(super) target_footprint_from_metadata: bool,
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
    let target_footprints = metadata_pair
        .map(|(left, right)| {
            target_footprints_from_metadata_pair_or_state(left, right, projection_state)
        })
        .unwrap_or_else(|| target_footprints_from_state(projection_state));
    let target_footprint_from_metadata =
        target_footprints.source == OesTargetFootprintSource::SourceMetadata;
    let target_projection_state = target_footprints.apply_to_state(projection_state);
    OesProjectionFrameContext {
        projection_plan: metadata_pair.and_then(|(left, right)| {
            projection_plan_from_metadata_and_state(
                left,
                right,
                views,
                target_projection_state,
                target_footprint_from_metadata,
            )
        }),
        target_projection_state,
        target_footprint_from_metadata,
        openxr_projection_fields: openxr_projection_contract_fields(
            reference_space_label,
            predicted_display_time,
            views,
        ),
        projection_area_target_fields: target_footprints.marker_fields(target_projection_state),
    }
}
