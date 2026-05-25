use ash::vk;

use crate::HeadsetCameraGpuFrame;

use super::{
    find_memory_type,
    gpu_camera_pipeline::allocate_camera_descriptor_set,
    gpu_camera_resources::{
        GpuCameraFormatKey, GpuCameraImport, GpuCameraImportKey, GpuCameraPipelineResources,
    },
};
#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn import_camera_hardware_buffer(
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

pub(super) unsafe fn transition_imported_camera_image(
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
