use ash::vk;
use openxr as xr;

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::gpu_camera_resources::GpuCameraImportKey;
use super::{
    gpu_camera_cache::{GpuCameraImportCache, GpuCameraImportCacheStats},
    gpu_camera_draw::{record_camera_draw, record_stereo_camera_draw},
    gpu_camera_import::{
        import_camera_hardware_buffer, query_camera_hardware_buffer_import_plan,
        transition_imported_camera_image,
    },
    gpu_camera_resource_state::GpuCameraResourceState,
    log_info,
    projection_geometry::ProjectedStereoHomographies,
};

const GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = crate::CAMERA_IMPORT_CACHE_LIMIT_MAX;

pub(super) struct GpuCameraRenderer {
    ahb: Option<ash::android::external_memory_android_hardware_buffer::Device>,
    resources: GpuCameraResourceState,
    cache: GpuCameraImportCache,
    pub(super) last_failure: Option<String>,
}

impl GpuCameraRenderer {
    pub(super) unsafe fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        memory_properties: vk::PhysicalDeviceMemoryProperties,
        projection_uniform_alignment: vk::DeviceSize,
        render_pass: vk::RenderPass,
        import_supported: bool,
    ) -> Self {
        let ahb = import_supported.then(|| {
            ash::android::external_memory_android_hardware_buffer::Device::new(instance, device)
        });
        Self {
            ahb,
            resources: GpuCameraResourceState::new(
                memory_properties,
                projection_uniform_alignment,
                render_pass,
            ),
            cache: GpuCameraImportCache::default(),
            last_failure: None,
        }
    }

    pub(super) fn cache_stats(&self) -> GpuCameraImportCacheStats {
        self.cache.stats()
    }

    pub(super) unsafe fn prepare_frame(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &HeadsetCameraGpuFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<Option<usize>, String> {
        if self.ahb.is_none() {
            self.last_failure = Some(
                "Vulkan Android hardware-buffer import or sampler YCbCr support missing"
                    .to_string(),
            );
            return Ok(None);
        }

        match self.prepare_frame_inner(
            device,
            cmd,
            frame,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        ) {
            Ok(index) => {
                self.cache.record_import_success();
                self.last_failure = None;
                Ok(Some(index))
            }
            Err(error) => {
                self.cache.record_import_failure();
                self.last_failure = Some(error.clone());
                Err(error)
            }
        }
    }

    pub(super) unsafe fn prepare_stereo_frame(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &StereoGpuCameraFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<Option<usize>, String> {
        if self.ahb.is_none() {
            self.last_failure = Some(
                "Vulkan Android hardware-buffer import or sampler YCbCr support missing"
                    .to_string(),
            );
            return Ok(None);
        }

        match self.prepare_stereo_frame_inner(
            device,
            cmd,
            frame,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        ) {
            Ok(index) => {
                self.cache.record_import_success();
                self.last_failure = None;
                Ok(Some(index))
            }
            Err(error) => {
                self.cache.record_import_failure();
                self.last_failure = Some(error.clone());
                Err(error)
            }
        }
    }

    unsafe fn prepare_stereo_frame_inner(
        &mut self,
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
        let _left_index = self.prepare_frame_inner(
            device,
            cmd,
            &frame.left,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        )?;
        let _right_index = self.prepare_frame_inner(
            device,
            cmd,
            &frame.right,
            sampler_binding_mode,
            import_image_layout_mode,
            import_cache_limit,
        )?;

        let resources = self
            .resources
            .resources()
            .ok_or_else(|| "GPU camera pipeline resources were not initialized".to_string())?;

        self.cache.ensure_stereo_descriptor(
            device,
            resources,
            left_key,
            right_key,
            import_cache_limit,
        )
    }

    unsafe fn prepare_frame_inner(
        &mut self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        frame: &HeadsetCameraGpuFrame,
        sampler_binding_mode: crate::CameraSamplerBindingMode,
        import_image_layout_mode: crate::CameraImportImageLayoutMode,
        import_cache_limit: usize,
    ) -> Result<usize, String> {
        let import_cache_limit = effective_camera_import_cache_limit(import_cache_limit);
        let key = GpuCameraImportKey::from_frame(frame);
        if let Some(index) = self.cache.import_index(key) {
            self.cache.record_import_hit();
            let transition_image = if self.resources.resources().is_some_and(|resources| {
                resources
                    .format_key
                    .import_image_layout_mode
                    .needs_transition()
            }) {
                self.cache.import_image_needing_transition(index)
            } else {
                None
            };
            if let Some(image) = transition_image {
                transition_imported_camera_image(device, cmd, image);
                self.cache.mark_import_layout_transitioned(index);
            }
            return Ok(index);
        }
        self.cache.record_import_miss();

        let ahb = self
            .ahb
            .as_ref()
            .ok_or_else(|| "Android hardware-buffer Vulkan extension is unavailable".to_string())?;
        let import_plan = query_camera_hardware_buffer_import_plan(
            ahb,
            frame,
            sampler_binding_mode,
            import_image_layout_mode,
        )?;
        let format_key = import_plan.format_key;
        if self.resources.needs_format_rebuild(format_key) {
            self.cache.destroy_stereo_descriptors(device);
            self.cache.destroy_imports(device);
            self.resources
                .rebuild_for_import_plan(device, &import_plan)?;
        }

        while self.cache.import_count() >= import_cache_limit {
            self.cache.evict_oldest_import(device);
        }

        let resources = self
            .resources
            .resources()
            .ok_or_else(|| "GPU camera pipeline resources were not initialized".to_string())?;
        let import = import_camera_hardware_buffer(
            device,
            self.resources.memory_properties(),
            resources,
            frame,
            key,
            format_key,
            import_plan.allocation_size,
            import_plan.memory_type_bits,
        )?;
        let index = self.cache.push_import(import);
        if format_key.import_image_layout_mode.needs_transition() {
            let image = self
                .cache
                .import_image(index)
                .ok_or_else(|| "camera import was unavailable after cache insertion".to_string())?;
            transition_imported_camera_image(device, cmd, image);
        }
        self.cache.mark_import_layout_transitioned(index);
        let cache_stats = self.cache.stats();
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

    pub(super) unsafe fn record_draw(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        resolution: vk::Extent2D,
        import_index: usize,
        frame: &HeadsetCameraGpuFrame,
        config: &crate::RuntimeConfig,
    ) {
        let Some(resources) = self.resources.resources() else {
            return;
        };
        let Some(import) = self.cache.import(import_index) else {
            return;
        };

        record_camera_draw(device, cmd, resolution, resources, import, frame, config);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) unsafe fn record_draw_stereo(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        resolution: vk::Extent2D,
        descriptor_index: usize,
        frame: &StereoGpuCameraFrame,
        config: &crate::RuntimeConfig,
        views: &[xr::View],
        frame_count: u64,
        applied_projection_homographies: Option<ProjectedStereoHomographies>,
    ) {
        let Some(resources) = self.resources.resources() else {
            return;
        };
        let Some(descriptor) = self.cache.stereo_descriptor(descriptor_index) else {
            return;
        };

        record_stereo_camera_draw(
            device,
            cmd,
            resolution,
            resources,
            descriptor,
            frame,
            config,
            views,
            frame_count,
            applied_projection_homographies,
        );
    }

    pub(super) unsafe fn destroy(&mut self, device: &ash::Device) {
        self.cache.destroy_imports(device);
        self.resources.destroy(device);
    }
}

fn effective_camera_import_cache_limit(limit: usize) -> usize {
    limit.clamp(2, GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX)
}
