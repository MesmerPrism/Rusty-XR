use ash::vk::{self, Handle};
use openxr as xr;

use crate::{log_error, log_info, runtime_config, OpenXrColorFormatMode};

use super::foveation_framebuffer_resources::{
    create_foveation_depth_attachment, create_fragment_density_image_view, DepthAttachment,
};
use super::openxr_swapchain_images::create_openxr_swapchain;
use super::{VIEW_COUNT, VIEW_TYPE, XR_RENDER_SCALE_DEFAULT};

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
