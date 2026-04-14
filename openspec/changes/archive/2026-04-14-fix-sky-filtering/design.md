## Context

The Marathon web renderer currently uses a single wgpu sampler with `FilterMode::Nearest` for all textures. This is configured in `render.rs` (line 697) and passed to `TextureManager::create_gpu_textures()` which binds it into every collection's bind group at binding 1. The fragment shader in `shader.wgsl` uses this single `texture_sampler` for all `textureSample` calls.

For wall and floor textures, nearest-neighbor filtering preserves Marathon's pixel-art aesthetic. However, landscape textures (transfer mode 9) are low-resolution bitmaps stretched across the entire sky via per-fragment azimuth/elevation calculations. Nearest-neighbor on these stretched textures produces visible blocky artifacts.

The current bind group layout for textures (group 1) has two entries: binding 0 for the texture array, binding 1 for the sampler.

## Goals / Non-Goals

**Goals:**
- Smooth sky/landscape rendering using bilinear filtering
- Preserve nearest-neighbor pixel-art look for all other surfaces (walls, floors, ceilings)
- Minimal changes to the bind group layout and shader

**Non-Goals:**
- Mipmapping (would require generating mip chains at load time; bilinear alone fixes the visible blockiness)
- Anisotropic filtering (unnecessary for the equirectangular landscape projection)
- Changing sprite rendering (sprites have their own separate pipeline and sampler)

## Decisions

### Decision 1: Two samplers in the texture bind group

Add a second sampler (binding 2, `FilterMode::Linear`) to the existing texture bind group layout. The fragment shader receives both samplers and selects based on transfer mode.

**Why not a separate bind group?** Adding a binding to the existing group is simpler than creating a new bind group and pipeline layout. The sampler is lightweight (no texture data).

**Why not just switch the single sampler to Linear?** That would blur all textures, losing the pixel-art look on walls and floors that Marathon players expect.

### Decision 2: Shader-side sampler selection

The fragment shader already has `transfer_mode` available as a flat-interpolated vertex attribute. A simple conditional selects which sampler to pass to `textureSample`. This avoids any mesh or CPU-side changes.

**Approach:** In `fs_main`, after computing UVs via `apply_transfer_mode`, check if `transfer_mode == TRANSFER_LANDSCAPE` and use `linear_sampler` for that case, `texture_sampler` (nearest) for all others.

### Decision 3: Pass both samplers through create_gpu_textures

`TextureManager::create_gpu_textures()` currently takes a single `&wgpu::Sampler`. Change it to take both samplers so bind groups include both at bindings 1 and 2. The fallback texture bind group also needs the second sampler.

## Risks / Trade-offs

- [Bind group size increase] Each texture bind group grows by one sampler entry. Samplers are trivially small on GPU; no measurable performance impact. → No mitigation needed.
- [WebGL2 compatibility] wgpu's WebGL2 backend supports multiple samplers in a bind group. The existing `filterable: true` sample type already permits both Nearest and Linear. → No risk.
- [Sprites unaffected] Sprites use their own pipeline, bind group layout, and sampler. This change does not touch sprites. → No action needed.
