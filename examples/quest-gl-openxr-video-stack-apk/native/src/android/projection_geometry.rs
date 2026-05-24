use openxr as xr;
use rusty_xr_camera_model::{
    FeedPlacementDescriptor, PerEyeVideoProjectionPlan, Rect2, Vec2, VideoProjectionMapping,
};
use rusty_xr_contracts::{
    Eye, InvalidProjectionFillPolicy, ProjectionFootprintRowSpan, ProjectionFootprintSummary,
    ProjectionGuideDomain,
};

use super::{OesContentMappingMode, OesProjectionAlphaMode, OesProjectionBorderPolicy};

const PROJECTION_FOOTPRINT_GRID: usize = 64;

#[derive(Clone, Debug)]
pub(super) struct OesProjectionPlan {
    pub(super) left: OesEyeProjection,
    pub(super) right: OesEyeProjection,
}

impl OesProjectionPlan {
    pub(super) fn eye(&self, view_index: usize) -> Option<&OesEyeProjection> {
        match view_index {
            0 => Some(&self.left),
            1 => Some(&self.right),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct OesEyeProjection {
    pub(super) eye: Eye,
    pub(super) surface_to_screen_h: [[f32; 3]; 3],
    pub(super) screen_to_surface_h: [[f32; 3]; 3],
    pub(super) surface_to_camera_h: [[f32; 3]; 3],
    pub(super) screen_to_camera_h: [[f32; 3]; 3],
    pub(super) source_label: String,
    pub(super) source_eye: String,
    pub(super) use_surface_texture_transform: bool,
    pub(super) content_mapping_mode: OesContentMappingMode,
    pub(super) geometry_plan: PerEyeVideoProjectionPlan,
}

pub(super) fn projected_footprint_summary(
    projection: &OesEyeProjection,
    projection_border_policy: OesProjectionBorderPolicy,
    frame_count: u64,
    source_sequence: u64,
) -> ProjectionFootprintSummary {
    let footprint_plan = &projection.geometry_plan.source_valid_screen_uv_footprint;
    let mut footprint = ProjectionFootprintSummary::new(
        "rusty_xr_gl_oes",
        format!("public_projected_camera_uv_{}", eye_label(projection.eye)),
    )
    .with_active_fraction(footprint_plan.active_fraction)
    .with_bbox_fraction(footprint_plan.bbox_ltrb())
    .with_invalid_fill_policy(projection_border_policy.invalid_source_uv_fill_policy())
    .with_guide_domain(ProjectionGuideDomain::ScreenCamera)
    .with_explicit_valid_mask(false)
    .with_note(format!(
        "Metadata-derived camera-UV valid footprint from broker stream-header and OpenXR view state at frame {frame_count}, source sequence {source_sequence}; contentMappingMode={}."
        , projection.content_mapping_mode.stable_id()
    ));

    for row in &footprint_plan.row_spans {
        footprint = if let Some((x0, x1)) = row.span {
            footprint.with_row_span(
                ProjectionFootprintRowSpan::new(row.row_y, row.active_fraction).with_span(x0, x1),
            )
        } else {
            footprint.with_row_span(ProjectionFootprintRowSpan::new(row.row_y, 0.0))
        };
    }
    footprint
}

pub(super) fn raw_copy_footprint_summary(frame_count: u64) -> ProjectionFootprintSummary {
    ProjectionFootprintSummary::new("rusty_xr_gl_oes", "public_raw_oes_full_surface")
        .with_active_fraction(1.0)
        .with_bbox_fraction([0.0, 0.0, 1.0, 1.0])
        .with_row_span(ProjectionFootprintRowSpan::new(0.0, 1.0).with_span(0.0, 1.0))
        .with_row_span(ProjectionFootprintRowSpan::new(0.5, 1.0).with_span(0.0, 1.0))
        .with_row_span(ProjectionFootprintRowSpan::new(1.0, 1.0).with_span(0.0, 1.0))
        .with_invalid_fill_policy(InvalidProjectionFillPolicy::NotApplicable)
        .with_guide_domain(ProjectionGuideDomain::SubmittedSurface)
        .with_explicit_valid_mask(true)
        .with_note(format!(
            "Full-surface public raw OES copy into the OpenXR GLES swapchain at frame {frame_count}."
        ))
}

pub(super) fn array_rect_xywh(rect: [f32; 4]) -> Rect2 {
    Rect2::new(Vec2::new(rect[0], rect[1]), Vec2::new(rect[2], rect[3]))
}

pub(super) fn shared_projection_mapping(mode: OesContentMappingMode) -> VideoProjectionMapping {
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

pub(super) fn expected_source_valid_footprint_fields(projection: &OesEyeProjection) -> String {
    let rect = screen_uv_rect_token(expected_source_valid_screen_uv_rect(projection));
    let eye_prefix = match projection.eye {
        Eye::Left => "left",
        Eye::Right => "right",
        Eye::Mono => "mono",
    };
    format!(
        "expectedSourceValidFootprintSource=renderer-authored expectedSourceValidFootprintStage=screen_to_camera_source_uv_bounds expectedSourceValidFootprintCoordinateSpace=display-eye-screen-uv expectedSourceValidFootprintMethod=renderer-grid-sampled-source-uv-validity expectedSourceValidFootprintRectSemantics=xywh {eye_prefix}ExpectedSourceValidScreenUvRect={rect} {}",
        projection.geometry_plan.marker_fields(eye_prefix)
    )
}

fn expected_source_valid_screen_uv_rect(projection: &OesEyeProjection) -> [f32; 4] {
    projection
        .geometry_plan
        .source_valid_screen_uv_footprint
        .bbox_xywh()
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

fn eye_label(eye: Eye) -> &'static str {
    match eye {
        Eye::Left => "left",
        Eye::Right => "right",
        Eye::Mono => "mono",
    }
}
