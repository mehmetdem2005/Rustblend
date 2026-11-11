pub mod math;
pub mod geometry;
pub mod camera;
pub mod scene;
pub mod render;
pub mod viewport_ui;
pub mod workspace;
pub mod history;

#[cfg(target_os = "android")]
mod gpu;

#[cfg(target_os = "android")]
mod android_app;

pub use math::{Mat4, Vec3};
pub use geometry::{Bounds, Edge, Face, Mesh, MeshError};
pub use camera::{Camera, CameraError};
pub use scene::{
    ObjectId,
    Scene,
    SceneError,
    SceneObject,
    Transform,
};
pub use render::{
    RenderError,
    RenderObject,
    RenderPacket,
    RenderVertex,
};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(
    app: winit::platform::android::activity::AndroidApp,
) {
    android_app::run(app);
}

pub mod project;

pub mod recovery_worker;
pub mod frame_retry;
