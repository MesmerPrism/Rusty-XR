use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::CameraCompositeTier;

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::{
    gpu_camera_projection::{source_uv_rect_xywh_for_frame, CameraProjectionPush},
    gpu_camera_projection_uniforms::CameraProjectionUniforms,
    gpu_camera_resources::{
        GpuCameraImport, GpuCameraPipelineResources, GpuCameraStereoDescriptor,
    },
    gpu_camera_uniforms::update_camera_projection_uniforms,
    log_error,
    projection_geometry::ProjectedStereoHomographies,
};

pub(super) unsafe fn record_camera_draw(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    resolution: vk::Extent2D,
    resources: &GpuCameraPipelineResources,
    import: &GpuCameraImport,
    frame: &HeadsetCameraGpuFrame,
    config: &crate::RuntimeConfig,
) {
    let viewport = [vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: resolution.width as f32,
        height: resolution.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    }];
    let scissor = [vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: resolution,
    }];
    let push = CameraProjectionPush::from_frame(frame, config);
    let source_uv_rect = source_uv_rect_xywh_for_frame(frame);
    let uniforms = CameraProjectionUniforms::identity()
        .with_source_uv_rects(source_uv_rect, source_uv_rect)
        .with_color_config(config);
    let uniform_offset = resources.projection_uniform_offset(0);
    if let Err(error) =
        update_camera_projection_uniforms(device, resources, uniform_offset, &uniforms)
    {
        log_error(format!(
            "Rusty XR update mono camera projection uniforms failed: {error}"
        ));
        return;
    }
    device.cmd_set_viewport(cmd, 0, &viewport);
    device.cmd_set_scissor(cmd, 0, &scissor);
    device.cmd_bind_pipeline(
        cmd,
        vk::PipelineBindPoint::GRAPHICS,
        resources.pipeline_for_config(config),
    );
    device.cmd_bind_descriptor_sets(
        cmd,
        vk::PipelineBindPoint::GRAPHICS,
        resources.pipeline_layout,
        0,
        &[import.descriptor_set],
        &[uniform_offset],
    );
    let push_bytes = std::slice::from_raw_parts(
        (&push as *const CameraProjectionPush).cast::<u8>(),
        std::mem::size_of::<CameraProjectionPush>(),
    );
    device.cmd_push_constants(
        cmd,
        resources.pipeline_layout,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        0,
        push_bytes,
    );
    let vertex_count = if config.camera_projection_mode.uses_world_canvas() {
        6
    } else {
        3
    };
    device.cmd_draw(cmd, vertex_count, 1, 0, 0);
}

#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn record_stereo_camera_draw(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    resolution: vk::Extent2D,
    resources: &GpuCameraPipelineResources,
    descriptor: &GpuCameraStereoDescriptor,
    frame: &StereoGpuCameraFrame,
    config: &crate::RuntimeConfig,
    views: &[xr::View],
    frame_count: u64,
    applied_projection_homographies: Option<ProjectedStereoHomographies>,
) {
    let viewport = [vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: resolution.width as f32,
        height: resolution.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    }];
    let scissor = [vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: resolution,
    }];
    let controls = config.stereo_projection_controls(frame_count);
    let (push, uniforms, projection_homographies) =
        if let Some(homographies) = applied_projection_homographies {
            let (push, uniforms) = CameraProjectionPush::from_projected_stereo_homographies(
                frame,
                config,
                &controls,
                &homographies,
            );
            (push, uniforms, Some(homographies))
        } else {
            CameraProjectionPush::from_stereo_frame(frame, config, &controls, views, resolution)
        };
    let projection_active = projection_homographies.is_some();
    let accepted_flat_visual_check = config.visual_release_accepted
        && controls.left_texture_transform.is_explicit_visual_check()
        && controls.right_texture_transform.is_explicit_visual_check();
    if config.camera_tier == CameraCompositeTier::GpuProjected
        && !projection_active
        && !accepted_flat_visual_check
    {
        return;
    }
    let uniform_offset = resources.projection_uniform_offset(frame_count);
    if let Err(error) =
        update_camera_projection_uniforms(device, resources, uniform_offset, &uniforms)
    {
        log_error(format!(
            "Rusty XR update stereo camera projection uniforms failed: {error}"
        ));
        return;
    }
    device.cmd_set_viewport(cmd, 0, &viewport);
    device.cmd_set_scissor(cmd, 0, &scissor);
    device.cmd_bind_pipeline(
        cmd,
        vk::PipelineBindPoint::GRAPHICS,
        resources.pipeline_for_config(config),
    );
    device.cmd_bind_descriptor_sets(
        cmd,
        vk::PipelineBindPoint::GRAPHICS,
        resources.pipeline_layout,
        0,
        &[descriptor.descriptor_set],
        &[uniform_offset],
    );
    let push_bytes = std::slice::from_raw_parts(
        (&push as *const CameraProjectionPush).cast::<u8>(),
        std::mem::size_of::<CameraProjectionPush>(),
    );
    device.cmd_push_constants(
        cmd,
        resources.pipeline_layout,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        0,
        push_bytes,
    );
    let vertex_count = if config.camera_projection_mode.uses_world_canvas() && projection_active {
        6
    } else {
        3
    };
    device.cmd_draw(cmd, vertex_count, 1, 0, 0);
}
