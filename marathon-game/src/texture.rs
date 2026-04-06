use marathon_formats::{Collection, ColorValue, ShapeDescriptor, ShapesFile};
use std::collections::HashMap;

/// A loaded texture collection ready for GPU upload.
pub struct LoadedCollection {
    pub bitmaps: Vec<Vec<u8>>,
    pub max_width: u32,
    pub max_height: u32,
}

/// Manages texture loading and GPU texture array creation.
pub struct TextureManager {
    pub collections: HashMap<u16, LoadedCollection>,
    pub gpu_textures: HashMap<u16, GpuCollectionTexture>,
}

pub struct GpuCollectionTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
}

impl TextureManager {
    pub fn load_collections(shapes: &ShapesFile, descriptors: &[ShapeDescriptor]) -> Self {
        let mut needed: Vec<u16> = descriptors
            .iter()
            .filter(|d| !d.is_none())
            .map(|d| d.collection() as u16)
            .collect();
        needed.sort_unstable();
        needed.dedup();

        let mut collections = HashMap::new();
        for &coll_idx in &needed {
            match shapes.collection(coll_idx as usize) {
                Ok(collection) => {
                    if let Some(loaded) = load_collection(&collection, 0) {
                        collections.insert(coll_idx, loaded);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to load collection {coll_idx}: {e}");
                }
            }
        }

        TextureManager {
            collections,
            gpu_textures: HashMap::new(),
        }
    }

    pub fn create_gpu_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
    ) {
        for (&coll_idx, loaded) in &self.collections {
            if loaded.bitmaps.is_empty() {
                continue;
            }

            let layer_count = loaded.bitmaps.len() as u32;
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("collection_{coll_idx}")),
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
                label: Some(&format!("collection_{coll_idx}_bind_group")),
                layout: texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            });

            self.gpu_textures.insert(
                coll_idx,
                GpuCollectionTexture {
                    texture,
                    view,
                    bind_group,
                },
            );
        }
    }
}

fn load_collection(collection: &Collection, clut_index: usize) -> Option<LoadedCollection> {
    if collection.bitmaps.is_empty() {
        return None;
    }

    let clut = collection.color_tables.get(clut_index)?;

    let max_width = collection.bitmaps.iter().map(|b| b.width as u32).max().unwrap_or(1);
    let max_height = collection.bitmaps.iter().map(|b| b.height as u32).max().unwrap_or(1);

    let bitmaps: Vec<Vec<u8>> = collection
        .bitmaps
        .iter()
        .map(|bitmap| convert_bitmap(bitmap, clut, max_width, max_height))
        .collect();

    Some(LoadedCollection {
        bitmaps,
        max_width,
        max_height,
    })
}

fn convert_bitmap(
    bitmap: &marathon_formats::Bitmap,
    clut: &[ColorValue],
    target_width: u32,
    target_height: u32,
) -> Vec<u8> {
    let w = bitmap.width as u32;
    let h = bitmap.height as u32;
    let mut rgba = vec![0u8; (target_width * target_height * 4) as usize];

    for y in 0..h.min(target_height) {
        for x in 0..w.min(target_width) {
            let src_idx = if bitmap.column_order {
                (x * h + y) as usize
            } else {
                (y * w + x) as usize
            };

            let pixel = *bitmap.pixels.get(src_idx).unwrap_or(&0);
            let dst_idx = ((y * target_width + x) * 4) as usize;

            if bitmap.transparent && pixel == 0 {
                rgba[dst_idx] = 0;
                rgba[dst_idx + 1] = 0;
                rgba[dst_idx + 2] = 0;
                rgba[dst_idx + 3] = 0;
            } else if let Some(color) = clut.get(pixel as usize) {
                rgba[dst_idx] = (color.red >> 8) as u8;
                rgba[dst_idx + 1] = (color.green >> 8) as u8;
                rgba[dst_idx + 2] = (color.blue >> 8) as u8;
                rgba[dst_idx + 3] = 255;
            }
        }
    }

    rgba
}
