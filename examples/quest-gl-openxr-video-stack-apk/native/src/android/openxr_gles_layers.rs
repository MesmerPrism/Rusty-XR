use std::ptr;

use openxr as xr;
use openxr::sys::Handle as _;

use super::{
    openxr_gles_passthrough::OpenXrGlesPassthroughUnderlay, openxr_gles_resources::EyeSwapchain,
};

pub(super) fn projection_views_from_swapchains<'a>(
    views: &[xr::View],
    swapchains: &'a [EyeSwapchain],
) -> Vec<xr::CompositionLayerProjectionView<'a, xr::OpenGlEs>> {
    let mut projection_views = Vec::with_capacity(swapchains.len().min(views.len()));
    for (index, eye) in swapchains.iter().enumerate() {
        let Some(view) = views.get(index) else {
            continue;
        };
        projection_views.push(
            xr::CompositionLayerProjectionView::new()
                .pose(view.pose)
                .fov(view.fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&eye.handle)
                        .image_array_index(0)
                        .image_rect(xr::Rect2Di {
                            offset: xr::Offset2Di { x: 0, y: 0 },
                            extent: xr::Extent2Di {
                                width: eye.width as i32,
                                height: eye.height as i32,
                            },
                        }),
                ),
        );
    }
    projection_views
}

pub(super) fn end_empty_openxr_frame(
    frame_stream: &mut xr::FrameStream<xr::OpenGlEs>,
    predicted_display_time: xr::Time,
    environment_blend_mode: xr::EnvironmentBlendMode,
    operation: &str,
) -> Result<(), String> {
    frame_stream
        .end(predicted_display_time, environment_blend_mode, &[])
        .map_err(|error| format!("{operation}: {error}"))
}

pub(super) fn end_projection_openxr_frame(
    frame_stream: &mut xr::FrameStream<xr::OpenGlEs>,
    predicted_display_time: xr::Time,
    environment_blend_mode: xr::EnvironmentBlendMode,
    stage: &xr::Space,
    projection_views: &[xr::CompositionLayerProjectionView<'_, xr::OpenGlEs>],
    projection_uses_source_alpha: bool,
    native_passthrough_underlay: Option<&OpenXrGlesPassthroughUnderlay>,
) -> Result<(), String> {
    let layer = xr::CompositionLayerProjection::new()
        .layer_flags(if projection_uses_source_alpha {
            xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA
        } else {
            xr::CompositionLayerFlags::EMPTY
        })
        .space(stage)
        .views(projection_views);
    let passthrough_layer =
        native_passthrough_underlay.map(|underlay| xr::sys::CompositionLayerPassthroughFB {
            ty: xr::sys::CompositionLayerPassthroughFB::TYPE,
            next: ptr::null(),
            flags: xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
            space: xr::sys::Space::NULL,
            layer_handle: underlay.layer,
        });
    let mut layers: Vec<&xr::CompositionLayerBase<xr::OpenGlEs>> =
        Vec::with_capacity(1 + usize::from(passthrough_layer.is_some()));
    if let Some(passthrough_layer) = passthrough_layer.as_ref() {
        let layer_base: &xr::CompositionLayerBase<xr::OpenGlEs> = unsafe {
            &*(passthrough_layer as *const xr::sys::CompositionLayerPassthroughFB
                as *const xr::CompositionLayerBase<xr::OpenGlEs>)
        };
        layers.push(layer_base);
    }
    layers.push(&layer);
    frame_stream
        .end(predicted_display_time, environment_blend_mode, &layers)
        .map_err(|error| format!("end OpenXR frame: {error}"))
}
