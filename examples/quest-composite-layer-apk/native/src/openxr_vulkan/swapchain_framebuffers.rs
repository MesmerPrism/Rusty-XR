use ash::vk::{self, Handle};

use crate::log_info;

use super::{
    foveation_framebuffer_resources::{
        create_foveation_depth_attachment, create_fragment_density_image_view, DepthAttachment,
    },
    VIEW_COUNT,
};

pub(super) struct Framebuffer {
    pub(super) framebuffer: vk::Framebuffer,
    pub(super) color: vk::ImageView,
    pub(super) depth: Option<DepthAttachment>,
    pub(super) fragment_density: vk::ImageView,
    pub(super) image: vk::Image,
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn create_swapchain_framebuffers(
    vk_device: &ash::Device,
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    render_pass: vk::RenderPass,
    color_format: vk::Format,
    resolution: vk::Extent2D,
    fixed_foveation_enabled: bool,
    color_images: &[u64],
    fragment_density_images: &[u64],
) -> Result<Vec<Framebuffer>, String> {
    let mut buffers = Vec::with_capacity(color_images.len());
    for (index, color_image) in color_images.iter().copied().enumerate() {
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
        let fragment_density = if fixed_foveation_enabled {
            let fragment_density_image =
                fragment_density_images.get(index).copied().ok_or_else(|| {
                    "OpenXR foveation image count did not match swapchain image count".to_string()
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
        let depth = if fixed_foveation_enabled {
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
        if fixed_foveation_enabled {
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
    Ok(buffers)
}
