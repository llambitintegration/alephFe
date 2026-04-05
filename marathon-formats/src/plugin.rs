use std::collections::HashSet;
use std::path::Path;

use bitflags::bitflags;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::PluginError;
use crate::types::four_chars;

bitflags! {
    /// Write access flags for solo Lua scripts.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SoloLuaWriteAccess: u32 {
        const WORLD = 0x01;
        const FOG = 0x02;
        const MUSIC = 0x04;
        const OVERLAYS = 0x08;
        const EPHEMERA = 0x10;
        const SOUND = 0x20;
    }
}

/// Scenario compatibility requirement.
#[derive(Debug, Clone)]
pub struct ScenarioRequirement {
    pub name: Option<String>,
    pub id: Option<String>,
    pub version: Option<String>,
}

/// A shapes patch file reference.
#[derive(Debug, Clone)]
pub struct ShapesPatch {
    pub file: String,
    pub requires_opengl: bool,
}

/// A single resource entry in a map patch.
#[derive(Debug, Clone)]
pub struct MapResource {
    pub resource_type: u32,
    pub id: i32,
    pub data: String,
}

/// A map patch with its associated checksums and resources.
#[derive(Debug, Clone)]
pub struct MapPatch {
    pub checksums: HashSet<u32>,
    pub resources: Vec<MapResource>,
}

/// Parsed Plugin.xml metadata.
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub minimum_version: Option<String>,
    pub auto_enable: bool,
    pub theme_dir: Option<String>,
    pub hud_lua: Option<String>,
    pub solo_lua: Option<String>,
    pub solo_lua_write_access: SoloLuaWriteAccess,
    pub stats_lua: Option<String>,
    pub required_scenarios: Vec<ScenarioRequirement>,
    pub mml_files: Vec<String>,
    pub shapes_patches: Vec<ShapesPatch>,
    pub sounds_patches: Vec<String>,
    pub map_patches: Vec<MapPatch>,
}

impl PluginMetadata {
    /// Parse plugin metadata from XML bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, PluginError> {
        let mut reader = Reader::from_reader(data);

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let mut plugin = build_from_attrs(&e)?;
                    parse_children(&mut reader, &mut plugin)?;
                    apply_theme_dir(&mut plugin);
                    plugin.mml_files.sort();
                    return Ok(plugin);
                }
                Ok(Event::Empty(e)) => {
                    let mut plugin = build_from_attrs(&e)?;
                    apply_theme_dir(&mut plugin);
                    return Ok(plugin);
                }
                Ok(Event::Eof) => return Err(PluginError::MissingName),
                Ok(_) => continue,
                Err(e) => return Err(PluginError::Xml(e)),
            }
        }
    }

    /// Parse plugin metadata from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, PluginError> {
        let data = std::fs::read(path)?;
        Self::from_bytes(&data)
    }

    /// Remove references to files that don't exist in `plugin_dir`.
    pub fn validate_references(&mut self, plugin_dir: &Path) {
        if let Some(ref path) = self.hud_lua {
            if !plugin_dir.join(path).exists() {
                self.hud_lua = None;
            }
        }
        if let Some(ref path) = self.solo_lua {
            if !plugin_dir.join(path).exists() {
                self.solo_lua = None;
            }
        }
        if let Some(ref path) = self.stats_lua {
            if !plugin_dir.join(path).exists() {
                self.stats_lua = None;
            }
        }
        self.mml_files.retain(|f| plugin_dir.join(f).exists());
        self.shapes_patches
            .retain(|p| plugin_dir.join(&p.file).exists());
        self.sounds_patches.retain(|f| plugin_dir.join(f).exists());
    }
}

/// Discover plugins by recursively scanning a directory for Plugin.xml files.
/// Skips dot-prefixed directories and silently skips unparseable plugins.
pub fn discover_plugins(dir: &Path) -> Vec<PluginMetadata> {
    let mut plugins = Vec::new();
    scan_directory(dir, &mut plugins);
    sort_plugins(&mut plugins);
    plugins
}

/// Sort plugins alphabetically by name.
pub fn sort_plugins(plugins: &mut [PluginMetadata]) {
    plugins.sort_by(|a, b| a.name.cmp(&b.name));
}

/// Resolve exclusive resources: last plugin in order wins for HUD Lua,
/// stats Lua, theme, and solo Lua (by write access flags).
pub fn resolve_exclusive_resources(plugins: &mut [PluginMetadata]) {
    let exclusive_mask = SoloLuaWriteAccess::WORLD
        | SoloLuaWriteAccess::FOG
        | SoloLuaWriteAccess::MUSIC
        | SoloLuaWriteAccess::OVERLAYS;

    let mut hud_claimed = false;
    let mut stats_claimed = false;
    let mut theme_claimed = false;
    let mut solo_accumulated = SoloLuaWriteAccess::empty();

    for plugin in plugins.iter_mut().rev() {
        if plugin.hud_lua.is_some() {
            if hud_claimed {
                plugin.hud_lua = None;
            } else {
                hud_claimed = true;
            }
        }
        if plugin.stats_lua.is_some() {
            if stats_claimed {
                plugin.stats_lua = None;
            } else {
                stats_claimed = true;
            }
        }
        if plugin.theme_dir.is_some() {
            if theme_claimed {
                plugin.theme_dir = None;
            } else {
                theme_claimed = true;
            }
        }
        if plugin.solo_lua.is_some() {
            let plugin_exclusive = if plugin
                .solo_lua_write_access
                .contains(SoloLuaWriteAccess::WORLD)
            {
                exclusive_mask
            } else {
                plugin.solo_lua_write_access & exclusive_mask
            };
            if plugin_exclusive.intersects(solo_accumulated) {
                plugin.solo_lua = None;
            } else {
                solo_accumulated |= plugin_exclusive;
            }
        }
    }
}

// ── Internal helpers ────────────────────────────────────────────────

fn scan_directory(dir: &Path, plugins: &mut Vec<PluginMetadata>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            let plugin_xml = path.join("Plugin.xml");
            if plugin_xml.is_file() {
                if let Ok(mut plugin) = PluginMetadata::from_file(&plugin_xml) {
                    plugin.validate_references(&path);
                    plugins.push(plugin);
                }
            } else {
                scan_directory(&path, plugins);
            }
        }
    }
}

fn element_name(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.name().as_ref()).into_owned()
}

fn get_attr(e: &BytesStart<'_>, name: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == name.as_bytes())
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
}

fn truncate(s: &str, max_len: usize) -> String {
    s.chars().take(max_len).collect()
}

fn type_str_to_u32(s: &str) -> Option<u32> {
    let bytes = s.as_bytes();
    if bytes.len() != 4 {
        return None;
    }
    Some(four_chars(bytes[0], bytes[1], bytes[2], bytes[3]))
}

fn build_from_attrs(e: &BytesStart<'_>) -> Result<PluginMetadata, PluginError> {
    let name = get_attr(e, "name").ok_or(PluginError::MissingName)?;
    Ok(PluginMetadata {
        name,
        description: get_attr(e, "description"),
        version: get_attr(e, "version"),
        minimum_version: get_attr(e, "minimum_version"),
        auto_enable: get_attr(e, "auto_enable").is_none_or(|v| v != "false"),
        theme_dir: get_attr(e, "theme_dir"),
        hud_lua: get_attr(e, "hud_lua"),
        solo_lua: get_attr(e, "solo_lua"),
        solo_lua_write_access: SoloLuaWriteAccess::WORLD,
        stats_lua: get_attr(e, "stats_lua"),
        required_scenarios: Vec::new(),
        mml_files: Vec::new(),
        shapes_patches: Vec::new(),
        sounds_patches: Vec::new(),
        map_patches: Vec::new(),
    })
}

fn apply_theme_dir(plugin: &mut PluginMetadata) {
    if plugin.theme_dir.is_some() {
        plugin.hud_lua = None;
        plugin.solo_lua = None;
        plugin.shapes_patches.clear();
        plugin.sounds_patches.clear();
        plugin.map_patches.clear();
    }
}

fn parse_children(
    reader: &mut Reader<&[u8]>,
    plugin: &mut PluginMetadata,
) -> Result<(), PluginError> {
    let mut solo_lua_count = 0u32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                handle_child_start(reader, &e, plugin, &mut solo_lua_count)?;
            }
            Ok(Event::Empty(e)) => {
                handle_child_empty(&e, plugin, &mut solo_lua_count);
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Ok(_) => continue,
            Err(e) => return Err(PluginError::Xml(e)),
        }
    }
    Ok(())
}

fn handle_child_start(
    reader: &mut Reader<&[u8]>,
    e: &BytesStart<'_>,
    plugin: &mut PluginMetadata,
    solo_lua_count: &mut u32,
) -> Result<(), PluginError> {
    let name = element_name(e);
    match name.as_str() {
        "scenario" => {
            if let Some(req) = parse_scenario(e) {
                plugin.required_scenarios.push(req);
            }
            skip_element(reader)?;
        }
        "mml" => {
            if let Some(file) = get_attr(e, "file") {
                plugin.mml_files.push(file);
            }
            skip_element(reader)?;
        }
        "solo_lua" => {
            *solo_lua_count += 1;
            if *solo_lua_count == 1 {
                if let Some(file) = get_attr(e, "file") {
                    plugin.solo_lua = Some(file);
                }
                plugin.solo_lua_write_access = parse_write_access(reader)?;
            } else {
                plugin.solo_lua = None;
                skip_element(reader)?;
            }
        }
        "shapes_patch" => {
            if let Some(file) = get_attr(e, "file") {
                let requires_opengl = get_attr(e, "requires_opengl").is_some_and(|v| v == "true");
                plugin.shapes_patches.push(ShapesPatch {
                    file,
                    requires_opengl,
                });
            }
            skip_element(reader)?;
        }
        "sounds_patch" => {
            if let Some(file) = get_attr(e, "file") {
                plugin.sounds_patches.push(file);
            }
            skip_element(reader)?;
        }
        "map_patch" => {
            let patch = parse_map_patch(reader)?;
            if !patch.checksums.is_empty() && !patch.resources.is_empty() {
                plugin.map_patches.push(patch);
            }
        }
        _ => skip_element(reader)?,
    }
    Ok(())
}

fn handle_child_empty(e: &BytesStart<'_>, plugin: &mut PluginMetadata, solo_lua_count: &mut u32) {
    let name = element_name(e);
    match name.as_str() {
        "scenario" => {
            if let Some(req) = parse_scenario(e) {
                plugin.required_scenarios.push(req);
            }
        }
        "mml" => {
            if let Some(file) = get_attr(e, "file") {
                plugin.mml_files.push(file);
            }
        }
        "solo_lua" => {
            *solo_lua_count += 1;
            if *solo_lua_count == 1 {
                if let Some(file) = get_attr(e, "file") {
                    plugin.solo_lua = Some(file);
                }
                plugin.solo_lua_write_access = SoloLuaWriteAccess::WORLD;
            } else {
                plugin.solo_lua = None;
            }
        }
        "shapes_patch" => {
            if let Some(file) = get_attr(e, "file") {
                let requires_opengl = get_attr(e, "requires_opengl").is_some_and(|v| v == "true");
                plugin.shapes_patches.push(ShapesPatch {
                    file,
                    requires_opengl,
                });
            }
        }
        "sounds_patch" => {
            if let Some(file) = get_attr(e, "file") {
                plugin.sounds_patches.push(file);
            }
        }
        _ => {}
    }
}

fn parse_scenario(e: &BytesStart<'_>) -> Option<ScenarioRequirement> {
    let name = get_attr(e, "name").map(|s| truncate(&s, 31));
    let id = get_attr(e, "id").map(|s| truncate(&s, 23));
    let version = get_attr(e, "version").map(|s| truncate(&s, 7));
    if name.is_none() && id.is_none() {
        return None;
    }
    Some(ScenarioRequirement { name, id, version })
}

fn parse_write_access(reader: &mut Reader<&[u8]>) -> Result<SoloLuaWriteAccess, PluginError> {
    let mut access = SoloLuaWriteAccess::empty();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if element_name(&e) == "write_access" {
                    let text = read_text_content(reader)?;
                    match text.as_str() {
                        "world" => access |= SoloLuaWriteAccess::WORLD,
                        "fog" => access |= SoloLuaWriteAccess::FOG,
                        "music" => access |= SoloLuaWriteAccess::MUSIC,
                        "overlays" => access |= SoloLuaWriteAccess::OVERLAYS,
                        "ephemera" => access |= SoloLuaWriteAccess::EPHEMERA,
                        "sound" => access |= SoloLuaWriteAccess::SOUND,
                        _ => {}
                    }
                } else {
                    skip_element(reader)?;
                }
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Ok(_) => continue,
            Err(e) => return Err(PluginError::Xml(e)),
        }
    }

    if access.is_empty() {
        access = SoloLuaWriteAccess::WORLD;
    }
    Ok(access)
}

fn parse_map_patch(reader: &mut Reader<&[u8]>) -> Result<MapPatch, PluginError> {
    let mut checksums = HashSet::new();
    let mut resources = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => match element_name(&e).as_str() {
                "checksum" => {
                    let text = read_text_content(reader)?;
                    if let Ok(v) = text.parse::<u32>() {
                        checksums.insert(v);
                    }
                }
                "resource" => {
                    parse_resource_attrs(&e, &mut resources);
                    skip_element(reader)?;
                }
                _ => skip_element(reader)?,
            },
            Ok(Event::Empty(e)) => {
                if element_name(&e) == "resource" {
                    parse_resource_attrs(&e, &mut resources);
                }
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Ok(_) => continue,
            Err(e) => return Err(PluginError::Xml(e)),
        }
    }

    Ok(MapPatch {
        checksums,
        resources,
    })
}

fn parse_resource_attrs(e: &BytesStart<'_>, resources: &mut Vec<MapResource>) {
    let res_type = get_attr(e, "type");
    let res_id = get_attr(e, "id");
    let res_data = get_attr(e, "data");
    if let (Some(type_str), Some(id_str), Some(data)) = (res_type, res_id, res_data) {
        if let (Some(type_u32), Ok(id)) = (type_str_to_u32(&type_str), id_str.parse::<i32>()) {
            resources.push(MapResource {
                resource_type: type_u32,
                id,
                data,
            });
        }
    }
}

fn read_text_content(reader: &mut Reader<&[u8]>) -> Result<String, PluginError> {
    let mut text = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Text(t)) => {
                text.push_str(&String::from_utf8_lossy(&t));
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Ok(_) => continue,
            Err(e) => return Err(PluginError::Xml(e)),
        }
    }
    Ok(text.trim().to_string())
}

fn skip_element(reader: &mut Reader<&[u8]>) -> Result<(), PluginError> {
    let mut depth = 1u32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => continue,
            Err(e) => return Err(PluginError::Xml(e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_plugin_parsing() {
        let xml = br#"<plugin name="Test Plugin" version="1.0" description="A test"/>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.name, "Test Plugin");
        assert_eq!(p.version.as_deref(), Some("1.0"));
        assert_eq!(p.description.as_deref(), Some("A test"));
    }

    #[test]
    fn test_missing_name_rejected() {
        let xml = br#"<plugin description="no name"/>"#;
        assert!(matches!(
            PluginMetadata::from_bytes(xml),
            Err(PluginError::MissingName)
        ));
    }

    #[test]
    fn test_auto_enable_default_true() {
        let xml = br#"<plugin name="P"/>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert!(p.auto_enable);
    }

    #[test]
    fn test_auto_enable_false() {
        let xml = br#"<plugin name="P" auto_enable="false"/>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert!(!p.auto_enable);
    }

    #[test]
    fn test_minimum_version() {
        let xml = br#"<plugin name="P" minimum_version="20230101"/>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.minimum_version.as_deref(), Some("20230101"));
    }

    #[test]
    fn test_scenario_parsing() {
        let xml = br#"<plugin name="P"><scenario name="Marathon Infinity" id="minf" version="1.0"/></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.required_scenarios.len(), 1);
        assert_eq!(
            p.required_scenarios[0].name.as_deref(),
            Some("Marathon Infinity")
        );
        assert_eq!(p.required_scenarios[0].id.as_deref(), Some("minf"));
        assert_eq!(p.required_scenarios[0].version.as_deref(), Some("1.0"));
    }

    #[test]
    fn test_scenario_name_truncation() {
        let long_name = "A".repeat(50);
        let xml = format!(
            r#"<plugin name="P"><scenario name="{}"/></plugin>"#,
            long_name
        );
        let p = PluginMetadata::from_bytes(xml.as_bytes()).unwrap();
        assert_eq!(p.required_scenarios[0].name.as_ref().unwrap().len(), 31);
    }

    #[test]
    fn test_scenario_id_truncation() {
        let long_id = "x".repeat(30);
        let xml = format!(
            r#"<plugin name="P"><scenario name="N" id="{}"/></plugin>"#,
            long_id
        );
        let p = PluginMetadata::from_bytes(xml.as_bytes()).unwrap();
        assert_eq!(p.required_scenarios[0].id.as_ref().unwrap().len(), 23);
    }

    #[test]
    fn test_scenario_version_truncation() {
        let long_ver = "1".repeat(20);
        let xml = format!(
            r#"<plugin name="P"><scenario name="N" version="{}"/></plugin>"#,
            long_ver
        );
        let p = PluginMetadata::from_bytes(xml.as_bytes()).unwrap();
        assert_eq!(p.required_scenarios[0].version.as_ref().unwrap().len(), 7);
    }

    #[test]
    fn test_scenario_missing_name_and_id_skipped() {
        let xml = br#"<plugin name="P"><scenario version="1.0"/></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert!(p.required_scenarios.is_empty());
    }

    #[test]
    fn test_mml_files_sorted() {
        let xml =
            br#"<plugin name="P"><mml file="c.mml"/><mml file="a.mml"/><mml file="b.mml"/></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.mml_files, vec!["a.mml", "b.mml", "c.mml"]);
    }

    #[test]
    fn test_hud_lua_attribute() {
        let xml = br#"<plugin name="P" hud_lua="Scripts/hud.lua"/>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.hud_lua.as_deref(), Some("Scripts/hud.lua"));
    }

    #[test]
    fn test_stats_lua_attribute() {
        let xml = br#"<plugin name="P" stats_lua="Scripts/stats.lua"/>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.stats_lua.as_deref(), Some("Scripts/stats.lua"));
    }

    #[test]
    fn test_solo_lua_element_with_write_access() {
        let xml = br#"<plugin name="P"><solo_lua file="solo.lua"><write_access>fog</write_access><write_access>music</write_access></solo_lua></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.solo_lua.as_deref(), Some("solo.lua"));
        assert!(p.solo_lua_write_access.contains(SoloLuaWriteAccess::FOG));
        assert!(p.solo_lua_write_access.contains(SoloLuaWriteAccess::MUSIC));
        assert!(!p.solo_lua_write_access.contains(SoloLuaWriteAccess::WORLD));
    }

    #[test]
    fn test_solo_lua_no_write_access_defaults_world() {
        let xml = br#"<plugin name="P"><solo_lua file="solo.lua"/></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.solo_lua.as_deref(), Some("solo.lua"));
        assert_eq!(p.solo_lua_write_access, SoloLuaWriteAccess::WORLD);
    }

    #[test]
    fn test_legacy_solo_lua_attribute() {
        let xml = br#"<plugin name="P" solo_lua="legacy.lua"/>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.solo_lua.as_deref(), Some("legacy.lua"));
        assert_eq!(p.solo_lua_write_access, SoloLuaWriteAccess::WORLD);
    }

    #[test]
    fn test_solo_lua_element_overrides_attribute() {
        let xml = br#"<plugin name="P" solo_lua="legacy.lua"><solo_lua file="element.lua"><write_access>fog</write_access></solo_lua></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.solo_lua.as_deref(), Some("element.lua"));
        assert!(p.solo_lua_write_access.contains(SoloLuaWriteAccess::FOG));
    }

    #[test]
    fn test_multiple_solo_lua_elements_clears() {
        let xml = br#"<plugin name="P"><solo_lua file="a.lua"/><solo_lua file="b.lua"/></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert!(p.solo_lua.is_none());
    }

    #[test]
    fn test_write_access_all_flags() {
        let xml = br#"<plugin name="P"><solo_lua file="s.lua"><write_access>world</write_access><write_access>fog</write_access><write_access>music</write_access><write_access>overlays</write_access><write_access>ephemera</write_access><write_access>sound</write_access></solo_lua></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(
            p.solo_lua_write_access,
            SoloLuaWriteAccess::WORLD
                | SoloLuaWriteAccess::FOG
                | SoloLuaWriteAccess::MUSIC
                | SoloLuaWriteAccess::OVERLAYS
                | SoloLuaWriteAccess::EPHEMERA
                | SoloLuaWriteAccess::SOUND
        );
    }

    #[test]
    fn test_shapes_patch() {
        let xml = br#"<plugin name="P"><shapes_patch file="a.shpA" requires_opengl="true"/><shapes_patch file="b.shpB"/></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.shapes_patches.len(), 2);
        assert_eq!(p.shapes_patches[0].file, "a.shpA");
        assert!(p.shapes_patches[0].requires_opengl);
        assert_eq!(p.shapes_patches[1].file, "b.shpB");
        assert!(!p.shapes_patches[1].requires_opengl);
    }

    #[test]
    fn test_sounds_patch() {
        let xml = br#"<plugin name="P"><sounds_patch file="patch.sndA"/></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.sounds_patches, vec!["patch.sndA"]);
    }

    #[test]
    fn test_map_patch() {
        let xml = br#"<plugin name="P"><map_patch><checksum>12345</checksum><checksum>67890</checksum><resource type="snd " id="100" data="sounds/custom.rsrc"/></map_patch></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.map_patches.len(), 1);
        let patch = &p.map_patches[0];
        assert!(patch.checksums.contains(&12345));
        assert!(patch.checksums.contains(&67890));
        assert_eq!(patch.resources.len(), 1);
        assert_eq!(
            patch.resources[0].resource_type,
            four_chars(b's', b'n', b'd', b' ')
        );
        assert_eq!(patch.resources[0].id, 100);
        assert_eq!(patch.resources[0].data, "sounds/custom.rsrc");
    }

    #[test]
    fn test_map_patch_bad_type_length_skipped() {
        let xml = br#"<plugin name="P"><map_patch><checksum>1</checksum><resource type="sn" id="1" data="x"/></map_patch></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        // resource skipped (type not 4 bytes), patch skipped (no resources)
        assert!(p.map_patches.is_empty());
    }

    #[test]
    fn test_map_patch_no_checksums_skipped() {
        let xml = br#"<plugin name="P"><map_patch><resource type="snd " id="1" data="x"/></map_patch></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert!(p.map_patches.is_empty());
    }

    #[test]
    fn test_map_patch_no_resources_skipped() {
        let xml = br#"<plugin name="P"><map_patch><checksum>1</checksum></map_patch></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert!(p.map_patches.is_empty());
    }

    #[test]
    fn test_theme_dir_clears_resources() {
        let xml = br#"<plugin name="P" theme_dir="resources" hud_lua="hud.lua"><shapes_patch file="p.shpA"/><sounds_patch file="p.sndA"/><map_patch><checksum>1</checksum><resource type="snd " id="1" data="x"/></map_patch></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.theme_dir.as_deref(), Some("resources"));
        assert!(p.hud_lua.is_none());
        assert!(p.solo_lua.is_none());
        assert!(p.shapes_patches.is_empty());
        assert!(p.sounds_patches.is_empty());
        assert!(p.map_patches.is_empty());
    }

    #[test]
    fn test_unknown_elements_ignored() {
        let xml = br#"<plugin name="P"><unknown_element foo="bar"><nested/></unknown_element><mml file="a.mml"/></plugin>"#;
        let p = PluginMetadata::from_bytes(xml).unwrap();
        assert_eq!(p.mml_files, vec!["a.mml"]);
    }

    #[test]
    fn test_malformed_xml() {
        // Completely unparseable - no valid root element
        let xml = b"not xml at all <<<>>>";
        assert!(PluginMetadata::from_bytes(xml).is_err());
    }

    #[test]
    fn test_sort_plugins() {
        let mut plugins = vec![
            PluginMetadata::from_bytes(br#"<plugin name="Charlie"/>"#).unwrap(),
            PluginMetadata::from_bytes(br#"<plugin name="Alpha"/>"#).unwrap(),
            PluginMetadata::from_bytes(br#"<plugin name="Bravo"/>"#).unwrap(),
        ];
        sort_plugins(&mut plugins);
        assert_eq!(plugins[0].name, "Alpha");
        assert_eq!(plugins[1].name, "Bravo");
        assert_eq!(plugins[2].name, "Charlie");
    }

    #[test]
    fn test_resolve_exclusive_hud_lua() {
        let mut plugins = vec![
            PluginMetadata::from_bytes(br#"<plugin name="A" hud_lua="a.lua"/>"#).unwrap(),
            PluginMetadata::from_bytes(br#"<plugin name="B" hud_lua="b.lua"/>"#).unwrap(),
        ];
        resolve_exclusive_resources(&mut plugins);
        assert!(plugins[0].hud_lua.is_none()); // overridden
        assert_eq!(plugins[1].hud_lua.as_deref(), Some("b.lua")); // last wins
    }

    #[test]
    fn test_resolve_exclusive_stats_lua() {
        let mut plugins = vec![
            PluginMetadata::from_bytes(br#"<plugin name="A" stats_lua="a.lua"/>"#).unwrap(),
            PluginMetadata::from_bytes(br#"<plugin name="B" stats_lua="b.lua"/>"#).unwrap(),
        ];
        resolve_exclusive_resources(&mut plugins);
        assert!(plugins[0].stats_lua.is_none());
        assert_eq!(plugins[1].stats_lua.as_deref(), Some("b.lua"));
    }

    #[test]
    fn test_type_str_to_u32_valid() {
        assert_eq!(
            type_str_to_u32("snd "),
            Some(four_chars(b's', b'n', b'd', b' '))
        );
    }

    #[test]
    fn test_type_str_to_u32_wrong_length() {
        assert_eq!(type_str_to_u32("sn"), None);
        assert_eq!(type_str_to_u32("sndxx"), None);
    }
}
