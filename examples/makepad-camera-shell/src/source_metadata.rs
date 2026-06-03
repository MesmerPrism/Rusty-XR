use crate::camera_texture_path::MakepadCameraTexturePath;
use crate::makepad_widgets::makepad_platform::event::video_playback::VideoTextureUpdateMetadata;
use rusty_xr_camera_model::{
    rect_xywh, target_footprint_debug_region_marker_fields, uv_rect_token, Rect2,
    SourceSamplingMode, Vec2, TARGET_SCREEN_FOOTPRINT_SCHEMA,
};
use serde_json::Value as JsonValue;

use super::{
    hotload_text_any, marker_token, MakepadCameraPair, DEFAULT_CAMERA_LEFT_TARGET_SCREEN_UV_RECT,
    DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE, DEFAULT_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT,
    DEFAULT_CAMERA_SOURCE_SAMPLING_MODE, DEFAULT_CAMERA_TARGET_SCREEN_UV_RECT,
    FRAME_RASTER_TOP_LEFT_Y_DOWN, KEY_CAMERA_LEFT_TARGET_SCREEN_UV_RECT,
    KEY_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT, KEY_CAMERA_SOURCE_SAMPLING_MODE,
    KEY_CAMERA_TARGET_SCREEN_UV_RECT, KEY_MAKEPAD_CAMERA_LEFT_TARGET_SCREEN_UV_RECT,
    KEY_MAKEPAD_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT, KEY_MAKEPAD_CAMERA_SOURCE_SAMPLING_MODE,
    KEY_MAKEPAD_CAMERA_TARGET_SCREEN_UV_RECT,
};

pub(crate) fn aspect_ratio_u32(width: u32, height: u32) -> f64 {
    if width > 0 && height > 0 {
        width as f64 / height as f64
    } else {
        1.0
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MakepadTargetScreenFootprintPair {
    pub(crate) left_rect: Rect2,
    pub(crate) right_rect: Rect2,
    pub(crate) from_metadata: bool,
    pub(crate) defaulted: bool,
}

impl MakepadTargetScreenFootprintPair {
    pub(crate) fn marker_fields(self) -> String {
        format!(
            "targetFootprintSchema={} targetCoordinateSpace=display-eye-screen-uv leftTargetScreenUvRect={} rightTargetScreenUvRect={} targetClipPolicy=clip-to-visible-eye targetFootprintMetadataSource={} targetFootprintDefault={} effectBoundary=target-footprint borderRegionSemantics=visible-render-surface-minus-target-footprint sourceInvalidSemantics=homography-path-only-target-local-stretch-clamps-edge-sample {}",
            TARGET_SCREEN_FOOTPRINT_SCHEMA,
            target_screen_uv_rect_token(self.left_rect),
            target_screen_uv_rect_token(self.right_rect),
            if self.from_metadata {
                "makepad-direct-camera-target-screen-uv-runtime"
            } else {
                "renderer-authored-fallback"
            },
            self.defaulted,
            target_footprint_debug_region_marker_fields(),
        )
    }
}

pub(crate) fn makepad_default_target_screen_uv_rect(left: bool) -> Rect2 {
    let default = if left {
        DEFAULT_CAMERA_LEFT_TARGET_SCREEN_UV_RECT
    } else {
        DEFAULT_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT
    };
    parse_uv_rect_xywh_text(default).unwrap_or(Rect2::UNIT)
}

pub(crate) fn makepad_runtime_target_screen_footprint_pair() -> MakepadTargetScreenFootprintPair {
    let shared_text = hotload_text_any(
        &[
            KEY_CAMERA_TARGET_SCREEN_UV_RECT,
            KEY_MAKEPAD_CAMERA_TARGET_SCREEN_UV_RECT,
        ],
        DEFAULT_CAMERA_TARGET_SCREEN_UV_RECT,
    );
    let left_default_text = if shared_text.is_empty() {
        DEFAULT_CAMERA_LEFT_TARGET_SCREEN_UV_RECT
    } else {
        shared_text.as_str()
    };
    let right_default_text = if shared_text.is_empty() {
        DEFAULT_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT
    } else {
        shared_text.as_str()
    };
    let left_text = hotload_text_any(
        &[
            KEY_CAMERA_LEFT_TARGET_SCREEN_UV_RECT,
            KEY_MAKEPAD_CAMERA_LEFT_TARGET_SCREEN_UV_RECT,
        ],
        left_default_text,
    );
    let right_text = hotload_text_any(
        &[
            KEY_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT,
            KEY_MAKEPAD_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT,
        ],
        right_default_text,
    );
    let left_rect = parse_uv_rect_xywh_text(&left_text)
        .unwrap_or_else(|| makepad_default_target_screen_uv_rect(true));
    let right_rect = parse_uv_rect_xywh_text(&right_text)
        .unwrap_or_else(|| makepad_default_target_screen_uv_rect(false));
    MakepadTargetScreenFootprintPair {
        left_rect,
        right_rect,
        from_metadata: true,
        defaulted: shared_text.is_empty()
            && left_text == DEFAULT_CAMERA_LEFT_TARGET_SCREEN_UV_RECT
            && right_text == DEFAULT_CAMERA_RIGHT_TARGET_SCREEN_UV_RECT,
    }
}

pub(crate) fn makepad_runtime_camera_source_sampling_mode() -> SourceSamplingMode {
    SourceSamplingMode::parse(&hotload_text_any(
        &[
            KEY_CAMERA_SOURCE_SAMPLING_MODE,
            KEY_MAKEPAD_CAMERA_SOURCE_SAMPLING_MODE,
        ],
        DEFAULT_CAMERA_SOURCE_SAMPLING_MODE,
    ))
    .unwrap_or(SourceSamplingMode::TargetLocalRaster)
}

pub(crate) fn parse_uv_rect_xywh_text(text: &str) -> Option<Rect2> {
    let parts: Vec<f32> = text
        .split(|character| matches!(character, ',' | ';' | ' ' | '\t'))
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| part.trim().parse::<f32>().ok())
        .collect();
    if parts.len() != 4 {
        return None;
    }
    let rect = Rect2::new(Vec2::new(parts[0], parts[1]), Vec2::new(parts[2], parts[3]));
    target_screen_rect_is_valid(rect).then_some(rect)
}

pub(crate) fn target_screen_uv_rect_token(rect: Rect2) -> String {
    uv_rect_token(rect_xywh(rect))
}

fn optional_target_screen_uv_rect_token(rect: Option<Rect2>) -> String {
    rect.map(target_screen_uv_rect_token)
        .unwrap_or_else(|| "missing".to_string())
}

fn target_screen_rect_is_valid(rect: Rect2) -> bool {
    rect.is_valid()
        && rect.size.x > 0.0
        && rect.size.y > 0.0
        && rect.origin.x >= 0.0
        && rect.origin.y >= 0.0
        && rect.max().x <= 1.0
        && rect.max().y <= 1.0
}

fn source_sampling_mode_from_metadata(
    projection_geometry_profile: &str,
    synthetic_projection_profile: &str,
    content_mapping_intent: &str,
) -> SourceSamplingMode {
    let profile = projection_geometry_profile
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-");
    let synthetic_profile = synthetic_projection_profile
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-");
    let intent = content_mapping_intent
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
        .replace(' ', "-");
    if SourceSamplingMode::parse(&intent).is_some_and(|mode| mode.uses_target_local_raster()) {
        return SourceSamplingMode::TargetLocalRaster;
    }
    if SourceSamplingMode::parse(&intent)
        .is_some_and(|mode| mode.uses_screen_to_camera_homography())
    {
        return SourceSamplingMode::ScreenToCameraHomography;
    }
    if matches!(
        intent.as_str(),
        "map-camera-frame-through-screen-to-camera-homography"
            | "map-stimulus-raster-through-camera-projection"
    ) {
        return SourceSamplingMode::ScreenToCameraHomography;
    }
    if profile == "camera-projection"
        || profile == "camera-matched"
        || profile == "physical-camera"
        || synthetic_profile == "camera-projection"
        || synthetic_profile == "camera-matched"
        || synthetic_profile == "physical-camera"
        || profile == "head-anchored-virtual-camera"
        || synthetic_profile == "head-anchored-virtual-camera"
    {
        return SourceSamplingMode::ScreenToCameraHomography;
    }
    SourceSamplingMode::TargetLocalRaster
}

pub(crate) fn makepad_camera_status_marker_line(
    phase: &str,
    runtime_profile: &str,
    transport_profile: &str,
    makepad_rev: &str,
    studio_host: &str,
) -> String {
    format!(
        "RUSTY_XR_MAKEPAD_CAMERA_STATUS schema=rusty.xr.makepad-camera.status.v1 phase={} profile={} transport={} renderer=makepad android_packager=cargo-makepad makepad_rev={} studio_host={}",
        phase,
        runtime_profile,
        transport_profile,
        makepad_rev,
        studio_host,
    )
}

pub(crate) fn makepad_camera2_acquisition_broker_h264_skipped_marker_line() -> &'static str {
    "RUSTY_XR_MAKEPAD_CAMERA2_ACQUISITION schema=rusty.xr.makepad-camera2.acquisition.v1 phase=start status=skipped reason=broker-h264-enabled import=broker-h264"
}

pub(crate) fn makepad_hardware_buffer_import_marker_line(body: &str) -> String {
    format!(
        "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 {}",
        body
    )
}

pub(crate) fn makepad_hardware_buffer_import_enumerated_marker_fields(
    pair: &MakepadCameraPair,
    source_count: usize,
    format_count: usize,
    left_frame_rate: &str,
    right_frame_rate: &str,
    pixel_format: &str,
) -> String {
    hardware_buffer_import_enumerated_marker_fields(
        source_count,
        format_count,
        &pair.source_binding_mode,
        pair.left.source_index,
        pair.right.source_index,
        pair.left.camera_id.as_deref().unwrap_or("unknown"),
        pair.right.camera_id.as_deref().unwrap_or("unknown"),
        pair.left.source_class,
        pair.right.source_class,
        pair.left.width,
        pair.left.height,
        pair.right.width,
        pair.right.height,
        left_frame_rate,
        right_frame_rate,
        pixel_format,
    )
}

pub(crate) fn makepad_hardware_buffer_import_enumerated_error_marker_fields(
    source_count: usize,
    format_count: usize,
) -> String {
    format!(
        "phase=enumerated status=error makepadSourceCount={} makepadFormatCount={} selected=false errorKind=no_yuv420_makepad_camera_stereo_pair",
        source_count,
        format_count,
    )
}

pub(crate) fn makepad_hardware_buffer_import_texture_updated_marker_fields(
    side_label: &str,
    yuv_enabled: bool,
    yuv_biplanar: bool,
    rotation_steps: f32,
    texture_path: MakepadCameraTexturePath,
    metadata: &VideoTextureUpdateMetadata,
    projection_border_policy: &str,
    processing_layer: &str,
) -> String {
    format!(
        "phase=texture-updated status=ok side={} yuvEnabled={} yuvBiplanar={} rotationSteps={:.0} projectionBorderPolicy={} processingLayer={} importPlan={} {}{}",
        side_label,
        yuv_enabled,
        yuv_biplanar,
        rotation_steps,
        marker_token(projection_border_policy),
        marker_token(processing_layer),
        texture_path.import_plan(),
        texture_path.marker_fields(),
        video_texture_update_metadata_marker_fields(metadata),
    )
}

fn video_texture_update_metadata_marker_fields(metadata: &VideoTextureUpdateMetadata) -> String {
    let mut fields = vec![
        format!("eventResourcePath={}", metadata.resource_path.as_str()),
        format!("descriptorShape={}", metadata.descriptor_shape.as_str()),
    ];
    if let Some(value) = metadata.camera_input_id {
        fields.push(format!("cameraInputId={}", (value.0).0));
    }
    if let Some(value) = metadata.camera_format_id {
        fields.push(format!("cameraFormatId={}", (value.0).0));
    }
    if let Some(value) = metadata.camera_frame_sequence {
        fields.push(format!("cameraFrameSeq={value}"));
    }
    if let Some(value) = metadata.camera_timestamp_ns {
        fields.push(format!("cameraTimestampNs={value}"));
    }
    if let Some(value) = metadata.acquire_time_ns {
        fields.push(format!("acquireTimeNs={value}"));
    }
    if let Some(value) = metadata.upload_sequence {
        fields.push(format!("uploadSeq={value}"));
    }
    if let Some(value) = metadata.upload_time_ns {
        fields.push(format!("uploadTimeNs={value}"));
    }
    if let Some(value) = metadata.import_sequence {
        fields.push(format!("importSeq={value}"));
    }
    if let Some(value) = metadata.import_time_ns {
        fields.push(format!("importTimeNs={value}"));
    }
    if let Some(value) = metadata.texture_update_sequence {
        fields.push(format!("textureUpdateSeq={value}"));
    }
    if metadata.width > 0 {
        fields.push(format!("textureWidth={}", metadata.width));
    }
    if metadata.height > 0 {
        fields.push(format!("textureHeight={}", metadata.height));
    }
    if let Some(value) = metadata.vulkan_format.as_deref() {
        fields.push(format!("vulkanFormat={}", marker_token(value)));
    }
    if let Some(value) = metadata.vulkan_external_format {
        fields.push(format!("vulkanExternalFormat={value}"));
    }
    #[cfg(feature = "makepad-hwb-ycbcr-metadata")]
    if let Some(ycbcr) = metadata.ycbcr_conversion.as_ref() {
        fields.push(format!(
            "suggestedYcbcrModel={}",
            marker_token(&ycbcr.suggested_model)
        ));
        fields.push(format!(
            "suggestedYcbcrRange={}",
            marker_token(&ycbcr.suggested_range)
        ));
        fields.push(format!(
            "effectiveYcbcrModel={}",
            marker_token(&ycbcr.effective_model)
        ));
        fields.push(format!(
            "effectiveYcbcrRange={}",
            marker_token(&ycbcr.effective_range)
        ));
        fields.push(format!(
            "ycbcrComponents={}",
            marker_token(&ycbcr.components)
        ));
        fields.push(format!(
            "suggestedXChromaOffset={}",
            marker_token(&ycbcr.suggested_x_chroma_offset)
        ));
        fields.push(format!(
            "suggestedYChromaOffset={}",
            marker_token(&ycbcr.suggested_y_chroma_offset)
        ));
        fields.push(format!(
            "conversionMode={}",
            marker_token(&ycbcr.conversion_mode)
        ));
        fields.push(format!(
            "samplerBindingMode={}",
            marker_token(&ycbcr.sampler_binding_mode)
        ));
        fields.push(format!(
            "samplerBindingCompliance={}",
            marker_token(&ycbcr.sampler_binding_compliance)
        ));
        fields.push(format!(
            "shaderSampleLowering={}",
            marker_token(&ycbcr.shader_sample_lowering)
        ));
        fields.push(
            "colorFixAttempt=hwb-external-combined-immutable-v4-default-sampler-remap".to_string(),
        );
    }
    if let Some(value) = metadata.resource_reused {
        fields.push(format!("resourceReused={value}"));
    }
    if metadata.fallback_active {
        fields.push("fallbackActive=true".to_string());
    }
    if let Some(value) = metadata.fallback_reason.as_deref() {
        fields.push(format!("fallbackReason={}", marker_token(value)));
    }
    format!(" {}", fields.join(" "))
}

pub(crate) fn makepad_hardware_buffer_import_complete_error_marker_fields(
    side_label: &str,
    message: &str,
) -> String {
    format!(
        "phase=complete status=error side={} errorKind=makepad_video_import_failed message={}",
        side_label,
        marker_token(message),
    )
}

pub(crate) fn makepad_hardware_buffer_import_timer_fired_marker_fields(
    source: &str,
    has_pair: bool,
    import_started: bool,
    import_finished: bool,
) -> String {
    format!(
        "phase=timer status=fired source={} hasPair={} importStarted={} importFinished={} importPlan=paired-makepad-video-hardware-buffer",
        source,
        has_pair,
        import_started,
        import_finished,
    )
}

pub(crate) fn makepad_hardware_buffer_import_timer_armed_marker_fields(
    reason: &str,
    delay_seconds: f64,
) -> String {
    format!(
        "phase=timer status=armed reason={} delaySeconds={:.1} signalFallback=true importPlan=paired-makepad-video-hardware-buffer",
        marker_token(reason),
        delay_seconds,
    )
}

pub(crate) fn makepad_stream_header_metadata_ignored_marker_fields(
    video_id: u64,
    texture_path: MakepadCameraTexturePath,
) -> String {
    format!(
        "phase=stream-header-metadata status=ignored side=unknown videoId={} reason=unexpected_video_id textureMode={} importPlan={}",
        video_id,
        texture_path.stable_id(),
        texture_path.import_plan(),
    )
}

pub(crate) fn makepad_stream_header_metadata_error_marker_fields(
    side_label: &str,
    metadata_bytes: usize,
    error: &str,
    texture_path: MakepadCameraTexturePath,
) -> String {
    format!(
        "phase=stream-header-metadata status=error side={} metadataBytes={} error={} textureMode={} importPlan={}",
        side_label,
        metadata_bytes,
        marker_token(error),
        texture_path.stable_id(),
        texture_path.import_plan(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn makepad_hardware_buffer_import_broker_h264_startup_marker_fields(
    broker_host: &str,
    broker_port: u16,
    left_stream_port: u16,
    right_stream_port: u16,
    source_mode: &str,
    decode_output_mode: &str,
    synthetic_pattern: &str,
    preferred_width: u32,
    preferred_height: u32,
    live_stream: bool,
    texture_path: MakepadCameraTexturePath,
) -> String {
    format!(
        "phase=startup status=broker-h264-enabled brokerHost={} brokerPort={} leftStreamPort={} rightStreamPort={} sourceMode={} decodeOutputMode={} syntheticPattern={} preferredWidth={} preferredHeight={} liveStream={} textureMode={} importPlan={}",
        marker_token(broker_host),
        broker_port,
        left_stream_port,
        right_stream_port,
        marker_token(source_mode),
        marker_token(decode_output_mode),
        marker_token(synthetic_pattern),
        preferred_width,
        preferred_height,
        live_stream,
        texture_path.stable_id(),
        texture_path.import_plan(),
    )
}

pub(crate) fn makepad_hardware_buffer_import_yuv_textures_ready_broker_marker_fields(
    side_label: &str,
) -> String {
    format!(
        "phase=yuv-textures-ready status=ok side={} textureMode=cpu-yuv-decoded-broker-h264 importPlan=broker-h264-stereo-mediacodec-yuv-texture",
        side_label,
    )
}

pub(crate) fn makepad_hardware_buffer_import_yuv_textures_ready_single_stream_marker_fields(
    side_label: &str,
    texture_path: MakepadCameraTexturePath,
) -> String {
    format!(
        "phase=yuv-textures-ready status=ok side={} textureMode=makepad-yuv-slot-allocation requestedTexturePath={} importPlan={} depthClip=false environmentDepthClip=false",
        side_label,
        texture_path.stable_id(),
        texture_path.import_plan(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn makepad_hardware_buffer_import_texture_handle_ready_marker_fields(
    side_label: &str,
    texture_handle: u32,
    broker_host: &str,
    broker_port: u16,
    stream_port: u16,
    source_mode: &str,
    synthetic_pattern: &str,
    live_stream: bool,
) -> String {
    format!(
        "phase=texture-handle-ready status=ok side={} textureHandle={} textureMode=external-oes brokerHost={} brokerPort={} streamPort={} sourceMode={} syntheticPattern={} liveStream={} importPlan=broker-h264-stereo-surface-texture",
        side_label,
        texture_handle,
        marker_token(broker_host),
        broker_port,
        stream_port,
        marker_token(source_mode),
        marker_token(synthetic_pattern),
        live_stream,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn makepad_hardware_buffer_import_broker_h264_prepare_request_marker_fields(
    side_label: &str,
    broker_host: &str,
    broker_port: u16,
    stream_port: u16,
    source_mode: &str,
    decode_output_mode: &str,
    synthetic_pattern: &str,
    live_stream: bool,
    texture_path: MakepadCameraTexturePath,
) -> String {
    format!(
        "phase=broker-h264-prepare-request status=sent side={} textureHandle=0 textureMode={} brokerHost={} brokerPort={} streamPort={} sourceMode={} decodeOutputMode={} syntheticPattern={} liveStream={} importPlan={}",
        side_label,
        texture_path.stable_id(),
        marker_token(broker_host),
        broker_port,
        stream_port,
        marker_token(source_mode),
        marker_token(decode_output_mode),
        marker_token(synthetic_pattern),
        live_stream,
        texture_path.import_plan(),
    )
}

pub(crate) fn makepad_hardware_buffer_import_prepared_marker_fields(
    side_label: &str,
    width: u32,
    height: u32,
    texture_path: MakepadCameraTexturePath,
) -> String {
    format!(
        "phase=prepared status=ok side={} width={} height={} importPath={} textureMode={} importPlan={} {}",
        side_label,
        width,
        height,
        texture_path.texture_import_path(),
        texture_path.stable_id(),
        texture_path.import_plan(),
        texture_path.marker_fields(),
    )
}

pub(crate) fn makepad_hardware_buffer_import_start_error_marker_fields() -> &'static str {
    "phase=start status=error errorKind=no_makepad_camera_stereo_pair"
}

pub(crate) fn makepad_hardware_buffer_import_start_waiting_marker_fields(
    wait_count: usize,
) -> String {
    format!(
        "phase=start status=waiting waitCount={} reason=no_makepad_camera_stereo_pair_yet",
        wait_count,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn makepad_hardware_buffer_import_start_marker_fields(
    pair: &MakepadCameraPair,
    broker_h264_enabled: bool,
    left_frame_rate: &str,
    right_frame_rate: &str,
    pixel_format: &str,
    left_stream_port: &str,
    right_stream_port: &str,
    delayed_after_acquisition_seconds: f64,
    texture_path: MakepadCameraTexturePath,
) -> String {
    hardware_buffer_import_start_marker_fields(
        broker_h264_enabled,
        pair.left.source_index,
        pair.right.source_index,
        pair.left.source_class,
        pair.right.source_class,
        pair.left.width,
        pair.left.height,
        pair.right.width,
        pair.right.height,
        left_frame_rate,
        right_frame_rate,
        pixel_format,
        left_stream_port,
        right_stream_port,
        delayed_after_acquisition_seconds,
        texture_path,
    )
}

pub(crate) fn makepad_hardware_buffer_import_raw_video_event_marker_line(
    event_name: &str,
    side_label: &str,
    video_id: u64,
    left_video_id: u64,
    right_video_id: u64,
) -> String {
    format!(
        "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=raw-video-event status=seen event={} side={} videoId={} leftVideoId={} rightVideoId={} depthClip=false environmentDepthClip=false importPlan=makepad-video-texture-event",
        event_name,
        side_label,
        video_id,
        left_video_id,
        right_video_id,
    )
}

#[allow(clippy::too_many_arguments)]
fn hardware_buffer_import_enumerated_marker_fields(
    source_count: usize,
    format_count: usize,
    source_binding_mode: &str,
    left_source_index: usize,
    right_source_index: usize,
    left_camera_id: &str,
    right_camera_id: &str,
    left_source_class: &str,
    right_source_class: &str,
    left_width: usize,
    left_height: usize,
    right_width: usize,
    right_height: usize,
    left_frame_rate: &str,
    right_frame_rate: &str,
    pixel_format: &str,
) -> String {
    format!(
        "phase=enumerated status=ok makepadSourceCount={} makepadFormatCount={} selected=true importPlan=paired-makepad-video-hardware-buffer sourceBindingMode={} leftSourceIndex={} rightSourceIndex={} leftCameraId={} rightCameraId={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftFrameRate={} rightFrameRate={} pixelFormat={}",
        source_count,
        format_count,
        source_binding_mode,
        left_source_index,
        right_source_index,
        marker_token(left_camera_id),
        marker_token(right_camera_id),
        left_source_class,
        right_source_class,
        left_width,
        left_height,
        right_width,
        right_height,
        left_frame_rate,
        right_frame_rate,
        pixel_format,
    )
}

#[allow(clippy::too_many_arguments)]
fn hardware_buffer_import_start_marker_fields(
    broker_h264_enabled: bool,
    left_source_index: usize,
    right_source_index: usize,
    left_source_class: &str,
    right_source_class: &str,
    left_width: usize,
    left_height: usize,
    right_width: usize,
    right_height: usize,
    left_frame_rate: &str,
    right_frame_rate: &str,
    pixel_format: &str,
    left_stream_port: &str,
    right_stream_port: &str,
    delayed_after_acquisition_seconds: f64,
    texture_path: MakepadCameraTexturePath,
) -> String {
    let texture_format = match texture_path {
        MakepadCameraTexturePath::BrokerH264CpuYuv => "VideoYuvPlaneStereo",
        MakepadCameraTexturePath::DirectCpuYuvPlane => "VideoYuvPlane",
        MakepadCameraTexturePath::DirectHardwareBufferExternal
        | MakepadCameraTexturePath::DirectHardwareBufferYuvPlane
        | MakepadCameraTexturePath::BrokerH264HardwareBuffer
        | MakepadCameraTexturePath::BrokerH264SurfaceTexture => "VideoExternal",
    };
    let (import_plan, import_path) = (
        texture_path.import_plan(),
        texture_path.texture_import_path(),
    );
    format!(
        "phase=start status=started importPlan={} brokerH264Enabled={} leftSourceIndex={} rightSourceIndex={} leftSourceClass={} rightSourceClass={} leftWidth={} leftHeight={} rightWidth={} rightHeight={} leftFrameRate={} rightFrameRate={} pixelFormat={} leftStreamPort={} rightStreamPort={} importPath={} textureMode={} textureFormat={} depthClip=false environmentDepthClip=false delayedAfterAcquisitionSeconds={:.0}",
        import_plan,
        broker_h264_enabled,
        left_source_index,
        right_source_index,
        left_source_class,
        right_source_class,
        left_width,
        left_height,
        right_width,
        right_height,
        left_frame_rate,
        right_frame_rate,
        pixel_format,
        left_stream_port,
        right_stream_port,
        import_path,
        texture_path.stable_id(),
        texture_format,
        delayed_after_acquisition_seconds,
    )
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
    pub(crate) source_sampling_mode: SourceSamplingMode,
    pub(crate) content_geometry_metadata_source: String,
    pub(crate) content_geometry_default: bool,
    pub(crate) source_valid_uv_rect: Rect2,
    pub(crate) target_footprint_schema: Option<String>,
    pub(crate) target_coordinate_space: String,
    pub(crate) target_screen_uv_rect: Option<Rect2>,
    pub(crate) target_clip_policy: String,
    pub(crate) target_footprint_metadata_source: String,
    pub(crate) target_footprint_default: bool,
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
        let source_sampling_mode = json_string_any(object, &["sourceSamplingMode"])
            .and_then(SourceSamplingMode::parse)
            .unwrap_or_else(|| {
                source_sampling_mode_from_metadata(
                    &projection_geometry_profile,
                    &synthetic_projection_profile,
                    &content_mapping_intent,
                )
            });
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
        let target_footprint_schema =
            json_string_any(object, &["targetFootprintSchema"]).map(str::to_string);
        let target_coordinate_space = json_string_any(object, &["targetCoordinateSpace"])
            .unwrap_or("display-eye-screen-uv")
            .to_string();
        let target_screen_uv_rect =
            json_rect2_xywh_any(object, &["targetScreenUvRect", "targetUvRect"]);
        let target_clip_policy = json_string_any(object, &["targetClipPolicy"])
            .unwrap_or("clip-to-visible-eye")
            .to_string();
        let target_footprint_metadata_source =
            json_string_any(object, &["targetFootprintMetadataSource"])
                .unwrap_or("missing")
                .to_string();
        let explicit_target_footprint = target_footprint_schema.is_some()
            || target_screen_uv_rect.is_some()
            || object.contains_key("targetCoordinateSpace")
            || object.contains_key("targetClipPolicy")
            || object.contains_key("targetFootprintMetadataSource");
        let target_footprint_default = !explicit_target_footprint
            || json_bool_any(object, &["targetFootprintDefault"]).unwrap_or(false);
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
            source_sampling_mode,
            content_geometry_metadata_source,
            content_geometry_default,
            source_valid_uv_rect,
            target_footprint_schema,
            target_coordinate_space,
            target_screen_uv_rect,
            target_clip_policy,
            target_footprint_metadata_source,
            target_footprint_default,
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
        (self.source_sampling_mode.uses_screen_to_camera_homography()
            && !self.requests_head_anchored_projection_area_mapping())
            || self.projection_profile_is("camera-matched")
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
        self.source_sampling_mode.uses_target_local_raster()
            || self.projection_profile_is("full-frame-diagnostic")
            || self.content_mapping_intent_is_any(&[
                "target-local-raster",
                "map-camera-frame-to-full-frame-projection-surface",
                "map-camera-frame-to-full-frame-projection-area",
                "map-full-frame-stimulus-to-projection-surface",
                "map-full-frame-stimulus-to-projection-area",
                "map-full-frame-content-to-projection-area",
            ])
    }

    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn requests_explicit_full_frame_content_mapping(&self) -> bool {
        self.source_sampling_mode.uses_target_local_raster()
            || self.content_mapping_intent_is_any(&[
                "target-local-raster",
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
        if self.requests_camera_projection_mapping() {
            "camera-projection"
        } else if self.requests_full_frame_projection_area_mapping() {
            "full-frame-diagnostic"
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

#[derive(Clone, Debug)]
struct ContentGeometryRecord {
    kind: String,
    width: u32,
    height: u32,
    aspect_ratio: f64,
    desired_display_aspect_ratio: f64,
    desired_projection_aspect_ratio: f64,
    coordinate_space: String,
    origin: String,
    x_axis: String,
    y_axis: String,
    mapping_intent: String,
    source_sampling_mode: SourceSamplingMode,
    metadata_source: String,
    metadata_default: bool,
}

impl ContentGeometryRecord {
    fn from_broker_metadata(metadata: &BrokerH264ProjectionMetadata) -> Self {
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
            source_sampling_mode: metadata.source_sampling_mode,
            metadata_source: metadata.content_geometry_metadata_source.clone(),
            metadata_default: metadata.content_geometry_default,
        }
    }

    fn direct_camera2(width: u32, height: u32, mapping_intent: &str) -> Self {
        let aspect_ratio = aspect_ratio_u32(width, height);
        Self {
            kind: "camera-frame".to_string(),
            width,
            height,
            aspect_ratio,
            desired_display_aspect_ratio: aspect_ratio,
            desired_projection_aspect_ratio: aspect_ratio,
            coordinate_space: "normalized-uv".to_string(),
            origin: "top-left".to_string(),
            x_axis: "right".to_string(),
            y_axis: "down".to_string(),
            mapping_intent: mapping_intent.to_string(),
            source_sampling_mode: source_sampling_mode_from_metadata(
                mapping_intent,
                mapping_intent,
                mapping_intent,
            ),
            metadata_source: "makepad-direct-camera2-import".to_string(),
            metadata_default: false,
        }
    }

    fn missing_broker() -> Self {
        Self {
            kind: "default-fallback".to_string(),
            width: 0,
            height: 0,
            aspect_ratio: 1.0,
            desired_display_aspect_ratio: 1.0,
            desired_projection_aspect_ratio: 1.0,
            coordinate_space: "normalized-uv".to_string(),
            origin: "top-left".to_string(),
            x_axis: "right".to_string(),
            y_axis: "down".to_string(),
            mapping_intent: "standard-missing-metadata-fallback".to_string(),
            source_sampling_mode: SourceSamplingMode::TargetLocalRaster,
            metadata_source: "missing".to_string(),
            metadata_default: true,
        }
    }
}

#[derive(Clone, Debug)]
struct StereoContentGeometryRecord {
    left: ContentGeometryRecord,
    right: ContentGeometryRecord,
    fallback_reason: &'static str,
}

impl StereoContentGeometryRecord {
    fn from_broker_pair(
        left: &BrokerH264ProjectionMetadata,
        right: &BrokerH264ProjectionMetadata,
    ) -> Self {
        Self {
            left: ContentGeometryRecord::from_broker_metadata(left),
            right: ContentGeometryRecord::from_broker_metadata(right),
            fallback_reason: "none",
        }
    }

    fn direct_camera2(width: u32, height: u32, mapping_intent: &str) -> Self {
        let left = ContentGeometryRecord::direct_camera2(width, height, mapping_intent);
        Self {
            right: left.clone(),
            left,
            fallback_reason: "none",
        }
    }

    fn missing_broker() -> Self {
        let left = ContentGeometryRecord::missing_broker();
        Self {
            right: left.clone(),
            left,
            fallback_reason: "broker-h264-content-geometry-metadata-missing",
        }
    }

    fn marker_fields(&self) -> String {
        format!(
            "leftContentKind={} rightContentKind={} leftContentWidth={} leftContentHeight={} rightContentWidth={} rightContentHeight={} leftContentAspectRatio={:.6} rightContentAspectRatio={:.6} leftDesiredDisplayAspectRatio={:.6} rightDesiredDisplayAspectRatio={:.6} leftDesiredProjectionAspectRatio={:.6} rightDesiredProjectionAspectRatio={:.6} leftContentCoordinateSpace={} rightContentCoordinateSpace={} leftContentOrigin={} rightContentOrigin={} leftContentXAxis={} rightContentXAxis={} leftContentYAxis={} rightContentYAxis={} leftContentMappingIntent={} rightContentMappingIntent={} leftSourceSamplingMode={} rightSourceSamplingMode={} leftContentGeometryMetadataSource={} rightContentGeometryMetadataSource={} leftContentGeometryDefault={} rightContentGeometryDefault={} contentGeometryFallbackReason={}",
            marker_token(&self.left.kind),
            marker_token(&self.right.kind),
            self.left.width,
            self.left.height,
            self.right.width,
            self.right.height,
            self.left.aspect_ratio,
            self.right.aspect_ratio,
            self.left.desired_display_aspect_ratio,
            self.right.desired_display_aspect_ratio,
            self.left.desired_projection_aspect_ratio,
            self.right.desired_projection_aspect_ratio,
            marker_token(&self.left.coordinate_space),
            marker_token(&self.right.coordinate_space),
            marker_token(&self.left.origin),
            marker_token(&self.right.origin),
            marker_token(&self.left.x_axis),
            marker_token(&self.right.x_axis),
            marker_token(&self.left.y_axis),
            marker_token(&self.right.y_axis),
            marker_token(&self.left.mapping_intent),
            marker_token(&self.right.mapping_intent),
            self.left.source_sampling_mode.stable_id(),
            self.right.source_sampling_mode.stable_id(),
            marker_token(&self.left.metadata_source),
            marker_token(&self.right.metadata_source),
            self.left.metadata_default,
            self.right.metadata_default,
            self.fallback_reason,
        )
    }
}

pub(crate) fn broker_pair_content_geometry_marker_fields(
    left: &BrokerH264ProjectionMetadata,
    right: &BrokerH264ProjectionMetadata,
) -> String {
    StereoContentGeometryRecord::from_broker_pair(left, right).marker_fields()
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BrokerProjectionPlanKind {
    FullFrameContent,
    CameraProjection,
    HeadAnchoredProjectionArea,
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrokerProjectionPlanDecision {
    pub(crate) kind: BrokerProjectionPlanKind,
    pub(crate) source_binding_mode: &'static str,
    pub(crate) projection_geometry_profile: String,
    pub(crate) camera_matched_live_fallback_allowed: bool,
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn broker_projection_plan_decision(
    left: &BrokerH264ProjectionMetadata,
    right: &BrokerH264ProjectionMetadata,
) -> Option<BrokerProjectionPlanDecision> {
    let full_frame_projection =
        left.is_full_frame_diagnostic_projection() && right.is_full_frame_diagnostic_projection();
    let explicit_full_frame_content_mapping = left.requests_explicit_full_frame_content_mapping()
        && right.requests_explicit_full_frame_content_mapping();
    let metadata_backed_projection =
        left.has_camera_projection_metadata() && right.has_camera_projection_metadata();
    let camera_projection_mapping =
        left.requests_camera_projection_mapping() && right.requests_camera_projection_mapping();
    let head_anchored_projection = left.requests_head_anchored_projection_area_mapping()
        && right.requests_head_anchored_projection_area_mapping();
    let camera_matched = camera_projection_mapping
        && left.projection_profile_is("camera-matched")
        && right.projection_profile_is("camera-matched");

    let kind = if explicit_full_frame_content_mapping
        || (full_frame_projection && !metadata_backed_projection)
    {
        BrokerProjectionPlanKind::FullFrameContent
    } else if (metadata_backed_projection && full_frame_projection) || camera_projection_mapping {
        BrokerProjectionPlanKind::CameraProjection
    } else if head_anchored_projection {
        BrokerProjectionPlanKind::HeadAnchoredProjectionArea
    } else {
        return None;
    };

    let source_binding_mode = if full_frame_projection {
        "broker-h264-stream-header-full-frame-diagnostic"
    } else if camera_matched {
        "broker-h264-stream-header-camera-matched"
    } else if camera_projection_mapping {
        "broker-h264-stream-header-camera-projection"
    } else if head_anchored_projection {
        "broker-h264-stream-header-head-anchored"
    } else {
        "broker-h264-stream-header"
    };
    let projection_geometry_profile = if full_frame_projection {
        "full-frame-diagnostic".to_string()
    } else if camera_matched {
        "camera-matched".to_string()
    } else if camera_projection_mapping {
        left.projection_mapping_profile_id().to_string()
    } else if head_anchored_projection {
        "head-anchored-virtual-camera".to_string()
    } else {
        left.projection_geometry_profile.clone()
    };

    Some(BrokerProjectionPlanDecision {
        kind,
        source_binding_mode,
        projection_geometry_profile,
        camera_matched_live_fallback_allowed: camera_matched,
    })
}

pub(crate) enum MakepadContentGeometrySource<'a> {
    BrokerH264 {
        left: Option<&'a BrokerH264ProjectionMetadata>,
        right: Option<&'a BrokerH264ProjectionMetadata>,
    },
    DirectCamera2 {
        width: usize,
        height: usize,
        projection_geometry_profile: &'a str,
    },
}

pub(crate) fn makepad_content_geometry_marker_fields(
    source: MakepadContentGeometrySource<'_>,
) -> String {
    match source {
        MakepadContentGeometrySource::BrokerH264 {
            left: Some(left),
            right: Some(right),
        } => broker_pair_content_geometry_marker_fields(left, right),
        MakepadContentGeometrySource::BrokerH264 { .. } => {
            missing_broker_content_geometry_marker_fields()
        }
        MakepadContentGeometrySource::DirectCamera2 {
            width,
            height,
            projection_geometry_profile,
        } => direct_camera2_content_geometry_marker_fields(
            width,
            height,
            projection_geometry_profile,
        ),
    }
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
    let projection_geometry_profile =
        normalize_direct_camera_projection_geometry_profile(projection_geometry_profile);
    let source_sampling_mode = makepad_runtime_camera_source_sampling_mode();
    let content_mapping_intent = if source_sampling_mode.uses_target_local_raster() {
        "target-local-raster"
    } else {
        match projection_geometry_profile.as_str() {
            "full-frame-diagnostic" => "target-local-raster",
            "camera-projection" => "map-camera-frame-through-screen-to-camera-homography",
            _ => "unsupported-direct-camera-projection-geometry-profile",
        }
    };
    let content_geometry = StereoContentGeometryRecord::direct_camera2(
        content_width,
        content_height,
        content_mapping_intent,
    );
    let target_footprint = makepad_runtime_target_screen_footprint_pair();
    format!(
        "projectionGeometryProfile={} geometry_profile={} {} {}",
        projection_geometry_profile,
        projection_geometry_profile,
        content_geometry.marker_fields(),
        target_footprint.marker_fields(),
    )
}

pub(crate) fn missing_broker_content_geometry_marker_fields() -> String {
    StereoContentGeometryRecord::missing_broker().marker_fields()
}

pub(crate) fn stream_header_metadata_marker_fields(
    side_label: &str,
    metadata: &BrokerH264ProjectionMetadata,
    texture_path: MakepadCameraTexturePath,
) -> String {
    let content_geometry = ContentGeometryRecord::from_broker_metadata(metadata);
    format!(
        "phase=stream-header-metadata status=ok side={} metadataBytes={} cameraId={} projectionMetadataReady={} poseSource={} poseCoordinateConvention={} source={} projectionGeometryProfile={} geometry_profile={} syntheticPattern={} orientationKind={} rasterOrientation={} uprightMarker={} orientationMetadataSource={} orientationDefault={} stimulusRasterOrientation={} stimulusUprightMarker={} stimulusOrientationDefault={} deliveredWidth={} deliveredHeight={} contentKind={} contentWidth={} contentHeight={} contentAspectRatio={:.6} desiredDisplayAspectRatio={:.6} desiredProjectionAspectRatio={:.6} contentCoordinateSpace={} contentOrigin={} contentXAxis={} contentYAxis={} contentMappingIntent={} sourceSamplingMode={} contentGeometryMetadataSource={} contentGeometryDefault={} sourceValidUvRect={} targetFootprintSchema={} targetCoordinateSpace={} targetScreenUvRect={} targetClipPolicy={} targetFootprintMetadataSource={} targetFootprintDefault={} effectBoundary=target-footprint textureMode={} importPlan={}",
        side_label,
        metadata.metadata_bytes,
        marker_token(&metadata.camera_id),
        metadata.projection_metadata_ready,
        marker_token(&metadata.pose_source),
        marker_token(&metadata.pose_coordinate_convention),
        marker_token(&metadata.source),
        marker_token(&metadata.projection_geometry_profile),
        marker_token(&metadata.projection_geometry_profile),
        marker_token(&metadata.synthetic_pattern),
        marker_token(&metadata.orientation_kind),
        marker_token(&metadata.raster_orientation),
        marker_token(&metadata.upright_marker),
        marker_token(&metadata.orientation_metadata_source),
        metadata.orientation_default,
        marker_token(&metadata.stimulus_raster_orientation),
        marker_token(&metadata.stimulus_upright_marker),
        metadata.stimulus_orientation_default,
        metadata.delivered_width,
        metadata.delivered_height,
        marker_token(&content_geometry.kind),
        content_geometry.width,
        content_geometry.height,
        content_geometry.aspect_ratio,
        content_geometry.desired_display_aspect_ratio,
        content_geometry.desired_projection_aspect_ratio,
        marker_token(&content_geometry.coordinate_space),
        marker_token(&content_geometry.origin),
        marker_token(&content_geometry.x_axis),
        marker_token(&content_geometry.y_axis),
        marker_token(&content_geometry.mapping_intent),
        content_geometry.source_sampling_mode.stable_id(),
        marker_token(&content_geometry.metadata_source),
        content_geometry.metadata_default,
        uv_rect_token(rect_xywh(metadata.source_valid_uv_rect)),
        marker_token(
            metadata
                .target_footprint_schema
                .as_deref()
                .unwrap_or(TARGET_SCREEN_FOOTPRINT_SCHEMA)
        ),
        marker_token(&metadata.target_coordinate_space),
        optional_target_screen_uv_rect_token(metadata.target_screen_uv_rect),
        marker_token(&metadata.target_clip_policy),
        marker_token(&metadata.target_footprint_metadata_source),
        metadata.target_footprint_default,
        texture_path.stable_id(),
        texture_path.import_plan(),
    )
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
        .filter(|rect| target_screen_rect_is_valid(*rect))
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
    parse_uv_rect_xywh_text(value.as_str()?)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_status_marker_keeps_source_metadata_shape() {
        assert_eq!(
            makepad_camera_status_marker_line(
                "startup",
                "fast-visual",
                "direct-camera",
                "185020114",
                "studio.local"
            ),
            "RUSTY_XR_MAKEPAD_CAMERA_STATUS schema=rusty.xr.makepad-camera.status.v1 phase=startup profile=fast-visual transport=direct-camera renderer=makepad android_packager=cargo-makepad makepad_rev=185020114 studio_host=studio.local"
        );
    }

    #[test]
    fn broker_h264_camera2_skip_marker_keeps_acquisition_shape() {
        assert_eq!(
            makepad_camera2_acquisition_broker_h264_skipped_marker_line(),
            "RUSTY_XR_MAKEPAD_CAMERA2_ACQUISITION schema=rusty.xr.makepad-camera2.acquisition.v1 phase=start status=skipped reason=broker-h264-enabled import=broker-h264"
        );
    }

    #[test]
    fn hardware_buffer_import_marker_line_keeps_prefix_shape() {
        assert_eq!(
            makepad_hardware_buffer_import_marker_line("phase=complete status=ok"),
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=complete status=ok"
        );
    }

    #[test]
    fn direct_camera2_content_geometry_uses_stereo_record() {
        let fields =
            direct_camera2_content_geometry_marker_fields(1280, 720, "camera2-platform-unprofiled");

        assert!(fields.contains(
            "projectionGeometryProfile=camera-projection geometry_profile=camera-projection"
        ));
        assert!(fields.contains("leftContentKind=camera-frame rightContentKind=camera-frame"));
        assert!(
            fields.contains("leftContentMappingIntent=target-local-raster rightContentMappingIntent=target-local-raster")
        );
        assert!(
            fields.contains("leftSourceSamplingMode=target-local-raster rightSourceSamplingMode=target-local-raster")
        );
        assert!(fields.contains("targetFootprintSchema=rusty.xr.target_screen_footprint.v1"));
        assert!(fields.contains("leftTargetScreenUvRect=0.171875,0.218750,0.750000,0.656250"));
        assert!(fields.contains("rightTargetScreenUvRect=0.078125,0.218750,0.750000,0.671875"));
    }

    #[test]
    fn missing_broker_content_geometry_uses_fallback_record() {
        assert_eq!(
            missing_broker_content_geometry_marker_fields(),
            "leftContentKind=default-fallback rightContentKind=default-fallback leftContentWidth=0 leftContentHeight=0 rightContentWidth=0 rightContentHeight=0 leftContentAspectRatio=1.000000 rightContentAspectRatio=1.000000 leftDesiredDisplayAspectRatio=1.000000 rightDesiredDisplayAspectRatio=1.000000 leftDesiredProjectionAspectRatio=1.000000 rightDesiredProjectionAspectRatio=1.000000 leftContentCoordinateSpace=normalized-uv rightContentCoordinateSpace=normalized-uv leftContentOrigin=top-left rightContentOrigin=top-left leftContentXAxis=right rightContentXAxis=right leftContentYAxis=down rightContentYAxis=down leftContentMappingIntent=standard-missing-metadata-fallback rightContentMappingIntent=standard-missing-metadata-fallback leftSourceSamplingMode=target-local-raster rightSourceSamplingMode=target-local-raster leftContentGeometryMetadataSource=missing rightContentGeometryMetadataSource=missing leftContentGeometryDefault=true rightContentGeometryDefault=true contentGeometryFallbackReason=broker-h264-content-geometry-metadata-missing"
        );
    }

    #[test]
    fn makepad_content_geometry_source_selects_direct_and_missing_broker_records() {
        let direct =
            makepad_content_geometry_marker_fields(MakepadContentGeometrySource::DirectCamera2 {
                width: 1280,
                height: 720,
                projection_geometry_profile: "full-frame-diagnostic",
            });
        assert!(direct.contains("projectionGeometryProfile=full-frame-diagnostic"));
        assert!(direct.contains("leftContentKind=camera-frame"));
        assert!(direct.contains("contentGeometryFallbackReason=none"));

        let missing =
            makepad_content_geometry_marker_fields(MakepadContentGeometrySource::BrokerH264 {
                left: None,
                right: None,
            });
        assert!(missing.contains("leftContentKind=default-fallback"));
        assert!(missing.contains(
            "contentGeometryFallbackReason=broker-h264-content-geometry-metadata-missing"
        ));
    }

    #[test]
    fn broker_content_geometry_marker_fields_use_parsed_records() {
        let left = BrokerH264ProjectionMetadata::parse(
            r#"{
                "projectionMetadataReady": true,
                "contentKind": "broker synthetic",
                "contentWidth": 640,
                "contentHeight": 480,
                "contentAspectRatio": 1.333333,
                "desiredDisplayAspectRatio": 1.333333,
                "desiredProjectionAspectRatio": 1.333333,
                "contentMappingIntent": "map broker stimulus",
                "contentGeometryMetadataSource": "stream header",
                "contentGeometryDefault": false
            }"#,
        )
        .unwrap();
        let right = BrokerH264ProjectionMetadata::parse(
            r#"{
                "projectionMetadataReady": true,
                "contentKind": "broker camera",
                "contentWidth": 800,
                "contentHeight": 600,
                "contentMappingIntent": "map broker camera",
                "contentGeometryMetadataSource": "stream header",
                "contentGeometryDefault": false
            }"#,
        )
        .unwrap();

        let fields = broker_pair_content_geometry_marker_fields(&left, &right);

        assert!(fields.contains("leftContentKind=broker_synthetic"));
        assert!(fields.contains("rightContentKind=broker_camera"));
        assert!(fields.contains("leftContentWidth=640 leftContentHeight=480"));
        assert!(fields.contains("rightContentWidth=800 rightContentHeight=600"));
        assert!(fields.contains("leftContentMappingIntent=map_broker_stimulus"));
        assert!(fields.contains("rightContentMappingIntent=map_broker_camera"));
        assert!(fields.contains("contentGeometryFallbackReason=none"));
    }

    #[test]
    fn broker_projection_plan_decision_is_metadata_driven() {
        let camera = BrokerH264ProjectionMetadata::parse(
            r#"{
                "projectionMetadataReady": true,
                "projectionGeometryProfile": "full-frame-diagnostic",
                "sourceSamplingMode": "screen-to-camera-homography",
                "contentMappingIntent": "map-camera-frame-to-full-frame-projection-area",
                "intrinsics": {"fx": 1024.0, "fy": 1024.0, "cx": 640.0, "cy": 640.0},
                "extrinsics": {"px": 0.0, "py": 0.0, "pz": 0.0, "qx": 0.0, "qy": 0.0, "qz": 0.0, "qw": 1.0}
            }"#,
        )
        .unwrap();
        let camera_decision = broker_projection_plan_decision(&camera, &camera).unwrap();
        assert_eq!(
            camera_decision.kind,
            BrokerProjectionPlanKind::CameraProjection
        );
        assert_eq!(
            camera_decision.source_binding_mode,
            "broker-h264-stream-header-full-frame-diagnostic"
        );
        assert_eq!(
            camera_decision.projection_geometry_profile,
            "full-frame-diagnostic"
        );

        let synthetic = BrokerH264ProjectionMetadata::parse(
            r#"{
                "projectionMetadataReady": true,
                "projectionGeometryProfile": "full-frame-diagnostic",
                "contentMappingIntent": "map-full-frame-stimulus-to-projection-surface"
            }"#,
        )
        .unwrap();
        let synthetic_decision = broker_projection_plan_decision(&synthetic, &synthetic).unwrap();
        assert_eq!(
            synthetic_decision.kind,
            BrokerProjectionPlanKind::FullFrameContent
        );

        let camera_matched = BrokerH264ProjectionMetadata::parse(
            r#"{
                "projectionMetadataReady": true,
                "projectionGeometryProfile": "camera-matched"
            }"#,
        )
        .unwrap();
        let fallback_decision =
            broker_projection_plan_decision(&camera_matched, &camera_matched).unwrap();
        assert_eq!(
            fallback_decision.kind,
            BrokerProjectionPlanKind::CameraProjection
        );
        assert!(fallback_decision.camera_matched_live_fallback_allowed);

        let head_anchored = BrokerH264ProjectionMetadata::parse(
            r#"{
                "projectionMetadataReady": true,
                "projectionGeometryProfile": "head-anchored-virtual-camera"
            }"#,
        )
        .unwrap();
        let head_anchored_decision =
            broker_projection_plan_decision(&head_anchored, &head_anchored).unwrap();
        assert_eq!(
            head_anchored_decision.kind,
            BrokerProjectionPlanKind::HeadAnchoredProjectionArea
        );
    }

    #[test]
    fn stream_header_metadata_marker_uses_content_geometry_record() {
        let metadata = BrokerH264ProjectionMetadata::parse(
            r#"{
                "projectionMetadataReady": true,
                "cameraId": "left camera",
                "source": "broker_app.synthetic_h264_stream",
                "poseSource": "stream header",
                "poseCoordinateConvention": "camera2 lens",
                "projectionGeometryProfile": "full-frame-diagnostic",
                "syntheticPattern": "uv grid",
                "contentKind": "broker synthetic",
                "contentWidth": 640,
                "contentHeight": 480,
                "contentMappingIntent": "map full frame content",
                "contentGeometryMetadataSource": "stream header",
                "contentGeometryDefault": false,
                "sourceValidUvRect": [0.1, 0.2, 0.8, 0.6]
            }"#,
        )
        .unwrap();

        let fields = stream_header_metadata_marker_fields(
            "left",
            &metadata,
            MakepadCameraTexturePath::BrokerH264CpuYuv,
        );

        assert!(fields.starts_with("phase=stream-header-metadata status=ok side=left"));
        assert!(fields.contains("cameraId=left_camera"));
        assert!(fields.contains("source=broker_app.synthetic_h264_stream"));
        assert!(fields.contains("contentKind=broker_synthetic"));
        assert!(fields.contains("contentWidth=640 contentHeight=480"));
        assert!(fields.contains("contentMappingIntent=map_full_frame_content"));
        assert!(fields.contains("sourceValidUvRect=0.100000,0.200000,0.800000,0.600000"));
        assert!(fields.ends_with(
            "textureMode=broker-h264-mediacodec-cpu-yuv importPlan=broker-h264-stereo-mediacodec-yuv-texture"
        ));
    }

    #[test]
    fn hardware_buffer_import_enumerated_markers_keep_source_metadata_shape() {
        let fields = hardware_buffer_import_enumerated_marker_fields(
            2,
            4,
            "camera2-direct",
            0,
            1,
            "camera 0",
            "camera 1",
            "back",
            "back",
            1280,
            720,
            1280,
            720,
            "30.00",
            "30.00",
            "YUV420",
        );

        assert!(fields.starts_with(
            "phase=enumerated status=ok makepadSourceCount=2 makepadFormatCount=4 selected=true"
        ));
        assert!(fields.contains("sourceBindingMode=camera2-direct"));
        assert!(fields.contains("leftCameraId=camera_0 rightCameraId=camera_1"));
        assert!(fields.contains("leftFrameRate=30.00 rightFrameRate=30.00 pixelFormat=YUV420"));

        let error = makepad_hardware_buffer_import_enumerated_error_marker_fields(0, 0);
        assert_eq!(
            error,
            "phase=enumerated status=error makepadSourceCount=0 makepadFormatCount=0 selected=false errorKind=no_yuv420_makepad_camera_stereo_pair"
        );
    }

    #[test]
    fn hardware_buffer_import_lifecycle_markers_keep_source_metadata_shape() {
        assert_eq!(
            makepad_hardware_buffer_import_texture_updated_marker_fields(
                "left",
                true,
                false,
                2.0,
                MakepadCameraTexturePath::DirectCpuYuvPlane,
                &VideoTextureUpdateMetadata::default(),
                "solid-red",
                "raw",
            ),
            "phase=texture-updated status=ok side=left yuvEnabled=true yuvBiplanar=false rotationSteps=2 projectionBorderPolicy=solid-red processingLayer=raw importPlan=paired-camera-cpu-yuv-fallback cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane visualColorStatus=accepted-cpu-yuv-reference eventResourcePath=unspecified descriptorShape=unspecified"
        );
        assert_eq!(
            makepad_hardware_buffer_import_complete_error_marker_fields(
                "right",
                "decoder failed",
            ),
            "phase=complete status=error side=right errorKind=makepad_video_import_failed message=decoder_failed"
        );
        assert_eq!(
            makepad_hardware_buffer_import_timer_fired_marker_fields(
                "signal-fallback",
                true,
                false,
                false,
            ),
            "phase=timer status=fired source=signal-fallback hasPair=true importStarted=false importFinished=false importPlan=paired-makepad-video-hardware-buffer"
        );
        assert_eq!(
            makepad_hardware_buffer_import_timer_armed_marker_fields("stereo pair retry", 0.5),
            "phase=timer status=armed reason=stereo_pair_retry delaySeconds=0.5 signalFallback=true importPlan=paired-makepad-video-hardware-buffer"
        );
        assert_eq!(
            makepad_hardware_buffer_import_start_error_marker_fields(),
            "phase=start status=error errorKind=no_makepad_camera_stereo_pair"
        );
        assert_eq!(
            makepad_hardware_buffer_import_start_waiting_marker_fields(3),
            "phase=start status=waiting waitCount=3 reason=no_makepad_camera_stereo_pair_yet"
        );
    }

    #[test]
    fn broker_h264_import_markers_keep_source_metadata_shape() {
        assert_eq!(
            makepad_hardware_buffer_import_texture_handle_ready_marker_fields(
                "left",
                42,
                "127.0.0.1",
                8765,
                8879,
                "broker camera",
                "uv grid",
                true,
            ),
            "phase=texture-handle-ready status=ok side=left textureHandle=42 textureMode=external-oes brokerHost=127.0.0.1 brokerPort=8765 streamPort=8879 sourceMode=broker_camera syntheticPattern=uv_grid liveStream=true importPlan=broker-h264-stereo-surface-texture"
        );
        assert_eq!(
            makepad_hardware_buffer_import_broker_h264_prepare_request_marker_fields(
                "right",
                "127.0.0.1",
                8765,
                8880,
                "synthetic h264",
                "hardware buffer",
                "diagnostic grid",
                false,
                MakepadCameraTexturePath::BrokerH264HardwareBuffer,
            ),
            "phase=broker-h264-prepare-request status=sent side=right textureHandle=0 textureMode=broker-h264-mediacodec-hardware-buffer brokerHost=127.0.0.1 brokerPort=8765 streamPort=8880 sourceMode=synthetic_h264 decodeOutputMode=hardware_buffer syntheticPattern=diagnostic_grid liveStream=false importPlan=broker-h264-stereo-mediacodec-hardware-buffer"
        );
        assert_eq!(
            makepad_hardware_buffer_import_broker_h264_startup_marker_fields(
                "127.0.0.1",
                8765,
                8879,
                8880,
                "broker camera",
                "cpu-yuv",
                "uv grid",
                1280,
                720,
                true,
                MakepadCameraTexturePath::BrokerH264CpuYuv,
            ),
            "phase=startup status=broker-h264-enabled brokerHost=127.0.0.1 brokerPort=8765 leftStreamPort=8879 rightStreamPort=8880 sourceMode=broker_camera decodeOutputMode=cpu-yuv syntheticPattern=uv_grid preferredWidth=1280 preferredHeight=720 liveStream=true textureMode=broker-h264-mediacodec-cpu-yuv importPlan=broker-h264-stereo-mediacodec-yuv-texture"
        );
        assert_eq!(
            makepad_stream_header_metadata_ignored_marker_fields(
                99,
                MakepadCameraTexturePath::BrokerH264HardwareBuffer,
            ),
            "phase=stream-header-metadata status=ignored side=unknown videoId=99 reason=unexpected_video_id textureMode=broker-h264-mediacodec-hardware-buffer importPlan=broker-h264-stereo-mediacodec-hardware-buffer"
        );
        assert_eq!(
            makepad_stream_header_metadata_error_marker_fields(
                "left",
                123,
                "bad json",
                MakepadCameraTexturePath::BrokerH264HardwareBuffer,
            ),
            "phase=stream-header-metadata status=error side=left metadataBytes=123 error=bad_json textureMode=broker-h264-mediacodec-hardware-buffer importPlan=broker-h264-stereo-mediacodec-hardware-buffer"
        );
        assert_eq!(
            makepad_hardware_buffer_import_yuv_textures_ready_broker_marker_fields("left"),
            "phase=yuv-textures-ready status=ok side=left textureMode=cpu-yuv-decoded-broker-h264 importPlan=broker-h264-stereo-mediacodec-yuv-texture"
        );
        assert_eq!(
            makepad_hardware_buffer_import_yuv_textures_ready_single_stream_marker_fields(
                "right",
                MakepadCameraTexturePath::DirectCpuYuvPlane,
            ),
            "phase=yuv-textures-ready status=ok side=right textureMode=makepad-yuv-slot-allocation requestedTexturePath=direct-camera-cpu-yuv-plane importPlan=paired-camera-cpu-yuv-fallback depthClip=false environmentDepthClip=false"
        );
        assert_eq!(
            makepad_hardware_buffer_import_prepared_marker_fields(
                "left",
                1280,
                720,
                MakepadCameraTexturePath::DirectCpuYuvPlane,
            ),
            "phase=prepared status=ok side=left width=1280 height=720 importPath=makepad-camera-cpu-yuv-plane textureMode=direct-camera-cpu-yuv-plane importPlan=paired-camera-cpu-yuv-fallback cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane visualColorStatus=accepted-cpu-yuv-reference"
        );
        assert_eq!(
            makepad_hardware_buffer_import_texture_updated_marker_fields(
                "left",
                true,
                false,
                0.0,
                MakepadCameraTexturePath::DirectCpuYuvPlane,
                &VideoTextureUpdateMetadata::default(),
                "solid-red",
                "raw",
            ),
            "phase=texture-updated status=ok side=left yuvEnabled=true yuvBiplanar=false rotationSteps=0 projectionBorderPolicy=solid-red processingLayer=raw importPlan=paired-camera-cpu-yuv-fallback cameraTexturePath=direct-camera-cpu-yuv-plane makepadVulkanImport=false textureImportPath=makepad-camera-cpu-yuv-plane cpuUploadPath=makepad-camera-cpu-yuv-plane visualColorStatus=accepted-cpu-yuv-reference eventResourcePath=unspecified descriptorShape=unspecified"
        );
        assert_eq!(
            hardware_buffer_import_start_marker_fields(
                true,
                0,
                1,
                "broker-h264",
                "broker-h264",
                1280,
                720,
                1280,
                720,
                "30.00",
                "30.00",
                "YUV420",
                "8879",
                "8880",
                0.25,
                MakepadCameraTexturePath::BrokerH264CpuYuv,
            ),
            "phase=start status=started importPlan=broker-h264-stereo-mediacodec-yuv-texture brokerH264Enabled=true leftSourceIndex=0 rightSourceIndex=1 leftSourceClass=broker-h264 rightSourceClass=broker-h264 leftWidth=1280 leftHeight=720 rightWidth=1280 rightHeight=720 leftFrameRate=30.00 rightFrameRate=30.00 pixelFormat=YUV420 leftStreamPort=8879 rightStreamPort=8880 importPath=broker-h264-mediacodec-cpu-yuv textureMode=broker-h264-mediacodec-cpu-yuv textureFormat=VideoYuvPlaneStereo depthClip=false environmentDepthClip=false delayedAfterAcquisitionSeconds=0"
        );
        assert_eq!(
            hardware_buffer_import_start_marker_fields(
                true,
                0,
                1,
                "broker-h264",
                "broker-h264",
                1280,
                720,
                1280,
                720,
                "50.00",
                "50.00",
                "PRIVATE",
                "8879",
                "8880",
                0.25,
                MakepadCameraTexturePath::BrokerH264HardwareBuffer,
            ),
            "phase=start status=started importPlan=broker-h264-stereo-mediacodec-hardware-buffer brokerH264Enabled=true leftSourceIndex=0 rightSourceIndex=1 leftSourceClass=broker-h264 rightSourceClass=broker-h264 leftWidth=1280 leftHeight=720 rightWidth=1280 rightHeight=720 leftFrameRate=50.00 rightFrameRate=50.00 pixelFormat=PRIVATE leftStreamPort=8879 rightStreamPort=8880 importPath=broker-h264-mediacodec-hardware-buffer-vulkan-import textureMode=broker-h264-mediacodec-hardware-buffer textureFormat=VideoExternal depthClip=false environmentDepthClip=false delayedAfterAcquisitionSeconds=0"
        );
        assert_eq!(
            makepad_hardware_buffer_import_raw_video_event_marker_line(
                "texture-updated",
                "left",
                100,
                100,
                101,
            ),
            "RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT schema=rusty.xr.makepad-hardware-buffer-import.v1 phase=raw-video-event status=seen event=texture-updated side=left videoId=100 leftVideoId=100 rightVideoId=101 depthClip=false environmentDepthClip=false importPlan=makepad-video-texture-event"
        );
    }
}
