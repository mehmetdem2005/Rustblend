use std::borrow::Cow;
use std::sync::Arc;

use crate::{
    camera::Camera,
    geometry::Mesh,
    math::{Quaternion, Vec3},
    render::RenderPacket,
    scene::{Scene, Transform},
    viewport_ui::{self, ToolMode, UiAction},
};

use winit::{
    dpi::PhysicalSize,
    event_loop::OwnedDisplayHandle,
    window::Window,
};

const MESH_SHADER: &str = r#"
struct CameraUniform {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_projection * vec4<f32>(input.position, 1.0);
    output.normal = input.normal;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_direction = normalize(vec3<f32>(-0.45, 0.82, 0.36));
    let diffuse = max(dot(normal, light_direction), 0.0);
    let lighting = 0.28 + diffuse * 0.72;
    return vec4<f32>(input.color * lighting, 1.0);
}
"#;

const LINE_SHADER: &str = r#"
struct CameraUniform {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_projection * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

const UI_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

const VERTEX_STRIDE: u64 = 9 * 4;
const UI_VERTEX_STRIDE: u64 = 5 * 4;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

const MESH_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

const UI_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3];

const LINE_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 24,
        shader_location: 2,
    },
];

pub struct GpuRenderer {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    device: wgpu::Device,
    queue: wgpu::Queue,

    mesh_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    ui_pipeline: Option<wgpu::RenderPipeline>,

    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    mesh_vertex_buffer: wgpu::Buffer,
    mesh_index_buffer: wgpu::Buffer,
    mesh_index_count: u32,

    line_vertex_buffer: wgpu::Buffer,
    line_vertex_count: u32,

    ui_vertex_buffer: wgpu::Buffer,
    ui_vertex_count: u32,

    tool_mode: ToolMode,
    edit_mode: bool,
    outliner_open: bool,

    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    scene: Scene,
    history: crate::history::History,
    window: Arc<Window>,
    adapter_name: String,
    first_frame_presented: bool,
    recovery_status: &'static str,
}

impl GpuRenderer {
    pub async fn new(
        display: OwnedDisplayHandle,
        window: Arc<Window>,
    ) -> Result<Self, String> {
        eprintln!("RUSTBLEND GPU: creating Vulkan + GLES instance.");

        let mut instance_desc =
            wgpu::InstanceDescriptor::new_with_display_handle(
                Box::new(display)
            );

        instance_desc.backends =
            wgpu::Backends::VULKAN | wgpu::Backends::GL;

        let instance = wgpu::Instance::new(instance_desc);

        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| {
                format!("GPU surface creation failed: {error}")
            })?;

        eprintln!("RUSTBLEND GPU: requesting compatible adapter.");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| {
                format!("No compatible GPU adapter found: {error}")
            })?;

        let adapter_info = adapter.get_info();
        let adapter_name = format!(
            "{} ({:?})",
            adapter_info.name,
            adapter_info.backend
        );

        eprintln!("RUSTBLEND GPU: selected adapter {adapter_name}.");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|error| {
                format!("GPU device creation failed: {error}")
            })?;

        device.on_uncaptured_error(Arc::new(|error| {
            eprintln!("RUSTBLEND GPU UNCAPTURED ERROR: {error}");
        }));

        eprintln!("RUSTBLEND GPU: device ready; uncaptured GPU errors will be logged.");

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let mut surface_config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| {
                "The selected GPU does not support the Android surface.".to_string()
            })?;

        surface_config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &surface_config);

        let mut camera = Camera::new(
            Vec3::new(0.0, 1.0, 0.0),
            6.2,
            width as f32 / height as f32,
        )
        .map_err(|error| format!("Camera creation failed: {error:?}"))?;

        camera
            .orbit(0.72, 0.50)
            .map_err(|error| format!("Camera setup failed: {error:?}"))?;

        let camera_bytes = f32_bytes(&camera.view_projection_matrix().elements);
        let camera_buffer = buffer_from_bytes(
            &device,
            "RUSTBLEND Camera Uniform",
            &camera_bytes,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let camera_bind_group_layout =
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("RUSTBLEND Camera Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }
                    ],
                }
            );

        let camera_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("RUSTBLEND Camera Bind Group"),
                layout: &camera_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    }
                ],
            }
        );

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("RUSTBLEND Viewport Pipeline Layout"),
                bind_group_layouts: &[Some(&camera_bind_group_layout)],
                immediate_size: 0,
            }
        );

        let mesh_shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("RUSTBLEND Mesh Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    Cow::Borrowed(MESH_SHADER)
                ),
            }
        );

        let line_shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("RUSTBLEND Grid and Edge Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    Cow::Borrowed(LINE_SHADER)
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

        let mesh_layout = [
            wgpu::VertexBufferLayout {
                array_stride: VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &MESH_ATTRIBUTES,
            }
        ];

        let line_layout = [
            wgpu::VertexBufferLayout {
                array_stride: VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &LINE_ATTRIBUTES,
            }
        ];

        let mesh_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RUSTBLEND 3D Mesh Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(mesh_layout[0].clone())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &color_targets,
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            }
        );

        let line_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RUSTBLEND Grid and Selection Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &line_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(line_layout[0].clone())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &line_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &color_targets,
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 1,
                        slope_scale: 1.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            }
        );

        eprintln!("RUSTBLEND GPU: compiling workspace UI shader.");
        let ui_validation_scope =
            device.push_error_scope(wgpu::ErrorFilter::Validation);

        let ui_shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("RUSTBLEND Workspace UI Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    Cow::Borrowed(UI_SHADER)
                ),
            }
        );

        let ui_layout = [
            wgpu::VertexBufferLayout {
                array_stride: UI_VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &UI_ATTRIBUTES,
            }
        ];

        let ui_pipeline_candidate = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RUSTBLEND Workspace UI Pipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &ui_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(ui_layout[0].clone())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ui_shader,
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

        let ui_pipeline = match ui_validation_scope.pop().await {
            Some(error) => {
                eprintln!("RUSTBLEND GPU: workspace UI pipeline disabled: {error}");
                None
            }
            None => {
                eprintln!("RUSTBLEND GPU: workspace UI pipeline ready.");
                Some(ui_pipeline_candidate)
            }
        };

        let tool_mode = ToolMode::Orbit;
        let edit_mode = false;
        let outliner_open = true;

        let mut scene = Scene::new();
        let cube = Mesh::cube(2.0)
            .map_err(|error| format!("Default cube creation failed: {error:?}"))?;

        let cube_transform = Transform::new(
            Vec3::new(0.0, 1.0, 0.0),
            Quaternion::IDENTITY,
            Vec3::new(1.0, 1.0, 1.0),
        )
        .map_err(|error| format!("Default cube transform failed: {error:?}"))?;

        let cube_id = scene
            .add_object_with_transform("Cube", cube, cube_transform)
            .map_err(|error| format!("Default scene creation failed: {error:?}"))?;

        scene
            .select(cube_id)
            .map_err(|error| format!("Default cube selection failed: {error:?}"))?;

        let packet = RenderPacket::from_scene(&scene)
            .map_err(|error| format!("Default scene render conversion failed: {error:?}"))?;

        let (mesh_vertices, mesh_indices) = build_mesh_buffers(&packet)?;
        let mesh_vertex_buffer = buffer_from_bytes(
            &device,
            "RUSTBLEND Cube Vertex Buffer",
            &f32_bytes(&mesh_vertices),
            wgpu::BufferUsages::VERTEX,
        );
        let mesh_index_buffer = buffer_from_bytes(
            &device,
            "RUSTBLEND Cube Index Buffer",
            &u32_bytes(&mesh_indices),
            wgpu::BufferUsages::INDEX,
        );

        let (line_vertices, grid_vertex_count) = build_grid_and_edges(&scene, edit_mode)?;
        let line_vertex_buffer = buffer_from_bytes(
            &device,
            "RUSTBLEND Grid and Cube Edges",
            &f32_bytes(&line_vertices),
            wgpu::BufferUsages::VERTEX,
        );

        let ui_vertices = viewport_ui::build_vertices(
            width,
            height,
            &scene,
            tool_mode,
            edit_mode,
            outliner_open,
            "NOT SAVED",
        );
        let ui_vertex_buffer = buffer_from_bytes(
            &device,
            "RUSTBLEND Workspace UI Vertex Buffer",
            &f32_bytes(&ui_vertices),
            wgpu::BufferUsages::VERTEX,
        );

        let (depth_texture, depth_view) =
            create_depth_resources(&device, width, height);

        eprintln!(
            "RUSTBLEND GPU: startup resources ready; ui_enabled={}.",
            ui_pipeline.is_some()
        );

        eprintln!(
            "RUSTBLEND GPU: 3D viewport ready. objects={}, triangles={}, grid_vertices={}.",
            packet.object_count(),
            mesh_indices.len() / 3,
            grid_vertex_count
        );

        Ok(Self {
            instance,
            surface,
            surface_config,
            device,
            queue,
            mesh_pipeline,
            line_pipeline,
            ui_pipeline,
            camera,
            camera_buffer,
            camera_bind_group,
            mesh_vertex_buffer,
            mesh_index_buffer,
            mesh_index_count: u32::try_from(mesh_indices.len())
                .map_err(|_| "Too many mesh indices.".to_string())?,
            line_vertex_buffer,
            line_vertex_count: u32::try_from(line_vertices.len() / 9)
                .map_err(|_| "Too many viewport line vertices.".to_string())?,
            ui_vertex_buffer,
            ui_vertex_count: u32::try_from(ui_vertices.len() / 5)
                .map_err(|_| "Too many UI vertices.".to_string())?,
            tool_mode,
            edit_mode,
            outliner_open,
            depth_texture,
            depth_view,
            scene,
            history: Default::default(),
            window,
            adapter_name,
            first_frame_presented: false,
            recovery_status: "NOT SAVED",
        })
    }

    /// Move CPU state out before Android releases the GPU surface.
    pub fn into_workspace(mut self) -> crate::workspace::WorkspaceState {
        self.history.commit(&self.scene);
        crate::workspace::WorkspaceState {
            history: self.history,
            scene: self.scene, camera: self.camera, tool: self.tool_mode,
            edit_mode: self.edit_mode, outliner_open: self.outliner_open,
        }
    }

    /// Keep the caller's snapshot intact if GPU reconstruction fails.
    pub fn restore_workspace(&mut self, saved: &crate::workspace::WorkspaceState) -> Result<(), String> {
        self.scene = saved.scene.clone();
        self.history = saved.history.clone();
        self.camera = saved.camera;
        self.camera.set_viewport(self.surface_config.width, self.surface_config.height)
            .map_err(|error| format!("Workspace camera restore failed: {error:?}"))?;
        self.tool_mode = saved.tool;
        self.edit_mode = saved.edit_mode;
        self.outliner_open = saved.outliner_open;
        self.update_camera_uniform();
        self.refresh_scene_buffers()?;
        self.refresh_ui_buffer()?;
        Ok(())
    }

    pub fn recovery_snapshot(&self) -> crate::workspace::WorkspaceState {
        crate::workspace::WorkspaceState {
            scene: self.scene.clone(), camera: self.camera, tool: self.tool_mode,
            edit_mode: self.edit_mode, outliner_open: self.outliner_open,
            history: Default::default(),
        }
    }

    pub fn set_recovery_status(&mut self, status: &'static str) {
        self.recovery_status = status;
        if let Err(error) = self.refresh_ui_buffer() {
            eprintln!("RUSTBLEND: recovery status UI failed: {error}");
        }
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn scene_object_count(&self) -> usize {
        self.scene.object_count()
    }

    fn refresh_scene_buffers(&mut self) -> Result<(), String> {
        let packet = RenderPacket::from_scene(&self.scene)
            .map_err(|error| format!("Scene render conversion failed: {error:?}"))?;

        let (mesh_vertices, mesh_indices, mesh_index_count) = if packet.is_empty() {
            (vec![0.0_f32; 9], vec![0_u32], 0_u32)
        } else {
            let (vertices, indices) = build_mesh_buffers(&packet)?;
            let count = u32::try_from(indices.len())
                .map_err(|_| "Too many scene indices.".to_string())?;
            (vertices, indices, count)
        };

        let (line_vertices, _) = build_grid_and_edges(&self.scene, self.edit_mode)?;

        self.mesh_vertex_buffer = buffer_from_bytes(
            &self.device,
            "RUSTBLEND Dynamic Scene Vertex Buffer",
            &f32_bytes(&mesh_vertices),
            wgpu::BufferUsages::VERTEX,
        );
        self.mesh_index_buffer = buffer_from_bytes(
            &self.device,
            "RUSTBLEND Dynamic Scene Index Buffer",
            &u32_bytes(&mesh_indices),
            wgpu::BufferUsages::INDEX,
        );
        self.mesh_index_count = mesh_index_count;

        self.line_vertex_buffer = buffer_from_bytes(
            &self.device,
            "RUSTBLEND Dynamic Grid and Edge Buffer",
            &f32_bytes(&line_vertices),
            wgpu::BufferUsages::VERTEX,
        );
        self.line_vertex_count = u32::try_from(line_vertices.len() / 9)
            .map_err(|_| "Too many viewport line vertices.".to_string())?;

        Ok(())
    }

    fn refresh_ui_buffer(&mut self) -> Result<(), String> {
        let vertices = viewport_ui::build_vertices(
            self.surface_config.width,
            self.surface_config.height,
            &self.scene,
            self.tool_mode,
            self.edit_mode,
            self.outliner_open,
            self.recovery_status,
        );

        if vertices.is_empty() || vertices.len() % 5 != 0 {
            return Err("Workspace UI generated invalid vertex data.".to_string());
        }

        self.ui_vertex_count = u32::try_from(vertices.len() / 5)
            .map_err(|_| "Too many workspace UI vertices.".to_string())?;
        self.ui_vertex_buffer = buffer_from_bytes(
            &self.device,
            "RUSTBLEND Workspace UI Vertex Buffer",
            &f32_bytes(&vertices),
            wgpu::BufferUsages::VERTEX,
        );

        Ok(())
    }

    fn refresh_workspace(&mut self, refresh_scene: bool) {
        if refresh_scene {
            if let Err(error) = self.refresh_scene_buffers() {
                eprintln!("RUSTBLEND: scene GPU refresh failed: {error}");
            }
        }

        if let Err(error) = self.refresh_ui_buffer() {
            eprintln!("RUSTBLEND: UI refresh failed: {error}");
        }

        self.window.request_redraw();
    }

    fn add_cube(&mut self) -> Result<(), String> {
        let index = self.scene.object_count();
        let column = (index % 4) as f32;
        let row = (index / 4) as f32;
        let position = Vec3::new(
            (column - 1.5) * 1.8,
            0.75,
            row * 1.8,
        );

        let mesh = Mesh::cube(1.5)
            .map_err(|error| format!("Cube creation failed: {error:?}"))?;
        let transform = Transform::new(
            position,
            Quaternion::IDENTITY,
            Vec3::new(1.0, 1.0, 1.0),
        )
        .map_err(|error| format!("Cube transform failed: {error:?}"))?;

        let name = format!("Cube {}", index + 1);
        let id = self.scene
            .add_object_with_transform(name, mesh, transform)
            .map_err(|error| format!("Scene rejected cube: {error:?}"))?;
        self.scene.select(id)
            .map_err(|error| format!("Cube selection failed: {error:?}"))?;

        Ok(())
    }

    fn duplicate_active(&mut self) -> Result<(), String> {
        self.scene
            .duplicate_active(Vec3::new(1.6, 0.0, 0.0))
            .map_err(|error| format!("Duplicate failed: {error:?}"))?;
        Ok(())
    }

    fn delete_active(&mut self) -> bool {
        let Some(id) = self.scene.active_object_id() else {
            return false;
        };

        if self.scene.remove(id).is_none() {
            return false;
        }

        if let Some(next_id) = self.scene.objects().last().map(|object| object.id()) {
            let _ = self.scene.select(next_id);
        }

        true
    }

    pub fn is_ui_position(&self, x: f32, y: f32) -> bool {
        viewport_ui::is_ui_position(
            self.surface_config.width,
            self.surface_config.height,
            x,
            y,
            self.outliner_open,
        )
    }

    pub fn handle_ui_tap(&mut self, x: f32, y: f32) -> bool {
        let Some(action) = viewport_ui::hit_test(
            self.surface_config.width,
            self.surface_config.height,
            x,
            y,
            &self.scene,
            self.outliner_open,
        ) else {
            return false;
        };

        let edits_scene = matches!(action, UiAction::AddCube | UiAction::Duplicate | UiAction::Delete);
        if edits_scene { self.history.begin(&self.scene); }
        match action {
            UiAction::Undo => {
                self.history.undo(&mut self.scene);
                self.refresh_workspace(true);
            }
            UiAction::Redo => {
                self.history.redo(&mut self.scene);
                self.refresh_workspace(true);
            }
            UiAction::AddCube => {
                if let Err(error) = self.add_cube() {
                    eprintln!("RUSTBLEND: {error}");
                }
                self.refresh_workspace(true);
            }

            UiAction::Duplicate => {
                if let Err(error) = self.duplicate_active() {
                    eprintln!("RUSTBLEND: {error}");
                }
                self.refresh_workspace(true);
            }

            UiAction::Delete => {
                self.delete_active();
                self.refresh_workspace(true);
            }

            UiAction::SetTool(tool) => {
                self.tool_mode = if self.tool_mode == tool {
                    ToolMode::Orbit
                } else {
                    tool
                };
                self.refresh_workspace(false);
            }

            UiAction::ToggleEdit => {
                self.edit_mode = !self.edit_mode;
                self.refresh_workspace(true);
            }

            UiAction::ToggleOutliner => {
                self.outliner_open = !self.outliner_open;
                self.refresh_workspace(false);
            }

            UiAction::SelectObject(index) => {
                if let Some(id) = self.scene.objects().get(index).map(|object| object.id()) {
                    let _ = self.scene.select(id);
                    self.refresh_workspace(true);
                }
            }

            UiAction::Ignore => {
                return true;
            }
        }

        if edits_scene { self.history.commit(&self.scene); }
        true
    }

    pub fn finish_drag(&mut self, cancel: bool) {
        if cancel {
            self.history.cancel(&mut self.scene);
            self.refresh_workspace(true);
        } else {
            self.history.commit(&self.scene);
        }
    }

    pub fn drag_active(&mut self, delta_x: f32, delta_y: f32) {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return;
        }

        if self.tool_mode != ToolMode::Orbit {
            self.history.begin(&self.scene);
        }
        match self.tool_mode {
            ToolMode::Orbit => {
                self.orbit_camera(-delta_x * 0.006, -delta_y * 0.006);
                return;
            }

            ToolMode::Move => {
                let forward = (self.camera.target() - self.camera.position())
                    .normalized()
                    .unwrap_or(Vec3::new(0.0, 0.0, -1.0));
                let right = forward.cross(Vec3::new(0.0, 1.0, 0.0))
                    .normalized()
                    .unwrap_or(Vec3::new(1.0, 0.0, 0.0));
                let up = right.cross(forward)
                    .normalized()
                    .unwrap_or(Vec3::new(0.0, 1.0, 0.0));
                let pixels_to_world = self.camera.distance()
                    / self.surface_config.height.max(1) as f32
                    * 1.8;
                let delta = right * (delta_x * pixels_to_world)
                    + up * (-delta_y * pixels_to_world);

                if let Err(error) = self.scene.translate_active(delta) {
                    eprintln!("RUSTBLEND: move failed: {error:?}");
                    return;
                }
            }

            ToolMode::Rotate => {
                if let Err(error) = self.scene.rotate_active(
                    Vec3::new(0.0, 1.0, 0.0),
                    delta_x * 0.009,
                ) {
                    eprintln!("RUSTBLEND: rotate failed: {error:?}");
                    return;
                }

                let forward = (self.camera.target() - self.camera.position())
                    .normalized()
                    .unwrap_or(Vec3::new(0.0, 0.0, -1.0));
                let right = forward.cross(Vec3::new(0.0, 1.0, 0.0))
                    .normalized()
                    .unwrap_or(Vec3::new(1.0, 0.0, 0.0));

                if let Err(error) = self.scene.rotate_active(right, delta_y * 0.009) {
                    eprintln!("RUSTBLEND: rotate failed: {error:?}");
                    return;
                }
            }

            ToolMode::Scale => {
                let factor = (-delta_y * 0.012).exp().clamp(0.65, 1.55);
                if let Err(error) = self.scene.scale_active(factor) {
                    eprintln!("RUSTBLEND: scale failed: {error:?}");
                    return;
                }
            }
        }

        self.refresh_workspace(true);
    }

    pub fn orbit_camera(
        &mut self,
        delta_yaw: f32,
        delta_pitch: f32,
    ) {
        if self.camera.orbit(delta_yaw, delta_pitch).is_ok() {
            self.update_camera_uniform();
            self.window.request_redraw();
        }
    }

    pub fn zoom_camera(&mut self, factor: f32) {
        if self.camera.zoom(factor).is_ok() {
            self.update_camera_uniform();
            self.window.request_redraw();
        }
    }

    fn update_camera_uniform(&mut self) {
        let bytes = f32_bytes(
            &self.camera.view_projection_matrix().elements
        );
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            &bytes,
        );
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.surface_config.width = size.width;
        self.surface_config.height = size.height;

        self.camera
            .set_viewport(size.width, size.height)
            .ok();

        self.surface.configure(
            &self.device,
            &self.surface_config,
        );

        let (depth_texture, depth_view) =
            create_depth_resources(&self.device, size.width, size.height);

        self.depth_texture = depth_texture;
        self.depth_view = depth_view;

        self.update_camera_uniform();
        self.refresh_workspace(false);
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
                            "RUSTBLEND GPU: surface recreation failed: {error}"
                        );
                    }
                }

                return;
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("RUSTBLEND GPU: surface validation failed.");
                return;
            }
        };

        let view = frame.texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("RUSTBLEND Viewport Command Encoder"),
            }
        );

        {
            let mut pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("RUSTBLEND 3D Viewport"),
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(
                                    wgpu::Color {
                                        r: 0.035,
                                        g: 0.045,
                                        b: 0.065,
                                        a: 1.0,
                                    }
                                ),
                                store: wgpu::StoreOp::Store,
                            },
                        })
                    ],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachment {
                            view: &self.depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }
                    ),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                }
            );

            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            pass.draw(0..self.line_vertex_count, 0..1);

            pass.set_pipeline(&self.mesh_pipeline);
            pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
            pass.set_index_buffer(
                self.mesh_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.mesh_index_count, 0, 0..1);
        }

        if let Some(ui_pipeline) = self.ui_pipeline.as_ref() {
            let mut ui_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("RUSTBLEND Workspace UI Overlay"),
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
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

            ui_pass.set_pipeline(ui_pipeline);
            ui_pass.set_vertex_buffer(0, self.ui_vertex_buffer.slice(..));
            ui_pass.draw(0..self.ui_vertex_count, 0..1);
        }

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);

        if !self.first_frame_presented {
            eprintln!("RUSTBLEND GPU: first 3D viewport frame presented.");
            self.first_frame_presented = true;
        }
    }
}

fn build_mesh_buffers(
    packet: &RenderPacket,
) -> Result<(Vec<f32>, Vec<u32>), String> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for object in packet.objects() {
        let base = u32::try_from(vertices.len() / 9)
            .map_err(|_| "Mesh vertex count exceeds u32.".to_string())?;

        let model = object.model_matrix();

        for vertex in object.vertices() {
            let local_position = Vec3::new(
                vertex.position[0],
                vertex.position[1],
                vertex.position[2],
            );

            let world_position = model
                .transform_point(local_position)
                .ok_or_else(|| "Mesh contains an invalid transformed vertex.".to_string())?;

            let local_normal = Vec3::new(
                vertex.normal[0],
                vertex.normal[1],
                vertex.normal[2],
            );

            let world_normal = model
                .transform_direction(local_normal)
                .normalized()
                .unwrap_or(Vec3::new(0.0, 1.0, 0.0));

            push_vertex(
                &mut vertices,
                world_position,
                world_normal,
                [0.68, 0.73, 0.82],
            );
        }

        for &index in object.indices() {
            indices.push(
                base.checked_add(index)
                    .ok_or_else(|| "Mesh index overflow.".to_string())?
            );
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err("Scene has no drawable mesh geometry.".to_string());
    }

    Ok((vertices, indices))
}

fn build_grid_and_edges(scene: &Scene, edit_mode: bool) -> Result<(Vec<f32>, u32), String> {
    let mut vertices = Vec::new();
    let extent = 12.0_f32;

    for step in -12..=12 {
        if step == 0 {
            continue;
        }

        let value = step as f32;
        let color = if step % 5 == 0 {
            [0.25, 0.30, 0.37]
        } else {
            [0.13, 0.16, 0.21]
        };

        push_line(
            &mut vertices,
            Vec3::new(-extent, 0.0, value),
            Vec3::new(extent, 0.0, value),
            color,
        );

        push_line(
            &mut vertices,
            Vec3::new(value, 0.0, -extent),
            Vec3::new(value, 0.0, extent),
            color,
        );
    }

    // World axes: X red, Y green, Z blue.
    push_line(
        &mut vertices,
        Vec3::new(-extent, 0.012, 0.0),
        Vec3::new(extent, 0.012, 0.0),
        [0.92, 0.25, 0.22],
    );

    push_line(
        &mut vertices,
        Vec3::new(0.0, 0.012, -extent),
        Vec3::new(0.0, 0.012, extent),
        [0.24, 0.52, 0.96],
    );

    push_line(
        &mut vertices,
        Vec3::new(0.0, 0.012, 0.0),
        Vec3::new(0.0, 3.2, 0.0),
        [0.24, 0.78, 0.42],
    );

    let grid_vertex_count = u32::try_from(vertices.len() / 9)
        .map_err(|_| "Grid vertex count exceeds u32.".to_string())?;

    for object in scene.objects() {
        let model = object.model_matrix();
        let edge_color = if scene.is_selected(object.id()) {
            [1.0, 0.48, 0.12]
        } else {
            [0.48, 0.55, 0.66]
        };

        for edge in object.mesh().edges() {
            let [a, b] = edge.vertices();
            let start = model
                .transform_point(object.mesh().vertices()[a])
                .ok_or_else(|| "Invalid first mesh edge vertex.".to_string())?;
            let end = model
                .transform_point(object.mesh().vertices()[b])
                .ok_or_else(|| "Invalid second mesh edge vertex.".to_string())?;

            push_line(
                &mut vertices,
                start,
                end,
                edge_color,
            );
        }

        if edit_mode {
            for local_vertex in object.mesh().vertices() {
                let point = model
                    .transform_point(*local_vertex)
                    .ok_or_else(|| "Invalid edit-mode vertex.".to_string())?;
                let radius = 0.045;
                let marker = [1.0, 0.76, 0.24];

                push_line(
                    &mut vertices,
                    point + Vec3::new(-radius, 0.0, 0.0),
                    point + Vec3::new(radius, 0.0, 0.0),
                    marker,
                );
                push_line(
                    &mut vertices,
                    point + Vec3::new(0.0, -radius, 0.0),
                    point + Vec3::new(0.0, radius, 0.0),
                    marker,
                );
                push_line(
                    &mut vertices,
                    point + Vec3::new(0.0, 0.0, -radius),
                    point + Vec3::new(0.0, 0.0, radius),
                    marker,
                );
            }
        }
    }

    Ok((vertices, grid_vertex_count))
}

fn push_line(
    output: &mut Vec<f32>,
    start: Vec3,
    end: Vec3,
    color: [f32; 3],
) {
    let line_normal = Vec3::new(0.0, 1.0, 0.0);
    push_vertex(output, start, line_normal, color);
    push_vertex(output, end, line_normal, color);
}

fn push_vertex(
    output: &mut Vec<f32>,
    position: Vec3,
    normal: Vec3,
    color: [f32; 3],
) {
    output.extend_from_slice(&[
        position.x,
        position.y,
        position.z,
        normal.x,
        normal.y,
        normal.z,
        color[0],
        color[1],
        color[2],
    ]);
}

fn f32_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);

    for value in values {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }

    bytes
}

fn u32_bytes(values: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);

    for value in values {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }

    bytes
}

fn buffer_from_bytes(
    device: &wgpu::Device,
    label: &str,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    assert!(!bytes.is_empty(), "GPU buffers must not be empty.");

    let buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some(label),
            size: bytes.len() as u64,
            usage,
            mapped_at_creation: true,
        }
    );

    {
        let mut mapped = buffer.slice(..).get_mapped_range_mut().expect("Fresh GPU buffer must map successfully.");
        mapped.copy_from_slice(bytes);
    }

    buffer.unmap();
    buffer
}

fn create_depth_resources(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(
        &wgpu::TextureDescriptor {
            label: Some("RUSTBLEND Viewport Depth Buffer"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }
    );

    let view = texture.create_view(
        &wgpu::TextureViewDescriptor::default()
    );

    (texture, view)
}

