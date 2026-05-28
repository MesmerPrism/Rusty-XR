use crate::{CameraProjectionMode, HeadsetCameraFrameDiagnostics, StereoGpuCameraFrame};
use rusty_xr_camera_model::{
    target_footprint_debug_region_marker_fields, TARGET_SCREEN_FOOTPRINT_SCHEMA,
};

use super::source_content_geometry::HwbStereoContentGeometry;

pub(super) fn projection_source_metadata_marker_fields(
    left: &HeadsetCameraFrameDiagnostics,
    right: &HeadsetCameraFrameDiagnostics,
    left_width: u32,
    left_height: u32,
    right_width: u32,
    right_height: u32,
    camera_projection_mode: CameraProjectionMode,
) -> String {
    let source = marker_token(
        left.source.as_deref().or(right.source.as_deref()),
        "unknown",
    );
    let source_mode = if source.contains("synthetic") {
        "broker-synthetic"
    } else if source.contains("broker") {
        "broker-h264"
    } else {
        "direct-camera2"
    };
    let geometry_profile = marker_token(
        left.projection_geometry_profile
            .as_deref()
            .or(left.synthetic_projection_profile.as_deref())
            .or(right.projection_geometry_profile.as_deref())
            .or(right.synthetic_projection_profile.as_deref()),
        fallback_projection_geometry_profile(camera_projection_mode),
    );
    let synthetic_pattern = marker_token(
        left.synthetic_pattern
            .as_deref()
            .or(right.synthetic_pattern.as_deref()),
        "unknown",
    );
    let orientation_kind = marker_token(
        left.orientation_kind
            .as_deref()
            .or(right.orientation_kind.as_deref()),
        "unknown",
    );
    let raster_orientation = marker_token(
        left.stimulus_raster_orientation
            .as_deref()
            .or(right.stimulus_raster_orientation.as_deref())
            .or(left.raster_orientation.as_deref())
            .or(right.raster_orientation.as_deref()),
        "unspecified",
    );
    let upright_marker = marker_token(
        left.stimulus_upright_marker
            .as_deref()
            .or(right.stimulus_upright_marker.as_deref())
            .or(left.upright_marker.as_deref())
            .or(right.upright_marker.as_deref()),
        "unspecified",
    );
    let orientation_metadata_source = marker_token(
        left.orientation_metadata_source
            .as_deref()
            .or(right.orientation_metadata_source.as_deref()),
        "missing",
    );
    let content_geometry = HwbStereoContentGeometry::from_diagnostics_pair(
        left,
        right,
        left_width,
        left_height,
        right_width,
        right_height,
    );
    let target_schema = marker_token(
        left.target_footprint_schema
            .as_deref()
            .or(right.target_footprint_schema.as_deref()),
        TARGET_SCREEN_FOOTPRINT_SCHEMA,
    );
    let target_coordinate_space = marker_token(
        left.target_coordinate_space
            .as_deref()
            .or(right.target_coordinate_space.as_deref()),
        "display-eye-screen-uv",
    );
    let target_clip_policy = marker_token(
        left.target_clip_policy
            .as_deref()
            .or(right.target_clip_policy.as_deref()),
        "clip-to-visible-eye",
    );
    let target_metadata_source = marker_token(
        left.target_footprint_metadata_source
            .as_deref()
            .or(right.target_footprint_metadata_source.as_deref()),
        "missing",
    );
    format!(
        "projectionMetadataReady=true source={} sourceMode={} brokerH264SourceMode={} sourceBindingMode=broker-h264-stream-header-{} brokerH264SyntheticProjectionProfile={} projection_profile={} geometry_profile={} syntheticPattern={} pattern={} orientationKind={} rasterOrientation={} uprightMarker={} orientationMetadataSource={} orientationDefault={} stimulusRasterOrientation={} stimulusUprightMarker={} stimulusOrientationDefault={} contentKind={} contentWidth={} contentHeight={} contentAspectRatio={} desiredDisplayAspectRatio={} desiredProjectionAspectRatio={} contentCoordinateSpace={} contentOrigin={} contentXAxis={} contentYAxis={} contentMappingIntent={} contentGeometryMetadataSource={} contentGeometryDefault={} contentUvRect={} sourceVisibleUvRect={} sourceCropRectState={} sourceCropRectOwner={} targetFootprintSchema={} targetCoordinateSpace={} leftTargetScreenUvRect={} rightTargetScreenUvRect={} targetClipPolicy={} targetFootprintMetadataSource={} targetFootprintDefault={} effectBoundary=target-footprint borderRegionSemantics=visible-render-surface-minus-target-footprint sourceInvalidSemantics=target-fragment-maps-outside-source-valid-uv {} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftContentWidth={} leftContentHeight={} rightContentWidth={} rightContentHeight={} leftContentUvRect={} rightContentUvRect={} leftSourceVisibleUvRect={} rightSourceVisibleUvRect={} leftSourceCropRectPx={} rightSourceCropRectPx={}",
        source,
        source_mode,
        source_mode,
        geometry_profile,
        geometry_profile,
        geometry_profile,
        geometry_profile,
        synthetic_pattern,
        synthetic_pattern,
        orientation_kind,
        raster_orientation,
        upright_marker,
        orientation_metadata_source,
        marker_bool(
            left.orientation_default.or(right.orientation_default),
            orientation_kind == "unknown",
        ),
        raster_orientation,
        upright_marker,
        marker_bool(
            left.stimulus_orientation_default
                .or(right.stimulus_orientation_default),
            raster_orientation == "unspecified",
        ),
        content_geometry.kind,
        content_geometry.width,
        content_geometry.height,
        marker_f32(Some(content_geometry.aspect_ratio), 1.0),
        marker_f32(Some(content_geometry.desired_display_aspect_ratio), 1.0),
        marker_f32(Some(content_geometry.desired_projection_aspect_ratio), 1.0),
        content_geometry.coordinate_space,
        content_geometry.origin,
        content_geometry.x_axis,
        content_geometry.y_axis,
        content_geometry.mapping_intent,
        content_geometry.metadata_source,
        marker_bool(Some(content_geometry.metadata_default), false),
        uv_rect_token(content_geometry.uv_rect),
        uv_rect_token(content_geometry.uv_rect),
        content_geometry.source_crop_rect_state,
        content_geometry.source_crop_rect_owner,
        target_schema,
        target_coordinate_space,
        target_screen_uv_rect_token(left.target_screen_uv_rect),
        target_screen_uv_rect_token(right.target_screen_uv_rect),
        target_clip_policy,
        target_metadata_source,
        marker_bool(
            left.target_footprint_default
                .or(right.target_footprint_default),
            target_metadata_source == "missing",
        ),
        target_footprint_debug_region_marker_fields(),
        left_width,
        left_height,
        right_width,
        right_height,
        content_geometry.left_width,
        content_geometry.left_height,
        content_geometry.right_width,
        content_geometry.right_height,
        uv_rect_token(content_geometry.left_uv_rect),
        uv_rect_token(content_geometry.right_uv_rect),
        uv_rect_token(content_geometry.left_uv_rect),
        uv_rect_token(content_geometry.right_uv_rect),
        pixel_rect_token(content_geometry.left_source_crop_rect_px),
        pixel_rect_token(content_geometry.right_source_crop_rect_px),
    )
}

pub(super) fn hwb_source_metadata_log_message(frame_index: u64, marker_fields: &str) -> String {
    format!(
        "Rusty XR HWB source metadata frame={} schema=rusty.xr.hwb-source-metadata.v1 phase=source-metadata status=ok sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler {}",
        frame_index, marker_fields
    )
}

pub(super) fn hwb_source_metadata_log_message_from_frame(
    frame: &StereoGpuCameraFrame,
    camera_projection_mode: CameraProjectionMode,
) -> String {
    let fields = projection_source_metadata_marker_fields(
        &frame.left.diagnostics,
        &frame.right.diagnostics,
        frame.left.width,
        frame.left.height,
        frame.right.width,
        frame.right.height,
        camera_projection_mode,
    );
    hwb_source_metadata_log_message(frame.index, &fields)
}

fn fallback_projection_geometry_profile(
    camera_projection_mode: CameraProjectionMode,
) -> &'static str {
    if camera_projection_mode.uses_world_canvas() {
        "full-frame-diagnostic"
    } else {
        "camera-projection"
    }
}

fn pixel_rect_token(rect: Option<[u32; 4]>) -> String {
    rect.map(|[left, top, right, bottom]| format!("{left},{top},{right},{bottom}"))
        .unwrap_or_else(|| "not-logged".to_string())
}

fn marker_token(value: Option<&str>, fallback: &str) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .replace(char::is_whitespace, "_")
}

fn marker_bool(value: Option<bool>, fallback: bool) -> &'static str {
    if value.unwrap_or(fallback) {
        "true"
    } else {
        "false"
    }
}

fn marker_f32(value: Option<f32>, fallback: f32) -> String {
    format!("{:.6}", value.unwrap_or(fallback))
}

fn uv_rect_token(rect: [f32; 4]) -> String {
    format!(
        "{:.6},{:.6},{:.6},{:.6}",
        rect[0], rect[1], rect[2], rect[3]
    )
}

fn target_screen_uv_rect_token(rect: Option<[f32; 4]>) -> String {
    rect.map(uv_rect_token)
        .unwrap_or_else(|| "not-logged".to_string())
}
