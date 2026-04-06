use marathon_formats::map::LightData;
use marathon_formats::{MapData, MediaData, StaticLightData, StaticPlatformData, WadFile};

/// Metadata about an available level in a map WAD.
pub struct LevelInfo {
    pub index: usize,
    pub name: String,
}

/// A loaded level with all parsed data.
pub struct LoadedLevel {
    pub map: MapData,
    pub level_name: String,
}

/// Enumerate available levels from a map WAD file.
pub fn enumerate_levels(wad: &WadFile) -> Vec<LevelInfo> {
    let mut levels = Vec::new();
    for (i, entry) in wad.entries().iter().enumerate() {
        let name = match MapData::from_entry(entry) {
            Ok(map) => map
                .map_info
                .as_ref()
                .map(|info| info.level_name.clone())
                .unwrap_or_else(|| format!("Level {i}")),
            Err(_) => format!("Level {i}"),
        };
        levels.push(LevelInfo { index: i, name });
    }
    levels
}

/// Load a specific level from a map WAD.
pub fn load_level(wad: &WadFile, index: usize) -> Result<LoadedLevel, String> {
    let entry = wad
        .entry(index)
        .ok_or_else(|| format!("Level index {index} out of range"))?;

    let map = MapData::from_entry(entry).map_err(|e| format!("Failed to parse map: {e}"))?;

    let level_name = map
        .map_info
        .as_ref()
        .map(|info| info.level_name.clone())
        .unwrap_or_else(|| format!("Level {index}"));

    Ok(LoadedLevel { map, level_name })
}

/// Collect all ShapeDescriptors referenced by a level (for texture loading).
pub fn collect_texture_descriptors(map: &MapData) -> Vec<marathon_formats::ShapeDescriptor> {
    let mut descs = Vec::new();

    for polygon in &map.polygons {
        descs.push(polygon.floor_texture);
        descs.push(polygon.ceiling_texture);
    }

    for side in &map.sides {
        descs.push(side.primary_texture.texture);
        descs.push(side.secondary_texture.texture);
        descs.push(side.transparent_texture.texture);
    }

    for media in &map.media {
        descs.push(media.texture);
    }

    descs
}

/// Platform animation state.
pub struct PlatformState {
    pub polygon_index: usize,
    pub current_height: f32,
    pub minimum_height: f32,
    pub maximum_height: f32,
    pub speed: f32,
    pub going_up: bool,
}

impl PlatformState {
    pub fn from_data(platform: &StaticPlatformData, map: &MapData) -> Self {
        let poly_idx = platform.polygon_index as usize;
        let polygon = &map.polygons[poly_idx];
        let min_h = platform.minimum_height as f32 / 1024.0;
        let max_h = platform.maximum_height as f32 / 1024.0;
        let current = polygon.floor_height as f32 / 1024.0;
        let speed = platform.speed as f32 / 1024.0;

        PlatformState {
            polygon_index: poly_idx,
            current_height: current,
            minimum_height: min_h,
            maximum_height: max_h,
            speed: speed.max(0.001),
            going_up: true,
        }
    }

    /// Update platform height for one frame. Returns the new height.
    pub fn update(&mut self, dt: f32) -> f32 {
        let delta = self.speed * dt;
        if self.going_up {
            self.current_height += delta;
            if self.current_height >= self.maximum_height {
                self.current_height = self.maximum_height;
                self.going_up = false;
            }
        } else {
            self.current_height -= delta;
            if self.current_height <= self.minimum_height {
                self.current_height = self.minimum_height;
                self.going_up = true;
            }
        }
        self.current_height
    }
}

/// Media animation state.
pub struct MediaState {
    pub polygon_index: usize,
    pub current_height: f32,
    pub low: f32,
    pub high: f32,
}

impl MediaState {
    pub fn from_data(media: &MediaData, map: &MapData) -> Option<Self> {
        // Find the polygon that references this media
        let poly_idx = map
            .polygons
            .iter()
            .position(|p| p.media_index >= 0 && map.media.get(p.media_index as usize).map(|m| std::ptr::eq(m, media)).unwrap_or(false));

        poly_idx.map(|idx| MediaState {
            polygon_index: idx,
            current_height: media.height as f32 / 1024.0,
            low: media.low as f32 / 1024.0,
            high: media.high as f32 / 1024.0,
        })
    }
}

/// Evaluate light intensity (0.0 to 1.0) from light data.
pub fn evaluate_light_intensity(lights: &LightData, light_index: i16) -> f32 {
    if light_index < 0 {
        return 1.0;
    }
    let idx = light_index as usize;

    match lights {
        LightData::Static(static_lights) => {
            if let Some(light) = static_lights.get(idx) {
                evaluate_static_light(light)
            } else {
                1.0
            }
        }
        LightData::Old(old_lights) => {
            if let Some(light) = old_lights.get(idx) {
                light.intensity.clamp(0.0, 1.0)
            } else {
                1.0
            }
        }
        LightData::None => 1.0,
    }
}

fn evaluate_static_light(light: &StaticLightData) -> f32 {
    // Use the primary active function's intensity as the base value.
    // The light function spec has intensity and delta fields.
    // For static evaluation, use the primary active intensity.
    let intensity = light.primary_active.intensity as f32 / 65536.0;
    intensity.clamp(0.0, 1.0)
}
