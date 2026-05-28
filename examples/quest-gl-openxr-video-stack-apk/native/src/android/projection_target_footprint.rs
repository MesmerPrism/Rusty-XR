use rusty_xr_camera_model::{
    rect_xywh, target_footprint_debug_region_marker_fields, uv_rect_token, Rect2,
    TargetScreenFootprint, Vec2, TARGET_SCREEN_FOOTPRINT_SCHEMA,
};

use super::{
    openxr_gles_config::OesProjectionRuntimeState,
    projection_contract_markers::projection_area_screen_uv_rect,
    source_metadata::OesProjectionMetadata,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OesTargetFootprintSource {
    SourceMetadata,
    RendererLegacyProjectionArea,
}

impl OesTargetFootprintSource {
    pub(super) const fn stable_id(self) -> &'static str {
        match self {
            Self::SourceMetadata => "source-metadata",
            Self::RendererLegacyProjectionArea => "renderer-legacy-projection-area",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OesProjectionAreaParams {
    pub(super) eye_offset_uv: [[f32; 2]; 2],
    pub(super) scale: [f32; 2],
    pub(super) radius: [f32; 2],
    pub(super) corner_radius_uv: f32,
    pub(super) opacity: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OesResolvedTargetFootprints {
    pub(super) left: TargetScreenFootprint,
    pub(super) right: TargetScreenFootprint,
    pub(super) source: OesTargetFootprintSource,
}

impl OesResolvedTargetFootprints {
    pub(super) fn from_metadata_pair_or_state(
        left: &OesProjectionMetadata,
        right: &OesProjectionMetadata,
        state: OesProjectionRuntimeState,
    ) -> Self {
        if let (Some(left_rect), Some(right_rect)) =
            (left.target_screen_uv_rect, right.target_screen_uv_rect)
        {
            if let (Some(left), Some(right)) = (
                TargetScreenFootprint::from_display_eye_screen_uv_rect(left_rect),
                TargetScreenFootprint::from_display_eye_screen_uv_rect(right_rect),
            ) {
                return Self {
                    left,
                    right,
                    source: OesTargetFootprintSource::SourceMetadata,
                };
            }
        }
        Self::from_legacy_state(state)
    }

    pub(super) fn from_legacy_state(state: OesProjectionRuntimeState) -> Self {
        let left = legacy_footprint_for_eye(state, 0);
        let right = legacy_footprint_for_eye(state, 1);
        Self {
            left,
            right,
            source: OesTargetFootprintSource::RendererLegacyProjectionArea,
        }
    }

    pub(super) fn apply_to_state(
        self,
        mut state: OesProjectionRuntimeState,
    ) -> OesProjectionRuntimeState {
        let params = self.legacy_projection_area_params(state);
        state.projection_area_eye_offset_uv = params.eye_offset_uv;
        state.projection_area_scale = params.scale;
        state.projection_area_radius = params.radius;
        state.projection_area_corner_radius_uv = params.corner_radius_uv;
        state.projection_area_opacity = params.opacity;
        state
    }

    pub(super) fn legacy_projection_area_params(
        self,
        state: OesProjectionRuntimeState,
    ) -> OesProjectionAreaParams {
        if self.source == OesTargetFootprintSource::RendererLegacyProjectionArea {
            return OesProjectionAreaParams {
                eye_offset_uv: state.projection_area_eye_offset_uv,
                scale: state.projection_area_scale,
                radius: state.projection_area_radius,
                corner_radius_uv: state.projection_area_corner_radius_uv,
                opacity: state.projection_area_opacity,
            };
        }

        let left_rect = self.left.visible_screen_uv_rect;
        let right_rect = self.right.visible_screen_uv_rect;
        let left_center = rect_center(left_rect);
        let right_center = rect_center(right_rect);
        let radius_x = ((left_rect.size.x + right_rect.size.x) * 0.25).clamp(0.001, 0.5);
        let radius_y = ((left_rect.size.y + right_rect.size.y) * 0.25).clamp(0.001, 0.5);
        OesProjectionAreaParams {
            eye_offset_uv: [
                [left_center.x - 0.5, left_center.y - 0.5],
                [right_center.x - 0.5, right_center.y - 0.5],
            ],
            scale: [1.0, 1.0],
            radius: [radius_x, radius_y],
            corner_radius_uv: 0.0,
            opacity: state.projection_area_opacity,
        }
    }

    pub(super) fn marker_fields(self, state: OesProjectionRuntimeState) -> String {
        let params = self.legacy_projection_area_params(state);
        let stereo_size_mismatch = rect_xywh(self.left.visible_screen_uv_rect)[2..4]
            != rect_xywh(self.right.visible_screen_uv_rect)[2..4];
        let source_sampling_domain = if self.source == OesTargetFootprintSource::SourceMetadata {
            "target-local-uv"
        } else {
            "display-eye-screen-uv"
        };
        format!(
            "targetFootprintSchema={} projectionAreaTargetSource={} projectionAreaTargetStage=target_footprint_mapping projectionAreaTargetCoordinateSpace=display-eye-screen-uv projectionAreaTargetRectSemantics=xywh resolvedTargetFootprintSource={} resolvedTargetCoordinateSpace=display-eye-screen-uv resolvedTargetRectSemantics=xywh targetFootprintSourceSamplingDomain={} surfaceCoverageSource=renderer-authored surfaceCoverageSemantics=whole-render-target surfaceCoverageScreenUvRect=0.000000,0.000000,1.000000,1.000000 feedPlacementSource={} feedPlacementSemantics=video_content_inside_target_footprint projectionDepthMeters={:.3} cameraPreviewFovYDegrees={:.3} cameraPreviewOffsetYMeters={:.3} cameraRawOverlayOverscan={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} targetClipPolicy=clip-to-visible-eye leftTargetScreenUvRect={} rightTargetScreenUvRect={} leftVisibleTargetScreenUvRect={} rightVisibleTargetScreenUvRect={} leftTargetFootprintClipped={} rightTargetFootprintClipped={} targetFootprintStereoSizeMismatch={} effectBoundary=target-footprint borderRegionSemantics=visible-render-surface-minus-target-footprint sourceInvalidSemantics=target-fragment-maps-outside-source-valid-uv leftProjectionAreaScreenUvRect={} rightProjectionAreaScreenUvRect={} leftFeedPlacementScreenUvRect={} rightFeedPlacementScreenUvRect={} leftProjectionAreaCenterUv={} rightProjectionAreaCenterUv={} {}",
            TARGET_SCREEN_FOOTPRINT_SCHEMA,
            self.source.stable_id(),
            self.source.stable_id(),
            source_sampling_domain,
            self.source.stable_id(),
            state.tuning.projection_depth_meters,
            state.tuning.camera_preview_fov_y_degrees,
            state.tuning.camera_preview_offset_y_meters,
            state.tuning.camera_raw_overlay_overscan,
            state.projection_alpha_mode.stable_id(),
            state.projection_alpha_scale,
            state.projection_alpha_bias,
            rect_token(self.left.requested_screen_uv_rect),
            rect_token(self.right.requested_screen_uv_rect),
            rect_token(self.left.visible_screen_uv_rect),
            rect_token(self.right.visible_screen_uv_rect),
            self.left.clipped,
            self.right.clipped,
            stereo_size_mismatch,
            rect_token(self.left.visible_screen_uv_rect),
            rect_token(self.right.visible_screen_uv_rect),
            rect_token(self.left.visible_screen_uv_rect),
            rect_token(self.right.visible_screen_uv_rect),
            vec2_token([
                params.eye_offset_uv[0][0] + 0.5,
                params.eye_offset_uv[0][1] + 0.5,
            ]),
            vec2_token([
                params.eye_offset_uv[1][0] + 0.5,
                params.eye_offset_uv[1][1] + 0.5,
            ]),
            target_footprint_debug_region_marker_fields(),
        )
    }
}

pub(super) fn target_footprints_from_state(
    state: OesProjectionRuntimeState,
) -> OesResolvedTargetFootprints {
    OesResolvedTargetFootprints::from_legacy_state(state)
}

pub(super) fn target_footprints_from_metadata_pair_or_state(
    left: &OesProjectionMetadata,
    right: &OesProjectionMetadata,
    state: OesProjectionRuntimeState,
) -> OesResolvedTargetFootprints {
    OesResolvedTargetFootprints::from_metadata_pair_or_state(left, right, state)
}

fn legacy_footprint_for_eye(state: OesProjectionRuntimeState, eye: usize) -> TargetScreenFootprint {
    let rect = rect_from_xywh(projection_area_screen_uv_rect(
        state.projection_area_eye_offset_uv[eye],
        state.projection_area_radius,
        state.projection_area_scale,
    ));
    TargetScreenFootprint::from_display_eye_screen_uv_rect(rect)
        .expect("legacy projection area rect should be visible")
}

fn rect_from_xywh(rect: [f32; 4]) -> Rect2 {
    Rect2::new(Vec2::new(rect[0], rect[1]), Vec2::new(rect[2], rect[3]))
}

fn rect_center(rect: Rect2) -> Vec2 {
    rect.origin + rect.size * 0.5
}

fn rect_token(rect: Rect2) -> String {
    uv_rect_token(rect_xywh(rect))
}

fn vec2_token(value: [f32; 2]) -> String {
    format!("{:.6},{:.6}", value[0], value[1])
}
