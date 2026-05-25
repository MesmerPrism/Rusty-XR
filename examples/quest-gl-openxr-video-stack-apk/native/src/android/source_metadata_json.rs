use rusty_xr_camera_model::{
    camera2_lens_pose_to_extrinsics, scale_intrinsics_to_image, CameraExtrinsics, CameraIntrinsics,
    ImageSize, Rect2, Vec2,
};

pub(super) fn json_u32(value: Option<&serde_json::Value>) -> Option<u32> {
    value
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

pub(super) fn json_u32_any(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<u32> {
    keys.iter().find_map(|key| json_u32(object.get(*key)))
}

pub(super) fn json_string_any<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(serde_json::Value::as_str))
}

pub(super) fn json_bool_any(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<bool> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(serde_json::Value::as_bool))
}

pub(super) fn json_f32(value: Option<&serde_json::Value>) -> Option<f32> {
    let value = value.and_then(serde_json::Value::as_f64)? as f32;
    value.is_finite().then_some(value)
}

pub(super) fn json_f32_any(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<f32> {
    keys.iter()
        .find_map(|key| json_f32(object.get(*key)))
        .filter(|value| value.is_finite() && *value > 0.0)
}

pub(super) fn json_rect2_xywh_any(
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

pub(super) fn parse_camera_intrinsics(
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

pub(super) fn parse_camera2_extrinsics(
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
