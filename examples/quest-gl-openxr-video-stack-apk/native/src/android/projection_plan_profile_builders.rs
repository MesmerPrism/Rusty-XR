use openxr as xr;
use rusty_xr_camera_model::{
    camera_basis_from_camera2_reference_pose_relative_to_center,
    head_anchored_preview_surface_corners, invert_homography, screen_to_camera_uv_homography,
    surface_to_camera_uv_homography, surface_to_eye_screen_uv_homography, CameraBasis,
    CameraIntrinsics, FeedPlacementDescriptor, ImageSize, PerEyeVideoProjectionPlan, Quat, Rect2,
    TrackingBasis, Vec2, Vec3, VideoProjectionMapping,
};
use rusty_xr_contracts::Eye;

use super::{
    openxr_gles_config::{OesContentMappingMode, OesProjectionBorderPolicy},
    projection_contract_markers::projection_area_screen_uv_rect,
    projection_geometry::{identity_homography, OesEyeProjection, OesProjectionPlan},
    source_metadata::{
        projection_source_label, projection_surface_aspect_from_metadata, OesProjectionMetadata,
    },
};

const PROJECTION_FOOTPRINT_GRID: usize = 64;

fn array_rect_xywh(rect: [f32; 4]) -> Rect2 {
    Rect2::new(Vec2::new(rect[0], rect[1]), Vec2::new(rect[2], rect[3]))
}

fn shared_projection_mapping(mode: OesContentMappingMode) -> VideoProjectionMapping {
    match mode {
        OesContentMappingMode::CameraProjection => VideoProjectionMapping::ScreenToSourceHomography,
        OesContentMappingMode::FullFrameStimulusToProjectionArea => {
            VideoProjectionMapping::FullFrameSurface
        }
        OesContentMappingMode::FullFrameStimulusToSurfaceHomography => {
            VideoProjectionMapping::SurfaceToSourceHomography
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn shared_per_eye_projection_plan(
    eye: Eye,
    content_mapping_mode: OesContentMappingMode,
    surface_to_screen_h: [[f32; 3]; 3],
    screen_to_surface_h: [[f32; 3]; 3],
    surface_to_camera_h: [[f32; 3]; 3],
    screen_to_camera_h: [[f32; 3]; 3],
    projection_area_offset_uv: [f32; 2],
    projection_area_scale: [f32; 2],
    projection_area_radius: [f32; 2],
    projection_area_opacity: f32,
    projection_border_policy: OesProjectionBorderPolicy,
    projection_border_opacity: f32,
    source_valid_uv_rect: Rect2,
) -> Option<PerEyeVideoProjectionPlan> {
    let feed_rect = array_rect_xywh(projection_area_screen_uv_rect(
        projection_area_offset_uv,
        projection_area_radius,
        projection_area_scale,
    ));
    let feed = FeedPlacementDescriptor::new(
        Rect2::UNIT,
        feed_rect,
        projection_area_opacity.clamp(0.0, 1.0),
    );
    PerEyeVideoProjectionPlan::from_homographies(
        eye,
        shared_projection_mapping(content_mapping_mode),
        surface_to_screen_h,
        screen_to_surface_h,
        surface_to_camera_h,
        screen_to_camera_h,
        feed,
        source_valid_uv_rect,
        projection_border_policy.shared_descriptor(projection_border_opacity),
        PROJECTION_FOOTPRINT_GRID,
    )
}

fn preview_surface_corners(
    tracking: TrackingBasis,
    preview_fov_y_degrees: f32,
    projection_depth_meters: f32,
    aspect: f32,
    raw_overscan: f32,
    preview_offset_y_meters: f32,
) -> Option<[Vec3; 4]> {
    let mut surface = head_anchored_preview_surface_corners(
        tracking,
        preview_fov_y_degrees,
        projection_depth_meters,
        aspect,
        raw_overscan,
    )
    .ok()?;
    let offset = tracking.up * preview_offset_y_meters.clamp(-2.0, 2.0);
    for corner in &mut surface {
        *corner += offset;
    }
    Some(surface)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn broker_synthetic_projection_plan_from_xr_views(
    left_metadata: &OesProjectionMetadata,
    right_metadata: &OesProjectionMetadata,
    width: u32,
    height: u32,
    views: &[xr::View],
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
) -> Option<OesProjectionPlan> {
    let left_view = views.first()?;
    let right_view = views.get(1)?;
    let tracking = tracking_basis_from_xr_views(left_view, right_view)?;
    let aspect =
        projection_surface_aspect_from_metadata(left_metadata, right_metadata, width, height);
    let surface = preview_surface_corners(
        tracking,
        projection_preview_fov_y_degrees,
        projection_depth_meters,
        aspect,
        projection_raw_overscan,
        projection_preview_offset_y_meters,
    )?;
    let intrinsics = synthetic_broker_intrinsics(width, height, projection_preview_fov_y_degrees)?;
    let camera_basis = CameraBasis::new(
        tracking.origin,
        tracking.right,
        tracking.up,
        tracking.forward,
    )?;
    let surface_to_camera =
        surface_to_camera_uv_homography(surface, camera_basis, intrinsics).ok()?;
    let left_eye_basis = eye_basis_from_xr_view(left_view)?;
    let right_eye_basis = eye_basis_from_xr_view(right_view)?;
    let left_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        left_eye_basis,
        left_view.fov.angle_left.tan(),
        left_view.fov.angle_right.tan(),
        left_view.fov.angle_down.tan(),
        left_view.fov.angle_up.tan(),
    )
    .ok()?;
    let right_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        right_eye_basis,
        right_view.fov.angle_left.tan(),
        right_view.fov.angle_right.tan(),
        right_view.fov.angle_down.tan(),
        right_view.fov.angle_up.tan(),
    )
    .ok()?;
    let left_screen_to_surface_h = screen_to_domain_with_visual_adjustment(
        invert_homography(left_surface_to_screen)?,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
    );
    let right_screen_to_surface_h = screen_to_domain_with_visual_adjustment(
        invert_homography(right_surface_to_screen)?,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
    );
    let left_screen_to_camera_h = screen_to_domain_with_visual_adjustment(
        screen_to_camera_uv_homography(left_surface_to_screen, surface_to_camera).ok()?,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
    );
    let right_screen_to_camera_h = screen_to_domain_with_visual_adjustment(
        screen_to_camera_uv_homography(right_surface_to_screen, surface_to_camera).ok()?,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
    );
    let left_use_surface_texture_transform =
        use_surface_texture_transform_for_stimulus(left_metadata);
    let right_use_surface_texture_transform =
        use_surface_texture_transform_for_stimulus(right_metadata);
    let left_source_label = projection_source_label(
        left_metadata,
        width,
        height,
        left_use_surface_texture_transform,
    );
    let right_source_label = projection_source_label(
        right_metadata,
        width,
        height,
        right_use_surface_texture_transform,
    );
    let content_mapping_mode = OesContentMappingMode::CameraProjection;
    let left_geometry_plan = shared_per_eye_projection_plan(
        Eye::Left,
        content_mapping_mode,
        left_surface_to_screen,
        left_screen_to_surface_h,
        surface_to_camera,
        left_screen_to_camera_h,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
        projection_area_radius,
        projection_area_opacity,
        projection_border_policy,
        projection_border_opacity,
        left_metadata.source_valid_uv_rect,
    )?;
    let right_geometry_plan = shared_per_eye_projection_plan(
        Eye::Right,
        content_mapping_mode,
        right_surface_to_screen,
        right_screen_to_surface_h,
        surface_to_camera,
        right_screen_to_camera_h,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
        projection_area_radius,
        projection_area_opacity,
        projection_border_policy,
        projection_border_opacity,
        right_metadata.source_valid_uv_rect,
    )?;

    Some(OesProjectionPlan {
        left: OesEyeProjection {
            eye: Eye::Left,
            surface_to_screen_h: left_surface_to_screen,
            screen_to_surface_h: left_screen_to_surface_h,
            surface_to_camera_h: surface_to_camera,
            screen_to_camera_h: left_screen_to_camera_h,
            source_label: left_source_label,
            source_eye: "left".to_string(),
            use_surface_texture_transform: left_use_surface_texture_transform,
            content_mapping_mode,
            geometry_plan: left_geometry_plan,
        },
        right: OesEyeProjection {
            eye: Eye::Right,
            surface_to_screen_h: right_surface_to_screen,
            screen_to_surface_h: right_screen_to_surface_h,
            surface_to_camera_h: surface_to_camera,
            screen_to_camera_h: right_screen_to_camera_h,
            source_label: right_source_label,
            source_eye: "right".to_string(),
            use_surface_texture_transform: right_use_surface_texture_transform,
            content_mapping_mode,
            geometry_plan: right_geometry_plan,
        },
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn broker_full_frame_projection_plan_from_xr_views(
    left_metadata: &OesProjectionMetadata,
    right_metadata: &OesProjectionMetadata,
    width: u32,
    height: u32,
    views: &[xr::View],
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
) -> Option<OesProjectionPlan> {
    let left_view = views.first()?;
    let right_view = views.get(1)?;
    let tracking = tracking_basis_from_xr_views(left_view, right_view)?;
    let aspect =
        projection_surface_aspect_from_metadata(left_metadata, right_metadata, width, height);
    let surface = preview_surface_corners(
        tracking,
        projection_preview_fov_y_degrees,
        projection_depth_meters,
        aspect,
        projection_raw_overscan,
        projection_preview_offset_y_meters,
    )?;
    let left_eye_basis = eye_basis_from_xr_view(left_view)?;
    let right_eye_basis = eye_basis_from_xr_view(right_view)?;
    let left_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        left_eye_basis,
        left_view.fov.angle_left.tan(),
        left_view.fov.angle_right.tan(),
        left_view.fov.angle_down.tan(),
        left_view.fov.angle_up.tan(),
    )
    .ok()?;
    let right_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        right_eye_basis,
        right_view.fov.angle_left.tan(),
        right_view.fov.angle_right.tan(),
        right_view.fov.angle_down.tan(),
        right_view.fov.angle_up.tan(),
    )
    .ok()?;
    let left_screen_to_surface_h = screen_to_domain_with_visual_adjustment(
        invert_homography(left_surface_to_screen)?,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
    );
    let right_screen_to_surface_h = screen_to_domain_with_visual_adjustment(
        invert_homography(right_surface_to_screen)?,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
    );
    let identity = identity_homography();
    let left_use_surface_texture_transform =
        use_surface_texture_transform_for_stimulus(left_metadata);
    let right_use_surface_texture_transform =
        use_surface_texture_transform_for_stimulus(right_metadata);
    let left_source_label = projection_source_label(
        left_metadata,
        width,
        height,
        left_use_surface_texture_transform,
    );
    let right_source_label = projection_source_label(
        right_metadata,
        width,
        height,
        right_use_surface_texture_transform,
    );
    let content_mapping_mode = OesContentMappingMode::FullFrameStimulusToSurfaceHomography;
    let left_geometry_plan = shared_per_eye_projection_plan(
        Eye::Left,
        content_mapping_mode,
        left_surface_to_screen,
        left_screen_to_surface_h,
        identity,
        left_screen_to_surface_h,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
        projection_area_radius,
        projection_area_opacity,
        projection_border_policy,
        projection_border_opacity,
        left_metadata.source_valid_uv_rect,
    )?;
    let right_geometry_plan = shared_per_eye_projection_plan(
        Eye::Right,
        content_mapping_mode,
        right_surface_to_screen,
        right_screen_to_surface_h,
        identity,
        right_screen_to_surface_h,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
        projection_area_radius,
        projection_area_opacity,
        projection_border_policy,
        projection_border_opacity,
        right_metadata.source_valid_uv_rect,
    )?;

    Some(OesProjectionPlan {
        left: OesEyeProjection {
            eye: Eye::Left,
            surface_to_screen_h: left_surface_to_screen,
            screen_to_surface_h: left_screen_to_surface_h,
            surface_to_camera_h: identity,
            screen_to_camera_h: left_screen_to_surface_h,
            source_label: left_source_label,
            source_eye: "left".to_string(),
            use_surface_texture_transform: left_use_surface_texture_transform,
            content_mapping_mode,
            geometry_plan: left_geometry_plan,
        },
        right: OesEyeProjection {
            eye: Eye::Right,
            surface_to_screen_h: right_surface_to_screen,
            screen_to_surface_h: right_screen_to_surface_h,
            surface_to_camera_h: identity,
            screen_to_camera_h: right_screen_to_surface_h,
            source_label: right_source_label,
            source_eye: "right".to_string(),
            use_surface_texture_transform: right_use_surface_texture_transform,
            content_mapping_mode,
            geometry_plan: right_geometry_plan,
        },
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn camera2_projection_plan_from_xr_views(
    left_metadata: &OesProjectionMetadata,
    right_metadata: &OesProjectionMetadata,
    width: u32,
    height: u32,
    views: &[xr::View],
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
) -> Option<OesProjectionPlan> {
    let left_view = views.first()?;
    let right_view = views.get(1)?;
    let tracking = tracking_basis_from_xr_views(left_view, right_view)?;
    let aspect =
        projection_surface_aspect_from_metadata(left_metadata, right_metadata, width, height);
    let surface = preview_surface_corners(
        tracking,
        projection_preview_fov_y_degrees,
        projection_depth_meters,
        aspect,
        projection_raw_overscan,
        projection_preview_offset_y_meters,
    )?;
    let left_extrinsics = left_metadata.extrinsics?;
    let right_extrinsics = right_metadata.extrinsics?;
    let reference_center = (left_extrinsics.world_from_camera.position
        + right_extrinsics.world_from_camera.position)
        * 0.5;
    let left_basis = camera_basis_from_camera2_reference_pose_relative_to_center(
        tracking,
        left_extrinsics,
        reference_center,
    )
    .ok()?;
    let right_basis = camera_basis_from_camera2_reference_pose_relative_to_center(
        tracking,
        right_extrinsics,
        reference_center,
    )
    .ok()?;
    let left_intrinsics = left_metadata.intrinsics?;
    let right_intrinsics = right_metadata.intrinsics?;
    let left_surface_to_camera =
        surface_to_camera_uv_homography(surface, left_basis, left_intrinsics).ok()?;
    let right_surface_to_camera =
        surface_to_camera_uv_homography(surface, right_basis, right_intrinsics).ok()?;
    let left_eye_basis = eye_basis_from_xr_view(left_view)?;
    let right_eye_basis = eye_basis_from_xr_view(right_view)?;
    let left_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        left_eye_basis,
        left_view.fov.angle_left.tan(),
        left_view.fov.angle_right.tan(),
        left_view.fov.angle_down.tan(),
        left_view.fov.angle_up.tan(),
    )
    .ok()?;
    let right_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        right_eye_basis,
        right_view.fov.angle_left.tan(),
        right_view.fov.angle_right.tan(),
        right_view.fov.angle_down.tan(),
        right_view.fov.angle_up.tan(),
    )
    .ok()?;
    let left_screen_to_surface_h = screen_to_domain_with_visual_adjustment(
        invert_homography(left_surface_to_screen)?,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
    );
    let right_screen_to_surface_h = screen_to_domain_with_visual_adjustment(
        invert_homography(right_surface_to_screen)?,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
    );
    let left_screen_to_camera_h = screen_to_domain_with_visual_adjustment(
        screen_to_camera_uv_homography(left_surface_to_screen, left_surface_to_camera).ok()?,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
    );
    let right_screen_to_camera_h = screen_to_domain_with_visual_adjustment(
        screen_to_camera_uv_homography(right_surface_to_screen, right_surface_to_camera).ok()?,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
    );
    let left_use_surface_texture_transform =
        use_surface_texture_transform_for_stimulus(left_metadata);
    let right_use_surface_texture_transform =
        use_surface_texture_transform_for_stimulus(right_metadata);
    let left_source_label = projection_source_label(
        left_metadata,
        width,
        height,
        left_use_surface_texture_transform,
    );
    let right_source_label = projection_source_label(
        right_metadata,
        width,
        height,
        right_use_surface_texture_transform,
    );
    let content_mapping_mode = OesContentMappingMode::CameraProjection;
    let left_geometry_plan = shared_per_eye_projection_plan(
        Eye::Left,
        content_mapping_mode,
        left_surface_to_screen,
        left_screen_to_surface_h,
        left_surface_to_camera,
        left_screen_to_camera_h,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
        projection_area_radius,
        projection_area_opacity,
        projection_border_policy,
        projection_border_opacity,
        left_metadata.source_valid_uv_rect,
    )?;
    let right_geometry_plan = shared_per_eye_projection_plan(
        Eye::Right,
        content_mapping_mode,
        right_surface_to_screen,
        right_screen_to_surface_h,
        right_surface_to_camera,
        right_screen_to_camera_h,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
        projection_area_radius,
        projection_area_opacity,
        projection_border_policy,
        projection_border_opacity,
        right_metadata.source_valid_uv_rect,
    )?;

    Some(OesProjectionPlan {
        left: OesEyeProjection {
            eye: Eye::Left,
            surface_to_screen_h: left_surface_to_screen,
            screen_to_surface_h: left_screen_to_surface_h,
            surface_to_camera_h: left_surface_to_camera,
            screen_to_camera_h: left_screen_to_camera_h,
            source_label: left_source_label,
            source_eye: "left".to_string(),
            use_surface_texture_transform: left_use_surface_texture_transform,
            content_mapping_mode,
            geometry_plan: left_geometry_plan,
        },
        right: OesEyeProjection {
            eye: Eye::Right,
            surface_to_screen_h: right_surface_to_screen,
            screen_to_surface_h: right_screen_to_surface_h,
            surface_to_camera_h: right_surface_to_camera,
            screen_to_camera_h: right_screen_to_camera_h,
            source_label: right_source_label,
            source_eye: "right".to_string(),
            use_surface_texture_transform: right_use_surface_texture_transform,
            content_mapping_mode,
            geometry_plan: right_geometry_plan,
        },
    })
}

fn synthetic_broker_intrinsics(
    width: u32,
    height: u32,
    preview_fov_y_degrees: f32,
) -> Option<CameraIntrinsics> {
    let width_f = width as f32;
    let height_f = height as f32;
    if width_f <= 0.0 || height_f <= 0.0 {
        return None;
    }
    let focal = height_f / (2.0 * (preview_fov_y_degrees.to_radians() * 0.5).tan());
    let intrinsics = CameraIntrinsics::new(
        Vec2::new(focal, focal),
        Vec2::new(width_f * 0.5, height_f * 0.5),
        ImageSize::new(width, height),
    );
    intrinsics.is_valid().then_some(intrinsics)
}

fn eye_basis_from_xr_view(view: &xr::View) -> Option<CameraBasis> {
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

fn tracking_basis_from_xr_views(left: &xr::View, right: &xr::View) -> Option<TrackingBasis> {
    let position = Vec3::new(
        (left.pose.position.x + right.pose.position.x) * 0.5,
        (left.pose.position.y + right.pose.position.y) * 0.5,
        (left.pose.position.z + right.pose.position.z) * 0.5,
    );
    let orientation = Quat::new(
        left.pose.orientation.x,
        left.pose.orientation.y,
        left.pose.orientation.z,
        left.pose.orientation.w,
    )
    .normalized_or(Quat::IDENTITY);
    TrackingBasis::new(
        position,
        orientation.rotate_vec3(Vec3::RIGHT),
        orientation.rotate_vec3(Vec3::UP),
        orientation.rotate_vec3(Vec3::FORWARD_NEG_Z),
    )
}

fn use_surface_texture_transform_for_stimulus(metadata: &OesProjectionMetadata) -> bool {
    !metadata.has_explicit_top_left_stimulus_orientation()
}

fn screen_to_domain_with_visual_adjustment(
    mut rows: [[f32; 3]; 3],
    offset_uv: [f32; 2],
    scale_uv: [f32; 2],
) -> [[f32; 3]; 3] {
    let scale_x = scale_uv[0].clamp(0.05, 4.0);
    let scale_y = scale_uv[1].clamp(0.05, 4.0);
    let input_x_offset = 0.5 - 0.5 * scale_x - offset_uv[0].clamp(-0.5, 0.5);
    let input_y_offset = 0.5 - 0.5 * scale_y - offset_uv[1].clamp(-0.5, 0.5);
    for row in &mut rows {
        row[2] += row[0] * input_x_offset + row[1] * input_y_offset;
        row[0] *= scale_x;
        row[1] *= scale_y;
    }
    rows
}
