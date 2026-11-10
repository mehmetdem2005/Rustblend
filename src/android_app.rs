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

#[derive(Debug, Clone, Copy)]
struct TouchState {
    start: (f64, f64),
    last: (f64, f64),
    moved: bool,
}

#[derive(Default)]
struct RustBlendApp {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    touches: HashMap<u64, TouchState>,
    suspended_workspace: Option<crate::workspace::WorkspaceState>,
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
                if !self.touches.is_empty() {
                    if let Some(renderer) = self.renderer.as_mut() {
                        renderer.finish_drag(false);
                    }
                }
                self.touches.insert(id, TouchState {
                    start: position,
                    last: position,
                    moved: false,
                });
            }

            TouchPhase::Moved => {
                let Some(previous_state) = self.touches.get(&id).copied() else {
                    return;
                };

                if self.touches.len() == 1 {
                    let delta_x = (position.0 - previous_state.last.0) as f32;
                    let delta_y = (position.1 - previous_state.last.1) as f32;
                    let moved = distance(previous_state.start, position) > 8.0;
                    let start = previous_state.start;

                    if let Some(state) = self.touches.get_mut(&id) {
                        state.last = position;
                        state.moved |= moved;
                    }

                    if moved {
                        if let Some(renderer) = self.renderer.as_mut() {
                            if !renderer.is_ui_position(start.0 as f32, start.1 as f32) {
                                renderer.drag_active(delta_x, delta_y);
                            }
                        }
                    }
                } else if self.touches.len() == 2 {
                    let other = self.touches.iter()
                        .find_map(|(&other_id, state)| {
                            (other_id != id).then_some(*state)
                        });

                    if let Some(other_state) = other {
                        let old_distance = distance(previous_state.last, other_state.last);
                        let new_distance = distance(position, other_state.last);

                        if let Some(state) = self.touches.get_mut(&id) {
                            state.last = position;
                            state.moved = true;
                        }
                        let other_id = self.touches.keys()
                            .copied()
                            .find(|other_id| *other_id != id);
                        if let Some(other_id) = other_id {
                            if let Some(state) = self.touches.get_mut(&other_id) {
                                state.moved = true;
                            }
                        }

                        let touches_on_ui = self.renderer.as_ref().is_some_and(|renderer| {
                            renderer.is_ui_position(
                                previous_state.start.0 as f32,
                                previous_state.start.1 as f32,
                            ) || renderer.is_ui_position(
                                other_state.start.0 as f32,
                                other_state.start.1 as f32,
                            )
                        });

                        if !touches_on_ui && old_distance > 1.0 && new_distance > 1.0 {
                            let factor = (old_distance / new_distance).clamp(0.75, 1.33) as f32;
                            if let Some(renderer) = self.renderer.as_mut() {
                                renderer.zoom_camera(factor);
                            }
                        }
                    }
                }
            }

            TouchPhase::Ended => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.finish_drag(false);
                }
                let active_count = self.touches.len();
                let state = self.touches.remove(&id);

                if active_count == 1 {
                    if let Some(state) = state {
                        if !state.moved {
                            if let Some(renderer) = self.renderer.as_mut() {
                                renderer.handle_ui_tap(
                                    position.0 as f32,
                                    position.1 as f32,
                                );
                            }
                        }
                    }
                }
            }

            TouchPhase::Cancelled => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.finish_drag(true);
                }
                self.touches.clear();
            }
        }

        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
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
                .with_title("RUSTBLEND Modeling Workspace")
        ) {
            Ok(window) => Arc::new(window),

            Err(error) => {
                eprintln!("RUSTBLEND: Android window creation failed: {error}");
                event_loop.exit();
                return;
            }
        };

        let display = event_loop.owned_display_handle();

        eprintln!("RUSTBLEND: Initializing modeling workspace.");

        eprintln!("RUSTBLEND: GPU renderer construction started.");

        let renderer_attempt = std::panic::catch_unwind(
            std::panic::AssertUnwindSafe(|| {
                let mut renderer = pollster::block_on(
                    GpuRenderer::new(display, window.clone())
                )?;
                if let Some(saved) = self.suspended_workspace.as_ref() {
                    renderer.restore_workspace(saved)?;
                }
                Ok::<_, String>(renderer)
            })
        );

        let renderer = match renderer_attempt {
            Ok(result) => result,
            Err(payload) => {
                let message = payload.downcast_ref::<&str>()
                    .map(|value| (*value).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "Unknown Rust panic during GPU initialization.".to_string());

                eprintln!("RUSTBLEND: caught initialization panic: {message}");
                Err(format!("GPU initialization panic: {message}"))
            }
        };

        match renderer {
            Ok(renderer) => {
                let gpu_name = renderer.adapter_name().to_owned();

                window.set_title(
                    &format!("RUSTBLEND | {gpu_name}")
                );

                eprintln!(
                    "RUSTBLEND: Modeling workspace ready on {gpu_name}; objects={}.",
                    renderer.scene_object_count()
                );

                self.window = Some(window.clone());
                self.renderer = Some(renderer);
                self.suspended_workspace = None;
                self.touches.clear();

                event_loop.set_control_flow(ControlFlow::Poll);
                window.request_redraw();
            }

            Err(error) => {
                eprintln!("RUSTBLEND: Modeling workspace initialization failed: {error}");

                window.set_title("RUSTBLEND - SAFE VIEWPORT MODE");

                self.window = Some(window);
                self.renderer = None;

                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        eprintln!("RUSTBLEND: Android activity suspended.");

        event_loop.set_control_flow(ControlFlow::Wait);
        if let Some(renderer) = self.renderer.take() {
            self.suspended_workspace = Some(renderer.into_workspace());
        }
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
                self.handle_touch(
                    touch.phase,
                    touch.id,
                    (touch.location.x, touch.location.y),
                );
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
