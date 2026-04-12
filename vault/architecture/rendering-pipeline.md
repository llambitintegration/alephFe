---
tags: [architecture, rendering, wgpu, shaders]
---

# Rendering Pipeline

Both `marathon-game` (desktop) and `marathon-web` (WASM) use `wgpu` for GPU-accelerated rendering. They share the same conceptual pipeline but differ in buffer strategy due to WebGL2 limitations.

## Rendering Architecture Overview

```
MapData + ShapesFile
       |
       v
  Mesh Generation (CPU)
  - build_level_mesh(): floors, ceilings, walls, media surfaces
  - Fan triangulation for polygonal floors/ceilings
  - Quad generation for wall sides
       |
       v
  Texture Loading (CPU -> GPU)
  - Decode Shapes bitmaps (indexed color -> RGBA8)
  - Upload as 2D texture arrays (one per collection)
  - Sampler with repeat addressing
       |
       v
  GPU Buffers
  - Vertex buffer (position, UV, polygon index, texture descriptor)
  - Index buffer (u32)
  - Camera uniform buffer (view_proj, yaw, pitch, elapsed_time)
  - Per-polygon storage buffer (desktop) or per-vertex bake (web)
       |
       v
  Render Passes
  Pass 1: Level geometry (shader.wgsl)
  Pass 2: Entity sprites (sprite_shader.wgsl, shared depth buffer)
```

## Vertex Format

### Desktop (marathon-game)

```rust
struct Vertex {           // 24 bytes
    position: [f32; 3],   // World-space XYZ
    uv: [f32; 2],         // Texture coordinates
    polygon_index: u32,   // Index into polygon storage buffer
    texture_descriptor: u32, // ShapeDescriptor (collection:shape)
}
```

The polygon_index is used in the shader to look up per-polygon data from a storage buffer.

### Web (marathon-web)

```rust
struct Vertex {           // 32 bytes
    position: [f32; 3],
    uv: [f32; 2],
    texture_descriptor: u32,
    light: f32,           // Pre-baked light intensity
    transfer_mode: u32,   // Pre-baked transfer mode
}
```

Light and transfer mode are baked per-vertex because WebGL2 does not reliably support storage buffers. The web version also uses `DrawBatch` structs to group indices by collection for batched draw calls.

## Shader Architecture

### Level Geometry Shader (shader.wgsl)

**Bind Groups:**

| Group | Binding | Desktop | Web |
|-------|---------|---------|-----|
| 0 | 0 | Camera uniform | Camera uniform |
| 1 | 0 | Polygon storage buffer | Texture array |
| 2 | 0 | Texture array | Texture sampler |
| 2 | 1 | Texture sampler | -- |

Desktop uses 3 bind groups (camera, polygon data, textures). Web uses 2 (camera, textures), since light/transfer are per-vertex.

**Vertex Shader:** Projects world-space position through the camera view-projection matrix. Passes UV, polygon index (desktop) or light/transfer (web), texture descriptor, and world position to the fragment shader.

**Fragment Shader:**
1. Decodes ShapeDescriptor to get the texture array layer index: `shape_index = descriptor & 0xFF`
2. Reads lighting: from polygon storage buffer (desktop) or interpolated vertex attribute (web)
3. Applies transfer mode to UVs:
   - `TRANSFER_NORMAL (0)` -- No modification
   - `TRANSFER_PULSATE (1)` -- Scale UVs around center with sin(time)
   - `TRANSFER_WOBBLE (2)` -- Offset UVs with sin/cos based on world position and time
   - `TRANSFER_SLIDE (4)` -- Scroll UVs in U direction over time
   - `TRANSFER_STATIC (6)` -- Replace texture entirely with hash-based noise
   - `TRANSFER_LANDSCAPE (9)` -- UV derived from camera yaw/pitch (sky rendering)
4. Samples the texture array at the computed UV and layer
5. Discards pixels with alpha < 0.01
6. Returns `rgb * light` with original alpha

### Sprite Billboard Shader (sprite_shader.wgsl)

Renders entity sprites as camera-facing quads. Same shader on both desktop and web.

**Bind Groups:**
- Group 0, Binding 0: Camera uniform (shared with level geometry)
- Group 1, Binding 0: Sprite texture array
- Group 1, Binding 1: Sprite sampler

**Vertex Format:**
```rust
struct SpriteVertex {
    position: [f32; 3],  // Pre-computed world-space quad corner
    uv: [f32; 2],
    tex_index: u32,       // Texture array layer
    tint: f32,            // Light multiplier
}
```

The sprite vertices are computed CPU-side each frame. The quad corners are billboarded toward the camera using yaw-only rotation (no pitch billboarding).

**Fragment Shader:** Samples texture array, alpha-tests, applies tint.

## Mesh Generation

`build_level_mesh()` in `mesh.rs` converts Marathon MapData into GPU-ready triangles:

### Floors and Ceilings

Each polygon is triangulated using fan triangulation from vertex 0:
```
For polygon with vertices [v0, v1, v2, v3, v4]:
  Triangle 1: v0, v2, v1  (floor: CW winding for downward normal)
  Triangle 2: v0, v3, v2
  Triangle 3: v0, v4, v3
  
  Ceiling uses opposite winding (v0, v1, v2) for upward normal.
```

UV coordinates are computed from the endpoint position minus the polygon's floor/ceiling_origin, divided by 1024 (1 WU = 1 texture repeat).

### Walls

Walls are generated per-line, per-side. A line can have:
- A clockwise side (facing one polygon owner)
- A counterclockwise side (facing the opposite polygon owner)

Each side has a `side_type` that determines which wall sections to generate:

| Side Type | Sections |
|-----------|----------|
| 0 (Full) | Primary texture: floor to ceiling |
| 1 (High) | Primary: adjacent ceiling to owner ceiling |
| 2 (Low) | Primary: owner floor to adjacent floor |
| 3, 4 (Split) | Primary (high), Secondary (low), Transparent (middle) |

Each section emits a 4-vertex quad with 2 triangles.

Wall UVs use the side texture's x0/y0 offset plus the wall length (computed from endpoint distance) and height.

### Media Surfaces

For polygons with a media_index >= 0, a flat surface is generated at the media height using the media's texture and origin.

### Coordinate System

Marathon uses:
- World units: i16 values where 1024 = 1 WU
- 2D map: X/Y plane
- Height: separate floor_height / ceiling_height

The renderer maps this to a Y-up 3D coordinate system:
- Marathon X -> render X
- Marathon Y -> render Z
- Marathon height -> render Y

## Texture System

### Loading Pipeline

1. `collect_texture_descriptors()` -- Scans all polygon floor/ceiling textures, side textures, and media textures to find referenced ShapeDescriptors
2. `TextureManager::load_collections()` -- For each referenced collection:
   - Loads the Collection from the ShapesFile
   - Selects CLUT 0 (color lookup table)
   - Converts each bitmap: indexed pixels -> RGBA8 using the CLUT
   - Handles column-major vs row-major bitmap storage
   - Transparent pixels (index 0 when bitmap.transparent) get alpha 0
   - Pads all bitmaps to the collection's max width/height
3. `create_gpu_textures()` -- For each loaded collection:
   - Creates a 2D texture array (one layer per bitmap)
   - Uploads RGBA8 data per layer
   - Creates a texture view with D2Array dimension
   - Creates a bind group with the texture view + sampler

### Texture Descriptor Decoding

`ShapeDescriptor` is a u16 packed as:
- Bits [12:8]: collection index (0-31)
- Bits [7:0]: shape index within collection (used as texture array layer)

In the shader: `shape_index = descriptor & 0xFF`

## Camera System

Both desktop and web use the same first-person camera:

```rust
struct CameraState {
    position: Vec3,  // World-space eye position
    yaw: f32,        // Horizontal angle (radians, Marathon facing)
    pitch: f32,      // Vertical angle (radians, positive = up)
}
```

The view direction is computed as:
```
dir = (cos(yaw) * cos(pitch), sin(pitch), sin(yaw) * cos(pitch))
```

Note the coordinate mapping: Marathon X,Y becomes render X,Z and height becomes Y.

Eye position is the player's floor position + `EYE_HEIGHT` (0.66 WU, approximately Marathon's camera_height).

The view-projection matrix uses:
- FOV: 90 degrees (matching Marathon's default)
- Near plane: 0.1
- Far plane: 1000.0
- Right-handed perspective projection

### Camera Interpolation

Camera state is double-buffered (`prev_camera`, `curr_camera`). Between simulation ticks, the rendered camera is linearly interpolated:

```rust
let alpha = tick_accumulator / TICK_DURATION;
let render_camera = prev_camera.lerp(curr_camera, alpha);
```

The web version additionally applies pending mouse deltas to the interpolated camera for immediate visual response before the next sim tick processes them.

## Entity Sprite Rendering

Entity sprites are rendered in a second render pass that shares the depth buffer with level geometry, providing correct occlusion.

### Sprite Resolution Pipeline

1. `sim_world.entities()` returns all active entity render states (position, facing, shape, frame)
2. Entity snapshots are double-buffered and interpolated like the camera
3. For each entity:
   - `compute_view_angle()` determines the angle from camera to entity, relative to entity facing
   - `resolve_entity_sprite()` looks up the correct bitmap from ShapesFile: collection, sequence, frame, view angle
4. `SpriteDrawCall` emitted with position, dimensions, collection, bitmap_index, tint
5. `SpriteRenderer::render()` builds billboarded quads CPU-side and draws them

### Billboarding

Sprite quads are billboarded around the Y axis (yaw only). The quad corners are offset from the entity's world position using the camera's yaw angle to compute perpendicular and vertical offsets.

## Desktop vs Web Rendering Comparison

| Feature | Desktop (marathon-game) | Web (marathon-web) |
|---------|------------------------|-------------------|
| wgpu backend | Native (Vulkan/Metal/DX12) | WebGPU or WebGL2 |
| Window system | winit | web-sys (canvas) |
| Polygon data | Storage buffer (GPU) | Baked per-vertex (CPU) |
| Draw calls | 1 draw (all geometry) | N draws (batched by collection) |
| HUD | wgpu render pass (planned) | HTML/CSS DOM elements |
| Automap | Not yet implemented | HTML canvas 2D context |
| Timing | std::time::Instant | js_sys::Date::now() |
| Audio | marathon-audio (kira) | Not yet implemented |
| Mouse capture | winit confined cursor | Pointer Lock API |
| Shader groups | 3 bind groups | 2 bind groups |
