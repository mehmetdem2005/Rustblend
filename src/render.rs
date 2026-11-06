use crate::math::Mat4;
use crate::scene::{ObjectId, Scene};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    DegenerateFace {
        object_id: ObjectId,
        face_index: usize,
    },
    TooManyVertices {
        object_id: ObjectId,
    },
}

#[derive(Debug, Clone)]
pub struct RenderObject {
    id: ObjectId,
    name: String,
    model_matrix: Mat4,
    selected: bool,
    vertices: Vec<RenderVertex>,
    indices: Vec<u32>,
}

impl RenderObject {
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model_matrix(&self) -> Mat4 {
        self.model_matrix
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn vertices(&self) -> &[RenderVertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderPacket {
    objects: Vec<RenderObject>,
}

impl RenderPacket {
    pub fn from_scene(scene: &Scene) -> Result<Self, RenderError> {
        let mut objects = Vec::with_capacity(scene.object_count());

        for object in scene.objects() {
            let mesh = object.mesh();

            let mut vertices = Vec::new();
            let mut indices = Vec::new();

            for (face_index, face) in mesh.faces().iter().enumerate() {
                let normal = mesh.face_normal(face_index)
                    .ok_or(RenderError::DegenerateFace {
                        object_id: object.id(),
                        face_index,
                    })?;

                let face_vertices = face.vertices();

                let base = u32::try_from(vertices.len())
                    .map_err(|_| RenderError::TooManyVertices {
                        object_id: object.id(),
                    })?;

                for &vertex_index in face_vertices {
                    let position = mesh.vertices()[vertex_index];

                    vertices.push(RenderVertex {
                        position: [
                            position.x,
                            position.y,
                            position.z,
                        ],
                        normal: [
                            normal.x,
                            normal.y,
                            normal.z,
                        ],
                    });
                }

                for corner in 1..(face_vertices.len() - 1) {
                    let b = base
                        .checked_add(
                            u32::try_from(corner)
                                .map_err(|_| RenderError::TooManyVertices {
                                    object_id: object.id(),
                                })?
                        )
                        .ok_or(RenderError::TooManyVertices {
                            object_id: object.id(),
                        })?;

                    let c = base
                        .checked_add(
                            u32::try_from(corner + 1)
                                .map_err(|_| RenderError::TooManyVertices {
                                    object_id: object.id(),
                                })?
                        )
                        .ok_or(RenderError::TooManyVertices {
                            object_id: object.id(),
                        })?;

                    indices.extend_from_slice(&[base, b, c]);
                }
            }

            objects.push(RenderObject {
                id: object.id(),
                name: object.name().to_owned(),
                model_matrix: object.model_matrix(),
                selected: scene.is_selected(object.id()),
                vertices,
                indices,
            });
        }

        Ok(Self { objects })
    }

    pub fn objects(&self) -> &[RenderObject] {
        &self.objects
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Face, Mesh};
    use crate::math::{Quaternion, Vec3};
    use crate::scene::{Scene, Transform};

    fn cube() -> Mesh {
        Mesh::cube(2.0).unwrap()
    }

    #[test]
    fn empty_scene_creates_empty_packet() {
        let scene = Scene::new();

        let packet = RenderPacket::from_scene(&scene).unwrap();

        assert!(packet.is_empty());
        assert_eq!(packet.object_count(), 0);
    }

    #[test]
    fn cube_generates_correct_render_geometry() {
        let mut scene = Scene::new();

        scene.add_object("Cube", cube()).unwrap();

        let packet = RenderPacket::from_scene(&scene).unwrap();
        let object = &packet.objects()[0];

        assert_eq!(object.vertex_count(), 24);
        assert_eq!(object.index_count(), 36);
        assert_eq!(object.triangle_count(), 12);
    }

    #[test]
    fn all_indices_are_valid() {
        let mut scene = Scene::new();

        scene.add_object("Cube", cube()).unwrap();

        let packet = RenderPacket::from_scene(&scene).unwrap();

        for object in packet.objects() {
            for &index in object.indices() {
                assert!((index as usize) < object.vertex_count());
            }
        }
    }

    #[test]
    fn cube_normals_are_flat_and_outward() {
        let mut scene = Scene::new();

        scene.add_object("Cube", cube()).unwrap();

        let packet = RenderPacket::from_scene(&scene).unwrap();
        let vertices = packet.objects()[0].vertices();

        for face in 0..6 {
            let start = face * 4;
            let face_vertices = &vertices[start..start + 4];

            let normal = face_vertices[0].normal;

            for vertex in face_vertices {
                assert_eq!(vertex.normal, normal);

                let length = (
                    normal[0] * normal[0]
                    + normal[1] * normal[1]
                    + normal[2] * normal[2]
                ).sqrt();

                assert!((length - 1.0).abs() < 0.00001);

                let outward = normal[0] * vertex.position[0]
                    + normal[1] * vertex.position[1]
                    + normal[2] * vertex.position[2];

                assert!(outward > 0.0);
            }
        }
    }

    #[test]
    fn render_object_preserves_transform() {
        let mut scene = Scene::new();

        let transform = Transform::new(
            Vec3::new(10.0, 20.0, 30.0),
            Quaternion::IDENTITY,
            Vec3::new(2.0, 3.0, 4.0),
        ).unwrap();

        scene.add_object_with_transform(
            "Cube",
            cube(),
            transform,
        ).unwrap();

        let packet = RenderPacket::from_scene(&scene).unwrap();

        assert_eq!(
            packet.objects()[0].model_matrix(),
            transform.model_matrix()
        );
    }

    #[test]
    fn render_object_preserves_selection() {
        let mut scene = Scene::new();

        let id = scene.add_object("Cube", cube()).unwrap();

        scene.select(id).unwrap();

        let packet = RenderPacket::from_scene(&scene).unwrap();

        assert!(packet.objects()[0].is_selected());
        assert_eq!(packet.objects()[0].id(), id);
    }

    #[test]
    fn multiple_objects_are_preserved() {
        let mut scene = Scene::new();

        scene.add_object("Cube", cube()).unwrap();
        scene.add_object("Second Cube", cube()).unwrap();

        let packet = RenderPacket::from_scene(&scene).unwrap();

        assert_eq!(packet.object_count(), 2);
        assert_eq!(packet.objects()[0].name(), "Cube");
        assert_eq!(packet.objects()[1].name(), "Second Cube");
    }

    #[test]
    fn degenerate_face_returns_error() {
        let mesh = Mesh::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ],
            vec![],
            vec![Face::new(vec![0, 1, 2])],
        ).unwrap();

        let mut scene = Scene::new();
        let id = scene.add_object("Bad Mesh", mesh).unwrap();

        // unwrap_err yalnızca hata değerini karşılaştırır.
        // RenderPacket için PartialEq gerekmez.
        let error = RenderPacket::from_scene(&scene).unwrap_err();

        assert_eq!(
            error,
            RenderError::DegenerateFace {
                object_id: id,
                face_index: 0,
            }
        );
    }

    #[test]
    fn empty_mesh_is_supported() {
        let mesh = Mesh::new(
            vec![],
            vec![],
            vec![],
        ).unwrap();

        let mut scene = Scene::new();

        scene.add_object("Empty", mesh).unwrap();

        let packet = RenderPacket::from_scene(&scene).unwrap();

        assert_eq!(packet.object_count(), 1);
        assert_eq!(packet.objects()[0].vertex_count(), 0);
        assert_eq!(packet.objects()[0].index_count(), 0);
    }
}
