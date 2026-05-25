use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::full_view_content_uv_scale;

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::{
    gpu_camera_projection_uniforms::CameraProjectionUniforms,
    projection_geometry::{projected_stereo_homographies, ProjectedStereoHomographies},
    projection_homography_utils::pack_homography_row,
    source_content_geometry::source_uv_rect_xywh_for_diagnostics,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct CameraProjectionPush {
    params: [f32; 4],
    color_adjust: [f32; 4],
    effect_params: [f32; 4],
    alpha_params: [f32; 4],
    area_params: [f32; 4],
    area_offset_params: [f32; 4],
    left_h0: [f32; 4],
    left_h1: [f32; 4],
    left_h2: [f32; 4],
    right_h0: [f32; 4],
    right_h1: [f32; 4],
    right_h2: [f32; 4],
}

impl CameraProjectionPush {
    pub(super) fn from_frame(
        _frame: &HeadsetCameraGpuFrame,
        config: &crate::RuntimeConfig,
    ) -> Self {
        let mono_flags = config.camera_texture_transform.shader_flags() & 0x1f;
        let packed_flags = (mono_flags | (mono_flags << 5))
            | config.camera_color_mode.shader_bit()
            | config.camera_feed_pipeline_mode.shader_bit()
            | config.camera_projection_effect_mode.shader_bit()
            | config.camera_projection_border_policy_shader_bit();
        let content_uv_scale = full_view_content_uv_scale(
            config.camera_full_view_overlay_overscan,
            config.camera_raw_overlay_overscan,
        )
        .unwrap_or(1.0);
        Self {
            params: [
                config.camera_raw_overlay_overscan.max(1.0),
                config.camera_edge_fade.clamp(0.0, 0.5),
                content_uv_scale,
                packed_flags as f32,
            ],
            color_adjust: config.camera_color_adjust_push(),
            effect_params: config.camera_effect_params_push(),
            alpha_params: config.camera_alpha_params_push(),
            area_params: config.camera_area_params_push(),
            area_offset_params: config.camera_area_offset_params_push(),
            left_h0: [1.0, 0.0, 0.0, 0.0],
            left_h1: [0.0, 1.0, 0.0, 0.0],
            left_h2: [0.0, 0.0, 1.0, 0.0],
            right_h0: [1.0, 0.0, 0.0, 0.0],
            right_h1: [0.0, 1.0, 0.0, 0.0],
            right_h2: [0.0, 0.0, 1.0, 0.0],
        }
    }

    pub(super) fn from_stereo_frame(
        frame: &StereoGpuCameraFrame,
        config: &crate::RuntimeConfig,
        controls: &crate::StereoProjectionControls,
        views: &[xr::View],
        resolution: vk::Extent2D,
    ) -> (
        Self,
        CameraProjectionUniforms,
        Option<ProjectedStereoHomographies>,
    ) {
        let content_uv_scale = full_view_content_uv_scale(
            config.camera_full_view_overlay_overscan,
            config.camera_raw_overlay_overscan,
        )
        .unwrap_or(1.0);
        let push = Self {
            params: [
                config.camera_raw_overlay_overscan.max(1.0),
                config.camera_edge_fade.clamp(0.0, 0.5),
                content_uv_scale,
                (controls.packed_shader_flags()
                    | config.camera_color_mode.shader_bit()
                    | config.camera_feed_pipeline_mode.shader_bit()
                    | config.camera_projection_effect_mode.shader_bit()
                    | config.camera_projection_border_policy_shader_bit()) as f32,
            ],
            color_adjust: config.camera_color_adjust_push(),
            effect_params: config.camera_effect_params_push(),
            alpha_params: config.camera_alpha_params_push(),
            area_params: config.camera_area_params_push(),
            area_offset_params: config.camera_area_offset_params_push(),
            left_h0: [1.0, 0.0, 0.0, 0.0],
            left_h1: [0.0, 1.0, 0.0, 0.0],
            left_h2: [0.0, 0.0, 1.0, 0.0],
            right_h0: [1.0, 0.0, 0.0, 0.0],
            right_h1: [0.0, 1.0, 0.0, 0.0],
            right_h2: [0.0, 0.0, 1.0, 0.0],
        };
        if !controls.left_texture_transform.is_explicit_visual_check()
            || !controls.right_texture_transform.is_explicit_visual_check()
        {
            return (
                push,
                CameraProjectionUniforms::identity()
                    .with_source_uv_rects(
                        source_uv_rect_xywh_for_diagnostics(&frame.left.diagnostics),
                        source_uv_rect_xywh_for_diagnostics(&frame.right.diagnostics),
                    )
                    .with_color_config(config),
                None,
            );
        }

        if let Some((left, right)) =
            projected_stereo_homographies(frame, config, controls, views, resolution)
        {
            let homographies = ProjectedStereoHomographies { left, right };
            let (push, uniforms) =
                Self::from_projected_stereo_homographies(frame, config, controls, &homographies);
            return (push, uniforms, Some(homographies));
        }
        (
            push,
            CameraProjectionUniforms::identity()
                .with_source_uv_rects(
                    source_uv_rect_xywh_for_diagnostics(&frame.left.diagnostics),
                    source_uv_rect_xywh_for_diagnostics(&frame.right.diagnostics),
                )
                .with_color_config(config),
            None,
        )
    }

    pub(super) fn from_projected_stereo_homographies(
        frame: &StereoGpuCameraFrame,
        config: &crate::RuntimeConfig,
        controls: &crate::StereoProjectionControls,
        homographies: &ProjectedStereoHomographies,
    ) -> (Self, CameraProjectionUniforms) {
        let content_uv_scale = full_view_content_uv_scale(
            config.camera_full_view_overlay_overscan,
            config.camera_raw_overlay_overscan,
        )
        .unwrap_or(1.0);
        let full_frame_mapping_flags = if homographies.left.full_frame_stimulus_mapping
            && homographies.right.full_frame_stimulus_mapping
        {
            crate::camera_color_pipeline::CAMERA_SHADER_FLAG_FULL_FRAME_STIMULUS_MAPPING
        } else {
            0
        };
        let mut push = Self {
            params: [
                -config.camera_raw_overlay_overscan.max(1.0),
                config.camera_edge_fade.clamp(0.0, 0.5),
                content_uv_scale,
                (controls.packed_shader_flags()
                    | config.camera_color_mode.shader_bit()
                    | config.camera_feed_pipeline_mode.shader_bit()
                    | config.camera_projection_effect_mode.shader_bit()
                    | config.camera_projection_border_policy_shader_bit()
                    | full_frame_mapping_flags) as f32,
            ],
            color_adjust: config.camera_color_adjust_push(),
            effect_params: config.camera_effect_params_push(),
            alpha_params: config.camera_alpha_params_push(),
            area_params: config.camera_area_params_push(),
            area_offset_params: config.camera_area_offset_params_push(),
            left_h0: [1.0, 0.0, 0.0, 0.0],
            left_h1: [0.0, 1.0, 0.0, 0.0],
            left_h2: [0.0, 0.0, 1.0, 0.0],
            right_h0: [1.0, 0.0, 0.0, 0.0],
            right_h1: [0.0, 1.0, 0.0, 0.0],
            right_h2: [0.0, 0.0, 1.0, 0.0],
        };
        let left_sample_rows = if config.camera_projection_mode.uses_world_canvas() {
            homographies.left.surface_to_camera
        } else {
            homographies.left.screen_to_camera
        };
        let right_sample_rows = if config.camera_projection_mode.uses_world_canvas() {
            homographies.right.surface_to_camera
        } else {
            homographies.right.screen_to_camera
        };
        push.left_h0 = pack_homography_row(left_sample_rows[0]);
        push.left_h1 = pack_homography_row(left_sample_rows[1]);
        push.left_h2 = pack_homography_row(left_sample_rows[2]);
        push.right_h0 = pack_homography_row(right_sample_rows[0]);
        push.right_h1 = pack_homography_row(right_sample_rows[1]);
        push.right_h2 = pack_homography_row(right_sample_rows[2]);
        (
            push,
            CameraProjectionUniforms::from_mappings(&homographies.left, &homographies.right)
                .with_source_uv_rects(
                    source_uv_rect_xywh_for_diagnostics(&frame.left.diagnostics),
                    source_uv_rect_xywh_for_diagnostics(&frame.right.diagnostics),
                )
                .with_color_config(config),
        )
    }
}
