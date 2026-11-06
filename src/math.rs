pub mod mat4;
pub use mat4::Mat4;
pub mod quaternion;
pub use quaternion::Quaternion;
use core::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x
            + self.y * rhs.y
            + self.z * rhs.z
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalized(self) -> Option<Self> {
        let len = self.length();

        if !len.is_finite() || len <= f32::EPSILON {
            return None;
        }

        Some(self / len)
    }

    pub fn distance(self, rhs: Self) -> f32 {
        (self - rhs).length()
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.z.is_finite()
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
        )
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self::new(
            self.x - rhs.x,
            self.y - rhs.y,
            self.z - rhs.z,
        )
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self::new(
            self.x * rhs,
            self.y * rhs,
            self.z * rhs,
        )
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self {
        Self::new(
            self.x / rhs,
            self.y / rhs,
            self.z / rhs,
        )
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_addition() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);

        assert_eq!(
            a + b,
            Vec3::new(5.0, 7.0, 9.0)
        );
    }

    #[test]
    fn test_dot_product() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);

        assert_eq!(a.dot(b), 32.0);
    }

    #[test]
    fn test_cross_product() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);

        assert_eq!(
            a.cross(b),
            Vec3::new(0.0, 0.0, 1.0)
        );
    }

    #[test]
    fn test_normalization() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let n = v.normalized().unwrap();

        assert!((n.length() - 1.0).abs() < 0.000001);
    }

    #[test]
    fn test_zero_vector() {
        assert!(Vec3::ZERO.normalized().is_none());
    }
}
