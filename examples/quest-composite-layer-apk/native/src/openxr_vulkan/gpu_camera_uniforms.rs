use ash::vk;

use super::{
    find_memory_type, gpu_camera_projection_uniforms::CameraProjectionUniforms,
    gpu_camera_resources::GpuCameraPipelineResources,
};

pub(super) const GPU_CAMERA_PROJECTION_UNIFORM_SLOTS: u32 = 3;

pub(super) unsafe fn create_camera_projection_uniform_buffer(
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

pub(super) unsafe fn update_camera_projection_uniforms(
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

fn align_uniform_stride(value: vk::DeviceSize, alignment: vk::DeviceSize) -> vk::DeviceSize {
    if alignment <= 1 {
        value
    } else {
        value.div_ceil(alignment) * alignment
    }
}
