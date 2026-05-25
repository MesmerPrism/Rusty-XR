use ash::vk;

use crate::HeadsetCameraGpuFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuCameraFormatKey {
    pub(super) format: vk::Format,
    pub(super) external_format: u64,
    pub(super) sampler_binding_mode: crate::CameraSamplerBindingMode,
    pub(super) import_image_layout_mode: crate::CameraImportImageLayoutMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuCameraImportKey {
    pub(super) buffer_id: u64,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) native_format: u64,
}

impl GpuCameraImportKey {
    pub(super) fn from_frame(frame: &HeadsetCameraGpuFrame) -> Self {
        Self {
            buffer_id: frame
                .descriptor
                .buffer_id
                .unwrap_or(frame.timestamp_ns as u64),
            width: frame.width,
            height: frame.height,
            native_format: frame.descriptor.native_format.unwrap_or_default(),
        }
    }
}

pub(super) struct GpuCameraPipelineResources {
    pub(super) format_key: GpuCameraFormatKey,
    pub(super) sampler_ycbcr_conversion: vk::SamplerYcbcrConversion,
    pub(super) sampler: vk::Sampler,
    pub(super) descriptor_set_layout: vk::DescriptorSetLayout,
    pub(super) descriptor_pool: vk::DescriptorPool,
    pub(super) pipeline_layout: vk::PipelineLayout,
    pub(super) pipeline: vk::Pipeline,
    pub(super) direct_pipeline: vk::Pipeline,
    pub(super) projection_uniform_buffer: vk::Buffer,
    pub(super) projection_uniform_memory: vk::DeviceMemory,
    pub(super) projection_uniform_stride: vk::DeviceSize,
    pub(super) projection_uniform_slots: u32,
}

impl GpuCameraPipelineResources {
    pub(super) unsafe fn destroy(self, device: &ash::Device) {
        device.destroy_pipeline(self.direct_pipeline, None);
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_descriptor_pool(self.descriptor_pool, None);
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        device.destroy_buffer(self.projection_uniform_buffer, None);
        device.free_memory(self.projection_uniform_memory, None);
        device.destroy_sampler(self.sampler, None);
        device.destroy_sampler_ycbcr_conversion(self.sampler_ycbcr_conversion, None);
    }

    pub(super) fn projection_uniform_offset(&self, frame_count: u64) -> u32 {
        let slot = frame_count % self.projection_uniform_slots.max(1) as u64;
        (slot * self.projection_uniform_stride) as u32
    }

    pub(super) fn pipeline_for_config(&self, config: &crate::RuntimeConfig) -> vk::Pipeline {
        if config
            .camera_processing_layer
            .requires_full_projection_pipeline()
            || config.camera_projection_border_policy_requires_full_pipeline()
        {
            self.pipeline
        } else if config
            .camera_projection_effect_mode
            .uses_raw_projection_pipeline()
        {
            self.direct_pipeline
        } else {
            self.pipeline
        }
    }
}

pub(super) struct GpuCameraImport {
    pub(super) key: GpuCameraImportKey,
    pub(super) image: vk::Image,
    pub(super) memory: vk::DeviceMemory,
    pub(super) image_view: vk::ImageView,
    pub(super) descriptor_set: vk::DescriptorSet,
    pub(super) descriptor_pool: vk::DescriptorPool,
    pub(super) needs_layout_transition: bool,
    pub(super) _hardware_buffer: crate::AndroidHardwareBufferHandle,
}

pub(super) struct GpuCameraStereoDescriptor {
    pub(super) left_key: GpuCameraImportKey,
    pub(super) right_key: GpuCameraImportKey,
    pub(super) descriptor_set: vk::DescriptorSet,
    pub(super) descriptor_pool: vk::DescriptorPool,
}

impl GpuCameraImport {
    pub(super) unsafe fn destroy(self, device: &ash::Device) {
        let _ = device.free_descriptor_sets(self.descriptor_pool, &[self.descriptor_set]);
        device.destroy_image_view(self.image_view, None);
        device.destroy_image(self.image, None);
        device.free_memory(self.memory, None);
    }
}

impl GpuCameraStereoDescriptor {
    pub(super) unsafe fn destroy(self, device: &ash::Device) {
        let _ = device.free_descriptor_sets(self.descriptor_pool, &[self.descriptor_set]);
    }
}
