use std::ptr;

use openxr as xr;
use openxr::sys::Handle as _;

use super::swapchain_resources::Swapchain;

pub(super) struct OpenXrVulkanLayerSubmission<'a> {
    pub(super) predicted_display_time: xr::Time,
    pub(super) environment_blend_mode: xr::EnvironmentBlendMode,
    pub(super) reference_space: &'a xr::Space,
    pub(super) views: &'a [xr::View],
    pub(super) swapchain: &'a Swapchain,
    pub(super) projection_uses_source_alpha: bool,
    pub(super) passthrough_layer: Option<xr::sys::PassthroughLayerFB>,
    pub(super) projection_layer_visible: bool,
}

pub(super) fn end_projection_openxr_frame(
    frame_stream: &mut xr::FrameStream<xr::Vulkan>,
    submission: OpenXrVulkanLayerSubmission<'_>,
) -> Result<usize, String> {
    let projection_views = projection_views_from_swapchain(submission.views, submission.swapchain);
    let projection_layer = xr::CompositionLayerProjection::new()
        .layer_flags(if submission.projection_uses_source_alpha {
            xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA
        } else {
            xr::CompositionLayerFlags::EMPTY
        })
        .space(submission.reference_space)
        .views(&projection_views);
    let passthrough_composition_layer =
        submission
            .passthrough_layer
            .map(|layer_handle| xr::sys::CompositionLayerPassthroughFB {
                ty: xr::sys::CompositionLayerPassthroughFB::TYPE,
                next: ptr::null(),
                flags: xr::CompositionLayerFlags::BLEND_TEXTURE_SOURCE_ALPHA,
                space: xr::sys::Space::NULL,
                layer_handle,
            });
    let mut layers: Vec<&xr::CompositionLayerBase<xr::Vulkan>> = Vec::with_capacity(
        (passthrough_composition_layer.is_some() as usize)
            + (submission.projection_layer_visible as usize),
    );
    if let Some(layer) = passthrough_composition_layer.as_ref() {
        let layer_base: &xr::CompositionLayerBase<xr::Vulkan> = unsafe {
            &*(layer as *const xr::sys::CompositionLayerPassthroughFB
                as *const xr::CompositionLayerBase<xr::Vulkan>)
        };
        layers.push(layer_base);
    }
    if submission.projection_layer_visible {
        layers.push(&projection_layer);
    }
    let submitted_layer_count = layers.len();
    frame_stream
        .end(
            submission.predicted_display_time,
            submission.environment_blend_mode,
            &layers,
        )
        .map_err(|error| format!("end OpenXR frame: {error}"))?;
    Ok(submitted_layer_count)
}

fn projection_views_from_swapchain<'a>(
    views: &[xr::View],
    swapchain: &'a Swapchain,
) -> [xr::CompositionLayerProjectionView<'a, xr::Vulkan>; 2] {
    let rect = xr::Rect2Di {
        offset: xr::Offset2Di { x: 0, y: 0 },
        extent: xr::Extent2Di {
            width: swapchain.resolution.width as _,
            height: swapchain.resolution.height as _,
        },
    };
    [
        xr::CompositionLayerProjectionView::new()
            .pose(views[0].pose)
            .fov(views[0].fov)
            .sub_image(
                xr::SwapchainSubImage::new()
                    .swapchain(&swapchain.handle)
                    .image_array_index(0)
                    .image_rect(rect),
            ),
        xr::CompositionLayerProjectionView::new()
            .pose(views[1].pose)
            .fov(views[1].fov)
            .sub_image(
                xr::SwapchainSubImage::new()
                    .swapchain(&swapchain.handle)
                    .image_array_index(1)
                    .image_rect(rect),
            ),
    ]
}
