use openxr as xr;

use super::{
    openxr_gles_config::{
        OesCameraProjectionMode, OesProjectionBorderPolicy, OesProjectionRuntimeState,
    },
    projection_geometry::OesProjectionPlan,
    projection_plan_profile_builders::{
        broker_full_frame_projection_plan_from_xr_views,
        broker_synthetic_projection_plan_from_xr_views, camera2_projection_plan_from_xr_views,
    },
    source_metadata::OesProjectionMetadata,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn projection_plan_from_metadata(
    left: &OesProjectionMetadata,
    right: &OesProjectionMetadata,
    views: &[xr::View],
    camera_projection_mode: OesCameraProjectionMode,
    projection_area_eye_offset_uv: [[f32; 2]; 2],
    projection_area_scale: [f32; 2],
    projection_area_radius: [f32; 2],
    projection_area_opacity: f32,
    projection_border_policy: OesProjectionBorderPolicy,
    projection_border_opacity: f32,
    projection_depth_meters: f32,
    projection_preview_fov_y_degrees: f32,
    projection_preview_offset_y_meters: f32,
    projection_raw_overscan: f32,
    target_footprint_from_metadata: bool,
) -> Option<OesProjectionPlan> {
    let width = left.delivered_width.max(right.delivered_width);
    let height = left.delivered_height.max(right.delivered_height);
    if !left.projection_metadata_ready
        || !right.projection_metadata_ready
        || width == 0
        || height == 0
    {
        return None;
    }
    let metadata_backed_projection = left.has_metadata_backed_camera_projection()
        && right.has_metadata_backed_camera_projection();
    let camera_projection_mapping =
        left.requests_camera_projection_mapping() && right.requests_camera_projection_mapping();
    let full_frame_diagnostic_profile =
        left.is_full_frame_diagnostic_projection() && right.is_full_frame_diagnostic_projection();
    let explicit_full_frame_content_mapping = left.requests_explicit_full_frame_content_mapping()
        && right.requests_explicit_full_frame_content_mapping();
    if explicit_full_frame_content_mapping
        || (full_frame_diagnostic_profile && !metadata_backed_projection)
    {
        broker_full_frame_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
            target_footprint_from_metadata,
        )
    } else if camera_projection_mode.uses_world_canvas() && metadata_backed_projection {
        camera2_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
            target_footprint_from_metadata,
        )
    } else if camera_projection_mode.uses_world_canvas() {
        broker_full_frame_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
            target_footprint_from_metadata,
        )
    } else if metadata_backed_projection
        && (camera_projection_mapping || full_frame_diagnostic_profile)
    {
        camera2_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
            target_footprint_from_metadata,
        )
    } else if left.requests_head_anchored_projection_area_mapping()
        && right.requests_head_anchored_projection_area_mapping()
    {
        broker_synthetic_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
            target_footprint_from_metadata,
        )
    } else if left.has_camera2_projection() && right.has_camera2_projection() {
        camera2_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
            target_footprint_from_metadata,
        )
    } else {
        None
    }
}

pub(super) fn projection_plan_from_metadata_and_state(
    left: &OesProjectionMetadata,
    right: &OesProjectionMetadata,
    views: &[xr::View],
    projection_state: OesProjectionRuntimeState,
    target_footprint_from_metadata: bool,
) -> Option<OesProjectionPlan> {
    projection_plan_from_metadata(
        left,
        right,
        views,
        projection_state.camera_projection_mode,
        projection_state.projection_area_eye_offset_uv,
        projection_state.projection_area_scale,
        projection_state.projection_area_radius,
        projection_state.projection_area_opacity,
        projection_state.projection_border_policy,
        projection_state.projection_border_opacity,
        projection_state.tuning.projection_depth_meters,
        projection_state.tuning.camera_preview_fov_y_degrees,
        projection_state.tuning.camera_preview_offset_y_meters,
        projection_state.tuning.camera_raw_overlay_overscan,
        target_footprint_from_metadata,
    )
}
