use std::time::Instant;

use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::{full_view_content_uv_scale, CameraCompositeTier};

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::gpu_camera_resources::{
    GpuCameraFormatKey, GpuCameraImport, GpuCameraImportKey, GpuCameraPipelineResources,
    GpuCameraStereoDescriptor,
};
use super::{
    allocate_camera_descriptor_set, create_gpu_camera_pipeline_resources,
    effective_camera_import_cache_limit, import_camera_hardware_buffer, log_error, log_info,
    projection_geometry::{
        identity_homography, pack_homography_row, projected_stereo_homographies,
        DisplayEyeProjectionMapping, ProjectedStereoHomographies,
    },
    source_metadata::source_uv_rect_ltrb_for_diagnostics,
    transition_imported_camera_image, update_camera_projection_uniforms,
};

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

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct CameraProjectionPush {
    params: [f32; 4],
    color_adjust: [f32; 4],
    effect_params: [f32; 4],
    alpha_params: [f32; 4],
    area_params: [f32; 4],
    area_offset_params: [f32; 4],
    left_h0: [f32; 4],
    left_h1: [f32; 4],
    left_h2: [f32; 4],
    right_h0: [f32; 4],
    right_h1: [f32; 4],
    right_h2: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct CameraProjectionUniforms {
    left_screen_to_surface_h0: [f32; 4],
    left_screen_to_surface_h1: [f32; 4],
    left_screen_to_surface_h2: [f32; 4],
    right_screen_to_surface_h0: [f32; 4],
    right_screen_to_surface_h1: [f32; 4],
    right_screen_to_surface_h2: [f32; 4],
    left_surface_to_screen_h0: [f32; 4],
    left_surface_to_screen_h1: [f32; 4],
    left_surface_to_screen_h2: [f32; 4],
    right_surface_to_screen_h0: [f32; 4],
    right_surface_to_screen_h1: [f32; 4],
    right_surface_to_screen_h2: [f32; 4],
    color_matrix_r0: [f32; 4],
    color_matrix_r1: [f32; 4],
    color_matrix_r2: [f32; 4],
    color_offset: [f32; 4],
    left_source_uv_rect: [f32; 4],
    right_source_uv_rect: [f32; 4],
    left_canvas_clip0: [f32; 4],
    left_canvas_clip1: [f32; 4],
    left_canvas_clip2: [f32; 4],
    left_canvas_clip3: [f32; 4],
    right_canvas_clip0: [f32; 4],
    right_canvas_clip1: [f32; 4],
    right_canvas_clip2: [f32; 4],
    right_canvas_clip3: [f32; 4],
}

impl CameraProjectionUniforms {
    fn identity() -> Self {
        let h = identity_homography();
        Self::from_rows(&h, &h, &h, &h)
    }

    fn from_mappings(
        left: &DisplayEyeProjectionMapping,
        right: &DisplayEyeProjectionMapping,
    ) -> Self {
        let mut uniforms = Self::from_rows(
            &left.screen_to_surface,
            &right.screen_to_surface,
            &left.surface_to_screen,
            &right.surface_to_screen,
        );
        uniforms.left_canvas_clip0 = left.canvas_clip[0];
        uniforms.left_canvas_clip1 = left.canvas_clip[1];
        uniforms.left_canvas_clip2 = left.canvas_clip[2];
        uniforms.left_canvas_clip3 = left.canvas_clip[3];
        uniforms.right_canvas_clip0 = right.canvas_clip[0];
        uniforms.right_canvas_clip1 = right.canvas_clip[1];
        uniforms.right_canvas_clip2 = right.canvas_clip[2];
        uniforms.right_canvas_clip3 = right.canvas_clip[3];
        uniforms
    }

    fn from_rows(
        left_screen_to_surface: &[[f32; 3]; 3],
        right_screen_to_surface: &[[f32; 3]; 3],
        left_surface_to_screen: &[[f32; 3]; 3],
        right_surface_to_screen: &[[f32; 3]; 3],
    ) -> Self {
        Self {
            left_screen_to_surface_h0: pack_homography_row(left_screen_to_surface[0]),
            left_screen_to_surface_h1: pack_homography_row(left_screen_to_surface[1]),
            left_screen_to_surface_h2: pack_homography_row(left_screen_to_surface[2]),
            right_screen_to_surface_h0: pack_homography_row(right_screen_to_surface[0]),
            right_screen_to_surface_h1: pack_homography_row(right_screen_to_surface[1]),
            right_screen_to_surface_h2: pack_homography_row(right_screen_to_surface[2]),
            left_surface_to_screen_h0: pack_homography_row(left_surface_to_screen[0]),
            left_surface_to_screen_h1: pack_homography_row(left_surface_to_screen[1]),
            left_surface_to_screen_h2: pack_homography_row(left_surface_to_screen[2]),
            right_surface_to_screen_h0: pack_homography_row(right_surface_to_screen[0]),
            right_surface_to_screen_h1: pack_homography_row(right_surface_to_screen[1]),
            right_surface_to_screen_h2: pack_homography_row(right_surface_to_screen[2]),
            color_matrix_r0: [1.0, 0.0, 0.0, 0.0],
            color_matrix_r1: [0.0, 1.0, 0.0, 0.0],
            color_matrix_r2: [0.0, 0.0, 1.0, 0.0],
            color_offset: [0.0, 0.0, 0.0, 0.0],
            left_source_uv_rect: full_source_uv_rect_xywh(),
            right_source_uv_rect: full_source_uv_rect_xywh(),
            left_canvas_clip0: [-1.0, -1.0, 0.0, 1.0],
            left_canvas_clip1: [1.0, -1.0, 0.0, 1.0],
            left_canvas_clip2: [1.0, 1.0, 0.0, 1.0],
            left_canvas_clip3: [-1.0, 1.0, 0.0, 1.0],
            right_canvas_clip0: [-1.0, -1.0, 0.0, 1.0],
            right_canvas_clip1: [1.0, -1.0, 0.0, 1.0],
            right_canvas_clip2: [1.0, 1.0, 0.0, 1.0],
            right_canvas_clip3: [-1.0, 1.0, 0.0, 1.0],
        }
    }

    fn with_color_config(mut self, config: &crate::RuntimeConfig) -> Self {
        self.color_matrix_r0 = [
            config.camera_color_matrix[0][0],
            config.camera_color_matrix[0][1],
            config.camera_color_matrix[0][2],
            0.0,
        ];
        self.color_matrix_r1 = [
            config.camera_color_matrix[1][0],
            config.camera_color_matrix[1][1],
            config.camera_color_matrix[1][2],
            0.0,
        ];
        self.color_matrix_r2 = [
            config.camera_color_matrix[2][0],
            config.camera_color_matrix[2][1],
            config.camera_color_matrix[2][2],
            0.0,
        ];
        self.color_offset = [
            config.camera_color_offset[0],
            config.camera_color_offset[1],
            config.camera_color_offset[2],
            0.0,
        ];
        self
    }

    fn with_source_uv_rects(mut self, left: [f32; 4], right: [f32; 4]) -> Self {
        self.left_source_uv_rect = left;
        self.right_source_uv_rect = right;
        self
    }
}

impl CameraProjectionPush {
    fn from_frame(_frame: &HeadsetCameraGpuFrame, config: &crate::RuntimeConfig) -> Self {
        let mono_flags = config.camera_texture_transform.shader_flags() & 0x1f;
        let packed_flags = (mono_flags | (mono_flags << 5))
            | config.camera_color_mode.shader_bit()
            | config.camera_feed_pipeline_mode.shader_bit()
            | config.camera_projection_effect_mode.shader_bit()
            | config.camera_projection_border_policy_shader_bit();
        let content_uv_scale = full_view_content_uv_scale(
            config.camera_full_view_overlay_overscan,
            config.camera_raw_overlay_overscan,
        )
        .unwrap_or(1.0);
        Self {
            params: [
                config.camera_raw_overlay_overscan.max(1.0),
                config.camera_edge_fade.clamp(0.0, 0.5),
                content_uv_scale,
                packed_flags as f32,
            ],
            color_adjust: config.camera_color_adjust_push(),
            effect_params: config.camera_effect_params_push(),
            alpha_params: config.camera_alpha_params_push(),
            area_params: config.camera_area_params_push(),
            area_offset_params: config.camera_area_offset_params_push(),
            left_h0: [1.0, 0.0, 0.0, 0.0],
            left_h1: [0.0, 1.0, 0.0, 0.0],
            left_h2: [0.0, 0.0, 1.0, 0.0],
            right_h0: [1.0, 0.0, 0.0, 0.0],
            right_h1: [0.0, 1.0, 0.0, 0.0],
            right_h2: [0.0, 0.0, 1.0, 0.0],
        }
    }

    pub(super) fn from_stereo_frame(
        frame: &StereoGpuCameraFrame,
        config: &crate::RuntimeConfig,
        controls: &crate::StereoProjectionControls,
        views: &[xr::View],
        resolution: vk::Extent2D,
    ) -> (
        Self,
        CameraProjectionUniforms,
        Option<ProjectedStereoHomographies>,
    ) {
        let content_uv_scale = full_view_content_uv_scale(
            config.camera_full_view_overlay_overscan,
            config.camera_raw_overlay_overscan,
        )
        .unwrap_or(1.0);
        let push = Self {
            params: [
                config.camera_raw_overlay_overscan.max(1.0),
                config.camera_edge_fade.clamp(0.0, 0.5),
                content_uv_scale,
                (controls.packed_shader_flags()
                    | config.camera_color_mode.shader_bit()
                    | config.camera_feed_pipeline_mode.shader_bit()
                    | config.camera_projection_effect_mode.shader_bit()
                    | config.camera_projection_border_policy_shader_bit()) as f32,
            ],
            color_adjust: config.camera_color_adjust_push(),
            effect_params: config.camera_effect_params_push(),
            alpha_params: config.camera_alpha_params_push(),
            area_params: config.camera_area_params_push(),
            area_offset_params: config.camera_area_offset_params_push(),
            left_h0: [1.0, 0.0, 0.0, 0.0],
            left_h1: [0.0, 1.0, 0.0, 0.0],
            left_h2: [0.0, 0.0, 1.0, 0.0],
            right_h0: [1.0, 0.0, 0.0, 0.0],
            right_h1: [0.0, 1.0, 0.0, 0.0],
            right_h2: [0.0, 0.0, 1.0, 0.0],
        };
        if !controls.left_texture_transform.is_explicit_visual_check()
            || !controls.right_texture_transform.is_explicit_visual_check()
        {
            return (
                push,
                CameraProjectionUniforms::identity()
                    .with_source_uv_rects(
                        source_uv_rect_xywh_for_frame(&frame.left),
                        source_uv_rect_xywh_for_frame(&frame.right),
                    )
                    .with_color_config(config),
                None,
            );
        }

        if let Some((left, right)) =
            projected_stereo_homographies(frame, config, controls, views, resolution)
        {
            let homographies = ProjectedStereoHomographies { left, right };
            let (push, uniforms) =
                Self::from_projected_stereo_homographies(frame, config, controls, &homographies);
            return (push, uniforms, Some(homographies));
        }
        (
            push,
            CameraProjectionUniforms::identity()
                .with_source_uv_rects(
                    source_uv_rect_xywh_for_frame(&frame.left),
                    source_uv_rect_xywh_for_frame(&frame.right),
                )
                .with_color_config(config),
            None,
        )
    }

    fn from_projected_stereo_homographies(
        frame: &StereoGpuCameraFrame,
        config: &crate::RuntimeConfig,
        controls: &crate::StereoProjectionControls,
        homographies: &ProjectedStereoHomographies,
    ) -> (Self, CameraProjectionUniforms) {
        let content_uv_scale = full_view_content_uv_scale(
            config.camera_full_view_overlay_overscan,
            config.camera_raw_overlay_overscan,
        )
        .unwrap_or(1.0);
        let full_frame_mapping_flags = if homographies.left.full_frame_stimulus_mapping
            && homographies.right.full_frame_stimulus_mapping
        {
            crate::camera_color_pipeline::CAMERA_SHADER_FLAG_FULL_FRAME_STIMULUS_MAPPING
        } else {
            0
        };
        let mut push = Self {
            params: [
                -config.camera_raw_overlay_overscan.max(1.0),
                config.camera_edge_fade.clamp(0.0, 0.5),
                content_uv_scale,
                (controls.packed_shader_flags()
                    | config.camera_color_mode.shader_bit()
                    | config.camera_feed_pipeline_mode.shader_bit()
                    | config.camera_projection_effect_mode.shader_bit()
                    | config.camera_projection_border_policy_shader_bit()
                    | full_frame_mapping_flags) as f32,
            ],
            color_adjust: config.camera_color_adjust_push(),
            effect_params: config.camera_effect_params_push(),
            alpha_params: config.camera_alpha_params_push(),
            area_params: config.camera_area_params_push(),
            area_offset_params: config.camera_area_offset_params_push(),
            left_h0: [1.0, 0.0, 0.0, 0.0],
            left_h1: [0.0, 1.0, 0.0, 0.0],
            left_h2: [0.0, 0.0, 1.0, 0.0],
            right_h0: [1.0, 0.0, 0.0, 0.0],
            right_h1: [0.0, 1.0, 0.0, 0.0],
            right_h2: [0.0, 0.0, 1.0, 0.0],
        };
        let left_sample_rows = if config.camera_projection_mode.uses_world_canvas() {
            homographies.left.surface_to_camera
        } else {
            homographies.left.screen_to_camera
        };
        let right_sample_rows = if config.camera_projection_mode.uses_world_canvas() {
            homographies.right.surface_to_camera
        } else {
            homographies.right.screen_to_camera
        };
        push.left_h0 = pack_homography_row(left_sample_rows[0]);
        push.left_h1 = pack_homography_row(left_sample_rows[1]);
        push.left_h2 = pack_homography_row(left_sample_rows[2]);
        push.right_h0 = pack_homography_row(right_sample_rows[0]);
        push.right_h1 = pack_homography_row(right_sample_rows[1]);
        push.right_h2 = pack_homography_row(right_sample_rows[2]);
        (
            push,
            CameraProjectionUniforms::from_mappings(&homographies.left, &homographies.right)
                .with_source_uv_rects(
                    source_uv_rect_xywh_for_frame(&frame.left),
                    source_uv_rect_xywh_for_frame(&frame.right),
                )
                .with_color_config(config),
        )
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct CameraRenderCadenceFrame {
    pub(super) render_frame_count: u64,
    pub(super) distinct_frame_count: u64,
    pub(super) repeated_render_frame_count: u64,
    pub(super) renders_per_camera_frame_avg: f64,
    pub(super) max_consecutive_render_frames_per_camera_frame: u64,
    pub(super) consumed_frame_hz: f64,
    pub(super) projection_render_hz: f64,
}

#[derive(Default)]
pub(super) struct CameraRenderCadenceStats {
    started: Option<Instant>,
    render_frame_count: u64,
    distinct_frame_count: u64,
    repeated_render_frame_count: u64,
    last_camera_frame_index: Option<u64>,
    current_consecutive_render_frames: u64,
    max_consecutive_render_frames_per_camera_frame: u64,
}

impl CameraRenderCadenceStats {
    pub(super) fn record(&mut self, camera_frame_index: u64) -> CameraRenderCadenceFrame {
        let started = *self.started.get_or_insert_with(Instant::now);
        self.render_frame_count = self.render_frame_count.saturating_add(1);

        if self.last_camera_frame_index == Some(camera_frame_index) {
            self.repeated_render_frame_count = self.repeated_render_frame_count.saturating_add(1);
            self.current_consecutive_render_frames =
                self.current_consecutive_render_frames.saturating_add(1);
        } else {
            self.distinct_frame_count = self.distinct_frame_count.saturating_add(1);
            self.last_camera_frame_index = Some(camera_frame_index);
            self.current_consecutive_render_frames = 1;
        }

        self.max_consecutive_render_frames_per_camera_frame = self
            .max_consecutive_render_frames_per_camera_frame
            .max(self.current_consecutive_render_frames);

        let elapsed_seconds = started.elapsed().as_secs_f64();
        let hz_divisor = if elapsed_seconds > 0.001 {
            elapsed_seconds
        } else {
            f64::INFINITY
        };
        let renders_per_camera_frame_avg = if self.distinct_frame_count > 0 {
            self.render_frame_count as f64 / self.distinct_frame_count as f64
        } else {
            0.0
        };

        CameraRenderCadenceFrame {
            render_frame_count: self.render_frame_count,
            distinct_frame_count: self.distinct_frame_count,
            repeated_render_frame_count: self.repeated_render_frame_count,
            renders_per_camera_frame_avg,
            max_consecutive_render_frames_per_camera_frame: self
                .max_consecutive_render_frames_per_camera_frame,
            consumed_frame_hz: self.distinct_frame_count as f64 / hz_divisor,
            projection_render_hz: self.render_frame_count as f64 / hz_divisor,
        }
    }
}

fn full_source_uv_rect_xywh() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

fn source_uv_rect_xywh_for_frame(frame: &HeadsetCameraGpuFrame) -> [f32; 4] {
    let [left, top, right, bottom] = source_uv_rect_ltrb_for_diagnostics(&frame.diagnostics);
    [left, top, (right - left).max(0.0), (bottom - top).max(0.0)]
}
