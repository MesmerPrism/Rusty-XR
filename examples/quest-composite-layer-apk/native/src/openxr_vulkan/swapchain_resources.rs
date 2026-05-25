use ash::vk;
use openxr as xr;

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
