use rusty_xr_camera_model::PerEyeVideoProjectionPlan;
use rusty_xr_contracts::Eye;

use super::openxr_gles_config::OesContentMappingMode;

#[derive(Clone, Debug)]
pub(super) struct OesProjectionPlan {
    pub(super) left: OesEyeProjection,
    pub(super) right: OesEyeProjection,
}

impl OesProjectionPlan {
    pub(super) fn eye(&self, view_index: usize) -> Option<&OesEyeProjection> {
        match view_index {
            0 => Some(&self.left),
            1 => Some(&self.right),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct OesEyeProjection {
    pub(super) eye: Eye,
    pub(super) surface_to_screen_h: [[f32; 3]; 3],
    pub(super) screen_to_surface_h: [[f32; 3]; 3],
    pub(super) surface_to_camera_h: [[f32; 3]; 3],
    pub(super) screen_to_camera_h: [[f32; 3]; 3],
    pub(super) source_label: String,
    pub(super) source_eye: String,
    pub(super) use_surface_texture_transform: bool,
    pub(super) content_mapping_mode: OesContentMappingMode,
    pub(super) geometry_plan: PerEyeVideoProjectionPlan,
}

impl OesEyeProjection {
    pub(super) fn source_transform_for_sample(
        &self,
        surface_texture_transform: [f32; 16],
    ) -> [f32; 16] {
        if self.use_surface_texture_transform {
            surface_texture_transform
        } else {
            identity_texture_transform()
        }
    }
}

pub(super) const fn identity_homography() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

const fn identity_texture_transform() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}
