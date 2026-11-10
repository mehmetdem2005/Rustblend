//! Transactional scene history. Snapshot storage is deliberately bounded;
//! large-mesh delta history is a later optimization.
use crate::scene::Scene;

#[derive(Debug, Clone, Default)]
pub struct History {
    undo: Vec<Scene>,
    redo: Vec<Scene>,
    pending: Option<Scene>,
}

const MAX_STEPS: usize = 32;

impl History {
    pub fn begin(&mut self, scene: &Scene) {
        if self.pending.is_none() {
            self.pending = Some(scene.clone());
        }
    }

    pub fn commit(&mut self, scene: &Scene) {
        if let Some(before) = self.pending.take() {
            if before != *scene {
                self.undo.push(before);
                if self.undo.len() > MAX_STEPS {
                    self.undo.remove(0);
                }
                self.redo.clear();
            }
        }
    }

    pub fn cancel(&mut self, scene: &mut Scene) {
        if let Some(before) = self.pending.take() {
            *scene = before;
        }
    }

    pub fn undo(&mut self, scene: &mut Scene) -> bool {
        self.commit(scene);
        if let Some(before) = self.undo.pop() {
            self.redo.push(std::mem::replace(scene, before));
            true
        } else { false }
    }

    pub fn redo(&mut self, scene: &mut Scene) -> bool {
        self.commit(scene);
        if let Some(after) = self.redo.pop() {
            self.undo.push(std::mem::replace(scene, after));
            true
        } else { false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mesh, Vec3};

    fn selected() -> Scene {
        let mut scene = Scene::new();
        let id = scene.add_object("Cube", Mesh::cube(2.0).unwrap()).unwrap();
        scene.select(id).unwrap();
        scene
    }

    #[test]
    fn drag_is_one_step_and_redo_restores_exact_scene() {
        let mut scene = selected();
        let original = scene.clone();
        let mut history = History::default();
        for _ in 0..10 {
            history.begin(&scene);
            scene.translate_active(Vec3::new(1.0, 0.0, 0.0)).unwrap();
        }
        history.commit(&scene);
        let moved = scene.clone();
        assert!(history.undo(&mut scene));
        assert_eq!(scene, original);
        assert!(!history.undo(&mut scene));
        assert!(history.redo(&mut scene));
        assert_eq!(scene, moved);
    }

    #[test]
    fn cancelled_and_noop_transactions_preserve_redo() {
        let mut scene = selected();
        let mut history = History::default();
        history.begin(&scene);
        scene.duplicate_active(Vec3::ZERO).unwrap();
        history.commit(&scene);
        let duplicated = scene.clone();
        history.undo(&mut scene);
        let original = scene.clone();
        history.begin(&scene);
        scene.translate_active(Vec3::new(2.0, 0.0, 0.0)).unwrap();
        history.cancel(&mut scene);
        assert_eq!(scene, original);
        history.begin(&scene);
        history.commit(&scene);
        assert!(history.redo(&mut scene));
        assert_eq!(scene, duplicated);
    }

    #[test]
    fn new_edit_clears_redo_and_deletion_restores_selection() {
        let mut scene = selected();
        let original = scene.clone();
        let mut history = History::default();
        history.begin(&scene);
        scene.remove(scene.active_object_id().unwrap());
        history.commit(&scene);
        assert!(scene.is_empty());
        history.undo(&mut scene);
        assert_eq!(scene, original);
        history.begin(&scene);
        scene.translate_active(Vec3::new(1.0, 0.0, 0.0)).unwrap();
        history.commit(&scene);
        assert!(!history.redo(&mut scene));
    }

    #[test]
    fn history_is_bounded_and_survives_workspace_snapshot() {
        let mut scene = selected();
        let mut history = History::default();
        for _ in 0..40 {
            history.begin(&scene);
            scene.translate_active(Vec3::new(1.0, 0.0, 0.0)).unwrap();
            history.commit(&scene);
        }
        let mut restored = history.clone();
        let mut count = 0;
        while restored.undo(&mut scene) { count += 1; }
        assert_eq!(count, MAX_STEPS);
    }
}
