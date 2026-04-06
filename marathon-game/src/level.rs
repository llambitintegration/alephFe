use marathon_formats::map::LightData;
use marathon_formats::{MapData, StaticLightData, WadFile};

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
    let intensity = light.primary_active.intensity as f32 / 65536.0;
    intensity.clamp(0.0, 1.0)
}
