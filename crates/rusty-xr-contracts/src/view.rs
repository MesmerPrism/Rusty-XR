use crate::{ImageSize, Pose};

/// Logical eye for per-view metadata.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eye {
    Mono,
    Left,
    Right,
}

/// OpenXR-style tangent-angle field of view in radians.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FieldOfView {
    pub angle_left_radians: f32,
    pub angle_right_radians: f32,
    pub angle_up_radians: f32,
    pub angle_down_radians: f32,
}

impl FieldOfView {
    pub const fn new(
        angle_left_radians: f32,
        angle_right_radians: f32,
        angle_up_radians: f32,
        angle_down_radians: f32,
    ) -> Self {
        Self {
            angle_left_radians,
            angle_right_radians,
            angle_up_radians,
            angle_down_radians,
        }
    }

    pub fn is_finite(self) -> bool {
        self.angle_left_radians.is_finite()
            && self.angle_right_radians.is_finite()
            && self.angle_up_radians.is_finite()
            && self.angle_down_radians.is_finite()
    }
}

/// Pose, projection, and target dimensions for a single rendered eye.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EyeView {
    pub eye: Eye,
    pub pose: Pose,
    pub fov: FieldOfView,
    pub recommended_image_size: Option<ImageSize>,
}

impl EyeView {
    pub const fn new(eye: Eye, pose: Pose, fov: FieldOfView) -> Self {
        Self {
            eye,
            pose,
            fov,
            recommended_image_size: None,
        }
    }

    pub const fn with_recommended_image_size(mut self, size: ImageSize) -> Self {
        self.recommended_image_size = Some(size);
        self
    }

    pub fn is_valid(self) -> bool {
        self.pose.is_finite()
            && self.fov.is_finite()
            && self
                .recommended_image_size
                .map(ImageSize::is_non_empty)
                .unwrap_or(true)
    }
}

/// Paired left/right eye views for stereo render or camera alignment.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StereoViews {
    pub left: EyeView,
    pub right: EyeView,
}

impl StereoViews {
    pub const fn new(left: EyeView, right: EyeView) -> Self {
        Self { left, right }
    }

    pub fn is_valid(self) -> bool {
        self.left.eye == Eye::Left
            && self.right.eye == Eye::Right
            && self.left.is_valid()
            && self.right.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Quat, Vec3};

    #[test]
    fn stereo_views_require_left_and_right_eyes() {
        let fov = FieldOfView::new(-0.7, 0.7, 0.7, -0.7);
        let pose = Pose::new(Vec3::ZERO, Quat::IDENTITY);
        let left = EyeView::new(Eye::Left, pose, fov);
        let right = EyeView::new(Eye::Right, pose, fov);
        let mono = EyeView::new(Eye::Mono, pose, fov);

        assert!(StereoViews::new(left, right).is_valid());
        assert!(!StereoViews::new(mono, right).is_valid());
    }
}
