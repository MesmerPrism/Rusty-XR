use openxr as xr;

pub(super) struct EyeSwapchain {
    pub(super) handle: xr::Swapchain<xr::OpenGlEs>,
    pub(super) images: Vec<u32>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) color_format: u32,
    pub(super) view_index: usize,
    pub(super) pattern: &'static str,
}
