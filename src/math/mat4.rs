use core::ops::Mul;
use super::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    // Column-major memory layout.
    pub elements: [f32; 16],
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        elements: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ],
    };

    pub const fn identity() -> Self {
        Self::IDENTITY
    }

    pub fn get(&self, row: usize, col: usize) -> f32 {
        self.elements[col * 4 + row]
    }

    pub fn from_translation(v: Vec3) -> Self {
        Self {
            elements: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                v.x, v.y, v.z, 1.0,
            ],
        }
    }

    pub fn from_scale(v: Vec3) -> Self {
        Self {
            elements: [
                v.x, 0.0, 0.0, 0.0,
                0.0, v.y, 0.0, 0.0,
                0.0, 0.0, v.z, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();

        Self {
            elements: [
                1.0, 0.0, 0.0, 0.0,
                0.0, c, s, 0.0,
                0.0, -s, c, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();

        Self {
            elements: [
                c, 0.0, -s, 0.0,
                0.0, 1.0, 0.0, 0.0,
                s, 0.0, c, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();

        Self {
            elements: [
                c, s, 0.0, 0.0,
                -s, c, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    // Right-handed perspective.
    // Depth range: 0.0 to 1.0 (GPU-friendly).
    pub fn perspective_rh_zo(
        fov_y: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Option<Self> {
        use core::f32::consts::PI;

        if !fov_y.is_finite()
            || !aspect.is_finite()
            || !near.is_finite()
            || !far.is_finite()
            || fov_y <= 0.0
            || fov_y >= PI
            || aspect <= 0.0
            || near <= 0.0
            || far <= near
        {
            return None;
        }

        let f = 1.0 / (fov_y * 0.5).tan();

        Some(Self {
            elements: [
                f / aspect, 0.0, 0.0, 0.0,
                0.0, f, 0.0, 0.0,
                0.0, 0.0, far / (near - far), -1.0,
                0.0, 0.0, (far * near) / (near - far), 0.0,
            ],
        })
    }

    pub fn transform_point(self, p: Vec3) -> Option<Vec3> {
        if !p.is_finite() {
            return None;
        }

        let x =
            self.get(0, 0) * p.x
            + self.get(0, 1) * p.y
            + self.get(0, 2) * p.z
            + self.get(0, 3);

        let y =
            self.get(1, 0) * p.x
            + self.get(1, 1) * p.y
            + self.get(1, 2) * p.z
            + self.get(1, 3);

        let z =
            self.get(2, 0) * p.x
            + self.get(2, 1) * p.y
            + self.get(2, 2) * p.z
            + self.get(2, 3);

        let w =
            self.get(3, 0) * p.x
            + self.get(3, 1) * p.y
            + self.get(3, 2) * p.z
            + self.get(3, 3);

        if !w.is_finite() || w.abs() <= f32::EPSILON {
            return None;
        }

        let result = Vec3::new(x / w, y / w, z / w);

        if result.is_finite() {
            Some(result)
        } else {
            None
        }
    }

    pub fn transform_direction(self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.get(0, 0) * v.x
                + self.get(0, 1) * v.y
                + self.get(0, 2) * v.z,

            self.get(1, 0) * v.x
                + self.get(1, 1) * v.y
                + self.get(1, 2) * v.z,

            self.get(2, 0) * v.x
                + self.get(2, 1) * v.y
                + self.get(2, 2) * v.z,
        )
    }

    pub fn transpose(self) -> Self {
        let mut result = [0.0; 16];

        for row in 0..4 {
            for col in 0..4 {
                result[row * 4 + col] = self.get(row, col);
            }
        }

        Self { elements: result }
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Mul for Mat4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let mut result = [0.0; 16];

        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;

                for k in 0..4 {
                    sum += self.get(row, k) * rhs.get(k, col);
                }

                result[col * 4 + row] = sum;
            }
        }

        Self { elements: result }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::FRAC_PI_2;

    #[test]
    fn identity_preserves_point() {
        let p = Vec3::new(2.0, 3.0, 4.0);

        assert_eq!(
            Mat4::identity().transform_point(p),
            Some(p)
        );
    }

    #[test]
    fn translation_moves_point() {
        let m = Mat4::from_translation(
            Vec3::new(10.0, 20.0, 30.0)
        );

        assert_eq!(
            m.transform_point(Vec3::new(1.0, 2.0, 3.0)),
            Some(Vec3::new(11.0, 22.0, 33.0))
        );
    }

    #[test]
    fn scaling_changes_size() {
        let m = Mat4::from_scale(
            Vec3::new(2.0, 3.0, 4.0)
        );

        assert_eq!(
            m.transform_point(Vec3::new(1.0, 1.0, 1.0)),
            Some(Vec3::new(2.0, 3.0, 4.0))
        );
    }

    #[test]
    fn rotation_z_90_degrees() {
        let m = Mat4::rotation_z(FRAC_PI_2);

        let p = m.transform_point(
            Vec3::new(1.0, 0.0, 0.0)
        ).unwrap();

        assert!(p.x.abs() < 0.00001);
        assert!((p.y - 1.0).abs() < 0.00001);
    }

    #[test]
    fn matrix_composition() {
        let translation = Mat4::from_translation(
            Vec3::new(10.0, 20.0, 30.0)
        );

        let scale = Mat4::from_scale(
            Vec3::new(2.0, 3.0, 4.0)
        );

        let combined = translation * scale;

        assert_eq!(
            combined.transform_point(
                Vec3::new(1.0, 1.0, 1.0)
            ),
            Some(Vec3::new(12.0, 23.0, 34.0))
        );
    }

    #[test]
    fn perspective_matrix_is_valid() {
        let m = Mat4::perspective_rh_zo(
            FRAC_PI_2,
            2.0,
            0.1,
            100.0,
        ).unwrap();

        assert!((m.get(0, 0) - 0.5).abs() < 0.00001);
        assert_eq!(m.get(3, 2), -1.0);
    }

    #[test]
    fn perspective_rejects_invalid_values() {
        assert!(
            Mat4::perspective_rh_zo(
                0.0, 1.0, 0.1, 100.0
            ).is_none()
        );

        assert!(
            Mat4::perspective_rh_zo(
                1.0, 0.0, 0.1, 100.0
            ).is_none()
        );
    }
}
