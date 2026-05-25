use super::source_metadata::OesProjectionMetadata;

impl OesProjectionMetadata {
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
