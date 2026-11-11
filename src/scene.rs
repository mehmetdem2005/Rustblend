use crate::geometry::Mesh;
use crate::math::{Mat4, Quaternion, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(u64);

impl ObjectId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneError {
    EmptyName,
    InvalidTransform,
    IdExhausted,
    ObjectNotFound,
    InvalidSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    position: Vec3,
    rotation: Quaternion,
    scale: Vec3,
}

impl Transform {
    pub fn new(
        position: Vec3,
        rotation: Quaternion,
        scale: Vec3,
    ) -> Result<Self, SceneError> {
        if !position.is_finite() || !scale.is_finite() {
            return Err(SceneError::InvalidTransform);
        }

        let rotation = rotation
            .normalized()
            .ok_or(SceneError::InvalidTransform)?;

        Ok(Self {
            position,
            rotation,
            scale,
        })
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn rotation(&self) -> Quaternion {
        self.rotation
    }

    pub fn scale(&self) -> Vec3 {
        self.scale
    }

    pub fn model_matrix(&self) -> Mat4 {
        let q = self.rotation;

        let xx = q.x * q.x;
        let yy = q.y * q.y;
        let zz = q.z * q.z;

        let xy = q.x * q.y;
        let xz = q.x * q.z;
        let yz = q.y * q.z;

        let xw = q.x * q.w;
        let yw = q.y * q.w;
        let zw = q.z * q.w;

        let rotation_matrix = Mat4 {
            elements: [
                1.0 - 2.0 * (yy + zz),
                2.0 * (xy + zw),
                2.0 * (xz - yw),
                0.0,

                2.0 * (xy - zw),
                1.0 - 2.0 * (xx + zz),
                2.0 * (yz + xw),
                0.0,

                2.0 * (xz + yw),
                2.0 * (yz - xw),
                1.0 - 2.0 * (xx + yy),
                0.0,

                0.0, 0.0, 0.0, 1.0,
            ],
        };

        Mat4::from_translation(self.position)
            * rotation_matrix
            * Mat4::from_scale(self.scale)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quaternion::IDENTITY,
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneObject {
    id: ObjectId,
    name: String,
    mesh: Mesh,
    transform: Transform,
}

impl SceneObject {
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    pub fn model_matrix(&self) -> Mat4 {
        self.transform.model_matrix()
    }

    pub fn set_transform(&mut self, transform: Transform) {
        self.transform = transform;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    objects: Vec<SceneObject>,
    next_id: u64,

    // Sahnedeki seçili nesneler.
    selected: Vec<ObjectId>,

    // Seçili nesneler arasındaki aktif nesne.
    active: Option<ObjectId>,
}

impl Scene {
    pub(crate) fn next_object_id(&self) -> u64 { self.next_id }

    /// Build off to the side, validating IDs and selection before exposing a scene.
    pub(crate) fn restore(
        records: Vec<(u64, String, Mesh, Transform)>, next_id: u64,
        selected: Vec<u64>, active: Option<u64>,
    ) -> Result<Self, SceneError> {
        let mut ids = std::collections::HashSet::new();
        let mut objects = Vec::with_capacity(records.len());
        if next_id == 0 { return Err(SceneError::InvalidSnapshot); }
        for (id, name, mesh, transform) in records {
            if id == 0 || id >= next_id || !ids.insert(id) || name.trim().is_empty() {
                return Err(SceneError::InvalidSnapshot);
            }
            objects.push(SceneObject { id: ObjectId(id), name, mesh, transform });
        }
        let mut selection = std::collections::HashSet::new();
        if selected.iter().any(|id| !ids.contains(id) || !selection.insert(*id))
            || active.is_some_and(|id| !selection.contains(&id))
            || (active.is_none() && !selected.is_empty()) {
            return Err(SceneError::InvalidSnapshot);
        }
        Ok(Self { objects, next_id, selected: selected.into_iter().map(ObjectId).collect(), active: active.map(ObjectId) })
    }

    pub fn reset_active_transform(&mut self) -> Result<(), SceneError> {
        let id = self.active.ok_or(SceneError::ObjectNotFound)?;
        self.get_mut(id).ok_or(SceneError::ObjectNotFound)?.set_transform(Transform::default());
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            next_id: 1,
            selected: Vec::new(),
            active: None,
        }
    }

    pub fn add_object(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
    ) -> Result<ObjectId, SceneError> {
        self.add_object_with_transform(
            name,
            mesh,
            Transform::default(),
        )
    }

    pub fn add_object_with_transform(
        &mut self,
        name: impl Into<String>,
        mesh: Mesh,
        transform: Transform,
    ) -> Result<ObjectId, SceneError> {
        let name = name.into();

        if name.trim().is_empty() {
            return Err(SceneError::EmptyName);
        }

        let id = ObjectId(self.next_id);

        let next_id = self.next_id
            .checked_add(1)
            .ok_or(SceneError::IdExhausted)?;

        self.objects.push(SceneObject {
            id,
            name,
            mesh,
            transform,
        });

        self.next_id = next_id;

        Ok(id)
    }

    pub fn get(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects
            .iter()
            .find(|object| object.id == id)
    }

    pub fn get_mut(
        &mut self,
        id: ObjectId,
    ) -> Option<&mut SceneObject> {
        self.objects
            .iter_mut()
            .find(|object| object.id == id)
    }

    // Tek nesne seçer ve önceki seçimi kaldırır.
    pub fn select(
        &mut self,
        id: ObjectId,
    ) -> Result<(), SceneError> {
        if self.get(id).is_none() {
            return Err(SceneError::ObjectNotFound);
        }

        self.selected.clear();
        self.selected.push(id);
        self.active = Some(id);

        Ok(())
    }

    // Mevcut seçime yeni nesne ekler.
    pub fn add_to_selection(
        &mut self,
        id: ObjectId,
    ) -> Result<(), SceneError> {
        if self.get(id).is_none() {
            return Err(SceneError::ObjectNotFound);
        }

        if !self.selected.contains(&id) {
            self.selected.push(id);
        }

        self.active = Some(id);

        Ok(())
    }

    // Bir nesnenin seçimini kaldırır.
    pub fn deselect(&mut self, id: ObjectId) -> bool {
        let Some(index) = self.selected
            .iter()
            .position(|selected_id| *selected_id == id)
        else {
            return false;
        };

        self.selected.remove(index);

        if self.active == Some(id) {
            self.active = self.selected.last().copied();
        }

        true
    }

    // Bütün seçimleri kaldırır.
    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.active = None;
    }

    // Bütün nesneleri seçer.
    pub fn select_all(&mut self) {
        self.selected = self.objects
            .iter()
            .map(|object| object.id)
            .collect();

        self.active = self.selected.last().copied();
    }

    // Aktif nesneyi belirler.
    // Seçili değilse otomatik olarak seçime ekler.
    pub fn set_active(
        &mut self,
        id: ObjectId,
    ) -> Result<(), SceneError> {
        if self.get(id).is_none() {
            return Err(SceneError::ObjectNotFound);
        }

        if !self.selected.contains(&id) {
            self.selected.push(id);
        }

        self.active = Some(id);

        Ok(())
    }

    pub fn is_selected(&self, id: ObjectId) -> bool {
        self.selected.contains(&id)
    }

    pub fn selected_ids(&self) -> &[ObjectId] {
        &self.selected
    }

    pub fn selection_count(&self) -> usize {
        self.selected.len()
    }

    pub fn active_object_id(&self) -> Option<ObjectId> {
        self.active
    }

    pub fn active_object(&self) -> Option<&SceneObject> {
        self.active.and_then(|id| self.get(id))
    }

    pub fn active_object_mut(
        &mut self,
    ) -> Option<&mut SceneObject> {
        let id = self.active?;
        self.get_mut(id)
    }

    // Nesneyi sahneden kaldırır.
    pub fn remove(&mut self, id: ObjectId) -> Option<SceneObject> {
        let index = self.objects
            .iter()
            .position(|object| object.id == id)?;

        let removed = self.objects.remove(index);

        self.selected.retain(|selected_id| *selected_id != id);

        if self.active == Some(id) {
            self.active = self.selected.last().copied();
        }

        Some(removed)
    }

    // Duplicates the active object and selects the new copy.
    pub fn duplicate_active(
        &mut self,
        offset: Vec3,
    ) -> Result<ObjectId, SceneError> {
        if !offset.is_finite() {
            return Err(SceneError::InvalidTransform);
        }

        let source = self.active_object()
            .cloned()
            .ok_or(SceneError::ObjectNotFound)?;

        let source_transform = source.transform();
        let transform = Transform::new(
            source_transform.position() + offset,
            source_transform.rotation(),
            source_transform.scale(),
        )?;

        let name = format!("{} Copy", source.name());
        let new_id = self.add_object_with_transform(
            name,
            source.mesh().clone(),
            transform,
        )?;

        self.select(new_id)?;
        Ok(new_id)
    }

    pub fn translate_active(
        &mut self,
        delta: Vec3,
    ) -> Result<(), SceneError> {
        if !delta.is_finite() {
            return Err(SceneError::InvalidTransform);
        }

        let id = self.active.ok_or(SceneError::ObjectNotFound)?;
        let current = self.get(id)
            .ok_or(SceneError::ObjectNotFound)?
            .transform();

        let next = Transform::new(
            current.position() + delta,
            current.rotation(),
            current.scale(),
        )?;

        self.get_mut(id)
            .ok_or(SceneError::ObjectNotFound)?
            .set_transform(next);

        Ok(())
    }

    pub fn rotate_active(
        &mut self,
        axis: Vec3,
        angle: f32,
    ) -> Result<(), SceneError> {
        let delta = Quaternion::from_axis_angle(axis, angle)
            .ok_or(SceneError::InvalidTransform)?;

        let id = self.active.ok_or(SceneError::ObjectNotFound)?;
        let current = self.get(id)
            .ok_or(SceneError::ObjectNotFound)?
            .transform();

        let next = Transform::new(
            current.position(),
            delta * current.rotation(),
            current.scale(),
        )?;

        self.get_mut(id)
            .ok_or(SceneError::ObjectNotFound)?
            .set_transform(next);

        Ok(())
    }

    pub fn scale_active(
        &mut self,
        factor: f32,
    ) -> Result<(), SceneError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(SceneError::InvalidTransform);
        }

        let id = self.active.ok_or(SceneError::ObjectNotFound)?;
        let current = self.get(id)
            .ok_or(SceneError::ObjectNotFound)?
            .transform();

        let next = Transform::new(
            current.position(),
            current.rotation(),
            current.scale() * factor,
        )?;

        self.get_mut(id)
            .ok_or(SceneError::ObjectNotFound)?
            .set_transform(next);

        Ok(())
    }

    pub fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::FRAC_PI_2;

    fn cube() -> Mesh {
        Mesh::cube(2.0).unwrap()
    }

    #[test]
    fn new_scene_is_empty() {
        let scene = Scene::new();

        assert!(scene.is_empty());
        assert_eq!(scene.object_count(), 0);
        assert_eq!(scene.selection_count(), 0);
        assert_eq!(scene.active_object_id(), None);
    }

    #[test]
    fn default_scene_starts_with_id_one() {
        let mut scene = Scene::default();

        let id = scene.add_object("Cube", cube()).unwrap();

        assert_eq!(id.get(), 1);
    }

    #[test]
    fn add_cube_to_scene() {
        let mut scene = Scene::new();

        let id = scene.add_object("Cube", cube()).unwrap();

        assert_eq!(scene.object_count(), 1);
        assert_eq!(scene.get(id).unwrap().name(), "Cube");
    }

    #[test]
    fn object_ids_are_unique() {
        let mut scene = Scene::new();

        let first = scene.add_object("Cube", cube()).unwrap();
        let second = scene.add_object("Sphere", cube()).unwrap();

        assert_ne!(first, second);
        assert_eq!(first.get(), 1);
        assert_eq!(second.get(), 2);
    }

    #[test]
    fn empty_name_is_rejected() {
        let mut scene = Scene::new();

        assert_eq!(
            scene.add_object("   ", cube()),
            Err(SceneError::EmptyName)
        );

        assert_eq!(scene.object_count(), 0);
    }

    #[test]
    fn find_object_by_id() {
        let mut scene = Scene::new();

        let id = scene.add_object("Cube", cube()).unwrap();
        let object = scene.get(id).unwrap();

        assert_eq!(object.id(), id);
        assert_eq!(object.mesh().vertex_count(), 8);
    }

    #[test]
    fn remove_object_from_scene() {
        let mut scene = Scene::new();

        let id = scene.add_object("Cube", cube()).unwrap();
        let removed = scene.remove(id);

        assert!(removed.is_some());
        assert!(scene.get(id).is_none());
        assert!(scene.is_empty());
    }

    #[test]
    fn default_transform_is_identity() {
        let transform = Transform::default();

        assert_eq!(
            transform.model_matrix(),
            Mat4::identity()
        );
    }

    #[test]
    fn transform_moves_object() {
        let transform = Transform::new(
            Vec3::new(10.0, 20.0, 30.0),
            Quaternion::IDENTITY,
            Vec3::new(1.0, 1.0, 1.0),
        ).unwrap();

        let result = transform.model_matrix()
            .transform_point(Vec3::ZERO)
            .unwrap();

        assert_eq!(
            result,
            Vec3::new(10.0, 20.0, 30.0)
        );
    }

    #[test]
    fn transform_scales_object() {
        let transform = Transform::new(
            Vec3::ZERO,
            Quaternion::IDENTITY,
            Vec3::new(2.0, 3.0, 4.0),
        ).unwrap();

        let result = transform.model_matrix()
            .transform_point(Vec3::new(1.0, 1.0, 1.0))
            .unwrap();

        assert_eq!(
            result,
            Vec3::new(2.0, 3.0, 4.0)
        );
    }

    #[test]
    fn transform_rotates_object() {
        let rotation = Quaternion::from_axis_angle(
            Vec3::new(0.0, 0.0, 1.0),
            FRAC_PI_2,
        ).unwrap();

        let transform = Transform::new(
            Vec3::ZERO,
            rotation,
            Vec3::new(1.0, 1.0, 1.0),
        ).unwrap();

        let result = transform.model_matrix()
            .transform_point(Vec3::new(1.0, 0.0, 0.0))
            .unwrap();

        assert!(result.x.abs() < 0.00001);
        assert!((result.y - 1.0).abs() < 0.00001);
    }

    #[test]
    fn transform_combines_scale_and_translation() {
        let transform = Transform::new(
            Vec3::new(10.0, 20.0, 30.0),
            Quaternion::IDENTITY,
            Vec3::new(2.0, 3.0, 4.0),
        ).unwrap();

        let result = transform.model_matrix()
            .transform_point(Vec3::new(1.0, 1.0, 1.0))
            .unwrap();

        assert_eq!(
            result,
            Vec3::new(12.0, 23.0, 34.0)
        );
    }

    #[test]
    fn invalid_transform_is_rejected() {
        assert_eq!(
            Transform::new(
                Vec3::new(f32::NAN, 0.0, 0.0),
                Quaternion::IDENTITY,
                Vec3::new(1.0, 1.0, 1.0),
            ),
            Err(SceneError::InvalidTransform)
        );

        assert_eq!(
            Transform::new(
                Vec3::ZERO,
                Quaternion::new(0.0, 0.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 1.0),
            ),
            Err(SceneError::InvalidTransform)
        );
    }

    #[test]
    fn select_single_object() {
        let mut scene = Scene::new();

        let id = scene.add_object("Cube", cube()).unwrap();

        scene.select(id).unwrap();

        assert!(scene.is_selected(id));
        assert_eq!(scene.selection_count(), 1);
        assert_eq!(scene.active_object_id(), Some(id));
    }

    #[test]
    fn additive_selection_selects_multiple_objects() {
        let mut scene = Scene::new();

        let first = scene.add_object("Cube", cube()).unwrap();
        let second = scene.add_object("Sphere", cube()).unwrap();

        scene.select(first).unwrap();
        scene.add_to_selection(second).unwrap();

        assert_eq!(scene.selection_count(), 2);
        assert!(scene.is_selected(first));
        assert!(scene.is_selected(second));
        assert_eq!(scene.active_object_id(), Some(second));
    }

    #[test]
    fn duplicate_selection_does_not_duplicate_id() {
        let mut scene = Scene::new();

        let id = scene.add_object("Cube", cube()).unwrap();

        scene.select(id).unwrap();
        scene.add_to_selection(id).unwrap();

        assert_eq!(scene.selection_count(), 1);
    }

    #[test]
    fn deselecting_active_chooses_previous_selection() {
        let mut scene = Scene::new();

        let first = scene.add_object("Cube", cube()).unwrap();
        let second = scene.add_object("Sphere", cube()).unwrap();

        scene.select(first).unwrap();
        scene.add_to_selection(second).unwrap();

        assert!(scene.deselect(second));

        assert_eq!(scene.active_object_id(), Some(first));
        assert!(scene.is_selected(first));
        assert!(!scene.is_selected(second));
    }

    #[test]
    fn clear_selection_removes_all_selections() {
        let mut scene = Scene::new();

        let first = scene.add_object("Cube", cube()).unwrap();
        let second = scene.add_object("Sphere", cube()).unwrap();

        scene.select(first).unwrap();
        scene.add_to_selection(second).unwrap();

        scene.clear_selection();

        assert_eq!(scene.selection_count(), 0);
        assert_eq!(scene.active_object_id(), None);
    }

    #[test]
    fn set_active_selects_object_automatically() {
        let mut scene = Scene::new();

        let id = scene.add_object("Cube", cube()).unwrap();

        scene.set_active(id).unwrap();

        assert_eq!(scene.active_object_id(), Some(id));
        assert!(scene.is_selected(id));
        assert_eq!(scene.active_object().unwrap().name(), "Cube");
    }

    #[test]
    fn removing_selected_object_updates_selection() {
        let mut scene = Scene::new();

        let first = scene.add_object("Cube", cube()).unwrap();
        let second = scene.add_object("Sphere", cube()).unwrap();

        scene.select(first).unwrap();
        scene.add_to_selection(second).unwrap();

        scene.remove(second).unwrap();

        assert_eq!(scene.selection_count(), 1);
        assert_eq!(scene.active_object_id(), Some(first));
        assert!(scene.is_selected(first));
    }

    #[test]
    fn removing_only_selected_object_clears_active() {
        let mut scene = Scene::new();

        let id = scene.add_object("Cube", cube()).unwrap();

        scene.select(id).unwrap();
        scene.remove(id).unwrap();

        assert_eq!(scene.active_object_id(), None);
        assert_eq!(scene.selection_count(), 0);
    }

    #[test]
    fn selecting_missing_object_returns_error() {
        let mut scene = Scene::new();

        let missing = ObjectId(999);

        assert_eq!(
            scene.select(missing),
            Err(SceneError::ObjectNotFound)
        );

        assert_eq!(
            scene.set_active(missing),
            Err(SceneError::ObjectNotFound)
        );
    }

    #[test]
    fn select_all_selects_every_object() {
        let mut scene = Scene::new();

        let first = scene.add_object("Cube", cube()).unwrap();
        let second = scene.add_object("Sphere", cube()).unwrap();

        scene.select_all();

        assert_eq!(scene.selection_count(), 2);
        assert!(scene.is_selected(first));
        assert!(scene.is_selected(second));
        assert_eq!(scene.active_object_id(), Some(second));
    }
}


#[cfg(test)]
mod modeling_operation_tests {
    use super::*;

    fn selected_scene() -> Scene {
        let mut scene = Scene::new();
        let id = scene.add_object("Cube", Mesh::cube(2.0).unwrap()).unwrap();
        scene.select(id).unwrap();
        scene
    }

    #[test]
    fn duplicate_active_copies_mesh_and_selects_new_object() {
        let mut scene = selected_scene();
        let source_id = scene.active_object_id().unwrap();

        let copy_id = scene.duplicate_active(Vec3::new(2.0, 0.0, 0.0)).unwrap();

        assert_ne!(source_id, copy_id);
        assert_eq!(scene.object_count(), 2);
        assert_eq!(scene.active_object_id(), Some(copy_id));
        assert_eq!(scene.active_object().unwrap().mesh().vertex_count(), 8);
        assert_eq!(
            scene.active_object().unwrap().transform().position(),
            Vec3::new(2.0, 0.0, 0.0)
        );
    }

    #[test]
    fn translate_active_changes_only_active_position() {
        let mut scene = selected_scene();

        scene.translate_active(Vec3::new(1.0, 2.0, 3.0)).unwrap();

        assert_eq!(
            scene.active_object().unwrap().transform().position(),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn rotate_active_changes_rotation() {
        let mut scene = selected_scene();

        scene.rotate_active(Vec3::new(0.0, 1.0, 0.0), core::f32::consts::FRAC_PI_2).unwrap();

        let rotation = scene.active_object().unwrap().transform().rotation();
        let rotated = rotation.rotate_vector(Vec3::new(0.0, 0.0, 1.0)).unwrap();

        assert!((rotated.x - 1.0).abs() < 0.0001);
        assert!(rotated.z.abs() < 0.0001);
    }

    #[test]
    fn scale_active_changes_uniform_scale() {
        let mut scene = selected_scene();

        scene.scale_active(1.5).unwrap();

        assert_eq!(
            scene.active_object().unwrap().transform().scale(),
            Vec3::new(1.5, 1.5, 1.5)
        );
    }

    #[test]
    fn invalid_active_scale_is_rejected_without_mutation() {
        let mut scene = selected_scene();

        assert_eq!(
            scene.scale_active(0.0),
            Err(SceneError::InvalidTransform)
        );

        assert_eq!(
            scene.active_object().unwrap().transform().scale(),
            Vec3::new(1.0, 1.0, 1.0)
        );
    }
}

#[cfg(test)]
mod reset_tests {
 use super::*;
 #[test] fn reset_can_be_undone_and_preserves_other_objects(){
  let mut scene=Scene::new();let a=scene.add_object("A",Mesh::cube(1.0).unwrap()).unwrap();
  let b=scene.add_object("B",Mesh::cube(1.0).unwrap()).unwrap();scene.select(a).unwrap();
  scene.translate_active(Vec3::new(3.0,2.0,1.0)).unwrap();let before=scene.clone();
  let mut history=crate::history::History::default();history.begin(&scene);
  scene.reset_active_transform().unwrap();history.commit(&scene);
  assert_eq!(*scene.get(a).unwrap().transform(),Transform::default());
  assert_eq!(scene.get(b),before.get(b));history.undo(&mut scene);assert_eq!(scene,before);
 }
 #[test] fn reset_empty_scene_is_safe(){assert_eq!(Scene::new().reset_active_transform(),Err(SceneError::ObjectNotFound));}
}
