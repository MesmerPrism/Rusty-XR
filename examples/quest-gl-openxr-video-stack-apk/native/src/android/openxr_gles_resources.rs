use openxr as xr;
use rusty_xr_quest_diagnostics::{
    OpenXrGlesFeasibilityStatus, OpenXrGlesSwapchainFormat, OpenXrGlesViewStatus,
};

use super::{
    log_info, GL_DEPTH24_STENCIL8, GL_DEPTH_COMPONENT16, GL_DEPTH_COMPONENT24, GL_RGB10_A2,
    GL_RGBA, GL_RGBA8, GL_SRGB8_ALPHA8, VIEW_COUNT, VIEW_TYPE,
};

pub(super) struct EyeSwapchain {
    pub(super) handle: xr::Swapchain<xr::OpenGlEs>,
    pub(super) images: Vec<u32>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) color_format: u32,
    pub(super) view_index: usize,
    pub(super) pattern: &'static str,
}

pub(super) fn create_eye_swapchains(
    instance: &xr::Instance,
    system: xr::SystemId,
    session: &xr::Session<xr::OpenGlEs>,
    status: &mut OpenXrGlesFeasibilityStatus,
) -> Result<Vec<EyeSwapchain>, String> {
    let view_configs = instance
        .enumerate_view_configuration_views(system, VIEW_TYPE)
        .map_err(|error| format!("enumerate view configuration views: {error}"))?;
    if view_configs.len() < VIEW_COUNT {
        return Err(format!(
            "OpenXR runtime reported {} view(s), expected at least {VIEW_COUNT}",
            view_configs.len()
        ));
    }

    let formats = session
        .enumerate_swapchain_formats()
        .map_err(|error| format!("enumerate OpenXR GLES swapchain formats: {error}"))?;
    let selected_format = select_color_format(&formats)
        .ok_or_else(|| "OpenXR runtime reported no GLES swapchain formats".to_string())?;
    status.swapchain_formats = formats
        .iter()
        .filter(|format| {
            **format == selected_format || is_color_format(**format) || is_depth_format(**format)
        })
        .map(|format| OpenXrGlesSwapchainFormat {
            format_id: *format as i64,
            label: gl_format_label(*format).to_string(),
            color_renderable: is_color_format(*format),
            depth_renderable: is_depth_format(*format),
            selected: *format == selected_format,
        })
        .collect();
    log_info(format!(
        "Rusty XR OpenXR GLES swapchain formats selected={} runtimeFormatCount={} trackedFormats={:?}",
        gl_format_label(selected_format),
        formats.len(),
        status
            .swapchain_formats
            .iter()
            .map(|format| format.label.as_str())
            .collect::<Vec<_>>()
    ));

    let mut swapchains = Vec::with_capacity(VIEW_COUNT);
    status.views.clear();
    for (index, view) in view_configs.iter().take(VIEW_COUNT).enumerate() {
        let width = view.recommended_image_rect_width;
        let height = view.recommended_image_rect_height;
        let handle = session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                format: selected_format,
                sample_count: 1,
                width,
                height,
                face_count: 1,
                array_size: 1,
                mip_count: 1,
            })
            .map_err(|error| format!("create OpenXR GLES swapchain for eye {index}: {error}"))?;
        let images = handle
            .enumerate_images()
            .map_err(|error| format!("enumerate GLES swapchain images for eye {index}: {error}"))?;
        let pattern = if index == 0 {
            "left-red-cyan-grid"
        } else {
            "right-blue-yellow-grid"
        };
        status.views.push(OpenXrGlesViewStatus::diagnostic_grid(
            index as u32,
            width,
            height,
            pattern,
        ));
        log_info(format!(
            "Rusty XR OpenXR GLES swapchain eye={} size={}x{} images={} sampleCountRecommended={}",
            index,
            width,
            height,
            images.len(),
            view.recommended_swapchain_sample_count
        ));
        swapchains.push(EyeSwapchain {
            handle,
            images,
            width,
            height,
            color_format: selected_format,
            view_index: index,
            pattern,
        });
    }

    Ok(swapchains)
}

fn select_color_format(formats: &[u32]) -> Option<u32> {
    [GL_SRGB8_ALPHA8, GL_RGBA8, GL_RGB10_A2, GL_RGBA]
        .into_iter()
        .find(|preferred| formats.contains(preferred))
        .or_else(|| formats.first().copied())
}

fn is_color_format(format: u32) -> bool {
    matches!(format, GL_SRGB8_ALPHA8 | GL_RGBA8 | GL_RGB10_A2 | GL_RGBA)
}

fn is_depth_format(format: u32) -> bool {
    matches!(
        format,
        GL_DEPTH_COMPONENT16 | GL_DEPTH_COMPONENT24 | GL_DEPTH24_STENCIL8
    )
}

pub(super) fn gl_format_label(format: u32) -> &'static str {
    match format {
        GL_SRGB8_ALPHA8 => "GL_SRGB8_ALPHA8",
        GL_RGBA8 => "GL_RGBA8",
        GL_RGB10_A2 => "GL_RGB10_A2",
        GL_RGBA => "GL_RGBA",
        GL_DEPTH_COMPONENT16 => "GL_DEPTH_COMPONENT16",
        GL_DEPTH_COMPONENT24 => "GL_DEPTH_COMPONENT24",
        GL_DEPTH24_STENCIL8 => "GL_DEPTH24_STENCIL8",
        _ => "GL_UNKNOWN",
    }
}
