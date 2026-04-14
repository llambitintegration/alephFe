## 1. Add linear sampler and update bind group layout

- [x] 1.1 In `render.rs`, create a second sampler with `FilterMode::Linear` and `AddressMode::Repeat` alongside the existing nearest sampler (around line 697)
- [x] 1.2 In `render.rs`, add a third entry to the texture bind group layout (`tex_bgl`) at binding 2 for the linear sampler (`SamplerBindingType::Filtering`)
- [x] 1.3 Update `TextureManager::create_gpu_textures()` in `texture.rs` to accept both samplers and bind the linear sampler at binding 2 in each collection's bind group
- [x] 1.4 Update the `create_fallback_texture()` call in `render.rs` to include the linear sampler in the fallback bind group

## 2. Update fragment shader to use dual samplers

- [x] 2.1 In `shader.wgsl`, add a second sampler binding: `@group(1) @binding(2) var linear_sampler: sampler;`
- [x] 2.2 In `fs_main`, use `linear_sampler` when `transfer_mode == TRANSFER_LANDSCAPE` and `texture_sampler` for all other modes in the `textureSample` call

## 3. Verify

- [x] 3.1 Build the project and confirm no compilation errors
- [ ] 3.2 Load a level with landscape/sky textures and verify smooth sky rendering
- [ ] 3.3 Verify wall and floor textures still render with crisp nearest-neighbor filtering
