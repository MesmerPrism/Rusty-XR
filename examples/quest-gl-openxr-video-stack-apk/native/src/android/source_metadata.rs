use rusty_xr_camera_model::{rect_xywh, uv_rect_token};

use super::{
    OesProjectionMetadata, DIRECT_CAMERA2_OES_SOURCE, OES_PROJECTED_RENDER_PATH,
    PROJECTION_SOURCE_ASPECT,
};

pub(super) fn aspect_ratio_u32(width: u32, height: u32) -> f32 {
    if width > 0 && height > 0 {
        width as f32 / height as f32
    } else {
        1.0
    }
}

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
    format!(
        "{OES_PROJECTED_RENDER_PATH}:metadata={}:source={}:camera_id={}:pose_source={}:coordinate_convention={}:projection_profile={}:geometry_profile={}:pattern={}:size={}x{}:projectionMetadataReady={}:orientationKind={}:rasterOrientation={}:uprightMarker={}:orientationMetadataSource={}:orientationDefault={}:stimulusRasterOrientation={}:stimulusUprightMarker={}:stimulusOrientationDefault={}:contentKind={}:contentWidth={}:contentHeight={}:contentAspectRatio={:.6}:desiredDisplayAspectRatio={:.6}:desiredProjectionAspectRatio={:.6}:contentCoordinateSpace={}:contentOrigin={}:contentXAxis={}:contentYAxis={}:contentMappingIntent={}:contentGeometryMetadataSource={}:contentGeometryDefault={}:sourceValidUvRect={}:{}",
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
        metadata.content_kind,
        metadata.content_width,
        metadata.content_height,
        metadata.content_aspect_ratio,
        metadata.desired_display_aspect_ratio,
        metadata.desired_projection_aspect_ratio,
        metadata.content_coordinate_space,
        metadata.content_origin,
        metadata.content_x_axis,
        metadata.content_y_axis,
        metadata.content_mapping_intent,
        metadata.content_geometry_metadata_source,
        metadata.content_geometry_default,
        uv_rect_token(rect_xywh(metadata.source_valid_uv_rect)),
        source_sampling_fields,
    )
}
