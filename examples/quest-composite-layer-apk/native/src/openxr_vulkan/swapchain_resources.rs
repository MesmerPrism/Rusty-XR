use std::ptr;

use ash::vk::{self, Handle};
use openxr as xr;
use openxr::sys::Handle as _;

use crate::{log_error, log_info, runtime_config, OpenXrColorFormatMode};

use super::{
    ensure_xr_success, find_memory_type, VIEW_COUNT, VIEW_TYPE, XR_FOVEATION_DEPTH_FORMAT,
    XR_FRAGMENT_DENSITY_MAP_FORMAT, XR_RENDER_SCALE_DEFAULT,
};

pub(super) struct OpenXrSwapchainImages {
    pub(super) handle: xr::Swapchain<xr::Vulkan>,
    pub(super) color_images: Vec<u64>,
    pub(super) fragment_density_images: Vec<u64>,
    pub(super) fixed_foveation_enabled: bool,
}

pub(super) struct Swapchain {
    pub(super) handle: xr::Swapchain<xr::Vulkan>,
    pub(super) buffers: Vec<Framebuffer>,
    pub(super) resolution: vk::Extent2D,
    pub(super) foveation_enabled: bool,
}

pub(super) struct Framebuffer {
    pub(super) framebuffer: vk::Framebuffer,
    pub(super) color: vk::ImageView,
    pub(super) depth: Option<DepthAttachment>,
    pub(super) fragment_density: vk::ImageView,
    pub(super) image: vk::Image,
}

pub(super) struct DepthAttachment {
    pub(super) image: vk::Image,
    pub(super) view: vk::ImageView,
    pub(super) memory: vk::DeviceMemory,
}

pub(super) unsafe fn destroy_swapchain(device: &ash::Device, swapchain: Swapchain) {
    for buffer in swapchain.buffers {
        device.destroy_framebuffer(buffer.framebuffer, None);
        if buffer.fragment_density != vk::ImageView::null() {
            device.destroy_image_view(buffer.fragment_density, None);
        }
        if let Some(depth) = buffer.depth {
            device.destroy_image_view(depth.view, None);
            device.destroy_image(depth.image, None);
            device.free_memory(depth.memory, None);
        }
        device.destroy_image_view(buffer.color, None);
    }
}

fn scaled_extent(recommended: vk::Extent2D, scale: f32) -> vk::Extent2D {
    let scale = sanitized_render_scale(scale);
    vk::Extent2D {
        width: ((recommended.width.max(1) as f32) * scale).round().max(1.0) as u32,
        height: ((recommended.height.max(1) as f32) * scale)
            .round()
            .max(1.0) as u32,
    }
}

fn sanitized_render_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale.clamp(0.25, 1.5)
    } else {
        XR_RENDER_SCALE_DEFAULT
    }
}

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

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn ensure_swapchain<'a>(
    xr_instance: &xr::Instance,
    session: &xr::Session<xr::Vulkan>,
    system: xr::SystemId,
    vk_device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    render_pass: vk::RenderPass,
    color_format_mode: OpenXrColorFormatMode,
    color_format: vk::Format,
    fixed_foveation_render_path: bool,
    swapchain: &'a mut Option<Swapchain>,
) -> Result<&'a mut Swapchain, String> {
    if swapchain.is_none() {
        let views = xr_instance
            .enumerate_view_configuration_views(system, VIEW_TYPE)
            .map_err(|error| format!("enumerate OpenXR view configuration: {error}"))?;
        if views.len() != VIEW_COUNT as usize {
            return Err(format!(
                "expected {VIEW_COUNT} OpenXR views, got {}",
                views.len()
            ));
        }
        if views[0] != views[1] {
            return Err(
                "this minimal multiview example requires matching eye dimensions".to_string(),
            );
        }

        let recommended_resolution = vk::Extent2D {
            width: views[0].recommended_image_rect_width,
            height: views[0].recommended_image_rect_height,
        };
        let config = runtime_config();
        let render_scale = sanitized_render_scale(config.xr_render_scale);
        let fixed_foveation_level = config.xr_fixed_foveation_level;
        let resolution = scaled_extent(recommended_resolution, render_scale);
        let use_fixed_foveation = fixed_foveation_level > 0 && fixed_foveation_render_path;
        if fixed_foveation_level > 0 && !use_fixed_foveation {
            log_error(format!(
                "Rusty XR fixed foveation requested level={} but required OpenXR/Vulkan fragment-density path is unavailable",
                fixed_foveation_level
            ));
        }
        let created_swapchain = create_openxr_swapchain(
            xr_instance,
            session,
            resolution,
            color_format_mode,
            color_format,
            fixed_foveation_level,
            use_fixed_foveation,
        )?;
        let mut buffers = Vec::with_capacity(created_swapchain.color_images.len());
        for (index, color_image) in created_swapchain.color_images.iter().copied().enumerate() {
            let color_image = vk::Image::from_raw(color_image);
            let color = vk_device
                .create_image_view(
                    &vk::ImageViewCreateInfo::default()
                        .image(color_image)
                        .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
                        .format(color_format)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: VIEW_COUNT,
                        }),
                    None,
                )
                .map_err(|error| format!("create Vulkan swapchain image view: {error}"))?;
            let fragment_density = if created_swapchain.fixed_foveation_enabled {
                let fragment_density_image = created_swapchain
                    .fragment_density_images
                    .get(index)
                    .copied()
                    .ok_or_else(|| {
                        "OpenXR foveation image count did not match swapchain image count"
                            .to_string()
                    })?;
                if fragment_density_image == 0 {
                    return Err(format!(
                        "OpenXR foveation image handle was null for swapchain image {index}"
                    ));
                }
                create_fragment_density_image_view(
                    vk_device,
                    vk::Image::from_raw(fragment_density_image),
                )?
            } else {
                vk::ImageView::null()
            };
            let depth = if created_swapchain.fixed_foveation_enabled {
                Some(create_foveation_depth_attachment(
                    vk_device,
                    memory_properties,
                    resolution,
                )?)
            } else {
                None
            };
            let mut attachments = vec![color];
            if let Some(depth) = &depth {
                attachments.push(depth.view);
            }
            if fragment_density != vk::ImageView::null() {
                attachments.push(fragment_density);
            }
            if created_swapchain.fixed_foveation_enabled {
                log_info(format!(
                    "Rusty XR OpenXR foveation framebuffer plan index={} colorImage=0x{:x} colorView=0x{:x} depthView=0x{:x} fragmentDensityView=0x{:x} attachments={}",
                    index,
                    color_image.as_raw(),
                    color.as_raw(),
                    depth.as_ref().map(|value| value.view.as_raw()).unwrap_or_default(),
                    fragment_density.as_raw(),
                    attachments.len()
                ));
            }
            let framebuffer = vk_device
                .create_framebuffer(
                    &vk::FramebufferCreateInfo::default()
                        .render_pass(render_pass)
                        .width(resolution.width)
                        .height(resolution.height)
                        .attachments(&attachments)
                        .layers(1),
                    None,
                )
                .map_err(|error| format!("create Vulkan framebuffer: {error}"))?;
            buffers.push(Framebuffer {
                framebuffer,
                color,
                depth,
                fragment_density,
                image: color_image,
            });
        }

        log_info(format!(
            "Rusty XR OpenXR swapchain created {}x{} from recommended {}x{} scale={} xrColorFormat={} vkFormat={:?} fixedFoveationLevel={} fixedFoveationEnabled={} fragmentDensityMapImages={} with {} image(s)",
            resolution.width,
            resolution.height,
            recommended_resolution.width,
            recommended_resolution.height,
            render_scale,
            color_format_mode.stable_id(),
            color_format,
            fixed_foveation_level,
            created_swapchain.fixed_foveation_enabled,
            created_swapchain.fragment_density_images.len(),
            buffers.len()
        ));
        *swapchain = Some(Swapchain {
            handle: created_swapchain.handle,
            buffers,
            resolution,
            foveation_enabled: created_swapchain.fixed_foveation_enabled,
        });
    }

    swapchain
        .as_mut()
        .ok_or_else(|| "swapchain was not initialized".to_string())
}

unsafe fn create_openxr_swapchain(
    xr_instance: &xr::Instance,
    session: &xr::Session<xr::Vulkan>,
    resolution: vk::Extent2D,
    color_format_mode: OpenXrColorFormatMode,
    color_format: vk::Format,
    fixed_foveation_level: u8,
    use_fixed_foveation: bool,
) -> Result<OpenXrSwapchainImages, String> {
    let mut foveation_create_info = xr::sys::SwapchainCreateInfoFoveationFB {
        ty: xr::sys::SwapchainCreateInfoFoveationFB::TYPE,
        next: ptr::null_mut(),
        flags: xr::sys::SwapchainCreateFoveationFlagsFB::FRAGMENT_DENSITY_MAP,
    };
    let create_info = xr::sys::SwapchainCreateInfo {
        ty: xr::sys::SwapchainCreateInfo::TYPE,
        next: if use_fixed_foveation {
            &mut foveation_create_info as *mut _ as *const _
        } else {
            ptr::null()
        },
        create_flags: xr::sys::SwapchainCreateFlags::EMPTY,
        usage_flags: xr::sys::SwapchainUsageFlags::COLOR_ATTACHMENT
            | xr::sys::SwapchainUsageFlags::SAMPLED
            | xr::sys::SwapchainUsageFlags::TRANSFER_DST,
        format: color_format.as_raw() as _,
        sample_count: 1,
        width: resolution.width,
        height: resolution.height,
        face_count: 1,
        array_size: VIEW_COUNT,
        mip_count: 1,
    };
    log_info(format!(
        "Rusty XR OpenXR swapchain request {}x{} xrColorFormat={} vkFormat={:?} fixedFoveationLevel={} fixedFoveationRequested={}",
        resolution.width,
        resolution.height,
        color_format_mode.stable_id(),
        color_format,
        fixed_foveation_level,
        use_fixed_foveation
    ));
    let mut raw_handle = xr::sys::Swapchain::NULL;
    ensure_xr_success(
        (xr_instance.fp().create_swapchain)(session.as_raw(), &create_info, &mut raw_handle),
        "xrCreateSwapchain",
    )?;

    let handle = xr::Swapchain::from_raw(session.clone(), raw_handle);
    let mut fixed_foveation_enabled = false;
    if use_fixed_foveation {
        let level = desired_fixed_foveation_level(fixed_foveation_level)
            .ok_or_else(|| "invalid fixed foveation level".to_string())?;
        match enable_openxr_fixed_foveation(xr_instance, session, &handle, level) {
            Ok(()) => {
                fixed_foveation_enabled = true;
            }
            Err(error) => {
                log_error(format!(
                    "Rusty XR OpenXR fixed foveation enable failed; continuing without foveation: {error}"
                ));
            }
        }
    }

    let (color_images, fragment_density_images) = if fixed_foveation_enabled {
        enumerate_openxr_foveation_swapchain_images(xr_instance, &handle)?
    } else {
        (
            handle
                .enumerate_images()
                .map_err(|error| format!("enumerate OpenXR swapchain images: {error}"))?,
            Vec::new(),
        )
    };
    log_info(format!(
        "Rusty XR OpenXR swapchain image enumeration fixedFoveationEnabled={} colorImages={} fragmentDensityImages={}",
        fixed_foveation_enabled,
        color_images.len(),
        fragment_density_images.len()
    ));

    Ok(OpenXrSwapchainImages {
        handle,
        color_images,
        fragment_density_images,
        fixed_foveation_enabled,
    })
}

fn desired_fixed_foveation_level(level: u8) -> Option<xr::FoveationLevelFB> {
    match level {
        0 => None,
        1 => Some(xr::FoveationLevelFB::LOW),
        2 => Some(xr::FoveationLevelFB::MEDIUM),
        _ => Some(xr::FoveationLevelFB::HIGH),
    }
}

unsafe fn enable_openxr_fixed_foveation(
    xr_instance: &xr::Instance,
    session: &xr::Session<xr::Vulkan>,
    swapchain: &xr::Swapchain<xr::Vulkan>,
    level: xr::FoveationLevelFB,
) -> Result<(), String> {
    let update_swapchain = xr_instance
        .exts()
        .fb_swapchain_update_state
        .as_ref()
        .ok_or_else(|| "XR_FB_swapchain_update_state is unavailable".to_string())?;
    let profile = session
        .create_foveation_profile(Some(xr::FoveationLevelProfile {
            level,
            vertical_offset: 0.0,
            dynamic: xr::FoveationDynamicFB::DISABLED,
        }))
        .map_err(|error| format!("xrCreateFoveationProfileFB: {error}"))?;
    let state = xr::sys::SwapchainStateFoveationFB {
        ty: xr::sys::SwapchainStateFoveationFB::TYPE,
        next: ptr::null_mut(),
        flags: xr::sys::SwapchainStateFoveationFlagsFB::EMPTY,
        profile: profile.as_raw(),
    };
    ensure_xr_success(
        (update_swapchain.update_swapchain)(
            swapchain.as_raw(),
            &state as *const _ as *const xr::sys::SwapchainStateBaseHeaderFB,
        ),
        "xrUpdateSwapchainFB",
    )?;
    Ok(())
}

unsafe fn enumerate_openxr_foveation_swapchain_images(
    xr_instance: &xr::Instance,
    swapchain: &xr::Swapchain<xr::Vulkan>,
) -> Result<(Vec<u64>, Vec<u64>), String> {
    let mut image_count = 0;
    ensure_xr_success(
        (xr_instance.fp().enumerate_swapchain_images)(
            swapchain.as_raw(),
            0,
            &mut image_count,
            ptr::null_mut(),
        ),
        "xrEnumerateSwapchainImages(count)",
    )?;
    let mut color_images = vec![
        xr::sys::SwapchainImageVulkanKHR {
            ty: xr::sys::SwapchainImageVulkanKHR::TYPE,
            next: ptr::null_mut(),
            image: 0,
        };
        image_count as usize
    ];
    let mut fragment_density_images = vec![
        xr::sys::SwapchainImageFoveationVulkanFB {
            ty: xr::sys::SwapchainImageFoveationVulkanFB::TYPE,
            next: ptr::null_mut(),
            image: 0,
            width: 0,
            height: 0,
        };
        image_count as usize
    ];
    for (color, fragment_density) in color_images
        .iter_mut()
        .zip(fragment_density_images.iter_mut())
    {
        color.next = fragment_density as *mut _ as *mut _;
    }
    let mut enumerated = 0;
    ensure_xr_success(
        (xr_instance.fp().enumerate_swapchain_images)(
            swapchain.as_raw(),
            image_count,
            &mut enumerated,
            color_images.as_mut_ptr() as *mut xr::sys::SwapchainImageBaseHeader,
        ),
        "xrEnumerateSwapchainImages",
    )?;
    color_images.truncate(enumerated as usize);
    fragment_density_images.truncate(enumerated as usize);
    for (index, (color, fragment_density)) in color_images
        .iter()
        .zip(fragment_density_images.iter())
        .enumerate()
    {
        log_info(format!(
            "Rusty XR OpenXR foveation image index={} colorImage=0x{:x} fragmentDensityImage=0x{:x} fragmentDensitySize={}x{}",
            index,
            color.image,
            fragment_density.image,
            fragment_density.width,
            fragment_density.height
        ));
    }
    Ok((
        color_images.into_iter().map(|image| image.image).collect(),
        fragment_density_images
            .into_iter()
            .map(|image| image.image)
            .collect(),
    ))
}

unsafe fn create_fragment_density_image_view(
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

unsafe fn create_foveation_depth_attachment(
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
