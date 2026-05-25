use ash::vk;

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::{
    gpu_camera_cache::GpuCameraImportCache,
    gpu_camera_import::{
        import_camera_hardware_buffer, query_camera_hardware_buffer_import_plan,
        transition_imported_camera_image,
    },
    gpu_camera_resource_state::GpuCameraResourceState,
    gpu_camera_resources::GpuCameraImportKey,
    log_info,
};

const GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = crate::CAMERA_IMPORT_CACHE_LIMIT_MAX;

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn prepare_camera_frame(
    ahb: &ash::android::external_memory_android_hardware_buffer::Device,
    resources: &mut GpuCameraResourceState,
    cache: &mut GpuCameraImportCache,
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    frame: &HeadsetCameraGpuFrame,
    sampler_binding_mode: crate::CameraSamplerBindingMode,
    import_image_layout_mode: crate::CameraImportImageLayoutMode,
    import_cache_limit: usize,
) -> Result<usize, String> {
    let import_cache_limit = effective_camera_import_cache_limit(import_cache_limit);
    prepare_camera_frame_with_limit(
        ahb,
        resources,
        cache,
        device,
        cmd,
        frame,
        sampler_binding_mode,
        import_image_layout_mode,
        import_cache_limit,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn prepare_stereo_camera_frame(
    ahb: &ash::android::external_memory_android_hardware_buffer::Device,
    resources: &mut GpuCameraResourceState,
    cache: &mut GpuCameraImportCache,
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    frame: &StereoGpuCameraFrame,
    sampler_binding_mode: crate::CameraSamplerBindingMode,
    import_image_layout_mode: crate::CameraImportImageLayoutMode,
    import_cache_limit: usize,
) -> Result<usize, String> {
    let import_cache_limit = effective_camera_import_cache_limit(import_cache_limit);
    let left_key = GpuCameraImportKey::from_frame(&frame.left);
    let right_key = GpuCameraImportKey::from_frame(&frame.right);
    let _left_index = prepare_camera_frame_with_limit(
        ahb,
        resources,
        cache,
        device,
        cmd,
        &frame.left,
        sampler_binding_mode,
        import_image_layout_mode,
        import_cache_limit,
    )?;
    let _right_index = prepare_camera_frame_with_limit(
        ahb,
        resources,
        cache,
        device,
        cmd,
        &frame.right,
        sampler_binding_mode,
        import_image_layout_mode,
        import_cache_limit,
    )?;

    let pipeline_resources = resources
        .resources()
        .ok_or_else(|| "GPU camera pipeline resources were not initialized".to_string())?;

    cache.ensure_stereo_descriptor(
        device,
        pipeline_resources,
        left_key,
        right_key,
        import_cache_limit,
    )
}

#[allow(clippy::too_many_arguments)]
unsafe fn prepare_camera_frame_with_limit(
    ahb: &ash::android::external_memory_android_hardware_buffer::Device,
    resources: &mut GpuCameraResourceState,
    cache: &mut GpuCameraImportCache,
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    frame: &HeadsetCameraGpuFrame,
    sampler_binding_mode: crate::CameraSamplerBindingMode,
    import_image_layout_mode: crate::CameraImportImageLayoutMode,
    import_cache_limit: usize,
) -> Result<usize, String> {
    let key = GpuCameraImportKey::from_frame(frame);
    if let Some(index) = cache.import_index(key) {
        cache.record_import_hit();
        let transition_image = if resources.resources().is_some_and(|resources| {
            resources
                .format_key
                .import_image_layout_mode
                .needs_transition()
        }) {
            cache.import_image_needing_transition(index)
        } else {
            None
        };
        if let Some(image) = transition_image {
            transition_imported_camera_image(device, cmd, image);
            cache.mark_import_layout_transitioned(index);
        }
        return Ok(index);
    }
    cache.record_import_miss();

    let import_plan = query_camera_hardware_buffer_import_plan(
        ahb,
        frame,
        sampler_binding_mode,
        import_image_layout_mode,
    )?;
    let format_key = import_plan.format_key;
    if resources.needs_format_rebuild(format_key) {
        cache.destroy_stereo_descriptors(device);
        cache.destroy_imports(device);
        resources.rebuild_for_import_plan(device, &import_plan)?;
    }

    while cache.import_count() >= import_cache_limit {
        cache.evict_oldest_import(device);
    }

    let pipeline_resources = resources
        .resources()
        .ok_or_else(|| "GPU camera pipeline resources were not initialized".to_string())?;
    let import = import_camera_hardware_buffer(
        device,
        resources.memory_properties(),
        pipeline_resources,
        frame,
        key,
        format_key,
        import_plan.allocation_size,
        import_plan.memory_type_bits,
    )?;
    let index = cache.push_import(import);
    if format_key.import_image_layout_mode.needs_transition() {
        let image = cache
            .import_image(index)
            .ok_or_else(|| "camera import was unavailable after cache insertion".to_string())?;
        transition_imported_camera_image(device, cmd, image);
    }
    cache.mark_import_layout_transitioned(index);
    let cache_stats = cache.stats();
    log_info(format!(
        "Rusty XR Vulkan imported camera hardware buffer size={}x{} nativeFormat={} externalFormat={} vkFormat={:?} samplerBindingMode={} importImageLayout={} allocationSize={} memoryTypeBits=0x{:x} suggestedYcbcrModel={:?} suggestedYcbcrRange={:?} samplerYcbcrComponents={:?} suggestedXChromaOffset={:?} suggestedYChromaOffset={:?} importCacheSize={} importCacheLimit={} importCacheMiss={} importCacheEvict={}",
        frame.width,
        frame.height,
        frame.descriptor.native_format.unwrap_or_default(),
        format_key.external_format,
        format_key.format,
        format_key.sampler_binding_mode.stable_id(),
        format_key.import_image_layout_mode.stable_id(),
        import_plan.allocation_size,
        import_plan.memory_type_bits,
        import_plan.format_props.suggested_ycbcr_model,
        import_plan.format_props.suggested_ycbcr_range,
        import_plan.format_props.sampler_ycbcr_conversion_components,
        import_plan.format_props.suggested_x_chroma_offset,
        import_plan.format_props.suggested_y_chroma_offset,
        cache_stats.import_count,
        import_cache_limit,
        cache_stats.import_cache_miss_count,
        cache_stats.import_cache_evict_count
    ));
    Ok(index)
}

fn effective_camera_import_cache_limit(limit: usize) -> usize {
    limit.clamp(2, GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX)
}
