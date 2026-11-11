use std::collections::HashMap;
use std::sync::Arc;

use crate::gpu::{GpuRenderer, FrameResult};
use crate::recovery_worker::{RecoveryWorker, SaveStatus};
use crate::frame_retry::FrameRetry;
use std::time::Instant;

#[derive(Debug)]
enum AppEvent { RecoveryFinished, ProjectsFinished }

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
    recovery: Option<RecoveryWorker>,
    frame_retry: FrameRetry,
    occluded: bool,
    projects: Option<crate::project_service::ProjectService>,
    active_project: Option<crate::project_catalog::ProjectEntry>,
}

impl RustBlendApp {
    fn project_action(&mut self, action: crate::project_ui::ProjectAction) {
        use crate::project_ui::ProjectAction;
        use crate::project_service::ProjectJob;
        if matches!(action, ProjectAction::Ignore) { return; }
        let Some(renderer) = self.renderer.as_mut() else { return; };
        if matches!(action, ProjectAction::Toggle | ProjectAction::Previous | ProjectAction::Next | ProjectAction::Close) {
            renderer.navigate_projects(action);
            if action != ProjectAction::Toggle || !renderer.project_panel_mut().open { return; }
        }
        let Some(service) = self.projects.as_mut() else {
            renderer.project_panel_mut().message = "PROJECT STORAGE UNAVAILABLE".into();
            renderer.refresh_projects(); return;
        };
        let job = match action {
            ProjectAction::Toggle => ProjectJob::List,
            ProjectAction::SaveCurrent => match self.active_project.as_ref() {
                Some(entry) => ProjectJob::SaveCurrent(entry.id,renderer.recovery_snapshot()),
                None => ProjectJob::SaveCopy(renderer.recovery_snapshot()),
            },
            ProjectAction::SaveCopy => ProjectJob::SaveCopy(renderer.recovery_snapshot()),
            ProjectAction::Open(id) => ProjectJob::Open(id,renderer.recovery_snapshot()),
            ProjectAction::New => ProjectJob::New(renderer.recovery_snapshot()),
            _ => return,
        };
        match service.submit(job) {
            Ok(()) => renderer.project_panel_mut().busy = true,
            Err(error) => renderer.project_panel_mut().message = error.to_uppercase(),
        }
        renderer.refresh_projects();
    }

    fn finish_project_job(&mut self) {
        let Some(result) = self.projects.as_mut().and_then(|service| service.poll()) else { return; };
        let mut changed = false;
        match result {
            Ok(response) => {
                let mut message = response.message;
                let switching = response.workspace.is_some();
                if let Some(state) = response.workspace {
                    if let Some(renderer) = self.renderer.as_mut() {
                        match renderer.open_project(&state) {
                            Ok(()) => changed = true,
                            Err(error) => { eprintln!("RUSTBLEND project open failed: {error}"); message = "OPEN FAILED - CURRENT WORK PRESERVED".into(); }
                        }
                    } else {
                        self.suspended_workspace = Some(state); changed = true;
                    }
                }
                if !switching || changed {
                    if switching || response.active.is_some() { self.active_project = response.active; }
                }
                if let Some(entry) = self.active_project.as_ref() {
                    message = format!("{} | {}",entry.name,message);
                }
                if let Some(renderer) = self.renderer.as_mut() {
                    let panel = renderer.project_panel_mut();
                    panel.entries = response.entries; panel.message = message; panel.page = 0; panel.busy = false;
                    renderer.refresh_projects();
                }
            }
            Err(error) => {
                eprintln!("RUSTBLEND project operation failed: {error}");
                if let Some(renderer) = self.renderer.as_mut() {
                    let panel = renderer.project_panel_mut(); panel.busy = false; panel.message = format!("FAILED: {}",error.to_uppercase());
                    renderer.refresh_projects();
                }
            }
        }
        if changed { self.save_recovery(); }
    }

    fn save_recovery(&mut self) {
        let Some(worker) = self.recovery.as_mut() else { return; };
        let snapshot = if let Some(renderer) = self.renderer.as_ref() {
            renderer.recovery_snapshot()
        } else if let Some(state) = self.suspended_workspace.as_ref() {
            state.recovery_snapshot()
        } else { return; };
        if let Err(error) = worker.submit(snapshot) {
            eprintln!("RUSTBLEND recovery submit failed: {error}");
            if let Some(renderer) = self.renderer.as_mut() { renderer.set_recovery_status("SAVE FAILED"); }
            return;
        }
        if let Some(renderer) = self.renderer.as_mut() { renderer.set_recovery_status("SAVING"); }
    }

    fn refresh_recovery_status(&mut self) {
        let status = match self.recovery.as_ref().map(|worker| worker.status()) {
            None => "RECOVERY BLOCKED",
            Some(_) if !self.touches.is_empty() => "NOT SAVED",
            Some(SaveStatus::Idle) => return,
            Some(SaveStatus::Saving) => "SAVING",
            Some(SaveStatus::Saved) => "SAVED",
            Some(SaveStatus::Failed(error)) => {
                eprintln!("RUSTBLEND recovery save failed: {error}"); "SAVE FAILED"
            }
        };
        if let Some(renderer) = self.renderer.as_mut() { renderer.set_recovery_status(status); }
    }

    fn handle_touch(
        &mut self,
        phase: TouchPhase,
        id: u64,
        position: (f64, f64),
    ) {
        if self.projects.as_ref().is_some_and(|service| service.busy()) { return; }
        match phase {
            TouchPhase::Started => {
                if self.recovery.is_some() {
                    if let Some(renderer) = self.renderer.as_mut() { renderer.set_recovery_status("NOT SAVED"); }
                }
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

        if let Some(action) = self.renderer.as_mut().and_then(|renderer|renderer.take_project_action()) {
            self.project_action(action);
        }
        if self.touches.is_empty() && matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
            self.save_recovery();
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

impl ApplicationHandler<AppEvent> for RustBlendApp {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::RecoveryFinished => self.refresh_recovery_status(),
            AppEvent::ProjectsFinished => self.finish_project_job(),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_none() || self.occluded {
            self.frame_retry.clear();
        } else if self.frame_retry.take_due(Instant::now()) {
            if let Some(window) = self.window.as_ref() { window.request_redraw(); }
        }
        event_loop.set_control_flow(match self.frame_retry.deadline() {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None => ControlFlow::Wait,
        });
    }

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
            Ok(mut renderer) => {
                renderer.set_recovery_status(if self.recovery.is_none() { "RECOVERY BLOCKED" }
                    else if self.suspended_workspace.is_some() { "RESTORED" } else { "NOT SAVED" });
                let gpu_name = renderer.adapter_name().to_owned();

                window.set_title(
                    &format!("RUSTBLEND | {gpu_name}")
                );

                eprintln!(
                    "RUSTBLEND: Modeling workspace ready on {gpu_name}; objects={}.",
                    renderer.scene_object_count()
                );

                self.window = Some(window.clone());
                if self.projects.as_ref().is_some_and(|service|service.busy()) {
                    let panel = renderer.project_panel_mut(); panel.open = true; panel.busy = true;
                    renderer.refresh_projects();
                }
                self.renderer = Some(renderer);
                self.suspended_workspace = None;
                self.touches.clear();

                self.occluded = false;
                self.frame_retry.reset();
                self.refresh_recovery_status();
                event_loop.set_control_flow(ControlFlow::Wait);
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
        self.save_recovery();
        self.frame_retry.clear();
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
                self.save_recovery();
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                self.occluded = size.width == 0 || size.height == 0;
                self.frame_retry.reset();
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
            }

            WindowEvent::Touch(touch) => {
                self.frame_retry.reset();
                self.handle_touch(
                    touch.phase,
                    touch.id,
                    (touch.location.x, touch.location.y),
                );
            }

            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                self.frame_retry.reset();
                if !occluded {
                    if let Some(window) = self.window.as_ref() { window.request_redraw(); }
                }
            }

            WindowEvent::RedrawRequested => {
                if self.occluded { return; }
                if let Some(renderer) = self.renderer.as_mut() {
                    match renderer.render() {
                        FrameResult::Presented => self.frame_retry.reset(),
                        FrameResult::Retry => self.frame_retry.failed(Instant::now()),
                        FrameResult::Unavailable => self.frame_retry.clear(),
                    }
                }
            }

            _ => {}
        }
    }
}

pub fn run(app: AndroidApp) {
    let recovery_path = app.internal_data_path().map(|p| p.join("projects"));
    let catalog_path = app.internal_data_path().map(|p| p.join("named-projects"));
    let mut builder = EventLoop::<AppEvent>::with_user_event();

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
    if let Some(path) = recovery_path {
        match crate::project::RecoveryStore::open(path) {
            Ok((store, state)) => {
                application.suspended_workspace = state;
                let proxy = event_loop.create_proxy();
                match RecoveryWorker::new(store, move || { let _ = proxy.send_event(AppEvent::RecoveryFinished); }) {
                    Ok(worker) => application.recovery = Some(worker),
                    Err(error) => eprintln!("RUSTBLEND recovery worker unavailable: {error}"),
                }
            }
            Err(error) => eprintln!("RUSTBLEND recovery disabled; existing files preserved: {error}"),
        }
    }

    if let Some(path) = catalog_path {
        let proxy = event_loop.create_proxy();
        match crate::project_catalog::ProjectCatalog::open(path).and_then(|catalog|
            crate::project_service::ProjectService::new(catalog, move || { let _ = proxy.send_event(AppEvent::ProjectsFinished); })) {
            Ok(service) => application.projects = Some(service),
            Err(error) => eprintln!("RUSTBLEND project storage unavailable: {error}"),
        }
    }

    if let Err(error) = event_loop.run_app(&mut application) {
        eprintln!("RUSTBLEND: Android application loop failed: {error}");
    }
}
