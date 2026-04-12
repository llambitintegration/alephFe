---
tags: [architecture, game-loop, state-machine, timing]
---

# Game Loop & State Machine

## Tick Rate

Marathon runs its simulation at a fixed **30 ticks per second** (33.333 ms per tick). This rate is canonical to the original Marathon engine and is preserved in this Rust rebuild for gameplay fidelity.

```rust
const TICKS_PER_SECOND: u64 = 30;
const TICK_DURATION_MICROS: u64 = 1_000_000 / 30; // 33333 us
```

## Game Loop Architecture

The game loop follows the classic "fix your timestep" pattern:

```
while running:
    now = current_time()
    dt = now - last_frame
    last_frame = now
    
    accumulator += dt
    
    // Fixed-step simulation
    while accumulator >= TICK_DURATION:
        accumulator -= TICK_DURATION
        prev_camera = curr_camera
        input = read_input()
        sim.tick(input)
        curr_camera = camera_from_sim()
    
    // Variable-rate rendering with interpolation
    alpha = accumulator / TICK_DURATION
    render_camera = lerp(prev_camera, curr_camera, alpha)
    render(render_camera)
```

### Desktop Implementation (marathon-game)

The desktop version uses `winit`'s `ApplicationHandler` trait:

```rust
struct App {
    // Timing
    start_time: Instant,
    last_frame: Instant,
    tick_accumulator_micros: u64,
    
    // Camera double-buffer
    prev_camera: CameraState,
    curr_camera: CameraState,
    
    // Entity double-buffer
    entity_snapshots: EntitySnapshots,
    
    // Input accumulation
    input: InputState,
    mouse_captured: bool,
    
    // Simulation
    sim: Option<SimWorld>,
    game_state: GameState,  // Playing or Paused
    
    // Deferred level load
    pending_level_load: Option<usize>,
    ...
}
```

The `about_to_wait` handler (called each frame by winit) drives the loop:
1. Compute elapsed time since last frame
2. Accumulate into `tick_accumulator_micros`
3. For each full tick:
   - Save prev_camera
   - Build TickInput from InputState (consuming mouse deltas)
   - Call `sim.tick(input)`
   - Read back camera state from sim
   - Snapshot entity positions for interpolation
   - Drain SimEvents
4. Compute interpolation alpha
5. Build interpolated camera uniform
6. Build sprite draw calls from interpolated entity snapshots
7. Render

### Web Implementation (marathon-web)

The web version uses `requestAnimationFrame` via a closure registered with `web_sys`:

```rust
struct GameState {
    last_frame_ms: f64,
    tick_accum_ms: f64,
    start_ms: f64,
    ...
}
```

Timing uses `js_sys::Date::now()` instead of `std::time::Instant`. The dt is capped at 100ms to prevent spiral-of-death on tab switches.

A key difference: the web version applies pending mouse deltas directly to the rendered camera (preview) before the sim tick consumes them. This eliminates one frame of mouse latency:

```rust
cam.yaw += self.input.mouse_dx as f32;
cam.pitch = (cam.pitch + self.input.mouse_dy as f32).clamp(-pitch_limit, pitch_limit);
```

The sim remains authoritative -- on the next tick, `to_mouse_delta()` consumes these deltas and updates `curr_camera` to match, so the preview transitions seamlessly.

## Camera Interpolation

Camera state is double-buffered:

```rust
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
}
```

Alpha (0.0 to 1.0) represents how far between the last tick and the next:
- `alpha = 0.0`: render at the previous tick state
- `alpha = 1.0`: render at the current tick state

Camera position comes from the sim player position with a Y-up coordinate swap:
```rust
// sim coords (X, Y, Z_height) -> render coords (X, Z_height + eye, Y)
camera.position = Vec3::new(pos.x, pos.z + EYE_HEIGHT, pos.y);
```

Where `EYE_HEIGHT = 0.66` WU (approximately 680/1024, matching Marathon's camera height).

## Entity Interpolation

Entity rendering uses the same double-buffer pattern:

```rust
struct EntitySnapshots {
    prev: Vec<RenderableEntity>,
    curr: Vec<RenderableEntity>,
}
```

On each tick, `advance()` swaps prev/curr and stores new entity states. At render time, `interpolated(alpha)` lerps each entity's position and facing angle. Entities only in `curr` (newly spawned) use their current position with no interpolation.

## Input System

### Desktop Input

Input state tracks held keys and accumulated mouse deltas:

```rust
struct InputState {
    forward: bool,
    backward: bool,
    strafe_left: bool,
    strafe_right: bool,
    fire_primary: bool,
    fire_secondary: bool,
    action: bool,
    mouse_dx: f64,  // accumulated since last tick
    mouse_dy: f64,
    escape_pressed: bool,
}
```

Key presses from `WindowEvent::KeyboardInput` toggle held state. Mouse movement from `DeviceEvent::MouseMotion` accumulates dx/dy.

At each tick, `to_tick_input()` converts held states to ActionFlags bits and mouse deltas to `mouse_yaw`/`mouse_pitch` (scaled by `MOUSE_SENSITIVITY = 0.003`), then resets the mouse accumulators.

### Web Input

Similar but registers event listeners via `web_sys`:
- Keyboard events on the document
- Mouse events via Pointer Lock API on the canvas

Mouse deltas are consumed slightly differently -- the web version has the extra "preview" step described above.

## Game State Machine

Defined in `marathon-integration/src/types.rs`:

```
                    +--------+
                    | Loading|
                    +--------+
                   /          \
                  v            v
           +---------+    +--------+
           | MainMenu|<-->| Loading|
           +---------+    +--------+
                               |
                               v
                          +---------+
                    +---->| Playing |<----+
                    |     +---------+     |
                    |      /  |   \       |
                    |     v   v    v      |
                 +------+ | +--------+ +-----+
                 |Paused| | |Terminal| |Inter-|
                 +------+ | +--------+ |miss.|
                          |      |     +-----+
                          v      v        |
                     +--------+           |
                     |GameOver|    +------+
                     +--------+    |
                          |        v
                          +-->+--------+
                              |MainMenu|
                              +--------+
```

### Valid Transitions (from `is_valid_transition()`)

| From | To | Trigger |
|------|----|---------|
| Loading | MainMenu | Initial load complete |
| Loading | Playing | Level ready |
| MainMenu | Loading | Start game / load save |
| Playing | Paused | Escape key |
| Paused | Playing | Resume |
| Paused | MainMenu | Quit to menu |
| Playing | Terminal | Walk into terminal polygon |
| Terminal | Playing | Exit terminal |
| Terminal | Intermission | Terminal teleport command |
| Playing | Intermission | Level complete trigger |
| Intermission | Loading | Next level transition |
| Playing | GameOver | Player death / campaign end |
| GameOver | MainMenu | Return to menu |

### TickAccumulator

The integration layer provides a `TickAccumulator` utility:

```rust
struct TickAccumulator {
    accumulated_micros: u64,
}

impl TickAccumulator {
    fn accumulate(&mut self, elapsed_micros: u64) -> u32;  // returns tick count
    fn interpolation_factor(&self) -> f64;                  // 0.0 to 1.0
}
```

This is the canonical timing abstraction. Both renderers implement their own version of this logic inline.

## Simulation Tick Pipeline

Within a single tick, systems execute in this order:

```
1. TickInput resource inserted
2. Player physics
   a. Read input flags + mouse deltas
   b. Compute player-local velocity (axis-decomposed)
   c. Compute facing (keyboard angular velocity + direct mouse yaw)
   d. Compute vertical look (keyboard rate + direct mouse pitch)
   e. Project local velocity to world-space
   f. Apply collision (wall slide, step climb, ceiling check)
   g. Convert post-collision world velocity back to local
3. Monster AI (not yet wired)
4. Weapon/combat (not yet wired)
5. Projectile physics (not yet wired)
6. Damage resolution (not yet wired)
7. World mechanics: platforms, lights, media, items (not yet wired)
8. Tick counter increment
```

The logic for steps 3-7 exists as standalone functions in their respective modules but is not yet called from the `tick()` method. This is the primary integration work remaining.

## Level Loading

Level loading is a multi-step process:

1. Parse the WAD file entry for the target level
2. Extract MapData from the entry's tags
3. Build GPU mesh from MapData
4. Collect all referenced texture descriptors
5. Load texture collections from ShapesFile
6. Create GPU texture arrays
7. Build polygon data (desktop: storage buffer; web: per-vertex bake)
8. Create SimWorld from MapData + PhysicsData
9. Set initial camera from player spawn position

Level transitions are deferred: `pending_level_load` is set by SimEvent::LevelTeleport, then processed at the top of the next frame to avoid mid-tick resource mutation.
