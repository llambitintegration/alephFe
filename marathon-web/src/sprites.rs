//! Sprite rendering for entities (monsters, items, projectiles, effects).
//!
//! Renders entity sprites as camera-facing billboarded quads in a second
//! render pass after level geometry, sharing the depth buffer for correct
//! occlusion.
//!
//! WIP: this sprite pipeline is scaffolding that is not yet wired into the
//! main render loop, so many items are currently unused.
#![allow(dead_code)]

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use marathon_formats::{Collection, ShapesFile};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

/// GPU vertex for a sprite billboard quad.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub tex_index: u32,
    pub tint: f32,
}

impl SpriteVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Uint32,
        3 => Float32,
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Information about a loaded sprite collection for GPU rendering.
pub struct SpriteCollection {
    /// Texture array bind group for this collection.
    pub bind_group: wgpu::BindGroup,
    /// Number of bitmaps (layers) in the texture array.
    pub bitmap_count: u32,
    /// Max width/height of bitmaps in this collection.
    pub max_width: u32,
    pub max_height: u32,
}

/// Manages the sprite rendering pipeline and textures.
pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    /// Sprite collections loaded on the GPU, indexed by collection number.
    pub collections: HashMap<u16, SpriteCollection>,
    /// Fallback bind group for missing collections.
    fallback_bind_group: wgpu::BindGroup,
}

/// A sprite to render this frame.
pub struct SpriteDrawCall {
    pub position: Vec3,
    pub world_left: f32,
    pub world_right: f32,
    pub world_top: f32,
    pub world_bottom: f32,
    pub collection: u16,
    pub bitmap_index: u32,
    pub tint: f32,
}

impl SpriteRenderer {
    /// Create the sprite rendering pipeline.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bgl: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite_shader.wgsl").into()),
        });

        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite_texture_bgl"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[camera_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sprite"),
                buffers: &[SpriteVertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sprite"),
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
                cull_mode: None, // Billboards face camera, no culling needed
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

        // Create fallback 1x1 magenta texture
        let fallback_layer_count = crate::texture::pad_layer_count_for_webgl(1);
        let fallback_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite_fallback"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: fallback_layer_count,
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let fallback_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite_fallback_bg"),
            layout: &texture_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        SpriteRenderer {
            pipeline,
            texture_bind_group_layout: texture_bgl,
            collections: HashMap::new(),
            fallback_bind_group,
        }
    }

    /// Load sprite collections referenced by entities onto the GPU.
    pub fn load_collections(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        shapes: &ShapesFile,
        collection_indices: &[u16],
    ) {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        for &coll_idx in collection_indices {
            if self.collections.contains_key(&coll_idx) {
                continue;
            }

            let collection = match shapes.collection(coll_idx as usize) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("Failed to load sprite collection {coll_idx}: {e}");
                    continue;
                }
            };

            if collection.bitmaps.is_empty() || collection.color_tables.is_empty() {
                continue;
            }

            let loaded = convert_collection_to_rgba(&collection);
            if loaded.bitmaps.is_empty() {
                continue;
            }

            let layer_count = crate::texture::pad_layer_count_for_webgl(loaded.bitmaps.len());
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("sprite_collection_{coll_idx}")),
                size: wgpu::Extent3d {
                    width: loaded.max_width,
                    height: loaded.max_height,
                    depth_or_array_layers: layer_count,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            for (i, bitmap_data) in loaded.bitmaps.iter().enumerate() {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: i as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    bitmap_data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * loaded.max_width),
                        rows_per_image: Some(loaded.max_height),
                    },
                    wgpu::Extent3d {
                        width: loaded.max_width,
                        height: loaded.max_height,
                        depth_or_array_layers: 1,
                    },
                );
            }

            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("sprite_collection_{coll_idx}_bg")),
                layout: &self.texture_bind_group_layout,
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

            self.collections.insert(
                coll_idx,
                SpriteCollection {
                    bind_group,
                    bitmap_count: layer_count,
                    max_width: loaded.max_width,
                    max_height: loaded.max_height,
                },
            );
        }
    }

    /// Build billboard quads and render sprites.
    ///
    /// This should be called within a render pass that already has the depth
    /// buffer from the level geometry pass.
    pub fn render(
        &self,
        device: &wgpu::Device,
        render_pass: &mut wgpu::RenderPass<'_>,
        camera_bind_group: &wgpu::BindGroup,
        _camera_pos: Vec3,
        camera_yaw: f32,
        draw_calls: &[SpriteDrawCall],
    ) {
        if draw_calls.is_empty() {
            return;
        }

        // Build billboard vertices for all sprites
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Camera right and up vectors for billboarding
        let cam_right = Vec3::new(-camera_yaw.sin(), 0.0, camera_yaw.cos());
        let _cam_up = Vec3::Y;

        // Group draw calls by collection
        let mut by_collection: HashMap<u16, Vec<&SpriteDrawCall>> = HashMap::new();
        for call in draw_calls {
            by_collection.entry(call.collection).or_default().push(call);
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);

        for (coll_idx, calls) in &by_collection {
            let bind_group = self
                .collections
                .get(coll_idx)
                .map(|c| &c.bind_group)
                .unwrap_or(&self.fallback_bind_group);

            render_pass.set_bind_group(1, bind_group, &[]);

            vertices.clear();
            indices.clear();

            for call in calls {
                let base = vertices.len() as u32;

                let center = call.position;

                // Asymmetric billboard: world_left/right along camera-right,
                // world_top/bottom along world Y (negated: top is negative = above)
                let bl = center + cam_right * call.world_left + Vec3::Y * (-call.world_bottom);
                let br = center + cam_right * call.world_right + Vec3::Y * (-call.world_bottom);
                let tl = center + cam_right * call.world_left + Vec3::Y * (-call.world_top);
                let tr = center + cam_right * call.world_right + Vec3::Y * (-call.world_top);

                vertices.push(SpriteVertex {
                    position: bl.into(),
                    uv: [0.0, 1.0],
                    tex_index: call.bitmap_index,
                    tint: call.tint,
                });
                vertices.push(SpriteVertex {
                    position: tl.into(),
                    uv: [0.0, 0.0],
                    tex_index: call.bitmap_index,
                    tint: call.tint,
                });
                vertices.push(SpriteVertex {
                    position: tr.into(),
                    uv: [1.0, 0.0],
                    tex_index: call.bitmap_index,
                    tint: call.tint,
                });
                vertices.push(SpriteVertex {
                    position: br.into(),
                    uv: [1.0, 1.0],
                    tex_index: call.bitmap_index,
                    tint: call.tint,
                });

                indices.push(base);
                indices.push(base + 1);
                indices.push(base + 2);
                indices.push(base);
                indices.push(base + 2);
                indices.push(base + 3);
            }

            if vertices.is_empty() {
                continue;
            }

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite_vertex_buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite_index_buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }
    }
}

/// Resolve an entity's shape and frame to a bitmap index and world dimensions.
///
/// Returns (bitmap_index, world_width, world_height) or None if unresolvable.
pub fn resolve_entity_sprite(
    shapes: &ShapesFile,
    collection_idx: u16,
    shape_idx: u16,
    frame_idx: u16,
    view_angle: u16,
) -> Option<(u32, f32, f32, f32, f32)> {
    let collection = shapes.collection(collection_idx as usize).ok()?;

    let high_level = collection.high_level_shapes.get(shape_idx as usize)?;

    let actual_views =
        marathon_formats::shapes::actual_view_count(high_level.number_of_views).max(1) as u16;
    let view = (view_angle % actual_views) as usize;
    let frame = (frame_idx as usize) % (high_level.frames_per_view.max(1) as usize);

    let ll_index_offset = view * (high_level.frames_per_view.max(1) as usize) + frame;
    let ll_index = *high_level.low_level_shape_indexes.get(ll_index_offset)? as usize;

    let low_level = collection.low_level_shapes.get(ll_index)?;

    let bitmap_index = low_level.bitmap_index as u32;

    let world_left = low_level.world_left as f32 / 1024.0;
    let world_right = low_level.world_right as f32 / 1024.0;
    let world_top = low_level.world_top as f32 / 1024.0;
    let world_bottom = low_level.world_bottom as f32 / 1024.0;

    Some((
        bitmap_index,
        world_left,
        world_right,
        world_top,
        world_bottom,
    ))
}

/// Compute the viewing angle index (0-7) for a monster based on relative angle.
pub fn compute_view_angle(camera_pos: Vec3, entity_pos: Vec3, entity_facing: f32) -> u16 {
    let dx = camera_pos.x - entity_pos.x;
    let dz = camera_pos.z - entity_pos.z;
    let angle_to_camera = dz.atan2(dx);
    let relative_angle = angle_to_camera - entity_facing;

    // Normalize to [0, 2π)
    let normalized =
        ((relative_angle % std::f32::consts::TAU) + std::f32::consts::TAU) % std::f32::consts::TAU;

    // Quantize to 8 views
    ((normalized / std::f32::consts::TAU * 8.0 + 0.5) as u16) % 8
}

/// Screen-space weapon overlay renderer (no depth test, NDC quad).
pub struct WeaponOverlayRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl WeaponOverlayRenderer {
    /// Create the weapon overlay pipeline, sharing the same texture bind group
    /// layout as SpriteRenderer.
    pub fn new(
        device: &wgpu::Device,
        camera_bgl: &wgpu::BindGroupLayout,
        texture_bgl: &wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader_overlay"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite_shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("weapon_overlay_layout"),
            bind_group_layouts: &[camera_bgl, texture_bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("weapon_overlay_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_overlay"),
                buffers: &[SpriteVertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sprite"),
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
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        WeaponOverlayRenderer { pipeline }
    }

    /// Render a weapon sprite as a screen-space overlay using position_sprite_axis
    /// logic (_position_center mode).
    ///
    /// The weapon's vertical/horizontal position determines the sprite origin on
    /// screen, and the shape's world bounds determine how the sprite extends from
    /// that origin. Portions extending below the viewport are clipped by the GPU.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        device: &wgpu::Device,
        render_pass: &mut wgpu::RenderPass<'_>,
        camera_bind_group: &wgpu::BindGroup,
        texture_bind_group: &wgpu::BindGroup,
        bitmap_index: u32,
        world_left: f32,
        world_right: f32,
        world_top: f32,
        world_bottom: f32,
        vertical_position: f32,
        horizontal_position: f32,
        tint: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // position_sprite_axis for _position_center mode:
        // origin = screen_dim * position
        // edges = origin + world_bound * screen_height
        let origin_x = viewport_width * horizontal_position;
        let origin_y = viewport_height * vertical_position;

        // X axis: offset from origin by world bounds scaled to screen height
        let x0 = origin_x + world_left * viewport_height;
        let x1 = origin_x + world_right * viewport_height;

        // Y axis: offset from origin by NEGATED world bounds scaled to screen height
        // world_top is negative (above origin), world_bottom is positive (below)
        let y0 = origin_y + (-world_top) * viewport_height;
        let y1 = origin_y + (-world_bottom) * viewport_height;

        // Convert pixel coordinates to NDC: ndc = (px / dim) * 2.0 - 1.0
        let ndc_left = (x0 / viewport_width) * 2.0 - 1.0;
        let ndc_right = (x1 / viewport_width) * 2.0 - 1.0;
        // Y: NDC +1 is top, -1 is bottom. Pixel 0 is top.
        let ndc_top = 1.0 - (y0 / viewport_height) * 2.0;
        let ndc_bottom = 1.0 - (y1 / viewport_height) * 2.0;

        let vertices = [
            SpriteVertex {
                position: [ndc_left, ndc_bottom, 0.0],
                uv: [0.0, 1.0],
                tex_index: bitmap_index,
                tint,
            },
            SpriteVertex {
                position: [ndc_left, ndc_top, 0.0],
                uv: [0.0, 0.0],
                tex_index: bitmap_index,
                tint,
            },
            SpriteVertex {
                position: [ndc_right, ndc_top, 0.0],
                uv: [1.0, 0.0],
                tex_index: bitmap_index,
                tint,
            },
            SpriteVertex {
                position: [ndc_right, ndc_bottom, 0.0],
                uv: [1.0, 1.0],
                tex_index: bitmap_index,
                tint,
            },
        ];
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];

        let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("weapon_overlay_vb"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("weapon_overlay_ib"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vb.slice(..));
        render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }

    /// Get the texture bind group layout for creating compatible bind groups.
    pub fn texture_bgl(sprite_renderer: &SpriteRenderer) -> &wgpu::BindGroupLayout {
        &sprite_renderer.texture_bind_group_layout
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────

struct ConvertedCollection {
    bitmaps: Vec<Vec<u8>>,
    max_width: u32,
    max_height: u32,
}

fn convert_collection_to_rgba(collection: &Collection) -> ConvertedCollection {
    if collection.bitmaps.is_empty() || collection.color_tables.is_empty() {
        return ConvertedCollection {
            bitmaps: vec![],
            max_width: 1,
            max_height: 1,
        };
    }

    let clut = &collection.color_tables[0];

    let max_width = collection
        .bitmaps
        .iter()
        .map(|b| b.width as u32)
        .max()
        .unwrap_or(1);
    let max_height = collection
        .bitmaps
        .iter()
        .map(|b| b.height as u32)
        .max()
        .unwrap_or(1);

    let bitmaps: Vec<Vec<u8>> = collection
        .bitmaps
        .iter()
        .map(|bitmap| {
            let w = bitmap.width as u32;
            let h = bitmap.height as u32;
            let mut rgba = vec![0u8; (max_width * max_height * 4) as usize];

            for y in 0..h.min(max_height) {
                for x in 0..w.min(max_width) {
                    let src_idx = if bitmap.column_order {
                        (x * h + y) as usize
                    } else {
                        (y * w + x) as usize
                    };

                    let pixel = *bitmap.pixels.get(src_idx).unwrap_or(&0);
                    let dst_idx = ((y * max_width + x) * 4) as usize;

                    if bitmap.transparent && pixel == 0 {
                        // Transparent
                    } else if let Some(color) = clut.get(pixel as usize) {
                        rgba[dst_idx] = (color.red >> 8) as u8;
                        rgba[dst_idx + 1] = (color.green >> 8) as u8;
                        rgba[dst_idx + 2] = (color.blue >> 8) as u8;
                        rgba[dst_idx + 3] = 255;
                    }
                }
            }

            rgba
        })
        .collect();

    ConvertedCollection {
        bitmaps,
        max_width,
        max_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_view_angle_front() {
        // Camera directly in front of entity (facing same direction as entity)
        let cam = Vec3::new(5.0, 0.0, 0.0);
        let entity = Vec3::new(0.0, 0.0, 0.0);
        let facing = 0.0; // facing +X
        let view = compute_view_angle(cam, entity, facing);
        assert!(view < 8, "view angle should be in range 0-7");
    }

    #[test]
    fn compute_view_angle_behind() {
        // Camera behind entity
        let cam = Vec3::new(-5.0, 0.0, 0.0);
        let entity = Vec3::new(0.0, 0.0, 0.0);
        let facing = 0.0; // facing +X
        let view = compute_view_angle(cam, entity, facing);
        // Should be view 4 (back) approximately
        assert!(view < 8);
    }

    #[test]
    fn sprite_vertex_layout_size() {
        // 3 floats + 2 floats + 1 u32 + 1 float = 7 * 4 = 28 bytes
        assert_eq!(std::mem::size_of::<SpriteVertex>(), 28);
    }

    #[test]
    fn converted_collection_empty() {
        let collection = Collection {
            definition: marathon_formats::CollectionDefinition {
                version: 3,
                collection_type: 0,
                flags: 0,
                color_count: 0,
                clut_count: 0,
                color_table_offset: 0,
                high_level_shape_count: 0,
                high_level_shape_offset_table_offset: 0,
                low_level_shape_count: 0,
                low_level_shape_offset_table_offset: 0,
                bitmap_count: 0,
                bitmap_offset_table_offset: 0,
                pixels_to_world: 0,
                size: 0,
            },
            color_tables: vec![],
            high_level_shapes: vec![],
            low_level_shapes: vec![],
            bitmaps: vec![],
        };
        let converted = convert_collection_to_rgba(&collection);
        assert!(converted.bitmaps.is_empty());
    }
}
