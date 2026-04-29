use core::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

/// Two-dimensional float vector used for pixel-domain values.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn splat(value: f32) -> Self {
        Self::new(value, value)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from(value: [f32; 2]) -> Self {
        Self::new(value[0], value[1])
    }
}

impl From<Vec2> for [f32; 2] {
    fn from(value: Vec2) -> Self {
        [value.x, value.y]
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

/// Three-dimensional float vector used at public app/experiment boundaries.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const RIGHT: Self = Self::new(1.0, 0.0, 0.0);
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);
    pub const FORWARD_NEG_Z: Self = Self::new(0.0, 0.0, -1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn splat(value: f32) -> Self {
        Self::new(value, value, value)
    }

    pub fn dot(self, other: Self) -> f32 {
        (self.x * other.x) + (self.y * other.y) + (self.z * other.z)
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            (self.y * other.z) - (self.z * other.y),
            (self.z * other.x) - (self.x * other.z),
            (self.x * other.y) - (self.y * other.x),
        )
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalized_or(self, fallback: Self) -> Self {
        let len_sq = self.length_squared();
        if len_sq <= 1.0e-12 || !len_sq.is_finite() {
            fallback
        } else {
            self / len_sq.sqrt()
        }
    }

    pub fn clamped_length(self, max_length: f32) -> Self {
        let max_length = max_length.max(0.0);
        if max_length <= 1.0e-6 {
            return Self::ZERO;
        }

        let len_sq = self.length_squared();
        let max_sq = max_length * max_length;
        if len_sq > max_sq {
            self.normalized_or(Self::ZERO) * max_length
        } else {
            self
        }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self::new(
            self.x.clamp(min.x, max.x),
            self.y.clamp(min.y, max.y),
            self.z.clamp(min.z, max.z),
        )
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(value: [f32; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(value: Vec3) -> Self {
        [value.x, value.y, value.z]
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

/// Quaternion stored as vector part plus scalar `w`.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub fn from_axis_angle(axis: Vec3, radians: f32) -> Self {
        let axis = axis.normalized_or(Vec3::UP);
        let half = radians * 0.5;
        let (sin_half, cos_half) = half.sin_cos();
        Self::new(
            axis.x * sin_half,
            axis.y * sin_half,
            axis.z * sin_half,
            cos_half,
        )
        .normalized_or(Self::IDENTITY)
    }

    pub fn length_squared(self) -> f32 {
        (self.x * self.x) + (self.y * self.y) + (self.z * self.z) + (self.w * self.w)
    }

    pub fn normalized_or(self, fallback: Self) -> Self {
        let len_sq = self.length_squared();
        if len_sq <= 1.0e-12 || !len_sq.is_finite() {
            fallback
        } else {
            self.scale(1.0 / len_sq.sqrt())
        }
    }

    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    pub fn inverse(self) -> Self {
        let len_sq = self.length_squared();
        if len_sq <= 1.0e-12 || !len_sq.is_finite() {
            Self::IDENTITY
        } else {
            self.conjugate().scale(1.0 / len_sq)
        }
    }

    pub fn rotate_vec3(self, value: Vec3) -> Vec3 {
        let q = self.normalized_or(Self::IDENTITY);
        let qv = Vec3::new(q.x, q.y, q.z);
        let t = qv.cross(value) * 2.0;
        value + (t * q.w) + qv.cross(t)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    fn scale(self, value: f32) -> Self {
        Self::new(
            self.x * value,
            self.y * value,
            self.z * value,
            self.w * value,
        )
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul for Quat {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            (self.w * rhs.x) + (self.x * rhs.w) + (self.y * rhs.z) - (self.z * rhs.y),
            (self.w * rhs.y) - (self.x * rhs.z) + (self.y * rhs.w) + (self.z * rhs.x),
            (self.w * rhs.z) + (self.x * rhs.y) - (self.y * rhs.x) + (self.z * rhs.w),
            (self.w * rhs.w) - (self.x * rhs.x) - (self.y * rhs.y) - (self.z * rhs.z),
        )
    }
}

/// Rigid pose in a caller-defined coordinate space.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub position: Vec3,
    pub orientation: Quat,
}

impl Pose {
    pub const IDENTITY: Self = Self::new(Vec3::ZERO, Quat::IDENTITY);

    pub const fn new(position: Vec3, orientation: Quat) -> Self {
        Self {
            position,
            orientation,
        }
    }

    pub fn transform_point(self, point: Vec3) -> Vec3 {
        self.position + self.orientation.rotate_vec3(point)
    }

    pub fn inverse_transform_point(self, point: Vec3) -> Vec3 {
        self.orientation
            .inverse()
            .rotate_vec3(point - self.position)
    }

    pub fn is_finite(self) -> bool {
        self.position.is_finite() && self.orientation.is_finite()
    }
}

impl Default for Pose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_cross_product_uses_right_handed_basis() {
        assert_eq!(Vec3::RIGHT.cross(Vec3::UP), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn quaternion_rotates_vector() {
        let q = Quat::from_axis_angle(Vec3::UP, core::f32::consts::FRAC_PI_2);
        let rotated = q.rotate_vec3(Vec3::RIGHT);

        assert!((rotated.x).abs() < 1.0e-5);
        assert!((rotated.z + 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn pose_round_trips_point() {
        let pose = Pose::new(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_axis_angle(Vec3::UP, 0.25),
        );
        let local = Vec3::new(0.25, -0.5, 2.0);
        let world = pose.transform_point(local);

        let round_trip = pose.inverse_transform_point(world);

        assert!((round_trip.x - local.x).abs() < 1.0e-5);
        assert!((round_trip.y - local.y).abs() < 1.0e-5);
        assert!((round_trip.z - local.z).abs() < 1.0e-5);
    }
}
