## Why

The `marathon-formats` crate gives us the ability to parse Marathon scenario data, but parsing alone does not validate that we can actually render this content. We need a concrete proof that Marathon's 2.5D portal-based geometry can be converted into modern GPU-friendly triangle meshes and rendered correctly with wgpu.

A standalone level viewer is the right next step because it isolates the rendering problem from game logic. Marathon's geometry model -- convex polygons with independent floor/ceiling heights, line/side-based wall definitions, and texture transfer modes -- is unusual enough that getting the mesh conversion and rendering pipeline right is a significant milestone on its own. If we tried to build rendering and simulation together, debugging geometry artifacts would be far harder.

This phase also establishes the rendering architecture that later phases (simulation, audio, multiplayer) will build on top of. The key architectural bet is that we can convert Marathon's portal-based geometry into static GPU meshes at level load time rather than rebuilding visible geometry every frame via CPU-side portal traversal as the original C++ engine does. Since ~95% of geometry is static (only platforms, media surfaces, animated textures, and light intensities change per frame), the GPU's own depth testing and frustum culling should handle visibility efficiently without a software portal renderer.

## What Changes

A new `marathon-viewer` crate that loads Marathon scenarios (map + shapes WAD files via `marathon-formats`) and renders navigable 3D levels using wgpu. No game logic, no monsters, no weapons, no HUD -- purely a geometry and rendering proof-of-concept.

The viewer will:

- Load any Marathon scenario and let the user select/switch between levels
- Convert polygon floor/ceiling surfaces into triangle meshes via fan triangulation of convex polygons (up to 8 vertices each)
- Build wall geometry from side definitions, handling the four side types (`full`, `high`, `low`, `split`) to compute correct wall height ranges based on adjacent polygon heights
- Load shape collections into GPU texture arrays and compute UV coordinates from side/polygon texture offsets
- Apply Marathon's transfer mode effects (landscape, slide, pulsate, wobble, static) in shaders
- Animate platforms (moving floor/ceiling heights) and media surfaces (liquid levels) by updating per-polygon uniform data rather than rebuilding meshes
- Apply basic lighting from Marathon light source definitions
- Provide free camera movement with WASD + mouse look for navigating levels

## Capabilities

### New Capabilities

- **`mesh-generation`**: Converts Marathon map geometry into GPU-ready triangle meshes. Handles fan triangulation of convex floor/ceiling polygons, construction of wall quads from line/side data across all four side types, and dynamic geometry updates for platforms and media surfaces. This is the core geometric translation layer between Marathon's 2.5D portal model and modern GPU rendering.

- **`texture-pipeline`**: Loads Marathon shape collections into GPU texture arrays (or atlas), computes UV coordinates from side and polygon texture offset data, and manages texture state for animated sequences. Bridges the gap between Marathon's indexed shape/frame/bitmap model and wgpu's texture binding model.

- **`level-rendering`**: The wgpu render pipeline, shader programs, camera system, and frame loop. Sets up the window (winit), configures the GPU device and swap chain, manages the depth buffer, handles camera projection and free-fly movement, and orchestrates per-frame draw calls. Also includes level selection and switching.

- **`transfer-modes`**: Shader-based implementations of Marathon's texture transfer modes. Landscape mode maps textures to view angle for sky/horizon effects. Slide mode scrolls UVs over time. Pulsate and wobble modes apply periodic UV distortion. Static mode renders randomized noise. These run entirely in the fragment shader using per-surface uniform data.

### Modified Capabilities

None. This is a new crate with no modifications to existing capabilities.

## Impact

- **`marathon-formats`** (dependency): The viewer depends on `marathon-formats` for all content loading -- map structures (polygons, lines, sides, endpoints, platforms, lights, media) and shape collections (textures, color tables). Any gaps in the format parsing will surface here. No changes to `marathon-formats` are proposed, but this will be its first real consumer and may reveal needed fixes or API adjustments.

- **Future crates** (`marathon-sim`, `marathon-audio`, `marathon-integration`): The rendering pipeline, mesh generation approach, and shader architecture established here will be reused directly. The static-mesh-with-dynamic-uniforms pattern for platforms and media sets the precedent for how simulation state updates will drive visual changes in later phases.
