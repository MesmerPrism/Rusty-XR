use ash::vk;

use super::find_memory_type;

pub(super) struct CameraUpload {
    pub(super) buffer: vk::Buffer,
    pub(super) memory: vk::DeviceMemory,
    pub(super) capacity: vk::DeviceSize,
}

impl CameraUpload {
    pub(super) unsafe fn destroy(self, device: &ash::Device) {
        device.destroy_buffer(self.buffer, None);
        device.free_memory(self.memory, None);
    }
}

pub(super) unsafe fn ensure_camera_upload<'a>(
    device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    upload: &'a mut Option<CameraUpload>,
    byte_len: vk::DeviceSize,
) -> Result<&'a mut CameraUpload, String> {
    let needs_new = upload
        .as_ref()
        .map(|upload| upload.capacity < byte_len)
        .unwrap_or(true);

    if needs_new {
        if let Some(old) = upload.take() {
            old.destroy(device);
        }

        let buffer = device
            .create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(byte_len)
                    .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE),
                None,
            )
            .map_err(|error| format!("create headset camera upload buffer: {error}"))?;
        let requirements = device.get_buffer_memory_requirements(buffer);
        let memory_type_index = find_memory_type(
            memory_properties,
            requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let memory = device
            .allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(requirements.size)
                    .memory_type_index(memory_type_index),
                None,
            )
            .map_err(|error| format!("allocate headset camera upload memory: {error}"))?;
        device
            .bind_buffer_memory(buffer, memory, 0)
            .map_err(|error| format!("bind headset camera upload memory: {error}"))?;

        *upload = Some(CameraUpload {
            buffer,
            memory,
            capacity: byte_len,
        });
    }

    upload
        .as_mut()
        .ok_or_else(|| "headset camera upload buffer was not initialized".to_string())
}

#[derive(Clone, Copy)]
pub(super) struct CameraCopy {
    pub(super) buffer: vk::Buffer,
    pub(super) width: u32,
    pub(super) height: u32,
}
