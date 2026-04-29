use crate::{HandJointName, Handedness, ImageSize, Pose, Vec2, Vec3};

/// Ray in world or app-defined coordinates for pointing, gaze, controller, or hand input.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionRay {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl InteractionRay {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalized_or(Vec3::FORWARD_NEG_Z),
        }
    }

    pub fn point_at(self, distance_meters: f32) -> Vec3 {
        self.origin + (self.direction * distance_meters)
    }

    pub fn is_valid(self) -> bool {
        self.origin.is_finite()
            && self.direction.is_finite()
            && self.direction.length_squared() > 1.0e-8
    }
}

/// A world-anchored 2D canvas plane for XR menus, panels, and debug surfaces.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XrCanvasSurface {
    pub pose: Pose,
    pub size_meters: Vec2,
    pub pixel_size: Option<ImageSize>,
}

impl XrCanvasSurface {
    pub const fn new(pose: Pose, size_meters: Vec2) -> Self {
        Self {
            pose,
            size_meters,
            pixel_size: None,
        }
    }

    pub const fn with_pixel_size(mut self, pixel_size: ImageSize) -> Self {
        self.pixel_size = Some(pixel_size);
        self
    }

    pub fn is_valid(self) -> bool {
        self.pose.is_finite()
            && self.size_meters.is_finite()
            && self.size_meters.x > 0.0
            && self.size_meters.y > 0.0
            && self.pixel_size.map(ImageSize::is_non_empty).unwrap_or(true)
    }

    /// Intersects a ray with the canvas plane.
    ///
    /// The local canvas lies on `z = 0`, is centered on the pose, and uses
    /// normalized UV coordinates where `(0, 0)` is the upper-left corner.
    pub fn hit_test(self, ray: InteractionRay) -> Option<XrCanvasHit> {
        if !self.is_valid() || !ray.is_valid() {
            return None;
        }

        let local_origin = self.pose.inverse_transform_point(ray.origin);
        let local_direction = self.pose.orientation.inverse().rotate_vec3(ray.direction);
        if local_direction.z.abs() <= 1.0e-6 {
            return None;
        }

        let distance = -local_origin.z / local_direction.z;
        if distance < 0.0 || !distance.is_finite() {
            return None;
        }

        let local = local_origin + (local_direction * distance);
        let half_size = self.size_meters * 0.5;
        if local.x < -half_size.x
            || local.x > half_size.x
            || local.y < -half_size.y
            || local.y > half_size.y
        {
            return None;
        }

        let uv = Vec2::new(
            (local.x + half_size.x) / self.size_meters.x,
            1.0 - ((local.y + half_size.y) / self.size_meters.y),
        );
        let pixel = self
            .pixel_size
            .map(|size| Vec2::new(uv.x * size.width as f32, uv.y * size.height as f32));

        Some(XrCanvasHit {
            distance_meters: distance,
            world_position: ray.point_at(distance),
            local_position: local,
            uv,
            pixel,
        })
    }
}

/// Result of a ray hit against an XR canvas.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XrCanvasHit {
    pub distance_meters: f32,
    pub world_position: Vec3,
    pub local_position: Vec3,
    pub uv: Vec2,
    pub pixel: Option<Vec2>,
}

/// Public hand-menu anchor description.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandMenuAnchor {
    pub handedness: Handedness,
    pub anchor_joint: HandJointName,
    pub pose: Pose,
    pub size_meters: Vec2,
    pub activation: HandMenuActivation,
    pub visible: bool,
}

impl HandMenuAnchor {
    pub const fn new(handedness: Handedness, pose: Pose, size_meters: Vec2) -> Self {
        Self {
            handedness,
            anchor_joint: HandJointName::Palm,
            pose,
            size_meters,
            activation: HandMenuActivation::Manual,
            visible: true,
        }
    }

    pub const fn canvas_surface(self) -> XrCanvasSurface {
        XrCanvasSurface::new(self.pose, self.size_meters)
    }

    pub fn is_valid(self) -> bool {
        self.pose.is_finite()
            && self.size_meters.is_finite()
            && self.size_meters.x > 0.0
            && self.size_meters.y > 0.0
    }
}

/// Activation mode for a hand-attached menu.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandMenuActivation {
    Manual,
    PalmUp,
    PalmDown,
    Pinch,
    OpenPalm,
}

/// A general hand influence point for particles, deformation, or UI proximity.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandInfluencePoint {
    pub handedness: Handedness,
    pub source_joint: HandJointName,
    pub position: Vec3,
    pub radius_meters: f32,
    pub gain: f32,
}

impl HandInfluencePoint {
    pub const fn new(
        handedness: Handedness,
        source_joint: HandJointName,
        position: Vec3,
        radius_meters: f32,
        gain: f32,
    ) -> Self {
        Self {
            handedness,
            source_joint,
            position,
            radius_meters,
            gain,
        }
    }

    pub fn is_valid(self) -> bool {
        self.position.is_finite()
            && self.radius_meters.is_finite()
            && self.radius_meters >= 0.0
            && self.gain.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Quat, Vec3};

    #[test]
    fn canvas_hit_test_reports_uv_and_pixels() {
        let canvas = XrCanvasSurface::new(Pose::IDENTITY, Vec2::new(2.0, 1.0))
            .with_pixel_size(ImageSize::new(200, 100));
        let ray = InteractionRay::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = canvas.hit_test(ray).expect("ray should hit canvas");

        assert!((hit.distance_meters - 1.0).abs() < 1.0e-6);
        assert_eq!(hit.uv, Vec2::new(0.5, 0.5));
        assert_eq!(hit.pixel, Some(Vec2::new(100.0, 50.0)));
    }

    #[test]
    fn canvas_hit_test_rejects_outside_ray() {
        let canvas = XrCanvasSurface::new(Pose::IDENTITY, Vec2::new(1.0, 1.0));
        let ray = InteractionRay::new(Vec3::new(1.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0));

        assert!(canvas.hit_test(ray).is_none());
    }

    #[test]
    fn hand_menu_anchor_is_canvas_compatible() {
        let anchor = HandMenuAnchor::new(
            Handedness::Left,
            Pose::new(Vec3::new(0.1, 1.2, -0.4), Quat::IDENTITY),
            Vec2::new(0.22, 0.12),
        );

        assert!(anchor.is_valid());
        assert!(anchor.canvas_surface().is_valid());
    }
}
