use super::{
    projection_geometry::DisplayEyeProjectionMapping,
    projection_homography_utils::{identity_homography, pack_homography_row},
};

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

    pub(super) fn from_mappings(
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

fn full_source_uv_rect_xywh() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}
