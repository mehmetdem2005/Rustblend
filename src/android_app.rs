use std::sync::Arc;

use crate::gpu::GpuRenderer;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
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
}

impl ApplicationHandler for RustBlendApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("RUSTBLEND")
        ) {
            Ok(window) => Arc::new(window),

            Err(error) => {
                eprintln!("Android penceresi oluşturulamadı: {error}");
                event_loop.exit();
                return;
            }
        };

        // wgpu başlatılırken Android ekran tanıtıcısını veriyoruz.
        let display = event_loop.owned_display_handle();

        let renderer = pollster::block_on(
            GpuRenderer::new(display, window.clone())
        );

        match renderer {
            Ok(renderer) => {
                let gpu_name = renderer.adapter_name().to_owned();

                window.set_title(
                    &format!("RUSTBLEND | {gpu_name}")
                );

                eprintln!("RUSTBLEND GPU: {gpu_name}");

                self.window = Some(window.clone());
                self.renderer = Some(renderer);

                window.request_redraw();
            }

            Err(error) => {
                eprintln!("RUSTBLEND GPU başlatma hatası: {error}");

                window.set_title("RUSTBLEND - GPU HATASI");

                self.window = Some(window);
            }
        }
    }

    // Android uygulaması arka plana geçtiğinde GPU yüzeyini bırak.
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.renderer = None;
        self.window = None;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if window.id() != window_id {
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

            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.render();
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
            eprintln!("Android event loop hatası: {error}");
            return;
        }
    };

    // Boştayken işlemciyi sürekli meşgul etme.
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut application = RustBlendApp::default();

    if let Err(error) = event_loop.run_app(&mut application) {
        eprintln!("RUSTBLEND uygulama döngüsü hatası: {error}");
    }
}
