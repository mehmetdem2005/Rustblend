use crate::math::Vec3;

// İki köşeyi birbirine bağlayan kenar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    vertices: [usize; 2],
}

impl Edge {
    pub const fn new(a: usize, b: usize) -> Self {
        Self {
            vertices: [a, b],
        }
    }

    pub const fn vertices(&self) -> [usize; 2] {
        self.vertices
    }
}

// Bir poligonu oluşturan köşeler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    vertices: Vec<usize>,
}

impl Face {
    pub fn new(vertices: Vec<usize>) -> Self {
        Self { vertices }
    }

    pub fn vertices(&self) -> &[usize] {
        &self.vertices
    }
}

// Geometri hataları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshError {
    InvalidCubeSize,
    NonFiniteVertex {
        index: usize,
    },
    EdgeHasSameVertex {
        edge: usize,
    },
    EdgeVertexOutOfBounds {
        edge: usize,
        vertex: usize,
    },
    FaceTooSmall {
        face: usize,
    },
    FaceVertexOutOfBounds {
        face: usize,
        vertex: usize,
    },
    FaceHasRepeatedVertex {
        face: usize,
        vertex: usize,
    },
}

// Ana 3B model yapısı.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    vertices: Vec<Vec3>,
    edges: Vec<Edge>,
    faces: Vec<Face>,
}

impl Mesh {
    // Verilen geometrinin geçerliliğini kontrol eder.
    pub fn new(
        vertices: Vec<Vec3>,
        edges: Vec<Edge>,
        faces: Vec<Face>,
    ) -> Result<Self, MeshError> {

        // Köşe kontrolü.
        for (index, vertex) in vertices.iter().enumerate() {
            if !vertex.is_finite() {
                return Err(MeshError::NonFiniteVertex {
                    index,
                });
            }
        }

        // Kenar kontrolü.
        for (edge_index, edge) in edges.iter().enumerate() {
            let [a, b] = edge.vertices;

            if a == b {
                return Err(MeshError::EdgeHasSameVertex {
                    edge: edge_index,
                });
            }

            for vertex_index in [a, b] {
                if vertex_index >= vertices.len() {
                    return Err(
                        MeshError::EdgeVertexOutOfBounds {
                            edge: edge_index,
                            vertex: vertex_index,
                        }
                    );
                }
            }
        }

        // Yüzey kontrolü.
        for (face_index, face) in faces.iter().enumerate() {
            if face.vertices.len() < 3 {
                return Err(MeshError::FaceTooSmall {
                    face: face_index,
                });
            }

            for (corner, &vertex_index)
                in face.vertices.iter().enumerate()
            {
                if vertex_index >= vertices.len() {
                    return Err(
                        MeshError::FaceVertexOutOfBounds {
                            face: face_index,
                            vertex: vertex_index,
                        }
                    );
                }

                if face.vertices[..corner].contains(&vertex_index) {
                    return Err(
                        MeshError::FaceHasRepeatedVertex {
                            face: face_index,
                            vertex: vertex_index,
                        }
                    );
                }
            }
        }

        Ok(Self {
            vertices,
            edges,
            faces,
        })
    }

    // Sıfırdan küp oluşturur.
    // size, küpün bir kenarının uzunluğudur.
    pub fn cube(size: f32) -> Result<Self, MeshError> {
        if !size.is_finite() || size <= 0.0 {
            return Err(MeshError::InvalidCubeSize);
        }

        let h = size * 0.5;

        // 8 köşe.
        let vertices = vec![
            Vec3::new(-h, -h, -h),
            Vec3::new( h, -h, -h),
            Vec3::new( h,  h, -h),
            Vec3::new(-h,  h, -h),

            Vec3::new(-h, -h,  h),
            Vec3::new( h, -h,  h),
            Vec3::new( h,  h,  h),
            Vec3::new(-h,  h,  h),
        ];

        // 12 kenar.
        let edges = vec![
            Edge::new(0, 1),
            Edge::new(1, 2),
            Edge::new(2, 3),
            Edge::new(3, 0),

            Edge::new(4, 5),
            Edge::new(5, 6),
            Edge::new(6, 7),
            Edge::new(7, 4),

            Edge::new(0, 4),
            Edge::new(1, 5),
            Edge::new(2, 6),
            Edge::new(3, 7),
        ];

        // 6 yüzey.
        // Her yüzeyin köşeleri dışa bakan
        // sırayla tanımlanmıştır.
        let faces = vec![
            Face::new(vec![0, 3, 2, 1]),
            Face::new(vec![4, 5, 6, 7]),
            Face::new(vec![0, 1, 5, 4]),
            Face::new(vec![1, 2, 6, 5]),
            Face::new(vec![2, 3, 7, 6]),
            Face::new(vec![3, 0, 4, 7]),
        ];

        Self::new(vertices, edges, faces)
    }

    // Köşeleri dışarıdan salt okunur olarak sunar.
    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }

    // Kenarları sunar.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    // Yüzeyleri sunar.
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_has_correct_topology() {
        let mesh = Mesh::cube(2.0).unwrap();

        assert_eq!(mesh.vertex_count(), 8);
        assert_eq!(mesh.edge_count(), 12);
        assert_eq!(mesh.face_count(), 6);
    }

    #[test]
    fn cube_faces_have_four_vertices() {
        let mesh = Mesh::cube(2.0).unwrap();

        for face in mesh.faces() {
            assert_eq!(face.vertices().len(), 4);

            for &index in face.vertices() {
                assert!(index < mesh.vertex_count());
            }
        }
    }

    #[test]
    fn cube_has_correct_dimensions() {
        let mesh = Mesh::cube(2.0).unwrap();

        assert_eq!(
            mesh.vertices()[0],
            Vec3::new(-1.0, -1.0, -1.0)
        );

        assert_eq!(
            mesh.vertices()[6],
            Vec3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn invalid_cube_size_is_rejected() {
        assert_eq!(
            Mesh::cube(0.0),
            Err(MeshError::InvalidCubeSize)
        );

        assert_eq!(
            Mesh::cube(-2.0),
            Err(MeshError::InvalidCubeSize)
        );

        assert_eq!(
            Mesh::cube(f32::NAN),
            Err(MeshError::InvalidCubeSize)
        );
    }

    #[test]
    fn non_finite_vertex_is_rejected() {
        let result = Mesh::new(
            vec![Vec3::new(f32::NAN, 0.0, 0.0)],
            vec![],
            vec![],
        );

        assert_eq!(
            result,
            Err(MeshError::NonFiniteVertex { index: 0 })
        );
    }

    #[test]
    fn edge_with_same_vertex_is_rejected() {
        let result = Mesh::new(
            vec![Vec3::ZERO],
            vec![Edge::new(0, 0)],
            vec![],
        );

        assert_eq!(
            result,
            Err(MeshError::EdgeHasSameVertex { edge: 0 })
        );
    }

    #[test]
    fn invalid_edge_index_is_rejected() {
        let result = Mesh::new(
            vec![Vec3::ZERO],
            vec![Edge::new(0, 1)],
            vec![],
        );

        assert_eq!(
            result,
            Err(
                MeshError::EdgeVertexOutOfBounds {
                    edge: 0,
                    vertex: 1,
                }
            )
        );
    }

    #[test]
    fn face_with_too_few_vertices_is_rejected() {
        let result = Mesh::new(
            vec![Vec3::ZERO; 2],
            vec![],
            vec![Face::new(vec![0, 1])],
        );

        assert_eq!(
            result,
            Err(MeshError::FaceTooSmall { face: 0 })
        );
    }

    #[test]
    fn face_with_invalid_index_is_rejected() {
        let result = Mesh::new(
            vec![Vec3::ZERO; 3],
            vec![],
            vec![Face::new(vec![0, 1, 3])],
        );

        assert_eq!(
            result,
            Err(
                MeshError::FaceVertexOutOfBounds {
                    face: 0,
                    vertex: 3,
                }
            )
        );
    }

    #[test]
    fn face_with_repeated_vertex_is_rejected() {
        let result = Mesh::new(
            vec![Vec3::ZERO; 3],
            vec![],
            vec![Face::new(vec![0, 1, 1])],
        );

        assert_eq!(
            result,
            Err(
                MeshError::FaceHasRepeatedVertex {
                    face: 0,
                    vertex: 1,
                }
            )
        );
    }
}
