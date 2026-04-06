## Context

The `marathon-formats` crate can parse Marathon's WAD-based scenario files — map geometry (polygons, lines, sides, endpoints), shape collections (indexed-color bitmaps), and associated metadata (lights, platforms, media). No crate yet exists to render this data.

Marathon uses a 2.5D portal-based geometry model: convex polygons (up to 8 vertices) define floor/ceiling surfaces at independent heights, lines connect polygons, and sides define wall textures between adjacent polygons at different heights. The original C++ engine traverses portals per-frame on the CPU to determine visibility. Modern GPUs with hardware depth testing make this unnecessary for static geometry.

The viewer will be the first real consumer of `marathon-formats` and will establish the rendering architecture reused by later crates.

**Constraints:**
- Must work with Marathon 2 and Marathon Infinity scenario files
- No game logic, simulation, or HUD — purely geometry visualization
- Must run on systems with Vulkan, Metal, or DX12 support (via wgpu backends)

## Goals / Non-Goals

**Goals:**
- Render Marathon levels as navigable 3D environments using wgpu
- Validate that marathon-formats provides sufficient data for rendering
- Establish the mesh generation and texture pipeline patterns for later crates
- Support all four side types (full, high, low, split) and all surface transfer modes
- Animate platforms and media surfaces without mesh rebuilds

**Non-Goals:**
- Game simulation (monsters, weapons, items, physics)
- HUD or UI beyond level selection
- Network/multiplayer support
- Sound or music playback
- Accurate portal-based visibility culling (rely on GPU depth testing instead)
- Marathon 1 format support (Marathon 2+ only)

## Decisions

### D1: Static mesh with dynamic uniforms (not per-frame mesh rebuild)

Convert all polygon floors/ceilings and wall quads into GPU vertex/index buffers at level load time. Platforms and media animate by updating per-polygon uniform data (heights, texture offsets) each frame rather than rebuilding vertex buffers.

**Why over per-frame rebuild:** ~95% of geometry is static. Uploading a few uniform updates per frame is far cheaper than re-triangulating and re-uploading vertex data. This also simplifies the rendering loop.

**Why over full portal traversal:** The original engine uses CPU portal traversal to avoid drawing hidden geometry. With GPU depth buffering this is unnecessary for a viewer — overdraw on modern hardware is cheap compared to the CPU cost of portal traversal for the small polygon counts Marathon uses (typically <1000 polygons per level).

### D2: Fan triangulation for floors/ceilings

Marathon polygons are convex with vertices listed in winding order. Fan triangulation from vertex 0 is correct and trivial: for an N-vertex polygon, emit triangles (0,1,2), (0,2,3), ..., (0,N-2,N-1).

**Why over ear-clipping or Delaunay:** Convexity is guaranteed by the format, so fan triangulation is always valid. No need for a more complex algorithm.

### D3: wgpu for GPU abstraction

Use wgpu with winit for windowing. wgpu provides a Rust-native graphics API that targets Vulkan, Metal, DX12, and WebGPU.

**Why over raw Vulkan/ash:** wgpu is cross-platform without extra boilerplate and is the de facto standard for Rust GPU work. The viewer doesn't need low-level control that raw Vulkan provides.

**Why over OpenGL/glow:** OpenGL is deprecated on macOS and lacks modern features. wgpu is the forward-looking choice.

### D4: Texture arrays per collection

Load each Marathon shape collection's bitmaps into a single 2D texture array. The fragment shader indexes into the array using the shape index from `ShapeDescriptor`. Color tables (CLUTs) are applied CPU-side at load time to produce RGBA pixels.

**Why texture arrays over atlas:** Marathon bitmaps within a collection share dimensions frequently, making arrays natural. Arrays avoid UV bleeding artifacts that atlases require padding to prevent. The `ShapeDescriptor` already encodes collection + shape index, mapping directly to array layer.

**Why CPU-side CLUT application:** Bitmaps are 8-bit indexed color. Applying the CLUT on the CPU during loading produces standard RGBA textures that the GPU can sample normally. This avoids needing a palette texture lookup in the shader and handles the column-major bitmap layout at load time.

### D5: Transfer modes in fragment shader

Implement transfer modes (landscape, slide, pulsate, wobble, static) as fragment shader logic keyed by a per-surface transfer mode ID passed via uniform/push constant. Time-varying modes receive elapsed time as a uniform.

**Why in shader over CPU:** Transfer modes modify UV coordinates or pixel output. Doing this in the shader avoids per-frame CPU work and keeps the geometry static. A single shader with branching on mode ID is simpler than multiple shader variants for the small number of modes.

### D6: Per-polygon uniform buffer for dynamic state

Each polygon gets a slot in a storage buffer containing: current floor/ceiling heights, light intensity, media height, and transfer mode parameters. The vertex shader reads from this buffer using the polygon index. Platform and media animation update only the affected slots each frame.

**Why storage buffer over per-draw uniforms:** A single large buffer avoids per-polygon draw call overhead. The vertex/fragment shaders index into it, enabling one draw call per surface type (floors, ceilings, walls) rather than one per polygon.

### D7: Camera as free-fly (not player-constrained)

The viewer uses unconstrained 3D camera movement (WASD + mouse look) rather than simulating player movement constrained to polygon floors. No collision detection needed.

**Why:** This is a geometry viewer, not a game. Free-fly lets users inspect geometry from any angle, including outside the level bounds — useful for debugging mesh generation.

### D8: Crate structure

Single crate `marathon-viewer` as a binary crate with internal modules:
- `mesh` — geometry conversion (floors, ceilings, walls)
- `texture` — shape collection loading and GPU texture creation
- `render` — wgpu pipeline setup, draw loop, camera
- `transfer` — transfer mode shader logic and uniform management
- `level` — level loading orchestration, platform/media animation

No library crate split — this is an end-user tool, not a reusable library. If rendering logic is needed by other crates later, it can be extracted at that point.

## Risks / Trade-offs

**[Risk] marathon-formats gaps** — The format crate has never been used for rendering. Missing fields, incorrect parsing, or undocumented format quirks may surface.
→ Mitigation: The viewer is explicitly a validation tool. Fix marathon-formats issues as they arise. Start with basic geometry before transfer modes and animation.

**[Risk] Texture dimension mismatches in arrays** — Texture arrays require uniform dimensions per layer. If bitmaps within a collection vary in size, they need resizing or separate arrays.
→ Mitigation: Group bitmaps by dimension within each collection. Use multiple arrays if needed. Marathon textures are typically 128x128 for walls, so variation should be limited.

**[Risk] Transfer mode fidelity** — Marathon's transfer modes are documented informally. Shader implementations may not match original behavior exactly.
→ Mitigation: Implement the common cases (normal, landscape, slide) first. Pulsate, wobble, and static can be approximated and refined with visual comparison against Aleph One.

**[Risk] Large levels and draw call count** — Some user-created levels have thousands of polygons. A naive one-draw-per-surface approach could bottleneck.
→ Mitigation: The storage buffer design (D6) enables batched draw calls. Floors, ceilings, and walls can each be drawn in a single indexed draw call. This should handle even large levels.

## Open Questions

- **Column-major bitmaps:** Marathon stores bitmaps in column-major order. Need to verify whether to transpose at load time or handle in UV mapping. Likely transpose at load time for simplicity.
- **Side texture alignment:** The exact algorithm for computing wall UVs from side texture offsets (x0, y0) and wall geometry needs to be worked out from Aleph One source code.
- **Light intensity mapping:** Marathon lights use function-based intensity animation. Need to determine whether to evaluate light functions on CPU each frame or approximate in shader. CPU evaluation is simpler and lights change infrequently.
