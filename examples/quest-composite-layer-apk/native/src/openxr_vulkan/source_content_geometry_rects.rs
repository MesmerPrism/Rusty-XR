use crate::HeadsetCameraFrameDiagnostics;

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

pub(super) fn source_uv_rect_xywh_for_diagnostics(
    diagnostics: &HeadsetCameraFrameDiagnostics,
) -> [f32; 4] {
    let [left, top, right, bottom] = source_uv_rect_ltrb_for_diagnostics(diagnostics);
    [left, top, (right - left).max(0.0), (bottom - top).max(0.0)]
}

pub(super) fn marker_token(value: Option<&str>, fallback: &str) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .replace(char::is_whitespace, "_")
}
