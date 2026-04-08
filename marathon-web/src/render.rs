use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wgpu::util::DeviceExt;

use marathon_formats::{MapData, PhysicsData, ShapesFile, WadFile};
use marathon_sim::tick::ActionFlags;
use marathon_sim::world::{SimConfig, SimWorld};

use crate::level;
use crate::mesh;
use crate::sprites::{SpriteDrawCall, SpriteRenderer};
use crate::texture::TextureManager;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [f32; 16],
    camera_yaw: f32,
    camera_pitch: f32,
    elapsed_time: f32,
    _padding: f32,
}


const EYE_HEIGHT: f32 = 0.66;
const TICKS_PER_SECOND: f64 = 30.0;
const TICK_DURATION_MS: f64 = 1000.0 / TICKS_PER_SECOND;

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

    fn to_uniform(&self, aspect: f32, elapsed: f32) -> CameraUniform {
        let dir = Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ).normalize();
        let view = Mat4::look_to_rh(self.position, dir, Vec3::Y);
        let proj = Mat4::perspective_rh(90.0_f32.to_radians(), aspect, 0.1, 1000.0);
        let vp = proj * view;
        CameraUniform {
            view_proj: vp.to_cols_array(),
            camera_yaw: self.yaw,
            camera_pitch: self.pitch,
            elapsed_time: elapsed,
            _padding: 0.0,
        }
    }
}

struct InputState {
    forward: bool, backward: bool, strafe_left: bool, strafe_right: bool,
    fire_primary: bool, fire_secondary: bool, action: bool,
    mouse_dx: f64, mouse_dy: f64,
}

impl InputState {
    fn new() -> Self {
        InputState {
            forward: false, backward: false, strafe_left: false, strafe_right: false,
            fire_primary: false, fire_secondary: false, action: false,
            mouse_dx: 0.0, mouse_dy: 0.0,
        }
    }

    fn to_action_flags(&mut self) -> ActionFlags {
        let mut bits = 0u32;
        if self.forward { bits |= ActionFlags::MOVE_FORWARD; }
        if self.backward { bits |= ActionFlags::MOVE_BACKWARD; }
        if self.strafe_left { bits |= ActionFlags::STRAFE_LEFT; }
        if self.strafe_right { bits |= ActionFlags::STRAFE_RIGHT; }
        if self.fire_primary { bits |= ActionFlags::FIRE_PRIMARY; }
        if self.fire_secondary { bits |= ActionFlags::FIRE_SECONDARY; }
        if self.action { bits |= ActionFlags::ACTION; }
        if self.mouse_dx > 2.0 { bits |= ActionFlags::TURN_RIGHT; }
        else if self.mouse_dx < -2.0 { bits |= ActionFlags::TURN_LEFT; }
        if self.mouse_dy > 2.0 { bits |= ActionFlags::LOOK_DOWN; }
        else if self.mouse_dy < -2.0 { bits |= ActionFlags::LOOK_UP; }
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        ActionFlags::new(bits)
    }
}

#[derive(Clone)]
struct RenderableEntity {
    position: Vec3, facing: f32, shape: u16, frame: u16,
}

struct EntitySnapshots { prev: Vec<RenderableEntity>, curr: Vec<RenderableEntity> }
impl EntitySnapshots {
    fn new() -> Self { Self { prev: Vec::new(), curr: Vec::new() } }
    fn advance(&mut self, new: Vec<RenderableEntity>) {
        std::mem::swap(&mut self.prev, &mut self.curr);
        self.curr = new;
    }
    fn interpolated(&self, alpha: f32) -> Vec<RenderableEntity> {
        self.curr.iter().enumerate().map(|(i, e)| {
            if let Some(p) = self.prev.get(i) {
                RenderableEntity { position: p.position.lerp(e.position, alpha), facing: p.facing + (e.facing - p.facing) * alpha, shape: e.shape, frame: e.frame }
            } else { e.clone() }
        }).collect()
    }
}

/// All game state for the web version.
struct GameState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    texture_manager: TextureManager,
    fallback_bind_group: wgpu::BindGroup,
    texture_bgl: wgpu::BindGroupLayout,
    sprite_renderer: SpriteRenderer,
    sim: Option<SimWorld>,
    shapes_file: ShapesFile,
    prev_camera: CameraState,
    curr_camera: CameraState,
    input: InputState,
    entity_snapshots: EntitySnapshots,
    last_frame_ms: f64,
    tick_accum_ms: f64,
    start_ms: f64,
    aspect: f32,
}

impl GameState {
    fn frame(&mut self) {
        let now = js_sys::Date::now();
        let dt_ms = (now - self.last_frame_ms).min(100.0); // Cap at 100ms
        self.last_frame_ms = now;
        let elapsed = ((now - self.start_ms) / 1000.0) as f32;

        // Simulation ticks
        self.tick_accum_ms += dt_ms;
        while self.tick_accum_ms >= TICK_DURATION_MS {
            self.tick_accum_ms -= TICK_DURATION_MS;
            self.prev_camera = self.curr_camera;
            let flags = self.input.to_action_flags();
            if let Some(ref mut sim) = self.sim {
                sim.tick(flags);
                if let Some(pos) = sim.player_position() {
                    self.curr_camera.position = Vec3::new(pos.x, pos.y + EYE_HEIGHT, pos.z);
                }
                if let Some(f) = sim.player_facing() { self.curr_camera.yaw = f; }
                let entities: Vec<RenderableEntity> = sim.entities().into_iter()
                    .map(|e| RenderableEntity { position: e.position, facing: e.facing, shape: e.shape, frame: e.frame })
                    .collect();
                self.entity_snapshots.advance(entities);
                let _ = sim.drain_events();
            }
        }

        let alpha = (self.tick_accum_ms / TICK_DURATION_MS) as f32;
        let cam = self.prev_camera.lerp(&self.curr_camera, alpha);
        let uniform = cam.to_uniform(self.aspect, elapsed);

        // Build sprite draw calls
        let sprites: Vec<SpriteDrawCall> = self.entity_snapshots.interpolated(alpha).iter().filter_map(|e| {
            let view = crate::sprites::compute_view_angle(cam.position, e.position, e.facing);
            let (bi, w, h) = crate::sprites::resolve_entity_sprite(&self.shapes_file, e.shape, 0, e.frame, view)?;
            Some(SpriteDrawCall { position: e.position, width: w, height: h, collection: e.shape, bitmap_index: bi, tint: 1.0 })
        }).collect();

        // Render
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = output.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.15, a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None, occlusion_query_set: None,
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &self.camera_bind_group, &[]);
            let tbg = self.texture_manager.gpu_textures.values().next().map(|t| &t.bind_group).unwrap_or(&self.fallback_bind_group);
            rp.set_bind_group(1, tbg, &[]);
            rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rp.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..self.num_indices, 0, 0..1);
            self.sprite_renderer.render(&self.device, &mut rp, &self.camera_bind_group, cam.position, cam.yaw, &sprites);
        }
        self.queue.submit(std::iter::once(enc.finish()));
        output.present();
    }
}

pub async fn run_web(
    map_bytes: &[u8],
    shapes_bytes: &[u8],
    physics_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse scenario data
    let map_wad = WadFile::from_bytes(map_bytes)?;
    let shapes_file = ShapesFile::from_bytes(shapes_bytes)?;
    let physics_data = WadFile::from_bytes(physics_bytes)
        .ok()
        .and_then(|wad| wad.entry(0).and_then(|e| PhysicsData::from_entry(e).ok()));

    log::info!("Parsed: {} levels", map_wad.entry_count());

    // Get canvas
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("marathon-canvas").unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

    let width = canvas.client_width() as u32;
    let height = canvas.client_height() as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    // Init wgpu — use GL backend (WebGL2) for broadest browser support
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        ..Default::default()
    });

    let surface_target = wgpu::SurfaceTarget::Canvas(canvas.clone());
    let surface = instance.create_surface(surface_target).unwrap();

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }).await.ok_or("No GPU adapter found")?;

    log::info!("GPU: {}", adapter.get_info().name);

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("dev"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
        memory_hints: Default::default(),
    }, None).await.map_err(|e| format!("Device error: {e}"))?;

    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats.first().copied().unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format, width, height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let (_, depth_view) = create_depth(&device, width, height);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shader"), source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    // Bind group layouts
    let cam_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None, entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0, visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None,
        }],
    });
    let tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None, entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2Array, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });

    let cam_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None, contents: &[0u8; 80], usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let cam_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None, layout: &cam_bgl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: cam_buf.as_entire_binding() }],
    });

    // Fallback texture
    let fb = create_fallback_texture(&device, &queue, &tex_bgl);

    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None, bind_group_layouts: &[&cam_bgl, &tex_bgl], push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&pl),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[mesh::Vertex::layout()], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default() }),
        primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, front_face: wgpu::FrontFace::Ccw, cull_mode: Some(wgpu::Face::Back), ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Less, stencil: Default::default(), bias: Default::default() }),
        multisample: Default::default(), multiview: None, cache: None,
    });

    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: &[0u8; 4], usage: wgpu::BufferUsages::VERTEX });
    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: &[0u8; 4], usage: wgpu::BufferUsages::INDEX });

    let sprite_renderer = SpriteRenderer::new(&device, &queue, &cam_bgl, format);

    let cam = CameraState { position: Vec3::ZERO, yaw: 0.0, pitch: 0.0 };
    let now = js_sys::Date::now();

    let mut state = GameState {
        surface, device, queue, config, depth_view, pipeline,
        camera_buffer: cam_buf, camera_bind_group: cam_bg,
        vertex_buffer: vb, index_buffer: ib, num_indices: 0,
        texture_manager: TextureManager { collections: Default::default(), gpu_textures: Default::default() },
        fallback_bind_group: fb, texture_bgl: tex_bgl,
        sprite_renderer, sim: None, shapes_file,
        prev_camera: cam, curr_camera: cam,
        input: InputState::new(), entity_snapshots: EntitySnapshots::new(),
        last_frame_ms: now, tick_accum_ms: 0.0, start_ms: now,
        aspect: width as f32 / height.max(1) as f32,
    };

    // Load level 0
    load_level_into(&mut state, &map_wad, physics_data.as_ref(), 0);

    // Set up input handlers on canvas
    let state = Rc::new(RefCell::new(state));
    setup_input_handlers(&canvas, state.clone());

    // Start render loop via requestAnimationFrame
    start_render_loop(state);

    Ok(())
}

fn create_depth(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let v = t.create_view(&Default::default());
    (t, v)
}

fn create_fallback_texture(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
    let layer_count = crate::texture::pad_layer_count_for_webgl(1);
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: None, size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: layer_count },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &[255, 0, 255, 255],
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
        wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
    );
    let view = tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
    let sampler = device.create_sampler(&Default::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None, layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
    })
}

fn load_level_into(state: &mut GameState, wad: &WadFile, physics: Option<&PhysicsData>, index: usize) {
    let loaded = match level::load_level(wad, index) {
        Ok(l) => l,
        Err(e) => { log::error!("Load level {index}: {e}"); return; }
    };
    log::info!("Level: {}", loaded.level_name);
    let map = &loaded.map;

    let poly_info: Vec<mesh::PolygonInfo> = map.polygons.iter().map(|p| {
        mesh::PolygonInfo {
            floor_light: level::evaluate_light_intensity(&map.lights, p.floor_lightsource_index),
            floor_transfer_mode: p.floor_transfer_mode as u32,
        }
    }).collect();
    let lm = mesh::build_level_mesh(map, &poly_info);
    let descs = level::collect_texture_descriptors(map);
    let mut tm = TextureManager::load_collections(&state.shapes_file, &descs);

    // Init sim
    if let Some(phys) = physics {
        match SimWorld::new(map, phys, &SimConfig { random_seed: 42, difficulty: 2 }) {
            Ok(sim) => { state.sim = Some(sim); }
            Err(e) => { log::error!("Sim: {e}"); }
        }
    }

    // Camera
    if let Some(ref mut sim) = state.sim {
        if let Some(pos) = sim.player_position() {
            let c = CameraState { position: Vec3::new(pos.x, pos.y + EYE_HEIGHT, pos.z), yaw: sim.player_facing().unwrap_or(0.0), pitch: 0.0 };
            state.prev_camera = c; state.curr_camera = c;
        }
    }

    // GPU upload
    state.vertex_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None, contents: bytemuck::cast_slice(&lm.vertices), usage: wgpu::BufferUsages::VERTEX,
    });
    state.index_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None, contents: bytemuck::cast_slice(&lm.indices), usage: wgpu::BufferUsages::INDEX,
    });
    state.num_indices = lm.indices.len() as u32;

    let sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::Repeat, address_mode_v: wgpu::AddressMode::Repeat, ..Default::default()
    });
    tm.create_gpu_textures(&state.device, &state.queue, &state.texture_bgl, &sampler);
    state.texture_manager = tm;

    let colls: Vec<u16> = (0..32).collect();
    state.sprite_renderer.load_collections(&state.device, &state.queue, &state.shapes_file, &colls);
}

fn setup_input_handlers(canvas: &web_sys::HtmlCanvasElement, state: Rc<RefCell<GameState>>) {
    // Keyboard
    let s = state.clone();
    let keydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        let mut st = s.borrow_mut();
        match e.code().as_str() {
            "KeyW" | "ArrowUp" => st.input.forward = true,
            "KeyS" | "ArrowDown" => st.input.backward = true,
            "KeyA" => st.input.strafe_left = true,
            "KeyD" => st.input.strafe_right = true,
            "Space" => st.input.action = true,
            _ => {}
        }
        e.prevent_default();
    });
    canvas.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref()).unwrap();
    keydown.forget();

    let s = state.clone();
    let keyup = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        let mut st = s.borrow_mut();
        match e.code().as_str() {
            "KeyW" | "ArrowUp" => st.input.forward = false,
            "KeyS" | "ArrowDown" => st.input.backward = false,
            "KeyA" => st.input.strafe_left = false,
            "KeyD" => st.input.strafe_right = false,
            "Space" => st.input.action = false,
            _ => {}
        }
    });
    canvas.add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref()).unwrap();
    keyup.forget();

    // Mouse movement (pointer lock)
    let s = state.clone();
    let mousemove = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        st.input.mouse_dx += e.movement_x() as f64;
        st.input.mouse_dy += e.movement_y() as f64;
    });
    canvas.add_event_listener_with_callback("mousemove", mousemove.as_ref().unchecked_ref()).unwrap();
    mousemove.forget();

    // Click to capture pointer
    let canvas_clone = canvas.clone();
    let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
        canvas_clone.request_pointer_lock();
    });
    canvas.add_event_listener_with_callback("click", click.as_ref().unchecked_ref()).unwrap();
    click.forget();

    // Mouse buttons
    let s = state.clone();
    let mousedown = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        match e.button() {
            0 => st.input.fire_primary = true,
            2 => st.input.fire_secondary = true,
            _ => {}
        }
    });
    canvas.add_event_listener_with_callback("mousedown", mousedown.as_ref().unchecked_ref()).unwrap();
    mousedown.forget();

    let s = state.clone();
    let mouseup = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
        let mut st = s.borrow_mut();
        match e.button() {
            0 => st.input.fire_primary = false,
            2 => st.input.fire_secondary = false,
            _ => {}
        }
    });
    canvas.add_event_listener_with_callback("mouseup", mouseup.as_ref().unchecked_ref()).unwrap();
    mouseup.forget();
}

fn start_render_loop(state: Rc<RefCell<GameState>>) {
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    let s = state.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        s.borrow_mut().frame();
        let window = web_sys::window().unwrap();
        window.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap();
    }));

    let window = web_sys::window().unwrap();
    window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap();
}
