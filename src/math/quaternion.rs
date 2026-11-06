use core::ops::Mul;
use super::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub const fn identity() -> Self {
        Self::IDENTITY
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.z.is_finite()
            && self.w.is_finite()
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x
            + self.y * rhs.y
            + self.z * rhs.z
            + self.w * rhs.w
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

        Some(Self::new(
            self.x / len,
            self.y / len,
            self.z / len,
            self.w / len,
        ))
    }

    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    // Angle is in radians.
    pub fn from_axis_angle(
        axis: Vec3,
        angle: f32,
    ) -> Option<Self> {
        if !angle.is_finite() {
            return None;
        }

        let axis = axis.normalized()?;
        let half = angle * 0.5;
        let (s, c) = half.sin_cos();

        let result = Self::new(
            axis.x * s,
            axis.y * s,
            axis.z * s,
            c,
        );

        if result.is_finite() {
            Some(result)
        } else {
            None
        }
    }

    // Rotates a vector using a normalized quaternion.
    pub fn rotate_vector(self, v: Vec3) -> Option<Vec3> {
        if !v.is_finite() {
            return None;
        }

        let q = self.normalized()?;

        let u = Vec3::new(q.x, q.y, q.z);
        let t = u.cross(v) * 2.0;

        let result = v
            + t * q.w
            + u.cross(t);

        if result.is_finite() {
            Some(result)
        } else {
            None
        }
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::identity()
    }
}

// Hamilton product.
// q1 * q2 applies q2 first, then q1.
impl Mul for Quaternion {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.w * rhs.x
                + self.x * rhs.w
                + self.y * rhs.z
                - self.z * rhs.y,

            self.w * rhs.y
                - self.x * rhs.z
                + self.y * rhs.w
                + self.z * rhs.x,

            self.w * rhs.z
                + self.x * rhs.y
                - self.y * rhs.x
                + self.z * rhs.w,

            self.w * rhs.w
                - self.x * rhs.x
                - self.y * rhs.y
                - self.z * rhs.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::FRAC_PI_2;

    #[test]
    fn identity_preserves_vector() {
        let v = Vec3::new(1.0, 2.0, 3.0);

        assert_eq!(
            Quaternion::identity().rotate_vector(v),
            Some(v)
        );
    }

    #[test]
    fn rotation_z_90_degrees() {
        let q = Quaternion::from_axis_angle(
            Vec3::new(0.0, 0.0, 1.0),
            FRAC_PI_2,
        ).unwrap();

        let v = q.rotate_vector(
            Vec3::new(1.0, 0.0, 0.0)
        ).unwrap();

        assert!(v.x.abs() < 0.00001);
        assert!((v.y - 1.0).abs() < 0.00001);
        assert!(v.z.abs() < 0.00001);
    }

    #[test]
    fn two_rotations_combine() {
        let q = Quaternion::from_axis_angle(
            Vec3::new(0.0, 0.0, 1.0),
            FRAC_PI_2,
        ).unwrap();

        let combined = q * q;

        let v = combined.rotate_vector(
            Vec3::new(1.0, 0.0, 0.0)
        ).unwrap();

        assert!((v.x + 1.0).abs() < 0.00001);
        assert!(v.y.abs() < 0.00001);
    }

    #[test]
    fn normalization_produces_unit_quaternion() {
        let q = Quaternion::new(2.0, 0.0, 0.0, 2.0);

        let normalized = q.normalized().unwrap();

        assert!((normalized.length() - 1.0).abs() < 0.00001);
    }

    #[test]
    fn zero_axis_is_rejected() {
        assert!(
            Quaternion::from_axis_angle(
                Vec3::ZERO,
                1.0
            ).is_none()
        );
    }

    #[test]
    fn zero_quaternion_is_rejected() {
        let q = Quaternion::new(0.0, 0.0, 0.0, 0.0);

        assert!(q.normalized().is_none());
        assert!(
            q.rotate_vector(Vec3::new(1.0, 0.0, 0.0)).is_none()
        );
    }

    #[test]
    fn conjugate_reverses_rotation() {
        let q = Quaternion::from_axis_angle(
            Vec3::new(0.0, 0.0, 1.0),
            FRAC_PI_2,
        ).unwrap();

        let v = Vec3::new(1.0, 0.0, 0.0);

        let rotated = q.rotate_vector(v).unwrap();
        let restored = q.conjugate()
            .rotate_vector(rotated).unwrap();

        assert!((restored.x - 1.0).abs() < 0.00001);
        assert!(restored.y.abs() < 0.00001);
    }
}
