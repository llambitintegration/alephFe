//! Integration tests for marathon-formats.
//!
//! Tests against real Marathon data files are skipped when the files are absent.
//! See tests/fixtures/README.md for instructions on obtaining test data.
//!
//! Tests against sample MML and Plugin.xml files run unconditionally.

use std::path::PathBuf;

use marathon_formats::mml::MmlDocument;
use marathon_formats::plugin::PluginMetadata;
use marathon_formats::tags::WadTag;
use marathon_formats::wad::WadFile;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> Option<PathBuf> {
    let path = fixtures_dir().join(name);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

// ── 10.2: WAD parsing – Marathon 2 map files ────────────────────────

#[test]
fn test_wad_m2_map_parsing() {
    let path = match fixture("Map") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: tests/fixtures/Map not found");
            return;
        }
    };

    let wad = WadFile::open(&path).expect("failed to open Map file");
    // Marathon 2 maps should have a valid header
    assert!(
        wad.header.version >= 0 && wad.header.version <= 4,
        "unexpected WAD version {}",
        wad.header.version
    );
    assert!(wad.entry_count() > 0, "map should have at least one level");

    // Check first level has standard map tags
    let entry = wad.entry(0).unwrap();
    assert!(
        entry.get_tag_data(WadTag::Endpoints).is_some()
            || entry.get_tag_data(WadTag::Points).is_some(),
        "level should have point/endpoint data"
    );
}

// ── 10.3: WAD parsing – Marathon Infinity map files ─────────────────

#[test]
fn test_wad_infinity_map_parsing() {
    // Look for Marathon Infinity map (version 4)
    let path = match fixture("Map.sceA").or_else(|| fixture("Map")) {
        Some(p) => p,
        None => {
            eprintln!("SKIP: no map file found in fixtures");
            return;
        }
    };

    let wad = WadFile::open(&path).expect("failed to open Map file");
    if wad.header.version != 4 {
        eprintln!(
            "SKIP: map is version {} (expected 4 for Infinity test)",
            wad.header.version
        );
        return;
    }

    assert!(wad.entry_count() > 0);
    // Infinity WADs should have directory entries with index fields
    let entry = wad.entry(0).unwrap();
    assert!(!entry.all_tags().is_empty(), "entry should have tags");
}

// ── 10.4: Map geometry ──────────────────────────────────────────────

#[test]
fn test_map_geometry_parsing() {
    let path = match fixture("Map.sceA").or_else(|| fixture("Map")) {
        Some(p) => p,
        None => {
            eprintln!("SKIP: no map file found in fixtures");
            return;
        }
    };

    let wad = WadFile::open(&path).expect("failed to open Map file");
    let entry = wad.entry(0).unwrap();

    let map = marathon_formats::map::MapData::from_entry(&entry).expect("failed to parse map data");

    // A real level should have geometry
    assert!(!map.endpoints.is_empty(), "level should have endpoints");
    assert!(!map.lines.is_empty(), "level should have lines");
    assert!(!map.polygons.is_empty(), "level should have polygons");

    // Snapshot assertions for Marathon 2 "Waterloo Waterpark" (level 0)
    // Values from data-marathon-2 commit eaf21a7
    assert_eq!(wad.entry_count(), 41, "Marathon 2 should have 41 levels");
    assert_eq!(map.endpoints.len(), 716, "level 0 endpoint count");
    assert_eq!(map.lines.len(), 1106, "level 0 line count");
    assert_eq!(map.polygons.len(), 369, "level 0 polygon count");

    // Run cross-reference validation
    let errors = map.validate();
    // Some levels may have benign reference issues, but it shouldn't panic
    if !errors.is_empty() {
        eprintln!("NOTE: {} cross-reference warnings on level 0", errors.len());
    }
}

// ── 10.5: Shapes parsing ────────────────────────────────────────────

#[test]
fn test_shapes_parsing() {
    let path = match fixture("Shapes") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: tests/fixtures/Shapes not found");
            return;
        }
    };

    let shapes =
        marathon_formats::shapes::ShapesFile::open(&path).expect("failed to open Shapes file");

    // Shapes files have 32 collection headers
    assert_eq!(shapes.headers().len(), 32);

    // At least some collections should have data
    let mut found_collection = false;
    let mut collections_with_data = 0u32;
    for i in 0..32 {
        if let Ok(col) = shapes.collection(i) {
            collections_with_data += 1;
            if !found_collection {
                found_collection = true;
                // Collection should have valid structure
                assert!(
                    !col.color_tables.is_empty(),
                    "collection {} should have CLUTs",
                    i
                );
                // Check that at least one bitmap can be parsed
                if !col.bitmaps.is_empty() {
                    let bm = &col.bitmaps[0];
                    assert!(bm.width > 0 && bm.height > 0);
                    assert_eq!(
                        bm.pixels.len(),
                        bm.width as usize * bm.height as usize,
                        "bitmap pixel count mismatch"
                    );
                }
            }
        }
    }
    assert!(
        found_collection,
        "should have at least one valid collection"
    );

    // Snapshot assertions for Marathon 2 Shapes
    // Values from data-marathon-2 commit eaf21a7
    assert_eq!(
        collections_with_data, 29,
        "Marathon 2 Shapes should have 29 collections with data"
    );
    let col0 = shapes.collection(0).unwrap();
    assert_eq!(col0.color_tables.len(), 1, "collection 0 CLUT count");
    assert_eq!(col0.bitmaps.len(), 56, "collection 0 bitmap count");
    assert_eq!(col0.bitmaps[0].width, 180, "collection 0 bitmap 0 width");
    assert_eq!(col0.bitmaps[0].height, 11, "collection 0 bitmap 0 height");
}

// ── 10.6: Sounds parsing ────────────────────────────────────────────

#[test]
fn test_sounds_parsing() {
    let path = match fixture("Sounds") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: tests/fixtures/Sounds not found");
            return;
        }
    };

    let sounds =
        marathon_formats::sounds::SoundsFile::open(&path).expect("failed to open Sounds file");
    let header = sounds.header();

    // Validate header
    assert_eq!(header.tag, 0x736E6432, "should have 'snd2' tag");
    assert!(header.sound_count > 0, "should have sounds");

    // Parse at least one sound definition
    let def = sounds.sound(0).expect("failed to get sound definition 0");
    // Permutation count should be reasonable
    assert!(def.permutations <= 5, "permutations should be 0-5");
}

// ── 10.7: Physics parsing ───────────────────────────────────────────

#[test]
fn test_physics_parsing() {
    let path = match fixture("Physics Model") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: tests/fixtures/Physics Model not found");
            return;
        }
    };

    let wad = WadFile::open(&path).expect("failed to open Physics file");
    assert!(wad.entry_count() > 0, "physics WAD should have entries");

    let entry = wad.entry(0).unwrap();
    let physics = marathon_formats::physics::PhysicsData::from_entry(&entry)
        .expect("failed to parse physics data");

    // Should have player physics (walking + running models)
    let constants = physics.physics.expect("should have physics constants");
    assert!(!constants.is_empty(), "should have physics constants");

    // Snapshot assertions for Marathon 2 Standard.phyA
    // Values from data-marathon-2 commit eaf21a7
    assert_eq!(
        constants.len(),
        2,
        "should have 2 physics models (walking + running)"
    );
    assert!(
        (constants[0].maximum_forward_velocity - 0.07142639).abs() < 0.0001,
        "walking forward velocity should match known value, got {}",
        constants[0].maximum_forward_velocity
    );
}

// ── 10.8: MML parsing ───────────────────────────────────────────────

#[test]
fn test_mml_sample_file() {
    let path = fixtures_dir().join("sample.mml");
    let doc = MmlDocument::from_file(&path).expect("failed to parse sample.mml");

    assert!(doc.weapons.is_some(), "should have weapons section");
    assert!(doc.monsters.is_some(), "should have monsters section");
    assert!(
        doc.dynamic_limits.is_some(),
        "should have dynamic_limits section"
    );
    assert!(doc.sounds.is_some(), "should have sounds section");
    assert!(doc.opengl.is_none(), "should not have opengl section");

    // Verify weapons section content
    let weapons = doc.weapons.unwrap();
    assert_eq!(weapons.elements.len(), 2, "should have 2 weapon elements");
    assert_eq!(weapons.elements[0].attributes.get("index").unwrap(), "0");
}

#[test]
fn test_mml_layering_with_file() {
    let base_xml = b"<marathon><weapons><weapon index=\"0\" speed=\"100\"/></weapons><monsters><monster index=\"0\"/></monsters></marathon>";
    let overlay_path = fixtures_dir().join("sample.mml");

    let base = MmlDocument::from_bytes(base_xml).unwrap();
    let overlay = MmlDocument::from_file(&overlay_path).unwrap();

    let result = MmlDocument::layer(base, overlay);
    // weapons should be overridden by sample.mml
    let weapons = result.weapons.unwrap();
    assert_eq!(
        weapons.elements[0].attributes.get("speed").unwrap(),
        "200",
        "overlay should replace base weapons"
    );
    // monsters should be overridden by sample.mml
    assert!(result.monsters.is_some());
}

// ── 10.9: Plugin metadata parsing ───────────────────────────────────

#[test]
fn test_plugin_sample_file() {
    let path = fixtures_dir().join("sample_plugin/Plugin.xml");
    let plugin = PluginMetadata::from_file(&path).expect("failed to parse sample Plugin.xml");

    assert_eq!(plugin.name, "Sample Plugin");
    assert_eq!(plugin.version.as_deref(), Some("1.0"));
    assert_eq!(
        plugin.description.as_deref(),
        Some("A sample plugin for testing")
    );
    assert!(plugin.auto_enable);

    // Scenario requirements
    assert_eq!(plugin.required_scenarios.len(), 1);
    assert_eq!(
        plugin.required_scenarios[0].name.as_deref(),
        Some("Marathon Infinity")
    );
    assert_eq!(plugin.required_scenarios[0].id.as_deref(), Some("minf"));

    // Resource references
    assert_eq!(plugin.mml_files, vec!["overrides.mml"]);
    assert_eq!(plugin.shapes_patches.len(), 1);
    assert_eq!(plugin.shapes_patches[0].file, "sprites.shpA");
    assert!(plugin.shapes_patches[0].requires_opengl);
    assert_eq!(plugin.sounds_patches, vec!["effects.sndA"]);
}

#[test]
fn test_plugin_validate_references() {
    let path = fixtures_dir().join("sample_plugin/Plugin.xml");
    let mut plugin = PluginMetadata::from_file(&path).unwrap();
    let plugin_dir = fixtures_dir().join("sample_plugin");

    // Validate references — none of the referenced files exist in the fixture
    plugin.validate_references(&plugin_dir);

    // All file references should be cleared since the files don't exist
    assert!(
        plugin.mml_files.is_empty(),
        "missing MML files should be cleared"
    );
    assert!(
        plugin.shapes_patches.is_empty(),
        "missing shapes patches should be cleared"
    );
    assert!(
        plugin.sounds_patches.is_empty(),
        "missing sounds patches should be cleared"
    );
}

#[test]
fn test_plugin_load_ordering() {
    let mut plugins = vec![
        PluginMetadata::from_bytes(br#"<plugin name="Zeta" hud_lua="z.lua"/>"#).unwrap(),
        PluginMetadata::from_bytes(br#"<plugin name="Alpha" hud_lua="a.lua"/>"#).unwrap(),
        PluginMetadata::from_bytes(br#"<plugin name="Mid" stats_lua="m.lua"/>"#).unwrap(),
    ];

    marathon_formats::plugin::sort_plugins(&mut plugins);
    assert_eq!(plugins[0].name, "Alpha");
    assert_eq!(plugins[1].name, "Mid");
    assert_eq!(plugins[2].name, "Zeta");

    marathon_formats::plugin::resolve_exclusive_resources(&mut plugins);
    // Alpha's hud_lua should be overridden (Zeta is last)
    assert!(plugins[0].hud_lua.is_none());
    // Zeta's hud_lua should win
    assert_eq!(plugins[2].hud_lua.as_deref(), Some("z.lua"));
    // Mid's stats_lua is the only one, should remain
    assert_eq!(plugins[1].stats_lua.as_deref(), Some("m.lua"));
}

// ── 10.10: Community scenario cross-format test ─────────────────────

#[test]
fn test_community_scenario_cross_format() {
    // This test requires a complete community scenario (e.g., Rubicon, Phoenix, Eternal).
    // Place the scenario data files in tests/fixtures/ to enable this test.
    let map_path = fixture("Map.sceA")
        .or_else(|| fixture("Map"))
        .or_else(|| fixture("Map.sce2"));
    let shapes_path = fixture("Shapes");
    let sounds_path = fixture("Sounds");

    let (map_path, shapes_path, sounds_path) = match (map_path, shapes_path, sounds_path) {
        (Some(m), Some(sh), Some(sn)) => (m, sh, sn),
        _ => {
            eprintln!("SKIP: community scenario files not found in fixtures");
            return;
        }
    };

    // Parse all formats — none should error
    let wad = WadFile::open(&map_path).expect("Map parse failed");
    assert!(wad.entry_count() > 0);

    let _shapes =
        marathon_formats::shapes::ShapesFile::open(&shapes_path).expect("Shapes parse failed");
    let _sounds =
        marathon_formats::sounds::SoundsFile::open(&sounds_path).expect("Sounds parse failed");

    // Parse map data from first level
    let entry = wad.entry(0).unwrap();
    let _map =
        marathon_formats::map::MapData::from_entry(&entry).expect("Map geometry parse failed");

    // If physics model exists, parse it too
    if let Some(phys_path) = fixture("Physics Model") {
        let phys_wad = WadFile::open(&phys_path).expect("Physics WAD parse failed");
        let phys_entry = phys_wad.entry(0).unwrap();
        let _physics = marathon_formats::physics::PhysicsData::from_entry(&phys_entry)
            .expect("Physics parse failed");
    }

    // If any MML files exist, try parsing them
    for mml_name in &["sample.mml"] {
        if let Some(mml_path) = fixture(mml_name) {
            let _mml = MmlDocument::from_file(&mml_path).expect("MML parse failed");
        }
    }
}

// ── GPL Fixture Tests: Aleph One Engine MML Files ──────────────────

fn alephone_fixture(name: &str) -> PathBuf {
    fixtures_dir().join("alephone").join(name)
}

#[test]
fn test_alephone_mml_carnage_messages() {
    let doc =
        MmlDocument::from_file(alephone_fixture("Carnage_Messages.mml")).expect("parse failed");

    assert!(doc.console.is_some(), "should have console section");
    let console = doc.console.unwrap();
    assert!(
        !console.elements.is_empty(),
        "console section should have carnage_message elements"
    );
    assert_eq!(console.elements[0].name, "carnage_message");
    assert!(console.elements[0]
        .attributes
        .contains_key("projectile_type"));
}

#[test]
fn test_alephone_mml_transparent_liquids() {
    let doc =
        MmlDocument::from_file(alephone_fixture("Transparent_Liquids.mml")).expect("parse failed");

    assert!(doc.opengl.is_some(), "should have opengl section");
    let opengl = doc.opengl.unwrap();
    assert!(
        opengl.elements.len() > 10,
        "opengl section should have many texture elements"
    );
    assert_eq!(opengl.elements[0].name, "texture");
    assert!(opengl.elements[0].attributes.contains_key("opac_type"));
}

#[test]
fn test_alephone_mml_transparent_sprites() {
    let doc =
        MmlDocument::from_file(alephone_fixture("Transparent_Sprites.mml")).expect("parse failed");

    assert!(doc.opengl.is_some(), "should have opengl section");
    let opengl = doc.opengl.unwrap();
    assert!(
        opengl.elements.len() > 50,
        "should have many texture elements (sprites file is 213 lines)"
    );
}

#[test]
fn test_alephone_mml_marathon_2_scenario() {
    let doc = MmlDocument::from_file(alephone_fixture("Marathon_2.mml")).expect("parse failed");

    assert!(doc.scenario.is_some(), "should have scenario section");
    assert!(doc.interface.is_some(), "should have interface section");

    let interface = doc.interface.unwrap();
    assert_eq!(interface.elements.len(), 4, "should have 4 rect elements");
    assert_eq!(interface.elements[0].name, "rect");
    assert!(interface.elements[0].attributes.contains_key("index"));
}

// ── GPL Fixture Tests: Aleph One Plugin.xml Files ──────────────────

#[test]
fn test_alephone_plugin_default_theme() {
    let plugin = PluginMetadata::from_file(alephone_fixture("default_theme_Plugin.xml"))
        .expect("parse failed");

    assert_eq!(plugin.name, "Marathon Infinity Theme");
    assert_eq!(plugin.version.as_deref(), Some("1.0"));
    assert_eq!(plugin.theme_dir.as_deref(), Some("resources"));
}

#[test]
fn test_alephone_plugin_basic_hud() {
    let plugin =
        PluginMetadata::from_file(alephone_fixture("BasicHUD_Plugin.xml")).expect("parse failed");

    assert_eq!(plugin.name, "Basic HUD");
    assert_eq!(plugin.hud_lua.as_deref(), Some("Basic HUD.lua"));
    assert!(!plugin.auto_enable, "auto_enable should be false");
}

#[test]
fn test_alephone_plugin_enhanced_hud() {
    let plugin = PluginMetadata::from_file(alephone_fixture("EnhancedHUD_Plugin.xml"))
        .expect("parse failed");

    assert_eq!(plugin.name, "Enhanced HUD");
    assert_eq!(plugin.hud_lua.as_deref(), Some("XBLA.lua"));
    assert_eq!(plugin.mml_files, vec!["FOV.mml"]);
}

#[test]
fn test_alephone_plugin_stats() {
    let plugin =
        PluginMetadata::from_file(alephone_fixture("Stats_Plugin.xml")).expect("parse failed");

    assert_eq!(plugin.name, "Marathon 2 Stats");
    assert_eq!(plugin.stats_lua.as_deref(), Some("stats.lua"));
    assert_eq!(plugin.required_scenarios.len(), 1);
    assert_eq!(
        plugin.required_scenarios[0].name.as_deref(),
        Some("Marathon 2")
    );
}

#[test]
fn test_alephone_plugin_transparent_liquids() {
    let plugin = PluginMetadata::from_file(alephone_fixture("TransparentLiquids_Plugin.xml"))
        .expect("parse failed");

    assert_eq!(plugin.name, "Transparent Liquids");
    assert_eq!(plugin.mml_files, vec!["Transparent_Liquids.mml"]);
    assert_eq!(
        plugin.required_scenarios.len(),
        2,
        "should require both Marathon 2 and Marathon Infinity"
    );

    let scenario_names: Vec<_> = plugin
        .required_scenarios
        .iter()
        .filter_map(|s| s.name.as_deref())
        .collect();
    assert!(scenario_names.contains(&"Marathon 2"));
    assert!(scenario_names.contains(&"Marathon Infinity"));
}

// ── GPL Fixture Tests: MML Layering ────────────────────────────────

#[test]
fn test_alephone_mml_layering_opengl_over_base() {
    // Base has weapons; overlay (Transparent_Liquids.mml) has opengl
    let base = MmlDocument::from_bytes(
        b"<marathon><weapons><weapon index=\"0\" speed=\"100\"/></weapons></marathon>",
    )
    .unwrap();
    let overlay =
        MmlDocument::from_file(alephone_fixture("Transparent_Liquids.mml")).expect("parse failed");

    let result = MmlDocument::layer(base, overlay);

    // weapons from base should survive (overlay has no weapons section)
    assert!(
        result.weapons.is_some(),
        "base weapons should survive layering"
    );
    // opengl from overlay should be present
    assert!(result.opengl.is_some(), "overlay opengl should be applied");
    let opengl = result.opengl.unwrap();
    assert!(opengl.elements.len() > 10);
}
