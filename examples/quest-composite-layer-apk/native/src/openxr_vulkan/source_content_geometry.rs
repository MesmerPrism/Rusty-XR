use crate::HeadsetCameraFrameDiagnostics;

use super::source_content_geometry_rects::{marker_token, source_uv_rect_ltrb_for_diagnostics};

pub(super) use super::source_content_geometry_rects::{
    full_source_uv_rect_ltrb, source_uv_rect_xywh_for_diagnostics,
};

#[derive(Clone, Debug)]
pub(super) struct HwbStereoContentGeometry {
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
    pub(super) source_sampling_mode: String,
    pub(super) mapping_intent: String,
    pub(super) metadata_source: String,
    pub(super) metadata_default: bool,
    pub(super) uv_rect: [f32; 4],
    pub(super) source_crop_rect_state: String,
    pub(super) source_crop_rect_owner: String,
    pub(super) left_width: u32,
    pub(super) left_height: u32,
    pub(super) right_width: u32,
    pub(super) right_height: u32,
    pub(super) left_uv_rect: [f32; 4],
    pub(super) right_uv_rect: [f32; 4],
    pub(super) left_source_crop_rect_px: Option<[u32; 4]>,
    pub(super) right_source_crop_rect_px: Option<[u32; 4]>,
}

impl HwbStereoContentGeometry {
    pub(super) fn from_diagnostics_pair(
        left: &HeadsetCameraFrameDiagnostics,
        right: &HeadsetCameraFrameDiagnostics,
        left_width: u32,
        left_height: u32,
        right_width: u32,
        right_height: u32,
    ) -> Self {
        let left_content_width = left.content_width.unwrap_or(left_width);
        let left_content_height = left.content_height.unwrap_or(left_height);
        let right_content_width = right.content_width.unwrap_or(right_width);
        let right_content_height = right.content_height.unwrap_or(right_height);
        let content_width = left_content_width.max(right_content_width);
        let content_height = left_content_height.max(right_content_height);
        let fallback_aspect_ratio = if content_height > 0 {
            content_width as f32 / content_height as f32
        } else {
            1.0
        };
        let left_source_uv_rect = source_uv_rect_ltrb_for_diagnostics(left);
        let right_source_uv_rect = source_uv_rect_ltrb_for_diagnostics(right);
        let content_uv_rect = if left_source_uv_rect == right_source_uv_rect {
            left_source_uv_rect
        } else {
            full_source_uv_rect_ltrb()
        };
        let metadata_source = marker_token(
            left.content_geometry_metadata_source
                .as_deref()
                .or(right.content_geometry_metadata_source.as_deref()),
            "missing",
        );

        Self {
            kind: marker_token(
                left.content_kind
                    .as_deref()
                    .or(right.content_kind.as_deref()),
                "camera-frame",
            ),
            width: content_width,
            height: content_height,
            aspect_ratio: left
                .content_aspect_ratio
                .or(right.content_aspect_ratio)
                .unwrap_or(fallback_aspect_ratio),
            desired_display_aspect_ratio: left
                .desired_display_aspect_ratio
                .or(right.desired_display_aspect_ratio)
                .unwrap_or(fallback_aspect_ratio),
            desired_projection_aspect_ratio: left
                .desired_projection_aspect_ratio
                .or(right.desired_projection_aspect_ratio)
                .unwrap_or(fallback_aspect_ratio),
            coordinate_space: marker_token(
                left.content_coordinate_space
                    .as_deref()
                    .or(right.content_coordinate_space.as_deref()),
                "normalized-uv",
            ),
            origin: marker_token(
                left.content_origin
                    .as_deref()
                    .or(right.content_origin.as_deref()),
                "top-left",
            ),
            x_axis: marker_token(
                left.content_x_axis
                    .as_deref()
                    .or(right.content_x_axis.as_deref()),
                "right",
            ),
            y_axis: marker_token(
                left.content_y_axis
                    .as_deref()
                    .or(right.content_y_axis.as_deref()),
                "down",
            ),
            source_sampling_mode: marker_token(
                left.source_sampling_mode
                    .as_deref()
                    .or(right.source_sampling_mode.as_deref()),
                inferred_source_sampling_mode(left, right),
            ),
            mapping_intent: marker_token(
                left.content_mapping_intent
                    .as_deref()
                    .or(right.content_mapping_intent.as_deref()),
                "map-full-frame-content-to-projection-area",
            ),
            metadata_default: left
                .content_geometry_default
                .or(right.content_geometry_default)
                .unwrap_or(metadata_source == "missing"),
            metadata_source,
            uv_rect: content_uv_rect,
            source_crop_rect_state: marker_token(
                left.source_crop_rect_state
                    .as_deref()
                    .or(right.source_crop_rect_state.as_deref()),
                "not-logged",
            ),
            source_crop_rect_owner: marker_token(
                left.source_crop_rect_owner
                    .as_deref()
                    .or(right.source_crop_rect_owner.as_deref()),
                "not-logged",
            ),
            left_width: left_content_width,
            left_height: left_content_height,
            right_width: right_content_width,
            right_height: right_content_height,
            left_uv_rect: left_source_uv_rect,
            right_uv_rect: right_source_uv_rect,
            left_source_crop_rect_px: left.source_crop_rect_px,
            right_source_crop_rect_px: right.source_crop_rect_px,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        openxr_vulkan::source_metadata::projection_source_metadata_marker_fields,
        CameraProjectionMode, HeadsetCameraFrameDiagnostics,
    };
    use rusty_xr_contracts::{CameraCompositeTier, StereoMediaLayout};

    fn diagnostics(
        source: &str,
        kind: Option<&str>,
        mapping_intent: Option<&str>,
    ) -> HeadsetCameraFrameDiagnostics {
        HeadsetCameraFrameDiagnostics {
            source: Some(source.to_string()),
            camera_id: Some("50".to_string()),
            lens_facing: Some("external".to_string()),
            lens_facing_rank: Some(0),
            selection_score: Some(1),
            requested_tier: CameraCompositeTier::GpuProjected,
            active_tier_label: "gpu-projected".to_string(),
            transport: "hardware-buffer".to_string(),
            pose_source: Some("camera2".to_string()),
            pose_coordinate_convention: Some("camera2-reference".to_string()),
            lens_pose_reference_label: Some("openxr-head".to_string()),
            diagnostic_source: None,
            synthetic_projection_profile: None,
            projection_geometry_profile: Some("camera-projection".to_string()),
            synthetic_pattern: Some("none".to_string()),
            orientation_kind: Some("camera-frame".to_string()),
            raster_orientation: Some("top-left-origin-y-down".to_string()),
            upright_marker: Some("camera-native-upright".to_string()),
            orientation_metadata_source: Some("source-metadata".to_string()),
            orientation_default: Some(false),
            stimulus_raster_orientation: Some("top-left-origin-y-down".to_string()),
            stimulus_upright_marker: Some("camera-native-upright".to_string()),
            stimulus_orientation_default: Some(false),
            content_kind: kind.map(str::to_string),
            content_width: Some(1280),
            content_height: Some(720),
            content_aspect_ratio: Some(1.777_777_8),
            desired_display_aspect_ratio: Some(1.777_777_8),
            desired_projection_aspect_ratio: Some(1.777_777_8),
            content_coordinate_space: Some("normalized-uv".to_string()),
            content_origin: Some("top-left".to_string()),
            content_x_axis: Some("right".to_string()),
            content_y_axis: Some("down".to_string()),
            content_uv_rect: Some([0.0, 0.0, 1.0, 1.0]),
            source_visible_uv_rect: Some([0.0, 0.0, 1.0, 1.0]),
            source_crop_rect_px: Some([0, 0, 1280, 720]),
            source_crop_rect_state: Some("metadata-ready".to_string()),
            source_crop_rect_owner: Some("source-metadata".to_string()),
            source_sampling_mode: Some(
                source_sampling_mode_for_mapping_intent(mapping_intent).to_string(),
            ),
            content_mapping_intent: mapping_intent.map(str::to_string),
            content_geometry_metadata_source: Some("source-metadata".to_string()),
            content_geometry_default: Some(false),
            target_footprint_schema: None,
            target_coordinate_space: None,
            target_screen_uv_rect: None,
            target_clip_policy: None,
            target_footprint_metadata_source: None,
            target_footprint_default: None,
            requested_stereo_layout: Some("separate".to_string()),
            stereo_layout: StereoMediaLayout::Separate,
            mono_fallback: false,
            fallback_reason: "none".to_string(),
        }
    }

    #[test]
    fn typed_content_geometry_supports_direct_camera_broker_camera_and_synthetic() {
        let cases = [
            (
                "camera2-direct",
                "camera-frame",
                "map-camera-frame-through-screen-to-camera-homography",
            ),
            (
                "broker-h264-camera",
                "broker-camera",
                "map-camera-frame-through-screen-to-camera-homography",
            ),
            (
                "broker-h264-synthetic",
                "broker-synthetic",
                "map-full-frame-stimulus-to-projection-area",
            ),
        ];

        for (source, kind, mapping_intent) in cases {
            let left = diagnostics(source, Some(kind), Some(mapping_intent));
            let right = diagnostics(source, Some(kind), Some(mapping_intent));
            let content_geometry = HwbStereoContentGeometry::from_diagnostics_pair(
                &left, &right, 1280, 720, 1280, 720,
            );

            assert_eq!(content_geometry.kind, marker_token(Some(kind), "missing"));
            assert_eq!(
                content_geometry.mapping_intent,
                marker_token(Some(mapping_intent), "missing")
            );
            assert_eq!(
                content_geometry.source_sampling_mode,
                source_sampling_mode_for_mapping_intent(Some(mapping_intent))
            );
            assert!(!content_geometry.metadata_default);

            let fields = projection_source_metadata_marker_fields(
                &left,
                &right,
                1280,
                720,
                1280,
                720,
                CameraProjectionMode::DisplayScreenHomography,
            );
            assert!(fields.contains(&format!(
                "contentKind={}",
                marker_token(Some(kind), "missing")
            )));
            assert!(fields.contains(&format!(
                "contentMappingIntent={}",
                marker_token(Some(mapping_intent), "missing")
            )));
            assert!(fields.contains(&format!(
                "sourceSamplingMode={}",
                source_sampling_mode_for_mapping_intent(Some(mapping_intent))
            )));
            assert!(fields.contains("contentGeometryMetadataSource=source-metadata"));
        }
    }
}

fn inferred_source_sampling_mode(
    left: &HeadsetCameraFrameDiagnostics,
    right: &HeadsetCameraFrameDiagnostics,
) -> &'static str {
    let mapping_intent = left
        .content_mapping_intent
        .as_deref()
        .or(right.content_mapping_intent.as_deref());
    source_sampling_mode_for_mapping_intent(mapping_intent)
}

fn source_sampling_mode_for_mapping_intent(mapping_intent: Option<&str>) -> &'static str {
    match mapping_intent {
        Some(
            "map-camera-frame-through-screen-to-camera-homography"
            | "map-stimulus-raster-through-camera-projection",
        ) => "screen-to-camera-homography",
        Some(
            "map-camera-frame-to-full-frame-projection-area"
            | "map-camera-frame-to-full-frame-projection-surface"
            | "map-full-frame-stimulus-to-projection-area"
            | "map-full-frame-stimulus-to-projection-surface"
            | "map-full-frame-content-to-projection-area"
            | "map-full-frame-content-to-projection-surface",
        ) => "target-local-raster",
        _ => "target-local-raster",
    }
}
