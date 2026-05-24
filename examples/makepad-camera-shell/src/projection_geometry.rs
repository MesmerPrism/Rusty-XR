use rusty_xr_camera_model::{
    homography_unit_square_bounding_rect, rect_xywh, source_valid_screen_uv_footprint,
    uv_rect_token, Rect2,
};

use super::{
    homography_token, hotload_bool, hotload_f32, makepad_blur_radius_px,
    makepad_current_source_color_contract_fields, makepad_projection_depth_meters,
    makepad_projection_panel_geometry, makepad_projection_preview_fov_y_degrees,
    makepad_projection_preview_offset_y_meters, makepad_projection_raw_overscan, marker_token, App,
    MakepadCameraPair, MakepadOpenXrProjectionContract, MakepadProcessingLayer,
    MakepadProjectionAlphaMode, MakepadProjectionBorderPolicy,
    KEY_MAKEPAD_NATIVE_PASSTHROUGH_ENABLED, KEY_MAKEPAD_PROJECTION_AREA_OFFSET_LEFT_UV,
    KEY_MAKEPAD_PROJECTION_AREA_OFFSET_RIGHT_UV, KEY_MAKEPAD_PROJECTION_AREA_OFFSET_VERTICAL_UV,
    KEY_MAKEPAD_PROJECTION_AREA_RADIUS_X_UV, KEY_MAKEPAD_PROJECTION_AREA_RADIUS_Y_UV,
    KEY_MAKEPAD_PROJECTION_AREA_SCALE_X, KEY_MAKEPAD_PROJECTION_AREA_SCALE_Y,
    SOURCE_VALID_FOOTPRINT_GRID, TARGET_DISPLAY_ASPECT, TARGET_PROJECTION_AREA_OFFSET_LEFT_UV,
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

fn openxr_contract_marker_fields(contract: &MakepadOpenXrProjectionContract) -> String {
    format!(
        "referenceSpace={} openxrReferenceSpace={} displayTimeSource={} predictedDisplayTimeSource={} predictedDisplayTimeNs={} viewPoseFovSource={} projectionDepthMeters={} cameraPreviewFovYDegrees={} cameraPreviewOffsetYMeters={} cameraRawOverlayOverscan={} leftRenderFovTangents={} rightRenderFovTangents={} leftRenderPosition={} rightRenderPosition={} leftRenderOrientation={} rightRenderOrientation={}",
        marker_token(&contract.reference_space),
        marker_token(&contract.openxr_reference_space),
        marker_token(&contract.display_time_source),
        marker_token(&contract.display_time_source),
        optional_i64_token(contract.predicted_display_time_ns),
        marker_token(&contract.view_pose_fov_source),
        optional_f32_token(contract.projection_depth_meters),
        optional_f32_token(contract.projection_preview_fov_y_degrees),
        optional_f32_token(contract.projection_preview_offset_y_meters),
        optional_f32_token(contract.projection_raw_overscan),
        optional_vec4_token(contract.left_render_fov_tangents),
        optional_vec4_token(contract.right_render_fov_tangents),
        optional_vec4_token(contract.left_render_position),
        optional_vec4_token(contract.right_render_position),
        optional_vec4_token(contract.left_render_orientation),
        optional_vec4_token(contract.right_render_orientation),
    )
}

fn vec4_token(values: [f32; 4]) -> String {
    format!(
        "[{:.6},{:.6},{:.6},{:.6}]",
        values[0], values[1], values[2], values[3]
    )
}

fn optional_vec4_token(values: Option<[f32; 4]>) -> String {
    values
        .map(vec4_token)
        .unwrap_or_else(|| "not-logged".to_string())
}

fn optional_i64_token(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not-logged".to_string())
}

fn optional_f32_token(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "not-logged".to_string())
}

fn screen_uv_rect_token(rect: [f32; 4]) -> String {
    format!(
        "{:.6},{:.6},{:.6},{:.6}",
        rect[0], rect[1], rect[2], rect[3]
    )
}

fn screen_uv_vec2_token(value: [f32; 2]) -> String {
    format!("{:.6},{:.6}", value[0], value[1])
}

fn projection_area_screen_uv_rect(
    offset_x_uv: f32,
    offset_y_uv: f32,
    radius_x_uv: f32,
    radius_y_uv: f32,
    scale_x: f32,
    scale_y: f32,
) -> [f32; 4] {
    let scale_x = scale_x.clamp(0.05, 4.0);
    let scale_y = scale_y.clamp(0.05, 4.0);
    let radius_x = radius_x_uv.clamp(0.05, 0.5);
    let radius_y = radius_y_uv.clamp(0.05, 0.5);
    let center_x = 0.5 + offset_x_uv.clamp(-0.5, 0.5) / scale_x;
    let center_y = 0.5 + offset_y_uv.clamp(-0.5, 0.5) / scale_y;
    [
        center_x - radius_x / scale_x,
        center_y - radius_y / scale_y,
        (radius_x * 2.0) / scale_x,
        (radius_y * 2.0) / scale_y,
    ]
}

fn projection_area_center_uv(
    offset_x_uv: f32,
    offset_y_uv: f32,
    scale_x: f32,
    scale_y: f32,
) -> [f32; 2] {
    [
        0.5 + offset_x_uv.clamp(-0.5, 0.5) / scale_x.clamp(0.05, 4.0),
        0.5 + offset_y_uv.clamp(-0.5, 0.5) / scale_y.clamp(0.05, 4.0),
    ]
}

pub(crate) fn makepad_projection_target_marker_fields() -> String {
    let tuning = App::horizontal_alignment_tuning();
    let policy = MakepadProjectionBorderPolicy::from_shader_code(tuning.projection_border_policy);
    let processing_layer = MakepadProcessingLayer::current();
    let alpha_mode = MakepadProjectionAlphaMode::from_shader_code(tuning.projection_alpha_mode);
    let opacity_needs_passthrough =
        tuning.projection_area_opacity < 0.999 || tuning.projection_border_opacity < 0.999;
    let native_passthrough = hotload_bool(
        KEY_MAKEPAD_NATIVE_PASSTHROUGH_ENABLED,
        policy.wants_native_passthrough()
            || opacity_needs_passthrough
            || alpha_mode.uses_dynamic_alpha(),
    );
    let projection_depth_meters = makepad_projection_depth_meters();
    let preview_fov_y_degrees = makepad_projection_preview_fov_y_degrees();
    let preview_offset_y_meters = makepad_projection_preview_offset_y_meters();
    let raw_overscan = makepad_projection_raw_overscan();
    let panel_geometry = makepad_projection_panel_geometry();
    let native_projection_area_left_uv = tuning.projection_area_offset_left_uv;
    let native_projection_area_right_uv = tuning.projection_area_offset_right_uv;
    let native_projection_area_vertical_uv = tuning.projection_area_offset_vertical_uv;
    let projection_area_left_offset_x_uv = -native_projection_area_left_uv;
    let projection_area_right_offset_x_uv = -native_projection_area_right_uv;
    let projection_area_offset_y_uv = native_projection_area_vertical_uv;
    let projection_area_scale_x = tuning.projection_area_scale_x;
    let projection_area_scale_y = tuning.projection_area_scale_y;
    let projection_area_radius_x_uv = tuning.projection_area_radius_x_uv;
    let projection_area_radius_y_uv = tuning.projection_area_radius_y_uv;
    let projection_area_corner_radius_uv = tuning.projection_area_corner_radius_uv;
    let left_projection_area_rect = projection_area_screen_uv_rect(
        projection_area_left_offset_x_uv,
        projection_area_offset_y_uv,
        projection_area_radius_x_uv,
        projection_area_radius_y_uv,
        projection_area_scale_x,
        projection_area_scale_y,
    );
    let right_projection_area_rect = projection_area_screen_uv_rect(
        projection_area_right_offset_x_uv,
        projection_area_offset_y_uv,
        projection_area_radius_x_uv,
        projection_area_radius_y_uv,
        projection_area_scale_x,
        projection_area_scale_y,
    );
    let left_projection_area_center = projection_area_center_uv(
        projection_area_left_offset_x_uv,
        projection_area_offset_y_uv,
        projection_area_scale_x,
        projection_area_scale_y,
    );
    let right_projection_area_center = projection_area_center_uv(
        projection_area_right_offset_x_uv,
        projection_area_offset_y_uv,
        projection_area_scale_x,
        projection_area_scale_y,
    );
    let source_color_contract = makepad_current_source_color_contract_fields();
    format!(
        "nativePassthroughRequested={} projectionBorderPolicy={} passthroughUnderlay={} projectionDepthMeters={:.3} panelTargetDepthMeters={:.3} cameraPreviewFovYDegrees={:.3} cameraPreviewOffsetYMeters={:.3} cameraRawOverlayOverscan={:.3} panelTargetAspect={:.3} panelTargetWidthMeters={:.3} panelTargetHeightMeters={:.3} panelTargetCenterYMeters={:.3} panelTargetZMeters={:.3} projectionAreaOpacity={:.3} projectionBorderOpacity={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} processingLayer={} blurRadiusPx={:.2} {} projectionAreaLeftOffsetXUv={:.4} projectionAreaRightOffsetXUv={:.4} projectionAreaOffsetYUv={:.4} makepadNativeProjectionAreaLeftUv={:.4} makepadNativeProjectionAreaRightUv={:.4} makepadNativeProjectionAreaVerticalUv={:.4} projectionAreaScaleX={:.4} projectionAreaScaleY={:.4} projectionAreaRadiusXUv={:.4} projectionAreaRadiusYUv={:.4} projectionAreaCornerRadiusUv={:.4} projectionAreaTargetSource=renderer-authored projectionAreaTargetStage=projection_area_mapping projectionAreaTargetCoordinateSpace=display-eye-screen-uv projectionAreaTargetRectSemantics=xywh projectionAreaOffsetConvention=positive-x-right-positive-y-down surfaceCoverageSource=renderer-authored surfaceCoverageSemantics=panel-covers-target-fov feedPlacementSource=renderer-authored feedPlacementSemantics=video_content_inside_panel borderRegionSemantics=surface_minus_feed borderFillPolicy={} leftProjectionAreaScreenUvRect={} rightProjectionAreaScreenUvRect={} leftFeedPlacementScreenUvRect={} rightFeedPlacementScreenUvRect={} leftProjectionAreaCenterUv={} rightProjectionAreaCenterUv={} rendererSurfaceUvOrigin=makepad-renderer-surface-uv displayScreenUvOrigin=top-left-origin-y-down displayScreenUvNormalization=renderer-v-flip-to-display-screen-uv",
        native_passthrough,
        policy.stable_id(),
        policy.wants_native_passthrough(),
        projection_depth_meters,
        panel_geometry.depth_meters,
        preview_fov_y_degrees,
        preview_offset_y_meters,
        raw_overscan,
        TARGET_DISPLAY_ASPECT,
        panel_geometry.width_meters,
        panel_geometry.height_meters,
        panel_geometry.offset_y_meters,
        panel_geometry.z_meters,
        tuning.projection_area_opacity,
        tuning.projection_border_opacity,
        alpha_mode.stable_id(),
        tuning.projection_alpha_scale,
        tuning.projection_alpha_bias,
        processing_layer.stable_id(),
        makepad_blur_radius_px(),
        source_color_contract,
        projection_area_left_offset_x_uv,
        projection_area_right_offset_x_uv,
        projection_area_offset_y_uv,
        native_projection_area_left_uv,
        native_projection_area_right_uv,
        native_projection_area_vertical_uv,
        projection_area_scale_x,
        projection_area_scale_y,
        projection_area_radius_x_uv,
        projection_area_radius_y_uv,
        projection_area_corner_radius_uv,
        policy.shared_fill_policy_id(),
        screen_uv_rect_token(left_projection_area_rect),
        screen_uv_rect_token(right_projection_area_rect),
        screen_uv_rect_token(left_projection_area_rect),
        screen_uv_rect_token(right_projection_area_rect),
        screen_uv_vec2_token(left_projection_area_center),
        screen_uv_vec2_token(right_projection_area_center),
    )
}
