use crate::{
    HeadsetCameraFrameDiagnostics, RuntimeConfig, StereoGpuCameraFrame, StereoProjectionControls,
};
use rusty_xr_contracts::{
    SourceSamplerYAxis, SourceSamplingContract, SourceSamplingTransformStage, SourceUvRect, Vec2,
};

pub(crate) const HWB_SOURCE_UV_CONTRACT: &str =
    "screen_to_camera_content_uv_to_hardware_buffer_sampler";
const HWB_SOURCE_SAMPLING_BACKEND: &str = "hwb";
const HWB_SOURCE_SAMPLING_MODE: &str = "hwb-runtime";
const HWB_SAMPLE_TRANSFORM: &str = "sourceVisibleUvRect+cameraTextureTransformFlags";
const HWB_SAMPLE_TRANSFORM_OWNER: &str =
    "android-media-image-crop-rect+vulkan-hwb-camera_projection_shader";
const HWB_OUTPUT_UV_LABEL: &str = "hardware-buffer-sampler-uv";
const HWB_SAMPLER_UV_ORIGIN: &str = "hardware-buffer-import-convention";
const HWB_TEXTURE_TRANSFORM_OWNER: &str = "vulkan-hwb-camera_projection_shader";

pub(crate) struct HwbSourceSamplingHandoff<'a> {
    frame: &'a StereoGpuCameraFrame,
    controls: &'a StereoProjectionControls,
    config: &'a RuntimeConfig,
}

pub(crate) struct HwbFinalProjectionStatusLog<'a> {
    pub(crate) frame: &'a StereoGpuCameraFrame,
    pub(crate) controls: &'a StereoProjectionControls,
    pub(crate) config: &'a RuntimeConfig,
    pub(crate) openxr_frame_count: u64,
    pub(crate) openxr_focused: bool,
    pub(crate) aligned_projection: bool,
    pub(crate) projection_homography_fields: &'a str,
    pub(crate) pose_source: &'a str,
    pub(crate) pose_reference: &'a str,
    pub(crate) pose_convention: &'a str,
    pub(crate) display_left_camera_id: &'a str,
    pub(crate) display_right_camera_id: &'a str,
    pub(crate) import_cache_size: usize,
    pub(crate) stereo_descriptor_cache_size: usize,
    pub(crate) temporal_projection_mode: &'a str,
    pub(crate) camera_frame_age_ms_avg: &'a str,
    pub(crate) camera_frame_age_ms_p95: &'a str,
    pub(crate) temporal_metrics: HwbFinalProjectionTemporalMetrics,
    pub(crate) camera_cadence_metrics: HwbCameraRenderCadenceMetrics,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct HwbFinalProjectionTemporalMetrics {
    pub(crate) frame_adoption_held: bool,
    pub(crate) frame_adoption_candidate_motion_px_p95: f64,
    pub(crate) stereo_pair_delta_ms_avg: f64,
    pub(crate) target_projection_motion_px_avg: f64,
    pub(crate) target_projection_motion_px_p95: f64,
    pub(crate) applied_projection_motion_px_avg: f64,
    pub(crate) applied_projection_motion_px_p95: f64,
    pub(crate) projection_residual_px_avg: f64,
    pub(crate) projection_residual_px_p95: f64,
    pub(crate) visual_lag_ms_avg: f64,
    pub(crate) visual_lag_ms_p95: f64,
    pub(crate) held_frame_count: u64,
    pub(crate) held_frame_duration_ms_max: f64,
    pub(crate) frame_crossfade_count: u64,
    pub(crate) invalid_uv_px_percent: f64,
    pub(crate) edge_fill_px_percent: f64,
    pub(crate) asw_enabled_frame_count: u64,
    pub(crate) asw_skipped_frame_count: u64,
    pub(crate) motion_vector_max_px: f64,
    pub(crate) motion_vector_clamped_count: u64,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct HwbCameraRenderCadenceMetrics {
    pub(crate) render_frame_count: u64,
    pub(crate) distinct_frame_count: u64,
    pub(crate) repeated_render_frame_count: u64,
    pub(crate) renders_per_camera_frame_avg: f64,
    pub(crate) max_consecutive_render_frames_per_camera_frame: u64,
    pub(crate) consumed_frame_hz: f64,
    pub(crate) projection_render_hz: f64,
}

impl<'a> HwbSourceSamplingHandoff<'a> {
    pub(crate) const fn new(
        frame: &'a StereoGpuCameraFrame,
        controls: &'a StereoProjectionControls,
        config: &'a RuntimeConfig,
    ) -> Self {
        Self {
            frame,
            controls,
            config,
        }
    }

    pub(crate) fn contract(&self) -> SourceSamplingContract {
        let left_source_uv_rect = source_uv_rect_ltrb_for_diagnostics(&self.frame.left.diagnostics);
        let right_source_uv_rect =
            source_uv_rect_ltrb_for_diagnostics(&self.frame.right.diagnostics);
        let content_uv_rect = if left_source_uv_rect == right_source_uv_rect {
            left_source_uv_rect
        } else {
            full_source_uv_rect_ltrb()
        };
        SourceSamplingContract::new(
            HWB_SOURCE_SAMPLING_BACKEND,
            HWB_SOURCE_SAMPLING_MODE,
            self.controls.source_eye_mapping,
            SourceSamplingTransformStage::PostHomographyPreSourceVisibleRectThenTextureSample,
        )
        .with_content_uv_rect(source_uv_rect_from_ltrb(content_uv_rect))
        .with_source_visible_uv_rect(source_uv_rect_from_ltrb(content_uv_rect))
        .with_transform(
            HWB_SAMPLE_TRANSFORM,
            HWB_SAMPLE_TRANSFORM_OWNER,
            source_uv_rect_transform_applied(self.frame)
                || self.controls.left_texture_transform.shader_flags() != 0
                || self.controls.right_texture_transform.shader_flags() != 0,
        )
        .with_sampler(
            HWB_OUTPUT_UV_LABEL,
            HWB_SAMPLER_UV_ORIGIN,
            SourceSamplerYAxis::RendererDefined,
        )
        .with_texture_transform(
            SourceSamplingTransformStage::PostHomographyPreTextureSample,
            HWB_TEXTURE_TRANSFORM_OWNER,
        )
    }

    pub(crate) fn marker_fields(&self) -> String {
        let contract = self.contract();
        let left_source_uv_rect = source_uv_rect_ltrb_for_diagnostics(&self.frame.left.diagnostics);
        let right_source_uv_rect =
            source_uv_rect_ltrb_for_diagnostics(&self.frame.right.diagnostics);
        let source_crop_rect_state = marker_token(
            self.frame
                .left
                .diagnostics
                .source_crop_rect_state
                .as_deref()
                .or(self
                    .frame
                    .right
                    .diagnostics
                    .source_crop_rect_state
                    .as_deref()),
            "not-logged",
        );
        let source_crop_rect_owner = marker_token(
            self.frame
                .left
                .diagnostics
                .source_crop_rect_owner
                .as_deref()
                .or(self
                    .frame
                    .right
                    .diagnostics
                    .source_crop_rect_owner
                    .as_deref()),
            "not-logged",
        );
        let source_mode = source_mode_for_frame(self.frame);
        let geometry_profile = projection_geometry_profile_for_frame(self.frame, self.config);
        let source_sampling_mode = source_sampling_mode_for_frame(self.frame);
        let source_uv_contract = if source_sampling_mode == "target-local-raster" {
            "target_local_raster_uv_to_hardware_buffer_sampler"
        } else {
            HWB_SOURCE_UV_CONTRACT
        };
        let source_sample_input_uv = if source_sampling_mode == "target-local-raster" {
            "target-local-raster-uv"
        } else {
            "screen-to-camera-homography-output"
        };
        format!(
            "schema=rusty.xr.hwb-source-sampling.v1 phase=source-sampling status=ok sourceMode={} projectionGeometryProfile={} geometry_profile={} sourceSamplingMode={} sourceEyeMapping={} sourceUvContract={} sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv={} sourceSampleTransformStage={} sourceSampleTransform={} sourceSampleTransformOwner={} sourceSampleTransformApplied={} sourceSampleOutputUv={} sourceSamplerUvOrigin={} sourceSamplerYAxis={} sourceTextureTransformStage={} sourceTextureTransformOwner={} contentUvRect={} sourceVisibleUvRect={} sourceCropRectState={} sourceCropRectOwner={} leftSourceVisibleUvRect={} rightSourceVisibleUvRect={} leftSourceCropRectPx={} rightSourceCropRectPx={} leftCameraTextureTransform={} rightCameraTextureTransform={} leftCameraTextureTransformFlags={} rightCameraTextureTransformFlags={} cameraTextureTransformSource={} cameraTextureTransformReason={} {}",
            source_mode,
            geometry_profile,
            geometry_profile,
            source_sampling_mode,
            contract.source_eye_mapping.stable_id(),
            source_uv_contract,
            source_sample_input_uv,
            legacy_transform_stage_token(contract.transform_stage),
            contract.transform_label,
            contract.transform_owner,
            contract.transform_applied,
            contract.output_uv_label,
            contract.sampler_uv_origin,
            contract.sampler_y_axis.stable_id(),
            legacy_transform_stage_token(contract.texture_transform_stage),
            contract.texture_transform_owner,
            uv_rect_token(source_uv_rect_to_ltrb(contract.content_uv_rect)),
            uv_rect_token(source_uv_rect_to_ltrb(contract.source_visible_uv_rect)),
            source_crop_rect_state,
            source_crop_rect_owner,
            uv_rect_token(left_source_uv_rect),
            uv_rect_token(right_source_uv_rect),
            pixel_rect_token(self.frame.left.diagnostics.source_crop_rect_px),
            pixel_rect_token(self.frame.right.diagnostics.source_crop_rect_px),
            self.controls.left_label(),
            self.controls.right_label(),
            self.controls.left_texture_transform.shader_flags(),
            self.controls.right_texture_transform.shader_flags(),
            self.config.camera_texture_transform.source_label.as_str(),
            self.config.camera_texture_transform.reason.as_str(),
            projection_hardware_buffer_marker_fields(self.frame),
        )
    }
}

pub(crate) fn hwb_source_sampling_detail_log_message(
    frame_index: u64,
    handoff: &HwbSourceSamplingHandoff<'_>,
) -> String {
    format!(
        "Rusty XR HWB source sampling detail frame={} {}",
        frame_index,
        handoff.marker_fields()
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn hwb_stereo_draw_prepared_log_message(
    frame: &StereoGpuCameraFrame,
    controls: &StereoProjectionControls,
    config: &RuntimeConfig,
    projection_active: bool,
    display_left_camera_id: &str,
    display_right_camera_id: &str,
    import_cache_size: usize,
    stereo_descriptor_cache_size: usize,
) -> String {
    let explicit_visual_check = controls.left_texture_transform.is_explicit_visual_check()
        && controls.right_texture_transform.is_explicit_visual_check();
    let accepted_flat_visual_check =
        !projection_active && config.visual_release_accepted && explicit_visual_check;
    let orientation_accepted =
        explicit_visual_check && (projection_active || accepted_flat_visual_check);
    let pose_source = frame
        .left
        .diagnostics
        .pose_source
        .as_deref()
        .unwrap_or("missing");
    let pose_reference = frame
        .left
        .diagnostics
        .lens_pose_reference_label
        .as_deref()
        .unwrap_or("unknown");
    let pose_convention = frame
        .left
        .diagnostics
        .pose_coordinate_convention
        .as_deref()
        .unwrap_or("unknown");
    let source_sample_transform_applied = source_uv_rect_transform_applied(frame)
        || controls.left_texture_transform.shader_flags() != 0
        || controls.right_texture_transform.shader_flags() != 0;
    let active_tier = if projection_active {
        "gpu-projected"
    } else if accepted_flat_visual_check {
        "gpu-flat-visual-check"
    } else {
        "gpu-buffer-probe"
    };
    let projection_shader_path = if projection_active {
        "projected"
    } else if accepted_flat_visual_check {
        "flat-visual-check"
    } else {
        "flat-probe"
    };
    let fallback_reason = if projection_active {
        if config.visual_release_accepted {
            "projected shader path active with manual visual acceptance"
        } else {
            "projected shader path active; visual orientation/alignment acceptance still required"
        }
    } else if accepted_flat_visual_check {
        "missing per-eye projection metadata; drawing accepted flat stereo visual-check path"
    } else {
        "missing per-eye projection metadata or explicit texture orientation"
    };

    format!(
        "Rusty XR GPU stereo camera draw prepared frame {} requestedTier={} activeTier={} alignedProjection={} stereoLayout=Separate pairedLeftRightGpuBuffers=true cpuUploadCount=0 poseSource={} poseReference={} poseConvention={} projectionMode={} cameraFeedMode={} cameraColorMode={} cameraColorShaderBit={} {} cameraColorContrast={} cameraColorBrightness={} cameraColorSaturation={} cameraImportImageLayout={} importCacheLimit={} sourceEyeMapping={} displayLeftCameraId={} displayRightCameraId={} leftCameraTextureTransform={} rightCameraTextureTransform={} leftCameraTextureTransformFlags={} rightCameraTextureTransformFlags={} cameraTextureTransformSource={} cameraTextureTransformReason={} sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv=screen-to-camera-homography-output sourceSampleTransformStage=post_homography_pre_source_visible_rect_then_texture_sample sourceSampleTransform=sourceVisibleUvRect+cameraTextureTransformFlags sourceSampleTransformOwner=android-media-image-crop-rect+vulkan-hwb-camera_projection_shader sourceSampleTransformApplied={} sourceSampleOutputUv=hardware-buffer-sampler-uv sourceSamplerUvOrigin=hardware-buffer-import-convention sourceSamplerYAxis=renderer-defined sourceTextureTransformStage=post_homography_pre_texture_sample sourceTextureTransformOwner=vulkan-hwb-camera_projection_shader orientationCheck={} orientationAccepted={} visualReleaseAccepted={} orientationDiagnosticMode={} orientationDiagnosticStep={} importCacheSize={} stereoDescriptorCacheSize={} projectionShaderPath={} projectionMetadataReady={} fallbackReason={}",
        frame.index,
        config.camera_tier.stable_id(),
        active_tier,
        projection_active,
        pose_source,
        pose_reference,
        pose_convention,
        config.camera_projection_mode.stable_id(),
        config.camera_feed_pipeline_mode.stable_id(),
        config.camera_color_mode.stable_id(),
        config.camera_color_mode.shader_bit(),
        config.hwb_source_color_contract_fields(),
        config.camera_color_contrast,
        config.camera_color_brightness,
        config.camera_color_saturation,
        config.camera_import_image_layout_mode.stable_id(),
        config.camera_import_cache_limit,
        controls.source_eye_mapping.stable_id(),
        display_left_camera_id,
        display_right_camera_id,
        controls.left_label(),
        controls.right_label(),
        controls.left_texture_transform.shader_flags(),
        controls.right_texture_transform.shader_flags(),
        config.camera_texture_transform.source_label.as_str(),
        config.camera_texture_transform.reason.as_str(),
        source_sample_transform_applied,
        config.camera_texture_transform.is_explicit_visual_check(),
        orientation_accepted,
        config.visual_release_accepted,
        controls.diagnostic_mode.stable_id(),
        controls.diagnostic_step,
        import_cache_size,
        stereo_descriptor_cache_size,
        projection_shader_path,
        stereo_projection_metadata_ready(frame),
        fallback_reason
    )
}

pub(crate) fn hwb_final_projection_status_log_message(
    status: HwbFinalProjectionStatusLog<'_>,
) -> String {
    let source_sample_transform_applied = source_uv_rect_transform_applied(status.frame)
        || status.controls.left_texture_transform.shader_flags() != 0
        || status.controls.right_texture_transform.shader_flags() != 0;
    let orientation_accepted = status
        .controls
        .left_texture_transform
        .is_explicit_visual_check()
        && status
            .controls
            .right_texture_transform
            .is_explicit_visual_check();
    let visual_inspection = if status.config.visual_release_accepted {
        "accepted"
    } else {
        "required"
    };
    let temporal_metrics = status.temporal_metrics;
    let camera_cadence_metrics = status.camera_cadence_metrics;

    format!(
        "Rusty XR final projection status frame={} openXrFrameCount={} openXrFocused={} activeTier=gpu-projected alignedProjection={} {} stereoLayout=Separate pairedLeftRightGpuBuffers=true poseSource={} poseReference={} poseConvention={} projectionMode={} cameraFeedMode={} cameraColorMode={} cameraColorShaderBit={} {} cameraColorContrast={} cameraColorBrightness={} cameraColorSaturation={} cameraImportImageLayout={} importCacheLimit={} sourceEyeMapping={} displayLeftCameraId={} displayRightCameraId={} leftCameraTextureTransform={} rightCameraTextureTransform={} leftCameraTextureTransformFlags={} rightCameraTextureTransformFlags={} cameraTextureTransformSource={} cameraTextureTransformReason={} sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler sourceHomographyOutputUv=content-normalized-top-left-y-down sourceSampleInputUv=screen-to-camera-homography-output sourceSampleTransformStage=post_homography_pre_source_visible_rect_then_texture_sample sourceSampleTransform=sourceVisibleUvRect+cameraTextureTransformFlags sourceSampleTransformOwner=android-media-image-crop-rect+vulkan-hwb-camera_projection_shader sourceSampleTransformApplied={} sourceSampleOutputUv=hardware-buffer-sampler-uv sourceSamplerUvOrigin=hardware-buffer-import-convention sourceSamplerYAxis=renderer-defined sourceTextureTransformStage=post_homography_pre_texture_sample sourceTextureTransformOwner=vulkan-hwb-camera_projection_shader orientationCheck=true orientationAccepted={} cpuUploadCount=0 projectionShaderPath=projected projectionSurface={} coordinateChain=camera2-sensor-reference-to-openxr-head-basis importCacheSize={} stereoDescriptorCacheSize={} noHardwareBufferLifetimeWarnings=true frameCadenceTargetHz={} visualInspection={} visualReleaseAccepted={} orientationDiagnosticMode={} orientationDiagnosticStep={} temporalProjectionMode={} frameAdoptionMode={} frameAdoptionHeld={} frameAdoptionCandidateMotionPxP95={:.3} cameraFrameAgeMsAvg={} cameraFrameAgeMsP95={} stereoPairDeltaMsAvg={:.3} targetProjectionMotionPxAvg={:.3} targetProjectionMotionPxP95={:.3} appliedProjectionMotionPxAvg={:.3} appliedProjectionMotionPxP95={:.3} projectionResidualPxAvg={:.3} projectionResidualPxP95={:.3} visualLagMsAvg={:.3} visualLagMsP95={:.3} heldFrameCount={} heldFrameDurationMsMax={:.3} frameCrossfadeCount={} invalidUvPxPercent={:.3} edgeFillPxPercent={:.3} aswEnabledFrameCount={} aswSkippedFrameCount={} motionVectorMaxPx={:.3} motionVectorClampedCount={} cameraProjectionRenderFrameCount={} cameraDistinctFrameCount={} cameraRepeatedRenderFrameCount={} cameraRendersPerCameraFrameAvg={:.3} cameraMaxConsecutiveRenderFramesPerCameraFrame={} cameraConsumedFrameHz={:.3} cameraProjectionRenderHz={:.3}",
        status.frame.index,
        status.openxr_frame_count,
        status.openxr_focused,
        status.aligned_projection,
        status.projection_homography_fields,
        status.pose_source,
        status.pose_reference,
        status.pose_convention,
        status.config.camera_projection_mode.stable_id(),
        status.config.camera_feed_pipeline_mode.stable_id(),
        status.config.camera_color_mode.stable_id(),
        status.config.camera_color_mode.shader_bit(),
        status.config.hwb_source_color_contract_fields(),
        status.config.camera_color_contrast,
        status.config.camera_color_brightness,
        status.config.camera_color_saturation,
        status.config.camera_import_image_layout_mode.stable_id(),
        status.config.camera_import_cache_limit,
        status.controls.source_eye_mapping.stable_id(),
        status.display_left_camera_id,
        status.display_right_camera_id,
        status.controls.left_label(),
        status.controls.right_label(),
        status.controls.left_texture_transform.shader_flags(),
        status.controls.right_texture_transform.shader_flags(),
        status.config.camera_texture_transform.source_label.as_str(),
        status.config.camera_texture_transform.reason.as_str(),
        source_sample_transform_applied,
        orientation_accepted,
        status.config.camera_projection_mode.projection_surface_label(),
        status.import_cache_size,
        status.stereo_descriptor_cache_size,
        status.config.xr_display_refresh_hz,
        visual_inspection,
        status.config.visual_release_accepted,
        status.controls.diagnostic_mode.stable_id(),
        status.controls.diagnostic_step,
        status.temporal_projection_mode,
        status.config.camera_frame_adoption_mode.stable_id(),
        temporal_metrics.frame_adoption_held,
        temporal_metrics.frame_adoption_candidate_motion_px_p95,
        status.camera_frame_age_ms_avg,
        status.camera_frame_age_ms_p95,
        temporal_metrics.stereo_pair_delta_ms_avg,
        temporal_metrics.target_projection_motion_px_avg,
        temporal_metrics.target_projection_motion_px_p95,
        temporal_metrics.applied_projection_motion_px_avg,
        temporal_metrics.applied_projection_motion_px_p95,
        temporal_metrics.projection_residual_px_avg,
        temporal_metrics.projection_residual_px_p95,
        temporal_metrics.visual_lag_ms_avg,
        temporal_metrics.visual_lag_ms_p95,
        temporal_metrics.held_frame_count,
        temporal_metrics.held_frame_duration_ms_max,
        temporal_metrics.frame_crossfade_count,
        temporal_metrics.invalid_uv_px_percent,
        temporal_metrics.edge_fill_px_percent,
        temporal_metrics.asw_enabled_frame_count,
        temporal_metrics.asw_skipped_frame_count,
        temporal_metrics.motion_vector_max_px,
        temporal_metrics.motion_vector_clamped_count,
        camera_cadence_metrics.render_frame_count,
        camera_cadence_metrics.distinct_frame_count,
        camera_cadence_metrics.repeated_render_frame_count,
        camera_cadence_metrics.renders_per_camera_frame_avg,
        camera_cadence_metrics.max_consecutive_render_frames_per_camera_frame,
        camera_cadence_metrics.consumed_frame_hz,
        camera_cadence_metrics.projection_render_hz
    )
}

fn source_sampling_mode_for_frame(frame: &StereoGpuCameraFrame) -> &'static str {
    if let Some(mode) = frame
        .left
        .diagnostics
        .source_sampling_mode
        .as_deref()
        .or(frame.right.diagnostics.source_sampling_mode.as_deref())
    {
        match mode.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "target-local-raster"
            | "target-local"
            | "target-raster"
            | "local-raster"
            | "raster"
            | "default" => return "target-local-raster",
            "screen-to-camera-homography"
            | "screen-camera-homography"
            | "screen-to-source-homography"
            | "camera-homography"
            | "camera-projection"
            | "homography" => return "screen-to-camera-homography",
            _ => {}
        }
    }
    match frame
        .left
        .diagnostics
        .content_mapping_intent
        .as_deref()
        .or(frame.right.diagnostics.content_mapping_intent.as_deref())
    {
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

fn source_mode_for_frame(frame: &StereoGpuCameraFrame) -> &'static str {
    let source = frame
        .left
        .diagnostics
        .source
        .as_deref()
        .or(frame.right.diagnostics.source.as_deref())
        .unwrap_or("direct-camera2")
        .to_ascii_lowercase();
    if source.contains("synthetic") {
        "broker-synthetic"
    } else if source.contains("broker") {
        "broker-h264"
    } else {
        "direct-camera2"
    }
}

fn projection_geometry_profile_for_frame(
    frame: &StereoGpuCameraFrame,
    config: &RuntimeConfig,
) -> String {
    let fallback = if config.camera_projection_mode.uses_world_canvas() {
        "full-frame-diagnostic"
    } else {
        "camera-projection"
    };
    marker_token(
        frame
            .left
            .diagnostics
            .projection_geometry_profile
            .as_deref()
            .or(frame
                .left
                .diagnostics
                .synthetic_projection_profile
                .as_deref())
            .or(frame
                .right
                .diagnostics
                .projection_geometry_profile
                .as_deref())
            .or(frame
                .right
                .diagnostics
                .synthetic_projection_profile
                .as_deref()),
        fallback,
    )
}

fn marker_token(value: Option<&str>, fallback: &str) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .replace(char::is_whitespace, "_")
}

fn full_source_uv_rect_ltrb() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

fn source_uv_rect_from_ltrb(rect: [f32; 4]) -> SourceUvRect {
    SourceUvRect::new(
        Vec2::new(rect[0], rect[1]),
        Vec2::new(rect[2] - rect[0], rect[3] - rect[1]),
    )
}

fn source_uv_rect_to_ltrb(rect: SourceUvRect) -> [f32; 4] {
    [
        rect.origin_uv.x,
        rect.origin_uv.y,
        rect.origin_uv.x + rect.size_uv.x,
        rect.origin_uv.y + rect.size_uv.y,
    ]
}

fn source_uv_rect_ltrb_for_diagnostics(diagnostics: &HeadsetCameraFrameDiagnostics) -> [f32; 4] {
    diagnostics
        .source_visible_uv_rect
        .or(diagnostics.content_uv_rect)
        .unwrap_or_else(full_source_uv_rect_ltrb)
}

fn source_uv_rect_is_full(rect: [f32; 4]) -> bool {
    const EPSILON: f32 = 0.0005;
    (rect[0]).abs() <= EPSILON
        && (rect[1]).abs() <= EPSILON
        && (rect[2] - 1.0).abs() <= EPSILON
        && (rect[3] - 1.0).abs() <= EPSILON
}

fn source_uv_rect_transform_applied(frame: &StereoGpuCameraFrame) -> bool {
    !source_uv_rect_is_full(source_uv_rect_ltrb_for_diagnostics(&frame.left.diagnostics))
        || !source_uv_rect_is_full(source_uv_rect_ltrb_for_diagnostics(
            &frame.right.diagnostics,
        ))
}

fn stereo_projection_metadata_ready(frame: &StereoGpuCameraFrame) -> bool {
    let left_pose = frame
        .left
        .diagnostics
        .pose_source
        .as_deref()
        .map(|value| matches!(value, "platform" | "estimated-profile"))
        .unwrap_or(false);
    let right_pose = frame
        .right
        .diagnostics
        .pose_source
        .as_deref()
        .map(|value| matches!(value, "platform" | "estimated-profile"))
        .unwrap_or(false);
    frame.left.metadata.has_projection_metadata()
        && frame.right.metadata.has_projection_metadata()
        && left_pose
        && right_pose
}

fn uv_rect_token(rect: [f32; 4]) -> String {
    format!(
        "{:.6},{:.6},{:.6},{:.6}",
        rect[0], rect[1], rect[2], rect[3]
    )
}

fn pixel_rect_token(rect: Option<[u32; 4]>) -> String {
    rect.map(|[left, top, right, bottom]| format!("{left},{top},{right},{bottom}"))
        .unwrap_or_else(|| "not-logged".to_string())
}

fn optional_u64_token(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not-logged".to_string())
}

fn optional_u32_token(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not-logged".to_string())
}

fn legacy_transform_stage_token(stage: SourceSamplingTransformStage) -> &'static str {
    match stage {
        SourceSamplingTransformStage::None => "none",
        SourceSamplingTransformStage::PostHomographyPreTextureSample => {
            "post_homography_pre_texture_sample"
        }
        SourceSamplingTransformStage::PostHomographyPreOesSample => {
            "post_homography_pre_oes_sample"
        }
        SourceSamplingTransformStage::PostHomographyPreYuvSample => {
            "post_homography_pre_yuv_sample"
        }
        SourceSamplingTransformStage::PostHomographyPreSourceVisibleRectThenTextureSample => {
            "post_homography_pre_source_visible_rect_then_texture_sample"
        }
        SourceSamplingTransformStage::Other => "other",
    }
}

fn projection_hardware_buffer_marker_fields(frame: &StereoGpuCameraFrame) -> String {
    format!(
        "leftHardwareBufferWidth={} leftHardwareBufferHeight={} leftHardwareBufferNativeFormat={} leftHardwareBufferUsage={} leftHardwareBufferLayers={} leftHardwareBufferStridePx={} leftHardwareBufferId={} rightHardwareBufferWidth={} rightHardwareBufferHeight={} rightHardwareBufferNativeFormat={} rightHardwareBufferUsage={} rightHardwareBufferLayers={} rightHardwareBufferStridePx={} rightHardwareBufferId={}",
        frame.left.width,
        frame.left.height,
        optional_u64_token(frame.left.descriptor.native_format),
        optional_u64_token(frame.left.descriptor.usage_flags),
        optional_u32_token(frame.left.descriptor.layer_count),
        optional_u32_token(frame.left.descriptor.stride_px),
        optional_u64_token(frame.left.descriptor.buffer_id),
        frame.right.width,
        frame.right.height,
        optional_u64_token(frame.right.descriptor.native_format),
        optional_u64_token(frame.right.descriptor.usage_flags),
        optional_u32_token(frame.right.descriptor.layer_count),
        optional_u32_token(frame.right.descriptor.stride_px),
        optional_u64_token(frame.right.descriptor.buffer_id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CameraOrientationDiagnosticMode, CameraProjectionMode, HeadsetCameraGpuFrame,
        RuntimeConfig, StereoGpuCameraFrame, StereoProjectionControls,
    };
    use rusty_xr_contracts::{
        CameraCompositeTier, CameraFrameMetadata, CameraGpuBufferDescriptor, CameraSourceId,
        CameraTextureTransform, ImageSize, StereoMediaLayout, StereoSourceEyeMapping,
    };

    fn diagnostics(source_visible_uv_rect: Option<[f32; 4]>) -> HeadsetCameraFrameDiagnostics {
        HeadsetCameraFrameDiagnostics {
            source: Some("direct-cam".to_string()),
            camera_id: Some("50".to_string()),
            lens_facing: Some("external".to_string()),
            lens_facing_rank: Some(0),
            selection_score: Some(1),
            requested_tier: CameraCompositeTier::GpuProjected,
            active_tier_label: "gpu-projected".to_string(),
            transport: "hardware-buffer".to_string(),
            pose_source: None,
            pose_coordinate_convention: None,
            lens_pose_reference_label: None,
            diagnostic_source: None,
            synthetic_projection_profile: None,
            projection_geometry_profile: None,
            synthetic_pattern: None,
            orientation_kind: None,
            raster_orientation: None,
            upright_marker: None,
            orientation_metadata_source: None,
            orientation_default: None,
            stimulus_raster_orientation: None,
            stimulus_upright_marker: None,
            stimulus_orientation_default: None,
            content_kind: Some("camera".to_string()),
            content_width: Some(1280),
            content_height: Some(1280),
            content_aspect_ratio: Some(1.0),
            desired_display_aspect_ratio: Some(1.0),
            desired_projection_aspect_ratio: Some(1.0),
            content_coordinate_space: Some("content-normalized-top-left-y-down".to_string()),
            content_origin: Some("top-left".to_string()),
            content_x_axis: Some("right".to_string()),
            content_y_axis: Some("down".to_string()),
            content_uv_rect: None,
            source_visible_uv_rect,
            source_crop_rect_px: Some([16, 32, 1200, 1240]),
            source_crop_rect_state: Some("metadata ready".to_string()),
            source_crop_rect_owner: Some("android media image".to_string()),
            source_sampling_mode: Some("screen-to-camera-homography".to_string()),
            content_mapping_intent: None,
            content_geometry_metadata_source: None,
            content_geometry_default: None,
            target_footprint_schema: None,
            target_coordinate_space: None,
            target_screen_uv_rect: None,
            target_clip_policy: None,
            target_footprint_metadata_source: None,
            target_footprint_default: None,
            requested_stereo_layout: None,
            stereo_layout: StereoMediaLayout::Separate,
            mono_fallback: false,
            fallback_reason: String::new(),
        }
    }

    fn gpu_frame(index: u64, source_visible_uv_rect: Option<[f32; 4]>) -> HeadsetCameraGpuFrame {
        let size = ImageSize::new(1280, 1280);
        HeadsetCameraGpuFrame {
            width: size.width,
            height: size.height,
            timestamp_ns: 1000 + index as i64,
            index,
            metadata: CameraFrameMetadata::without_intrinsics(
                CameraSourceId::new("test-camera"),
                index,
                size,
            ),
            diagnostics: diagnostics(source_visible_uv_rect),
            descriptor: CameraGpuBufferDescriptor::new("test-camera", size, "hardware-buffer")
                .with_native_format(35)
                .with_usage_flags(0x100)
                .with_layer_count(1)
                .with_stride_px(1280)
                .with_buffer_id(9000 + index),
            #[cfg(target_os = "android")]
            hardware_buffer: crate::AndroidHardwareBufferHandle {
                ptr: std::ptr::null_mut(),
            },
        }
    }

    fn stereo_frame(source_visible_uv_rect: Option<[f32; 4]>) -> StereoGpuCameraFrame {
        StereoGpuCameraFrame {
            index: 1,
            left: gpu_frame(1, source_visible_uv_rect),
            right: gpu_frame(2, source_visible_uv_rect),
            pair_delta_ns: 10,
            midpoint_timestamp_ns: 1005,
        }
    }

    fn controls() -> StereoProjectionControls {
        StereoProjectionControls {
            source_eye_mapping: StereoSourceEyeMapping::DisplayLeftFromLeftSource,
            left_texture_transform: CameraTextureTransform::default(),
            right_texture_transform: CameraTextureTransform::default(),
            diagnostic_mode: CameraOrientationDiagnosticMode::Off,
            diagnostic_step: 0,
        }
    }

    #[test]
    fn hwb_handoff_reports_identity_full_rect_contract() {
        let frame = stereo_frame(None);
        let controls = controls();
        let config = RuntimeConfig::default();
        let handoff = HwbSourceSamplingHandoff::new(&frame, &controls, &config);
        let contract = handoff.contract();
        let fields = handoff.marker_fields();
        assert!(contract.is_valid());
        assert_eq!(contract.backend, "hwb");
        assert_eq!(
            contract.transform_stage,
            SourceSamplingTransformStage::PostHomographyPreSourceVisibleRectThenTextureSample
        );
        assert_eq!(
            contract.texture_transform_stage,
            SourceSamplingTransformStage::PostHomographyPreTextureSample
        );
        assert_eq!(contract.sampler_y_axis, SourceSamplerYAxis::RendererDefined);
        assert!(fields.contains("schema=rusty.xr.hwb-source-sampling.v1"));
        assert!(fields.contains("sourceMode=direct-camera2"));
        assert!(fields.contains("projectionGeometryProfile=camera-projection"));
        assert!(fields.contains("sourceSamplingMode=screen-to-camera-homography"));
        assert!(fields
            .contains("sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler"));
        assert!(fields.contains("sourceEyeMapping=display-left-from-left-source"));
        assert!(fields.contains("sourceSampleTransformApplied=false"));
        assert!(fields.contains("sourceTextureTransformStage=post_homography_pre_texture_sample"));
        assert!(fields.contains("contentUvRect=0.000000,0.000000,1.000000,1.000000"));
        assert!(fields.contains("leftHardwareBufferWidth=1280"));
    }

    #[test]
    fn hwb_handoff_reports_world_canvas_geometry_profile() {
        let frame = stereo_frame(None);
        let controls = controls();
        let mut config = RuntimeConfig::default();
        config.camera_projection_mode = CameraProjectionMode::WorldCanvas;
        let fields = HwbSourceSamplingHandoff::new(&frame, &controls, &config).marker_fields();
        assert!(fields.contains("projectionGeometryProfile=full-frame-diagnostic"));
        assert!(fields.contains("geometry_profile=full-frame-diagnostic"));
    }

    #[test]
    fn hwb_handoff_reports_source_visible_rect_transform() {
        let frame = stereo_frame(Some([0.1, 0.2, 0.9, 0.8]));
        let controls = controls();
        let config = RuntimeConfig::default();
        let handoff = HwbSourceSamplingHandoff::new(&frame, &controls, &config);
        let contract = handoff.contract();
        let fields = handoff.marker_fields();
        let visible_rect = source_uv_rect_to_ltrb(contract.source_visible_uv_rect);
        assert!((visible_rect[0] - 0.1).abs() < 0.0001);
        assert!((visible_rect[1] - 0.2).abs() < 0.0001);
        assert!((visible_rect[2] - 0.9).abs() < 0.0001);
        assert!((visible_rect[3] - 0.8).abs() < 0.0001);
        assert!(contract.transform_applied);
        assert!(fields.contains("sourceSampleTransformApplied=true"));
        assert!(fields.contains("sourceVisibleUvRect=0.100000,0.200000,0.900000,0.800000"));
        assert!(fields.contains("sourceCropRectState=metadata_ready"));
        assert!(fields.contains("sourceCropRectOwner=android_media_image"));
    }

    #[test]
    fn hwb_source_sampling_detail_log_message_keeps_prefix_shape() {
        let frame = stereo_frame(Some([0.1, 0.2, 0.9, 0.8]));
        let controls = controls();
        let config = RuntimeConfig::default();
        let handoff = HwbSourceSamplingHandoff::new(&frame, &controls, &config);

        let message = hwb_source_sampling_detail_log_message(frame.index, &handoff);

        assert!(message.starts_with(
            "Rusty XR HWB source sampling detail frame=1 schema=rusty.xr.hwb-source-sampling.v1"
        ));
        assert!(message.contains("phase=source-sampling status=ok"));
        assert!(message
            .contains("sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler"));
        assert!(message.contains("sourceVisibleUvRect=0.100000,0.200000,0.900000,0.800000"));
    }

    #[test]
    fn hwb_draw_prepared_marker_uses_source_sampling_record() {
        let frame = stereo_frame(Some([0.1, 0.2, 0.9, 0.8]));
        let controls = controls();
        let config = RuntimeConfig::default();

        let message = hwb_stereo_draw_prepared_log_message(
            &frame, &controls, &config, true, "50", "51", 3, 2,
        );

        assert!(message.starts_with("Rusty XR GPU stereo camera draw prepared frame 1"));
        assert!(message.contains("activeTier=gpu-projected alignedProjection=true"));
        assert!(message
            .contains("sourceUvContract=screen_to_camera_content_uv_to_hardware_buffer_sampler"));
        assert!(message.contains("sourceSampleTransformApplied=true"));
        assert!(message.contains("displayLeftCameraId=50 displayRightCameraId=51"));
        assert!(message.contains("importCacheSize=3 stereoDescriptorCacheSize=2"));
        assert!(message.contains("projectionShaderPath=projected"));
        assert!(message.contains(
            "fallbackReason=projected shader path active; visual orientation/alignment acceptance still required"
        ));
    }

    #[test]
    fn hwb_final_projection_status_marker_keeps_contract_shape() {
        let frame = stereo_frame(Some([0.1, 0.2, 0.9, 0.8]));
        let controls = controls();
        let config = RuntimeConfig::default();

        let message = hwb_final_projection_status_log_message(HwbFinalProjectionStatusLog {
            frame: &frame,
            controls: &controls,
            config: &config,
            openxr_frame_count: 12,
            openxr_focused: true,
            aligned_projection: true,
            projection_homography_fields:
                "projectionHomographyReady=true projectionAreaTransformStage=surface_to_camera",
            pose_source: "camera2",
            pose_reference: "openxr-head",
            pose_convention: "camera2-reference-to-openxr-head-basis",
            display_left_camera_id: "50",
            display_right_camera_id: "51",
            import_cache_size: 3,
            stereo_descriptor_cache_size: 2,
            temporal_projection_mode: "metrics-only",
            camera_frame_age_ms_avg: "4.500",
            camera_frame_age_ms_p95: "4.500",
            temporal_metrics: HwbFinalProjectionTemporalMetrics {
                frame_adoption_held: false,
                frame_adoption_candidate_motion_px_p95: 1.25,
                stereo_pair_delta_ms_avg: 0.01,
                target_projection_motion_px_avg: 2.0,
                target_projection_motion_px_p95: 3.0,
                applied_projection_motion_px_avg: 4.0,
                applied_projection_motion_px_p95: 5.0,
                projection_residual_px_avg: 6.0,
                projection_residual_px_p95: 7.0,
                visual_lag_ms_avg: 8.0,
                visual_lag_ms_p95: 9.0,
                held_frame_count: 10,
                held_frame_duration_ms_max: 11.0,
                frame_crossfade_count: 12,
                invalid_uv_px_percent: 13.0,
                edge_fill_px_percent: 14.0,
                asw_enabled_frame_count: 15,
                asw_skipped_frame_count: 16,
                motion_vector_max_px: 17.0,
                motion_vector_clamped_count: 18,
            },
            camera_cadence_metrics: HwbCameraRenderCadenceMetrics {
                render_frame_count: 19,
                distinct_frame_count: 20,
                repeated_render_frame_count: 21,
                renders_per_camera_frame_avg: 22.0,
                max_consecutive_render_frames_per_camera_frame: 23,
                consumed_frame_hz: 24.0,
                projection_render_hz: 25.0,
            },
        });

        assert!(message.starts_with(
            "Rusty XR final projection status frame=1 openXrFrameCount=12 openXrFocused=true"
        ));
        assert!(message.contains("activeTier=gpu-projected alignedProjection=true"));
        assert!(message.contains(
            "projectionHomographyReady=true projectionAreaTransformStage=surface_to_camera"
        ));
        assert!(message.contains("sourceSampleTransformApplied=true"));
        assert!(message.contains("displayLeftCameraId=50 displayRightCameraId=51"));
        assert!(message.contains("projectionShaderPath=projected"));
        assert!(message.contains("importCacheSize=3 stereoDescriptorCacheSize=2"));
        assert!(message.contains("temporalProjectionMode=metrics-only frameAdoptionMode=off"));
        assert!(message.contains("cameraFrameAgeMsAvg=4.500 cameraFrameAgeMsP95=4.500"));
        assert!(message.contains("cameraProjectionRenderFrameCount=19"));
        assert!(message.contains("cameraProjectionRenderHz=25.000"));
    }
}
