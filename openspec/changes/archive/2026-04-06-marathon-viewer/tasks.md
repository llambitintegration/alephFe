## 1. Crate Scaffolding

- [x] 1.1 Create marathon-viewer crate with Cargo.toml (dependencies: marathon-formats as path dep, wgpu, winit, pollster, glam, clap, bytemuck) and add to workspace
- [x] 1.2 Create main.rs with CLI arg parsing (--map and --shapes paths) and basic error handling
- [x] 1.3 Create module structure: mesh.rs, texture.rs, render.rs, transfer.rs, level.rs

## 2. Window and GPU Setup

- [x] 2.1 Implement winit window creation and wgpu device/surface initialization in render.rs
- [x] 2.2 Implement depth buffer creation and recreation on resize
- [x] 2.3 Implement frame loop with winit event loop (ControlFlow::Poll), handling resize and close events

## 3. Camera System

- [x] 3.1 Implement free-fly camera struct with position, yaw, pitch, and projection matrix (90° FOV, near=0.1, far=1000)
- [x] 3.2 Implement WASD movement relative to camera facing direction
- [x] 3.3 Implement mouse look with yaw/pitch update and pitch clamping
- [x] 3.4 Create camera uniform buffer and bind group for view-projection matrix

## 4. Level Loading

- [x] 4.1 Implement level loading: open map WAD and shapes WAD, enumerate available levels by name
- [x] 4.2 Parse MapData from selected WAD entry and extract polygons, lines, sides, endpoints, lights, platforms, media
- [x] 4.3 Implement level switching (key binding to cycle levels, resource cleanup and reload)

## 5. Mesh Generation

- [x] 5.1 Implement floor triangulation: fan triangulation of convex polygons with floor_height Y, floor_origin UVs, polygon index tagging
- [x] 5.2 Implement ceiling triangulation: same as floor with ceiling_height, reversed winding, ceiling_origin UVs
- [x] 5.3 Implement full wall (side_type=0): quad from floor_height to ceiling_height between line endpoints with primary_texture UVs
- [x] 5.4 Implement high wall (side_type=1): quad from adjacent ceiling to this ceiling with primary_texture
- [x] 5.5 Implement low wall (side_type=2): quad from this floor to adjacent floor with primary_texture
- [x] 5.6 Implement split wall (side_type=3): three quads (low/transparent/high) with secondary/transparent/primary textures
- [x] 5.7 Implement wall UV computation from side texture offsets (x0, y0) and wall dimensions
- [x] 5.8 Build combined vertex buffer (position vec3, UV vec2, polygon_index u32, texture descriptor u32) and index buffer
- [x] 5.9 Implement media surface geometry: flat quad at media height within containing polygon

## 6. Texture Pipeline

- [x] 6.1 Implement shape collection loading: load collections referenced by level textures via ShapesFile
- [x] 6.2 Implement bitmap-to-RGBA conversion: apply CLUT, handle transparency, transpose column-major to row-major
- [x] 6.3 Create wgpu texture arrays per collection (group bitmaps, handle dimension mismatches)
- [x] 6.4 Implement ShapeDescriptor resolution: collection → texture array, LowLevelShape → bitmap_index → array layer
- [x] 6.5 Create texture bind groups with nearest-neighbor filtering and repeat address mode

## 7. Render Pipeline

- [x] 7.1 Write WGSL vertex shader: transform positions by view-projection, pass UVs and polygon_index to fragment stage, read heights from storage buffer
- [x] 7.2 Write WGSL fragment shader: sample texture array, apply light intensity multiplier, branch on transfer mode ID
- [x] 7.3 Create render pipeline with vertex layout, bind group layouts, depth test (Less), and back-face culling
- [x] 7.4 Implement per-polygon storage buffer: create, populate with floor/ceiling heights, light intensities, transfer mode IDs, texture offsets
- [x] 7.5 Implement draw calls: bind pipeline, bind groups, vertex/index buffers, and issue indexed draw

## 8. Lighting

- [x] 8.1 Implement light intensity evaluation from StaticLightData (compute initial intensity from light functions)
- [x] 8.2 Implement OldLightData fallback for Marathon 1-style lights
- [x] 8.3 Write per-frame light update: evaluate animated light functions and update storage buffer entries

## 9. Transfer Modes

- [x] 9.1 Define transfer mode constants (normal=0, pulsate=1, wobble=2, slide=4, static=6, landscape=9)
- [x] 9.2 Implement normal mode in fragment shader (passthrough UV sampling)
- [x] 9.3 Implement landscape mode: UV from view angle (yaw→U, pitch→V) instead of world geometry
- [x] 9.4 Implement slide mode: time-varying UV offset from texture offset values
- [x] 9.5 Implement pulsate mode: sinusoidal UV scale toward surface center
- [x] 9.6 Implement wobble mode: sinusoidal UV distortion based on position and time
- [x] 9.7 Implement static mode: replace texture with per-frame random noise
- [x] 9.8 Pass elapsed time uniform to fragment shader for animated transfer modes

## 10. Platform and Media Animation

- [x] 10.1 Implement platform state tracking: current height, speed, min/max bounds, direction
- [x] 10.2 Implement per-frame platform height update and storage buffer write
- [x] 10.3 Implement media state tracking: current height from MediaData
- [x] 10.4 Implement per-frame media height update and storage buffer write
