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

use marathon_audio::{AudioConfig, AudioEngine, AudioEvent, ListenerState};
use marathon_formats::{PhysicsData, ShapesFile, SoundsFile, WadFile};
use marathon_sim::tick::ActionFlags;
use marathon_sim::world::{SimConfig, SimEvent, SimWorld};

use crate::level;
use crate::mesh;
use crate::sprites::{SpriteDrawCall, SpriteRenderer};
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
pub struct PolygonGpuData {
    pub floor_height: f32,
    pub ceiling_height: f32,
    pub floor_light: f32,
    pub ceiling_light: f32,
    pub floor_transfer_mode: u32,
    pub ceiling_transfer_mode: u32,
    pub media_height: f32,
    pub media_transfer_mode: u32,
    pub floor_tex_offset_x: f32,
    pub floor_tex_offset_y: f32,
    pub ceiling_tex_offset_x: f32,
    pub ceiling_tex_offset_y: f32,
}

/// Marathon eye height: ~0.66 world units above floor (roughly 680/1024).
const EYE_HEIGHT: f32 = 0.66;

/// Snapshot of an entity's renderable state at a given tick.
#[derive(Clone)]
struct RenderableEntity {
    position: Vec3,
    facing: f32,
    shape: u16,
    frame: u16,
}

/// Double-buffered entity snapshots for interpolation.
struct EntitySnapshots {
    prev: Vec<RenderableEntity>,
    curr: Vec<RenderableEntity>,
}

impl EntitySnapshots {
    fn new() -> Self {
        EntitySnapshots {
            prev: Vec::new(),
            curr: Vec::new(),
        }
    }

    fn advance(&mut self, new_entities: Vec<RenderableEntity>) {
        std::mem::swap(&mut self.prev, &mut self.curr);
        self.curr = new_entities;
    }

    fn interpolated(&self, alpha: f32) -> Vec<RenderableEntity> {
        // For entities in both snapshots, lerp position
        // For entities only in curr, use curr position
        // For entities only in prev, skip
        self.curr
            .iter()
            .enumerate()
            .map(|(i, entity)| {
                if let Some(prev) = self.prev.get(i) {
                    RenderableEntity {
                        position: prev.position.lerp(entity.position, alpha),
                        facing: prev.facing + (entity.facing - prev.facing) * alpha,
                        shape: entity.shape,
                        frame: entity.frame,
                    }
                } else {
                    entity.clone()
                }
            })
            .collect()
    }
}

/// Simulation ticks per second.
const TICKS_PER_SECOND: u64 = 30;
const TICK_DURATION_MICROS: u64 = 1_000_000 / TICKS_PER_SECOND;

/// First-person camera state, driven by simulation.
#[derive(Clone, Copy)]
struct CameraState {
    position: Vec3,
    yaw: f32,
    pitch: f32,
}

impl CameraState {
    fn lerp(&self, other: &CameraState, t: f32) -> CameraState {
        CameraState {
            position: self.position.lerp(other.position, t),
            yaw: self.yaw + (other.yaw - self.yaw) * t,
            pitch: self.pitch + (other.pitch - self.pitch) * t,
        }
    }

    fn view_proj(&self, aspect: f32) -> Mat4 {
        let dir = Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize();
        let view = Mat4::look_to_rh(self.position, dir, Vec3::Y);
        let fov = 90.0_f32.to_radians();
        let proj = Mat4::perspective_rh(fov, aspect, 0.1, 1000.0);
        proj * view
    }

    fn to_uniform(&self, aspect: f32, elapsed: f32) -> CameraUniform {
        let vp = self.view_proj(aspect);
        CameraUniform {
            view_proj: vp.to_cols_array(),
            camera_yaw: self.yaw,
            camera_pitch: self.pitch,
            elapsed_time: elapsed,
            _padding: 0.0,
        }
    }
}

/// Input state accumulated between ticks.
struct InputState {
    // Key held states
    forward: bool,
    backward: bool,
    strafe_left: bool,
    strafe_right: bool,
    fire_primary: bool,
    fire_secondary: bool,
    action: bool,
    // Accumulated mouse deltas since last tick
    mouse_dx: f64,
    mouse_dy: f64,
    // Single-press events (consumed per tick)
    escape_pressed: bool,
}

impl InputState {
    fn new() -> Self {
        InputState {
            forward: false,
            backward: false,
            strafe_left: false,
            strafe_right: false,
            fire_primary: false,
            fire_secondary: false,
            action: false,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            escape_pressed: false,
        }
    }

    fn to_action_flags(&mut self) -> ActionFlags {
        let mut bits = 0u32;
        if self.forward {
            bits |= ActionFlags::MOVE_FORWARD;
        }
        if self.backward {
            bits |= ActionFlags::MOVE_BACKWARD;
        }
        if self.strafe_left {
            bits |= ActionFlags::STRAFE_LEFT;
        }
        if self.strafe_right {
            bits |= ActionFlags::STRAFE_RIGHT;
        }
        if self.fire_primary {
            bits |= ActionFlags::FIRE_PRIMARY;
        }
        if self.fire_secondary {
            bits |= ActionFlags::FIRE_SECONDARY;
        }
        if self.action {
            bits |= ActionFlags::ACTION;
        }

        // Convert mouse delta to turn/look flags
        if self.mouse_dx > 2.0 {
            bits |= ActionFlags::TURN_RIGHT;
        } else if self.mouse_dx < -2.0 {
            bits |= ActionFlags::TURN_LEFT;
        }
        if self.mouse_dy > 2.0 {
            bits |= ActionFlags::LOOK_DOWN;
        } else if self.mouse_dy < -2.0 {
            bits |= ActionFlags::LOOK_UP;
        }

        // Reset accumulated deltas
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        self.escape_pressed = false;

        ActionFlags::new(bits)
    }
}

/// Game state enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameState {
    Playing,
    Paused,
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
    fallback_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sprite_renderer: Option<SpriteRenderer>,
    camera_bind_group_layout: wgpu::BindGroupLayout,
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
            let (dt, dv) =
                Self::create_depth_texture(&self.device, new_size.width, new_size.height);
            self.depth_texture = dt;
            self.depth_view = dv;
        }
    }

    fn render(
        &self,
        camera_uniform: &CameraUniform,
        sprite_draw_calls: &[SpriteDrawCall],
        camera_pos: Vec3,
        camera_yaw: f32,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(camera_uniform));

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

            // Pass 1: Level geometry
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.polygon_bind_group, &[]);

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

            // Pass 2: Entity sprites (shares depth buffer)
            if let Some(ref sprite_renderer) = self.sprite_renderer {
                sprite_renderer.render(
                    &self.device,
                    &mut render_pass,
                    &self.camera_bind_group,
                    camera_pos,
                    camera_yaw,
                    sprite_draw_calls,
                );
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub struct App {
    map_path: PathBuf,
    shapes_path: PathBuf,
    sounds_path: Option<PathBuf>,
    starting_level: usize,
    gpu: Option<GpuState>,
    aspect: f32,

    // Scenario data
    map_wad: Option<WadFile>,
    shapes_file: Option<ShapesFile>,
    sounds_file: Option<SoundsFile>,
    physics_data: Option<PhysicsData>,

    // Audio
    audio_engine: Option<AudioEngine>,

    // Simulation
    sim: Option<SimWorld>,
    game_state: GameState,

    // Camera (double-buffered for interpolation)
    prev_camera: CameraState,
    curr_camera: CameraState,

    // Timing
    start_time: Instant,
    last_frame: Instant,
    tick_accumulator_micros: u64,

    // Input
    input: InputState,
    mouse_captured: bool,

    // Entity snapshots for interpolated rendering
    entity_snapshots: EntitySnapshots,

    // Deferred level load
    pending_level_load: Option<usize>,

    // Window
    window: Option<Arc<Window>>,
}

impl App {
    pub fn new(
        map_path: PathBuf,
        shapes_path: PathBuf,
        sounds_path: Option<PathBuf>,
        starting_level: usize,
    ) -> Self {
        let default_camera = CameraState {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
        };
        App {
            map_path,
            shapes_path,
            sounds_path,
            starting_level,
            gpu: None,
            aspect: 16.0 / 9.0,
            map_wad: None,
            shapes_file: None,
            sounds_file: None,
            physics_data: None,
            audio_engine: None,
            sim: None,
            game_state: GameState::Playing,
            prev_camera: default_camera,
            curr_camera: default_camera,
            start_time: Instant::now(),
            last_frame: Instant::now(),
            tick_accumulator_micros: 0,
            input: InputState::new(),
            mouse_captured: false,
            entity_snapshots: EntitySnapshots::new(),
            pending_level_load: None,
            window: None,
        }
    }

    fn build_sprite_draw_calls(&self, alpha: f32) -> Vec<SpriteDrawCall> {
        let entities = self.entity_snapshots.interpolated(alpha);
        let shapes = match &self.shapes_file {
            Some(s) => s,
            None => return Vec::new(),
        };

        let camera_pos = self.curr_camera.position;

        entities
            .iter()
            .filter_map(|entity| {
                // Use shape field as collection index, frame as the frame index
                let collection_idx = entity.shape;
                let view_angle = crate::sprites::compute_view_angle(
                    camera_pos,
                    entity.position,
                    entity.facing,
                );

                let (bitmap_index, width, height) = crate::sprites::resolve_entity_sprite(
                    shapes,
                    collection_idx,
                    0, // sequence 0 (default)
                    entity.frame,
                    view_angle,
                )?;

                Some(SpriteDrawCall {
                    position: entity.position,
                    width,
                    height,
                    collection: collection_idx,
                    bitmap_index,
                    tint: 1.0,
                })
            })
            .collect()
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

        // Build per-polygon GPU data
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

        // Initialize simulation
        let physics = match &self.physics_data {
            Some(p) => p.clone(),
            None => {
                log::warn!("No physics data found, simulation may not work correctly");
                return;
            }
        };
        let config = SimConfig {
            random_seed: 42,
            difficulty: 2, // Normal
        };
        match SimWorld::new(map, &physics, &config) {
            Ok(sim) => {
                self.sim = Some(sim);
                log::info!("Simulation initialized");
            }
            Err(e) => {
                log::error!("Failed to initialize simulation: {e}");
            }
        }

        // Set initial camera from player spawn
        if let Some(ref mut sim) = self.sim {
            if let Some(pos) = sim.player_position() {
                let cam = CameraState {
                    position: Vec3::new(pos.x, pos.y + EYE_HEIGHT, pos.z),
                    yaw: sim.player_facing().unwrap_or(0.0),
                    pitch: 0.0,
                };
                self.prev_camera = cam;
                self.curr_camera = cam;
            }
        } else if let Some(poly) = map.polygons.first() {
            let cx = poly.center.x as f32 / 1024.0;
            let cy = (poly.floor_height as f32 / 1024.0) + EYE_HEIGHT;
            let cz = poly.center.y as f32 / 1024.0;
            let cam = CameraState {
                position: Vec3::new(cx, cy, cz),
                yaw: 0.0,
                pitch: 0.0,
            };
            self.prev_camera = cam;
            self.curr_camera = cam;
        }

        // Upload to GPU
        if let Some(gpu) = &mut self.gpu {
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
        }

        // Load audio for level
        if let (Some(ref mut audio), Some(ref sounds)) =
            (&mut self.audio_engine, &self.sounds_file)
        {
            audio.load_level(map, sounds);
            log::info!("Audio loaded for level");
        }

        // Load sprite collections for entities
        if let (Some(gpu), Some(ref shapes)) = (&mut self.gpu, &self.shapes_file) {
            // Collect all collections that entities might reference
            // Marathon collections: 0-31, objects typically use collections 10-31
            let entity_collections: Vec<u16> = (0..32).collect();
            if let Some(ref mut sprite_renderer) = gpu.sprite_renderer {
                sprite_renderer.load_collections(
                    &gpu.device,
                    &gpu.queue,
                    shapes,
                    &entity_collections,
                );
            }
        }

        self.game_state = GameState::Playing;
        self.tick_accumulator_micros = 0;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Marathon")
            .with_inner_size(PhysicalSize::new(1280u32, 720u32));

        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());

        // Load WAD files
        let map_wad = WadFile::open(&self.map_path).unwrap_or_else(|e| {
            eprintln!("Failed to open map WAD: {e}");
            std::process::exit(1);
        });
        let shapes_file = ShapesFile::open(&self.shapes_path).unwrap_or_else(|e| {
            eprintln!("Failed to open shapes file: {e}");
            std::process::exit(1);
        });

        // Try to load physics from map WAD entries
        let physics_data = map_wad
            .entries()
            .iter()
            .find_map(|entry| PhysicsData::from_entry(entry).ok());

        // Load sounds file if provided
        let sounds_file = self.sounds_path.as_ref().and_then(|path| {
            match SoundsFile::open(path) {
                Ok(sf) => {
                    log::info!("Loaded sounds file");
                    Some(sf)
                }
                Err(e) => {
                    log::warn!("Failed to load sounds file: {e}");
                    None
                }
            }
        });

        // Initialize audio engine (non-fatal if it fails)
        let audio_engine = match AudioEngine::new(AudioConfig {
            max_channels: 32,
            music_volume: 1.0,
            sfx_volume: 1.0,
        }) {
            Ok(engine) => {
                log::info!("Audio engine initialized");
                Some(engine)
            }
            Err(e) => {
                log::warn!("Audio unavailable: {e}");
                None
            }
        };

        let levels = level::enumerate_levels(&map_wad);
        log::info!("Found {} levels", levels.len());

        self.map_wad = Some(map_wad);
        self.shapes_file = Some(shapes_file);
        self.sounds_file = sounds_file;
        self.physics_data = physics_data;
        self.audio_engine = audio_engine;

        // Initialize wgpu
        let size = window.inner_size();
        self.aspect = size.width as f32 / size.height.max(1) as f32;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
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

        let camera_uniform = self.curr_camera.to_uniform(self.aspect, 0.0);
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

        // Polygon storage buffer
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

        // Fallback texture
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
            wgpu::TexelCopyTextureInfo {
                texture: &fallback_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8, 0, 255, 255],
            wgpu::TexelCopyBufferLayout {
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

        // Pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&camera_bgl, &polygon_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[mesh::Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
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

        let sprite_renderer = SpriteRenderer::new(
            &device,
            &queue,
            &camera_bgl,
            surface_format,
        );

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
            sprite_renderer: Some(sprite_renderer),
            camera_bind_group_layout: camera_bgl,
        });

        self.window = Some(window);
        self.start_time = Instant::now();
        self.last_frame = Instant::now();

        // Load the starting level
        self.load_level(self.starting_level);
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
                self.aspect = size.width as f32 / size.height.max(1) as f32;
                if let Some(gpu) = &mut self.gpu {
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
                match key {
                    KeyCode::KeyW | KeyCode::ArrowUp => self.input.forward = pressed,
                    KeyCode::KeyS | KeyCode::ArrowDown => self.input.backward = pressed,
                    KeyCode::KeyA => self.input.strafe_left = pressed,
                    KeyCode::KeyD => self.input.strafe_right = pressed,
                    KeyCode::Space => self.input.action = pressed,
                    KeyCode::Escape if pressed => {
                        match self.game_state {
                            GameState::Playing => {
                                if self.mouse_captured {
                                    self.game_state = GameState::Paused;
                                    self.mouse_captured = false;
                                    if let Some(w) = &self.window {
                                        let _ = w
                                            .set_cursor_grab(winit::window::CursorGrabMode::None);
                                        w.set_cursor_visible(true);
                                    }
                                    log::info!("Game paused");
                                }
                            }
                            GameState::Paused => {
                                self.game_state = GameState::Playing;
                                self.mouse_captured = true;
                                if let Some(w) = &self.window {
                                    let _ = w
                                        .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                                        .or_else(|_| {
                                            w.set_cursor_grab(
                                                winit::window::CursorGrabMode::Confined,
                                            )
                                        });
                                    w.set_cursor_visible(false);
                                }
                                // Reset accumulator to avoid catch-up ticks
                                self.tick_accumulator_micros = 0;
                                self.last_frame = Instant::now();
                                log::info!("Game resumed");
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if !self.mouse_captured && self.game_state == GameState::Playing {
                    self.mouse_captured = true;
                    if let Some(w) = &self.window {
                        let _ = w
                            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                            .or_else(|_| {
                                w.set_cursor_grab(winit::window::CursorGrabMode::Confined)
                            });
                        w.set_cursor_visible(false);
                    }
                } else {
                    match button {
                        MouseButton::Left => self.input.fire_primary = true,
                        MouseButton::Right => self.input.fire_secondary = true,
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt_micros = (now - self.last_frame).as_micros() as u64;
                self.last_frame = now;
                let elapsed = (now - self.start_time).as_secs_f32();

                // Run simulation ticks if playing
                if self.game_state == GameState::Playing {
                    self.tick_accumulator_micros += dt_micros;

                    // Cap at 5 ticks per frame to avoid spiral of death
                    let max_ticks = 5u64;
                    let mut ticks_run = 0u64;

                    while self.tick_accumulator_micros >= TICK_DURATION_MICROS
                        && ticks_run < max_ticks
                    {
                        self.tick_accumulator_micros -= TICK_DURATION_MICROS;
                        ticks_run += 1;

                        // Save previous camera for interpolation
                        self.prev_camera = self.curr_camera;

                        // Produce action flags from input
                        let flags = self.input.to_action_flags();

                        // Advance simulation
                        if let Some(ref mut sim) = self.sim {
                            sim.tick(flags);

                            // Update camera from player state
                            if let Some(pos) = sim.player_position() {
                                self.curr_camera.position =
                                    Vec3::new(pos.x, pos.y + EYE_HEIGHT, pos.z);
                            }
                            if let Some(facing) = sim.player_facing() {
                                self.curr_camera.yaw = facing;
                            }

                            // Collect entity states for rendering
                            let entities: Vec<RenderableEntity> = sim
                                .entities()
                                .into_iter()
                                .map(|e| RenderableEntity {
                                    position: e.position,
                                    facing: e.facing,
                                    shape: e.shape,
                                    frame: e.frame,
                                })
                                .collect();
                            self.entity_snapshots.advance(entities);

                            // Process simulation events
                            let events = sim.drain_events();
                            let mut audio_events = Vec::new();
                            for event in events {
                                match event {
                                    SimEvent::LevelTeleport { target_level } => {
                                        log::info!("Level teleport to level {target_level}");
                                        self.pending_level_load = Some(target_level);
                                    }
                                    SimEvent::SoundTrigger {
                                        sound_index,
                                        position,
                                    } => {
                                        audio_events.push(AudioEvent::PlaySound(
                                            marathon_audio::PlaySoundRequest {
                                                sound_index,
                                                source_entity: None,
                                                source_polygon: -1,
                                                x: position.x,
                                                y: position.y,
                                                z: position.z,
                                            },
                                        ));
                                    }
                                    _ => {}
                                }
                            }

                            // Update audio engine
                            if let Some(ref mut audio) = self.audio_engine {
                                let player_pos = sim.player_position().unwrap_or(Vec3::ZERO);
                                let player_facing = sim.player_facing().unwrap_or(0.0);
                                let listener = ListenerState {
                                    x: player_pos.x,
                                    y: player_pos.y,
                                    z: player_pos.z,
                                    facing_angle: player_facing,
                                    polygon_index: sim
                                        .player_polygon()
                                        .map(|p| p as i16)
                                        .unwrap_or(-1),
                                };
                                audio.update(1.0 / TICKS_PER_SECOND as f32, listener, &audio_events);
                            }

                            // Update GPU polygon buffer with platform/media/light state
                            if let Some(gpu) = &self.gpu {
                                let snapshot = sim.snapshot();

                                for platform in &snapshot.platforms {
                                    let offset = platform.polygon_index
                                        * std::mem::size_of::<PolygonGpuData>();
                                    // Write floor_height (first field)
                                    gpu.queue.write_buffer(
                                        &gpu.polygon_buffer,
                                        offset as u64,
                                        bytemuck::bytes_of(&platform.current_floor),
                                    );
                                    // Write ceiling_height (second field, 4 bytes in)
                                    gpu.queue.write_buffer(
                                        &gpu.polygon_buffer,
                                        (offset + 4) as u64,
                                        bytemuck::bytes_of(&platform.current_ceiling),
                                    );
                                }

                                for media in &snapshot.media {
                                    let offset = media.polygon_index
                                        * std::mem::size_of::<PolygonGpuData>();
                                    // Write media_height (7th field, offset 24 bytes)
                                    gpu.queue.write_buffer(
                                        &gpu.polygon_buffer,
                                        (offset + 24) as u64,
                                        bytemuck::bytes_of(&media.current_height),
                                    );
                                }

                                for light in &snapshot.lights {
                                    // Update all polygons that reference this light
                                    // (light_index maps to floor/ceiling lightsource indices)
                                    // This is a simplified approach — write intensity
                                    // for floor_light and ceiling_light fields
                                    let _ = light; // Full light update requires polygon→light mapping
                                }
                            }
                        }
                    }

                    // If we still have excess time, cap it
                    if self.tick_accumulator_micros > TICK_DURATION_MICROS * max_ticks {
                        self.tick_accumulator_micros = 0;
                    }

                    // Handle deferred level load (outside sim borrow)
                    if let Some(target) = self.pending_level_load.take() {
                        self.load_level(target);
                    }
                }

                // Interpolate camera
                let alpha = if self.game_state == GameState::Playing {
                    (self.tick_accumulator_micros as f32) / (TICK_DURATION_MICROS as f32)
                } else {
                    1.0
                };
                let render_camera = self.prev_camera.lerp(&self.curr_camera, alpha);
                let camera_uniform = render_camera.to_uniform(self.aspect, elapsed);

                // Build sprite draw calls from entity snapshots
                let sprite_draw_calls = self.build_sprite_draw_calls(alpha);

                // Render
                if let Some(gpu) = &self.gpu {
                    match gpu.render(
                        &camera_uniform,
                        &sprite_draw_calls,
                        render_camera.position,
                        render_camera.yaw,
                    ) {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            let size =
                                PhysicalSize::new(gpu.config.width, gpu.config.height);
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
                self.input.mouse_dx += delta.0;
                self.input.mouse_dy += delta.1;
            }
        }
    }
}

pub fn run(
    map_path: PathBuf,
    shapes_path: PathBuf,
    sounds_path: Option<PathBuf>,
    starting_level: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(map_path, shapes_path, sounds_path, starting_level);
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_state_lerp_midpoint() {
        let a = CameraState {
            position: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
        };
        let b = CameraState {
            position: Vec3::new(10.0, 20.0, 30.0),
            yaw: 2.0,
            pitch: 1.0,
        };
        let mid = a.lerp(&b, 0.5);
        assert!((mid.position.x - 5.0).abs() < 0.001);
        assert!((mid.position.y - 10.0).abs() < 0.001);
        assert!((mid.position.z - 15.0).abs() < 0.001);
        assert!((mid.yaw - 1.0).abs() < 0.001);
        assert!((mid.pitch - 0.5).abs() < 0.001);
    }

    #[test]
    fn camera_state_lerp_endpoints() {
        let a = CameraState {
            position: Vec3::new(1.0, 2.0, 3.0),
            yaw: 1.0,
            pitch: 0.5,
        };
        let b = CameraState {
            position: Vec3::new(4.0, 5.0, 6.0),
            yaw: 2.0,
            pitch: 1.5,
        };

        let at_zero = a.lerp(&b, 0.0);
        assert!((at_zero.position.x - 1.0).abs() < 0.001);
        assert!((at_zero.yaw - 1.0).abs() < 0.001);

        let at_one = a.lerp(&b, 1.0);
        assert!((at_one.position.x - 4.0).abs() < 0.001);
        assert!((at_one.yaw - 2.0).abs() < 0.001);
    }

    #[test]
    fn camera_view_proj_is_finite() {
        let cam = CameraState {
            position: Vec3::new(5.0, 1.0, 5.0),
            yaw: 0.5,
            pitch: 0.1,
        };
        let vp = cam.view_proj(16.0 / 9.0);
        for i in 0..16 {
            assert!(vp.to_cols_array()[i].is_finite(), "view_proj element {i} is not finite");
        }
    }

    #[test]
    fn camera_to_uniform_stores_values() {
        let cam = CameraState {
            position: Vec3::ZERO,
            yaw: 1.5,
            pitch: 0.3,
        };
        let uniform = cam.to_uniform(16.0 / 9.0, 42.0);
        assert!((uniform.camera_yaw - 1.5).abs() < 0.001);
        assert!((uniform.camera_pitch - 0.3).abs() < 0.001);
        assert!((uniform.elapsed_time - 42.0).abs() < 0.001);
    }

    #[test]
    fn input_state_default_produces_empty_flags() {
        let mut input = InputState::new();
        let flags = input.to_action_flags();
        assert!(flags.is_empty());
    }

    #[test]
    fn input_state_forward_sets_flag() {
        let mut input = InputState::new();
        input.forward = true;
        let flags = input.to_action_flags();
        assert!(flags.contains(ActionFlags::MOVE_FORWARD));
    }

    #[test]
    fn input_state_multiple_keys() {
        let mut input = InputState::new();
        input.forward = true;
        input.strafe_left = true;
        input.fire_primary = true;
        let flags = input.to_action_flags();
        assert!(flags.contains(ActionFlags::MOVE_FORWARD));
        assert!(flags.contains(ActionFlags::STRAFE_LEFT));
        assert!(flags.contains(ActionFlags::FIRE_PRIMARY));
        assert!(!flags.contains(ActionFlags::MOVE_BACKWARD));
    }

    #[test]
    fn input_state_mouse_deltas_produce_turn_flags() {
        let mut input = InputState::new();
        input.mouse_dx = 10.0;
        let flags = input.to_action_flags();
        assert!(flags.contains(ActionFlags::TURN_RIGHT));

        let mut input = InputState::new();
        input.mouse_dx = -10.0;
        let flags = input.to_action_flags();
        assert!(flags.contains(ActionFlags::TURN_LEFT));
    }

    #[test]
    fn input_state_mouse_resets_after_to_action_flags() {
        let mut input = InputState::new();
        input.mouse_dx = 50.0;
        input.mouse_dy = -30.0;
        let _ = input.to_action_flags();
        assert!((input.mouse_dx).abs() < 0.001);
        assert!((input.mouse_dy).abs() < 0.001);
    }

    #[test]
    fn entity_snapshots_new_is_empty() {
        let snaps = EntitySnapshots::new();
        assert!(snaps.prev.is_empty());
        assert!(snaps.curr.is_empty());
    }

    #[test]
    fn entity_snapshots_advance_swaps_buffers() {
        let mut snaps = EntitySnapshots::new();

        let tick1 = vec![RenderableEntity {
            position: Vec3::new(1.0, 0.0, 0.0),
            facing: 0.0,
            shape: 0,
            frame: 0,
        }];
        snaps.advance(tick1);
        assert!(snaps.prev.is_empty());
        assert_eq!(snaps.curr.len(), 1);

        let tick2 = vec![RenderableEntity {
            position: Vec3::new(2.0, 0.0, 0.0),
            facing: 0.0,
            shape: 0,
            frame: 0,
        }];
        snaps.advance(tick2);
        assert_eq!(snaps.prev.len(), 1);
        assert_eq!(snaps.curr.len(), 1);
        assert!((snaps.prev[0].position.x - 1.0).abs() < 0.001);
        assert!((snaps.curr[0].position.x - 2.0).abs() < 0.001);
    }

    #[test]
    fn entity_snapshots_interpolation() {
        let mut snaps = EntitySnapshots::new();

        snaps.advance(vec![RenderableEntity {
            position: Vec3::new(0.0, 0.0, 0.0),
            facing: 0.0,
            shape: 5,
            frame: 0,
        }]);
        snaps.advance(vec![RenderableEntity {
            position: Vec3::new(10.0, 0.0, 0.0),
            facing: 1.0,
            shape: 5,
            frame: 1,
        }]);

        let interp = snaps.interpolated(0.5);
        assert_eq!(interp.len(), 1);
        assert!((interp[0].position.x - 5.0).abs() < 0.001);
        assert!((interp[0].facing - 0.5).abs() < 0.001);
        // Shape uses current tick value
        assert_eq!(interp[0].frame, 1);
    }

    #[test]
    fn entity_snapshots_new_entity_no_interpolation() {
        let mut snaps = EntitySnapshots::new();

        // Empty first tick
        snaps.advance(vec![]);
        // One entity appears
        snaps.advance(vec![RenderableEntity {
            position: Vec3::new(5.0, 0.0, 0.0),
            facing: 0.0,
            shape: 0,
            frame: 0,
        }]);

        let interp = snaps.interpolated(0.5);
        assert_eq!(interp.len(), 1);
        // No previous state, so should use current position as-is
        assert!((interp[0].position.x - 5.0).abs() < 0.001);
    }

    #[test]
    fn polygon_gpu_data_layout() {
        // Verify the struct is exactly 48 bytes (12 x f32/u32)
        assert_eq!(std::mem::size_of::<PolygonGpuData>(), 48);
    }

    #[test]
    fn camera_uniform_layout() {
        // 16 floats (mat4) + 4 floats = 80 bytes
        assert_eq!(std::mem::size_of::<CameraUniform>(), 80);
    }

    #[test]
    fn tick_timing_constants() {
        assert_eq!(TICKS_PER_SECOND, 30);
        assert_eq!(TICK_DURATION_MICROS, 33333);
    }
}
