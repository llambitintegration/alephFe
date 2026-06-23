use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wgpu::util::DeviceExt;

use marathon_formats::{PhysicsData, ShapesFile, WadFile};
use marathon_sim::tick::{ActionFlags, TickInput};
use marathon_sim::world::{SimConfig, SimWorld};
use marathon_sim::WorldSnapshot;

use crate::level;
use crate::mesh;
use crate::sprites::{SpriteDrawCall, SpriteRenderer};
use crate::texture::TextureManager;

thread_local! {
    /// Handle to the live game state, stashed so the debug WASM hooks
    /// (`__marathonDebug.*`) can reach the running sim from a plain exported
    /// function. Set once in `run_web`; never touched by normal gameplay.
    static GAME_STATE: RefCell<Option<Rc<RefCell<GameState>>>> = const { RefCell::new(None) };
}

/// DEBUG-ONLY WASM hook: reposition + re-face the player directly in front of
/// the nearest activatable door so that a subsequent action-key (Space) press
/// activates it. Exposed to JS as `window.__marathonDebug.faceNearestDoor()`
/// (see [`install_debug_hooks`]). Returns `true` when a door was found and the
/// player was moved; used to make door-interaction e2e tests deterministic.
///
/// This is never invoked by gameplay — only by the test harness / debug console.
#[wasm_bindgen]
pub fn debug_face_nearest_door() -> bool {
    GAME_STATE.with(|cell| {
        let handle = cell.borrow().clone();
        match handle {
            Some(state_rc) => {
                let mut state = state_rc.borrow_mut();
                match state.sim.as_mut() {
                    Some(sim) => sim.debug_face_nearest_door().is_some(),
                    None => false,
                }
            }
            None => false,
        }
    })
}

/// DEBUG-ONLY WASM hook: reposition + re-face the player in front of the
/// nearest light-switch control panel so a subsequent action-key (Space) press
/// toggles its light. Exposed to JS as
/// `window.__marathonDebug.faceNearestLightSwitch()`. Returns the `light_index`
/// the switch controls (>= 0) so the test can watch that light, or `-1` when
/// the level has no light switch / no live sim. Never invoked by gameplay.
#[wasm_bindgen]
pub fn debug_face_nearest_light_switch() -> i32 {
    GAME_STATE.with(|cell| {
        let handle = cell.borrow().clone();
        match handle {
            Some(state_rc) => {
                let mut state = state_rc.borrow_mut();
                match state.sim.as_mut() {
                    Some(sim) => sim
                        .debug_face_nearest_light_switch()
                        .map(|idx| idx as i32)
                        .unwrap_or(-1),
                    None => -1,
                }
            }
            None => -1,
        }
    })
}

/// DEBUG-ONLY WASM hook: read the current intensity (0.0..=1.0) of a light by
/// `light_index`. Exposed as `window.__marathonDebug.lightIntensity(idx)`.
/// Returns `1.0` (the renderer's no-light fallback) for unknown indices and
/// when there is no live sim, so the e2e compares the *delta* across an
/// action-key press rather than an absolute value.
#[wasm_bindgen]
pub fn debug_light_intensity(light_index: u32) -> f32 {
    GAME_STATE.with(|cell| {
        let handle = cell.borrow().clone();
        match handle {
            Some(state_rc) => {
                let mut state = state_rc.borrow_mut();
                match state.sim.as_mut() {
                    Some(sim) => sim
                        .light_intensities()
                        .get(light_index as usize)
                        .copied()
                        .unwrap_or(1.0),
                    None => 1.0,
                }
            }
            None => 1.0,
        }
    })
}

/// DEBUG-ONLY WASM hook: drive the action-key → light-switch toggle through the
/// real sim path and report the controlled light's intensity straddling the
/// toggle. Exposed as `window.__marathonDebug.toggleNearestLightSwitch()`,
/// returning `[light_index, intensity_before, intensity_after]` (length 3) on
/// success, or an empty array when there is no light switch / no live sim.
///
/// Marathon lights auto-cycle every tick, so a switch-driven light never holds
/// a value an e2e can poll between presses; this measures the toggle atomically
/// in the sim (faces the switch, runs one ACTION-rising-edge tick, reads the
/// snapped intensity) so the assertion is deterministic. The full action path
/// (find_action_key_target → ToggleLight) is exercised exactly as a Space press
/// would, and the door half of the same test proves the keyboard→action wiring.
#[wasm_bindgen]
pub fn debug_toggle_nearest_light_switch() -> js_sys::Array {
    let out = js_sys::Array::new();
    GAME_STATE.with(|cell| {
        let handle = cell.borrow().clone();
        if let Some(state_rc) = handle {
            let mut state = state_rc.borrow_mut();
            if let Some(sim) = state.sim.as_mut() {
                if let Some((idx, before, after)) = sim.debug_toggle_nearest_light_switch() {
                    out.push(&JsValue::from_f64(idx as f64));
                    out.push(&JsValue::from_f64(before as f64));
                    out.push(&JsValue::from_f64(after as f64));
                }
            }
        }
    });
    out
}

/// Install the `window.__marathonDebug` namespace (creating it if absent) and
/// attach `faceNearestDoor` so e2e tests can deterministically face a door.
fn install_debug_hooks() {
    let global = js_sys::global();
    let window: JsValue = global.into();
    // window.__marathonDebug ||= {}
    let key = JsValue::from_str("__marathonDebug");
    let ns = js_sys::Reflect::get(&window, &key).unwrap_or(JsValue::UNDEFINED);
    let ns_obj: js_sys::Object = if ns.is_object() {
        ns.unchecked_into()
    } else {
        let o = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&window, &key, &o);
        o
    };

    // __marathonDebug.faceNearestDoor = () => debug_face_nearest_door()
    let closure = Closure::<dyn Fn() -> bool>::new(debug_face_nearest_door);
    let _ = js_sys::Reflect::set(
        &ns_obj,
        &JsValue::from_str("faceNearestDoor"),
        closure.as_ref().unchecked_ref(),
    );
    // Leak the closure so it stays alive for the lifetime of the page.
    closure.forget();

    // __marathonDebug.faceNearestLightSwitch = () => debug_face_nearest_light_switch()
    let ls_closure = Closure::<dyn Fn() -> i32>::new(debug_face_nearest_light_switch);
    let _ = js_sys::Reflect::set(
        &ns_obj,
        &JsValue::from_str("faceNearestLightSwitch"),
        ls_closure.as_ref().unchecked_ref(),
    );
    ls_closure.forget();

    // __marathonDebug.lightIntensity = (idx) => debug_light_intensity(idx)
    let li_closure = Closure::<dyn Fn(u32) -> f32>::new(debug_light_intensity);
    let _ = js_sys::Reflect::set(
        &ns_obj,
        &JsValue::from_str("lightIntensity"),
        li_closure.as_ref().unchecked_ref(),
    );
    li_closure.forget();

    // __marathonDebug.toggleNearestLightSwitch = () => [idx, before, after]
    let tl_closure = Closure::<dyn Fn() -> js_sys::Array>::new(debug_toggle_nearest_light_switch);
    let _ = js_sys::Reflect::set(
        &ns_obj,
        &JsValue::from_str("toggleNearestLightSwitch"),
        tl_closure.as_ref().unchecked_ref(),
    );
    tl_closure.forget();
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [f32; 16],
    camera_yaw: f32,
    camera_pitch: f32,
    elapsed_time: f32,
    _padding: f32,
    camera_position: [f32; 3],
    _padding2: f32,
}

const EYE_HEIGHT: f32 = 0.66;
const TICKS_PER_SECOND: f64 = 30.0;
const MOUSE_SENSITIVITY: f64 = 0.003;
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
        )
        .normalize();
        let view = Mat4::look_to_lh(self.position, dir, Vec3::Y);
        let proj = Mat4::perspective_lh(90.0_f32.to_radians(), aspect, 0.1, 1000.0);
        let vp = proj * view;
        CameraUniform {
            view_proj: vp.to_cols_array(),
            camera_yaw: self.yaw,
            camera_pitch: self.pitch,
            elapsed_time: elapsed,
            _padding: 0.0,
            camera_position: self.position.to_array(),
            _padding2: 0.0,
        }
    }
}

struct InputState {
    forward: bool,
    backward: bool,
    strafe_left: bool,
    strafe_right: bool,
    fire_primary: bool,
    fire_secondary: bool,
    action: bool,
    mouse_dx: f64,
    mouse_dy: f64,
    toggle_map: bool,
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
            toggle_map: false,
        }
    }

    fn to_action_flags(&self) -> ActionFlags {
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
        ActionFlags::new(bits)
    }

    /// Clear both fire inputs. Called when pointer lock is lost so a held fire
    /// can't stay latched after the player tabs/clicks away (which would make
    /// the weapon keep firing — and the muzzle-flash sprite keep showing —
    /// with no way to release it).
    fn clear_fire(&mut self) {
        self.fire_primary = false;
        self.fire_secondary = false;
    }

    fn to_mouse_delta(&mut self) -> (f32, f32) {
        // Yaw: positive mouse_dx (rightward) increases sim facing.
        // cam.yaw = -facing then decreases, turning camera toward render -Z = Marathon RIGHT.
        // Pitch: negate mouse_dy so mouse-up (negative dy) produces positive pitch = look up.
        let yaw = self.mouse_dx as f32;
        let pitch = -(self.mouse_dy as f32);
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        (yaw, pitch)
    }
}

/// Whether a primary/secondary fire mousedown should be registered as a weapon
/// fire, given whether the canvas currently owns the pointer lock.
///
/// The click that *acquires* pointer lock (mouse capture) arrives while the
/// canvas does NOT yet own the lock; treating that click as a fire makes the
/// weapon discharge the instant the player clicks to take mouse control (and
/// leaves the muzzle-flash sprite on screen). Once the canvas owns the lock,
/// subsequent clicks are real fire intents. So: only register fire while the
/// pointer is already locked to the canvas.
fn fire_allowed_while(pointer_locked: bool) -> bool {
    pointer_locked
}

#[derive(Clone)]
struct RenderableEntity {
    position: Vec3,
    facing: f32,
    shape: u16,
    frame: u16,
}

struct EntitySnapshots {
    prev: Vec<RenderableEntity>,
    curr: Vec<RenderableEntity>,
}
impl EntitySnapshots {
    fn new() -> Self {
        Self {
            prev: Vec::new(),
            curr: Vec::new(),
        }
    }
    fn advance(&mut self, new: Vec<RenderableEntity>) {
        std::mem::swap(&mut self.prev, &mut self.curr);
        self.curr = new;
    }
    fn interpolated(&self, alpha: f32) -> Vec<RenderableEntity> {
        self.curr
            .iter()
            .enumerate()
            .map(|(i, e)| {
                if let Some(p) = self.prev.get(i) {
                    RenderableEntity {
                        position: p.position.lerp(e.position, alpha),
                        facing: p.facing + (e.facing - p.facing) * alpha,
                        shape: e.shape,
                        frame: e.frame,
                    }
                } else {
                    e.clone()
                }
            })
            .collect()
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
    /// Per-polygon dynamic data texture (Rgba32Float, 2 texels/polygon, one
    /// row per polygon). Sized for `map.polygons.len()` at level load and
    /// rewritten each frame; vertex/index buffers stay static.
    poly_data_texture: wgpu::Texture,
    poly_data_bind_group: wgpu::BindGroup,
    poly_data_bgl: wgpu::BindGroupLayout,
    poly_count: usize,
    draw_batches: Vec<mesh::DrawBatch>,
    texture_manager: TextureManager,
    fallback_bind_group: wgpu::BindGroup,
    texture_bgl: wgpu::BindGroupLayout,
    sprite_renderer: SpriteRenderer,
    weapon_overlay: crate::sprites::WeaponOverlayRenderer,
    sim: Option<SimWorld>,
    /// Most recent per-frame render snapshot (box 3.1). Taken once per sim tick
    /// in `frame()` and consumed by the camera, entity, weapon-overlay,
    /// data-texture, and HUD paths in place of scattered per-frame accessors.
    latest_snapshot: Option<WorldSnapshot>,
    shapes_file: ShapesFile,
    prev_camera: CameraState,
    curr_camera: CameraState,
    input: InputState,
    entity_snapshots: EntitySnapshots,
    last_frame_ms: f64,
    tick_accum_ms: f64,
    start_ms: f64,
    aspect: f32,
    hud_update_counter: u32,
    automap_visible: bool,
    map_lines: Vec<([f32; 2], [f32; 2])>,
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
            let (mouse_yaw, mouse_pitch) = self.input.to_mouse_delta();
            let tick_input = TickInput {
                action_flags: flags,
                mouse_yaw,
                mouse_pitch,
            };
            if let Some(ref mut sim) = self.sim {
                sim.tick(tick_input);
                // Box 3.1: a single render_snapshot() per frame replaces the
                // scattered player_*/entities()/drain_events() accessor calls.
                // Camera, entity snapshots, weapon overlay, the data-texture
                // upload, and the HUD all read from this one snapshot below
                // (drain_events runs inside render_snapshot, matching the old
                // once-per-frame drain).
                let snapshot = sim.render_snapshot();
                if let Some(ref player) = snapshot.player {
                    let pos = player.position;
                    self.curr_camera.position = Vec3::new(pos.x, pos.z + EYE_HEIGHT, -pos.y);
                    self.curr_camera.yaw = -player.facing;
                    self.curr_camera.pitch = player.vertical_look;
                }
                let entities: Vec<RenderableEntity> = snapshot
                    .entities
                    .iter()
                    .map(|e| RenderableEntity {
                        position: Vec3::new(e.position.x, e.position.z, -e.position.y), // sim→render: negate Z to fix mirror
                        facing: -e.facing,
                        shape: e.shape,
                        frame: e.frame,
                    })
                    .collect();
                self.entity_snapshots.advance(entities);
                self.latest_snapshot = Some(snapshot);
            }
        }

        let alpha = (self.tick_accum_ms / TICK_DURATION_MS) as f32;
        let mut cam = self.prev_camera.lerp(&self.curr_camera, alpha);
        // Apply pending mouse delta (not yet consumed by a sim tick) directly
        // to the rendered camera so mouse look reflects immediately. The sim
        // remains authoritative: on the next tick `to_mouse_delta()` consumes
        // these and updates `curr_camera.yaw/pitch` to match, so the preview
        // transitions seamlessly.
        cam.yaw -= self.input.mouse_dx as f32;
        let pitch_limit = std::f32::consts::FRAC_PI_6; // ~30° matches Marathon's maximum_elevation
        cam.pitch = (cam.pitch + (-self.input.mouse_dy as f32)).clamp(-pitch_limit, pitch_limit);
        let uniform = cam.to_uniform(self.aspect, elapsed);

        // Build sprite draw calls
        let sprites: Vec<SpriteDrawCall> = self
            .entity_snapshots
            .interpolated(alpha)
            .iter()
            .filter_map(|e| {
                let view = crate::sprites::compute_view_angle(cam.position, e.position, e.facing);
                let (bi, wl, wr, wt, wb) = crate::sprites::resolve_entity_sprite(
                    &self.shapes_file,
                    e.shape,
                    0,
                    e.frame,
                    view,
                )?;
                Some(SpriteDrawCall {
                    position: e.position,
                    world_left: wl,
                    world_right: wr,
                    world_top: wt,
                    world_bottom: wb,
                    collection: e.shape,
                    bitmap_index: bi,
                    tint: 1.0,
                })
            })
            .collect();

        // Resolve weapon sprite for screen-space overlay (rendered after main
        // pass). Box 3.1: sourced from the per-frame snapshot's weapon field
        // instead of a separate player_weapon_state() accessor call.
        let weapon_sprite = self
            .latest_snapshot
            .as_ref()
            .and_then(|s| s.weapon.as_ref())
            .and_then(|ws| {
                let (bi, wl, wr, wt, wb) = crate::sprites::resolve_entity_sprite(
                    &self.shapes_file,
                    ws.collection,
                    ws.shape,
                    ws.frame,
                    0,
                )?;
                Some((
                    ws.collection,
                    bi,
                    wl,
                    wr,
                    wt,
                    wb,
                    ws.vertical_position,
                    ws.horizontal_position,
                ))
            });

        // Render
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
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
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &self.camera_bind_group, &[]);
            rp.set_bind_group(2, &self.poly_data_bind_group, &[]);
            rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rp.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            // Draw geometry in batches, binding the correct texture collection for each
            for batch in &self.draw_batches {
                let bg = self
                    .texture_manager
                    .gpu_textures
                    .get(&batch.collection_index)
                    .map(|t| &t.bind_group)
                    .unwrap_or(&self.fallback_bind_group);
                rp.set_bind_group(1, bg, &[]);
                rp.draw_indexed(
                    batch.index_start..batch.index_start + batch.index_count,
                    0,
                    0..1,
                );
            }
            self.sprite_renderer.render(
                &self.device,
                &mut rp,
                &self.camera_bind_group,
                cam.position,
                cam.yaw,
                &sprites,
            );
        }
        // Weapon overlay pass (no depth test, screen-space)
        if let Some((coll, bi, wl, wr, wt, wb, vert_pos, horiz_pos)) = weapon_sprite {
            let tex_bg = self
                .sprite_renderer
                .collections
                .get(&coll)
                .map(|c| &c.bind_group)
                .unwrap_or(
                    &self
                        .sprite_renderer
                        .collections
                        .values()
                        .next()
                        .map(|c| &c.bind_group)
                        .unwrap_or(&self.fallback_bind_group),
                );
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("weapon_overlay_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.weapon_overlay.render(
                &self.device,
                &mut rp,
                &self.camera_bind_group,
                tex_bg,
                bi,
                wl,
                wr,
                wt,
                wb,
                vert_pos,
                horiz_pos,
                1.0,
                self.config.width as f32,
                self.config.height as f32,
            );
        }
        // Per-frame dynamic poly-data upload (box 4.2). Fed from the single
        // `render_snapshot().poly_dynamic` taken in the tick loop above (box
        // 3.1) instead of a separate `poly_dynamic_data()` accessor call. The
        // snapshot's per-polygon floor/ceiling/media heights and animated light
        // intensities rewrite the data texture, so doors/platforms/light/media
        // animate without re-baking geometry. This is the ONLY per-frame
        // geometry-driving GPU write: the vertex and index buffers are created
        // once in `load_level_into` and are NEVER recreated here in `frame()` —
        // only `write_texture` (inside `write_poly_data_texture`) runs per
        // frame. Box 4.2's buffer-stability constraint is enforced by that
        // invariant.
        if let Some(ref snapshot) = self.latest_snapshot {
            let mapped = crate::poly_data::poly_dyn_data_from_snapshot(snapshot);
            crate::poly_data::write_poly_data_texture(
                &self.queue,
                &self.poly_data_texture,
                &mapped,
            );
        }

        self.queue.submit(std::iter::once(enc.finish()));
        output.present();

        // HUD update (throttled to ~10fps = every 3 ticks at 30fps). Box 3.1:
        // player health/shield/oxygen/facing come from the per-frame snapshot's
        // player view. `player_weapon_info` and `nearby_entities` are not part
        // of the WorldSnapshot contract, so they remain direct sim queries.
        self.hud_update_counter += 1;
        if self.hud_update_counter.is_multiple_of(3) {
            if let (Some(player), Some(sim)) = (
                self.latest_snapshot.as_ref().and_then(|s| s.player.clone()),
                self.sim.as_mut(),
            ) {
                let weapon_info = sim.player_weapon_info();
                let entities = sim.nearby_entities(8.0);
                update_hud(
                    player.health,
                    player.shield,
                    player.oxygen,
                    weapon_info,
                    player.facing,
                    &entities,
                );
            }
        }

        // Automap toggle
        if self.input.toggle_map {
            self.input.toggle_map = false;
            self.automap_visible = !self.automap_visible;
            if let Some(el) = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("automap-canvas")
            {
                let _ = el.set_attribute("style", if self.automap_visible {
                    "display:block;position:fixed;inset:0;z-index:4;background:rgba(0,0,0,0.7);pointer-events:none"
                } else {
                    "display:none"
                });
            }
        }

        // Automap rendering
        if self.automap_visible {
            draw_automap(&self.map_lines, cam.position.x, cam.position.z, cam.yaw);
        }
    }
}

fn update_hud(
    health: i16,
    shield: i16,
    oxygen: i16,
    weapon_info: Option<(usize, u16, u16)>,
    player_yaw: f32,
    entities: &[(f32, f32, u8)],
) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    // Show HUD (grid layout)
    if let Some(hud) = document.get_element_by_id("hud") {
        let _ = hud.set_attribute("style", "display: grid");
    }

    let max_health: f32 = 150.0;
    let max_shield: f32 = 150.0;
    let max_oxygen: f32 = 600.0;

    let health_pct = (health as f32 / max_health * 100.0).clamp(0.0, 100.0);
    let shield_pct = (shield as f32 / max_shield * 100.0).clamp(0.0, 100.0);
    let oxygen_pct = (oxygen as f32 / max_oxygen * 100.0).clamp(0.0, 100.0);

    // Health bar color tiers
    let health_color = if health_pct > 66.0 {
        "#4a4"
    } else if health_pct > 33.0 {
        "#aa4"
    } else {
        "#a44"
    };

    if let Some(el) = document.get_element_by_id("health-fill") {
        let _ = el.set_attribute(
            "style",
            &format!("width:{health_pct:.0}%;background:{health_color}"),
        );
    }
    if let Some(el) = document.get_element_by_id("health-val") {
        el.set_text_content(Some(&format!("{health}")));
    }
    if let Some(el) = document.get_element_by_id("shield-fill") {
        let _ = el.set_attribute("style", &format!("width:{shield_pct:.0}%;background:#48c"));
    }
    if let Some(el) = document.get_element_by_id("shield-val") {
        el.set_text_content(Some(&format!("{shield}")));
    }

    // Oxygen: hide when full
    if let Some(el) = document.get_element_by_id("oxygen-group") {
        let _ = el.set_attribute(
            "style",
            if oxygen >= 600 {
                "display:none"
            } else {
                "display:flex"
            },
        );
    }
    if let Some(el) = document.get_element_by_id("oxygen-fill") {
        let _ = el.set_attribute("style", &format!("width:{oxygen_pct:.0}%;background:#4ac"));
    }
    if let Some(el) = document.get_element_by_id("oxygen-val") {
        el.set_text_content(Some(&format!("{oxygen}")));
    }

    // Weapon display via JS
    if let Some((def_idx, pri, sec)) = weapon_info {
        let _ = js_sys::eval(&format!(
            "window.updateWeaponDisplay({},{},{})",
            def_idx, pri as i32, sec as i32
        ));
    }

    // Motion sensor via JS
    let entity_arr: Vec<f32> = entities
        .iter()
        .flat_map(|(x, z, t)| vec![*x, *z, *t as f32])
        .collect();
    let js_arr = js_sys::Float32Array::new_with_length(entity_arr.len() as u32);
    js_arr.copy_from(&entity_arr);
    let _ = js_sys::Reflect::set(
        &wasm_bindgen::JsValue::from(js_sys::global()),
        &wasm_bindgen::JsValue::from_str("_sensorData"),
        &js_arr,
    );
    let _ = js_sys::eval(&format!(
        "window.updateMotionSensor({},window._sensorData)",
        player_yaw
    ));
}

fn draw_automap(lines: &[([f32; 2], [f32; 2])], player_x: f32, player_z: f32, player_yaw: f32) {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = match document.get_element_by_id("automap-canvas") {
        Some(el) => el.dyn_into::<web_sys::HtmlCanvasElement>().unwrap(),
        None => return,
    };
    let w = canvas.client_width() as u32;
    let h = canvas.client_height() as u32;
    canvas.set_width(w);
    canvas.set_height(h);

    let ctx: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap();

    ctx.clear_rect(0.0, 0.0, w as f64, h as f64);

    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let scale = 12.0; // pixels per world unit

    // Draw lines
    ctx.set_stroke_style_str("#4a9");
    ctx.set_line_width(1.0);
    ctx.begin_path();
    for &(a, b) in lines {
        let ax = cx + (a[0] - player_x) as f64 * scale;
        let ay = cy + (a[1] - player_z) as f64 * scale;
        let bx = cx + (b[0] - player_x) as f64 * scale;
        let by = cy + (b[1] - player_z) as f64 * scale;
        ctx.move_to(ax, ay);
        ctx.line_to(bx, by);
    }
    ctx.stroke();

    // Draw player marker (arrow)
    ctx.set_fill_style_str("#ff4");
    ctx.save();
    ctx.translate(cx, cy).unwrap();
    ctx.rotate(-player_yaw as f64).unwrap();
    ctx.begin_path();
    ctx.move_to(0.0, -8.0);
    ctx.line_to(-5.0, 6.0);
    ctx.line_to(5.0, 6.0);
    ctx.close_path();
    ctx.fill();
    ctx.restore();
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

    log::info!(
        "Parsed: {} levels, physics={}",
        map_wad.entry_count(),
        physics_data.is_some()
    );

    // Get canvas
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("marathon-canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

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

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or("No GPU adapter found")?;

    log::info!("GPU: {}", adapter.get_info().name);

    let mut limits = wgpu::Limits::downlevel_webgl2_defaults();
    limits.max_color_attachments = 1;
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("dev"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .map_err(|e| format!("Device error: {e}"))?;

    let caps = surface.get_capabilities(&adapter);
    let format = caps
        .formats
        .first()
        .copied()
        .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let (_, depth_view) = create_depth(&device, width, height);

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    // Bind group layouts
    let cam_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
    let tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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

    let poly_data_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("poly_data_bgl"),
        entries: &crate::poly_data::data_texture_bgl_entries(),
    });

    let cam_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &[0u8; std::mem::size_of::<CameraUniform>()],
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let cam_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &cam_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: cam_buf.as_entire_binding(),
        }],
    });

    // Fallback texture
    let fb = create_fallback_texture(&device, &queue, &tex_bgl);

    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&cam_bgl, &tex_bgl, &poly_data_bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pl),
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
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview: None,
        cache: None,
    });

    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &[0u8; 4],
        usage: wgpu::BufferUsages::VERTEX,
    });
    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &[0u8; 4],
        usage: wgpu::BufferUsages::INDEX,
    });

    let sprite_renderer = SpriteRenderer::new(&device, &queue, &cam_bgl, format);
    let weapon_overlay = crate::sprites::WeaponOverlayRenderer::new(
        &device,
        &cam_bgl,
        crate::sprites::WeaponOverlayRenderer::texture_bgl(&sprite_renderer),
        format,
    );

    // Placeholder 1-polygon data texture; resized in load_level_into.
    let (poly_data_texture, poly_data_bind_group) =
        create_poly_data_texture(&device, &poly_data_bgl, 1);

    let cam = CameraState {
        position: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
    };
    let now = js_sys::Date::now();

    let mut state = GameState {
        surface,
        device,
        queue,
        config,
        depth_view,
        pipeline,
        camera_buffer: cam_buf,
        camera_bind_group: cam_bg,
        vertex_buffer: vb,
        index_buffer: ib,
        num_indices: 0,
        poly_data_texture,
        poly_data_bind_group,
        poly_data_bgl,
        poly_count: 1,
        draw_batches: Vec::new(),
        texture_manager: TextureManager {
            collections: Default::default(),
            gpu_textures: Default::default(),
        },
        fallback_bind_group: fb,
        texture_bgl: tex_bgl,
        sprite_renderer,
        weapon_overlay,
        sim: None,
        latest_snapshot: None,
        shapes_file,
        prev_camera: cam,
        curr_camera: cam,
        input: InputState::new(),
        entity_snapshots: EntitySnapshots::new(),
        last_frame_ms: now,
        tick_accum_ms: 0.0,
        start_ms: now,
        aspect: width as f32 / height.max(1) as f32,
        hud_update_counter: 0,
        automap_visible: false,
        map_lines: Vec::new(),
    };

    // Load level 0
    load_level_into(&mut state, &map_wad, physics_data.as_ref(), 0);

    // Set up input handlers on canvas
    let state = Rc::new(RefCell::new(state));
    setup_input_handlers(&canvas, state.clone());

    // Stash the state handle and install the debug WASM hooks
    // (window.__marathonDebug.faceNearestDoor) for deterministic e2e tests.
    GAME_STATE.with(|cell| *cell.borrow_mut() = Some(state.clone()));
    install_debug_hooks();

    // Start render loop via requestAnimationFrame
    start_render_loop(state);

    Ok(())
}

/// Create the per-polygon data texture + bind group, sized for `poly_count`
/// polygons (see `poly_data` module for the 2-texels/polygon layout).
fn create_poly_data_texture(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    poly_count: usize,
) -> (wgpu::Texture, wgpu::BindGroup) {
    let extent = crate::poly_data::data_texture_extent(poly_count)
        .expect("polygon count exceeds WebGL2 max texture dimension");
    let tex = device.create_texture(&crate::poly_data::data_texture_descriptor(extent));
    let view = tex.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("poly_data_sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("poly_data_bind_group"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    (tex, bind_group)
}

fn create_depth(device: &wgpu::Device, w: u32, h: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let v = t.create_view(&Default::default());
    (t, v)
}

fn create_fallback_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let layer_count = crate::texture::pad_layer_count_for_webgl(1);
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: layer_count,
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
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255, 0, 255, 255],
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
    let view = tex.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    })
}

fn load_level_into(
    state: &mut GameState,
    wad: &WadFile,
    physics: Option<&PhysicsData>,
    index: usize,
) {
    let loaded = match level::load_level(wad, index) {
        Ok(l) => l,
        Err(e) => {
            log::error!("Load level {index}: {e}");
            return;
        }
    };
    log::info!("Level: {}", loaded.level_name);
    let map = &loaded.map;

    let poly_info: Vec<mesh::PolygonInfo> = map
        .polygons
        .iter()
        .map(|p| mesh::PolygonInfo {
            floor_light: level::evaluate_light_intensity(&map.lights, p.floor_lightsource_index),
            floor_transfer_mode: p.floor_transfer_mode as u32,
            ceiling_light: level::evaluate_light_intensity(
                &map.lights,
                p.ceiling_lightsource_index,
            ),
            ceiling_transfer_mode: p.ceiling_transfer_mode as u32,
        })
        .collect();

    let lm = mesh::build_level_mesh(map, &poly_info);
    let descs = level::collect_texture_descriptors(map);
    let mut tm = TextureManager::load_collections(&state.shapes_file, &descs);

    // Extract map lines for automap
    state.map_lines = map
        .lines
        .iter()
        .map(|line| {
            let a = &map.endpoints[line.endpoint_indexes[0] as usize];
            let b = &map.endpoints[line.endpoint_indexes[1] as usize];
            (
                [a.vertex.x as f32 / 1024.0, a.vertex.y as f32 / 1024.0],
                [b.vertex.x as f32 / 1024.0, b.vertex.y as f32 / 1024.0],
            )
        })
        .collect();

    // Init sim
    if let Some(phys) = physics {
        if let Some(pc) = phys.physics.as_ref().and_then(|p| p.first()) {
            log::info!("Physics: fwd_vel={:.4} accel={:.4} ang_accel={:.4} radius={:.4} height={:.4} step={:.4}",
                pc.maximum_forward_velocity, pc.acceleration, pc.angular_acceleration,
                pc.radius, pc.height, pc.step_delta);
        }
        match SimWorld::new(
            map,
            phys,
            &SimConfig {
                random_seed: 42,
                difficulty: 2,
            },
        ) {
            Ok(sim) => {
                state.sim = Some(sim);
            }
            Err(e) => {
                log::error!("Sim init failed: {e}");
            }
        }
    }

    // Camera — sim coords: (x=mapX, y=mapY, z=vertical), render coords: (X=mapX, Y=vertical, Z=-mapY)
    if let Some(ref mut sim) = state.sim {
        if let Some(pos) = sim.player_position() {
            let c = CameraState {
                position: Vec3::new(pos.x, pos.z + EYE_HEIGHT, -pos.y),
                yaw: -sim.player_facing().unwrap_or(0.0),
                pitch: sim.player_vertical_look().unwrap_or(0.0),
            };
            state.prev_camera = c;
            state.curr_camera = c;
        }
    }

    // GPU upload
    state.vertex_buffer = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&lm.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
    state.index_buffer = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&lm.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
    state.num_indices = lm.indices.len() as u32;
    state.draw_batches = lm.batches;

    // Per-polygon data texture sized for this level's polygon count.
    let poly_count = map.polygons.len();
    let (pdt, pdbg) = create_poly_data_texture(&state.device, &state.poly_data_bgl, poly_count);
    state.poly_data_texture = pdt;
    state.poly_data_bind_group = pdbg;
    state.poly_count = poly_count;
    // Initial upload: real per-polygon heights + light from load (box 2.3).
    // Box 4.2 will drive per-frame updates from the sim.
    let initial = crate::poly_data::build_poly_dyn_data(map);
    crate::poly_data::write_poly_data_texture(&state.queue, &state.poly_data_texture, &initial);

    let sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    tm.create_gpu_textures(&state.device, &state.queue, &state.texture_bgl, &sampler);
    state.texture_manager = tm;

    let colls: Vec<u16> = (0..32).collect();
    state
        .sprite_renderer
        .load_collections(&state.device, &state.queue, &state.shapes_file, &colls);
}

fn setup_input_handlers(canvas: &web_sys::HtmlCanvasElement, state: Rc<RefCell<GameState>>) {
    // Keyboard
    let s = state.clone();
    let keydown =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let mut st = s.borrow_mut();
            match e.code().as_str() {
                "KeyW" | "ArrowUp" => st.input.forward = true,
                "KeyS" | "ArrowDown" => st.input.backward = true,
                "KeyA" => st.input.strafe_left = true,
                "KeyD" => st.input.strafe_right = true,
                "Space" => st.input.action = true,
                "Tab" => {
                    st.input.toggle_map = true;
                    e.prevent_default();
                    return;
                }
                _ => {}
            }
            e.prevent_default();
        });
    canvas
        .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
        .unwrap();
    keydown.forget();

    let s = state.clone();
    let keyup =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
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
    canvas
        .add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref())
        .unwrap();
    keyup.forget();

    // Mouse movement (pointer lock)
    let s = state.clone();
    let mousemove =
        Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let mut st = s.borrow_mut();
            st.input.mouse_dx += e.movement_x() as f64 * MOUSE_SENSITIVITY;
            st.input.mouse_dy += e.movement_y() as f64 * MOUSE_SENSITIVITY;
        });
    canvas
        .add_event_listener_with_callback("mousemove", mousemove.as_ref().unchecked_ref())
        .unwrap();
    mousemove.forget();

    // Click to capture pointer
    let canvas_clone = canvas.clone();
    let click = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |_: web_sys::MouseEvent| {
        canvas_clone.request_pointer_lock();
    });
    canvas
        .add_event_listener_with_callback("click", click.as_ref().unchecked_ref())
        .unwrap();
    click.forget();

    // Mouse buttons. Only register a fire when the canvas already owns the
    // pointer lock: the click that *acquires* the lock (mouse capture) fires
    // BEFORE the lock engages, and treating it as a fire would discharge the
    // weapon — and leave the muzzle flash on screen — the instant the player
    // clicks to take control. See `fire_allowed_while`.
    let s = state.clone();
    // Mouse-button listeners are attached to the DOCUMENT, not the canvas.
    // During pointer lock the canvas can miss the releasing `mouseup`, which
    // left `fire_primary` latched true — the weapon kept firing and the gun
    // stuck in its firing pose after every shot, with no way to release it
    // (clearing on lock-loss doesn't help while the player stays locked). The
    // document reliably receives button events while the pointer is locked.
    let mouse_event_target: web_sys::EventTarget = web_sys::window()
        .and_then(|w| w.document())
        .expect("document for mouse listeners")
        .into();
    let canvas_for_down = canvas.clone();
    let mousedown =
        Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
            let locked = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.pointer_lock_element())
                .map(|el| {
                    let canvas_el: &web_sys::Element = canvas_for_down.as_ref();
                    &el == canvas_el
                })
                .unwrap_or(false);
            if !fire_allowed_while(locked) {
                return;
            }
            let mut st = s.borrow_mut();
            match e.button() {
                0 => st.input.fire_primary = true,
                2 => st.input.fire_secondary = true,
                _ => {}
            }
        });
    mouse_event_target
        .add_event_listener_with_callback("mousedown", mousedown.as_ref().unchecked_ref())
        .unwrap();
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
    mouse_event_target
        .add_event_listener_with_callback("mouseup", mouseup.as_ref().unchecked_ref())
        .unwrap();
    mouseup.forget();

    // When the canvas loses the pointer lock (player tabs/clicks away, presses
    // Esc, etc.) the corresponding `mouseup` may never reach our handler, which
    // would otherwise leave `fire_primary`/`fire_secondary` latched true — the
    // weapon keeps firing and the muzzle-flash sprite stays on screen with no
    // way to release it. Clear both fire inputs on every pointer-lock change
    // where the canvas no longer owns the lock.
    let s = state.clone();
    let canvas_for_lock = canvas.clone();
    let lock_change = Closure::<dyn FnMut()>::new(move || {
        let locked = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.pointer_lock_element())
            .map(|el| {
                let canvas_el: &web_sys::Element = canvas_for_lock.as_ref();
                &el == canvas_el
            })
            .unwrap_or(false);
        if !locked {
            s.borrow_mut().input.clear_fire();
        }
    });
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let _ = document.add_event_listener_with_callback(
            "pointerlockchange",
            lock_change.as_ref().unchecked_ref(),
        );
    }
    lock_change.forget();
}

fn start_render_loop(state: Rc<RefCell<GameState>>) {
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    let s = state.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        s.borrow_mut().frame();
        let window = web_sys::window().unwrap();
        window
            .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
    }));

    let window = web_sys::window().unwrap();
    window
        .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    // Pure-logic unit checks for the fire-gating helpers. These exercise plain
    // Rust (no web-sys), documenting the Bug 1/Bug 2 fire-input contract.

    /// Bug 1: the click that *acquires* pointer lock must NOT fire the weapon.
    /// That click's `mousedown` arrives while the canvas does not yet own the
    /// lock, so fire must be suppressed then and only allowed once locked.
    #[test]
    fn fire_suppressed_until_pointer_locked() {
        assert!(
            !fire_allowed_while(false),
            "the lock-acquiring click (not yet locked) must not fire the weapon"
        );
        assert!(
            fire_allowed_while(true),
            "clicks while the pointer is locked are real fire intents"
        );
    }

    /// A mousedown received before lock is acquired leaves fire untouched;
    /// once locked, the same button press latches fire. This mirrors the
    /// real handler's gate (`if !fire_allowed_while(locked) { return; }`).
    #[test]
    fn mousedown_gate_matches_lock_state() {
        let mut input = InputState::new();

        // Pre-lock click: gate rejects, fire stays clear.
        if fire_allowed_while(false) {
            input.fire_primary = true;
        }
        assert!(!input.fire_primary, "pre-lock click must not set fire");

        // Post-lock click: gate allows, fire latches.
        if fire_allowed_while(true) {
            input.fire_primary = true;
        }
        assert!(input.fire_primary, "post-lock click sets fire");
    }

    /// Bug 2 (latched-fire path): losing the pointer lock must clear any held
    /// fire so the weapon stops firing and the muzzle-flash sprite stops
    /// re-showing even when the releasing `mouseup` never reaches us.
    #[test]
    fn clear_fire_releases_both_triggers() {
        let mut input = InputState::new();
        input.fire_primary = true;
        input.fire_secondary = true;
        input.clear_fire();
        assert!(!input.fire_primary);
        assert!(!input.fire_secondary);
    }
}
