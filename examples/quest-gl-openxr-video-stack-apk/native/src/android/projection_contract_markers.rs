use openxr as xr;

use super::openxr_gles_config::{OesProjectionAlphaMode, OesProjectionRuntimeState};

pub(super) fn projection_area_screen_uv_rect(
    offset_uv: [f32; 2],
    radius_uv: [f32; 2],
    scale_uv: [f32; 2],
) -> [f32; 4] {
    let scale_x = scale_uv[0].clamp(0.05, 4.0);
    let scale_y = scale_uv[1].clamp(0.05, 4.0);
    let radius_x = radius_uv[0].clamp(0.05, 0.5);
    let radius_y = radius_uv[1].clamp(0.05, 0.5);
    let center_x = 0.5 + offset_uv[0].clamp(-0.5, 0.5) / scale_x;
    let center_y = 0.5 + offset_uv[1].clamp(-0.5, 0.5) / scale_y;
    [
        center_x - radius_x / scale_x,
        center_y - radius_y / scale_y,
        (radius_x * 2.0) / scale_x,
        (radius_y * 2.0) / scale_y,
    ]
}

#[allow(clippy::too_many_arguments)]
pub(super) fn projection_area_target_marker_fields(
    left_offset_uv: [f32; 2],
    right_offset_uv: [f32; 2],
    radius_uv: [f32; 2],
    scale_uv: [f32; 2],
    projection_alpha_mode: OesProjectionAlphaMode,
    projection_alpha_scale: f32,
    projection_alpha_bias: f32,
    projection_depth_meters: f32,
    projection_preview_fov_y_degrees: f32,
    projection_preview_offset_y_meters: f32,
    projection_raw_overscan: f32,
) -> String {
    let left_feed_rect = projection_area_screen_uv_rect(left_offset_uv, radius_uv, scale_uv);
    let right_feed_rect = projection_area_screen_uv_rect(right_offset_uv, radius_uv, scale_uv);
    format!(
        "projectionAreaTargetSource=renderer-authored projectionAreaTargetStage=projection_area_mapping projectionAreaTargetCoordinateSpace=display-eye-screen-uv projectionAreaTargetRectSemantics=xywh projectionAreaOffsetConvention=positive-x-right-positive-y-down surfaceCoverageSource=renderer-authored surfaceCoverageSemantics=whole-render-target surfaceCoverageScreenUvRect=0.000000,0.000000,1.000000,1.000000 feedPlacementSource=renderer-authored feedPlacementSemantics=video_content_inside_surface borderRegionSemantics=surface_minus_feed projectionDepthMeters={:.3} cameraPreviewFovYDegrees={:.3} cameraPreviewOffsetYMeters={:.3} cameraRawOverlayOverscan={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} rendererSurfaceUvOrigin=gles-renderer-surface-uv displayScreenUvOrigin=top-left-origin-y-down displayScreenUvNormalization=renderer-v-flip-to-display-screen-uv leftProjectionAreaScreenUvRect={} rightProjectionAreaScreenUvRect={} leftFeedPlacementScreenUvRect={} rightFeedPlacementScreenUvRect={} leftProjectionAreaCenterUv={} rightProjectionAreaCenterUv={}",
        projection_depth_meters,
        projection_preview_fov_y_degrees,
        projection_preview_offset_y_meters,
        projection_raw_overscan,
        projection_alpha_mode.stable_id(),
        projection_alpha_scale,
        projection_alpha_bias,
        screen_uv_rect_token(left_feed_rect),
        screen_uv_rect_token(right_feed_rect),
        screen_uv_rect_token(left_feed_rect),
        screen_uv_rect_token(right_feed_rect),
        screen_uv_vec2_token(projection_area_center_uv(left_offset_uv, scale_uv)),
        screen_uv_vec2_token(projection_area_center_uv(right_offset_uv, scale_uv)),
    )
}

pub(super) fn projection_area_target_marker_fields_from_state(
    state: OesProjectionRuntimeState,
) -> String {
    projection_area_target_marker_fields(
        state.projection_area_eye_offset_uv[0],
        state.projection_area_eye_offset_uv[1],
        state.projection_area_radius,
        state.projection_area_scale,
        state.projection_alpha_mode,
        state.projection_alpha_scale,
        state.projection_alpha_bias,
        state.tuning.projection_depth_meters,
        state.tuning.camera_preview_fov_y_degrees,
        state.tuning.camera_preview_offset_y_meters,
        state.tuning.camera_raw_overlay_overscan,
    )
}

pub(super) fn openxr_projection_contract_fields(
    openxr_reference_space: &str,
    predicted_display_time: xr::Time,
    views: &[xr::View],
) -> String {
    let Some(left) = views.first() else {
        return format!(
            "referenceSpace=app-reference-space openxrReferenceSpace={openxr_reference_space} displayTimeSource=not-logged predictedDisplayTimeSource=not-logged predictedDisplayTimeNs=not-logged viewPoseFovSource=not-logged"
        );
    };
    let right = views.get(1).unwrap_or(left);
    format!(
        "referenceSpace=app-reference-space openxrReferenceSpace={openxr_reference_space} displayTimeSource=predicted-display-time predictedDisplayTimeSource=predicted-display-time predictedDisplayTimeNs={} viewPoseFovSource=xrLocateViews leftRenderFovTangents={} rightRenderFovTangents={} leftRenderPosition={} rightRenderPosition={} leftRenderOrientation={} rightRenderOrientation={}",
        predicted_display_time.as_nanos(),
        vec4_token(fov_tangents(left.fov)),
        vec4_token(fov_tangents(right.fov)),
        vec4_token(pose_position(left.pose)),
        vec4_token(pose_position(right.pose)),
        vec4_token(pose_orientation(left.pose)),
        vec4_token(pose_orientation(right.pose))
    )
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

fn projection_area_center_uv(offset_uv: [f32; 2], scale_uv: [f32; 2]) -> [f32; 2] {
    [
        0.5 + offset_uv[0].clamp(-0.5, 0.5) / scale_uv[0].clamp(0.05, 4.0),
        0.5 + offset_uv[1].clamp(-0.5, 0.5) / scale_uv[1].clamp(0.05, 4.0),
    ]
}

fn vec4_token(values: [f32; 4]) -> String {
    format!(
        "[{:.6},{:.6},{:.6},{:.6}]",
        values[0], values[1], values[2], values[3]
    )
}

fn fov_tangents(fov: xr::Fovf) -> [f32; 4] {
    [
        fov.angle_left.tan(),
        fov.angle_right.tan(),
        fov.angle_up.tan(),
        fov.angle_down.tan(),
    ]
}

fn pose_position(pose: xr::Posef) -> [f32; 4] {
    [pose.position.x, pose.position.y, pose.position.z, 1.0]
}

fn pose_orientation(pose: xr::Posef) -> [f32; 4] {
    [
        pose.orientation.x,
        pose.orientation.y,
        pose.orientation.z,
        pose.orientation.w,
    ]
}
