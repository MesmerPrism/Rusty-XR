use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::CameraCompositeTier;

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::gpu_camera_resources::{
    GpuCameraFormatKey, GpuCameraImport, GpuCameraImportKey, GpuCameraPipelineResources,
    GpuCameraStereoDescriptor,
};
use super::{
    gpu_camera_import::{import_camera_hardware_buffer, transition_imported_camera_image},
    gpu_camera_pipeline::{
        allocate_camera_descriptor_set, create_gpu_camera_pipeline_resources,
        update_camera_projection_uniforms,
    },
    gpu_camera_projection::{
        source_uv_rect_xywh_for_frame, CameraProjectionPush, CameraProjectionUniforms,
    },
    log_error, log_info,
    projection_geometry::ProjectedStereoHomographies,
};

const GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = crate::CAMERA_IMPORT_CACHE_LIMIT_MAX;

pub(super) struct GpuCameraRenderer {
    ahb: Option<ash::android::external_memory_android_hardware_buffer::Device>,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    projection_uniform_alignment: vk::DeviceSize,
    render_pass: vk::RenderPass,
    resources: Option<GpuCameraPipelineResources>,
    pub(super) imports: Vec<GpuCameraImport>,
    pub(super) stereo_descriptors: Vec<GpuCameraStereoDescriptor>,
    pub(super) import_success_count: u64,
    pub(super) import_failure_count: u64,
    pub(super) import_cache_hit_count: u64,
    pub(super) import_cache_miss_count: u64,
    pub(super) import_cache_evict_count: u64,
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
            memory_properties,
            projection_uniform_alignment,
            render_pass,
            resources: None,
            imports: Vec::new(),
            stereo_descriptors: Vec::new(),
            import_success_count: 0,
            import_failure_count: 0,
            import_cache_hit_count: 0,
            import_cache_miss_count: 0,
            import_cache_evict_count: 0,
            last_failure: None,
        }
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
                self.import_success_count = self.import_success_count.saturating_add(1);
                self.last_failure = None;
                Ok(Some(index))
            }
            Err(error) => {
                self.import_failure_count = self.import_failure_count.saturating_add(1);
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
                self.import_success_count = self.import_success_count.saturating_add(1);
                self.last_failure = None;
                Ok(Some(index))
            }
            Err(error) => {
                self.import_failure_count = self.import_failure_count.saturating_add(1);
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

        if let Some(index) = self.stereo_descriptors.iter().position(|descriptor| {
            descriptor.left_key == left_key && descriptor.right_key == right_key
        }) {
            return Ok(index);
        }

        let left_import = self
            .imports
            .iter()
            .find(|import| import.key == left_key)
            .ok_or_else(|| {
                "left stereo camera import was evicted before descriptor binding".to_string()
            })?;
        let right_import = self
            .imports
            .iter()
            .find(|import| import.key == right_key)
            .ok_or_else(|| {
                "right stereo camera import was evicted before descriptor binding".to_string()
            })?;
        let resources = self
            .resources
            .as_ref()
            .ok_or_else(|| "GPU camera pipeline resources were not initialized".to_string())?;

        while self.stereo_descriptors.len() >= import_cache_limit {
            let old = self.stereo_descriptors.remove(0);
            old.destroy(device);
        }

        let descriptor_set = allocate_camera_descriptor_set(
            device,
            resources,
            left_import.image_view,
            right_import.image_view,
        )?;
        self.stereo_descriptors.push(GpuCameraStereoDescriptor {
            left_key,
            right_key,
            descriptor_set,
            descriptor_pool: resources.descriptor_pool,
        });
        Ok(self.stereo_descriptors.len() - 1)
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
        if let Some(index) = self.imports.iter().position(|import| import.key == key) {
            self.import_cache_hit_count = self.import_cache_hit_count.saturating_add(1);
            if self.imports[index].needs_layout_transition
                && self.resources.as_ref().is_some_and(|resources| {
                    resources
                        .format_key
                        .import_image_layout_mode
                        .needs_transition()
                })
            {
                transition_imported_camera_image(device, cmd, self.imports[index].image);
                self.imports[index].needs_layout_transition = false;
            }
            return Ok(index);
        }
        self.import_cache_miss_count = self.import_cache_miss_count.saturating_add(1);

        let mut format_props = vk::AndroidHardwareBufferFormatPropertiesANDROID::default();
        let mut properties =
            vk::AndroidHardwareBufferPropertiesANDROID::default().push_next(&mut format_props);
        let ahb = self
            .ahb
            .as_ref()
            .ok_or_else(|| "Android hardware-buffer Vulkan extension is unavailable".to_string())?;
        ahb.get_android_hardware_buffer_properties(
            frame.hardware_buffer.as_ptr().cast(),
            &mut properties,
        )
        .map_err(|error| format!("query AHardwareBuffer Vulkan properties: {error}"))?;
        let allocation_size = properties.allocation_size;
        let memory_type_bits = properties.memory_type_bits;

        let format_key = GpuCameraFormatKey {
            format: if format_props.external_format != 0 {
                vk::Format::UNDEFINED
            } else {
                format_props.format
            },
            external_format: format_props.external_format,
            sampler_binding_mode,
            import_image_layout_mode,
        };
        if self
            .resources
            .as_ref()
            .map(|resources| resources.format_key != format_key)
            .unwrap_or(true)
        {
            self.destroy_stereo_descriptors(device);
            self.destroy_imports(device);
            self.destroy_resources(device);
            self.resources = Some(create_gpu_camera_pipeline_resources(
                device,
                &self.memory_properties,
                self.projection_uniform_alignment,
                self.render_pass,
                format_key,
                &format_props,
            )?);
        }

        while self.imports.len() >= import_cache_limit {
            let old = self.imports.remove(0);
            self.destroy_stereo_descriptors_for_key(device, old.key);
            old.destroy(device);
            self.import_cache_evict_count = self.import_cache_evict_count.saturating_add(1);
        }

        let resources = self
            .resources
            .as_ref()
            .ok_or_else(|| "GPU camera pipeline resources were not initialized".to_string())?;
        let import = import_camera_hardware_buffer(
            device,
            &self.memory_properties,
            resources,
            frame,
            key,
            format_key,
            allocation_size,
            memory_type_bits,
        )?;
        self.imports.push(import);
        let index = self.imports.len() - 1;
        if format_key.import_image_layout_mode.needs_transition() {
            transition_imported_camera_image(device, cmd, self.imports[index].image);
        }
        self.imports[index].needs_layout_transition = false;
        log_info(format!(
            "Rusty XR Vulkan imported camera hardware buffer size={}x{} nativeFormat={} externalFormat={} vkFormat={:?} samplerBindingMode={} importImageLayout={} allocationSize={} memoryTypeBits=0x{:x} suggestedYcbcrModel={:?} suggestedYcbcrRange={:?} samplerYcbcrComponents={:?} suggestedXChromaOffset={:?} suggestedYChromaOffset={:?} importCacheSize={} importCacheLimit={} importCacheMiss={} importCacheEvict={}",
            frame.width,
            frame.height,
            frame.descriptor.native_format.unwrap_or_default(),
            format_key.external_format,
            format_key.format,
            format_key.sampler_binding_mode.stable_id(),
            format_key.import_image_layout_mode.stable_id(),
            allocation_size,
            memory_type_bits,
            format_props.suggested_ycbcr_model,
            format_props.suggested_ycbcr_range,
            format_props.sampler_ycbcr_conversion_components,
            format_props.suggested_x_chroma_offset,
            format_props.suggested_y_chroma_offset,
            self.imports.len(),
            import_cache_limit,
            self.import_cache_miss_count,
            self.import_cache_evict_count
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
        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let Some(import) = self.imports.get(import_index) else {
            return;
        };

        let viewport = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: resolution.width as f32,
            height: resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissor = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: resolution,
        }];
        let push = CameraProjectionPush::from_frame(frame, config);
        let source_uv_rect = source_uv_rect_xywh_for_frame(frame);
        let uniforms = CameraProjectionUniforms::identity()
            .with_source_uv_rects(source_uv_rect, source_uv_rect)
            .with_color_config(config);
        let uniform_offset = resources.projection_uniform_offset(0);
        if let Err(error) =
            update_camera_projection_uniforms(device, resources, uniform_offset, &uniforms)
        {
            log_error(format!(
                "Rusty XR update mono camera projection uniforms failed: {error}"
            ));
            return;
        }
        device.cmd_set_viewport(cmd, 0, &viewport);
        device.cmd_set_scissor(cmd, 0, &scissor);
        device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_for_config(config),
        );
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_layout,
            0,
            &[import.descriptor_set],
            &[uniform_offset],
        );
        let push_bytes = std::slice::from_raw_parts(
            (&push as *const CameraProjectionPush).cast::<u8>(),
            std::mem::size_of::<CameraProjectionPush>(),
        );
        device.cmd_push_constants(
            cmd,
            resources.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            push_bytes,
        );
        let vertex_count = if config.camera_projection_mode.uses_world_canvas() {
            6
        } else {
            3
        };
        device.cmd_draw(cmd, vertex_count, 1, 0, 0);
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
        let Some(resources) = self.resources.as_ref() else {
            return;
        };
        let Some(descriptor) = self.stereo_descriptors.get(descriptor_index) else {
            return;
        };

        let viewport = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: resolution.width as f32,
            height: resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissor = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: resolution,
        }];
        let controls = config.stereo_projection_controls(frame_count);
        let (push, uniforms, projection_homographies) =
            if let Some(homographies) = applied_projection_homographies {
                let (push, uniforms) = CameraProjectionPush::from_projected_stereo_homographies(
                    frame,
                    config,
                    &controls,
                    &homographies,
                );
                (push, uniforms, Some(homographies))
            } else {
                CameraProjectionPush::from_stereo_frame(frame, config, &controls, views, resolution)
            };
        let projection_active = projection_homographies.is_some();
        let accepted_flat_visual_check = config.visual_release_accepted
            && controls.left_texture_transform.is_explicit_visual_check()
            && controls.right_texture_transform.is_explicit_visual_check();
        if config.camera_tier == CameraCompositeTier::GpuProjected
            && !projection_active
            && !accepted_flat_visual_check
        {
            return;
        }
        let uniform_offset = resources.projection_uniform_offset(frame_count);
        if let Err(error) =
            update_camera_projection_uniforms(device, resources, uniform_offset, &uniforms)
        {
            log_error(format!(
                "Rusty XR update stereo camera projection uniforms failed: {error}"
            ));
            return;
        }
        device.cmd_set_viewport(cmd, 0, &viewport);
        device.cmd_set_scissor(cmd, 0, &scissor);
        device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_for_config(config),
        );
        device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::GRAPHICS,
            resources.pipeline_layout,
            0,
            &[descriptor.descriptor_set],
            &[uniform_offset],
        );
        let push_bytes = std::slice::from_raw_parts(
            (&push as *const CameraProjectionPush).cast::<u8>(),
            std::mem::size_of::<CameraProjectionPush>(),
        );
        device.cmd_push_constants(
            cmd,
            resources.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            push_bytes,
        );
        let vertex_count = if config.camera_projection_mode.uses_world_canvas() && projection_active
        {
            6
        } else {
            3
        };
        device.cmd_draw(cmd, vertex_count, 1, 0, 0);
    }

    pub(super) unsafe fn destroy(&mut self, device: &ash::Device) {
        self.destroy_imports(device);
        self.destroy_resources(device);
    }

    unsafe fn destroy_imports(&mut self, device: &ash::Device) {
        self.destroy_stereo_descriptors(device);
        for import in self.imports.drain(..) {
            import.destroy(device);
        }
    }

    unsafe fn destroy_stereo_descriptors_for_key(
        &mut self,
        device: &ash::Device,
        key: GpuCameraImportKey,
    ) {
        let mut index = 0;
        while index < self.stereo_descriptors.len() {
            if self.stereo_descriptors[index].left_key == key
                || self.stereo_descriptors[index].right_key == key
            {
                let old = self.stereo_descriptors.remove(index);
                old.destroy(device);
            } else {
                index += 1;
            }
        }
    }

    unsafe fn destroy_stereo_descriptors(&mut self, device: &ash::Device) {
        for descriptor in self.stereo_descriptors.drain(..) {
            descriptor.destroy(device);
        }
    }

    unsafe fn destroy_resources(&mut self, device: &ash::Device) {
        if let Some(resources) = self.resources.take() {
            resources.destroy(device);
        }
    }
}

fn effective_camera_import_cache_limit(limit: usize) -> usize {
    limit.clamp(2, GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX)
}
