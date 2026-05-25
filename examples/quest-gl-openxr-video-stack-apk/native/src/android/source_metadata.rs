use rusty_xr_camera_model::{CameraExtrinsics, CameraIntrinsics, Rect2};

use super::source_content_geometry::OesContentGeometryRecord;
use super::source_metadata_json::{
    json_bool_any, json_string_any, json_u32, parse_camera2_extrinsics, parse_camera_intrinsics,
};

pub(super) fn aspect_ratio_u32(width: u32, height: u32) -> f32 {
    if width > 0 && height > 0 {
        width as f32 / height as f32
    } else {
        1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OesInputSourceKind {
    None,
    BrokerH264,
    DirectCamera2,
}

impl OesInputSourceKind {
    pub(super) fn from_label(label: Option<&str>) -> Self {
        let normalized = label.unwrap_or("").trim().to_ascii_lowercase();
        if normalized == "none" || normalized == "static" {
            Self::None
        } else if normalized.contains("direct")
            || normalized.contains("camera2-oes")
            || normalized.contains("camera-oes")
        {
            Self::DirectCamera2
        } else {
            Self::BrokerH264
        }
    }

    pub(super) fn codec_mime(self) -> Option<&'static str> {
        match self {
            Self::BrokerH264 => Some("video/avc"),
            Self::DirectCamera2 => Some("camera2/surface-texture"),
            Self::None => None,
        }
    }

    pub(super) fn stream_label(self, view_index: usize) -> String {
        let eye_name = if view_index == 0 { "left" } else { "right" };
        match self {
            Self::None => format!("static-grid:{eye_name}"),
            Self::BrokerH264 => format!("broker-h264-oes:{eye_name}"),
            Self::DirectCamera2 => format!("direct-camera2-oes:{eye_name}"),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct OesProjectionMetadata {
    pub(super) camera_id: String,
    pub(super) source: String,
    pub(super) pose_source: String,
    pub(super) pose_coordinate_convention: String,
    pub(super) synthetic_projection_profile: String,
    pub(super) projection_geometry_profile: String,
    pub(super) synthetic_pattern: String,
    pub(super) orientation_kind: String,
    pub(super) raster_orientation: String,
    pub(super) upright_marker: String,
    pub(super) orientation_metadata_source: String,
    pub(super) orientation_default: bool,
    pub(super) stimulus_raster_orientation: String,
    pub(super) stimulus_upright_marker: String,
    pub(super) stimulus_orientation_default: bool,
    pub(super) content_kind: String,
    pub(super) content_width: u32,
    pub(super) content_height: u32,
    pub(super) content_aspect_ratio: f32,
    pub(super) desired_display_aspect_ratio: f32,
    pub(super) desired_projection_aspect_ratio: f32,
    pub(super) content_coordinate_space: String,
    pub(super) content_origin: String,
    pub(super) content_x_axis: String,
    pub(super) content_y_axis: String,
    pub(super) content_mapping_intent: String,
    pub(super) content_geometry_metadata_source: String,
    pub(super) content_geometry_default: bool,
    pub(super) source_valid_uv_rect: Rect2,
    pub(super) projection_metadata_ready: bool,
    pub(super) delivered_width: u32,
    pub(super) delivered_height: u32,
    pub(super) intrinsics: Option<CameraIntrinsics>,
    pub(super) extrinsics: Option<CameraExtrinsics>,
}

impl OesProjectionMetadata {
    pub(super) fn parse(value: &serde_json::Value) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| "metadata_root_not_object".to_string())?;
        let camera_id = object
            .get("cameraId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("broker-h264")
            .to_string();
        let source = object
            .get("source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let pose_source = object
            .get("poseSource")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("missing")
            .to_string();
        let pose_coordinate_convention = object
            .get("poseCoordinateConvention")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let projection_geometry_fallback = "unknown";
        let synthetic_projection_profile = object
            .get("syntheticProjectionProfile")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let projection_geometry_profile = object
            .get("projectionGeometryProfile")
            .or_else(|| object.get("syntheticProjectionProfile"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or(projection_geometry_fallback)
            .to_string();
        let synthetic_pattern = object
            .get("syntheticPattern")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let orientation_kind = json_string_any(object, &["orientationKind"])
            .unwrap_or("unknown")
            .to_string();
        let raster_orientation = json_string_any(
            object,
            &[
                "rasterOrientation",
                "frameRasterOrientation",
                "stimulusRasterOrientation",
            ],
        )
        .unwrap_or("unspecified")
        .to_string();
        let upright_marker = json_string_any(
            object,
            &[
                "uprightMarker",
                "frameUprightMarker",
                "stimulusUprightMarker",
            ],
        )
        .unwrap_or("unspecified")
        .to_string();
        let orientation_metadata_source = json_string_any(
            object,
            &[
                "orientationMetadataSource",
                "frameOrientationMetadataSource",
                "stimulusOrientationMetadataSource",
            ],
        )
        .unwrap_or("missing")
        .to_string();
        let explicit_orientation_metadata = object.contains_key("rasterOrientation")
            || object.contains_key("frameRasterOrientation")
            || object.contains_key("stimulusRasterOrientation");
        let orientation_default = !explicit_orientation_metadata
            || json_bool_any(
                object,
                &[
                    "orientationDefault",
                    "frameOrientationDefault",
                    "stimulusOrientationDefault",
                ],
            )
            .unwrap_or(false);
        let stimulus_raster_orientation = object
            .get("stimulusRasterOrientation")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unspecified")
            .to_string();
        let stimulus_upright_marker = object
            .get("stimulusUprightMarker")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unspecified")
            .to_string();
        let stimulus_orientation_default = !object.contains_key("stimulusRasterOrientation")
            || object
                .get("stimulusOrientationDefault")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
        let projection_metadata_ready = object
            .get("projectionMetadataReady")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let delivered_width = json_u32(object.get("deliveredWidth")).unwrap_or(0);
        let delivered_height = json_u32(object.get("deliveredHeight")).unwrap_or(0);
        let content_geometry =
            OesContentGeometryRecord::parse(object, delivered_width, delivered_height);
        let intrinsics = parse_camera_intrinsics(object, delivered_width, delivered_height);
        let extrinsics = parse_camera2_extrinsics(object);
        Ok(Self {
            camera_id,
            source,
            pose_source,
            pose_coordinate_convention,
            synthetic_projection_profile,
            projection_geometry_profile,
            synthetic_pattern,
            orientation_kind,
            raster_orientation,
            upright_marker,
            orientation_metadata_source,
            orientation_default,
            stimulus_raster_orientation,
            stimulus_upright_marker,
            stimulus_orientation_default,
            content_kind: content_geometry.kind,
            content_width: content_geometry.width,
            content_height: content_geometry.height,
            content_aspect_ratio: content_geometry.aspect_ratio,
            desired_display_aspect_ratio: content_geometry.desired_display_aspect_ratio,
            desired_projection_aspect_ratio: content_geometry.desired_projection_aspect_ratio,
            content_coordinate_space: content_geometry.coordinate_space,
            content_origin: content_geometry.origin,
            content_x_axis: content_geometry.x_axis,
            content_y_axis: content_geometry.y_axis,
            content_mapping_intent: content_geometry.mapping_intent,
            content_geometry_metadata_source: content_geometry.metadata_source,
            content_geometry_default: content_geometry.metadata_default,
            source_valid_uv_rect: content_geometry.source_valid_uv_rect,
            projection_metadata_ready,
            delivered_width,
            delivered_height,
            intrinsics,
            extrinsics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::source_metadata_labels::{
        projection_source_label, stream_projection_metadata_log_message,
    };
    use super::*;
    use serde_json::json;

    fn metadata_json(source: &str, kind: &str, mapping_intent: &str) -> serde_json::Value {
        json!({
            "schema": "rusty.xr.camera_projection.stream_source_metadata.v1",
            "source": source,
            "cameraId": "left-camera",
            "poseSource": "stream-header",
            "poseCoordinateConvention": "camera2-reference",
            "projectionGeometryProfile": "camera-projection",
            "syntheticPattern": "none",
            "orientationKind": "camera-frame",
            "rasterOrientation": "top-left-origin-y-down",
            "uprightMarker": "camera-native-upright",
            "orientationMetadataSource": "stream-header",
            "orientationDefault": false,
            "stimulusRasterOrientation": "top-left-origin-y-down",
            "stimulusUprightMarker": "camera-native-upright",
            "stimulusOrientationDefault": false,
            "deliveredWidth": 1280,
            "deliveredHeight": 720,
            "contentGeometrySchema": "rusty.xr.stream_content_geometry.v1",
            "contentKind": kind,
            "contentWidth": 1280,
            "contentHeight": 720,
            "contentAspectRatio": 1.7777778,
            "desiredDisplayAspectRatio": 1.7777778,
            "desiredProjectionAspectRatio": 1.7777778,
            "contentCoordinateSpace": "normalized-uv",
            "contentOrigin": "top-left",
            "contentXAxis": "right",
            "contentYAxis": "down",
            "contentMappingIntent": mapping_intent,
            "contentGeometryMetadataSource": "stream-header",
            "contentGeometryDefault": false,
            "sourceValidUvRect": [0.0, 0.0, 1.0, 1.0],
            "projectionMetadataReady": true
        })
    }

    #[test]
    fn typed_content_geometry_supports_direct_camera_broker_camera_and_synthetic() {
        let cases = [
            (
                DIRECT_CAMERA2_OES_SOURCE,
                "camera-frame",
                "map-camera-frame-through-screen-to-camera-homography",
                "direct_camera2_characteristics",
            ),
            (
                "broker_app.camera_h264_stream",
                "broker-camera",
                "map-camera-frame-through-screen-to-camera-homography",
                "camera2_stream_header",
            ),
            (
                "broker_app.synthetic_h264_stream",
                "broker-synthetic",
                "map-full-frame-stimulus-to-projection-area",
                "broker_stream_header",
            ),
        ];

        for (source, kind, mapping_intent, metadata_label) in cases {
            let metadata =
                OesProjectionMetadata::parse(&metadata_json(source, kind, mapping_intent))
                    .expect("metadata should parse");
            let content_geometry = OesContentGeometryRecord::from_metadata(&metadata);

            assert_eq!(content_geometry.kind, kind);
            assert_eq!(content_geometry.mapping_intent, mapping_intent);
            assert_eq!(content_geometry.metadata_source, "stream-header");
            assert!(!content_geometry.metadata_default);

            let source_label = projection_source_label(&metadata, 1280, 720, false);
            assert!(source_label.contains(&format!(
                "{OES_PROJECTED_RENDER_PATH}:metadata={metadata_label}"
            )));
            assert!(source_label.contains(&format!("contentKind={kind}")));
            assert!(source_label.contains(&format!("contentMappingIntent={mapping_intent}")));

            let log_line = stream_projection_metadata_log_message(0, &metadata);
            assert!(log_line.contains(&format!("contentKind={kind}")));
            assert!(log_line.contains(&format!("contentMappingIntent={mapping_intent}")));
            assert!(log_line.contains("contentGeometryMetadataSource=stream-header"));
        }
    }

    #[test]
    fn input_source_kind_preserves_stable_labels() {
        let cases = [
            (
                None,
                OesInputSourceKind::BrokerH264,
                Some("video/avc"),
                "broker-h264-oes:left",
            ),
            (
                Some("none"),
                OesInputSourceKind::None,
                None,
                "static-grid:left",
            ),
            (
                Some("camera2-oes"),
                OesInputSourceKind::DirectCamera2,
                Some("camera2/surface-texture"),
                "direct-camera2-oes:left",
            ),
        ];

        for (label, expected, mime, stream_label) in cases {
            let source = OesInputSourceKind::from_label(label);
            assert_eq!(source, expected);
            assert_eq!(source.codec_mime(), mime);
            assert_eq!(source.stream_label(0), stream_label);
        }
    }
}
