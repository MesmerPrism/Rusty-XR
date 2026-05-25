use std::ptr;

use ash::vk;
use openxr as xr;
use openxr::sys::Handle as _;

use crate::{log_error, log_info, OpenXrColorFormatMode};

use super::{
    ensure_xr_success,
    openxr_foveation_swapchain::{
        enable_requested_openxr_fixed_foveation, enumerate_openxr_foveation_swapchain_images,
    },
};

pub(super) struct OpenXrSwapchainImages {
    pub(super) handle: xr::Swapchain<xr::Vulkan>,
    pub(super) color_images: Vec<u64>,
    pub(super) fragment_density_images: Vec<u64>,
    pub(super) fixed_foveation_enabled: bool,
}

pub(super) unsafe fn create_openxr_swapchain(
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
        array_size: super::VIEW_COUNT,
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
        match enable_requested_openxr_fixed_foveation(
            xr_instance,
            session,
            &handle,
            fixed_foveation_level,
        ) {
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
