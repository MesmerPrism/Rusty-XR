use rusty_xr_camera_model::{
    camera2_lens_pose_to_extrinsics, rect_xywh, scale_intrinsics_to_image, uv_rect_token,
    CameraExtrinsics, CameraIntrinsics, ImageSize, Rect2, Vec2,
};

use super::{DIRECT_CAMERA2_OES_SOURCE, OES_PROJECTED_RENDER_PATH, PROJECTION_SOURCE_ASPECT};

pub(super) fn aspect_ratio_u32(width: u32, height: u32) -> f32 {
    if width > 0 && height > 0 {
        width as f32 / height as f32
    } else {
        1.0
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

#[derive(Clone, Debug)]
pub(super) struct OesContentGeometryRecord {
    kind: String,
    width: u32,
    height: u32,
    aspect_ratio: f32,
    desired_display_aspect_ratio: f32,
    desired_projection_aspect_ratio: f32,
    coordinate_space: String,
    origin: String,
    x_axis: String,
    y_axis: String,
    mapping_intent: String,
    metadata_source: String,
    metadata_default: bool,
    source_valid_uv_rect: Rect2,
}

impl OesContentGeometryRecord {
    fn parse(
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

    fn from_metadata(metadata: &OesProjectionMetadata) -> Self {
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

    pub(super) fn is_synthetic(&self) -> bool {
        self.source == "broker_app.synthetic_h264_stream"
    }

    fn projection_profile_is(&self, expected: &str) -> bool {
        self.synthetic_projection_profile == expected
            || self.projection_geometry_profile == expected
    }

    fn content_mapping_intent_is_any(&self, expected: &[&str]) -> bool {
        expected
            .iter()
            .any(|value| self.content_mapping_intent == *value)
    }

    pub(super) fn requests_camera_projection_mapping(&self) -> bool {
        self.projection_profile_is("camera-matched")
            || self.projection_profile_is("camera-projection")
            || self.projection_profile_is("physical-camera")
            || self.content_mapping_intent_is_any(&[
                "map-camera-frame-through-screen-to-camera-homography",
                "map-stimulus-raster-through-camera-projection",
            ])
    }

    pub(super) fn is_full_frame_diagnostic_projection(&self) -> bool {
        self.requests_full_frame_projection_area_mapping()
    }

    pub(super) fn requests_full_frame_projection_area_mapping(&self) -> bool {
        self.projection_profile_is("full-frame-diagnostic")
            || self.content_mapping_intent_is_any(&[
                "map-camera-frame-to-full-frame-projection-surface",
                "map-camera-frame-to-full-frame-projection-area",
                "map-full-frame-stimulus-to-projection-surface",
                "map-full-frame-stimulus-to-projection-area",
                "map-full-frame-content-to-projection-area",
            ])
    }

    pub(super) fn requests_explicit_full_frame_content_mapping(&self) -> bool {
        self.content_mapping_intent_is_any(&[
            "map-full-frame-stimulus-to-projection-surface",
            "map-full-frame-stimulus-to-projection-area",
            "map-full-frame-content-to-projection-surface",
            "map-full-frame-content-to-projection-area",
        ])
    }

    pub(super) fn requests_head_anchored_projection_area_mapping(&self) -> bool {
        self.projection_profile_is("head-anchored-virtual-camera")
            || self.content_mapping_intent_is_any(&[
                "fit-stimulus-raster-in-head-anchored-projection-area",
            ])
    }

    pub(super) fn has_explicit_top_left_stimulus_orientation(&self) -> bool {
        !self.stimulus_orientation_default
            && self.stimulus_raster_orientation == "top-left-origin-y-down"
    }

    pub(super) fn has_camera2_projection(&self) -> bool {
        self.projection_metadata_ready
            && self.intrinsics.is_some()
            && self.extrinsics.is_some()
            && self.delivered_width > 0
            && self.delivered_height > 0
    }

    pub(super) fn has_metadata_backed_camera_projection(&self) -> bool {
        self.has_camera2_projection()
    }
}

fn json_u32(value: Option<&serde_json::Value>) -> Option<u32> {
    value
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn json_u32_any(object: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<u32> {
    keys.iter().find_map(|key| json_u32(object.get(*key)))
}

fn json_string_any<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(serde_json::Value::as_str))
}

fn json_bool_any(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<bool> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(serde_json::Value::as_bool))
}

fn json_f32(value: Option<&serde_json::Value>) -> Option<f32> {
    let value = value.and_then(serde_json::Value::as_f64)? as f32;
    value.is_finite().then_some(value)
}

fn json_f32_any(object: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<f32> {
    keys.iter()
        .find_map(|key| json_f32(object.get(*key)))
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn json_rect2_xywh_any(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<Rect2> {
    keys.iter()
        .find_map(|key| json_rect2_xywh(object.get(*key)))
        .filter(|rect| {
            rect.is_valid()
                && rect.size.x > 0.0
                && rect.size.y > 0.0
                && rect.origin.x >= 0.0
                && rect.origin.y >= 0.0
                && rect.max().x <= 1.0
                && rect.max().y <= 1.0
        })
}

fn json_rect2_xywh(value: Option<&serde_json::Value>) -> Option<Rect2> {
    let value = value?;
    if let Some(array) = value.as_array() {
        if array.len() != 4 {
            return None;
        }
        return Some(Rect2::new(
            Vec2::new(json_f32(array.first())?, json_f32(array.get(1))?),
            Vec2::new(json_f32(array.get(2))?, json_f32(array.get(3))?),
        ));
    }
    if let Some(object) = value.as_object() {
        let x = json_f32(object.get("x")).or_else(|| json_f32(object.get("left")))?;
        let y = json_f32(object.get("y")).or_else(|| json_f32(object.get("top")))?;
        let width = json_f32(object.get("width")).or_else(|| json_f32(object.get("w")))?;
        let height = json_f32(object.get("height")).or_else(|| json_f32(object.get("h")))?;
        return Some(Rect2::new(Vec2::new(x, y), Vec2::new(width, height)));
    }
    let text = value.as_str()?;
    let parts: Vec<f32> = text
        .split(',')
        .filter_map(|part| part.trim().parse::<f32>().ok())
        .collect();
    if parts.len() == 4 {
        Some(Rect2::new(
            Vec2::new(parts[0], parts[1]),
            Vec2::new(parts[2], parts[3]),
        ))
    } else {
        None
    }
}

fn json_object_size(value: Option<&serde_json::Value>) -> Option<ImageSize> {
    let object = value?.as_object()?;
    let width = json_u32(object.get("width"))?;
    let height = json_u32(object.get("height"))?;
    ImageSize::new(width, height)
        .is_non_empty()
        .then_some(ImageSize::new(width, height))
}

fn parse_camera_intrinsics(
    object: &serde_json::Map<String, serde_json::Value>,
    delivered_width: u32,
    delivered_height: u32,
) -> Option<CameraIntrinsics> {
    let intrinsics = object.get("intrinsics")?.as_object()?;
    let source_size = json_object_size(object.get("intrinsicsDomain"))
        .or_else(|| json_object_size(object.get("activeArrayDomain")))
        .or_else(|| json_object_size(object.get("sensorPixelDomain")))
        .or_else(|| {
            ImageSize::new(delivered_width, delivered_height)
                .is_non_empty()
                .then_some(ImageSize::new(delivered_width, delivered_height))
        })?;
    let target_size = ImageSize::new(delivered_width, delivered_height);
    if !target_size.is_non_empty() {
        return None;
    }
    let source_intrinsics = CameraIntrinsics::new(
        Vec2::new(
            json_f32(intrinsics.get("fx"))?,
            json_f32(intrinsics.get("fy"))?,
        ),
        Vec2::new(
            json_f32(intrinsics.get("cx"))?,
            json_f32(intrinsics.get("cy"))?,
        ),
        source_size,
    )
    .with_skew_px(json_f32(intrinsics.get("skew")).unwrap_or(0.0));
    scale_intrinsics_to_image(source_intrinsics, source_size, target_size).ok()
}

fn parse_camera2_extrinsics(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Option<CameraExtrinsics> {
    let extrinsics = object.get("extrinsics")?.as_object()?;
    camera2_lens_pose_to_extrinsics(
        [
            json_f32(extrinsics.get("px"))?,
            json_f32(extrinsics.get("py"))?,
            json_f32(extrinsics.get("pz"))?,
        ],
        [
            json_f32(extrinsics.get("qx"))?,
            json_f32(extrinsics.get("qy"))?,
            json_f32(extrinsics.get("qz"))?,
            json_f32(extrinsics.get("qw"))?,
        ],
    )
    .ok()
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
        source_sampling_fields,
    )
}

pub(super) fn stream_projection_metadata_log_message(
    view_index: usize,
    metadata: &OesProjectionMetadata,
) -> String {
    let content_geometry = OesContentGeometryRecord::from_metadata(metadata);
    format!(
        "Rusty XR OpenXR GLES OES stream projection metadata eye={} source={} cameraId={} ready={} size={}x{} syntheticPattern={} orientationKind={} rasterOrientation={} uprightMarker={} orientationMetadataSource={} orientationDefault={} stimulusRasterOrientation={} stimulusUprightMarker={} stimulusOrientationDefault={} contentKind={} contentSize={}x{} contentAspectRatio={:.6} desiredDisplayAspectRatio={:.6} desiredProjectionAspectRatio={:.6} contentCoordinateSpace={} contentOrigin={} contentXAxis={} contentYAxis={} contentMappingIntent={} contentGeometryMetadataSource={} contentGeometryDefault={} sourceValidUvRect={}",
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
    )
}

#[cfg(test)]
mod tests {
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
}
