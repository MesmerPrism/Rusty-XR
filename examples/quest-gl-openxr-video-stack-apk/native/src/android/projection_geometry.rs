use openxr as xr;
use rusty_xr_camera_model::{
    camera_basis_from_camera2_reference_pose_relative_to_center,
    head_anchored_preview_surface_corners, invert_homography, screen_to_camera_uv_homography,
    surface_to_camera_uv_homography, surface_to_eye_screen_uv_homography, CameraBasis,
    CameraIntrinsics, FeedPlacementDescriptor, ImageSize, PerEyeVideoProjectionPlan, Quat, Rect2,
    TrackingBasis, Vec2, Vec3, VideoProjectionMapping,
};
use rusty_xr_contracts::{
    Eye, InvalidProjectionFillPolicy, ProjectionFootprintRowSpan, ProjectionFootprintSummary,
    ProjectionGuideDomain, ProjectionStageKind, ProjectionStageTokenRow,
};

use super::{
    gl_format_label, log_error, log_info,
    openxr_gles_config::{
        OesCameraProjectionMode, OesColorControls, OesContentMappingMode, OesProjectionAlphaMode,
        OesProjectionBorderPolicy, OesProjectionRuntimeState, OesSourceColorTransfer,
    },
    source_metadata::{
        projection_source_label, projection_surface_aspect_from_metadata, OesProjectionMetadata,
    },
    GL_SRGB8_ALPHA8, OES_COPY_RENDER_PATH, OES_PROJECTED_RENDER_PATH,
};

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

impl OesEyeProjection {
    pub(super) fn source_transform_for_sample(
        &self,
        surface_texture_transform: [f32; 16],
    ) -> [f32; 16] {
        if self.use_surface_texture_transform {
            surface_texture_transform
        } else {
            identity_texture_transform()
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OesSourceColorContract<'a> {
    pub(super) input_encoding: &'a str,
    pub(super) transform: &'a str,
    pub(super) transform_applied: bool,
    pub(super) output_encoding: &'a str,
    pub(super) swapchain_color_format: &'a str,
    pub(super) swapchain_color_encoding: &'a str,
}

pub(super) struct OesProjectionFrameContext {
    pub(super) projection_plan: Option<OesProjectionPlan>,
    pub(super) openxr_projection_fields: String,
    pub(super) projection_area_target_fields: String,
}

pub(super) fn projection_frame_context_from_state(
    reference_space_label: &str,
    predicted_display_time: xr::Time,
    views: &[xr::View],
    projection_state: OesProjectionRuntimeState,
    metadata_pair: Option<(&OesProjectionMetadata, &OesProjectionMetadata)>,
) -> OesProjectionFrameContext {
    OesProjectionFrameContext {
        projection_plan: metadata_pair.and_then(|(left, right)| {
            projection_plan_from_metadata_and_state(left, right, views, projection_state)
        }),
        openxr_projection_fields: openxr_projection_contract_fields(
            reference_space_label,
            predicted_display_time,
            views,
        ),
        projection_area_target_fields: projection_area_target_marker_fields_from_state(
            projection_state,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn projection_plan_from_metadata(
    left: &OesProjectionMetadata,
    right: &OesProjectionMetadata,
    views: &[xr::View],
    camera_projection_mode: OesCameraProjectionMode,
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
    let width = left.delivered_width.max(right.delivered_width);
    let height = left.delivered_height.max(right.delivered_height);
    if !left.projection_metadata_ready
        || !right.projection_metadata_ready
        || width == 0
        || height == 0
    {
        return None;
    }
    let metadata_backed_projection = left.has_metadata_backed_camera_projection()
        && right.has_metadata_backed_camera_projection();
    let camera_projection_mapping =
        left.requests_camera_projection_mapping() && right.requests_camera_projection_mapping();
    let full_frame_diagnostic_profile =
        left.is_full_frame_diagnostic_projection() && right.is_full_frame_diagnostic_projection();
    let explicit_full_frame_content_mapping = left.requests_explicit_full_frame_content_mapping()
        && right.requests_explicit_full_frame_content_mapping();
    if camera_projection_mode.uses_world_canvas() && metadata_backed_projection {
        camera2_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
        )
    } else if explicit_full_frame_content_mapping
        || camera_projection_mode.uses_world_canvas()
        || (full_frame_diagnostic_profile && !metadata_backed_projection)
    {
        broker_full_frame_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
        )
    } else if metadata_backed_projection
        && (camera_projection_mapping || full_frame_diagnostic_profile)
    {
        camera2_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
        )
    } else if left.requests_head_anchored_projection_area_mapping()
        && right.requests_head_anchored_projection_area_mapping()
    {
        broker_synthetic_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
        )
    } else if left.has_camera2_projection() && right.has_camera2_projection() {
        camera2_projection_plan_from_xr_views(
            left,
            right,
            width,
            height,
            views,
            projection_area_eye_offset_uv,
            projection_area_scale,
            projection_area_radius,
            projection_area_opacity,
            projection_border_policy,
            projection_border_opacity,
            projection_depth_meters,
            projection_preview_fov_y_degrees,
            projection_preview_offset_y_meters,
            projection_raw_overscan,
        )
    } else {
        None
    }
}

pub(super) fn projection_plan_from_metadata_and_state(
    left: &OesProjectionMetadata,
    right: &OesProjectionMetadata,
    views: &[xr::View],
    projection_state: OesProjectionRuntimeState,
) -> Option<OesProjectionPlan> {
    projection_plan_from_metadata(
        left,
        right,
        views,
        projection_state.camera_projection_mode,
        projection_state.projection_area_eye_offset_uv,
        projection_state.projection_area_scale,
        projection_state.projection_area_radius,
        projection_state.projection_area_opacity,
        projection_state.projection_border_policy,
        projection_state.projection_border_opacity,
        projection_state.tuning.projection_depth_meters,
        projection_state.tuning.camera_preview_fov_y_degrees,
        projection_state.tuning.camera_preview_offset_y_meters,
        projection_state.tuning.camera_raw_overlay_overscan,
    )
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

pub(super) fn projection_stage_rows(
    projection: Option<&OesEyeProjection>,
) -> [(ProjectionStageKind, [[f32; 3]; 3]); 4] {
    projection
        .map(|projection| {
            [
                (
                    ProjectionStageKind::SurfaceToScreen,
                    projection.surface_to_screen_h,
                ),
                (
                    ProjectionStageKind::ScreenToSurface,
                    projection.screen_to_surface_h,
                ),
                (
                    ProjectionStageKind::SurfaceToCamera,
                    projection.surface_to_camera_h,
                ),
                (
                    ProjectionStageKind::ScreenToCamera,
                    projection.screen_to_camera_h,
                ),
            ]
        })
        .unwrap_or([
            (ProjectionStageKind::SurfaceToScreen, identity_homography()),
            (ProjectionStageKind::ScreenToSurface, identity_homography()),
            (ProjectionStageKind::SurfaceToCamera, identity_homography()),
            (ProjectionStageKind::ScreenToCamera, identity_homography()),
        ])
}

pub(super) fn projection_source_contract_fields(
    projection: Option<&OesEyeProjection>,
    frame_count: u64,
    source_sequence: u64,
) -> String {
    projection
        .map(|projection| {
            format!(
                "{} {} source_eye={}:content_mapping={}:frame={frame_count}:source_sequence={source_sequence}",
                projection.source_label,
                expected_source_valid_footprint_fields(projection),
                projection.source_eye,
                projection.content_mapping_mode.stable_id()
            )
        })
        .unwrap_or_else(|| {
            format!("{OES_COPY_RENDER_PATH}:frame={frame_count}:source_sequence={source_sequence}")
        })
}

pub(super) fn projection_stage_source_label(
    projection: Option<&OesEyeProjection>,
    frame_count: u64,
    source_sequence: u64,
) -> String {
    projection
        .map(|projection| {
            format!(
                "{OES_PROJECTED_RENDER_PATH}:source_eye={}:frame={frame_count}:source_sequence={source_sequence}",
                projection.source_eye
            )
        })
        .unwrap_or_else(|| {
            format!("{OES_COPY_RENDER_PATH}:frame={frame_count}:source_sequence={source_sequence}")
        })
}

pub(super) fn source_sampling_projection_contract_log_message(
    source_contract_fields: &str,
) -> String {
    format!(
        "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase=source-sampling status=ready {} projectionHomographyReady=true projectionMappingReady=true visibleCameraProjectionReady=true",
        source_contract_fields.replace(':', " ")
    )
}

pub(super) fn projection_coordinate_contract_log_message(phase: &str, fields: &str) -> String {
    format!(
        "Rusty XR OpenXR GLES projection contract schema=rusty.xr.projection-coordinate-contract.v1 phase={} status=ready {}",
        phase, fields
    )
}

pub(super) fn source_color_contract_fields(fields: OesSourceColorContract<'_>) -> String {
    format!(
        "sourceColorInputEncoding={} sourceColorTransformStage=post_oes_sample_pre_camera_color_controls sourceColorTransform={} sourceColorTransformOwner=gles-oes-copy-shader sourceColorTransformApplied={} sourceColorOutputEncoding={} cameraColorControlStage=post_source_color_transfer swapchainColorFormat={} swapchainColorEncoding={}",
        fields.input_encoding,
        fields.transform,
        fields.transform_applied,
        fields.output_encoding,
        fields.swapchain_color_format,
        fields.swapchain_color_encoding
    )
}

pub(super) fn source_color_contract(
    camera_color_controls: OesColorControls,
    swapchain_color_format: u32,
) -> OesSourceColorContract<'static> {
    let transfer = camera_color_controls.source_transfer;
    OesSourceColorContract {
        input_encoding: transfer.input_encoding(),
        transform: transfer.stable_id(),
        transform_applied: transfer != OesSourceColorTransfer::Identity,
        output_encoding: transfer.output_encoding(),
        swapchain_color_format: gl_format_label(swapchain_color_format),
        swapchain_color_encoding: if swapchain_color_format == GL_SRGB8_ALPHA8 {
            "srgb"
        } else {
            "linear-or-runtime-default"
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn log_projection_diagnostics(
    view_index: usize,
    frame_count: u64,
    source_sequence: u64,
    projection: Option<&OesEyeProjection>,
    projection_border_policy: OesProjectionBorderPolicy,
    camera_color_controls: OesColorControls,
    swapchain_color_format: u32,
    openxr_projection_fields: &str,
    projection_area_target_fields: &str,
) {
    let source_color_fields = source_color_contract_fields(source_color_contract(
        camera_color_controls,
        swapchain_color_format,
    ));
    for message in projection_diagnostic_log_messages(
        view_index,
        frame_count,
        source_sequence,
        projection,
        projection_border_policy,
        &source_color_fields,
        openxr_projection_fields,
        projection_area_target_fields,
    ) {
        match message {
            Ok(line) => log_info(line),
            Err(error) => log_error(error),
        }
    }
}

pub(super) fn projection_stage_row_log_messages(
    eye: Eye,
    projection: Option<&OesEyeProjection>,
    frame_count: u64,
    source_sequence: u64,
) -> Vec<Result<String, String>> {
    let compact_source_label =
        projection_stage_source_label(projection, frame_count, source_sequence);
    projection_stage_rows(projection)
        .into_iter()
        .map(|(stage, rows)| {
            let row = ProjectionStageTokenRow::new("rusty_xr_gl_oes", eye, stage)
                .with_rows(rows)
                .with_source(compact_source_label.clone());
            serde_json::to_string(&row)
                .map(|json| format!("Rusty XR OpenXR GLES projection stage row {json}"))
                .map_err(|error| {
                    format!("Rusty XR OpenXR GLES projection stage serialization failed: {error}")
                })
        })
        .collect()
}

pub(super) fn projection_footprint_log_message(
    projection: Option<&OesEyeProjection>,
    projection_border_policy: OesProjectionBorderPolicy,
    frame_count: u64,
    source_sequence: u64,
) -> Result<String, String> {
    let footprint = projection
        .map(|projection| {
            projected_footprint_summary(
                projection,
                projection_border_policy,
                frame_count,
                source_sequence,
            )
        })
        .unwrap_or_else(|| raw_copy_footprint_summary(frame_count));
    serde_json::to_string(&footprint)
        .map(|json| format!("Rusty XR OpenXR GLES projection footprint {json}"))
        .map_err(|error| {
            format!("Rusty XR OpenXR GLES projection footprint serialization failed: {error}")
        })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn projection_diagnostic_log_messages(
    view_index: usize,
    frame_count: u64,
    source_sequence: u64,
    projection: Option<&OesEyeProjection>,
    projection_border_policy: OesProjectionBorderPolicy,
    source_color_fields: &str,
    openxr_projection_fields: &str,
    projection_area_target_fields: &str,
) -> Vec<Result<String, String>> {
    let Some(eye) = eye_from_view_index(view_index) else {
        return Vec::new();
    };
    let source_contract_fields =
        projection_source_contract_fields(projection, frame_count, source_sequence);
    let mut messages = vec![
        Ok(source_sampling_projection_contract_log_message(
            &source_contract_fields,
        )),
        Ok(projection_coordinate_contract_log_message(
            "source-color",
            source_color_fields,
        )),
        Ok(projection_coordinate_contract_log_message(
            "projection-plan",
            openxr_projection_fields,
        )),
        Ok(projection_coordinate_contract_log_message(
            "draw-vars-bound",
            projection_area_target_fields,
        )),
    ];
    messages.extend(projection_stage_row_log_messages(
        eye,
        projection,
        frame_count,
        source_sequence,
    ));
    messages.push(projection_footprint_log_message(
        projection,
        projection_border_policy,
        frame_count,
        source_sequence,
    ));
    messages
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

#[allow(clippy::too_many_arguments)]
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

pub(super) fn projection_area_target_marker_fields_from_state(
    state: OesProjectionRuntimeState,
) -> String {
    projection_area_target_marker_fields(
        state.projection_area_eye_offset_uv[0],
        state.projection_area_eye_offset_uv[1],
        state.projection_area_radius,
        state.projection_area_scale,
        state.projection_alpha_mode,
        state.projection_alpha_scale,
        state.projection_alpha_bias,
        state.tuning.projection_depth_meters,
        state.tuning.camera_preview_fov_y_degrees,
        state.tuning.camera_preview_offset_y_meters,
        state.tuning.camera_raw_overlay_overscan,
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
fn broker_synthetic_projection_plan_from_xr_views(
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
fn broker_full_frame_projection_plan_from_xr_views(
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
fn camera2_projection_plan_from_xr_views(
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

pub(super) const fn identity_homography() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

const fn identity_texture_transform() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
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

fn eye_from_view_index(view_index: usize) -> Option<Eye> {
    match view_index {
        0 => Some(Eye::Left),
        1 => Some(Eye::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_projection_diagnostic_log_wrappers_keep_shape() {
        let rows = projection_stage_row_log_messages(Eye::Left, None, 12, 34);
        assert_eq!(rows.len(), 4);
        let first = rows[0].as_ref().expect("stage row should serialize");
        assert!(first.starts_with("Rusty XR OpenXR GLES projection stage row {"));
        assert!(first.contains("\"backend\":\"rusty_xr_gl_oes\""));
        assert!(first.contains("rusty_xr_gl_oes:frame=12:source_sequence=34"));

        let footprint =
            projection_footprint_log_message(None, OesProjectionBorderPolicy::SolidRed, 12, 34)
                .expect("footprint should serialize");
        assert!(footprint.starts_with("Rusty XR OpenXR GLES projection footprint {"));
        assert!(footprint.contains("public_raw_oes_full_surface"));
        assert!(footprint.contains("Full-surface public raw OES copy"));
    }

    #[test]
    fn projection_diagnostic_log_messages_keep_contract_order() {
        let messages = projection_diagnostic_log_messages(
            0,
            12,
            34,
            None,
            OesProjectionBorderPolicy::SolidRed,
            "sourceColorTransform=identity",
            "referenceSpace=app-reference-space",
            "projectionAreaTargetSource=renderer-authored",
        );
        assert_eq!(messages.len(), 9);
        let rendered: Vec<String> = messages
            .into_iter()
            .map(|message| message.expect("diagnostic message should serialize"))
            .collect();
        assert!(rendered[0].contains("phase=source-sampling status=ready"));
        assert!(rendered[1].contains("phase=source-color status=ready"));
        assert!(rendered[2].contains("phase=projection-plan status=ready"));
        assert!(rendered[3].contains("phase=draw-vars-bound status=ready"));
        assert!(rendered[4].starts_with("Rusty XR OpenXR GLES projection stage row {"));
        assert!(rendered[8].starts_with("Rusty XR OpenXR GLES projection footprint {"));
    }

    #[test]
    fn source_color_contract_fields_keep_marker_shape() {
        assert_eq!(
            source_color_contract_fields(OesSourceColorContract {
                input_encoding: "external-oes-srgb-nonlinear-rgb",
                transform: "srgb-to-linear",
                transform_applied: true,
                output_encoding: "linear-rgb",
                swapchain_color_format: "GL_SRGB8_ALPHA8",
                swapchain_color_encoding: "srgb",
            }),
            "sourceColorInputEncoding=external-oes-srgb-nonlinear-rgb sourceColorTransformStage=post_oes_sample_pre_camera_color_controls sourceColorTransform=srgb-to-linear sourceColorTransformOwner=gles-oes-copy-shader sourceColorTransformApplied=true sourceColorOutputEncoding=linear-rgb cameraColorControlStage=post_source_color_transfer swapchainColorFormat=GL_SRGB8_ALPHA8 swapchainColorEncoding=srgb"
        );
    }
}
