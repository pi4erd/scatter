mod camera;
mod mesh;
mod texture;

use std::{collections::HashMap, sync::Arc};

use bytemuck::{Pod, Zeroable};
use camera::{Camera, CameraController};
use mesh::{Mesh, Vertex};
use pollster::FutureExt;
use texture::Texture;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, event::WindowEvent, keyboard::KeyCode, window::Window};

use crate::window::Game;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GameInfo {
    resolution: [u32; 2],
    time: f32,
    delta_time: f32,
}

#[allow(dead_code)]
pub struct MyGame<'s> {
    window: Arc<Window>,
    surface: wgpu::Surface<'s>,
    surface_config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    bind_groups: HashMap<String, wgpu::BindGroup>,
    uniform_buffers: Vec<wgpu::Buffer>,

    start_time: std::time::Instant,
    prev_time: f32,

    depth_texture: Texture,
    pipelines: Vec<wgpu::RenderPipeline>,
    meshes: Vec<Mesh>,

    camera: Camera,
    camera_controller: CameraController,
}

impl MyGame<'_> {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            flags: wgpu::InstanceFlags::VALIDATION,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to create an adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .block_on()
            .expect("Failed to create device");

        let device_info = adapter.get_info();

        log::info!(
            "Chosen device {} ({:?}) with driver {}.",
            device_info.name,
            device_info.device_type,
            device_info.driver
        );

        let surface_caps = surface.get_capabilities(&adapter);

        const PREFERRED_PRESENT_MODE: wgpu::PresentMode = wgpu::PresentMode::Fifo;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps
                .formats
                .into_iter()
                .filter(|s| s.is_srgb())
                .next()
                .expect("sRGB format must be available"),
            width: size.width,
            height: size.height,
            present_mode: surface_caps
                .present_modes
                .into_iter()
                .find(|p| *p == PREFERRED_PRESENT_MODE)
                .unwrap_or(wgpu::PresentMode::Fifo),
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let camera = Camera::new();
        let camera_controller = CameraController::new(5.0, 0.003);

        let uniform_buffers = Self::create_uniform_buffers(&device, &camera, size);

        let (bind_group_layouts, bind_groups) = Self::create_bind_groups(&device, &uniform_buffers);

        let pipelines = Self::create_pipelines(&device, &surface_config, &bind_group_layouts);
        let meshes = Self::create_meshes(&device);

        let depth_texture =
            Texture::create_depth_texture(&device, &surface_config, Some("depth_texture"));

        window.set_cursor_visible(false);

        window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .unwrap_or_else(|_| {
                _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined)
            });

        Self {
            window,
            surface,
            surface_config,
            adapter,
            device,
            queue,

            bind_groups,
            uniform_buffers,

            start_time: std::time::Instant::now(),
            prev_time: 0.0,

            depth_texture,
            pipelines,
            meshes,

            camera,
            camera_controller,
        }
    }

    fn create_uniform_buffers(
        device: &wgpu::Device,
        camera: &Camera,
        size: PhysicalSize<u32>,
    ) -> Vec<wgpu::Buffer> {
        let game_info = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("game_info"),
            contents: bytemuck::cast_slice(&[GameInfo {
                resolution: [size.width, size.height],
                time: 0.0,
                delta_time: 0.01,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera"),
            contents: bytemuck::cast_slice(&[camera.uniform()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        return vec![game_info, camera];
    }

    fn update_uniform_buffers(&mut self) {
        // Update gameinfo buffer
        let time = (std::time::Instant::now() - self.start_time).as_secs_f32();
        let delta_time = time - self.prev_time;
        self.prev_time = time;

        let size = self.window.inner_size();

        let game_info = GameInfo {
            resolution: [size.width, size.height],
            time,
            delta_time,
        };

        self.queue.write_buffer(
            &self.uniform_buffers[0],
            0,
            bytemuck::cast_slice(&[game_info]),
        );
        self.queue.write_buffer(
            &self.uniform_buffers[1],
            0,
            bytemuck::cast_slice(&[self.camera.uniform()]),
        );
    }

    fn create_bind_groups(
        device: &wgpu::Device,
        uniform_buffers: &[wgpu::Buffer],
    ) -> (
        HashMap<String, wgpu::BindGroupLayout>,
        HashMap<String, wgpu::BindGroup>,
    ) {
        let game_info_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("game_info_bind_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let game_info_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game_info_bind_group"),
            layout: &game_info_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffers[0].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffers[1].as_entire_binding(),
                },
            ],
        });

        let (mut layouts, mut groups) = (
            HashMap::<String, wgpu::BindGroupLayout>::new(),
            HashMap::<String, wgpu::BindGroup>::new(),
        );

        layouts.insert("game_info".to_string(), game_info_bind_layout);
        groups.insert("game_info".to_string(), game_info_bind_group);

        return (layouts, groups);
    }

    #[allow(dead_code)]
    fn create_screen_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> Texture {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screen_texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Texture {
            texture,
            view,
            sampler,
        }
    }

    fn create_meshes(device: &wgpu::Device) -> Vec<Mesh> {
        let my_vertices = [
            Vertex {
                position: [-1.0, -1.0, 0.0],
                uv: [0.0, 0.0],
            },
            Vertex {
                position: [1.0, -1.0, 0.0],
                uv: [1.0, 0.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                uv: [1.0, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0, 0.0],
                uv: [0.0, 1.0],
            },
        ];
        let indices = [0, 1, 2, 0, 2, 3];

        let test_mesh = Mesh::create(device, &my_vertices, &indices);

        return vec![test_mesh];
    }

    fn create_pipelines(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        bind_group_layouts: &HashMap<String, wgpu::BindGroupLayout>,
    ) -> Vec<wgpu::RenderPipeline> {
        let _diffuse_module =
            device.create_shader_module(wgpu::include_wgsl!("shaders/diffuse.wgsl"));
        let scatter_module =
            device.create_shader_module(wgpu::include_wgsl!("shaders/scatter.wgsl"));

        // For pipelines that require access to camera features and model matrix
        // Used in opaque and transparent passes
        let world_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world_layout"),
            bind_group_layouts: &[&bind_group_layouts["game_info"]],
            push_constant_ranges: &[],
        });

        let scatter_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scatter_pipeline"),
            layout: Some(&world_layout),
            vertex: wgpu::VertexState {
                module: &scatter_module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &scatter_module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multiview: None,
            cache: None,
        });

        return vec![scatter_pipeline];
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);

        self.depth_texture = Texture::create_depth_texture(
            &self.device,
            &self.surface_config,
            Some("depth_texture"),
        );
        // self.screen_texture = Self::create_screen_texture(&self.device, &self.surface_config);
    }

    fn update(&mut self, delta: f32) {
        self.camera_controller.update(&mut self.camera, delta);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let time = (std::time::Instant::now() - self.start_time).as_secs_f32();
        let delta = time - self.prev_time;
        self.update(delta);
        self.update_uniform_buffers();
        self.prev_time = time;

        let image = self.surface.get_current_texture()?;

        let view = image
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut opaque_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("opaque_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            opaque_pass.set_pipeline(&self.pipelines[0]);

            opaque_pass.set_bind_group(0, self.bind_groups.get("game_info"), &[]);

            self.meshes[0].draw(&mut opaque_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        image.present();

        Ok(())
    }
}

impl Game for MyGame<'_> {
    fn init(window: Arc<Window>) -> Self {
        Self::new(window).block_on()
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.camera_controller.process_window_events(&event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => match self.render() {
                Ok(()) => {}
                Err(wgpu::SurfaceError::Lost) => todo!("Surface error"),
                Err(e) => panic!("Error while trying to render: {e}"),
            },
            WindowEvent::Resized(new_size) => {
                self.resize(new_size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == KeyCode::Escape && event.state.is_pressed() {
                    event_loop.exit();
                }

                if event.physical_key == KeyCode::KeyF && event.state.is_pressed() {
                    self.window.set_fullscreen(match self.window.fullscreen() {
                        Some(_) => None,
                        None => Some(winit::window::Fullscreen::Borderless(None))
                    });
                }
            }
            _ => {

            }
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.camera_controller.process_device_events(&event);
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window.request_redraw();
    }
}
