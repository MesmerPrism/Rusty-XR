use openxr as xr;
use rusty_xr_camera_model::{
    camera_basis_from_camera2_reference_pose_relative_to_center, invert_homography,
    screen_to_camera_uv_homography, surface_to_camera_uv_homography,
    surface_to_eye_screen_uv_homography,
};
use rusty_xr_contracts::Eye;

use super::{
    openxr_gles_config::{OesContentMappingMode, OesProjectionBorderPolicy},
    projection_geometry::{OesEyeProjection, OesProjectionPlan},
    projection_plan_shared::{
        eye_basis_from_xr_view, preview_surface_corners, screen_to_domain_with_visual_adjustment,
        shared_per_eye_projection_plan, source_sampling_visual_adjustment,
        tracking_basis_from_xr_views, use_surface_texture_transform_for_stimulus,
    },
    source_metadata::OesProjectionMetadata,
    source_metadata_labels::{projection_source_label, projection_surface_aspect_from_metadata},
};

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
    target_footprint_from_metadata: bool,
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
    let (left_sample_offset_uv, left_sample_scale_uv) = source_sampling_visual_adjustment(
        target_footprint_from_metadata,
        projection_area_eye_offset_uv[0],
        projection_area_scale,
    );
    let (right_sample_offset_uv, right_sample_scale_uv) = source_sampling_visual_adjustment(
        target_footprint_from_metadata,
        projection_area_eye_offset_uv[1],
        projection_area_scale,
    );
    let left_screen_to_surface_h = screen_to_domain_with_visual_adjustment(
        invert_homography(left_surface_to_screen)?,
        left_sample_offset_uv,
        left_sample_scale_uv,
    );
    let right_screen_to_surface_h = screen_to_domain_with_visual_adjustment(
        invert_homography(right_surface_to_screen)?,
        right_sample_offset_uv,
        right_sample_scale_uv,
    );
    let left_screen_to_camera_h = screen_to_domain_with_visual_adjustment(
        screen_to_camera_uv_homography(left_surface_to_screen, left_surface_to_camera).ok()?,
        left_sample_offset_uv,
        left_sample_scale_uv,
    );
    let right_screen_to_camera_h = screen_to_domain_with_visual_adjustment(
        screen_to_camera_uv_homography(right_surface_to_screen, right_surface_to_camera).ok()?,
        right_sample_offset_uv,
        right_sample_scale_uv,
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
