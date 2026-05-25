use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::Vec3;

use super::projection_eye_mapping::projected_display_eye_homography;
use super::projection_profile::frame_requests_full_frame_stimulus_mapping;
use super::StereoHomographyProjection;
use crate::StereoGpuCameraFrame;

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
