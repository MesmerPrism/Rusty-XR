use rusty_xr_camera_model::{
    source_valid_screen_uv_footprint, target_footprint_debug_region_marker_fields, Rect2,
};

use super::projection_geometry::{DisplayEyeProjectionMapping, ProjectedStereoHomographies};
use super::projection_target_footprint::{
    target_footprint_params_from_stereo_frame, HwbTargetFootprintParams,
};
use crate::StereoGpuCameraFrame;

const SOURCE_VALID_FOOTPRINT_GRID: usize = 64;

pub(super) fn projected_homography_marker_fields(
    homographies: &ProjectedStereoHomographies,
    config: &crate::RuntimeConfig,
    target_footprint: HwbTargetFootprintParams,
) -> String {
    let surface_aspect_contract = if config.camera_projection_mode.uses_world_canvas() {
        "full_target_canvas_aspect"
    } else {
        "content_frame_aspect_not_display_eye_fov"
    };
    format!(
        "projectionHomographyReady=true projectionAreaTransformStage=screen_space_xy_offset projectionAreaWarpParity=reference_unwarped_screen_uv projectionCanvasMode={} projectionCanvasSampleRows={} projectionCanvasIndicator={} projectionSurfaceAspectContract={} leftProjectionSurfaceAspect={:.6} rightProjectionSurfaceAspect={:.6} leftProjectionSurfaceAspectSource={} rightProjectionSurfaceAspectSource={} leftSurfaceToCameraH={} rightSurfaceToCameraH={} leftScreenToCameraH={} rightScreenToCameraH={} leftScreenToSurfaceH={} rightScreenToSurfaceH={} leftSurfaceToScreenH={} rightSurfaceToScreenH={} {} {}",
        if config.camera_projection_mode.uses_world_canvas() {
            "full-target-canvas-quad"
        } else {
            "fullscreen-collapsed-surface"
        },
        if config.camera_projection_mode.uses_world_canvas() {
            "surface_to_camera_full_target"
        } else {
            "screen_to_camera"
        },
        "none",
        surface_aspect_contract,
        homographies.left.surface_aspect,
        homographies.right.surface_aspect,
        homographies.left.surface_aspect_source,
        homographies.right.surface_aspect_source,
        homography_token(homographies.left.surface_to_camera),
        homography_token(homographies.right.surface_to_camera),
        homography_token(homographies.left.screen_to_camera),
        homography_token(homographies.right.screen_to_camera),
        homography_token(homographies.left.screen_to_surface),
        homography_token(homographies.right.screen_to_surface),
        homography_token(homographies.left.surface_to_screen),
        homography_token(homographies.right.surface_to_screen),
        expected_source_valid_footprint_marker_fields(homographies),
        projection_area_target_marker_fields(config, target_footprint, homographies),
    )
}

pub(super) fn projected_homography_status_marker_fields(
    applied: Option<&ProjectedStereoHomographies>,
    target: Option<&ProjectedStereoHomographies>,
    config: &crate::RuntimeConfig,
    frame: Option<&StereoGpuCameraFrame>,
) -> String {
    let target_footprint = frame
        .map(|frame| target_footprint_params_from_stereo_frame(frame, config))
        .unwrap_or_else(|| HwbTargetFootprintParams::from_config(config));
    applied
        .or(target)
        .map(|homographies| {
            projected_homography_marker_fields(homographies, config, target_footprint)
        })
        .unwrap_or_else(|| {
            "projectionHomographyReady=false projectionAreaTransformStage=none projectionAreaWarpParity=reference_unwarped_screen_uv".to_string()
        })
}

fn homography_token(rows: [[f32; 3]; 3]) -> String {
    rows.iter()
        .flat_map(|row| row.iter())
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",")
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
    offset_uv: [f32; 2],
    radius_uv: [f32; 2],
    scale_uv: f32,
) -> [f32; 4] {
    let scale = scale_uv.clamp(0.05, 4.0);
    let radius_x = radius_uv[0].clamp(0.05, 0.5);
    let radius_y = radius_uv[1].clamp(0.05, 0.5);
    let center_x = 0.5 + offset_uv[0].clamp(-0.5, 0.5) / scale;
    let center_y = 0.5 + offset_uv[1].clamp(-0.5, 0.5) / scale;
    [
        center_x - radius_x / scale,
        center_y - radius_y / scale,
        (radius_x * 2.0) / scale,
        (radius_y * 2.0) / scale,
    ]
}

fn projection_area_center_uv(offset_uv: [f32; 2], scale_uv: f32) -> [f32; 2] {
    let scale = scale_uv.clamp(0.05, 4.0);
    [
        0.5 + offset_uv[0].clamp(-0.5, 0.5) / scale,
        0.5 + offset_uv[1].clamp(-0.5, 0.5) / scale,
    ]
}

fn projection_area_offset_response_uv(offset_uv: [f32; 2], scale_uv: f32) -> [f32; 2] {
    let scale = scale_uv.clamp(0.05, 4.0);
    [
        offset_uv[0].clamp(-0.5, 0.5) / scale,
        offset_uv[1].clamp(-0.5, 0.5) / scale,
    ]
}

fn projection_area_source_to_screen_gain_uv(radius_uv: [f32; 2], scale_uv: f32) -> [f32; 2] {
    let scale = scale_uv.clamp(0.05, 4.0);
    [
        (radius_uv[0].clamp(0.05, 0.5) * 2.0) / scale,
        (radius_uv[1].clamp(0.05, 0.5) * 2.0) / scale,
    ]
}

fn projection_area_target_marker_fields(
    config: &crate::RuntimeConfig,
    target_footprint: HwbTargetFootprintParams,
    homographies: &ProjectedStereoHomographies,
) -> String {
    let left_offset = [
        target_footprint.area_offset_params[0],
        target_footprint.area_offset_params[1],
    ];
    let right_offset = [
        target_footprint.area_offset_params[2],
        target_footprint.area_offset_params[3],
    ];
    let [radius_x, radius_y, _corner_radius, scale] = target_footprint.area_params;
    let radius = [radius_x, radius_y];
    let source_to_screen_gain = projection_area_source_to_screen_gain_uv(radius, scale);
    let left_feed_rect = projection_area_screen_uv_rect(left_offset, radius, scale);
    let right_feed_rect = projection_area_screen_uv_rect(right_offset, radius, scale);
    let target_source = if target_footprint.from_metadata {
        "source-metadata"
    } else {
        "renderer-authored"
    };
    let resolved_source = if target_footprint.from_metadata {
        "source-metadata"
    } else {
        "renderer-legacy-projection-area"
    };
    let source_sampling_domain = if homographies.left.full_frame_stimulus_mapping
        && homographies.right.full_frame_stimulus_mapping
    {
        "target-local-uv"
    } else {
        "display-eye-screen-uv"
    };
    format!(
        "projectionAreaTargetSource={} projectionAreaTargetStage=target_footprint_mapping projectionAreaTargetCoordinateSpace=display-eye-screen-uv projectionAreaTargetRectSemantics=xywh resolvedTargetFootprintSource={} targetFootprintSourceSamplingDomain={} effectBoundary=target-footprint projectionAreaOffsetConvention=positive-x-right-positive-y-down projectionAreaOffsetResponseCoordinateSpace=display-eye-screen-uv projectionAreaOffsetResponseModel=screen_uv_delta_equals_offset_uv_div_projectionAreaScaleUv projectionAreaShaderScreenBaseFormula=screenBase=(surfaceUv-0.5)*projectionAreaScaleUv+0.5 projectionAreaFullFrameContentFormula=contentUv=(screenBase-offsetUv-(0.5-radiusUv))/(2*radiusUv) projectionAreaSourceToScreenGainUv={} surfaceCoverageSource=renderer-authored surfaceCoverageSemantics=canvas-or-layer-covers-target-fov feedPlacementSource={} feedPlacementSemantics=video_content_inside_target_footprint borderRegionSemantics=visible-render-surface-minus-target-footprint sourceInvalidSemantics=target-fragment-maps-outside-source-valid-uv cameraPipelinePreset={} cameraProjectionEffectMode={} projectionBorderPolicy={} projectionBorderPolicyActive={} projectionBorderShaderBit={} borderFillPolicy={} projectionDepthMeters={:.3} cameraPreviewFovYDegrees={:.3} cameraPreviewOffsetYMeters={:.3} cameraRawOverlayOverscan={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} leftProjectionAreaOffsetUv={} rightProjectionAreaOffsetUv={} leftProjectionAreaOffsetResponseUv={} rightProjectionAreaOffsetResponseUv={} leftProjectionAreaScreenUvRect={} rightProjectionAreaScreenUvRect={} leftFeedPlacementScreenUvRect={} rightFeedPlacementScreenUvRect={} leftProjectionAreaCenterUv={} rightProjectionAreaCenterUv={} {}",
        target_source,
        resolved_source,
        source_sampling_domain,
        screen_uv_vec2_token(source_to_screen_gain),
        target_source,
        config.camera_pipeline_preset.stable_id(),
        config.camera_projection_effect_mode.stable_id(),
        config.camera_projection_border_policy.stable_id(),
        config.camera_projection_border_policy_active(),
        config.camera_projection_border_policy_shader_bit(),
        config
            .camera_projection_border_policy
            .shared_fill_policy_id(),
        config.camera_projection_depth_meters,
        config.camera_preview_fov_y_degrees,
        config.camera_preview_offset_y_meters,
        config.camera_raw_overlay_overscan,
        config.camera_projection_alpha_mode.stable_id(),
        config.camera_projection_alpha_scale,
        config.camera_projection_alpha_bias,
        screen_uv_vec2_token(left_offset),
        screen_uv_vec2_token(right_offset),
        screen_uv_vec2_token(projection_area_offset_response_uv(left_offset, scale)),
        screen_uv_vec2_token(projection_area_offset_response_uv(right_offset, scale)),
        screen_uv_rect_token(left_feed_rect),
        screen_uv_rect_token(right_feed_rect),
        screen_uv_rect_token(left_feed_rect),
        screen_uv_rect_token(right_feed_rect),
        screen_uv_vec2_token(projection_area_center_uv(left_offset, scale)),
        screen_uv_vec2_token(projection_area_center_uv(right_offset, scale)),
        target_footprint_debug_region_marker_fields(),
    )
}

fn expected_source_valid_screen_uv_rect(mapping: &DisplayEyeProjectionMapping) -> [f32; 4] {
    if mapping.full_frame_stimulus_mapping {
        return [0.0, 0.0, 1.0, 1.0];
    }
    source_valid_screen_uv_footprint(
        mapping.screen_to_camera,
        Rect2::UNIT,
        SOURCE_VALID_FOOTPRINT_GRID,
    )
    .bbox_xywh()
}

fn expected_source_valid_footprint_marker_fields(
    homographies: &ProjectedStereoHomographies,
) -> String {
    format!(
        "expectedSourceValidFootprintSource=renderer-authored expectedSourceValidFootprintStage=screen_to_camera_source_uv_bounds expectedSourceValidFootprintCoordinateSpace=display-eye-screen-uv expectedSourceValidFootprintMethod=renderer-grid-sampled-source-uv-validity expectedSourceValidFootprintRectSemantics=xywh projectionGeometrySchema=rusty.xr.video_projection_geometry.v1 projectionMapping=screen-to-source-homography sourceValidUvRect=0.000000,0.000000,1.000000,1.000000 borderRegionSemantics=surface_minus_feed leftExpectedSourceValidScreenUvRect={} rightExpectedSourceValidScreenUvRect={}",
        screen_uv_rect_token(expected_source_valid_screen_uv_rect(&homographies.left)),
        screen_uv_rect_token(expected_source_valid_screen_uv_rect(&homographies.right)),
    )
}
