//! CPU-only suspended editor state. This is not a disk recovery format.
use crate::{camera::Camera, scene::Scene, viewport_ui::ToolMode};

#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub scene: Scene,
    pub history: crate::history::History,
    pub camera: Camera,
    pub tool: ToolMode,
    pub edit_mode: bool,
    pub outliner_open: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mesh, Vec3};

    #[test]
    fn retry_snapshot_preserves_scene_identity_selection_and_camera() {
        let mut scene = Scene::new();
        let id = scene.add_object("Edited", Mesh::cube(2.0).unwrap()).unwrap();
        scene.select(id).unwrap();
        scene.translate_active(Vec3::new(3.0, 2.0, 1.0)).unwrap();
        let camera = Camera::new(Vec3::ZERO, 7.0, 1.0).unwrap();
        let saved = WorkspaceState {
            scene, history: Default::default(), camera, tool: ToolMode::Move,
            edit_mode: true, outliner_open: false,
        };
        let mut failed_attempt = saved.clone();
        failed_attempt.scene.remove(id);
        assert_eq!(saved.scene.active_object_id(), Some(id));
        assert_eq!(saved.scene.get(id).unwrap().transform().position(), Vec3::new(3.0, 2.0, 1.0));
        let mut restored = saved.clone();
        assert_eq!(restored.camera, camera);
        assert_eq!(restored.tool, ToolMode::Move);
        assert!(restored.edit_mode);
        assert!(!restored.outliner_open);
        let new_id = restored.scene.add_object("Next", Mesh::cube(1.0).unwrap()).unwrap();
        assert_ne!(id, new_id);
        assert_eq!(saved.scene.object_count(), 1);
    }
}
