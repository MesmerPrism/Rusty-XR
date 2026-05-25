use std::{ffi::CString, time::Instant};

use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::{full_view_content_uv_scale, CameraCompositeTier};

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::gpu_camera_resources::{
    GpuCameraFormatKey, GpuCameraImport, GpuCameraImportKey, GpuCameraPipelineResources,
    GpuCameraStereoDescriptor,
};
use super::{
    find_memory_type, log_error, log_info,
    projection_geometry::{
        identity_homography, pack_homography_row, projected_stereo_homographies,
        DisplayEyeProjectionMapping, ProjectedStereoHomographies,
    },
    source_metadata::source_uv_rect_ltrb_for_diagnostics,
    spirv_words,
};

const GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = crate::CAMERA_IMPORT_CACHE_LIMIT_MAX;
const GPU_CAMERA_PROJECTION_UNIFORM_SLOTS: u32 = 3;

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

fn effective_camera_import_cache_limit(limit: usize) -> usize {
    limit.clamp(2, GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX)
}

unsafe fn create_gpu_camera_pipeline_resources(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    projection_uniform_alignment: vk::DeviceSize,
    render_pass: vk::RenderPass,
    format_key: GpuCameraFormatKey,
    format_props: &vk::AndroidHardwareBufferFormatPropertiesANDROID<'_>,
) -> Result<GpuCameraPipelineResources, String> {
    let mut external_format =
        vk::ExternalFormatANDROID::default().external_format(format_key.external_format);
    let mut conversion_info = vk::SamplerYcbcrConversionCreateInfo::default()
        .format(format_key.format)
        .ycbcr_model(format_props.suggested_ycbcr_model)
        .ycbcr_range(format_props.suggested_ycbcr_range)
        .components(format_props.sampler_ycbcr_conversion_components)
        .x_chroma_offset(format_props.suggested_x_chroma_offset)
        .y_chroma_offset(format_props.suggested_y_chroma_offset)
        .chroma_filter(vk::Filter::LINEAR);
    if format_key.external_format != 0 {
        conversion_info = conversion_info.push_next(&mut external_format);
    }
    let sampler_ycbcr_conversion = device
        .create_sampler_ycbcr_conversion(&conversion_info, None)
        .map_err(|error| format!("create camera sampler YCbCr conversion: {error}"))?;

    let mut sampler_conversion_info =
        vk::SamplerYcbcrConversionInfo::default().conversion(sampler_ycbcr_conversion);
    let sampler = device
        .create_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::LINEAR)
                .min_filter(vk::Filter::LINEAR)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .push_next(&mut sampler_conversion_info),
            None,
        )
        .map_err(|error| format!("create camera sampler: {error}"))?;

    let immutable_samplers = [sampler];
    let descriptor_binding = match format_key.sampler_binding_mode {
        crate::CameraSamplerBindingMode::CombinedImmutableSampler => vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .immutable_samplers(&immutable_samplers),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .immutable_samplers(&immutable_samplers),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
        ],
        crate::CameraSamplerBindingMode::SeparateImageSampler => vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            vk::DescriptorSetLayoutBinding::default()
                .binding(3)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ],
    };
    let descriptor_set_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_binding),
            None,
        )
        .map_err(|error| format!("create camera descriptor set layout: {error}"))?;
    let max_descriptor_sets = (GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX as u32) * 2;
    let pool_sizes = match format_key.sampler_binding_mode {
        crate::CameraSamplerBindingMode::CombinedImmutableSampler => vec![
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count((GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX as u32) * 4),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(max_descriptor_sets),
        ],
        crate::CameraSamplerBindingMode::SeparateImageSampler => vec![
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count((GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX as u32) * 4),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(max_descriptor_sets),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(max_descriptor_sets),
        ],
    };
    let descriptor_pool = device
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .pool_sizes(&pool_sizes)
                .max_sets(max_descriptor_sets),
            None,
        )
        .map_err(|error| format!("create camera descriptor pool: {error}"))?;
    let (projection_uniform_buffer, projection_uniform_memory, projection_uniform_stride) =
        create_camera_projection_uniform_buffer(
            device,
            memory_properties,
            projection_uniform_alignment,
        )?;

    let push_ranges = [vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(std::mem::size_of::<CameraProjectionPush>() as u32)];
    let set_layouts = [descriptor_set_layout];
    let pipeline_layout = device
        .create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&set_layouts)
                .push_constant_ranges(&push_ranges),
            None,
        )
        .map_err(|error| format!("create camera pipeline layout: {error}"))?;
    let pipeline = create_gpu_camera_pipeline(
        device,
        render_pass,
        pipeline_layout,
        format_key.sampler_binding_mode,
        false,
    )?;
    let direct_pipeline = match create_gpu_camera_pipeline(
        device,
        render_pass,
        pipeline_layout,
        format_key.sampler_binding_mode,
        true,
    ) {
        Ok(pipeline) => pipeline,
        Err(error) => {
            device.destroy_pipeline(pipeline, None);
            return Err(error);
        }
    };

    Ok(GpuCameraPipelineResources {
        format_key,
        sampler_ycbcr_conversion,
        sampler,
        descriptor_set_layout,
        descriptor_pool,
        pipeline_layout,
        pipeline,
        direct_pipeline,
        projection_uniform_buffer,
        projection_uniform_memory,
        projection_uniform_stride,
        projection_uniform_slots: GPU_CAMERA_PROJECTION_UNIFORM_SLOTS,
    })
}

unsafe fn allocate_camera_descriptor_set(
    device: &ash::Device,
    resources: &GpuCameraPipelineResources,
    left_image_view: vk::ImageView,
    right_image_view: vk::ImageView,
) -> Result<vk::DescriptorSet, String> {
    let descriptor_set_layouts = [resources.descriptor_set_layout];
    let descriptor_set = match device.allocate_descriptor_sets(
        &vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(resources.descriptor_pool)
            .set_layouts(&descriptor_set_layouts),
    ) {
        Ok(mut sets) => sets
            .pop()
            .ok_or_else(|| "camera descriptor allocation returned no set".to_string())?,
        Err(error) => {
            return Err(format!("allocate camera descriptor set: {error}"));
        }
    };
    let image_layout =
        camera_import_descriptor_layout(resources.format_key.import_image_layout_mode);
    let left_info = [vk::DescriptorImageInfo::default()
        .sampler(resources.sampler)
        .image_view(left_image_view)
        .image_layout(image_layout)];
    let right_info = [vk::DescriptorImageInfo::default()
        .sampler(resources.sampler)
        .image_view(right_image_view)
        .image_layout(image_layout)];
    let projection_info = [vk::DescriptorBufferInfo::default()
        .buffer(resources.projection_uniform_buffer)
        .offset(0)
        .range(std::mem::size_of::<CameraProjectionUniforms>() as vk::DeviceSize)];
    match resources.format_key.sampler_binding_mode {
        crate::CameraSamplerBindingMode::CombinedImmutableSampler => {
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&left_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&right_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                    .buffer_info(&projection_info),
            ];
            device.update_descriptor_sets(&writes, &[]);
        }
        crate::CameraSamplerBindingMode::SeparateImageSampler => {
            let left_sampled_image = [vk::DescriptorImageInfo::default()
                .image_view(left_image_view)
                .image_layout(image_layout)];
            let right_sampled_image = [vk::DescriptorImageInfo::default()
                .image_view(right_image_view)
                .image_layout(image_layout)];
            let sampler_info = [vk::DescriptorImageInfo::default().sampler(resources.sampler)];
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&left_sampled_image),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&right_sampled_image),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                    .buffer_info(&projection_info),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(3)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .image_info(&sampler_info),
            ];
            device.update_descriptor_sets(&writes, &[]);
        }
    }
    Ok(descriptor_set)
}

unsafe fn create_camera_projection_uniform_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    min_uniform_alignment: vk::DeviceSize,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::DeviceSize), String> {
    let uniform_size = std::mem::size_of::<CameraProjectionUniforms>() as vk::DeviceSize;
    let stride = align_uniform_stride(uniform_size, min_uniform_alignment.max(16));
    let total_size = stride * GPU_CAMERA_PROJECTION_UNIFORM_SLOTS.max(1) as vk::DeviceSize;
    let buffer = device
        .create_buffer(
            &vk::BufferCreateInfo::default()
                .size(total_size)
                .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE),
            None,
        )
        .map_err(|error| format!("create camera projection uniform buffer: {error}"))?;
    let requirements = device.get_buffer_memory_requirements(buffer);
    let memory_type_index = match find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    ) {
        Ok(index) => index,
        Err(error) => {
            device.destroy_buffer(buffer, None);
            return Err(error);
        }
    };
    let memory = match device.allocate_memory(
        &vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index),
        None,
    ) {
        Ok(memory) => memory,
        Err(error) => {
            device.destroy_buffer(buffer, None);
            return Err(format!(
                "allocate camera projection uniform memory: {error}"
            ));
        }
    };
    if let Err(error) = device.bind_buffer_memory(buffer, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_buffer(buffer, None);
        return Err(format!("bind camera projection uniform memory: {error}"));
    }
    Ok((buffer, memory, stride))
}

fn align_uniform_stride(value: vk::DeviceSize, alignment: vk::DeviceSize) -> vk::DeviceSize {
    if alignment <= 1 {
        value
    } else {
        value.div_ceil(alignment) * alignment
    }
}

unsafe fn update_camera_projection_uniforms(
    device: &ash::Device,
    resources: &GpuCameraPipelineResources,
    offset: u32,
    uniforms: &CameraProjectionUniforms,
) -> Result<(), String> {
    let byte_len = std::mem::size_of::<CameraProjectionUniforms>() as vk::DeviceSize;
    let mapped = device
        .map_memory(
            resources.projection_uniform_memory,
            offset as vk::DeviceSize,
            byte_len,
            vk::MemoryMapFlags::empty(),
        )
        .map_err(|error| format!("map camera projection uniform memory: {error}"))?;
    std::ptr::copy_nonoverlapping(
        (uniforms as *const CameraProjectionUniforms).cast::<u8>(),
        mapped.cast::<u8>(),
        byte_len as usize,
    );
    device.unmap_memory(resources.projection_uniform_memory);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
unsafe fn import_camera_hardware_buffer(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    resources: &GpuCameraPipelineResources,
    frame: &HeadsetCameraGpuFrame,
    key: GpuCameraImportKey,
    format_key: GpuCameraFormatKey,
    allocation_size: vk::DeviceSize,
    memory_type_bits: u32,
) -> Result<GpuCameraImport, String> {
    let mut external_memory = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::ANDROID_HARDWARE_BUFFER_ANDROID);
    let mut external_format =
        vk::ExternalFormatANDROID::default().external_format(format_key.external_format);
    let mut image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format_key.format)
        .extent(vk::Extent3D {
            width: frame.width,
            height: frame.height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .push_next(&mut external_memory);
    if format_key.external_format != 0 {
        image_info = image_info.push_next(&mut external_format);
    }
    let image = device
        .create_image(&image_info, None)
        .map_err(|error| format!("create imported camera image: {error}"))?;

    let memory_type_index = match find_memory_type_relaxed(memory_properties, memory_type_bits) {
        Ok(index) => index,
        Err(error) => {
            device.destroy_image(image, None);
            return Err(error);
        }
    };
    let mut import_info = vk::ImportAndroidHardwareBufferInfoANDROID::default()
        .buffer(frame.hardware_buffer.as_ptr().cast());
    let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
    let memory = match device.allocate_memory(
        &vk::MemoryAllocateInfo::default()
            .allocation_size(allocation_size)
            .memory_type_index(memory_type_index)
            .push_next(&mut import_info)
            .push_next(&mut dedicated),
        None,
    ) {
        Ok(memory) => memory,
        Err(error) => {
            device.destroy_image(image, None);
            return Err(format!("allocate imported camera memory: {error}"));
        }
    };
    if let Err(error) = device.bind_image_memory(image, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_image(image, None);
        return Err(format!("bind imported camera memory: {error}"));
    }

    let mut view_conversion =
        vk::SamplerYcbcrConversionInfo::default().conversion(resources.sampler_ycbcr_conversion);
    let image_view = match device.create_image_view(
        &vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format_key.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .push_next(&mut view_conversion),
        None,
    ) {
        Ok(image_view) => image_view,
        Err(error) => {
            device.free_memory(memory, None);
            device.destroy_image(image, None);
            return Err(format!("create imported camera image view: {error}"));
        }
    };

    let descriptor_set =
        match allocate_camera_descriptor_set(device, resources, image_view, image_view) {
            Ok(descriptor_set) => descriptor_set,
            Err(error) => {
                device.destroy_image_view(image_view, None);
                device.free_memory(memory, None);
                device.destroy_image(image, None);
                return Err(error);
            }
        };

    Ok(GpuCameraImport {
        key,
        image,
        memory,
        image_view,
        descriptor_set,
        descriptor_pool: resources.descriptor_pool,
        needs_layout_transition: format_key.import_image_layout_mode.needs_transition(),
        _hardware_buffer: frame.hardware_buffer.clone(),
    })
}

unsafe fn create_gpu_camera_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    sampler_binding_mode: crate::CameraSamplerBindingMode,
    direct_raw_projection: bool,
) -> Result<vk::Pipeline, String> {
    let vertex_words = spirv_words(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/camera_projection.vert.spv"
    )))?;
    let fragment_words = match (sampler_binding_mode, direct_raw_projection) {
        (crate::CameraSamplerBindingMode::CombinedImmutableSampler, false) => spirv_words(
            include_bytes!(concat!(env!("OUT_DIR"), "/camera_projection.frag.spv")),
        )?,
        (crate::CameraSamplerBindingMode::CombinedImmutableSampler, true) => {
            spirv_words(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/camera_projection_direct.frag.spv"
            )))?
        }
        (crate::CameraSamplerBindingMode::SeparateImageSampler, false) => {
            spirv_words(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/camera_projection_separate_sampler.frag.spv"
            )))?
        }
        (crate::CameraSamplerBindingMode::SeparateImageSampler, true) => {
            spirv_words(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/camera_projection_direct_separate_sampler.frag.spv"
            )))?
        }
    };
    let vertex_module = device
        .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(&vertex_words),
            None,
        )
        .map_err(|error| format!("create camera vertex shader module: {error}"))?;
    let fragment_module = match device.create_shader_module(
        &vk::ShaderModuleCreateInfo::default().code(&fragment_words),
        None,
    ) {
        Ok(module) => module,
        Err(error) => {
            device.destroy_shader_module(vertex_module, None);
            return Err(format!("create camera fragment shader module: {error}"));
        }
    };
    let entry = CString::new("main").expect("static shader entry point is valid");
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex_module)
            .name(&entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment_module)
            .name(&entry),
    ];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .line_width(1.0);
    let multisample = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let color_blend_attachment = [vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA)];
    let color_blend =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(&color_blend_attachment);
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::ALWAYS)
        .stencil_test_enable(false);
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let create_info = [vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisample)
        .color_blend_state(&color_blend)
        .depth_stencil_state(&depth_stencil)
        .dynamic_state(&dynamic)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)];
    let pipeline_result =
        device.create_graphics_pipelines(vk::PipelineCache::null(), &create_info, None);
    device.destroy_shader_module(fragment_module, None);
    device.destroy_shader_module(vertex_module, None);
    pipeline_result
        .map(|mut pipelines| pipelines.remove(0))
        .map_err(|(_, error)| format!("create camera graphics pipeline: {error}"))
}

fn camera_import_descriptor_layout(mode: crate::CameraImportImageLayoutMode) -> vk::ImageLayout {
    match mode {
        crate::CameraImportImageLayoutMode::ShaderReadOnlyTransition => {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        }
        crate::CameraImportImageLayoutMode::GeneralNoTransition => vk::ImageLayout::GENERAL,
    }
}

unsafe fn transition_imported_camera_image(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    image: vk::Image,
) {
    let barrier = [vk::ImageMemoryBarrier::default()
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        })
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::SHADER_READ)];
    device.cmd_pipeline_barrier(
        cmd,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &barrier,
    );
}

fn find_memory_type_relaxed(
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    memory_type_bits: u32,
) -> Result<u32, String> {
    find_memory_type(
        memory_properties,
        memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
    .or_else(|_| {
        for index in 0..memory_properties.memory_type_count {
            if (memory_type_bits & (1 << index)) != 0 {
                return Ok(index);
            }
        }
        Err(format!(
            "no Vulkan memory type supports imported Android hardware buffer bits 0x{memory_type_bits:x}"
        ))
    })
}
