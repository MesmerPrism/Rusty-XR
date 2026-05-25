use ash::vk;

use super::{VIEW_COUNT, XR_FOVEATION_DEPTH_FORMAT, XR_FRAGMENT_DENSITY_MAP_FORMAT};

pub(super) unsafe fn create_openxr_render_pass(
    device: &ash::Device,
    use_fragment_density_map: bool,
    color_format: vk::Format,
) -> Result<vk::RenderPass, String> {
    let color_attachment = vk::AttachmentDescription {
        format: color_format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        ..Default::default()
    };
    let depth_attachment = vk::AttachmentDescription {
        format: XR_FOVEATION_DEPTH_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::DONT_CARE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        ..Default::default()
    };
    let fragment_density_attachment = vk::AttachmentDescription {
        format: XR_FRAGMENT_DENSITY_MAP_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::DONT_CARE,
        store_op: vk::AttachmentStoreOp::DONT_CARE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::FRAGMENT_DENSITY_MAP_OPTIMAL_EXT,
        final_layout: vk::ImageLayout::FRAGMENT_DENSITY_MAP_OPTIMAL_EXT,
        ..Default::default()
    };
    let attachments = if use_fragment_density_map {
        vec![
            color_attachment,
            depth_attachment,
            fragment_density_attachment,
        ]
    } else {
        vec![color_attachment]
    };
    let color_refs = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
    let fragment_density_ref = vk::AttachmentReference {
        attachment: 2,
        layout: vk::ImageLayout::FRAGMENT_DENSITY_MAP_OPTIMAL_EXT,
    };
    let depth_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };
    let mut subpass = vk::SubpassDescription::default()
        .color_attachments(&color_refs)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);
    if use_fragment_density_map {
        subpass = subpass.depth_stencil_attachment(&depth_ref);
    }
    let subpasses = [subpass];
    let depth_stage = if use_fragment_density_map {
        vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
    } else {
        vk::PipelineStageFlags::empty()
    };
    let depth_access = if use_fragment_density_map {
        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
    } else {
        vk::AccessFlags::empty()
    };
    let fdm_stage = if use_fragment_density_map {
        vk::PipelineStageFlags::FRAGMENT_DENSITY_PROCESS_EXT
    } else {
        vk::PipelineStageFlags::empty()
    };
    let fdm_access = if use_fragment_density_map {
        vk::AccessFlags::FRAGMENT_DENSITY_MAP_READ_EXT
    } else {
        vk::AccessFlags::empty()
    };
    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        dst_subpass: 0,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | depth_stage | fdm_stage,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | depth_stage | fdm_stage,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE | depth_access | fdm_access,
        ..Default::default()
    }];
    let view_mask = !(!0 << VIEW_COUNT);
    let view_masks = [view_mask];
    let correlation_masks = [view_mask];
    let mut multiview = vk::RenderPassMultiviewCreateInfo::default()
        .view_masks(&view_masks)
        .correlation_masks(&correlation_masks);
    let mut fragment_density_info = vk::RenderPassFragmentDensityMapCreateInfoEXT::default()
        .fragment_density_map_attachment(fragment_density_ref);
    let mut render_pass_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);
    if use_fragment_density_map {
        render_pass_info = render_pass_info.push_next(&mut fragment_density_info);
    }
    render_pass_info = render_pass_info.push_next(&mut multiview);
    device
        .create_render_pass(&render_pass_info, None)
        .map_err(|error| format!("create render pass: {error}"))
}
