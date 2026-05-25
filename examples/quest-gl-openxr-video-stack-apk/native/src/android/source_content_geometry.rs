use rusty_xr_camera_model::Rect2;

use super::source_metadata::{aspect_ratio_u32, OesProjectionMetadata};
use super::source_metadata_json::{
    json_bool_any, json_f32_any, json_rect2_xywh_any, json_string_any, json_u32_any,
};

#[derive(Clone, Debug)]
pub(super) struct OesContentGeometryRecord {
    pub(super) kind: String,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) aspect_ratio: f32,
    pub(super) desired_display_aspect_ratio: f32,
    pub(super) desired_projection_aspect_ratio: f32,
    pub(super) coordinate_space: String,
    pub(super) origin: String,
    pub(super) x_axis: String,
    pub(super) y_axis: String,
    pub(super) mapping_intent: String,
    pub(super) metadata_source: String,
    pub(super) metadata_default: bool,
    pub(super) source_valid_uv_rect: Rect2,
}

impl OesContentGeometryRecord {
    pub(super) fn parse(
        object: &serde_json::Map<String, serde_json::Value>,
        delivered_width: u32,
        delivered_height: u32,
    ) -> Self {
        let explicit_content_geometry = object.contains_key("contentGeometrySchema")
            || object.contains_key("contentWidth")
            || object.contains_key("contentHeight")
            || object.contains_key("contentMappingIntent");
        let width =
            json_u32_any(object, &["contentWidth", "stimulusWidth"]).unwrap_or(delivered_width);
        let height =
            json_u32_any(object, &["contentHeight", "stimulusHeight"]).unwrap_or(delivered_height);
        let aspect_ratio = json_f32_any(object, &["contentAspectRatio", "stimulusAspectRatio"])
            .unwrap_or_else(|| aspect_ratio_u32(width, height));
        let desired_display_aspect_ratio = json_f32_any(
            object,
            &[
                "desiredDisplayAspectRatio",
                "desiredProjectionAspectRatio",
                "desiredAspectRatio",
            ],
        )
        .unwrap_or(aspect_ratio);
        let desired_projection_aspect_ratio = json_f32_any(
            object,
            &[
                "desiredProjectionAspectRatio",
                "desiredDisplayAspectRatio",
                "desiredAspectRatio",
            ],
        )
        .unwrap_or(desired_display_aspect_ratio);
        let source_valid_uv_rect = json_rect2_xywh_any(
            object,
            &["sourceValidUvRect", "contentUvRect", "stimulusUvRect"],
        )
        .unwrap_or(Rect2::UNIT);

        Self {
            kind: json_string_any(object, &["contentKind", "stimulusKind"])
                .unwrap_or("unknown")
                .to_string(),
            width,
            height,
            aspect_ratio,
            desired_display_aspect_ratio,
            desired_projection_aspect_ratio,
            coordinate_space: json_string_any(object, &["contentCoordinateSpace"])
                .unwrap_or("normalized-uv")
                .to_string(),
            origin: json_string_any(object, &["contentOrigin", "stimulusOrigin"])
                .unwrap_or("top-left")
                .to_string(),
            x_axis: json_string_any(object, &["contentXAxis"])
                .unwrap_or("right")
                .to_string(),
            y_axis: json_string_any(object, &["contentYAxis", "stimulusYAxis"])
                .unwrap_or("down")
                .to_string(),
            mapping_intent: json_string_any(object, &["contentMappingIntent"])
                .unwrap_or("unspecified")
                .to_string(),
            metadata_source: json_string_any(object, &["contentGeometryMetadataSource"])
                .unwrap_or("missing")
                .to_string(),
            metadata_default: !explicit_content_geometry
                || json_bool_any(object, &["contentGeometryDefault"]).unwrap_or(false),
            source_valid_uv_rect,
        }
    }

    pub(super) fn from_metadata(metadata: &OesProjectionMetadata) -> Self {
        Self {
            kind: metadata.content_kind.clone(),
            width: metadata.content_width,
            height: metadata.content_height,
            aspect_ratio: metadata.content_aspect_ratio,
            desired_display_aspect_ratio: metadata.desired_display_aspect_ratio,
            desired_projection_aspect_ratio: metadata.desired_projection_aspect_ratio,
            coordinate_space: metadata.content_coordinate_space.clone(),
            origin: metadata.content_origin.clone(),
            x_axis: metadata.content_x_axis.clone(),
            y_axis: metadata.content_y_axis.clone(),
            mapping_intent: metadata.content_mapping_intent.clone(),
            metadata_source: metadata.content_geometry_metadata_source.clone(),
            metadata_default: metadata.content_geometry_default,
            source_valid_uv_rect: metadata.source_valid_uv_rect,
        }
    }
}
