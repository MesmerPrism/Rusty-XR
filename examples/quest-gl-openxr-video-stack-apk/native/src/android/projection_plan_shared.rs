use openxr as xr;
use rusty_xr_camera_model::{
    head_anchored_preview_surface_corners, CameraBasis, CameraIntrinsics, FeedPlacementDescriptor,
    ImageSize, PerEyeVideoProjectionPlan, Quat, Rect2, TrackingBasis, Vec2, Vec3,
    VideoProjectionMapping,
};
use rusty_xr_contracts::Eye;

use super::{
    openxr_gles_config::{OesContentMappingMode, OesProjectionBorderPolicy},
    projection_contract_markers::projection_area_screen_uv_rect,
    source_metadata::OesProjectionMetadata,
};

const PROJECTION_FOOTPRINT_GRID: usize = 64;

#[allow(clippy::too_many_arguments)]
pub(super) fn shared_per_eye_projection_plan(
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

pub(super) fn preview_surface_corners(
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

pub(super) fn synthetic_broker_intrinsics(
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

pub(super) fn eye_basis_from_xr_view(view: &xr::View) -> Option<CameraBasis> {
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

pub(super) fn tracking_basis_from_xr_views(
    left: &xr::View,
    right: &xr::View,
) -> Option<TrackingBasis> {
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

pub(super) fn use_surface_texture_transform_for_stimulus(metadata: &OesProjectionMetadata) -> bool {
    !metadata.has_explicit_top_left_stimulus_orientation()
}

pub(super) fn screen_to_domain_with_visual_adjustment(
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

pub(super) fn source_sampling_visual_adjustment(
    target_footprint_from_metadata: bool,
    offset_uv: [f32; 2],
    scale_uv: [f32; 2],
) -> ([f32; 2], [f32; 2]) {
    if target_footprint_from_metadata {
        ([0.0, 0.0], [1.0, 1.0])
    } else {
        (offset_uv, scale_uv)
    }
}

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
