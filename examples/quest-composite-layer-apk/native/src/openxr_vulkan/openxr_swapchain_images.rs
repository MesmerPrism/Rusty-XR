use std::ptr;

use ash::vk;
use openxr as xr;
use openxr::sys::Handle as _;

use crate::{log_error, log_info, OpenXrColorFormatMode};

use super::ensure_xr_success;

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
