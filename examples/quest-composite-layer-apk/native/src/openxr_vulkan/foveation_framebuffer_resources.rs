use ash::vk;

use super::{
    find_memory_type, VIEW_COUNT, XR_FOVEATION_DEPTH_FORMAT, XR_FRAGMENT_DENSITY_MAP_FORMAT,
};

pub(super) struct DepthAttachment {
    pub(super) image: vk::Image,
    pub(super) view: vk::ImageView,
    pub(super) memory: vk::DeviceMemory,
}

pub(super) unsafe fn create_fragment_density_image_view(
    device: &ash::Device,
    image: vk::Image,
) -> Result<vk::ImageView, String> {
    device
        .create_image_view(
            &vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(XR_FRAGMENT_DENSITY_MAP_FORMAT)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }),
            None,
        )
        .map_err(|error| format!("create fragment density image view: {error}"))
}

pub(super) unsafe fn create_foveation_depth_attachment(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    resolution: vk::Extent2D,
) -> Result<DepthAttachment, String> {
    let image = device
        .create_image(
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(XR_FOVEATION_DEPTH_FORMAT)
                .extent(vk::Extent3D {
                    width: resolution.width.max(1),
                    height: resolution.height.max(1),
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(VIEW_COUNT)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            None,
        )
        .map_err(|error| format!("create foveation depth image: {error}"))?;
    let requirements = device.get_image_memory_requirements(image);
    let memory_type_index = match find_memory_type(
        memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    ) {
        Ok(index) => index,
        Err(error) => {
            device.destroy_image(image, None);
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
            device.destroy_image(image, None);
            return Err(format!("allocate foveation depth memory: {error}"));
        }
    };
    if let Err(error) = device.bind_image_memory(image, memory, 0) {
        device.free_memory(memory, None);
        device.destroy_image(image, None);
        return Err(format!("bind foveation depth memory: {error}"));
    }
    let view = match device.create_image_view(
        &vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(XR_FOVEATION_DEPTH_FORMAT)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: VIEW_COUNT,
            }),
        None,
    ) {
        Ok(view) => view,
        Err(error) => {
            device.free_memory(memory, None);
            device.destroy_image(image, None);
            return Err(format!("create foveation depth image view: {error}"));
        }
    };
    Ok(DepthAttachment {
        image,
        view,
        memory,
    })
}
