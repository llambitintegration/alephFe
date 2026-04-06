use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::level;
use crate::mesh;
use crate::texture::TextureManager;

/// Camera uniform data sent to GPU.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [f32; 16],
    camera_yaw: f32,
    camera_pitch: f32,
    elapsed_time: f32,
    _padding: f32,
}

/// Per-polygon data in the storage buffer.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PolygonGpuData {
    floor_height: f32,
    ceiling_height: f32,
    floor_light: f32,
    ceiling_light: f32,
    floor_transfer_mode: u32,
    ceiling_transfer_mode: u32,
    media_height: f32,
    media_transfer_mode: u32,
    floor_tex_offset_x: f32,
    floor_tex_offset_y: f32,
    ceiling_tex_offset_x: f32,
    ceiling_tex_offset_y: f32,
}

/// Free-fly camera.
struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    fov: f32,
    near: f32,
    far: f32,
    aspect: f32,
    // Input state
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    speed: f32,
    mouse_sensitivity: f32,
}

impl Camera {
    fn new(aspect: f32) -> Self {
        Camera {
            position: Vec3::new(0.0, 0.5, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            fov: 90.0_f32.to_radians(),
            near: 0.1,
            far: 1000.0,
            aspect,
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            speed: 5.0,
            mouse_sensitivity: 0.003,
        }
    }

    fn direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    fn update(&mut self, dt: f32) {
        let dir = self.direction();
        let right = dir.cross(Vec3::Y).normalize();
        let move_speed = self.speed * dt;

        if self.forward {
            self.position += dir * move_speed;
        }
        if self.backward {
            self.position -= dir * move_speed;
        }
        if self.right {
            self.position += right * move_speed;
        }
        if self.left {
            self.position -= right * move_speed;
        }
        if self.up {
            self.position += Vec3::Y * move_speed;
        }
        if self.down {
            self.position -= Vec3::Y * move_speed;
        }
    }

    fn view_proj(&self) -> Mat4 {
        let dir = self.direction();
        let view = Mat4::look_to_rh(self.position, dir, Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far);
        proj * view
    }

    fn process_mouse(&mut self, dx: f64, dy: f64) {
        self.yaw += dx as f32 * self.mouse_sensitivity;
        self.pitch -= dy as f32 * self.mouse_sensitivity;
        self.pitch = self.pitch.clamp(-1.5, 1.5);
    }

    fn process_key(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.backward = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.up = pressed,
            KeyCode::ShiftLeft => self.down = pressed,
            _ => {}
        }
    }

    fn to_uniform(&self, elapsed: f32) -> CameraUniform {
        let vp = self.view_proj();
        CameraUniform {
            view_proj: vp.to_cols_array(),
            camera_yaw: self.yaw,
            camera_pitch: self.pitch,
            elapsed_time: elapsed,
            _padding: 0.0,
        }
    }
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    polygon_buffer: wgpu::Buffer,
    polygon_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    texture_manager: TextureManager,
    // Fallback texture bind group for surfaces with no loaded collection
    fallback_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuState {
    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        (texture, view)
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            let (dt, dv) = Self::create_depth_texture(&self.device, new_size.width, new_size.height);
            self.depth_texture = dt;
            self.depth_view = dv;
        }
    }

    fn render(&self, camera: &Camera, elapsed: f32) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let uniform = camera.to_uniform(elapsed);
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.polygon_bind_group, &[]);

            // Use first available texture bind group, or fallback
            let tex_bg = self
                .texture_manager
                .gpu_textures
                .values()
                .next()
                .map(|t| &t.bind_group)
                .unwrap_or(&self.fallback_bind_group);
            render_pass.set_bind_group(2, tex_bg, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct App {
    map_path: PathBuf,
    shapes_path: PathBuf,
    gpu: Option<GpuState>,
    camera: Camera,
    start_time: Instant,
    last_frame: Instant,
    current_level: usize,
    map_wad: Option<marathon_formats::WadFile>,
    shapes_file: Option<marathon_formats::ShapesFile>,
    level_count: usize,
    window: Option<Arc<Window>>,
    mouse_captured: bool,
    platform_states: Vec<level::PlatformState>,
}

impl App {
    fn new(map_path: PathBuf, shapes_path: PathBuf) -> Self {
        App {
            map_path,
            shapes_path,
            gpu: None,
            camera: Camera::new(16.0 / 9.0),
            start_time: Instant::now(),
            last_frame: Instant::now(),
            current_level: 0,
            map_wad: None,
            shapes_file: None,
            level_count: 0,
            window: None,
            mouse_captured: false,
            platform_states: Vec::new(),
        }
    }

    fn load_level(&mut self, index: usize) {
        let wad = self.map_wad.as_ref().unwrap();
        let shapes = self.shapes_file.as_ref().unwrap();

        let loaded = match level::load_level(wad, index) {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to load level {index}: {e}");
                return;
            }
        };

        log::info!("Loaded level: {}", loaded.level_name);

        let map = &loaded.map;

        // Build mesh
        let level_mesh = mesh::build_level_mesh(map);
        log::info!(
            "Mesh: {} vertices, {} indices",
            level_mesh.vertices.len(),
            level_mesh.indices.len()
        );

        // Collect texture descriptors and load textures
        let descriptors = level::collect_texture_descriptors(map);
        let mut tex_manager = TextureManager::load_collections(shapes, &descriptors);

        // Build per-polygon storage buffer data
        let polygon_data: Vec<PolygonGpuData> = map
            .polygons
            .iter()
            .map(|poly| {
                let floor_light =
                    level::evaluate_light_intensity(&map.lights, poly.floor_lightsource_index);
                let ceiling_light =
                    level::evaluate_light_intensity(&map.lights, poly.ceiling_lightsource_index);

                let media_height = if poly.media_index >= 0 {
                    map.media
                        .get(poly.media_index as usize)
                        .map(|m| m.height as f32 / 1024.0)
                        .unwrap_or(0.0)
                } else {
                    0.0
                };

                PolygonGpuData {
                    floor_height: poly.floor_height as f32 / 1024.0,
                    ceiling_height: poly.ceiling_height as f32 / 1024.0,
                    floor_light,
                    ceiling_light,
                    floor_transfer_mode: poly.floor_transfer_mode as u32,
                    ceiling_transfer_mode: poly.ceiling_transfer_mode as u32,
                    media_height,
                    media_transfer_mode: if poly.media_index >= 0 {
                        map.media
                            .get(poly.media_index as usize)
                            .map(|m| m.transfer_mode as u32)
                            .unwrap_or(0)
                    } else {
                        0
                    },
                    floor_tex_offset_x: poly.floor_origin.x as f32 / 1024.0,
                    floor_tex_offset_y: poly.floor_origin.y as f32 / 1024.0,
                    ceiling_tex_offset_x: poly.ceiling_origin.x as f32 / 1024.0,
                    ceiling_tex_offset_y: poly.ceiling_origin.y as f32 / 1024.0,
                }
            })
            .collect();

        // Initialize platform states
        self.platform_states = map
            .platforms
            .iter()
            .map(|p| level::PlatformState::from_data(p, map))
            .collect();

        if let Some(gpu) = &mut self.gpu {
            // Upload mesh data
            gpu.vertex_buffer =
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vertex_buffer"),
                        contents: bytemuck::cast_slice(&level_mesh.vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
            gpu.index_buffer =
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("index_buffer"),
                        contents: bytemuck::cast_slice(&level_mesh.indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });
            gpu.num_indices = level_mesh.indices.len() as u32;

            // Upload polygon data
            let poly_buf_data = if polygon_data.is_empty() {
                vec![PolygonGpuData::zeroed()]
            } else {
                polygon_data
            };
            gpu.polygon_buffer =
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("polygon_buffer"),
                        contents: bytemuck::cast_slice(&poly_buf_data),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    });
            gpu.polygon_bind_group =
                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("polygon_bind_group"),
                    layout: &gpu.render_pipeline.get_bind_group_layout(1),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: gpu.polygon_buffer.as_entire_binding(),
                    }],
                });

            // Create GPU textures
            let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("texture_sampler"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                ..Default::default()
            });
            tex_manager.create_gpu_textures(
                &gpu.device,
                &gpu.queue,
                &gpu.texture_bind_group_layout,
                &sampler,
            );
            gpu.texture_manager = tex_manager;

            // Set camera to first polygon center
            if let Some(poly) = map.polygons.first() {
                let cx = poly.center.x as f32 / 1024.0;
                let cy = (poly.floor_height as f32 + 512.0) / 1024.0;
                let cz = poly.center.y as f32 / 1024.0;
                self.camera.position = Vec3::new(cx, cy, cz);
            }
        }

        self.current_level = index;
    }

    fn switch_level(&mut self, direction: i32) {
        if self.level_count == 0 {
            return;
        }
        let next = ((self.current_level as i32 + direction).rem_euclid(self.level_count as i32))
            as usize;
        self.load_level(next);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Marathon Viewer")
            .with_inner_size(PhysicalSize::new(1280u32, 720u32));

        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        // Load WAD files
        let map_wad = marathon_formats::WadFile::open(&self.map_path).unwrap_or_else(|e| {
            eprintln!("Failed to open map WAD: {e}");
            std::process::exit(1);
        });
        let shapes_file =
            marathon_formats::ShapesFile::open(&self.shapes_path).unwrap_or_else(|e| {
                eprintln!("Failed to open shapes file: {e}");
                std::process::exit(1);
            });

        let levels = level::enumerate_levels(&map_wad);
        self.level_count = levels.len();
        log::info!("Found {} levels:", levels.len());
        for l in &levels {
            log::info!("  {}: {}", l.index, l.name);
        }

        self.map_wad = Some(map_wad);
        self.shapes_file = Some(shapes_file);

        // Initialize wgpu
        let size = window.inner_size();
        self.camera.aspect = size.width as f32 / size.height as f32;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let (device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .expect("Failed to find GPU adapter");

            log::info!("Using GPU: {}", adapter.get_info().name);

            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .expect("Failed to create device")
        });

        let surface_format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (depth_texture, depth_view) =
            GpuState::create_depth_texture(&device, config.width, config.height);

        // Shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Camera bind group layout + buffer
        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_uniform = self.camera.to_uniform(0.0);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Polygon storage buffer bind group layout
        let polygon_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("polygon_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let initial_poly_data = vec![PolygonGpuData::zeroed()];
        let polygon_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("polygon_buffer"),
            contents: bytemuck::cast_slice(&initial_poly_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let polygon_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("polygon_bg"),
            layout: &polygon_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: polygon_buffer.as_entire_binding(),
            }],
        });

        // Texture bind group layout
        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Fallback 1x1 white texture for surfaces with no loaded collection
        let fallback_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fallback_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &fallback_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8, 0, 255, 255], // Magenta for debugging
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let fallback_view = fallback_tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let fallback_sampler = device.create_sampler(&Default::default());
        let fallback_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fallback_texture_bg"),
            layout: &texture_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&fallback_sampler),
                },
            ],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&camera_bgl, &polygon_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        // Render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[mesh::Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Empty initial buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: &[0u8; 4],
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: &[0u8; 4],
            usage: wgpu::BufferUsages::INDEX,
        });

        self.gpu = Some(GpuState {
            surface,
            device,
            queue,
            config,
            depth_texture,
            depth_view,
            render_pipeline,
            camera_buffer,
            camera_bind_group,
            polygon_buffer,
            polygon_bind_group,
            vertex_buffer,
            index_buffer,
            num_indices: 0,
            texture_manager: TextureManager {
                collections: Default::default(),
                gpu_textures: Default::default(),
            },
            fallback_bind_group,
            texture_bind_group_layout: texture_bgl,
        });

        self.window = Some(window);

        // Load first level
        if self.level_count > 0 {
            self.load_level(0);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    self.camera.aspect = size.width as f32 / size.height.max(1) as f32;
                    gpu.resize(size);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                self.camera.process_key(key, pressed);

                if pressed {
                    match key {
                        KeyCode::Escape => {
                            if self.mouse_captured {
                                self.mouse_captured = false;
                                if let Some(w) = &self.window {
                                    let _ = w.set_cursor_grab(winit::window::CursorGrabMode::None);
                                    w.set_cursor_visible(true);
                                }
                            } else {
                                event_loop.exit();
                            }
                        }
                        KeyCode::BracketRight => self.switch_level(1),
                        KeyCode::BracketLeft => self.switch_level(-1),
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.mouse_captured {
                    self.mouse_captured = true;
                    if let Some(w) = &self.window {
                        let _ = w.set_cursor_grab(winit::window::CursorGrabMode::Locked)
                            .or_else(|_| w.set_cursor_grab(winit::window::CursorGrabMode::Confined));
                        w.set_cursor_visible(false);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;
                let elapsed = (now - self.start_time).as_secs_f32();

                self.camera.update(dt);

                // Update platform animations
                if let Some(gpu) = &self.gpu {
                    for platform in &mut self.platform_states {
                        let new_height = platform.update(dt);
                        let offset = platform.polygon_index * std::mem::size_of::<PolygonGpuData>();
                        // Write just the floor_height field
                        gpu.queue.write_buffer(
                            &gpu.polygon_buffer,
                            offset as u64,
                            bytemuck::bytes_of(&new_height),
                        );
                    }
                }

                if let Some(gpu) = &self.gpu {
                    match gpu.render(&self.camera, elapsed) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            let size = PhysicalSize::new(gpu.config.width, gpu.config.height);
                            if let Some(gpu) = &mut self.gpu {
                                gpu.resize(size);
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => log::error!("Render error: {e}"),
                    }
                }

                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.mouse_captured {
                self.camera.process_mouse(delta.0, delta.1);
            }
        }
    }
}

pub fn run(map_path: PathBuf, shapes_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(map_path, shapes_path);
    event_loop.run_app(&mut app)?;

    Ok(())
}
