use crate::math::Vec3;
use super::mesh::Mesh;

// 3B sınır kutusu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    pub fn new(min: Vec3, max: Vec3) -> Option<Self> {
        if !min.is_finite() || !max.is_finite() {
            return None;
        }

        if min.x > max.x
            || min.y > max.y
            || min.z > max.z
        {
            return None;
        }

        Some(Self { min, max })
    }

    pub fn min(&self) -> Vec3 {
        self.min
    }

    pub fn max(&self) -> Vec3 {
        self.max
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn contains(&self, point: Vec3) -> bool {
        if !point.is_finite() {
            return false;
        }

        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }
}

impl Mesh {
    // Modelin kapladığı alanı hesaplar.
    pub fn bounds(&self) -> Option<Bounds> {
        let mut iter = self.vertices().iter();

        let first = *iter.next()?;

        let mut min = first;
        let mut max = first;

        for &vertex in iter {
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);

            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
        }

        Some(Bounds { min, max })
    }

    // Newell yöntemiyle yüzeyin normalini hesaplar.
    pub fn face_normal(&self, face_index: usize) -> Option<Vec3> {
        let face = self.faces().get(face_index)?;
        let indices = face.vertices();

        if indices.len() < 3 {
            return None;
        }

        let mut normal = Vec3::ZERO;

        for i in 0..indices.len() {
            let current = *self.vertices().get(indices[i])?;

            let next_index = indices[(i + 1) % indices.len()];

            let next = *self.vertices().get(next_index)?;

            normal = normal + Vec3::new(
                (current.y - next.y) * (current.z + next.z),
                (current.z - next.z) * (current.x + next.x),
                (current.x - next.x) * (current.y + next.y),
            );
        }

        normal.normalized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::mesh::Face;

    #[test]
    fn cube_bounds_dimensions() {
        let mesh = Mesh::cube(2.0).unwrap();
        let bounds = mesh.bounds().unwrap();

        assert_eq!(
            bounds.min(),
            Vec3::new(-1.0, -1.0, -1.0)
        );

        assert_eq!(
            bounds.max(),
            Vec3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn bounds_center_and_size() {
        let bounds = Bounds::new(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ).unwrap();

        assert_eq!(bounds.center(), Vec3::ZERO);

        assert_eq!(
            bounds.size(),
            Vec3::new(2.0, 2.0, 2.0)
        );
    }

    #[test]
    fn bounds_contains_points() {
        let bounds = Bounds::new(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ).unwrap();

        assert!(bounds.contains(Vec3::ZERO));

        assert!(bounds.contains(
            Vec3::new(1.0, 1.0, 1.0)
        ));

        assert!(!bounds.contains(
            Vec3::new(2.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn cube_face_normals_point_outward() {
        let mesh = Mesh::cube(2.0).unwrap();

        let expected = [
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
        ];

        for (i, expected_normal) in expected.iter().enumerate() {
            let normal = mesh.face_normal(i).unwrap();

            assert!(
                (normal - *expected_normal).length() < 0.00001
            );
        }
    }

    #[test]
    fn invalid_face_index_returns_none() {
        let mesh = Mesh::cube(2.0).unwrap();

        assert!(mesh.face_normal(99).is_none());
    }

    #[test]
    fn degenerate_face_has_no_normal() {
        let mesh = Mesh::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ],
            vec![],
            vec![Face::new(vec![0, 1, 2])],
        ).unwrap();

        assert!(mesh.face_normal(0).is_none());
    }

    #[test]
    fn empty_mesh_has_no_bounds() {
        let mesh = Mesh::new(
            vec![],
            vec![],
            vec![],
        ).unwrap();

        assert!(mesh.bounds().is_none());
    }
}
