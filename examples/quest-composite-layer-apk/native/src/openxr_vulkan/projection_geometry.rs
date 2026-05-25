use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::{
    camera_basis_from_camera2_reference_pose_relative_to_center,
    head_anchored_preview_surface_corners, invert_homography, scale_intrinsics_to_image,
    screen_to_camera_uv_homography, source_valid_screen_uv_footprint,
    surface_to_camera_uv_homography, surface_to_eye_screen_uv_homography, CameraBasis, Quat, Rect2,
    TrackingBasis, Vec3,
};

use super::{osc_overlay_eye_projection, project_points_to_eye_clip, StereoHomographyProjection};
use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

const SOURCE_VALID_FOOTPRINT_GRID: usize = 64;

#[derive(Clone, Copy)]
pub(super) struct ProjectedStereoHomographies {
    pub(super) left: DisplayEyeProjectionMapping,
    pub(super) right: DisplayEyeProjectionMapping,
}

#[derive(Clone, Copy)]
pub(super) struct DisplayEyeProjectionMapping {
    pub(super) surface_to_camera: [[f32; 3]; 3],
    pub(super) screen_to_camera: [[f32; 3]; 3],
    pub(super) screen_to_surface: [[f32; 3]; 3],
    pub(super) surface_to_screen: [[f32; 3]; 3],
    pub(super) canvas_clip: [[f32; 4]; 4],
    pub(super) surface_aspect: f32,
    pub(super) surface_aspect_source: &'static str,
    pub(super) full_frame_stimulus_mapping: bool,
}

pub(super) fn projected_homographies_with_screen_to_camera(
    homographies: &ProjectedStereoHomographies,
    applied: StereoHomographyProjection,
) -> ProjectedStereoHomographies {
    ProjectedStereoHomographies {
        left: DisplayEyeProjectionMapping {
            screen_to_camera: applied.left_screen_to_camera,
            ..homographies.left
        },
        right: DisplayEyeProjectionMapping {
            screen_to_camera: applied.right_screen_to_camera,
            ..homographies.right
        },
    }
}

pub(super) fn identity_homography() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

pub(super) fn full_target_canvas_clip() -> [[f32; 4]; 4] {
    [
        [-1.0, -1.0, 0.0, 1.0],
        [1.0, -1.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
        [-1.0, 1.0, 0.0, 1.0],
    ]
}

pub(super) fn full_target_canvas_aspect(
    display_view: &xr::View,
    resolution: vk::Extent2D,
) -> (f32, &'static str) {
    if let Some(aspect) = fov_aspect(display_view.fov) {
        return (aspect.clamp(0.25, 4.0), "display-eye-fov");
    }
    if resolution.height > 0 {
        return (
            (resolution.width as f32 / resolution.height as f32).clamp(0.25, 4.0),
            "swapchain-resolution-fallback",
        );
    }
    (1.0, "square-fallback")
}

pub(super) fn content_surface_aspect(
    width: f32,
    height: f32,
    resolution: vk::Extent2D,
) -> (f32, &'static str) {
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        return ((width / height).clamp(0.25, 4.0), "camera-content-size");
    }
    if resolution.height > 0 {
        return (
            (resolution.width as f32 / resolution.height as f32).clamp(0.25, 4.0),
            "swapchain-resolution-fallback",
        );
    }
    (1.0, "square-fallback")
}

pub(super) fn projected_stereo_homographies(
    frame: &StereoGpuCameraFrame,
    config: &crate::RuntimeConfig,
    controls: &crate::StereoProjectionControls,
    views: &[xr::View],
    resolution: vk::Extent2D,
) -> Option<(DisplayEyeProjectionMapping, DisplayEyeProjectionMapping)> {
    let full_frame_stimulus_mapping = frame_requests_full_frame_stimulus_mapping(&frame.left)
        && frame_requests_full_frame_stimulus_mapping(&frame.right);
    let reference_center = if full_frame_stimulus_mapping {
        Vec3::ZERO
    } else {
        let left_extrinsics = frame.left.metadata.extrinsics?;
        let right_extrinsics = frame.right.metadata.extrinsics?;
        if !left_extrinsics.is_valid() || !right_extrinsics.is_valid() {
            return None;
        }
        (left_extrinsics.world_from_camera.position + right_extrinsics.world_from_camera.position)
            * 0.5
    };
    let left_view = views.first()?;
    let right_view = views.get(1).unwrap_or(left_view);
    let (display_left_source, display_right_source) = match controls.source_eye_mapping {
        crate::StereoSourceEyeMapping::DisplayLeftFromLeftSource => (&frame.left, &frame.right),
        crate::StereoSourceEyeMapping::DisplayLeftFromRightSource => (&frame.right, &frame.left),
    };
    let left = projected_display_eye_homography(
        display_left_source,
        config,
        views,
        left_view,
        0,
        resolution,
        reference_center,
    )?;
    let right = projected_display_eye_homography(
        display_right_source,
        config,
        views,
        right_view,
        1,
        resolution,
        reference_center,
    )?;
    Some((left, right))
}

fn projected_display_eye_homography(
    frame: &HeadsetCameraGpuFrame,
    config: &crate::RuntimeConfig,
    views: &[xr::View],
    display_view: &xr::View,
    display_eye_index: usize,
    resolution: vk::Extent2D,
    reference_center: Vec3,
) -> Option<DisplayEyeProjectionMapping> {
    if frame_requests_full_frame_stimulus_mapping(frame) {
        return projected_full_frame_display_eye_homography(
            frame,
            config,
            views,
            display_view,
            display_eye_index,
            resolution,
        );
    }
    let intrinsics = frame.metadata.intrinsics?;
    let source_domain = frame.metadata.intrinsics_domain?;
    let scaled = scale_intrinsics_to_image(
        intrinsics,
        source_domain.size,
        frame.metadata.delivered_size,
    )
    .ok()?;
    let width = frame.metadata.delivered_size.width as f32;
    let height = frame.metadata.delivered_size.height as f32;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let extrinsics = frame.metadata.extrinsics?;
    if !extrinsics.is_valid() {
        return None;
    }
    let tracking = tracking_basis_from_views(views)?;
    let (aspect, aspect_source) = content_surface_aspect(width, height, resolution);
    // Build the homography over the camera-content surface, not the larger
    // visible full-view surface. The fragment shader expands full-view UVs
    // into content UVs before applying this homography, matching a real
    // head-anchored overlay whose border may extend beyond the camera-covered
    // content region.
    let surface_corners = camera_preview_surface_corners(tracking, config, aspect)?;
    let camera_basis = camera_basis_from_camera2_reference_pose_relative_to_center(
        tracking,
        extrinsics,
        reference_center,
    )
    .ok()?;
    let eye_basis = eye_basis_from_view(display_view)?;
    let surface_to_screen = surface_to_eye_screen_uv_homography(
        surface_corners,
        eye_basis,
        display_view.fov.angle_left.tan(),
        display_view.fov.angle_right.tan(),
        display_view.fov.angle_down.tan(),
        display_view.fov.angle_up.tan(),
    )
    .ok()?;
    let canvas_clip =
        project_points_to_eye_clip(osc_overlay_eye_projection(display_view)?, surface_corners)?;
    let surface_to_camera =
        surface_to_camera_uv_homography(surface_corners, camera_basis, scaled).ok()?;
    // Both public projection modes render through the same fullscreen
    // multiview pass today. Reconstruct the head-anchored content-surface UV
    // from the current display-eye geometry so the shader samples the camera
    // feed as if a real quad had supplied rasterized surface coordinates.
    // The mode remains visible in logs/catalogs so a future mesh-quad backend
    // can be A/B tested without changing launch profiles.
    let [offset_x_uv, offset_y_uv] =
        config.camera_projection_area_offset_for_eye(display_eye_index);
    let screen_to_surface = screen_to_domain_with_visual_offset(
        invert_homography(surface_to_screen)?,
        offset_x_uv,
        offset_y_uv,
    );
    let screen_to_camera = screen_to_domain_with_visual_offset(
        screen_to_camera_uv_homography(surface_to_screen, surface_to_camera).ok()?,
        offset_x_uv,
        offset_y_uv,
    );
    let surface_to_screen =
        domain_to_screen_with_visual_offset(surface_to_screen, offset_x_uv, offset_y_uv);
    let (
        surface_to_camera,
        screen_to_surface,
        surface_to_screen,
        canvas_clip,
        surface_aspect,
        surface_aspect_source,
    ) = if config.camera_projection_mode.uses_world_canvas() {
        let (target_aspect, target_aspect_source) =
            full_target_canvas_aspect(display_view, resolution);
        (
            screen_to_camera,
            identity_homography(),
            identity_homography(),
            full_target_canvas_clip(),
            target_aspect,
            target_aspect_source,
        )
    } else {
        (
            surface_to_camera,
            screen_to_surface,
            surface_to_screen,
            canvas_clip,
            aspect,
            aspect_source,
        )
    };
    Some(DisplayEyeProjectionMapping {
        surface_to_camera,
        screen_to_camera,
        screen_to_surface,
        surface_to_screen,
        canvas_clip,
        surface_aspect,
        surface_aspect_source,
        full_frame_stimulus_mapping: false,
    })
}

fn projected_full_frame_display_eye_homography(
    frame: &HeadsetCameraGpuFrame,
    config: &crate::RuntimeConfig,
    views: &[xr::View],
    display_view: &xr::View,
    display_eye_index: usize,
    resolution: vk::Extent2D,
) -> Option<DisplayEyeProjectionMapping> {
    let tracking = tracking_basis_from_views(views)?;
    let width = frame.metadata.delivered_size.width as f32;
    let height = frame.metadata.delivered_size.height as f32;
    let (aspect, aspect_source) = content_surface_aspect(width, height, resolution);
    let surface_corners = camera_preview_surface_corners(tracking, config, aspect)?;
    let eye_basis = eye_basis_from_view(display_view)?;
    let surface_to_screen = surface_to_eye_screen_uv_homography(
        surface_corners,
        eye_basis,
        display_view.fov.angle_left.tan(),
        display_view.fov.angle_right.tan(),
        display_view.fov.angle_down.tan(),
        display_view.fov.angle_up.tan(),
    )
    .ok()?;
    let canvas_clip =
        project_points_to_eye_clip(osc_overlay_eye_projection(display_view)?, surface_corners)?;
    let [offset_x_uv, offset_y_uv] =
        config.camera_projection_area_offset_for_eye(display_eye_index);
    let screen_to_surface = screen_to_domain_with_visual_offset(
        invert_homography(surface_to_screen)?,
        offset_x_uv,
        offset_y_uv,
    );
    let surface_to_screen =
        domain_to_screen_with_visual_offset(surface_to_screen, offset_x_uv, offset_y_uv);
    let (screen_to_surface, surface_to_screen, canvas_clip, surface_aspect, surface_aspect_source) =
        if config.camera_projection_mode.uses_world_canvas() {
            let (target_aspect, target_aspect_source) =
                full_target_canvas_aspect(display_view, resolution);
            (
                identity_homography(),
                identity_homography(),
                full_target_canvas_clip(),
                target_aspect,
                target_aspect_source,
            )
        } else {
            (
                screen_to_surface,
                surface_to_screen,
                canvas_clip,
                aspect,
                aspect_source,
            )
        };
    Some(DisplayEyeProjectionMapping {
        surface_to_camera: identity_homography(),
        screen_to_camera: screen_to_surface,
        screen_to_surface,
        surface_to_screen,
        canvas_clip,
        surface_aspect,
        surface_aspect_source,
        full_frame_stimulus_mapping: true,
    })
}

fn frame_requests_full_frame_stimulus_mapping(frame: &HeadsetCameraGpuFrame) -> bool {
    let full_frame_profile = frame
        .diagnostics
        .synthetic_projection_profile
        .as_deref()
        .is_some_and(|value| value == "full-frame-diagnostic")
        || frame
            .diagnostics
            .projection_geometry_profile
            .as_deref()
            .is_some_and(|value| value == "full-frame-diagnostic");
    if !full_frame_profile {
        return false;
    }
    let Some(mapping_intent) = frame.diagnostics.content_mapping_intent.as_deref() else {
        return false;
    };
    matches!(
        mapping_intent,
        "map-full-frame-stimulus-to-projection-area"
            | "map-full-frame-stimulus-to-projection-surface"
            | "map-full-frame-content-to-projection-area"
            | "map-full-frame-content-to-projection-surface"
    )
}

pub(super) fn camera_preview_surface_corners(
    tracking: TrackingBasis,
    config: &crate::RuntimeConfig,
    aspect: f32,
) -> Option<[Vec3; 4]> {
    let mut surface_corners = head_anchored_preview_surface_corners(
        tracking,
        config.camera_preview_fov_y_degrees,
        config.camera_projection_depth_meters.max(0.05),
        aspect,
        config.camera_raw_overlay_overscan,
    )
    .ok()?;
    let offset = tracking.up * config.camera_preview_offset_y_meters.clamp(-2.0, 2.0);
    for corner in &mut surface_corners {
        *corner = *corner + offset;
    }
    Some(surface_corners)
}

pub(super) fn eye_basis_from_view(view: &xr::View) -> Option<CameraBasis> {
    let orientation = Quat::new(
        view.pose.orientation.x,
        view.pose.orientation.y,
        view.pose.orientation.z,
        view.pose.orientation.w,
    )
    .normalized_or(Quat::IDENTITY);
    CameraBasis::new(
        Vec3::new(
            view.pose.position.x,
            view.pose.position.y,
            view.pose.position.z,
        ),
        orientation.rotate_vec3(Vec3::RIGHT),
        orientation.rotate_vec3(Vec3::UP),
        orientation.rotate_vec3(Vec3::FORWARD_NEG_Z),
    )
}

pub(super) fn tracking_basis_from_views(views: &[xr::View]) -> Option<TrackingBasis> {
    let first = views.first()?;
    let position = if views.len() >= 2 {
        let left = views[0].pose.position;
        let right = views[1].pose.position;
        Vec3::new(
            (left.x + right.x) * 0.5,
            (left.y + right.y) * 0.5,
            (left.z + right.z) * 0.5,
        )
    } else {
        Vec3::new(
            first.pose.position.x,
            first.pose.position.y,
            first.pose.position.z,
        )
    };
    let orientation = Quat::new(
        first.pose.orientation.x,
        first.pose.orientation.y,
        first.pose.orientation.z,
        first.pose.orientation.w,
    )
    .normalized_or(Quat::IDENTITY);
    TrackingBasis::new(
        position,
        orientation.rotate_vec3(Vec3::RIGHT),
        orientation.rotate_vec3(Vec3::UP),
        orientation.rotate_vec3(Vec3::FORWARD_NEG_Z),
    )
}

pub(super) fn pack_homography_row(row: [f32; 3]) -> [f32; 4] {
    [row[0], row[1], row[2], 0.0]
}

pub(super) fn screen_to_domain_with_visual_offset(
    mut rows: [[f32; 3]; 3],
    offset_x_uv: f32,
    offset_y_uv: f32,
) -> [[f32; 3]; 3] {
    let input_x_offset = -offset_x_uv.clamp(-0.5, 0.5);
    let input_y_offset = -offset_y_uv.clamp(-0.5, 0.5);
    for row in &mut rows {
        row[2] += row[0] * input_x_offset + row[1] * input_y_offset;
    }
    rows
}

pub(super) fn domain_to_screen_with_visual_offset(
    mut rows: [[f32; 3]; 3],
    offset_x_uv: f32,
    offset_y_uv: f32,
) -> [[f32; 3]; 3] {
    let output_x_offset = offset_x_uv.clamp(-0.5, 0.5);
    let output_y_offset = offset_y_uv.clamp(-0.5, 0.5);
    let projective_row = rows[2];
    for (column, projective_value) in projective_row.into_iter().enumerate() {
        rows[0][column] += projective_value * output_x_offset;
        rows[1][column] += projective_value * output_y_offset;
    }
    rows
}

pub(super) fn projected_homography_marker_fields(
    homographies: &ProjectedStereoHomographies,
    config: &crate::RuntimeConfig,
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
        projection_area_target_marker_fields(config),
    )
}

pub(super) fn projected_homography_status_marker_fields(
    applied: Option<&ProjectedStereoHomographies>,
    target: Option<&ProjectedStereoHomographies>,
    config: &crate::RuntimeConfig,
) -> String {
    applied
        .or(target)
        .map(|homographies| projected_homography_marker_fields(homographies, config))
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

pub(super) fn fov_aspect(fov: xr::Fovf) -> Option<f32> {
    let width = fov.angle_right.tan() - fov.angle_left.tan();
    let height = fov.angle_up.tan() - fov.angle_down.tan();
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        Some(width / height)
    } else {
        None
    }
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

fn projection_area_target_marker_fields(config: &crate::RuntimeConfig) -> String {
    let left_offset = config.camera_projection_area_offset_for_eye(0);
    let right_offset = config.camera_projection_area_offset_for_eye(1);
    let [radius_x, radius_y, _corner_radius, scale] = config.camera_area_params_push();
    let radius = [radius_x, radius_y];
    let source_to_screen_gain = projection_area_source_to_screen_gain_uv(radius, scale);
    let left_feed_rect = projection_area_screen_uv_rect(left_offset, radius, scale);
    let right_feed_rect = projection_area_screen_uv_rect(right_offset, radius, scale);
    format!(
        "projectionAreaTargetSource=renderer-authored projectionAreaTargetStage=projection_area_mapping projectionAreaTargetCoordinateSpace=display-eye-screen-uv projectionAreaTargetRectSemantics=xywh projectionAreaOffsetConvention=positive-x-right-positive-y-down projectionAreaOffsetResponseCoordinateSpace=display-eye-screen-uv projectionAreaOffsetResponseModel=screen_uv_delta_equals_offset_uv_div_projectionAreaScaleUv projectionAreaShaderScreenBaseFormula=screenBase=(surfaceUv-0.5)*projectionAreaScaleUv+0.5 projectionAreaFullFrameContentFormula=contentUv=(screenBase-offsetUv-(0.5-radiusUv))/(2*radiusUv) projectionAreaSourceToScreenGainUv={} surfaceCoverageSource=renderer-authored surfaceCoverageSemantics=canvas-or-layer-covers-target-fov feedPlacementSource=renderer-authored feedPlacementSemantics=video_content_inside_surface borderRegionSemantics=surface_minus_feed cameraPipelinePreset={} cameraProjectionEffectMode={} projectionBorderPolicy={} projectionBorderPolicyActive={} projectionBorderShaderBit={} borderFillPolicy={} projectionDepthMeters={:.3} cameraPreviewFovYDegrees={:.3} cameraPreviewOffsetYMeters={:.3} cameraRawOverlayOverscan={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} leftProjectionAreaOffsetUv={} rightProjectionAreaOffsetUv={} leftProjectionAreaOffsetResponseUv={} rightProjectionAreaOffsetResponseUv={} leftProjectionAreaScreenUvRect={} rightProjectionAreaScreenUvRect={} leftFeedPlacementScreenUvRect={} rightFeedPlacementScreenUvRect={} leftProjectionAreaCenterUv={} rightProjectionAreaCenterUv={}",
        screen_uv_vec2_token(source_to_screen_gain),
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

pub(super) fn projection_openxr_contract_fields(
    openxr_reference_space: &str,
    predicted_display_time: xr::Time,
    views: &[xr::View],
) -> String {
    let Some(left) = views.first() else {
        return format!(
            "referenceSpace=app-reference-space openxrReferenceSpace={} displayTimeSource=not-logged predictedDisplayTimeSource=not-logged predictedDisplayTimeNs=not-logged viewPoseFovSource=not-logged",
            marker_token(Some(openxr_reference_space), "unknown")
        );
    };
    let right = views.get(1).unwrap_or(left);
    format!(
        "referenceSpace=app-reference-space openxrReferenceSpace={} displayTimeSource=predicted-display-time predictedDisplayTimeSource=predicted-display-time predictedDisplayTimeNs={} viewPoseFovSource=xrLocateViews leftRenderFovTangents={} rightRenderFovTangents={} leftRenderPosition={} rightRenderPosition={} leftRenderOrientation={} rightRenderOrientation={}",
        marker_token(Some(openxr_reference_space), "unknown"),
        predicted_display_time.as_nanos(),
        format_vec4(fov_tangents(left.fov)),
        format_vec4(fov_tangents(right.fov)),
        format_vec4(pose_position(left.pose)),
        format_vec4(pose_position(right.pose)),
        format_vec4(pose_orientation(left.pose)),
        format_vec4(pose_orientation(right.pose))
    )
}

pub(super) fn projection_openxr_contract_log_message(
    frame_index: u64,
    openxr_frame_count: u64,
    aligned_projection: bool,
    openxr_reference_space: &str,
    predicted_display_time: xr::Time,
    views: &[xr::View],
) -> String {
    format!(
        "Rusty XR OpenXR projection contract frame={} openXrFrameCount={} activeTier=gpu-projected alignedProjection={} {}",
        frame_index,
        openxr_frame_count,
        aligned_projection,
        projection_openxr_contract_fields(openxr_reference_space, predicted_display_time, views)
    )
}

pub(super) fn display_eye_uv_fiducial_marker_fields(config: &crate::RuntimeConfig) -> &'static str {
    use crate::camera_color_pipeline::CameraProjectionEffectMode;
    match config.camera_projection_effect_mode {
        CameraProjectionEffectMode::DisplayEyeUvFiducial => "displayEyeUvFiducialActive=true displayEyeUvFiducialSchema=rusty.xr.display_eye_uv_fiducial.v1 displayEyeUvFiducialCoordinateSpace=display-eye-screen-uv displayEyeUvFiducialUvBasis=projection_screen_uv_base displayEyeUvFiducialShaderFormula=displayEyeUv=(surfaceUv-0.5)*projectionAreaScaleUv+0.5 displayEyeUvFiducialMarkersUv=cyan_upper_left@0.250000,0.250000;red_left_mid@0.250000,0.500000;yellow_top_mid@0.500000,0.250000;green_center@0.500000,0.500000;magenta_bottom_mid@0.500000,0.750000;blue_right_mid@0.750000,0.500000",
        CameraProjectionEffectMode::ProjectionContentUvFiducial => "displayEyeUvFiducialActive=true displayEyeUvFiducialSchema=rusty.xr.display_eye_uv_fiducial.v1 displayEyeUvFiducialCoordinateSpace=projection-content-uv displayEyeUvFiducialUvBasis=full_frame_content_uv displayEyeUvFiducialShaderFormula=contentUv=(projectionScreenUv-(0.5-radiusUv))/(2*radiusUv);projectionScreenUv=(surfaceUv-0.5)*projectionAreaScaleUv+0.5-offsetUv displayEyeUvFiducialMarkersUv=cyan_upper_left@0.250000,0.250000;red_left_mid@0.250000,0.500000;yellow_top_mid@0.500000,0.250000;green_center@0.500000,0.500000;magenta_bottom_mid@0.500000,0.750000;blue_right_mid@0.750000,0.500000",
        CameraProjectionEffectMode::SourceSamplingWitness => "displayEyeUvFiducialActive=true displayEyeUvFiducialSchema=rusty.xr.source_sampling_witness.v1 displayEyeUvFiducialCoordinateSpace=source-sampling-witness displayEyeUvFiducialUvBasis=actual-source-image+full_frame_content_uv+hardware-buffer-sampler-uv displayEyeUvFiducialShaderFormula=contentUv=(projectionScreenUv-(0.5-radiusUv))/(2*radiusUv);sourceSamplerUv=cameraTextureTransform(sourceVisibleUvRect(contentUv)) displayEyeUvFiducialMarkersUv=content_grid_yellow_white@0.125,0.250,0.500;source_sampler_grid_cyan_magenta@0.125,0.250,0.500",
        _ => "displayEyeUvFiducialActive=false",
    }
}

pub(super) fn display_eye_uv_fiducial_contract_log_message(
    frame_index: u64,
    openxr_frame_count: u64,
    marker_fields: &str,
) -> String {
    format!(
        "Rusty XR display-eye UV fiducial contract frame={} openXrFrameCount={} {}",
        frame_index, openxr_frame_count, marker_fields
    )
}

fn marker_token(value: Option<&str>, fallback: &str) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .replace(char::is_whitespace, "_")
}

fn format_vec4(values: [f32; 4]) -> String {
    format!(
        "[{:.6},{:.6},{:.6},{:.6}]",
        values[0], values[1], values[2], values[3]
    )
}

fn fov_tangents(fov: xr::sys::Fovf) -> [f32; 4] {
    [
        fov.angle_left.tan(),
        fov.angle_right.tan(),
        fov.angle_up.tan(),
        fov.angle_down.tan(),
    ]
}

fn pose_position(pose: xr::sys::Posef) -> [f32; 4] {
    [pose.position.x, pose.position.y, pose.position.z, 1.0]
}

fn pose_orientation(pose: xr::sys::Posef) -> [f32; 4] {
    [
        pose.orientation.x,
        pose.orientation.y,
        pose.orientation.z,
        pose.orientation.w,
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        display_eye_uv_fiducial_contract_log_message, display_eye_uv_fiducial_marker_fields,
        projected_homography_status_marker_fields,
    };
    use crate::{camera_color_pipeline::CameraProjectionEffectMode, RuntimeConfig};

    #[test]
    fn display_eye_uv_fiducial_marker_fields_keep_contract_shape() {
        let mut config = RuntimeConfig::default();

        config.camera_projection_effect_mode = CameraProjectionEffectMode::DisplayEyeUvFiducial;
        let display_eye = display_eye_uv_fiducial_marker_fields(&config);
        assert!(display_eye.contains("displayEyeUvFiducialActive=true"));
        assert!(display_eye.contains("displayEyeUvFiducialCoordinateSpace=display-eye-screen-uv"));
        assert!(display_eye.contains("displayEyeUvFiducialShaderFormula=displayEyeUv="));

        config.camera_projection_effect_mode = CameraProjectionEffectMode::SourceSamplingWitness;
        let source_sampling = display_eye_uv_fiducial_marker_fields(&config);
        assert!(source_sampling.contains("schema=rusty.xr.source_sampling_witness.v1"));
        assert!(
            source_sampling.contains("displayEyeUvFiducialCoordinateSpace=source-sampling-witness")
        );

        config.camera_projection_effect_mode = CameraProjectionEffectMode::BorderComposite;
        assert_eq!(
            display_eye_uv_fiducial_marker_fields(&config),
            "displayEyeUvFiducialActive=false"
        );
    }

    #[test]
    fn display_eye_uv_fiducial_contract_log_message_keeps_prefix_shape() {
        assert_eq!(
            display_eye_uv_fiducial_contract_log_message(
                7,
                42,
                "displayEyeUvFiducialActive=true"
            ),
            "Rusty XR display-eye UV fiducial contract frame=7 openXrFrameCount=42 displayEyeUvFiducialActive=true"
        );
    }

    #[test]
    fn projected_homography_status_marker_fields_keeps_missing_shape() {
        assert_eq!(
            projected_homography_status_marker_fields(None, None, &RuntimeConfig::default()),
            "projectionHomographyReady=false projectionAreaTransformStage=none projectionAreaWarpParity=reference_unwarped_screen_uv"
        );
    }
}
