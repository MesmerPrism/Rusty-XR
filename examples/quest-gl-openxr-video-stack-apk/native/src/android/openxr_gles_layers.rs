use openxr as xr;

use super::openxr_gles_resources::EyeSwapchain;

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
