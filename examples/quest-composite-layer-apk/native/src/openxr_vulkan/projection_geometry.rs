use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::{
    camera_basis_from_camera2_reference_pose_relative_to_center, invert_homography,
    scale_intrinsics_to_image, screen_to_camera_uv_homography, surface_to_camera_uv_homography,
    surface_to_eye_screen_uv_homography, Vec3,
};

use super::projection_homography_utils::{
    domain_to_screen_with_visual_offset, full_target_canvas_clip, identity_homography,
    screen_to_domain_with_visual_offset,
};
use super::projection_profile::{
    content_surface_aspect, frame_requests_full_frame_stimulus_mapping, full_target_canvas_aspect,
};
use super::projection_view_basis::{
    camera_preview_surface_corners, eye_basis_from_view, tracking_basis_from_views,
};
use super::{osc_overlay_eye_projection, project_points_to_eye_clip, StereoHomographyProjection};
use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

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
