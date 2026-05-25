use super::{
    log_info,
    openxr_gles_config::{
        OesColorControls, OesProcessingLayer, OesProjectionRuntimeState, OesProjectionTuning,
    },
    projection_contract_markers::projection_area_target_marker_fields_from_state,
    projection_runtime_resolution::{
        oes_projection_runtime_resolution_enabled, oes_projection_runtime_resolution_from_state,
        oes_projection_runtime_state_from_resolution,
    },
};
use rusty_xr_runtime_config as rxrc;

pub(super) fn log_oes_projection_runtime_manifest(
    phase: &str,
    runtime: &rxrc::ProjectionRuntimeConfigResolution,
    resolved_manifest_consumption_enabled: bool,
) {
    for line in runtime.manifest_marker_lines("oes", phase) {
        log_info(line);
    }
    log_info(format!(
            "RUSTY_XR_OES_PROJECTION_RUNTIME schema=rusty.xr.oes-projection-runtime.v1 phase={} mode={} resolvedManifestConsumptionEnabled={}",
            phase,
            if resolved_manifest_consumption_enabled {
                "resolved-manifest"
            } else {
                "legacy"
            },
            resolved_manifest_consumption_enabled
        ));
}

pub(super) fn oes_projection_tuning_hotload_log_message(
    tuning_source: &str,
    frame_count: u64,
    tuning: OesProjectionTuning,
) -> String {
    format!(
        "Rusty XR OpenXR GLES projection tuning hotload source={} frame={} projectionDepthMeters={:.6} cameraPreviewFovYDegrees={:.6} cameraPreviewOffsetYMeters={:.6} cameraRawOverlayOverscan={:.6} propertyPrefix=debug.rustyxr",
        tuning_source,
        frame_count,
        tuning.projection_depth_meters,
        tuning.camera_preview_fov_y_degrees,
        tuning.camera_preview_offset_y_meters,
        tuning.camera_raw_overlay_overscan
    )
}

pub(super) fn oes_projection_runtime_hotload_log_message(
    tuning_source: &str,
    frame_count: u64,
    projection_state: OesProjectionRuntimeState,
) -> String {
    format!(
        "Rusty XR OpenXR GLES projection runtime hotload source={} frame={} projectionDepthMeters={:.6} cameraPreviewFovYDegrees={:.6} cameraPreviewOffsetYMeters={:.6} cameraRawOverlayOverscan={:.6} projectionAreaOffsetUv={:.6},{:.6} projectionAreaScale={:.6},{:.6} projectionAreaRadiusUv={:.6},{:.6} projectionAreaOpacity={:.3} projectionBorderOpacity={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} cameraProjectionMode={} projectionBorderPolicy={} propertyPrefix=debug.rustyxr",
        tuning_source,
        frame_count,
        projection_state.tuning.projection_depth_meters,
        projection_state.tuning.camera_preview_fov_y_degrees,
        projection_state.tuning.camera_preview_offset_y_meters,
        projection_state.tuning.camera_raw_overlay_overscan,
        projection_state.projection_area_offset_uv[0],
        projection_state.projection_area_offset_uv[1],
        projection_state.projection_area_scale[0],
        projection_state.projection_area_scale[1],
        projection_state.projection_area_radius[0],
        projection_state.projection_area_radius[1],
        projection_state.projection_area_opacity,
        projection_state.projection_border_opacity,
        projection_state.projection_alpha_mode.stable_id(),
        projection_state.projection_alpha_scale,
        projection_state.projection_alpha_bias,
        projection_state.camera_projection_mode.stable_id(),
        projection_state.projection_border_policy.stable_id()
    )
}

pub(super) struct OesProjectionRuntimeController {
    activity_projection_state: OesProjectionRuntimeState,
    resolved_manifest_consumption_enabled: bool,
    runtime: rxrc::ProjectionRuntimeConfigResolution,
    current_state: OesProjectionRuntimeState,
}

impl OesProjectionRuntimeController {
    pub(super) fn from_activity(
        app: &android_activity::AndroidApp,
        activity_projection_state: OesProjectionRuntimeState,
    ) -> Self {
        let runtime = oes_projection_runtime_resolution_from_state(activity_projection_state);
        let resolved_manifest_consumption_enabled = oes_projection_runtime_resolution_enabled(app);
        let current_state = if resolved_manifest_consumption_enabled {
            oes_projection_runtime_state_from_resolution(
                activity_projection_state,
                &runtime.resolution,
            )
        } else {
            activity_projection_state.with_legacy_system_properties()
        };

        Self {
            activity_projection_state,
            resolved_manifest_consumption_enabled,
            runtime,
            current_state,
        }
    }

    pub(super) fn current_state(&self) -> OesProjectionRuntimeState {
        self.current_state
    }

    pub(super) fn log_manifest(&self, phase: &str) {
        log_oes_projection_runtime_manifest(
            phase,
            &self.runtime,
            self.resolved_manifest_consumption_enabled,
        );
    }

    pub(super) fn log_initial_tuning_if_changed(&self, base_tuning: OesProjectionTuning) {
        if self.current_state.tuning == base_tuning {
            return;
        }
        log_info(oes_projection_tuning_hotload_log_message(
            self.tuning_source(),
            0,
            self.current_state.tuning,
        ));
    }

    pub(super) fn refresh_state(&mut self, frame_count: u64) -> OesProjectionRuntimeState {
        let next_state = self.resolve_current_state();
        if next_state != self.current_state {
            self.current_state = next_state;
            log_info(oes_projection_runtime_hotload_log_message(
                self.tuning_source(),
                frame_count,
                self.current_state,
            ));
        }
        self.current_state
    }

    fn resolve_current_state(&mut self) -> OesProjectionRuntimeState {
        if self.resolved_manifest_consumption_enabled {
            self.runtime =
                oes_projection_runtime_resolution_from_state(self.activity_projection_state);
            oes_projection_runtime_state_from_resolution(
                self.activity_projection_state,
                &self.runtime.resolution,
            )
        } else {
            self.activity_projection_state
                .with_legacy_system_properties()
        }
    }

    fn tuning_source(&self) -> &'static str {
        if self.resolved_manifest_consumption_enabled {
            "resolved-projection-runtime"
        } else {
            "android-system-property"
        }
    }
}

pub(super) fn log_oes_projection_startup_summary(
    projection_state: OesProjectionRuntimeState,
    processing_layer: OesProcessingLayer,
    blur_radius_px: f32,
    native_passthrough_underlay_requested: bool,
    native_passthrough_extension_enabled: bool,
    camera_color_controls: OesColorControls,
) {
    let projection_area_target_fields =
        projection_area_target_marker_fields_from_state(projection_state);
    log_info(format!(
            "Rusty XR OpenXR GLES projection border policy={} processingLayer={} cameraProjectionMode={} cameraBlurRadiusPx={:.3} projectionDepthMeters={:.3} cameraPreviewFovYDegrees={:.3} cameraPreviewOffsetYMeters={:.3} cameraRawOverlayOverscan={:.3} projectionAreaOffsetXUv={:.6} projectionAreaOffsetYUv={:.6} projectionAreaLeftOffsetXUv={:.6} projectionAreaLeftOffsetYUv={:.6} projectionAreaRightOffsetXUv={:.6} projectionAreaRightOffsetYUv={:.6} projectionAreaScale={:.6},{:.6} projectionAreaRadiusUv={:.6},{:.6} projectionAreaCornerRadiusUv={:.6} projectionAreaOpacity={:.3} projectionBorderOpacity={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} {} nativePassthroughUnderlayRequested={} nativePassthroughExtensionEnabled={} oesSourceColorTransfer={} sourceColorInputEncoding={} sourceColorOutputEncoding={} cameraColorMatrix={:?} cameraColorOffset={:?} cameraColorContrast={:.3} cameraColorBrightness={:.3} cameraColorSaturation={:.3}",
            projection_state.projection_border_policy.stable_id(),
            processing_layer.stable_id(),
            projection_state.camera_projection_mode.stable_id(),
            blur_radius_px,
            projection_state.tuning.projection_depth_meters,
            projection_state.tuning.camera_preview_fov_y_degrees,
            projection_state.tuning.camera_preview_offset_y_meters,
            projection_state.tuning.camera_raw_overlay_overscan,
            projection_state.projection_area_offset_uv[0],
            projection_state.projection_area_offset_uv[1],
            projection_state.projection_area_eye_offset_uv[0][0],
            projection_state.projection_area_eye_offset_uv[0][1],
            projection_state.projection_area_eye_offset_uv[1][0],
            projection_state.projection_area_eye_offset_uv[1][1],
            projection_state.projection_area_scale[0],
            projection_state.projection_area_scale[1],
            projection_state.projection_area_radius[0],
            projection_state.projection_area_radius[1],
            projection_state.projection_area_corner_radius_uv,
            projection_state.projection_area_opacity,
            projection_state.projection_border_opacity,
            projection_state.projection_alpha_mode.stable_id(),
            projection_state.projection_alpha_scale,
            projection_state.projection_alpha_bias,
            projection_area_target_fields,
            native_passthrough_underlay_requested,
            native_passthrough_extension_enabled,
            camera_color_controls.source_transfer.stable_id(),
            camera_color_controls.source_transfer.input_encoding(),
            camera_color_controls.source_transfer.output_encoding(),
            camera_color_controls.matrix,
            camera_color_controls.offset,
            camera_color_controls.contrast,
            camera_color_controls.brightness,
            camera_color_controls.saturation
        ));
}

#[cfg(test)]
mod tests {
    use super::super::openxr_gles_config::{
        OesCameraProjectionMode, OesProjectionAlphaMode, OesProjectionBorderPolicy,
    };
    use super::*;

    #[test]
    fn tuning_hotload_log_message_keeps_shape() {
        let line = oes_projection_tuning_hotload_log_message(
            "resolved-projection-runtime",
            0,
            OesProjectionTuning {
                projection_depth_meters: 1.25,
                camera_preview_fov_y_degrees: 72.0,
                camera_preview_offset_y_meters: 0.125,
                camera_raw_overlay_overscan: 1.5,
            },
        );

        assert_eq!(
            line,
            "Rusty XR OpenXR GLES projection tuning hotload source=resolved-projection-runtime frame=0 projectionDepthMeters=1.250000 cameraPreviewFovYDegrees=72.000000 cameraPreviewOffsetYMeters=0.125000 cameraRawOverlayOverscan=1.500000 propertyPrefix=debug.rustyxr"
        );
    }

    #[test]
    fn runtime_hotload_log_message_keeps_shape() {
        let line = oes_projection_runtime_hotload_log_message(
            "android-system-property",
            42,
            OesProjectionRuntimeState {
                tuning: OesProjectionTuning {
                    projection_depth_meters: 1.25,
                    camera_preview_fov_y_degrees: 72.0,
                    camera_preview_offset_y_meters: 0.125,
                    camera_raw_overlay_overscan: 1.5,
                },
                projection_area_offset_uv: [0.01, -0.02],
                projection_area_eye_offset_uv: [[0.0, 0.0], [0.0, 0.0]],
                projection_area_scale: [0.95, 0.85],
                projection_area_radius: [0.47, 0.36],
                projection_area_corner_radius_uv: 0.08,
                projection_area_opacity: 0.75,
                projection_border_opacity: 0.5,
                projection_alpha_mode: OesProjectionAlphaMode::Green,
                projection_alpha_scale: 1.25,
                projection_alpha_bias: -0.25,
                camera_projection_mode: OesCameraProjectionMode::WorldCanvas,
                projection_border_policy: OesProjectionBorderPolicy::PassthroughUnderlay,
            },
        );

        assert_eq!(
            line,
            "Rusty XR OpenXR GLES projection runtime hotload source=android-system-property frame=42 projectionDepthMeters=1.250000 cameraPreviewFovYDegrees=72.000000 cameraPreviewOffsetYMeters=0.125000 cameraRawOverlayOverscan=1.500000 projectionAreaOffsetUv=0.010000,-0.020000 projectionAreaScale=0.950000,0.850000 projectionAreaRadiusUv=0.470000,0.360000 projectionAreaOpacity=0.750 projectionBorderOpacity=0.500 projectionAlphaMode=green projectionAlphaScale=1.250 projectionAlphaBias=-0.250 cameraProjectionMode=world-canvas projectionBorderPolicy=passthrough-underlay propertyPrefix=debug.rustyxr"
        );
    }
}
