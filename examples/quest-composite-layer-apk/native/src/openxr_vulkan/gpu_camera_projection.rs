use ash::vk;
use openxr as xr;
use rusty_xr_camera_model::full_view_content_uv_scale;

use crate::{HeadsetCameraGpuFrame, StereoGpuCameraFrame};

use super::{
    projection_geometry::{
        projected_stereo_homographies, DisplayEyeProjectionMapping, ProjectedStereoHomographies,
    },
    projection_homography_utils::{identity_homography, pack_homography_row},
    source_content_geometry::source_uv_rect_ltrb_for_diagnostics,
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

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct CameraProjectionUniforms {
    left_screen_to_surface_h0: [f32; 4],
    left_screen_to_surface_h1: [f32; 4],
    left_screen_to_surface_h2: [f32; 4],
    right_screen_to_surface_h0: [f32; 4],
    right_screen_to_surface_h1: [f32; 4],
    right_screen_to_surface_h2: [f32; 4],
    left_surface_to_screen_h0: [f32; 4],
    left_surface_to_screen_h1: [f32; 4],
    left_surface_to_screen_h2: [f32; 4],
    right_surface_to_screen_h0: [f32; 4],
    right_surface_to_screen_h1: [f32; 4],
    right_surface_to_screen_h2: [f32; 4],
    color_matrix_r0: [f32; 4],
    color_matrix_r1: [f32; 4],
    color_matrix_r2: [f32; 4],
    color_offset: [f32; 4],
    left_source_uv_rect: [f32; 4],
    right_source_uv_rect: [f32; 4],
    left_canvas_clip0: [f32; 4],
    left_canvas_clip1: [f32; 4],
    left_canvas_clip2: [f32; 4],
    left_canvas_clip3: [f32; 4],
    right_canvas_clip0: [f32; 4],
    right_canvas_clip1: [f32; 4],
    right_canvas_clip2: [f32; 4],
    right_canvas_clip3: [f32; 4],
}

impl CameraProjectionUniforms {
    pub(super) fn identity() -> Self {
        let h = identity_homography();
        Self::from_rows(&h, &h, &h, &h)
    }

    fn from_mappings(
        left: &DisplayEyeProjectionMapping,
        right: &DisplayEyeProjectionMapping,
    ) -> Self {
        let mut uniforms = Self::from_rows(
            &left.screen_to_surface,
            &right.screen_to_surface,
            &left.surface_to_screen,
            &right.surface_to_screen,
        );
        uniforms.left_canvas_clip0 = left.canvas_clip[0];
        uniforms.left_canvas_clip1 = left.canvas_clip[1];
        uniforms.left_canvas_clip2 = left.canvas_clip[2];
        uniforms.left_canvas_clip3 = left.canvas_clip[3];
        uniforms.right_canvas_clip0 = right.canvas_clip[0];
        uniforms.right_canvas_clip1 = right.canvas_clip[1];
        uniforms.right_canvas_clip2 = right.canvas_clip[2];
        uniforms.right_canvas_clip3 = right.canvas_clip[3];
        uniforms
    }

    fn from_rows(
        left_screen_to_surface: &[[f32; 3]; 3],
        right_screen_to_surface: &[[f32; 3]; 3],
        left_surface_to_screen: &[[f32; 3]; 3],
        right_surface_to_screen: &[[f32; 3]; 3],
    ) -> Self {
        Self {
            left_screen_to_surface_h0: pack_homography_row(left_screen_to_surface[0]),
            left_screen_to_surface_h1: pack_homography_row(left_screen_to_surface[1]),
            left_screen_to_surface_h2: pack_homography_row(left_screen_to_surface[2]),
            right_screen_to_surface_h0: pack_homography_row(right_screen_to_surface[0]),
            right_screen_to_surface_h1: pack_homography_row(right_screen_to_surface[1]),
            right_screen_to_surface_h2: pack_homography_row(right_screen_to_surface[2]),
            left_surface_to_screen_h0: pack_homography_row(left_surface_to_screen[0]),
            left_surface_to_screen_h1: pack_homography_row(left_surface_to_screen[1]),
            left_surface_to_screen_h2: pack_homography_row(left_surface_to_screen[2]),
            right_surface_to_screen_h0: pack_homography_row(right_surface_to_screen[0]),
            right_surface_to_screen_h1: pack_homography_row(right_surface_to_screen[1]),
            right_surface_to_screen_h2: pack_homography_row(right_surface_to_screen[2]),
            color_matrix_r0: [1.0, 0.0, 0.0, 0.0],
            color_matrix_r1: [0.0, 1.0, 0.0, 0.0],
            color_matrix_r2: [0.0, 0.0, 1.0, 0.0],
            color_offset: [0.0, 0.0, 0.0, 0.0],
            left_source_uv_rect: full_source_uv_rect_xywh(),
            right_source_uv_rect: full_source_uv_rect_xywh(),
            left_canvas_clip0: [-1.0, -1.0, 0.0, 1.0],
            left_canvas_clip1: [1.0, -1.0, 0.0, 1.0],
            left_canvas_clip2: [1.0, 1.0, 0.0, 1.0],
            left_canvas_clip3: [-1.0, 1.0, 0.0, 1.0],
            right_canvas_clip0: [-1.0, -1.0, 0.0, 1.0],
            right_canvas_clip1: [1.0, -1.0, 0.0, 1.0],
            right_canvas_clip2: [1.0, 1.0, 0.0, 1.0],
            right_canvas_clip3: [-1.0, 1.0, 0.0, 1.0],
        }
    }

    pub(super) fn with_color_config(mut self, config: &crate::RuntimeConfig) -> Self {
        self.color_matrix_r0 = [
            config.camera_color_matrix[0][0],
            config.camera_color_matrix[0][1],
            config.camera_color_matrix[0][2],
            0.0,
        ];
        self.color_matrix_r1 = [
            config.camera_color_matrix[1][0],
            config.camera_color_matrix[1][1],
            config.camera_color_matrix[1][2],
            0.0,
        ];
        self.color_matrix_r2 = [
            config.camera_color_matrix[2][0],
            config.camera_color_matrix[2][1],
            config.camera_color_matrix[2][2],
            0.0,
        ];
        self.color_offset = [
            config.camera_color_offset[0],
            config.camera_color_offset[1],
            config.camera_color_offset[2],
            0.0,
        ];
        self
    }

    pub(super) fn with_source_uv_rects(mut self, left: [f32; 4], right: [f32; 4]) -> Self {
        self.left_source_uv_rect = left;
        self.right_source_uv_rect = right;
        self
    }
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
                        source_uv_rect_xywh_for_frame(&frame.left),
                        source_uv_rect_xywh_for_frame(&frame.right),
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
                    source_uv_rect_xywh_for_frame(&frame.left),
                    source_uv_rect_xywh_for_frame(&frame.right),
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
                    source_uv_rect_xywh_for_frame(&frame.left),
                    source_uv_rect_xywh_for_frame(&frame.right),
                )
                .with_color_config(config),
        )
    }
}

fn full_source_uv_rect_xywh() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

pub(super) fn source_uv_rect_xywh_for_frame(frame: &HeadsetCameraGpuFrame) -> [f32; 4] {
    let [left, top, right, bottom] = source_uv_rect_ltrb_for_diagnostics(&frame.diagnostics);
    [left, top, (right - left).max(0.0), (bottom - top).max(0.0)]
}
