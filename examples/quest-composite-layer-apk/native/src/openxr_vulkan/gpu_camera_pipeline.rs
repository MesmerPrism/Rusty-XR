use std::ffi::CString;

use ash::vk;

use super::{
    gpu_camera_projection::{CameraProjectionPush, CameraProjectionUniforms},
    gpu_camera_resources::{GpuCameraFormatKey, GpuCameraPipelineResources},
    gpu_camera_uniforms::{
        create_camera_projection_uniform_buffer, GPU_CAMERA_PROJECTION_UNIFORM_SLOTS,
    },
    spirv_words,
};

const GPU_CAMERA_IMPORT_CACHE_LIMIT_MAX: usize = crate::CAMERA_IMPORT_CACHE_LIMIT_MAX;
pub(super) unsafe fn create_gpu_camera_pipeline_resources(
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
