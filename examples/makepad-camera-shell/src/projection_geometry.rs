use rusty_xr_camera_model::{
    homography_unit_square_bounding_rect, rect_xywh, source_valid_screen_uv_footprint,
    uv_rect_token, Rect2,
};

use super::{
    homography_token, hotload_f32, makepad_projection_target_marker_fields, marker_token,
    openxr_contract_marker_fields, projection_area_screen_uv_rect, screen_uv_rect_token,
    MakepadCameraPair, MakepadProjectionBorderPolicy, KEY_MAKEPAD_PROJECTION_AREA_OFFSET_LEFT_UV,
    KEY_MAKEPAD_PROJECTION_AREA_OFFSET_RIGHT_UV, KEY_MAKEPAD_PROJECTION_AREA_OFFSET_VERTICAL_UV,
    KEY_MAKEPAD_PROJECTION_AREA_RADIUS_X_UV, KEY_MAKEPAD_PROJECTION_AREA_RADIUS_Y_UV,
    KEY_MAKEPAD_PROJECTION_AREA_SCALE_X, KEY_MAKEPAD_PROJECTION_AREA_SCALE_Y,
    SOURCE_VALID_FOOTPRINT_GRID, TARGET_PROJECTION_AREA_OFFSET_LEFT_UV,
    TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV, TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV,
    TARGET_PROJECTION_AREA_RADIUS_X_UV, TARGET_PROJECTION_AREA_RADIUS_Y_UV,
    TARGET_PROJECTION_AREA_SCALE_X, TARGET_PROJECTION_AREA_SCALE_Y,
};

pub(crate) fn projection_homography_marker_fields(pair: &MakepadCameraPair) -> String {
    format!(
        "projectionHomographyReady={} runtimeXrViewStateReady={} sourceBindingMode={} projectionGeometryProfile={} geometry_profile={} displayLeftCameraId={} displayRightCameraId={} makepadLeftCameraId={} makepadRightCameraId={} sourceRasterOriginPolicy=explicit-broker-raster-or-camera2-import sourceRasterUvCorrectionStage=projection-plan-for-broker-top-left-raster projectionAreaTransformStage=pre_homography_screen_uv projectionAreaWarpParity=diagnostic_only contentUvRect=0,0,1,1 cpuUploadRect=0,0,{},{} cpuUploadStride=not-exposed {} leftSurfaceToCameraH={} rightSurfaceToCameraH={} leftSurfaceToScreenH={} rightSurfaceToScreenH={} leftScreenToCameraH={} rightScreenToCameraH={} leftScreenToSurfaceH={} rightScreenToSurfaceH={} {} {}",
        pair.projection_homography_ready,
        pair.runtime_xr_view_state_ready,
        pair.source_binding_mode,
        marker_token(&pair.projection_geometry_profile),
        marker_token(&pair.projection_geometry_profile),
        marker_token(pair.left.camera_id.as_deref().unwrap_or("unknown")),
        marker_token(pair.right.camera_id.as_deref().unwrap_or("unknown")),
        marker_token(pair.left.camera_id.as_deref().unwrap_or("unknown")),
        marker_token(pair.right.camera_id.as_deref().unwrap_or("unknown")),
        pair.left.width,
        pair.left.height,
        openxr_contract_marker_fields(&pair.openxr_contract),
        homography_token(pair.left_surface_to_camera_h),
        homography_token(pair.right_surface_to_camera_h),
        homography_token(pair.left_surface_to_screen_h),
        homography_token(pair.right_surface_to_screen_h),
        homography_token(pair.left_screen_to_camera_h),
        homography_token(pair.right_screen_to_camera_h),
        homography_token(pair.left_screen_to_surface_h),
        homography_token(pair.right_screen_to_surface_h),
        expected_source_valid_footprint_marker_fields(pair),
        makepad_projection_target_marker_fields()
    )
}

fn expected_source_valid_screen_uv_rect(
    screen_to_camera_h: [[f32; 3]; 3],
    source_valid_uv_rect: Rect2,
) -> [f32; 4] {
    source_valid_screen_uv_footprint(
        screen_to_camera_h,
        source_valid_uv_rect,
        SOURCE_VALID_FOOTPRINT_GRID,
    )
    .bbox_xywh()
}

fn expected_source_valid_footprint_marker_fields(pair: &MakepadCameraPair) -> String {
    let left_rect = expected_source_valid_screen_uv_rect(
        pair.left_screen_to_camera_h,
        pair.left_source_valid_uv_rect,
    );
    let right_rect = expected_source_valid_screen_uv_rect(
        pair.right_screen_to_camera_h,
        pair.right_source_valid_uv_rect,
    );
    let policy = MakepadProjectionBorderPolicy::current();
    let native_left_offset = hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_OFFSET_LEFT_UV,
        TARGET_PROJECTION_AREA_OFFSET_LEFT_UV,
        -0.5,
        0.5,
    );
    let native_right_offset = hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_OFFSET_RIGHT_UV,
        TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV,
        -0.5,
        0.5,
    );
    let vertical_offset = hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_OFFSET_VERTICAL_UV,
        TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV,
        -0.5,
        0.5,
    );
    let scale_x = hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_SCALE_X,
        TARGET_PROJECTION_AREA_SCALE_X,
        0.05,
        4.0,
    );
    let scale_y = hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_SCALE_Y,
        TARGET_PROJECTION_AREA_SCALE_Y,
        0.05,
        4.0,
    );
    let radius_x = hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_RADIUS_X_UV,
        TARGET_PROJECTION_AREA_RADIUS_X_UV,
        0.05,
        0.5,
    );
    let radius_y = hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_RADIUS_Y_UV,
        TARGET_PROJECTION_AREA_RADIUS_Y_UV,
        0.05,
        0.5,
    );
    let left_feed_rect = projection_area_screen_uv_rect(
        -native_left_offset,
        vertical_offset,
        radius_x,
        radius_y,
        scale_x,
        scale_y,
    );
    let right_feed_rect = projection_area_screen_uv_rect(
        -native_right_offset,
        vertical_offset,
        radius_x,
        radius_y,
        scale_x,
        scale_y,
    );
    let left_surface_rect =
        homography_unit_square_bounding_rect(pair.left_surface_to_screen_h).unwrap_or(Rect2::UNIT);
    let right_surface_rect =
        homography_unit_square_bounding_rect(pair.right_surface_to_screen_h).unwrap_or(Rect2::UNIT);
    format!(
        "expectedSourceValidFootprintSource=renderer-authored expectedSourceValidFootprintStage=screen_to_camera_source_uv_bounds expectedSourceValidFootprintCoordinateSpace=display-eye-screen-uv expectedSourceValidFootprintMethod=renderer-grid-sampled-source-uv-validity expectedSourceValidFootprintRectSemantics=xywh projectionGeometrySchema=rusty.xr.video_projection_geometry.v1 projectionMapping=screen-to-source-homography surfaceCoverageSource=shared-homography feedPlacementSource=renderer-authored borderRegionSemantics=surface_minus_feed borderFillPolicy={} leftSurfaceCoverageScreenUvRect={} rightSurfaceCoverageScreenUvRect={} leftFeedPlacementScreenUvRect={} rightFeedPlacementScreenUvRect={} leftSourceValidUvRect={} rightSourceValidUvRect={} leftExpectedSourceValidScreenUvRect={} rightExpectedSourceValidScreenUvRect={}",
        policy.shared_fill_policy_id(),
        uv_rect_token(rect_xywh(left_surface_rect)),
        uv_rect_token(rect_xywh(right_surface_rect)),
        screen_uv_rect_token(left_feed_rect),
        screen_uv_rect_token(right_feed_rect),
        uv_rect_token(rect_xywh(pair.left_source_valid_uv_rect)),
        uv_rect_token(rect_xywh(pair.right_source_valid_uv_rect)),
        screen_uv_rect_token(left_rect),
        screen_uv_rect_token(right_rect),
    )
}
