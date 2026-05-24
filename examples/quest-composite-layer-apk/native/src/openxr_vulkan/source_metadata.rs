use crate::{CameraProjectionMode, HeadsetCameraFrameDiagnostics};

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
    let content_kind = marker_token(
        left.content_kind
            .as_deref()
            .or(right.content_kind.as_deref()),
        "camera-frame",
    );
    let content_coordinate_space = marker_token(
        left.content_coordinate_space
            .as_deref()
            .or(right.content_coordinate_space.as_deref()),
        "normalized-uv",
    );
    let content_origin = marker_token(
        left.content_origin
            .as_deref()
            .or(right.content_origin.as_deref()),
        "top-left",
    );
    let content_x_axis = marker_token(
        left.content_x_axis
            .as_deref()
            .or(right.content_x_axis.as_deref()),
        "right",
    );
    let content_y_axis = marker_token(
        left.content_y_axis
            .as_deref()
            .or(right.content_y_axis.as_deref()),
        "down",
    );
    let content_mapping_intent = marker_token(
        left.content_mapping_intent
            .as_deref()
            .or(right.content_mapping_intent.as_deref()),
        "map-full-frame-content-to-projection-area",
    );
    let content_geometry_source = marker_token(
        left.content_geometry_metadata_source
            .as_deref()
            .or(right.content_geometry_metadata_source.as_deref()),
        "missing",
    );
    let left_content_width = left.content_width.unwrap_or(left_width);
    let left_content_height = left.content_height.unwrap_or(left_height);
    let right_content_width = right.content_width.unwrap_or(right_width);
    let right_content_height = right.content_height.unwrap_or(right_height);
    let content_width = left_content_width.max(right_content_width);
    let content_height = left_content_height.max(right_content_height);
    let content_aspect = marker_f32(
        left.content_aspect_ratio.or(right.content_aspect_ratio),
        if content_height > 0 {
            content_width as f32 / content_height as f32
        } else {
            1.0
        },
    );
    let display_aspect = marker_f32(
        left.desired_display_aspect_ratio
            .or(right.desired_display_aspect_ratio),
        if content_height > 0 {
            content_width as f32 / content_height as f32
        } else {
            1.0
        },
    );
    let projection_aspect = marker_f32(
        left.desired_projection_aspect_ratio
            .or(right.desired_projection_aspect_ratio),
        if content_height > 0 {
            content_width as f32 / content_height as f32
        } else {
            1.0
        },
    );
    let left_source_uv_rect = source_uv_rect_ltrb_for_diagnostics(left);
    let right_source_uv_rect = source_uv_rect_ltrb_for_diagnostics(right);
    let content_uv_rect = if left_source_uv_rect == right_source_uv_rect {
        left_source_uv_rect
    } else {
        full_source_uv_rect_ltrb()
    };
    let source_crop_rect_state = marker_token(
        left.source_crop_rect_state
            .as_deref()
            .or(right.source_crop_rect_state.as_deref()),
        "not-logged",
    );
    let source_crop_rect_owner = marker_token(
        left.source_crop_rect_owner
            .as_deref()
            .or(right.source_crop_rect_owner.as_deref()),
        "not-logged",
    );
    format!(
        "projectionMetadataReady=true source={} sourceMode={} brokerH264SourceMode={} sourceBindingMode=broker-h264-stream-header-{} brokerH264SyntheticProjectionProfile={} projection_profile={} geometry_profile={} syntheticPattern={} pattern={} orientationKind={} rasterOrientation={} uprightMarker={} orientationMetadataSource={} orientationDefault={} stimulusRasterOrientation={} stimulusUprightMarker={} stimulusOrientationDefault={} contentKind={} contentWidth={} contentHeight={} contentAspectRatio={} desiredDisplayAspectRatio={} desiredProjectionAspectRatio={} contentCoordinateSpace={} contentOrigin={} contentXAxis={} contentYAxis={} contentMappingIntent={} contentGeometryMetadataSource={} contentGeometryDefault={} contentUvRect={} sourceVisibleUvRect={} sourceCropRectState={} sourceCropRectOwner={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftContentWidth={} leftContentHeight={} rightContentWidth={} rightContentHeight={} leftContentUvRect={} rightContentUvRect={} leftSourceVisibleUvRect={} rightSourceVisibleUvRect={} leftSourceCropRectPx={} rightSourceCropRectPx={}",
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
        content_kind,
        content_width,
        content_height,
        content_aspect,
        display_aspect,
        projection_aspect,
        content_coordinate_space,
        content_origin,
        content_x_axis,
        content_y_axis,
        content_mapping_intent,
        content_geometry_source,
        marker_bool(
            left.content_geometry_default
                .or(right.content_geometry_default),
            content_geometry_source == "missing",
        ),
        uv_rect_token(content_uv_rect),
        uv_rect_token(content_uv_rect),
        source_crop_rect_state,
        source_crop_rect_owner,
        left_width,
        left_height,
        right_width,
        right_height,
        left_content_width,
        left_content_height,
        right_content_width,
        right_content_height,
        uv_rect_token(left_source_uv_rect),
        uv_rect_token(right_source_uv_rect),
        uv_rect_token(left_source_uv_rect),
        uv_rect_token(right_source_uv_rect),
        pixel_rect_token(left.source_crop_rect_px),
        pixel_rect_token(right.source_crop_rect_px),
    )
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

pub(super) fn full_source_uv_rect_ltrb() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

pub(super) fn source_uv_rect_ltrb_for_diagnostics(
    diagnostics: &HeadsetCameraFrameDiagnostics,
) -> [f32; 4] {
    diagnostics
        .source_visible_uv_rect
        .or(diagnostics.content_uv_rect)
        .unwrap_or_else(full_source_uv_rect_ltrb)
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
