use ash::vk;
use openxr as xr;

use crate::{log_error, log_info, runtime_config, OpenXrColorFormatMode};

use super::openxr_swapchain_images::create_openxr_swapchain;
use super::swapchain_framebuffers::{create_swapchain_framebuffers, Framebuffer};
use super::{VIEW_COUNT, VIEW_TYPE, XR_RENDER_SCALE_DEFAULT};

pub(super) struct Swapchain {
    pub(super) handle: xr::Swapchain<xr::Vulkan>,
    pub(super) buffers: Vec<Framebuffer>,
    pub(super) resolution: vk::Extent2D,
    pub(super) foveation_enabled: bool,
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
        let buffers = create_swapchain_framebuffers(
            vk_device,
            memory_properties,
            render_pass,
            color_format,
            resolution,
            created_swapchain.fixed_foveation_enabled,
            &created_swapchain.color_images,
            &created_swapchain.fragment_density_images,
        )?;

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
