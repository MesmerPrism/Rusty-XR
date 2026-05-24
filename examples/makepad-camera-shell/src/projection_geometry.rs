use rusty_xr_camera_model::{
    homography_unit_square_bounding_rect, rect_xywh, source_valid_screen_uv_footprint,
    uv_rect_token, Rect2,
};
#[cfg(target_os = "android")]
use rusty_xr_camera_model::{
    camera2_lens_pose_to_extrinsics, camera_basis_from_camera2_reference_pose_relative_to_center,
    head_anchored_preview_surface_corners, invert_homography, scale_intrinsics_to_image,
    screen_to_camera_uv_homography, surface_to_camera_uv_homography,
    surface_to_eye_screen_uv_homography, CameraBasis, CameraIntrinsics, ImageSize, Quat,
    TrackingBasis, Vec2, Vec3,
};
#[cfg(target_os = "android")]
use std::ffi::CString;

use super::{
    hotload_bool, hotload_f32, makepad_blur_radius_px, makepad_current_source_color_contract_fields,
    makepad_projection_depth_meters,
    makepad_projection_panel_geometry, makepad_projection_preview_fov_y_degrees,
    makepad_projection_preview_offset_y_meters, makepad_projection_raw_overscan, marker_token, App,
    MakepadCameraPair, MakepadProcessingLayer, MakepadProjectionAlphaMode,
    MakepadProjectionBorderPolicy,
    KEY_MAKEPAD_NATIVE_PASSTHROUGH_ENABLED, KEY_MAKEPAD_PROJECTION_AREA_OFFSET_LEFT_UV,
    KEY_MAKEPAD_PROJECTION_AREA_OFFSET_RIGHT_UV, KEY_MAKEPAD_PROJECTION_AREA_OFFSET_VERTICAL_UV,
    KEY_MAKEPAD_PROJECTION_AREA_RADIUS_X_UV, KEY_MAKEPAD_PROJECTION_AREA_RADIUS_Y_UV,
    KEY_MAKEPAD_PROJECTION_AREA_SCALE_X, KEY_MAKEPAD_PROJECTION_AREA_SCALE_Y,
    SOURCE_VALID_FOOTPRINT_GRID, TARGET_DISPLAY_ASPECT, TARGET_PROJECTION_AREA_OFFSET_LEFT_UV,
    TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV, TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV,
    TARGET_PROJECTION_AREA_RADIUS_X_UV, TARGET_PROJECTION_AREA_RADIUS_Y_UV,
    TARGET_PROJECTION_AREA_SCALE_X, TARGET_PROJECTION_AREA_SCALE_Y,
};
#[cfg(target_os = "android")]
use super::Camera2StereoPlan;
#[cfg(target_os = "android")]
use super::source_metadata::{
    broker_pair_content_geometry_marker_fields, BrokerH264ProjectionMetadata,
};
#[cfg(target_os = "android")]
use crate::acamera_sys::ACAMERA_LENS_FACING_BACK;

#[cfg(target_os = "android")]
use super::android_camera_probe::{CameraSource, NativeIntrinsics};

#[cfg(target_os = "android")]
const DEFAULT_PROJECTION_TARGET_DEPTH_METERS: f32 = 1.0;
#[cfg(target_os = "android")]
const PROJECTION_PREVIEW_FOV_Y_DEGREES: f32 = 60.0;
#[cfg(target_os = "android")]
const PROJECTION_RAW_OVERSCAN: f32 = 1.06;
#[cfg(target_os = "android")]
const PROJECTION_SOURCE_ASPECT: f32 = 1.0;
#[cfg(target_os = "android")]
const DISPLAY_EYE_OFFSET_METERS: f32 = 0.032;
#[cfg(target_os = "android")]
const DISPLAY_FOV_Y_DEGREES: f32 = 92.0;
#[cfg(target_os = "android")]
const DISPLAY_ASPECT: f32 = 1.0;
#[cfg(target_os = "android")]
const DEFAULT_DISPLAY_SOURCE_EYE_MAPPING: &str = "display-left-from-left-source";
#[cfg(target_os = "android")]
const IDENTITY_HOMOGRAPHY: [[f32; 3]; 3] =
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

#[cfg(target_os = "android")]
#[derive(Clone, Copy, Debug)]
pub struct XrDisplayEyeView {
    pub position: [f32; 3],
    pub orientation: [f32; 4],
    pub angle_left: f32,
    pub angle_right: f32,
    pub angle_up: f32,
    pub angle_down: f32,
    pub valid: bool,
}

#[cfg(target_os = "android")]
#[derive(Clone, Copy, Debug)]
pub struct XrDisplayViews {
    pub left: XrDisplayEyeView,
    pub right: XrDisplayEyeView,
    pub predicted_display_time_ns: i64,
    pub reference_space: &'static str,
    pub projection_depth_meters: f32,
    pub projection_preview_fov_y_degrees: f32,
    pub projection_preview_offset_y_meters: f32,
    pub projection_raw_overscan: f32,
}

#[cfg(target_os = "android")]
#[derive(Clone, Copy, Debug)]
pub struct XrProjectionContract {
    pub reference_space: &'static str,
    pub openxr_reference_space: &'static str,
    pub display_time_source: &'static str,
    pub predicted_display_time_ns: Option<i64>,
    pub view_pose_fov_source: &'static str,
    pub projection_depth_meters: Option<f32>,
    pub projection_preview_fov_y_degrees: Option<f32>,
    pub projection_preview_offset_y_meters: Option<f32>,
    pub projection_raw_overscan: Option<f32>,
    pub left_render_fov_tangents: Option<[f32; 4]>,
    pub right_render_fov_tangents: Option<[f32; 4]>,
    pub left_render_position: Option<[f32; 4]>,
    pub right_render_position: Option<[f32; 4]>,
    pub left_render_orientation: Option<[f32; 4]>,
    pub right_render_orientation: Option<[f32; 4]>,
}

#[cfg(target_os = "android")]
impl XrProjectionContract {
    fn missing() -> Self {
        Self {
            reference_space: "not-logged",
            openxr_reference_space: "not-logged",
            display_time_source: "not-logged",
            predicted_display_time_ns: None,
            view_pose_fov_source: "not-logged",
            projection_depth_meters: None,
            projection_preview_fov_y_degrees: None,
            projection_preview_offset_y_meters: None,
            projection_raw_overscan: None,
            left_render_fov_tangents: None,
            right_render_fov_tangents: None,
            left_render_position: None,
            right_render_position: None,
            left_render_orientation: None,
            right_render_orientation: None,
        }
    }

    fn from_views(views: XrDisplayViews) -> Self {
        Self {
            reference_space: "app-reference-space",
            openxr_reference_space: views.reference_space,
            display_time_source: "predicted-display-time",
            predicted_display_time_ns: Some(views.predicted_display_time_ns),
            view_pose_fov_source: "makepad-xr-XrUpdateEvent",
            projection_depth_meters: Some(views.projection_depth_meters),
            projection_preview_fov_y_degrees: Some(views.projection_preview_fov_y_degrees),
            projection_preview_offset_y_meters: Some(views.projection_preview_offset_y_meters),
            projection_raw_overscan: Some(views.projection_raw_overscan),
            left_render_fov_tangents: Some(fov_tangents(views.left)),
            right_render_fov_tangents: Some(fov_tangents(views.right)),
            left_render_position: Some(position_vec4(views.left)),
            right_render_position: Some(position_vec4(views.right)),
            left_render_orientation: Some(orientation_vec4(views.left)),
            right_render_orientation: Some(orientation_vec4(views.right)),
        }
    }
}

#[cfg(target_os = "android")]
#[derive(Clone, Debug)]
struct ProjectionHomographies {
    left_surface_to_camera_h: [[f32; 3]; 3],
    right_surface_to_camera_h: [[f32; 3]; 3],
    left_surface_to_screen_h: [[f32; 3]; 3],
    right_surface_to_screen_h: [[f32; 3]; 3],
    left_screen_to_camera_h: [[f32; 3]; 3],
    right_screen_to_camera_h: [[f32; 3]; 3],
    left_screen_to_surface_h: [[f32; 3]; 3],
    right_screen_to_surface_h: [[f32; 3]; 3],
}

#[cfg(target_os = "android")]
impl ProjectionHomographies {
    fn identity() -> Self {
        Self {
            left_surface_to_camera_h: IDENTITY_HOMOGRAPHY,
            right_surface_to_camera_h: IDENTITY_HOMOGRAPHY,
            left_surface_to_screen_h: IDENTITY_HOMOGRAPHY,
            right_surface_to_screen_h: IDENTITY_HOMOGRAPHY,
            left_screen_to_camera_h: IDENTITY_HOMOGRAPHY,
            right_screen_to_camera_h: IDENTITY_HOMOGRAPHY,
            left_screen_to_surface_h: IDENTITY_HOMOGRAPHY,
            right_screen_to_surface_h: IDENTITY_HOMOGRAPHY,
        }
    }
}

#[cfg(target_os = "android")]
#[derive(Clone)]
pub(super) struct StereoProjectionSources {
    pub(super) left_source_index: usize,
    pub(super) left: CameraSource,
    pub(super) right_source_index: usize,
    pub(super) right: CameraSource,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[cfg(target_os = "android")]
#[derive(Clone, Debug)]
pub struct StereoProjectionPlan {
    pub left_source_index: usize,
    pub right_source_index: usize,
    pub left_camera_id: String,
    pub right_camera_id: String,
    pub left_facing: &'static str,
    pub right_facing: &'static str,
    pub width: u32,
    pub height: u32,
    pub projection_metadata_ready: bool,
    pub pose_source: &'static str,
    pub source_eye_mapping: &'static str,
    pub coordinate_chain: &'static str,
    pub fallback_reason: &'static str,
    pub left_surface_to_camera_h: [[f32; 3]; 3],
    pub right_surface_to_camera_h: [[f32; 3]; 3],
    pub left_surface_to_screen_h: [[f32; 3]; 3],
    pub right_surface_to_screen_h: [[f32; 3]; 3],
    pub left_screen_to_camera_h: [[f32; 3]; 3],
    pub right_screen_to_camera_h: [[f32; 3]; 3],
    pub left_screen_to_surface_h: [[f32; 3]; 3],
    pub right_screen_to_surface_h: [[f32; 3]; 3],
    pub projection_homography_ready: bool,
    pub runtime_xr_view_state_ready: bool,
    pub openxr_contract: XrProjectionContract,
}

#[cfg(target_os = "android")]
#[derive(Clone, Debug)]
pub struct BrokerProjectionSource {
    pub camera_id: String,
    pub intrinsics_fx: f32,
    pub intrinsics_fy: f32,
    pub intrinsics_cx: f32,
    pub intrinsics_cy: f32,
    pub intrinsics_skew: f32,
    pub intrinsics_domain_width: u32,
    pub intrinsics_domain_height: u32,
    pub pose_translation: [f32; 3],
    pub pose_rotation: [f32; 4],
}

#[cfg(target_os = "android")]
impl BrokerProjectionSource {
    fn camera_source(&self) -> Option<CameraSource> {
        let camera_id_c = CString::new(self.camera_id.clone()).ok()?;
        let intrinsics_width = self.intrinsics_domain_width.max(1);
        let intrinsics_height = self.intrinsics_domain_height.max(1);
        Some(CameraSource {
            camera_id_c,
            lens_facing: ACAMERA_LENS_FACING_BACK,
            logical_multi_camera: false,
            physical_camera_ids: Vec::new(),
            sensor_sync_type: None,
            private_sizes: Vec::new(),
            intrinsics: Some(NativeIntrinsics {
                fx: self.intrinsics_fx,
                fy: self.intrinsics_fy,
                cx: self.intrinsics_cx,
                cy: self.intrinsics_cy,
                skew: self.intrinsics_skew,
            }),
            active_array_size: Some((intrinsics_width, intrinsics_height)),
            pose_translation: Some(self.pose_translation),
            pose_rotation: Some(self.pose_rotation),
        })
    }
}

#[cfg(target_os = "android")]
impl StereoProjectionPlan {
    pub(super) fn from_sources(sources: &StereoProjectionSources) -> Self {
        let left = &sources.left;
        let right = &sources.right;
        let width = sources.width;
        let height = sources.height;
        let projection_metadata_ready = left.has_projection_metadata()
            && right.has_projection_metadata()
            && width > 0
            && height > 0;
        let (homographies, projection_homography_ready) =
            stereo_projection_homographies(left, right, width, height)
                .map(|homographies| (homographies, true))
                .unwrap_or_else(|| (ProjectionHomographies::identity(), false));

        Self {
            left_source_index: sources.left_source_index,
            right_source_index: sources.right_source_index,
            left_camera_id: left.camera_id_label(),
            right_camera_id: right.camera_id_label(),
            left_facing: left.lens_facing_label(),
            right_facing: right.lens_facing_label(),
            width,
            height,
            projection_metadata_ready,
            pose_source: if projection_metadata_ready {
                "platform"
            } else {
                "missing"
            },
            source_eye_mapping: display_source_eye_mapping(),
            coordinate_chain: "camera2-sensor-reference-to-openxr-head-basis",
            fallback_reason: if projection_metadata_ready {
                "none"
            } else {
                "selected stereo pair is missing intrinsics or pose metadata"
            },
            left_surface_to_camera_h: homographies.left_surface_to_camera_h,
            right_surface_to_camera_h: homographies.right_surface_to_camera_h,
            left_surface_to_screen_h: homographies.left_surface_to_screen_h,
            right_surface_to_screen_h: homographies.right_surface_to_screen_h,
            left_screen_to_camera_h: homographies.left_screen_to_camera_h,
            right_screen_to_camera_h: homographies.right_screen_to_camera_h,
            left_screen_to_surface_h: homographies.left_screen_to_surface_h,
            right_screen_to_surface_h: homographies.right_screen_to_surface_h,
            projection_homography_ready,
            runtime_xr_view_state_ready: false,
            openxr_contract: XrProjectionContract::missing(),
        }
    }

    pub(super) fn from_sources_and_xr_views(
        sources: &StereoProjectionSources,
        views: XrDisplayViews,
    ) -> Option<Self> {
        let left = &sources.left;
        let right = &sources.right;
        let projection_metadata_ready = left.has_projection_metadata()
            && right.has_projection_metadata()
            && sources.width > 0
            && sources.height > 0
            && views.left.valid
            && views.right.valid;
        let homographies = stereo_projection_homographies_from_xr_views(
            left,
            right,
            sources.width,
            sources.height,
            views,
        )?;

        Some(Self {
            left_source_index: sources.left_source_index,
            right_source_index: sources.right_source_index,
            left_camera_id: left.camera_id_label(),
            right_camera_id: right.camera_id_label(),
            left_facing: left.lens_facing_label(),
            right_facing: right.lens_facing_label(),
            width: sources.width,
            height: sources.height,
            projection_metadata_ready,
            pose_source: "platform-openxr-view",
            source_eye_mapping: display_source_eye_mapping(),
            coordinate_chain: "camera2-sensor-reference-to-openxr-head-basis",
            fallback_reason: "none",
            left_surface_to_camera_h: homographies.left_surface_to_camera_h,
            right_surface_to_camera_h: homographies.right_surface_to_camera_h,
            left_surface_to_screen_h: homographies.left_surface_to_screen_h,
            right_surface_to_screen_h: homographies.right_surface_to_screen_h,
            left_screen_to_camera_h: homographies.left_screen_to_camera_h,
            right_screen_to_camera_h: homographies.right_screen_to_camera_h,
            left_screen_to_surface_h: homographies.left_screen_to_surface_h,
            right_screen_to_surface_h: homographies.right_screen_to_surface_h,
            projection_homography_ready: projection_metadata_ready,
            runtime_xr_view_state_ready: true,
            openxr_contract: XrProjectionContract::from_views(views),
        })
    }
}

#[cfg(target_os = "android")]
pub fn broker_physical_projection_plan_from_xr_views(
    left_source: BrokerProjectionSource,
    right_source: BrokerProjectionSource,
    width: u32,
    height: u32,
    views: XrDisplayViews,
) -> Option<StereoProjectionPlan> {
    if width == 0 || height == 0 {
        return None;
    }
    let left_camera_id = left_source.camera_id.clone();
    let right_camera_id = right_source.camera_id.clone();
    let left = left_source.camera_source()?;
    let right = right_source.camera_source()?;
    let homographies =
        stereo_projection_homographies_from_xr_views(&left, &right, width, height, views)?;

    Some(StereoProjectionPlan {
        left_source_index: 0,
        right_source_index: 1,
        left_camera_id,
        right_camera_id,
        left_facing: "back",
        right_facing: "back",
        width,
        height,
        projection_metadata_ready: true,
        pose_source: "broker-stream-header-platform-openxr-view",
        source_eye_mapping: display_source_eye_mapping(),
        coordinate_chain: "broker-h264-physical-camera-stream-header-to-openxr-view",
        fallback_reason: "none",
        left_surface_to_camera_h: homographies.left_surface_to_camera_h,
        right_surface_to_camera_h: homographies.right_surface_to_camera_h,
        left_surface_to_screen_h: homographies.left_surface_to_screen_h,
        right_surface_to_screen_h: homographies.right_surface_to_screen_h,
        left_screen_to_camera_h: homographies.left_screen_to_camera_h,
        right_screen_to_camera_h: homographies.right_screen_to_camera_h,
        left_screen_to_surface_h: homographies.left_screen_to_surface_h,
        right_screen_to_surface_h: homographies.right_screen_to_surface_h,
        projection_homography_ready: true,
        runtime_xr_view_state_ready: true,
        openxr_contract: XrProjectionContract::from_views(views),
    })
}

#[cfg(target_os = "android")]
fn preview_surface_corners(
    tracking: TrackingBasis,
    views: XrDisplayViews,
    aspect: f32,
) -> Option<[Vec3; 4]> {
    let mut surface = head_anchored_preview_surface_corners(
        tracking,
        views.projection_preview_fov_y_degrees,
        views.projection_depth_meters,
        aspect,
        views.projection_raw_overscan,
    )
    .ok()?;
    let offset = tracking.up * views.projection_preview_offset_y_meters.clamp(-2.0, 2.0);
    for corner in &mut surface {
        *corner = *corner + offset;
    }
    Some(surface)
}

#[cfg(target_os = "android")]
pub fn broker_synthetic_projection_plan_from_xr_views(
    left_camera_id: &str,
    right_camera_id: &str,
    width: u32,
    height: u32,
    views: XrDisplayViews,
) -> Option<StereoProjectionPlan> {
    if width == 0 || height == 0 {
        return None;
    }
    let tracking = tracking_basis_from_xr_views(views)?;
    let aspect = projection_surface_aspect(width, height);
    let surface = preview_surface_corners(tracking, views, aspect)?;
    let intrinsics =
        synthetic_broker_intrinsics(width, height, views.projection_preview_fov_y_degrees)?;
    let camera_basis = CameraBasis::new(
        tracking.origin,
        tracking.right,
        tracking.up,
        tracking.forward,
    )?;
    let surface_to_camera =
        surface_to_camera_uv_homography(surface, camera_basis, intrinsics).ok()?;
    let left_eye_basis = eye_basis_from_xr_view(views.left)?;
    let right_eye_basis = eye_basis_from_xr_view(views.right)?;
    let left_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        left_eye_basis,
        views.left.angle_left.tan(),
        views.left.angle_right.tan(),
        views.left.angle_down.tan(),
        views.left.angle_up.tan(),
    )
    .ok()?;
    let right_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        right_eye_basis,
        views.right.angle_left.tan(),
        views.right.angle_right.tan(),
        views.right.angle_down.tan(),
        views.right.angle_up.tan(),
    )
    .ok()?;
    let left_screen_to_surface_h = invert_homography(left_surface_to_screen)?;
    let right_screen_to_surface_h = invert_homography(right_surface_to_screen)?;
    let left_screen_to_camera_h =
        screen_to_camera_uv_homography(left_surface_to_screen, surface_to_camera).ok()?;
    let right_screen_to_camera_h =
        screen_to_camera_uv_homography(right_surface_to_screen, surface_to_camera).ok()?;

    Some(StereoProjectionPlan {
        left_source_index: 0,
        right_source_index: 1,
        left_camera_id: left_camera_id.to_string(),
        right_camera_id: right_camera_id.to_string(),
        left_facing: "synthetic",
        right_facing: "synthetic",
        width,
        height,
        projection_metadata_ready: true,
        pose_source: "estimated-profile",
        source_eye_mapping: "left-right",
        coordinate_chain: "broker-synthetic-head-anchored-preview-to-openxr-view",
        fallback_reason: "none",
        left_surface_to_camera_h: surface_to_camera,
        right_surface_to_camera_h: surface_to_camera,
        left_surface_to_screen_h: left_surface_to_screen,
        right_surface_to_screen_h: right_surface_to_screen,
        left_screen_to_camera_h,
        right_screen_to_camera_h,
        left_screen_to_surface_h,
        right_screen_to_surface_h,
        projection_homography_ready: true,
        runtime_xr_view_state_ready: true,
        openxr_contract: XrProjectionContract::from_views(views),
    })
}

#[cfg(target_os = "android")]
pub fn broker_full_frame_projection_plan_from_xr_views(
    left_camera_id: &str,
    right_camera_id: &str,
    width: u32,
    height: u32,
    views: XrDisplayViews,
) -> Option<StereoProjectionPlan> {
    if width == 0 || height == 0 {
        return None;
    }
    let tracking = tracking_basis_from_xr_views(views)?;
    let aspect = projection_surface_aspect(width, height);
    let surface = preview_surface_corners(tracking, views, aspect)?;
    let left_eye_basis = eye_basis_from_xr_view(views.left)?;
    let right_eye_basis = eye_basis_from_xr_view(views.right)?;
    let left_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        left_eye_basis,
        views.left.angle_left.tan(),
        views.left.angle_right.tan(),
        views.left.angle_down.tan(),
        views.left.angle_up.tan(),
    )
    .ok()?;
    let right_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        right_eye_basis,
        views.right.angle_left.tan(),
        views.right.angle_right.tan(),
        views.right.angle_down.tan(),
        views.right.angle_up.tan(),
    )
    .ok()?;
    let left_screen_to_surface_h = invert_homography(left_surface_to_screen)?;
    let right_screen_to_surface_h = invert_homography(right_surface_to_screen)?;

    Some(StereoProjectionPlan {
        left_source_index: 0,
        right_source_index: 1,
        left_camera_id: left_camera_id.to_string(),
        right_camera_id: right_camera_id.to_string(),
        left_facing: "synthetic",
        right_facing: "synthetic",
        width,
        height,
        projection_metadata_ready: true,
        pose_source: "projection-surface",
        source_eye_mapping: "left-right",
        coordinate_chain: "broker-synthetic-full-frame-projection-surface-to-openxr-view",
        fallback_reason: "none",
        left_surface_to_camera_h: IDENTITY_HOMOGRAPHY,
        right_surface_to_camera_h: IDENTITY_HOMOGRAPHY,
        left_surface_to_screen_h: left_surface_to_screen,
        right_surface_to_screen_h: right_surface_to_screen,
        left_screen_to_camera_h: left_screen_to_surface_h,
        right_screen_to_camera_h: right_screen_to_surface_h,
        left_screen_to_surface_h,
        right_screen_to_surface_h,
        projection_homography_ready: true,
        runtime_xr_view_state_ready: true,
        openxr_contract: XrProjectionContract::from_views(views),
    })
}

#[cfg(target_os = "android")]
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

#[cfg(target_os = "android")]
fn stereo_projection_homographies(
    left: &CameraSource,
    right: &CameraSource,
    delivered_width: u32,
    delivered_height: u32,
) -> Option<ProjectionHomographies> {
    let left_extrinsics =
        camera2_lens_pose_to_extrinsics(left.pose_translation?, left.pose_rotation?).ok()?;
    let right_extrinsics =
        camera2_lens_pose_to_extrinsics(right.pose_translation?, right.pose_rotation?).ok()?;
    let reference_center = (left_extrinsics.world_from_camera.position
        + right_extrinsics.world_from_camera.position)
        * 0.5;
    let tracking = TrackingBasis::new(Vec3::ZERO, Vec3::RIGHT, Vec3::UP, Vec3::FORWARD_NEG_Z)?;
    let surface = head_anchored_preview_surface_corners(
        tracking,
        PROJECTION_PREVIEW_FOV_Y_DEGREES,
        DEFAULT_PROJECTION_TARGET_DEPTH_METERS,
        PROJECTION_SOURCE_ASPECT,
        PROJECTION_RAW_OVERSCAN,
    )
    .ok()?;
    let left_intrinsics = scaled_intrinsics(left, delivered_width, delivered_height)?;
    let right_intrinsics = scaled_intrinsics(right, delivered_width, delivered_height)?;
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
    let left_h = surface_to_camera_uv_homography(surface, left_basis, left_intrinsics).ok()?;
    let right_h = surface_to_camera_uv_homography(surface, right_basis, right_intrinsics).ok()?;
    let (display_left_surface_to_camera_h, display_right_surface_to_camera_h) =
        display_mapped_surface_to_camera_homographies(left_h, right_h);
    let left_eye_basis = display_eye_basis(-DISPLAY_EYE_OFFSET_METERS)?;
    let right_eye_basis = display_eye_basis(DISPLAY_EYE_OFFSET_METERS)?;
    let tan_y = (DISPLAY_FOV_Y_DEGREES * 0.5).to_radians().tan();
    let tan_x = tan_y * DISPLAY_ASPECT.max(0.1);
    let left_surface_to_screen =
        surface_to_eye_screen_uv_homography(surface, left_eye_basis, -tan_x, tan_x, -tan_y, tan_y)
            .ok()?;
    let right_surface_to_screen =
        surface_to_eye_screen_uv_homography(surface, right_eye_basis, -tan_x, tan_x, -tan_y, tan_y)
            .ok()?;
    let left_screen_to_surface_h = invert_homography(left_surface_to_screen)?;
    let right_screen_to_surface_h = invert_homography(right_surface_to_screen)?;
    let left_screen_to_camera_h =
        screen_to_camera_uv_homography(left_surface_to_screen, display_left_surface_to_camera_h)
            .ok()?;
    let right_screen_to_camera_h =
        screen_to_camera_uv_homography(right_surface_to_screen, display_right_surface_to_camera_h)
            .ok()?;
    Some(ProjectionHomographies {
        left_surface_to_camera_h: display_left_surface_to_camera_h,
        right_surface_to_camera_h: display_right_surface_to_camera_h,
        left_surface_to_screen_h: left_surface_to_screen,
        right_surface_to_screen_h: right_surface_to_screen,
        left_screen_to_camera_h,
        right_screen_to_camera_h,
        left_screen_to_surface_h,
        right_screen_to_surface_h,
    })
}

#[cfg(target_os = "android")]
fn stereo_projection_homographies_from_xr_views(
    left: &CameraSource,
    right: &CameraSource,
    delivered_width: u32,
    delivered_height: u32,
    views: XrDisplayViews,
) -> Option<ProjectionHomographies> {
    let left_extrinsics =
        camera2_lens_pose_to_extrinsics(left.pose_translation?, left.pose_rotation?).ok()?;
    let right_extrinsics =
        camera2_lens_pose_to_extrinsics(right.pose_translation?, right.pose_rotation?).ok()?;
    let reference_center = (left_extrinsics.world_from_camera.position
        + right_extrinsics.world_from_camera.position)
        * 0.5;
    let tracking = tracking_basis_from_xr_views(views)?;
    let aspect = projection_surface_aspect(delivered_width, delivered_height);
    let surface = preview_surface_corners(tracking, views, aspect)?;
    let left_intrinsics = scaled_intrinsics(left, delivered_width, delivered_height)?;
    let right_intrinsics = scaled_intrinsics(right, delivered_width, delivered_height)?;
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
    let left_h = surface_to_camera_uv_homography(surface, left_basis, left_intrinsics).ok()?;
    let right_h = surface_to_camera_uv_homography(surface, right_basis, right_intrinsics).ok()?;
    let (display_left_surface_to_camera_h, display_right_surface_to_camera_h) =
        display_mapped_surface_to_camera_homographies(left_h, right_h);
    let left_eye_basis = eye_basis_from_xr_view(views.left)?;
    let right_eye_basis = eye_basis_from_xr_view(views.right)?;
    let left_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        left_eye_basis,
        views.left.angle_left.tan(),
        views.left.angle_right.tan(),
        views.left.angle_down.tan(),
        views.left.angle_up.tan(),
    )
    .ok()?;
    let right_surface_to_screen = surface_to_eye_screen_uv_homography(
        surface,
        right_eye_basis,
        views.right.angle_left.tan(),
        views.right.angle_right.tan(),
        views.right.angle_down.tan(),
        views.right.angle_up.tan(),
    )
    .ok()?;
    let left_screen_to_surface_h = invert_homography(left_surface_to_screen)?;
    let right_screen_to_surface_h = invert_homography(right_surface_to_screen)?;
    let left_screen_to_camera_h =
        screen_to_camera_uv_homography(left_surface_to_screen, display_left_surface_to_camera_h)
            .ok()?;
    let right_screen_to_camera_h =
        screen_to_camera_uv_homography(right_surface_to_screen, display_right_surface_to_camera_h)
            .ok()?;
    Some(ProjectionHomographies {
        left_surface_to_camera_h: display_left_surface_to_camera_h,
        right_surface_to_camera_h: display_right_surface_to_camera_h,
        left_surface_to_screen_h: left_surface_to_screen,
        right_surface_to_screen_h: right_surface_to_screen,
        left_screen_to_camera_h,
        right_screen_to_camera_h,
        left_screen_to_surface_h,
        right_screen_to_surface_h,
    })
}

#[cfg(target_os = "android")]
fn display_mapped_surface_to_camera_homographies(
    physical_left_h: [[f32; 3]; 3],
    physical_right_h: [[f32; 3]; 3],
) -> ([[f32; 3]; 3], [[f32; 3]; 3]) {
    // Homography rows are display-indexed. The display/source eye mapping is
    // applied only when selecting the source texture; remapping homographies
    // here double-swaps the cameras for the diagnostic inverted-source lane.
    (physical_left_h, physical_right_h)
}

#[cfg(target_os = "android")]
fn display_source_eye_mapping() -> &'static str {
    match option_env!("RUSTY_XR_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING") {
        Some("display-left-from-left-source") => "display-left-from-left-source",
        Some("display-left-from-right-source") => "display-left-from-right-source",
        _ => DEFAULT_DISPLAY_SOURCE_EYE_MAPPING,
    }
}

#[cfg(target_os = "android")]
fn eye_basis_from_xr_view(view: XrDisplayEyeView) -> Option<CameraBasis> {
    if !view.valid {
        return None;
    }
    let orientation = Quat::new(
        view.orientation[0],
        view.orientation[1],
        view.orientation[2],
        view.orientation[3],
    )
    .normalized_or(Quat::IDENTITY);
    CameraBasis::new(
        Vec3::new(view.position[0], view.position[1], view.position[2]),
        orientation.rotate_vec3(Vec3::RIGHT),
        orientation.rotate_vec3(Vec3::UP),
        orientation.rotate_vec3(Vec3::FORWARD_NEG_Z),
    )
}

#[cfg(target_os = "android")]
fn tracking_basis_from_xr_views(views: XrDisplayViews) -> Option<TrackingBasis> {
    if !views.left.valid || !views.right.valid {
        return None;
    }
    let position = Vec3::new(
        (views.left.position[0] + views.right.position[0]) * 0.5,
        (views.left.position[1] + views.right.position[1]) * 0.5,
        (views.left.position[2] + views.right.position[2]) * 0.5,
    );
    let orientation = Quat::new(
        views.left.orientation[0],
        views.left.orientation[1],
        views.left.orientation[2],
        views.left.orientation[3],
    )
    .normalized_or(Quat::IDENTITY);
    TrackingBasis::new(
        position,
        orientation.rotate_vec3(Vec3::RIGHT),
        orientation.rotate_vec3(Vec3::UP),
        orientation.rotate_vec3(Vec3::FORWARD_NEG_Z),
    )
}

#[cfg(target_os = "android")]
fn projection_surface_aspect(width: u32, height: u32) -> f32 {
    if width == 0 || height == 0 {
        return PROJECTION_SOURCE_ASPECT;
    }
    ((width as f32) / (height as f32)).clamp(0.25, 4.0)
}

#[cfg(target_os = "android")]
fn fov_tangents(view: XrDisplayEyeView) -> [f32; 4] {
    [
        view.angle_left.tan(),
        view.angle_right.tan(),
        view.angle_up.tan(),
        view.angle_down.tan(),
    ]
}

#[cfg(target_os = "android")]
fn position_vec4(view: XrDisplayEyeView) -> [f32; 4] {
    [view.position[0], view.position[1], view.position[2], 1.0]
}

#[cfg(target_os = "android")]
fn orientation_vec4(view: XrDisplayEyeView) -> [f32; 4] {
    [
        view.orientation[0],
        view.orientation[1],
        view.orientation[2],
        view.orientation[3],
    ]
}

#[cfg(target_os = "android")]
fn display_eye_basis(x_offset_meters: f32) -> Option<CameraBasis> {
    CameraBasis::new(
        Vec3::new(x_offset_meters, 0.0, 0.0),
        Vec3::RIGHT,
        Vec3::UP,
        Vec3::FORWARD_NEG_Z,
    )
}

#[cfg(target_os = "android")]
fn scaled_intrinsics(
    source: &CameraSource,
    delivered_width: u32,
    delivered_height: u32,
) -> Option<CameraIntrinsics> {
    let intrinsics = source.intrinsics?;
    let source_size = source
        .active_array_size
        .unwrap_or((delivered_width, delivered_height));
    let source_intrinsics = CameraIntrinsics::new(
        Vec2::new(intrinsics.fx, intrinsics.fy),
        Vec2::new(intrinsics.cx, intrinsics.cy),
        ImageSize::new(source_size.0, source_size.1),
    )
    .with_skew_px(intrinsics.skew);
    scale_intrinsics_to_image(
        source_intrinsics,
        ImageSize::new(source_size.0, source_size.1),
        ImageSize::new(delivered_width, delivered_height),
    )
    .ok()
}

#[derive(Clone)]
pub(crate) struct MakepadOpenXrProjectionContract {
    reference_space: String,
    openxr_reference_space: String,
    display_time_source: String,
    predicted_display_time_ns: Option<i64>,
    view_pose_fov_source: String,
    projection_depth_meters: Option<f32>,
    projection_preview_fov_y_degrees: Option<f32>,
    projection_preview_offset_y_meters: Option<f32>,
    projection_raw_overscan: Option<f32>,
    left_render_fov_tangents: Option<[f32; 4]>,
    right_render_fov_tangents: Option<[f32; 4]>,
    left_render_position: Option<[f32; 4]>,
    right_render_position: Option<[f32; 4]>,
    left_render_orientation: Option<[f32; 4]>,
    right_render_orientation: Option<[f32; 4]>,
}

impl MakepadOpenXrProjectionContract {
    pub(crate) fn missing() -> Self {
        Self {
            reference_space: "not-logged".to_string(),
            openxr_reference_space: "not-logged".to_string(),
            display_time_source: "not-logged".to_string(),
            predicted_display_time_ns: None,
            view_pose_fov_source: "not-logged".to_string(),
            projection_depth_meters: None,
            projection_preview_fov_y_degrees: None,
            projection_preview_offset_y_meters: None,
            projection_raw_overscan: None,
            left_render_fov_tangents: None,
            right_render_fov_tangents: None,
            left_render_position: None,
            right_render_position: None,
            left_render_orientation: None,
            right_render_orientation: None,
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) fn from_android(contract: XrProjectionContract) -> Self {
        Self {
            reference_space: contract.reference_space.to_string(),
            openxr_reference_space: contract.openxr_reference_space.to_string(),
            display_time_source: contract.display_time_source.to_string(),
            predicted_display_time_ns: contract.predicted_display_time_ns,
            view_pose_fov_source: contract.view_pose_fov_source.to_string(),
            projection_depth_meters: contract.projection_depth_meters,
            projection_preview_fov_y_degrees: contract.projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters: contract.projection_preview_offset_y_meters,
            projection_raw_overscan: contract.projection_raw_overscan,
            left_render_fov_tangents: contract.left_render_fov_tangents,
            right_render_fov_tangents: contract.right_render_fov_tangents,
            left_render_position: contract.left_render_position,
            right_render_position: contract.right_render_position,
            left_render_orientation: contract.left_render_orientation,
            right_render_orientation: contract.right_render_orientation,
        }
    }
}

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

pub(crate) fn makepad_draw_vars_bound_marker_fields(
    pair: &MakepadCameraPair,
    yuv_mode: bool,
    broker_h264_surface_texture: bool,
    single_stream_visual_proof: bool,
    updated_stream_visual_proof_side: &str,
) -> String {
    draw_vars_bound_marker_fields(
        yuv_mode,
        broker_h264_surface_texture,
        single_stream_visual_proof,
        updated_stream_visual_proof_side,
        &projection_homography_marker_fields(pair),
    )
}

pub(crate) fn makepad_visible_panel_bound_marker_fields(
    pair: &MakepadCameraPair,
    left_rotation_steps: f32,
    right_rotation_steps: f32,
    single_stream_visual_proof: bool,
    updated_stream_visual_proof_side: &str,
) -> String {
    visible_panel_bound_marker_fields(
        &pair.source_eye_mapping,
        pair.left.source_index,
        pair.right.source_index,
        left_rotation_steps,
        right_rotation_steps,
        single_stream_visual_proof,
        updated_stream_visual_proof_side,
        &projection_homography_marker_fields(pair),
    )
}

pub(crate) fn makepad_projection_complete_marker_fields(
    pair: &MakepadCameraPair,
    paired_streams_ready: bool,
    broker_h264_surface_texture: bool,
    aligned_projection: bool,
    visible_projection_ready: bool,
    projection_mode: &str,
    left_rotation_steps: f32,
    right_rotation_steps: f32,
    projection_scale: f64,
    xr_render_scale: f64,
) -> String {
    let homography_marker_fields = projection_homography_marker_fields(pair);
    let fallback_reason = marker_token(&pair.fallback_reason);
    let cpu_upload_path = if broker_h264_surface_texture {
        "broker-h264-mediacodec-cpu-yuv"
    } else {
        "makepad-camera-cpu-yuv-plane"
    };
    complete_marker_fields(CompleteMarkerFields {
        paired_streams_ready,
        broker_h264_surface_texture,
        projection_mapping_ready: pair.projection_homography_ready,
        aligned_projection,
        visible_projection_ready,
        projection_metadata_ready: pair.projection_metadata_ready,
        pose_source: &pair.pose_source,
        source_eye_mapping: &pair.source_eye_mapping,
        coordinate_chain: &pair.coordinate_chain,
        projection_mode,
        left_source_index: pair.left.source_index,
        right_source_index: pair.right.source_index,
        left_source_class: pair.left.source_class,
        right_source_class: pair.right.source_class,
        left_width: pair.left.width,
        left_height: pair.left.height,
        right_width: pair.right.width,
        right_height: pair.right.height,
        left_rotation_steps,
        right_rotation_steps,
        projection_scale,
        xr_render_scale,
        homography_marker_fields: &homography_marker_fields,
        cpu_upload_path,
        fallback_reason: &fallback_reason,
    })
}

pub(crate) fn makepad_projection_start_marker_fields(
    pair: &MakepadCameraPair,
    projection_mode: &str,
    projection_scale: f64,
    xr_render_scale: f64,
) -> String {
    let homography_marker_fields = projection_homography_marker_fields(pair);
    let fallback_reason = marker_token(&pair.fallback_reason);
    projection_start_marker_fields(
        pair.projection_homography_ready,
        pair.projection_metadata_ready,
        &pair.pose_source,
        &pair.source_eye_mapping,
        &pair.coordinate_chain,
        &homography_marker_fields,
        pair.left.source_index,
        pair.right.source_index,
        projection_mode,
        projection_scale,
        xr_render_scale,
        &fallback_reason,
    )
}

pub(crate) fn makepad_paired_projection_progress_marker_fields(
    pair: &MakepadCameraPair,
    phase: &str,
    left_prepared: bool,
    right_prepared: bool,
    left_updated: bool,
    right_updated: bool,
) -> String {
    let homography_marker_fields = projection_homography_marker_fields(pair);
    let fallback_reason = marker_token(&pair.fallback_reason);
    paired_projection_progress_marker_fields(
        phase,
        left_prepared,
        right_prepared,
        left_updated,
        right_updated,
        pair.projection_homography_ready,
        pair.projection_metadata_ready,
        &pair.pose_source,
        &pair.source_eye_mapping,
        &homography_marker_fields,
        pair.left.source_index,
        pair.right.source_index,
        &fallback_reason,
    )
}

pub(crate) fn makepad_single_stream_proof_wait_marker_fields(
    left_updated: bool,
    right_updated: bool,
    left_yuv_ready: bool,
    right_yuv_ready: bool,
    projection_mapping_ready: bool,
    visible_projection_ready: bool,
    updated_stream_visual_proof_side: &str,
) -> String {
    single_stream_proof_wait_marker_fields(
        left_updated,
        right_updated,
        left_yuv_ready,
        right_yuv_ready,
        projection_mapping_ready,
        visible_projection_ready,
        updated_stream_visual_proof_side,
    )
}

pub(crate) fn makepad_projection_enumerated_marker_fields(
    pair: &MakepadCameraPair,
    source_count: usize,
    format_count: usize,
) -> String {
    let homography_marker_fields = projection_homography_marker_fields(pair);
    let fallback_reason = marker_token(&pair.fallback_reason);
    projection_enumerated_marker_fields(
        source_count,
        format_count,
        pair.projection_homography_ready,
        pair.projection_metadata_ready,
        &pair.pose_source,
        &pair.source_eye_mapping,
        &pair.coordinate_chain,
        &homography_marker_fields,
        pair.left.source_index,
        pair.right.source_index,
        &pair.source_binding_mode,
        pair.left.source_class,
        pair.right.source_class,
        pair.left.width,
        pair.left.height,
        pair.right.width,
        pair.right.height,
        &fallback_reason,
    )
}

pub(crate) struct MakepadStereoComparisonMarkerInputs<'a> {
    pub(crate) phase: &'a str,
    pub(crate) runtime_profile: &'a str,
    pub(crate) comparison_baseline: &'a str,
    pub(crate) camera_tier: &'a str,
    pub(crate) acquisition_profile: &'a str,
    pub(crate) transport_profile: &'a str,
    pub(crate) projection_mode: &'a str,
    pub(crate) synthetic_scene: &'a str,
    pub(crate) projection_scale: f64,
    pub(crate) xr_render_scale: f64,
    pub(crate) aligned_projection: bool,
    pub(crate) visible_projection_ready: bool,
    pub(crate) makepad_fork_branch: &'a str,
    pub(crate) makepad_fork_commit: &'a str,
}

pub(crate) fn makepad_stereo_comparison_marker_line(
    pair: &MakepadCameraPair,
    inputs: MakepadStereoComparisonMarkerInputs<'_>,
) -> String {
    let homography_marker_fields = projection_homography_marker_fields(pair);
    stereo_comparison_marker_line(
        pair,
        &inputs,
        &homography_marker_fields,
    )
}

fn draw_vars_bound_marker_fields(
    yuv_mode: bool,
    broker_h264_surface_texture: bool,
    single_stream_visual_proof: bool,
    updated_stream_visual_proof_side: &str,
    homography_marker_fields: &str,
) -> String {
    format!(
        "phase=draw-vars-bound status=ok cameraReady=true yuvMode={} neutralWaitingPanel=true textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping drawVarsTextureRedraw=true shaderAreaStateUpdate=true leftYuvTextureBound={} rightYuvTextureBound={} brokerH264SurfaceTexture={} singleStreamVisualProof={} updatedStreamVisualProofSide={} visibleCameraProjectionReady=true sceneOwnedPanel=true projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 {} nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review legacyPanelTargetDefaults=deprecated panelTargetFields=runtime diagnosticVisualLayer=none depthClip=false environmentDepthClip=false visualInspection=required visualReleaseAccepted=false",
        yuv_mode,
        yuv_mode,
        yuv_mode,
        broker_h264_surface_texture,
        single_stream_visual_proof,
        updated_stream_visual_proof_side,
        homography_marker_fields,
    )
}

#[allow(clippy::too_many_arguments)]
fn projection_enumerated_marker_fields(
    source_count: usize,
    format_count: usize,
    projection_mapping_ready: bool,
    projection_metadata_ready: bool,
    pose_source: &str,
    source_eye_mapping: &str,
    coordinate_chain: &str,
    homography_marker_fields: &str,
    left_source_index: usize,
    right_source_index: usize,
    source_binding_mode: &str,
    left_source_class: &str,
    right_source_class: &str,
    left_width: usize,
    left_height: usize,
    right_width: usize,
    right_height: usize,
    fallback_reason: &str,
) -> String {
    format!(
        "phase=enumerated status=ok makepadSourceCount={} makepadFormatCount={} pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} {} leftSourceIndex={} rightSourceIndex={} sourceBindingMode={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} fallbackReason={}",
        source_count,
        format_count,
        projection_mapping_ready,
        projection_metadata_ready,
        pose_source,
        source_eye_mapping,
        coordinate_chain,
        homography_marker_fields,
        left_source_index,
        right_source_index,
        source_binding_mode,
        left_source_class,
        right_source_class,
        left_width,
        left_height,
        right_width,
        right_height,
        fallback_reason,
    )
}

fn single_stream_proof_wait_marker_fields(
    left_updated: bool,
    right_updated: bool,
    left_yuv_ready: bool,
    right_yuv_ready: bool,
    projection_mapping_ready: bool,
    visible_projection_ready: bool,
    updated_stream_visual_proof_side: &str,
) -> String {
    format!(
        "phase=single-stream-proof status=waiting pairedLeftRightCameraFrames=false singleStreamCameraPixels=true leftUpdated={} rightUpdated={} leftYuvReady={} rightYuvReady={} projectionMappingReady={} alignedProjection=false visibleCameraProjectionReady={} sceneOwnedPanel=true projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review legacyPanelTargetDefaults=deprecated panelTargetFields=runtime diagnosticVisualLayer=none neutralWaitingPanel=true depthClip=false environmentDepthClip=false drawVarsTextureRedraw=true shaderAreaStateUpdate=true updatedStreamVisualProofSide={} visualInspection=required visualReleaseAccepted=false fallbackReason=waiting_for_second_cpu_yuv_stream",
        left_updated,
        right_updated,
        left_yuv_ready,
        right_yuv_ready,
        projection_mapping_ready,
        visible_projection_ready,
        updated_stream_visual_proof_side,
    )
}

fn stereo_comparison_marker_line(
    pair: &MakepadCameraPair,
    inputs: &MakepadStereoComparisonMarkerInputs<'_>,
    homography_marker_fields: &str,
) -> String {
    stereo_comparison_marker_line_fields(
        StereoComparisonPairFields {
            left_source_index: pair.left.source_index,
            right_source_index: pair.right.source_index,
            source_eye_mapping: &pair.source_eye_mapping,
        },
        inputs,
        homography_marker_fields,
    )
}

struct StereoComparisonPairFields<'a> {
    left_source_index: usize,
    right_source_index: usize,
    source_eye_mapping: &'a str,
}

fn stereo_comparison_marker_line_fields(
    pair: StereoComparisonPairFields<'_>,
    inputs: &MakepadStereoComparisonMarkerInputs<'_>,
    homography_marker_fields: &str,
) -> String {
    format!(
        "RUSTY_XR_MAKEPAD_STEREO_COMPARISON schema=rusty.xr.makepad-stereo-comparison.v1 phase={} profile={} comparisonBaseline={} cameraTier={} acquisition={} transport={} projectionMode={} syntheticScene={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} sourceEyeMapping={} projectionScale={:.2} xrRenderScale={:.2} pairedLeftRightCameraFrames=true alignedProjection={} visibleCameraProjectionReady={} renderPath=makepad-xr projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 colorReference=android-yuv420-888-plane-order perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 {} makepadForkBranch={} makepadForkCommit={} nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review legacyPanelTargetDefaults=deprecated panelTargetFields=runtime diagnosticVisualLayer=none neutralWaitingPanel=true visualIsolation=s118_projected_footprint_solid_red_exterior depthClip=false environmentDepthClip=false cpuUploadPath=makepad-camera-cpu-yuv-plane drawVarsTextureRedraw=true shaderAreaStateUpdate=true visualInspection=required visualReleaseAccepted=false",
        inputs.phase,
        inputs.runtime_profile,
        inputs.comparison_baseline,
        inputs.camera_tier,
        inputs.acquisition_profile,
        inputs.transport_profile,
        inputs.projection_mode,
        inputs.synthetic_scene,
        pair.left_source_index,
        pair.right_source_index,
        pair.source_eye_mapping,
        inputs.projection_scale,
        inputs.xr_render_scale,
        inputs.aligned_projection,
        inputs.visible_projection_ready,
        homography_marker_fields,
        inputs.makepad_fork_branch,
        inputs.makepad_fork_commit,
    )
}

fn paired_projection_progress_marker_fields(
    phase: &str,
    left_prepared: bool,
    right_prepared: bool,
    left_updated: bool,
    right_updated: bool,
    projection_mapping_ready: bool,
    projection_metadata_ready: bool,
    pose_source: &str,
    source_eye_mapping: &str,
    homography_marker_fields: &str,
    left_source_index: usize,
    right_source_index: usize,
    fallback_reason: &str,
) -> String {
    format!(
        "phase={} status=progress leftPrepared={} rightPrepared={} leftUpdated={} rightUpdated={} pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} {} leftSourceIndex={} rightSourceIndex={} fallbackReason={}",
        phase,
        left_prepared,
        right_prepared,
        left_updated,
        right_updated,
        projection_mapping_ready,
        projection_metadata_ready,
        pose_source,
        source_eye_mapping,
        homography_marker_fields,
        left_source_index,
        right_source_index,
        fallback_reason,
    )
}

fn projection_start_marker_fields(
    projection_mapping_ready: bool,
    projection_metadata_ready: bool,
    pose_source: &str,
    source_eye_mapping: &str,
    coordinate_chain: &str,
    homography_marker_fields: &str,
    left_source_index: usize,
    right_source_index: usize,
    projection_mode: &str,
    projection_scale: f64,
    xr_render_scale: f64,
    fallback_reason: &str,
) -> String {
    format!(
        "phase=start status=started pairedLeftRightGpuBuffers=false projectionMappingReady={} alignedProjection=false projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} {} leftSourceIndex={} rightSourceIndex={} projectionMode={} projectionScale={:.2} xrRenderScale={:.2} fallbackReason={}",
        projection_mapping_ready,
        projection_metadata_ready,
        pose_source,
        source_eye_mapping,
        coordinate_chain,
        homography_marker_fields,
        left_source_index,
        right_source_index,
        projection_mode,
        projection_scale,
        xr_render_scale,
        fallback_reason,
    )
}

struct CompleteMarkerFields<'a> {
    paired_streams_ready: bool,
    broker_h264_surface_texture: bool,
    projection_mapping_ready: bool,
    aligned_projection: bool,
    visible_projection_ready: bool,
    projection_metadata_ready: bool,
    pose_source: &'a str,
    source_eye_mapping: &'a str,
    coordinate_chain: &'a str,
    projection_mode: &'a str,
    left_source_index: usize,
    right_source_index: usize,
    left_source_class: &'a str,
    right_source_class: &'a str,
    left_width: usize,
    left_height: usize,
    right_width: usize,
    right_height: usize,
    left_rotation_steps: f32,
    right_rotation_steps: f32,
    projection_scale: f64,
    xr_render_scale: f64,
    homography_marker_fields: &'a str,
    cpu_upload_path: &'a str,
    fallback_reason: &'a str,
}

fn complete_marker_fields(fields: CompleteMarkerFields<'_>) -> String {
    format!(
        "phase=complete status=ok pairedLeftRightCameraFrames={} brokerH264SurfaceTexture={} makepadVulkanImport=false projectionMappingReady={} alignedProjection={} visibleCameraProjectionReady={} projectionMetadataReady={} poseSource={} sourceEyeMapping={} coordinateChain={} projectionMode={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftRotationSteps={:.0} rightRotationSteps={:.0} projectionScale={:.2} xrRenderScale={:.2} renderPath=makepad-xr projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 {} nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review legacyPanelTargetDefaults=deprecated panelTargetFields=runtime cpuUploadPath={} diagnosticVisualLayer=none neutralWaitingPanel=true visualIsolation=s118_projected_footprint_solid_red_exterior depthClip=false environmentDepthClip=false drawVarsTextureRedraw=true shaderAreaStateUpdate=true visualInspection=required visualReleaseAccepted=false fallbackReason={}",
        fields.paired_streams_ready,
        fields.broker_h264_surface_texture,
        fields.projection_mapping_ready,
        fields.aligned_projection,
        fields.visible_projection_ready,
        fields.projection_metadata_ready,
        fields.pose_source,
        fields.source_eye_mapping,
        fields.coordinate_chain,
        fields.projection_mode,
        fields.left_source_index,
        fields.right_source_index,
        fields.left_source_class,
        fields.right_source_class,
        fields.left_width,
        fields.left_height,
        fields.right_width,
        fields.right_height,
        fields.left_rotation_steps,
        fields.right_rotation_steps,
        fields.projection_scale,
        fields.xr_render_scale,
        fields.homography_marker_fields,
        fields.cpu_upload_path,
        fields.fallback_reason,
    )
}

fn visible_panel_bound_marker_fields(
    source_eye_mapping: &str,
    left_source_index: usize,
    right_source_index: usize,
    left_rotation_steps: f32,
    right_rotation_steps: f32,
    single_stream_visual_proof: bool,
    updated_stream_visual_proof_side: &str,
    homography_marker_fields: &str,
) -> String {
    format!(
        "phase=visible-panel-bound status=ok visibleCameraProjectionReady=true eyeSelection=per-eye-direct-camera-yuv-color-limited601-noswap-border sourceEyeMapping={} leftEyeSource=makepad-camera-source-{} rightEyeSource=makepad-camera-source-{} leftRotationSteps={:.0} rightRotationSteps={:.0} sceneOwnedPanel=true projectionShaderPath=makepad-full-frame-source-display-row-vertical-uv textureProbeMode=single-quad-target-screen-uv syntheticLumaSlotProof=false directCameraYuvColorAccepted=false directCameraYuvColorSwapUv=false colorConversion=per-eye-yuv-noswap-limited-bt601 colorReference=android-yuv420-888-plane-order perEyeTextureSelection=true activeEyeSelector=xr_view_id sourceEyeSelector=display_source_eye_mapping projectionPanelPlacement=single-quad-fullscreen-target-screen-uv s62VisiblePanelBaseline=true s67bBasePassthroughOffPanel=true s68ActiveEyeNonWorldPanelPlacement=true s69SourceEyeSwap=true s69bHorizontalMirrorFix=false s70SquareAspectFix=true s72HeadCenteredSquareRestored=true s72MetadataUvBaselineCorrection=true s73ScalarHomographyBinding=true s74LiteralHomographyRows=false s75DynamicHomographyBinding=false s76DirectDrawVarsHomography=true s77SourceUvValidityFallback=true s78ClipSpaceSurfaceHomography=true s79TargetSourceEyeMapping=false s80FullViewContentUvScale=false s81DynamicScreenSurfaceUv=false s82CollapsedScreenToCameraHomography=false s83DrawPassProjectionInverseHomography=false s84ProjectionInverseNearFarFallback=false s85ForcedScreenToCameraFallback=false s86DirectYuvFullscreenControl=false s87RuntimeXrViewHomography=true s88SourceValidityFallback=true s89SingleQuadTargetScreenUv=true s90CameraIdSourceBinding=true s91ProjectionMathCorrection=true s91ConfigurableSourceEyeSelector=true s91DisplayIndexedHomographyRows=true s91VerticalOnlyTextureUv=true contentUvScale=1.6000 projectionUvCorrection=runtime-openxr-view-screen-to-camera-homography-configured-source-display-row-vertical-uv displayEyeOffsetMeters=0.032 displayFovSource=makepad_xr_update_runtime_openxr_view displayAspect=1.00 {} nativePassthroughStaticMarker=deprecated s98NativePassthroughHudSplitStaticMarker=deprecated s109SolidRedProjectionExterior=true s118ProjectedFootprintLiveWindow=true backgroundClearColor=203040 diagnosticUvTransform=see-source-sampling diagnosticUvRotation=0 diagnosticHorizontalMirrorCorrected=requires-visual-review legacyPanelTargetDefaults=deprecated panelTargetFields=runtime diagnosticVisualLayer=none neutralWaitingPanel=true visualIsolation=s118_projected_footprint_solid_red_exterior depthClip=false environmentDepthClip=false singleStreamVisualProof={} updatedStreamVisualProofSide={} cpuUploadPath=makepad-camera-cpu-yuv-plane drawVarsTextureRedraw=true shaderAreaStateUpdate=true visualInspection=required visualReleaseAccepted=false",
        source_eye_mapping,
        left_source_index,
        right_source_index,
        left_rotation_steps,
        right_rotation_steps,
        homography_marker_fields,
        single_stream_visual_proof,
        updated_stream_visual_proof_side,
    )
}

#[cfg(target_os = "android")]
pub(crate) fn broker_projection_plan_marker_fields(
    pair: &MakepadCameraPair,
    plan: &Camera2StereoPlan,
    left_metadata: &BrokerH264ProjectionMetadata,
    right_metadata: &BrokerH264ProjectionMetadata,
) -> String {
    format!(
        "phase=broker-h264-projection-plan status=ok projectionMetadataReady={} runtimeXrViewStateReady={} poseSource={} poseCoordinateConvention={} sourceEyeMapping={} sourceBindingMode={} coordinateChain={} projection_profile={} geometry_profile={} leftCameraId={} rightCameraId={} width={} height={} leftMetadataBytes={} rightMetadataBytes={} leftMetadataSource={} rightMetadataSource={} leftProjectionGeometryProfile={} rightProjectionGeometryProfile={} leftSourceValidUvRect={} rightSourceValidUvRect={} leftSyntheticPattern={} rightSyntheticPattern={} leftOrientationKind={} rightOrientationKind={} leftRasterOrientation={} rightRasterOrientation={} leftUprightMarker={} rightUprightMarker={} leftOrientationMetadataSource={} rightOrientationMetadataSource={} leftOrientationDefault={} rightOrientationDefault={} leftStimulusRasterOrientation={} rightStimulusRasterOrientation={} leftStimulusUprightMarker={} rightStimulusUprightMarker={} {} {}",
        pair.projection_metadata_ready,
        pair.runtime_xr_view_state_ready,
        marker_token(&pair.pose_source),
        marker_token(&left_metadata.pose_coordinate_convention),
        marker_token(&pair.source_eye_mapping),
        marker_token(&pair.source_binding_mode),
        marker_token(&pair.coordinate_chain),
        marker_token(&left_metadata.synthetic_projection_profile),
        marker_token(&left_metadata.projection_geometry_profile),
        marker_token(&plan.left_camera_id),
        marker_token(&plan.right_camera_id),
        plan.width,
        plan.height,
        left_metadata.metadata_bytes,
        right_metadata.metadata_bytes,
        marker_token(&left_metadata.source),
        marker_token(&right_metadata.source),
        marker_token(&left_metadata.projection_geometry_profile),
        marker_token(&right_metadata.projection_geometry_profile),
        uv_rect_token(rect_xywh(left_metadata.source_valid_uv_rect)),
        uv_rect_token(rect_xywh(right_metadata.source_valid_uv_rect)),
        marker_token(&left_metadata.synthetic_pattern),
        marker_token(&right_metadata.synthetic_pattern),
        marker_token(&left_metadata.orientation_kind),
        marker_token(&right_metadata.orientation_kind),
        marker_token(&left_metadata.raster_orientation),
        marker_token(&right_metadata.raster_orientation),
        marker_token(&left_metadata.upright_marker),
        marker_token(&right_metadata.upright_marker),
        marker_token(&left_metadata.orientation_metadata_source),
        marker_token(&right_metadata.orientation_metadata_source),
        left_metadata.orientation_default,
        right_metadata.orientation_default,
        marker_token(&left_metadata.stimulus_raster_orientation),
        marker_token(&right_metadata.stimulus_raster_orientation),
        marker_token(&left_metadata.stimulus_upright_marker),
        marker_token(&right_metadata.stimulus_upright_marker),
        broker_pair_content_geometry_marker_fields(left_metadata, right_metadata),
        projection_homography_marker_fields(pair),
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

fn homography_token(rows: [[f32; 3]; 3]) -> String {
    rows.iter()
        .flat_map(|row| row.iter())
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",")
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

#[cfg(test)]
mod tests {
    use super::{
        complete_marker_fields, draw_vars_bound_marker_fields,
        paired_projection_progress_marker_fields, projection_enumerated_marker_fields,
        projection_start_marker_fields, single_stream_proof_wait_marker_fields,
        stereo_comparison_marker_line_fields, visible_panel_bound_marker_fields,
        CompleteMarkerFields, MakepadStereoComparisonMarkerInputs, StereoComparisonPairFields,
    };

    #[test]
    fn draw_vars_bound_marker_keeps_projection_contract_shape() {
        let fields = draw_vars_bound_marker_fields(
            true,
            false,
            true,
            "left",
            "projectionHomographyReady=true runtimeXrViewStateReady=true",
        );

        assert!(fields.starts_with(
            "phase=draw-vars-bound status=ok cameraReady=true yuvMode=true"
        ));
        assert!(fields.contains(
            "leftYuvTextureBound=true rightYuvTextureBound=true brokerH264SurfaceTexture=false"
        ));
        assert!(fields
            .contains("singleStreamVisualProof=true updatedStreamVisualProofSide=left"));
        assert!(fields.contains("projectionHomographyReady=true runtimeXrViewStateReady=true"));
        assert!(fields.ends_with("visualInspection=required visualReleaseAccepted=false"));
    }

    #[test]
    fn visible_panel_bound_marker_keeps_projection_contract_shape() {
        let fields = visible_panel_bound_marker_fields(
            "left-right",
            0,
            1,
            90.0,
            270.0,
            false,
            "paired",
            "projectionHomographyReady=true runtimeXrViewStateReady=true",
        );

        assert!(fields.starts_with(
            "phase=visible-panel-bound status=ok visibleCameraProjectionReady=true"
        ));
        assert!(fields.contains(
            "sourceEyeMapping=left-right leftEyeSource=makepad-camera-source-0 rightEyeSource=makepad-camera-source-1"
        ));
        assert!(fields.contains("leftRotationSteps=90 rightRotationSteps=270"));
        assert!(fields.contains("projectionHomographyReady=true runtimeXrViewStateReady=true"));
        assert!(fields
            .contains("singleStreamVisualProof=false updatedStreamVisualProofSide=paired"));
        assert!(fields.ends_with("visualInspection=required visualReleaseAccepted=false"));
    }

    #[test]
    fn complete_marker_keeps_projection_contract_shape() {
        let fields = complete_marker_fields(CompleteMarkerFields {
            paired_streams_ready: true,
            broker_h264_surface_texture: false,
            projection_mapping_ready: true,
            aligned_projection: true,
            visible_projection_ready: true,
            projection_metadata_ready: true,
            pose_source: "camera2-openxr-view",
            source_eye_mapping: "left-right",
            coordinate_chain: "camera2-to-shader-surface",
            projection_mode: "camera",
            left_source_index: 0,
            right_source_index: 1,
            left_source_class: "back",
            right_source_class: "back",
            left_width: 1280,
            left_height: 1280,
            right_width: 1280,
            right_height: 1280,
            left_rotation_steps: 90.0,
            right_rotation_steps: 270.0,
            projection_scale: 1.25,
            xr_render_scale: 1.5,
            homography_marker_fields: "projectionHomographyReady=true runtimeXrViewStateReady=true",
            cpu_upload_path: "makepad-camera-cpu-yuv-plane",
            fallback_reason: "none",
        });

        assert!(fields.starts_with(
            "phase=complete status=ok pairedLeftRightCameraFrames=true"
        ));
        assert!(fields.contains(
            "projectionMappingReady=true alignedProjection=true visibleCameraProjectionReady=true"
        ));
        assert!(fields.contains(
            "projectionMode=camera leftEyeSource=makepad-camera-source-0 rightEyeSource=makepad-camera-source-1"
        ));
        assert!(fields.contains("leftRotationSteps=90 rightRotationSteps=270"));
        assert!(fields.contains("projectionScale=1.25 xrRenderScale=1.50"));
        assert!(fields.contains("projectionHomographyReady=true runtimeXrViewStateReady=true"));
        assert!(fields.contains("cpuUploadPath=makepad-camera-cpu-yuv-plane"));
        assert!(fields.ends_with("fallbackReason=none"));
    }

    #[test]
    fn projection_start_marker_keeps_projection_contract_shape() {
        let fields = projection_start_marker_fields(
            true,
            true,
            "camera2-openxr-view",
            "left-right",
            "camera2-to-shader-surface",
            "projectionHomographyReady=true runtimeXrViewStateReady=true",
            0,
            1,
            "camera",
            1.25,
            1.5,
            "none",
        );

        assert!(fields.starts_with(
            "phase=start status=started pairedLeftRightGpuBuffers=false"
        ));
        assert!(fields.contains(
            "projectionMappingReady=true alignedProjection=false projectionMetadataReady=true"
        ));
        assert!(fields.contains("poseSource=camera2-openxr-view sourceEyeMapping=left-right"));
        assert!(fields.contains("projectionHomographyReady=true runtimeXrViewStateReady=true"));
        assert!(fields.contains("projectionMode=camera projectionScale=1.25 xrRenderScale=1.50"));
        assert!(fields.ends_with("fallbackReason=none"));
    }

    #[test]
    fn paired_projection_progress_marker_keeps_projection_contract_shape() {
        let fields = paired_projection_progress_marker_fields(
            "texture-updated",
            true,
            false,
            true,
            false,
            true,
            true,
            "camera2-openxr-view",
            "left-right",
            "projectionHomographyReady=true runtimeXrViewStateReady=true",
            0,
            1,
            "waiting_for_right",
        );

        assert!(fields.starts_with(
            "phase=texture-updated status=progress leftPrepared=true rightPrepared=false"
        ));
        assert!(fields.contains("leftUpdated=true rightUpdated=false"));
        assert!(fields.contains(
            "projectionMappingReady=true alignedProjection=false projectionMetadataReady=true"
        ));
        assert!(fields.contains("poseSource=camera2-openxr-view sourceEyeMapping=left-right"));
        assert!(fields.contains("projectionHomographyReady=true runtimeXrViewStateReady=true"));
        assert!(fields.ends_with(
            "leftSourceIndex=0 rightSourceIndex=1 fallbackReason=waiting_for_right"
        ));
    }

    #[test]
    fn single_stream_proof_wait_marker_keeps_projection_contract_shape() {
        let fields =
            single_stream_proof_wait_marker_fields(true, false, true, false, true, true, "left");

        assert!(fields.starts_with(
            "phase=single-stream-proof status=waiting pairedLeftRightCameraFrames=false"
        ));
        assert!(fields.contains(
            "singleStreamCameraPixels=true leftUpdated=true rightUpdated=false"
        ));
        assert!(fields.contains("leftYuvReady=true rightYuvReady=false"));
        assert!(fields.contains(
            "projectionMappingReady=true alignedProjection=false visibleCameraProjectionReady=true"
        ));
        assert!(fields.contains("updatedStreamVisualProofSide=left"));
        assert!(fields.ends_with("fallbackReason=waiting_for_second_cpu_yuv_stream"));
    }

    #[test]
    fn projection_enumerated_marker_keeps_projection_contract_shape() {
        let fields = projection_enumerated_marker_fields(
            2,
            4,
            true,
            true,
            "camera2-openxr-view",
            "left-right",
            "camera2-to-shader-surface",
            "projectionHomographyReady=true runtimeXrViewStateReady=true",
            0,
            1,
            "camera2-direct",
            "back",
            "back",
            1280,
            1280,
            1280,
            1280,
            "none",
        );

        assert!(fields.starts_with(
            "phase=enumerated status=ok makepadSourceCount=2 makepadFormatCount=4"
        ));
        assert!(fields.contains(
            "projectionMappingReady=true alignedProjection=false projectionMetadataReady=true"
        ));
        assert!(fields.contains("projectionHomographyReady=true runtimeXrViewStateReady=true"));
        assert!(fields.contains("sourceBindingMode=camera2-direct leftSourceClass=back"));
        assert!(fields.ends_with("fallbackReason=none"));
    }

    #[test]
    fn stereo_comparison_marker_keeps_projection_contract_shape() {
        let inputs = MakepadStereoComparisonMarkerInputs {
            phase: "paired-projection-ready",
            runtime_profile: "fast-visual",
            comparison_baseline: "canvas-custom",
            camera_tier: "direct-camera",
            acquisition_profile: "camera2",
            transport_profile: "cpu-yuv",
            projection_mode: "camera",
            synthetic_scene: "hidden",
            projection_scale: 1.25,
            xr_render_scale: 1.5,
            aligned_projection: true,
            visible_projection_ready: true,
            makepad_fork_branch: "branch",
            makepad_fork_commit: "commit",
        };
        let fields = stereo_comparison_marker_line_fields(
            StereoComparisonPairFields {
                left_source_index: 0,
                right_source_index: 1,
                source_eye_mapping: "left-right",
            },
            &inputs,
            "projectionHomographyReady=true runtimeXrViewStateReady=true",
        );

        assert!(fields.starts_with(
            "RUSTY_XR_MAKEPAD_STEREO_COMPARISON schema=rusty.xr.makepad-stereo-comparison.v1"
        ));
        assert!(fields.contains("phase=paired-projection-ready profile=fast-visual"));
        assert!(fields.contains("projectionMode=camera syntheticScene=hidden"));
        assert!(fields.contains("projectionScale=1.25 xrRenderScale=1.50"));
        assert!(fields.contains("projectionHomographyReady=true runtimeXrViewStateReady=true"));
        assert!(fields.contains("makepadForkBranch=branch makepadForkCommit=commit"));
        assert!(fields.ends_with("visualInspection=required visualReleaseAccepted=false"));
    }
}
