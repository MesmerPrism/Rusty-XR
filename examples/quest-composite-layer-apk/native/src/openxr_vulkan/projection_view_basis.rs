use openxr as xr;
use rusty_xr_camera_model::{
    head_anchored_preview_surface_corners, CameraBasis, Quat, TrackingBasis, Vec3,
};

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
        *corner += offset;
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

pub(super) fn fov_aspect(fov: xr::Fovf) -> Option<f32> {
    let width = fov.angle_right.tan() - fov.angle_left.tan();
    let height = fov.angle_up.tan() - fov.angle_down.tan();
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        Some(width / height)
    } else {
        None
    }
}
