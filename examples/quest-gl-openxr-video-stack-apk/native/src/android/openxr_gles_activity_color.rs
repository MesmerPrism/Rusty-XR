use super::{
    openxr_gles_activity_env::with_activity_env,
    openxr_gles_config::{activity_string_extra, OesColorControls, OesSourceColorTransfer},
};

pub(super) fn camera_color_controls_from_activity(
    app: &android_activity::AndroidApp,
) -> OesColorControls {
    let defaults = OesColorControls::default();
    with_activity_env(app, defaults, |env, activity| {
        let matrix = activity_string_extra(env, activity, "rustyxr.cameraColorMatrix")
            .as_deref()
            .map(parse_color_matrix)
            .unwrap_or(defaults.matrix);
        let offset = activity_string_extra(env, activity, "rustyxr.cameraColorOffset")
            .as_deref()
            .map(parse_color_offset)
            .unwrap_or(defaults.offset);
        let contrast = activity_string_extra(env, activity, "rustyxr.cameraColorContrast")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.contrast)
            .clamp(0.0, 4.0);
        let brightness = activity_string_extra(env, activity, "rustyxr.cameraColorBrightness")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.brightness)
            .clamp(-1.0, 1.0);
        let saturation = activity_string_extra(env, activity, "rustyxr.cameraColorSaturation")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.saturation)
            .clamp(0.0, 4.0);
        let source_transfer =
            activity_string_extra(env, activity, "rustyxr.oesSourceColorTransfer")
                .as_deref()
                .and_then(OesSourceColorTransfer::parse)
                .unwrap_or(defaults.source_transfer);
        OesColorControls {
            matrix,
            offset,
            contrast,
            brightness,
            saturation,
            source_transfer,
        }
    })
}

fn parse_color_components(value: &str) -> Vec<f32> {
    value
        .split([';', ',', ' '])
        .filter(|item| !item.trim().is_empty())
        .filter_map(|item| item.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .collect()
}

fn parse_color_matrix(value: &str) -> [[f32; 3]; 3] {
    let values = parse_color_components(value);
    if values.len() != 9 {
        return OesColorControls::default().matrix;
    }
    [
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
        [values[6], values[7], values[8]],
    ]
}

fn parse_color_offset(value: &str) -> [f32; 3] {
    let values = parse_color_components(value);
    if values.len() != 3 {
        return OesColorControls::default().offset;
    }
    [
        values[0].clamp(-1.0, 1.0),
        values[1].clamp(-1.0, 1.0),
        values[2].clamp(-1.0, 1.0),
    ]
}
