use std::collections::HashMap;
use std::sync::Arc;

use crate::gpu::GpuRenderer;

use winit::{
    application::ApplicationHandler,
    event::{TouchPhase, WindowEvent},
    event_loop::{
        ActiveEventLoop,
        ControlFlow,
        EventLoop,
    },
    platform::android::{
        activity::AndroidApp,
        EventLoopBuilderExtAndroid,
    },
    window::{
        Window,
        WindowId,
    },
};

#[derive(Default)]
struct RustBlendApp {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    touches: HashMap<u64, (f64, f64)>,
}

impl RustBlendApp {
    fn handle_touch(
        &mut self,
        phase: TouchPhase,
        id: u64,
        position: (f64, f64),
    ) {
        match phase {
            TouchPhase::Started => {
                self.touches.insert(id, position);
            }

            TouchPhase::Moved => {
                let previous = self.touches.get(&id).copied();
                self.touches.insert(id, position);

                let Some(previous) = previous else {
                    return;
                };

                if self.touches.len() == 1 {
                    let delta_x = (position.0 - previous.0) as f32;
                    let delta_y = (position.1 - previous.1) as f32;

                    if let Some(renderer) = self.renderer.as_mut() {
                        renderer.orbit_camera(
                            -delta_x * 0.006,
                            -delta_y * 0.006,
                        );
                    }
                } else if self.touches.len() == 2 {
                    let other = self.touches.iter()
                        .find_map(|(&other_id, &other_position)| {
                            (other_id != id).then_some(other_position)
                        });

                    if let Some(other) = other {
                        let old_distance = distance(previous, other);
                        let new_distance = distance(position, other);

                        if old_distance > 1.0 && new_distance > 1.0 {
                            let factor = (old_distance / new_distance)
                                .clamp(0.75, 1.33) as f32;

                            if let Some(renderer) = self.renderer.as_mut() {
                                renderer.zoom_camera(factor);
                            }
                        }
                    }
                }
            }

            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.touches.remove(&id);
            }
        }
    }
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

impl ApplicationHandler for RustBlendApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        eprintln!("RUSTBLEND: Android activity resumed.");

        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("RUSTBLEND 3D Viewport")
        ) {
            Ok(window) => Arc::new(window),

            Err(error) => {
                eprintln!("RUSTBLEND: Android window creation failed: {error}");
                event_loop.exit();
                return;
            }
        };

        let display = event_loop.owned_display_handle();

        eprintln!("RUSTBLEND: Initializing 3D viewport.");

        let renderer = pollster::block_on(
            GpuRenderer::new(display, window.clone())
        );

        match renderer {
            Ok(renderer) => {
                let gpu_name = renderer.adapter_name().to_owned();

                window.set_title(
                    &format!("RUSTBLEND | {gpu_name}")
                );

                eprintln!(
                    "RUSTBLEND: 3D viewport ready on {gpu_name}; objects={}.",
                    renderer.scene_object_count()
                );

                self.window = Some(window.clone());
                self.renderer = Some(renderer);
                self.touches.clear();

                event_loop.set_control_flow(ControlFlow::Poll);
                window.request_redraw();
            }

            Err(error) => {
                eprintln!("RUSTBLEND: 3D viewport initialization failed: {error}");

                window.set_title("RUSTBLEND - VIEWPORT ERROR");

                self.window = Some(window);
                self.renderer = None;

                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        eprintln!("RUSTBLEND: Android activity suspended.");

        event_loop.set_control_flow(ControlFlow::Wait);
        self.renderer = None;
        self.window = None;
        self.touches.clear();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let correct_window = self.window
            .as_ref()
            .is_some_and(|window| window.id() == window_id);

        if !correct_window {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
            }

            WindowEvent::Touch(touch) => {
                let position = (
                    touch.location.x,
                    touch.location.y,
                );

                self.handle_touch(
                    touch.phase,
                    touch.id,
                    position,
                );

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.render();

                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }

            _ => {}
        }
    }
}

pub fn run(app: AndroidApp) {
    let mut builder = EventLoop::builder();

    builder.with_android_app(app);

    let event_loop = match builder.build() {
        Ok(event_loop) => event_loop,

        Err(error) => {
            eprintln!("RUSTBLEND: Android event loop creation failed: {error}");
            return;
        }
    };

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut application = RustBlendApp::default();

    if let Err(error) = event_loop.run_app(&mut application) {
        eprintln!("RUSTBLEND: Android application loop failed: {error}");
    }
}
