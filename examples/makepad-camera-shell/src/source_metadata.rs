use rusty_xr_camera_model::{Rect2, Vec2};
use serde_json::Value as JsonValue;

use super::{
    marker_token, DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE, FRAME_RASTER_TOP_LEFT_Y_DOWN,
};

pub(crate) fn aspect_ratio_u32(width: u32, height: u32) -> f64 {
    if width > 0 && height > 0 {
        width as f64 / height as f64
    } else {
        1.0
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct BrokerH264ProjectionMetadata {
    pub(crate) camera_id: String,
    pub(crate) source: String,
    pub(crate) pose_source: String,
    pub(crate) pose_coordinate_convention: String,
    pub(crate) synthetic_projection_profile: String,
    pub(crate) projection_geometry_profile: String,
    pub(crate) synthetic_pattern: String,
    pub(crate) orientation_kind: String,
    pub(crate) raster_orientation: String,
    pub(crate) upright_marker: String,
    pub(crate) orientation_metadata_source: String,
    pub(crate) orientation_default: bool,
    pub(crate) stimulus_raster_orientation: String,
    pub(crate) stimulus_upright_marker: String,
    pub(crate) stimulus_orientation_metadata_source: String,
    pub(crate) stimulus_orientation_default: bool,
    pub(crate) content_kind: String,
    pub(crate) content_width: u32,
    pub(crate) content_height: u32,
    pub(crate) content_aspect_ratio: f64,
    pub(crate) desired_display_aspect_ratio: f64,
    pub(crate) desired_projection_aspect_ratio: f64,
    pub(crate) content_coordinate_space: String,
    pub(crate) content_origin: String,
    pub(crate) content_x_axis: String,
    pub(crate) content_y_axis: String,
    pub(crate) content_mapping_intent: String,
    pub(crate) content_geometry_metadata_source: String,
    pub(crate) content_geometry_default: bool,
    pub(crate) source_valid_uv_rect: Rect2,
    pub(crate) projection_metadata_ready: bool,
    pub(crate) delivered_width: u32,
    pub(crate) delivered_height: u32,
    pub(crate) intrinsics: Option<BrokerH264Intrinsics>,
    pub(crate) intrinsics_domain: Option<BrokerH264PixelDomain>,
    pub(crate) active_array_domain: Option<BrokerH264PixelDomain>,
    pub(crate) sensor_pixel_domain: Option<BrokerH264PixelDomain>,
    pub(crate) extrinsics: Option<BrokerH264Extrinsics>,
    pub(crate) metadata_bytes: usize,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct BrokerH264Intrinsics {
    pub(crate) fx: f32,
    pub(crate) fy: f32,
    pub(crate) cx: f32,
    pub(crate) cy: f32,
    pub(crate) skew: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct BrokerH264Extrinsics {
    pub(crate) translation: [f32; 3],
    pub(crate) rotation: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) struct BrokerH264PixelDomain {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl BrokerH264ProjectionMetadata {
    pub(crate) fn parse(metadata_json: &str) -> Result<Self, String> {
        let value: JsonValue =
            serde_json::from_str(metadata_json).map_err(|err| format!("invalid_json_{err}"))?;
        let object = value
            .as_object()
            .ok_or_else(|| "metadata_root_not_object".to_string())?;
        let camera_id = object
            .get("cameraId")
            .and_then(JsonValue::as_str)
            .unwrap_or("broker-h264")
            .to_string();
        let source = object
            .get("source")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown")
            .to_string();
        let pose_source = object
            .get("poseSource")
            .and_then(JsonValue::as_str)
            .unwrap_or("missing")
            .to_string();
        let pose_coordinate_convention = object
            .get("poseCoordinateConvention")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown")
            .to_string();
        let projection_geometry_fallback = "unknown";
        let synthetic_projection_profile = object
            .get("syntheticProjectionProfile")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown")
            .to_string();
        let projection_geometry_profile = object
            .get("projectionGeometryProfile")
            .or_else(|| object.get("syntheticProjectionProfile"))
            .and_then(JsonValue::as_str)
            .unwrap_or(projection_geometry_fallback)
            .to_string();
        let synthetic_pattern = object
            .get("syntheticPattern")
            .and_then(JsonValue::as_str)
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
            .and_then(JsonValue::as_str)
            .unwrap_or("unspecified")
            .to_string();
        let stimulus_upright_marker = object
            .get("stimulusUprightMarker")
            .and_then(JsonValue::as_str)
            .unwrap_or("unspecified")
            .to_string();
        let stimulus_orientation_metadata_source = object
            .get("stimulusOrientationMetadataSource")
            .and_then(JsonValue::as_str)
            .unwrap_or("missing")
            .to_string();
        let stimulus_orientation_default = !object.contains_key("stimulusRasterOrientation")
            || object
                .get("stimulusOrientationDefault")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
        let projection_metadata_ready = object
            .get("projectionMetadataReady")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let delivered_width = json_u32(object.get("deliveredWidth")).unwrap_or(0);
        let delivered_height = json_u32(object.get("deliveredHeight")).unwrap_or(0);
        let explicit_content_geometry = object.contains_key("contentGeometrySchema")
            || object.contains_key("contentWidth")
            || object.contains_key("contentHeight")
            || object.contains_key("contentMappingIntent");
        let content_kind = json_string_any(object, &["contentKind", "stimulusKind"])
            .unwrap_or("unknown")
            .to_string();
        let content_width =
            json_u32_any(object, &["contentWidth", "stimulusWidth"]).unwrap_or(delivered_width);
        let content_height =
            json_u32_any(object, &["contentHeight", "stimulusHeight"]).unwrap_or(delivered_height);
        let content_aspect_ratio =
            json_f64_any(object, &["contentAspectRatio", "stimulusAspectRatio"])
                .unwrap_or_else(|| aspect_ratio_u32(content_width, content_height));
        let desired_display_aspect_ratio = json_f64_any(
            object,
            &[
                "desiredDisplayAspectRatio",
                "desiredProjectionAspectRatio",
                "desiredAspectRatio",
            ],
        )
        .unwrap_or(content_aspect_ratio);
        let desired_projection_aspect_ratio = json_f64_any(
            object,
            &[
                "desiredProjectionAspectRatio",
                "desiredDisplayAspectRatio",
                "desiredAspectRatio",
            ],
        )
        .unwrap_or(desired_display_aspect_ratio);
        let content_coordinate_space = json_string_any(object, &["contentCoordinateSpace"])
            .unwrap_or("normalized-uv")
            .to_string();
        let content_origin = json_string_any(object, &["contentOrigin", "stimulusOrigin"])
            .unwrap_or("top-left")
            .to_string();
        let content_x_axis = json_string_any(object, &["contentXAxis"])
            .unwrap_or("right")
            .to_string();
        let content_y_axis = json_string_any(object, &["contentYAxis", "stimulusYAxis"])
            .unwrap_or("down")
            .to_string();
        let content_mapping_intent = json_string_any(object, &["contentMappingIntent"])
            .unwrap_or("unspecified")
            .to_string();
        let content_geometry_metadata_source =
            json_string_any(object, &["contentGeometryMetadataSource"])
                .unwrap_or("missing")
                .to_string();
        let content_geometry_default = !explicit_content_geometry
            || json_bool_any(object, &["contentGeometryDefault"]).unwrap_or(false);
        let source_valid_uv_rect = json_rect2_xywh_any(
            object,
            &["sourceValidUvRect", "contentUvRect", "stimulusUvRect"],
        )
        .unwrap_or(Rect2::UNIT);
        let intrinsics = parse_broker_intrinsics(object.get("intrinsics"));
        let intrinsics_domain = parse_broker_pixel_domain(object.get("intrinsicsDomain"));
        let active_array_domain = parse_broker_pixel_domain(object.get("activeArrayDomain"));
        let sensor_pixel_domain = parse_broker_pixel_domain(object.get("sensorPixelDomain"));
        let extrinsics = parse_broker_extrinsics(object.get("extrinsics"));

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
            stimulus_orientation_metadata_source,
            stimulus_orientation_default,
            content_kind,
            content_width,
            content_height,
            content_aspect_ratio,
            desired_display_aspect_ratio,
            desired_projection_aspect_ratio,
            content_coordinate_space,
            content_origin,
            content_x_axis,
            content_y_axis,
            content_mapping_intent,
            content_geometry_metadata_source,
            content_geometry_default,
            source_valid_uv_rect,
            projection_metadata_ready,
            delivered_width,
            delivered_height,
            intrinsics,
            intrinsics_domain,
            active_array_domain,
            sensor_pixel_domain,
            extrinsics,
            metadata_bytes: metadata_json.len(),
        })
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn ready_size(
        &self,
        fallback_width: usize,
        fallback_height: usize,
    ) -> Option<(u32, u32)> {
        if !self.projection_metadata_ready {
            return None;
        }
        let width = self.delivered_width.max(fallback_width as u32);
        let height = self.delivered_height.max(fallback_height as u32);
        (width > 0 && height > 0).then_some((width, height))
    }

    pub(crate) fn has_explicit_top_left_stimulus_orientation(&self) -> bool {
        self.has_explicit_stimulus_orientation()
            && self.stimulus_raster_orientation == FRAME_RASTER_TOP_LEFT_Y_DOWN
    }

    pub(crate) fn has_explicit_stimulus_orientation(&self) -> bool {
        !self.stimulus_orientation_default && self.stimulus_raster_orientation != "unspecified"
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn projection_profile_is(&self, expected: &str) -> bool {
        self.synthetic_projection_profile == expected
            || self.projection_geometry_profile == expected
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    fn content_mapping_intent_is_any(&self, expected: &[&str]) -> bool {
        expected
            .iter()
            .any(|value| self.content_mapping_intent == *value)
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn requests_camera_projection_mapping(&self) -> bool {
        self.projection_profile_is("camera-matched")
            || self.projection_profile_is("camera-projection")
            || self.projection_profile_is("physical-camera")
            || self.content_mapping_intent_is_any(&[
                "map-camera-frame-through-screen-to-camera-homography",
                "map-stimulus-raster-through-camera-projection",
            ])
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn is_full_frame_diagnostic_projection(&self) -> bool {
        self.requests_full_frame_projection_area_mapping()
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn requests_full_frame_projection_area_mapping(&self) -> bool {
        self.projection_profile_is("full-frame-diagnostic")
            || self.content_mapping_intent_is_any(&[
                "map-camera-frame-to-full-frame-projection-surface",
                "map-camera-frame-to-full-frame-projection-area",
                "map-full-frame-stimulus-to-projection-surface",
                "map-full-frame-stimulus-to-projection-area",
                "map-full-frame-content-to-projection-area",
            ])
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn requests_explicit_full_frame_content_mapping(&self) -> bool {
        self.content_mapping_intent_is_any(&[
            "map-full-frame-stimulus-to-projection-surface",
            "map-full-frame-stimulus-to-projection-area",
            "map-full-frame-content-to-projection-surface",
            "map-full-frame-content-to-projection-area",
        ])
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn requests_head_anchored_projection_area_mapping(&self) -> bool {
        self.projection_profile_is("head-anchored-virtual-camera")
            || self.content_mapping_intent_is_any(&[
                "fit-stimulus-raster-in-head-anchored-projection-area",
            ])
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn has_camera_projection_metadata(&self) -> bool {
        self.projection_metadata_ready && self.intrinsics.is_some() && self.extrinsics.is_some()
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn projection_mapping_profile_id(&self) -> &'static str {
        if self.requests_full_frame_projection_area_mapping() {
            "full-frame-diagnostic"
        } else if self.requests_camera_projection_mapping() {
            "camera-projection"
        } else if self.requests_head_anchored_projection_area_mapping() {
            "head-anchored-virtual-camera"
        } else {
            "unspecified"
        }
    }

    #[cfg(target_os = "android")]
    pub(crate) fn android_projection_source(
        &self,
    ) -> Option<super::android_camera_probe::BrokerProjectionSource> {
        let intrinsics = self.intrinsics?;
        let domain = self
            .intrinsics_domain
            .or(self.active_array_domain)
            .or(self.sensor_pixel_domain)
            .unwrap_or(BrokerH264PixelDomain {
                width: self.delivered_width,
                height: self.delivered_height,
            });
        let extrinsics = self.extrinsics?;
        Some(super::android_camera_probe::BrokerProjectionSource {
            camera_id: self.camera_id.clone(),
            intrinsics_fx: intrinsics.fx,
            intrinsics_fy: intrinsics.fy,
            intrinsics_cx: intrinsics.cx,
            intrinsics_cy: intrinsics.cy,
            intrinsics_skew: intrinsics.skew,
            intrinsics_domain_width: domain.width,
            intrinsics_domain_height: domain.height,
            pose_translation: extrinsics.translation,
            pose_rotation: extrinsics.rotation,
        })
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

fn json_string_any<'a>(
    object: &'a serde_json::Map<String, JsonValue>,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(JsonValue::as_str))
}

fn json_bool_any(object: &serde_json::Map<String, JsonValue>, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(JsonValue::as_bool))
}

fn json_u32(value: Option<&JsonValue>) -> Option<u32> {
    value
        .and_then(JsonValue::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn json_u32_any(object: &serde_json::Map<String, JsonValue>, keys: &[&str]) -> Option<u32> {
    keys.iter().find_map(|key| json_u32(object.get(*key)))
}

fn json_f64_any(object: &serde_json::Map<String, JsonValue>, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(JsonValue::as_f64))
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn json_rect2_xywh_any(
    object: &serde_json::Map<String, JsonValue>,
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

fn json_rect2_xywh(value: Option<&JsonValue>) -> Option<Rect2> {
    let value = value?;
    if let Some(array) = value.as_array() {
        if array.len() != 4 {
            return None;
        }
        return Some(Rect2::new(
            Vec2::new(
                json_f32_value(array.first())?,
                json_f32_value(array.get(1))?,
            ),
            Vec2::new(json_f32_value(array.get(2))?, json_f32_value(array.get(3))?),
        ));
    }
    if let Some(object) = value.as_object() {
        let x = json_f32_field_any(object, &["x", "left"])?;
        let y = json_f32_field_any(object, &["y", "top"])?;
        let width = json_f32_field_any(object, &["width", "w"])?;
        let height = json_f32_field_any(object, &["height", "h"])?;
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

fn json_f32_value(value: Option<&JsonValue>) -> Option<f32> {
    value
        .and_then(JsonValue::as_f64)
        .filter(|value| value.is_finite())
        .map(|value| value as f32)
}

fn json_f32_field_any(object: &serde_json::Map<String, JsonValue>, keys: &[&str]) -> Option<f32> {
    keys.iter().find_map(|key| json_f32_value(object.get(*key)))
}

fn json_f32_field(object: &serde_json::Map<String, JsonValue>, key: &str) -> Option<f32> {
    object
        .get(key)
        .and_then(JsonValue::as_f64)
        .filter(|value| value.is_finite())
        .map(|value| value as f32)
}

fn parse_broker_intrinsics(value: Option<&JsonValue>) -> Option<BrokerH264Intrinsics> {
    let object = value?.as_object()?;
    Some(BrokerH264Intrinsics {
        fx: json_f32_field(object, "fx")?,
        fy: json_f32_field(object, "fy")?,
        cx: json_f32_field(object, "cx")?,
        cy: json_f32_field(object, "cy")?,
        skew: json_f32_field(object, "skew").unwrap_or(0.0),
    })
}

fn parse_broker_extrinsics(value: Option<&JsonValue>) -> Option<BrokerH264Extrinsics> {
    let object = value?.as_object()?;
    Some(BrokerH264Extrinsics {
        translation: [
            json_f32_field(object, "px")?,
            json_f32_field(object, "py")?,
            json_f32_field(object, "pz")?,
        ],
        rotation: [
            json_f32_field(object, "qx")?,
            json_f32_field(object, "qy")?,
            json_f32_field(object, "qz")?,
            json_f32_field(object, "qw")?,
        ],
    })
}

fn parse_broker_pixel_domain(value: Option<&JsonValue>) -> Option<BrokerH264PixelDomain> {
    let object = value?.as_object()?;
    let width = json_u32(object.get("width"))?;
    let height = json_u32(object.get("height"))?;
    (width > 0 && height > 0).then_some(BrokerH264PixelDomain { width, height })
}
