use ash::vk;

use super::{
    gpu_camera_projection::CameraProjectionUniforms,
    gpu_camera_resources::GpuCameraPipelineResources,
};

const GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = crate::CAMERA_IMPORT_CACHE_LIMIT_MAX;

pub(super) unsafe fn create_camera_descriptor_set_layout(
    device: &ash::Device,
    sampler_binding_mode: crate::CameraSamplerBindingMode,
    sampler: vk::Sampler,
) -> Result<vk::DescriptorSetLayout, String> {
    let immutable_samplers = [sampler];
    let descriptor_binding = match sampler_binding_mode {
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
    device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_binding),
            None,
        )
        .map_err(|error| format!("create camera descriptor set layout: {error}"))
}

pub(super) unsafe fn create_camera_descriptor_pool(
    device: &ash::Device,
    sampler_binding_mode: crate::CameraSamplerBindingMode,
) -> Result<vk::DescriptorPool, String> {
    let max_descriptor_sets = (GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX as u32) * 2;
    let pool_sizes = match sampler_binding_mode {
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
    device
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .pool_sizes(&pool_sizes)
                .max_sets(max_descriptor_sets),
            None,
        )
        .map_err(|error| format!("create camera descriptor pool: {error}"))
}

pub(super) unsafe fn allocate_camera_descriptor_set(
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

fn camera_import_descriptor_layout(mode: crate::CameraImportImageLayoutMode) -> vk::ImageLayout {
    match mode {
        crate::CameraImportImageLayoutMode::ShaderReadOnlyTransition => {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        }
        crate::CameraImportImageLayoutMode::GeneralNoTransition => vk::ImageLayout::GENERAL,
    }
}
