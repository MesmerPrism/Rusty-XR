use super::{
    marker_token, BrokerH264ProjectionMetadata, DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE,
};

pub(crate) fn aspect_ratio_u32(width: u32, height: u32) -> f64 {
    if width > 0 && height > 0 {
        width as f64 / height as f64
    } else {
        1.0
    }
}

pub(crate) fn broker_pair_content_geometry_marker_fields(
    left: &BrokerH264ProjectionMetadata,
    right: &BrokerH264ProjectionMetadata,
) -> String {
    format!(
        "leftContentKind={} rightContentKind={} leftContentWidth={} leftContentHeight={} rightContentWidth={} rightContentHeight={} leftContentAspectRatio={:.6} rightContentAspectRatio={:.6} leftDesiredDisplayAspectRatio={:.6} rightDesiredDisplayAspectRatio={:.6} leftDesiredProjectionAspectRatio={:.6} rightDesiredProjectionAspectRatio={:.6} leftContentCoordinateSpace={} rightContentCoordinateSpace={} leftContentOrigin={} rightContentOrigin={} leftContentXAxis={} rightContentXAxis={} leftContentYAxis={} rightContentYAxis={} leftContentMappingIntent={} rightContentMappingIntent={} leftContentGeometryMetadataSource={} rightContentGeometryMetadataSource={} leftContentGeometryDefault={} rightContentGeometryDefault={} contentGeometryFallbackReason=none",
        marker_token(&left.content_kind),
        marker_token(&right.content_kind),
        left.content_width,
        left.content_height,
        right.content_width,
        right.content_height,
        left.content_aspect_ratio,
        right.content_aspect_ratio,
        left.desired_display_aspect_ratio,
        right.desired_display_aspect_ratio,
        left.desired_projection_aspect_ratio,
        right.desired_projection_aspect_ratio,
        marker_token(&left.content_coordinate_space),
        marker_token(&right.content_coordinate_space),
        marker_token(&left.content_origin),
        marker_token(&right.content_origin),
        marker_token(&left.content_x_axis),
        marker_token(&right.content_x_axis),
        marker_token(&left.content_y_axis),
        marker_token(&right.content_y_axis),
        marker_token(&left.content_mapping_intent),
        marker_token(&right.content_mapping_intent),
        marker_token(&left.content_geometry_metadata_source),
        marker_token(&right.content_geometry_metadata_source),
        left.content_geometry_default,
        right.content_geometry_default,
    )
}

pub(crate) fn normalize_direct_camera_projection_geometry_profile(value: &str) -> String {
    match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "" => DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE.to_string(),
        "full-frame" | "full-frame-diagnostic" => "full-frame-diagnostic".to_string(),
        "camera-projection"
        | "camera2-projection"
        | "camera2-platform"
        | "camera2-platform-unprofiled"
        | "physical-camera"
        | "camera-footprint"
        | "screen-to-camera-homography"
        | "custom" => "camera-projection".to_string(),
        other => format!("unsupported-direct-camera-projection-geometry-profile-{other}"),
    }
}

pub(crate) fn direct_camera2_content_geometry_marker_fields(
    width: usize,
    height: usize,
    projection_geometry_profile: &str,
) -> String {
    let content_width = u32::try_from(width).unwrap_or(0);
    let content_height = u32::try_from(height).unwrap_or(0);
    let aspect_ratio = aspect_ratio_u32(content_width, content_height);
    let projection_geometry_profile =
        normalize_direct_camera_projection_geometry_profile(projection_geometry_profile);
    let content_mapping_intent = match projection_geometry_profile.as_str() {
        "full-frame-diagnostic" => "map-full-frame-camera-frame-to-projection-surface",
        "camera-projection" => "map-camera-frame-through-screen-to-camera-homography",
        _ => "unsupported-direct-camera-projection-geometry-profile",
    };
    format!(
        "projectionGeometryProfile={} geometry_profile={} leftContentKind=camera-frame rightContentKind=camera-frame leftContentWidth={} leftContentHeight={} rightContentWidth={} rightContentHeight={} leftContentAspectRatio={:.6} rightContentAspectRatio={:.6} leftDesiredDisplayAspectRatio={:.6} rightDesiredDisplayAspectRatio={:.6} leftDesiredProjectionAspectRatio={:.6} rightDesiredProjectionAspectRatio={:.6} leftContentCoordinateSpace=normalized-uv rightContentCoordinateSpace=normalized-uv leftContentOrigin=top-left rightContentOrigin=top-left leftContentXAxis=right rightContentXAxis=right leftContentYAxis=down rightContentYAxis=down leftContentMappingIntent={} rightContentMappingIntent={} leftContentGeometryMetadataSource=makepad-direct-camera2-import rightContentGeometryMetadataSource=makepad-direct-camera2-import leftContentGeometryDefault=false rightContentGeometryDefault=false contentGeometryFallbackReason=none",
        projection_geometry_profile,
        projection_geometry_profile,
        content_width,
        content_height,
        content_width,
        content_height,
        aspect_ratio,
        aspect_ratio,
        aspect_ratio,
        aspect_ratio,
        aspect_ratio,
        aspect_ratio,
        content_mapping_intent,
        content_mapping_intent,
    )
}

pub(crate) fn missing_broker_content_geometry_marker_fields() -> String {
    "leftContentKind=default-fallback rightContentKind=default-fallback leftContentWidth=0 leftContentHeight=0 rightContentWidth=0 rightContentHeight=0 leftContentAspectRatio=1.000000 rightContentAspectRatio=1.000000 leftDesiredDisplayAspectRatio=1.000000 rightDesiredDisplayAspectRatio=1.000000 leftDesiredProjectionAspectRatio=1.000000 rightDesiredProjectionAspectRatio=1.000000 leftContentCoordinateSpace=normalized-uv rightContentCoordinateSpace=normalized-uv leftContentOrigin=top-left rightContentOrigin=top-left leftContentXAxis=right rightContentXAxis=right leftContentYAxis=down rightContentYAxis=down leftContentMappingIntent=standard-missing-metadata-fallback rightContentMappingIntent=standard-missing-metadata-fallback leftContentGeometryMetadataSource=missing rightContentGeometryMetadataSource=missing leftContentGeometryDefault=true rightContentGeometryDefault=true contentGeometryFallbackReason=broker-h264-content-geometry-metadata-missing".to_string()
}
