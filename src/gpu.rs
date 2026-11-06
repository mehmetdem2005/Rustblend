use std::borrow::Cow;
use std::sync::Arc;

use winit::{
    dpi::PhysicalSize,
    event_loop::OwnedDisplayHandle,
    window::Window,
};

const TRIANGLE_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32
) -> VertexOutput {

    let positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.65),
        vec2<f32>(-0.65, -0.55),
        vec2<f32>(0.65, -0.55)
    );

    let colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.15, 0.25),
        vec3<f32>(0.1, 0.65, 1.0),
        vec3<f32>(0.2, 1.0, 0.55)
    );

    var output: VertexOutput;

    output.position = vec4<f32>(
        positions[index],
        0.0,
        1.0
    );

    output.color = colors[index];

    return output;
}

@fragment
fn fs_main(
    input: VertexOutput
) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

pub struct GpuRenderer {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    device: wgpu::Device,
    queue: wgpu::Queue,

    pipeline: wgpu::RenderPipeline,
    window: Arc<Window>,

    adapter_name: String,
}

impl GpuRenderer {
    pub async fn new(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
    ) -> Result<Self, String> {

        let mut instance_desc =
            wgpu::InstanceDescriptor::new_with_display_handle(
                Box::new(display)
            );

        instance_desc.backends = wgpu::Backends::GL;

        let instance = wgpu::Instance::new(instance_desc);

        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| {
                format!("GPU yüzeyi oluşturulamadı: {e}")
            })?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),

                // wgpu 30 için gerekli alan.
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| {
                format!("GPU bulunamadı: {e}")
            })?;

        let adapter_name = adapter.get_info().name;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor::default()
            )
            .await
            .map_err(|e| {
                format!("GPU cihazı açılamadı: {e}")
            })?;

        let size = window.inner_size();

        let mut surface_config = surface
            .get_default_config(
                &adapter,
                size.width.max(1),
                size.height.max(1),
            )
            .ok_or_else(|| {
                "GPU ekran yüzeyini desteklemiyor.".to_string()
            })?;

        surface_config.present_mode =
            wgpu::PresentMode::AutoVsync;

        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("RUSTBLEND Triangle Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    Cow::Borrowed(TRIANGLE_SHADER)
                ),
            }
        );

        let color_targets = [
            Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })
        ];

        let pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RUSTBLEND Triangle Pipeline"),
                layout: None,

                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },

                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &color_targets,
                }),

                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            }
        );

        Ok(Self {
            instance,
            surface,
            surface_config,
            device,
            queue,
            pipeline,
            window,
            adapter_name,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.surface_config.width = size.width;
        self.surface_config.height = size.height;

        self.surface.configure(
            &self.device,
            &self.surface_config,
        );

        self.window.request_redraw();
    }

    pub fn render(&mut self) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,

            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }

            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(
                    &self.device,
                    &self.surface_config,
                );

                self.window.request_redraw();
                return;
            }

            wgpu::CurrentSurfaceTexture::Lost => {
                match self.instance.create_surface(
                    self.window.clone()
                ) {
                    Ok(surface) => {
                        self.surface = surface;

                        self.surface.configure(
                            &self.device,
                            &self.surface_config,
                        );

                        self.window.request_redraw();
                    }

                    Err(error) => {
                        eprintln!(
                            "GPU yüzeyi yenilenemedi: {error}"
                        );
                    }
                }

                return;
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("GPU yüzey doğrulama hatası.");
                return;
            }
        };

        let view = frame.texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("RUSTBLEND Command Encoder"),
            }
        );

        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("RUSTBLEND Main Render Pass"),

                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,

                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(
                                    wgpu::Color {
                                        r: 0.025,
                                        g: 0.035,
                                        b: 0.065,
                                        a: 1.0,
                                    }
                                ),
                                store: wgpu::StoreOp::Store,
                            },
                        })
                    ],

                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                }
            );

            render_pass.set_pipeline(&self.pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();

        // wgpu 30'da sunum işlemi Queue üzerinden yapılır.
        self.queue.present(frame);
    }
}
