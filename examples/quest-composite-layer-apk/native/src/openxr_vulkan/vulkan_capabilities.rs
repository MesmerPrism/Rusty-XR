use std::ffi::CStr;

use ash::vk;

use super::XR_FRAGMENT_DENSITY_MAP_FORMAT;

pub(super) unsafe fn query_fragment_density_map_support(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<bool, String> {
    if !physical_device_supports_extension(
        instance,
        physical_device,
        ash::ext::fragment_density_map::NAME,
    )? {
        return Ok(false);
    }

    let mut features = vk::PhysicalDeviceFragmentDensityMapFeaturesEXT::default();
    let mut features2 = vk::PhysicalDeviceFeatures2::default().push_next(&mut features);
    instance.get_physical_device_features2(physical_device, &mut features2);
    if features.fragment_density_map != vk::TRUE {
        return Ok(false);
    }

    let format_props = instance
        .get_physical_device_format_properties(physical_device, XR_FRAGMENT_DENSITY_MAP_FORMAT);
    Ok(format_props
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::FRAGMENT_DENSITY_MAP_EXT))
}

pub(super) unsafe fn physical_device_supports_extension(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    extension_name: &CStr,
) -> Result<bool, String> {
    let extensions = instance
        .enumerate_device_extension_properties(physical_device)
        .map_err(|error| format!("enumerate Vulkan device extensions: {error}"))?;
    Ok(extensions
        .iter()
        .any(|extension| CStr::from_ptr(extension.extension_name.as_ptr()) == extension_name))
}
