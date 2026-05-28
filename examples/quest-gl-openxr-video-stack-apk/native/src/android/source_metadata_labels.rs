use rusty_xr_camera_model::{
    rect_xywh, target_footprint_debug_region_marker_fields, uv_rect_token,
};

use super::source_content_geometry::OesContentGeometryRecord;
use super::source_metadata::{aspect_ratio_u32, OesProjectionMetadata};
use super::{DIRECT_CAMERA2_OES_SOURCE, OES_PROJECTED_RENDER_PATH, PROJECTION_SOURCE_ASPECT};

pub(super) fn projection_surface_aspect_from_metadata(
    left: &OesProjectionMetadata,
    right: &OesProjectionMetadata,
    width: u32,
    height: u32,
) -> f32 {
    [
        left.desired_projection_aspect_ratio,
        right.desired_projection_aspect_ratio,
        left.content_aspect_ratio,
        right.content_aspect_ratio,
        aspect_ratio_u32(width, height),
    ]
    .into_iter()
    .find(|value| value.is_finite() && *value > 0.0)
    .unwrap_or(PROJECTION_SOURCE_ASPECT)
    .clamp(0.25, 4.0)
}

pub(super) fn projection_source_label(
    metadata: &OesProjectionMetadata,
    width: u32,
    height: u32,
    use_surface_texture_transform: bool,
) -> String {
    let content_geometry = OesContentGeometryRecord::from_metadata(metadata);
    let metadata_label = if metadata.is_synthetic() {
        "broker_stream_header"
    } else if metadata.source == DIRECT_CAMERA2_OES_SOURCE {
        "direct_camera2_characteristics"
    } else {
        "camera2_stream_header"
    };
    let source_sampling_fields =
        crate::source_sampling::OesSourceSamplingHandoff::new(use_surface_texture_transform)
            .marker_fields();
    let target_rect = target_screen_uv_rect_token(content_geometry.target_screen_uv_rect);
    format!(
        "{OES_PROJECTED_RENDER_PATH}:metadata={}:source={}:camera_id={}:pose_source={}:coordinate_convention={}:projection_profile={}:geometry_profile={}:pattern={}:size={}x{}:projectionMetadataReady={}:orientationKind={}:rasterOrientation={}:uprightMarker={}:orientationMetadataSource={}:orientationDefault={}:stimulusRasterOrientation={}:stimulusUprightMarker={}:stimulusOrientationDefault={}:contentKind={}:contentWidth={}:contentHeight={}:contentAspectRatio={:.6}:desiredDisplayAspectRatio={:.6}:desiredProjectionAspectRatio={:.6}:contentCoordinateSpace={}:contentOrigin={}:contentXAxis={}:contentYAxis={}:contentMappingIntent={}:contentGeometryMetadataSource={}:contentGeometryDefault={}:sourceValidUvRect={}:targetFootprintSchema={}:targetCoordinateSpace={}:targetScreenUvRect={}:targetClipPolicy={}:targetFootprintMetadataSource={}:targetFootprintDefault={}:{}:{}",
        metadata_label,
        metadata.source,
        metadata.camera_id,
        metadata.pose_source,
        metadata.pose_coordinate_convention,
        metadata.projection_geometry_profile,
        metadata.projection_geometry_profile,
        metadata.synthetic_pattern,
        width,
        height,
        metadata.projection_metadata_ready,
        metadata.orientation_kind,
        metadata.raster_orientation,
        metadata.upright_marker,
        metadata.orientation_metadata_source,
        metadata.orientation_default,
        metadata.stimulus_raster_orientation,
        metadata.stimulus_upright_marker,
        metadata.stimulus_orientation_default,
        content_geometry.kind,
        content_geometry.width,
        content_geometry.height,
        content_geometry.aspect_ratio,
        content_geometry.desired_display_aspect_ratio,
        content_geometry.desired_projection_aspect_ratio,
        content_geometry.coordinate_space,
        content_geometry.origin,
        content_geometry.x_axis,
        content_geometry.y_axis,
        content_geometry.mapping_intent,
        content_geometry.metadata_source,
        content_geometry.metadata_default,
        uv_rect_token(rect_xywh(content_geometry.source_valid_uv_rect)),
        content_geometry.target_footprint_schema,
        content_geometry.target_coordinate_space,
        target_rect,
        content_geometry.target_clip_policy,
        content_geometry.target_footprint_metadata_source,
        content_geometry.target_footprint_default,
        target_footprint_debug_region_marker_fields(),
        source_sampling_fields,
    )
}

pub(super) fn stream_projection_metadata_log_message(
    view_index: usize,
    metadata: &OesProjectionMetadata,
) -> String {
    let content_geometry = OesContentGeometryRecord::from_metadata(metadata);
    let target_rect = target_screen_uv_rect_token(content_geometry.target_screen_uv_rect);
    format!(
        "Rusty XR OpenXR GLES OES stream projection metadata eye={} source={} cameraId={} ready={} size={}x{} syntheticPattern={} orientationKind={} rasterOrientation={} uprightMarker={} orientationMetadataSource={} orientationDefault={} stimulusRasterOrientation={} stimulusUprightMarker={} stimulusOrientationDefault={} contentKind={} contentSize={}x{} contentAspectRatio={:.6} desiredDisplayAspectRatio={:.6} desiredProjectionAspectRatio={:.6} contentCoordinateSpace={} contentOrigin={} contentXAxis={} contentYAxis={} contentMappingIntent={} contentGeometryMetadataSource={} contentGeometryDefault={} sourceValidUvRect={} targetFootprintSchema={} targetCoordinateSpace={} targetScreenUvRect={} targetClipPolicy={} targetFootprintMetadataSource={} targetFootprintDefault={} {}",
        view_index,
        metadata.source,
        metadata.camera_id,
        metadata.projection_metadata_ready,
        metadata.delivered_width,
        metadata.delivered_height,
        metadata.synthetic_pattern,
        metadata.orientation_kind,
        metadata.raster_orientation,
        metadata.upright_marker,
        metadata.orientation_metadata_source,
        metadata.orientation_default,
        metadata.stimulus_raster_orientation,
        metadata.stimulus_upright_marker,
        metadata.stimulus_orientation_default,
        content_geometry.kind,
        content_geometry.width,
        content_geometry.height,
        content_geometry.aspect_ratio,
        content_geometry.desired_display_aspect_ratio,
        content_geometry.desired_projection_aspect_ratio,
        content_geometry.coordinate_space,
        content_geometry.origin,
        content_geometry.x_axis,
        content_geometry.y_axis,
        content_geometry.mapping_intent,
        content_geometry.metadata_source,
        content_geometry.metadata_default,
        uv_rect_token(rect_xywh(content_geometry.source_valid_uv_rect)),
        content_geometry.target_footprint_schema,
        content_geometry.target_coordinate_space,
        target_rect,
        content_geometry.target_clip_policy,
        content_geometry.target_footprint_metadata_source,
        content_geometry.target_footprint_default,
        target_footprint_debug_region_marker_fields(),
    )
}

fn target_screen_uv_rect_token(rect: Option<rusty_xr_camera_model::Rect2>) -> String {
    rect.map(|rect| uv_rect_token(rect_xywh(rect)))
        .unwrap_or_else(|| "not-logged".to_string())
}
