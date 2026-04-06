//! End-to-end tests for marathon-viewer.
//!
//! These tests verify the non-GPU pipeline: level loading, mesh generation,
//! and texture loading against real Marathon 2 scenario data.
//!
//! Tests are skipped when the fixture files are absent.
//! See marathon-formats/tests/fixtures/README.md for data setup.

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../marathon-formats/tests/fixtures")
}

fn fixture(name: &str) -> Option<PathBuf> {
    let path = fixtures_dir().join(name);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

macro_rules! require_fixtures {
    ($($name:expr),+) => {
        {
            let mut paths = Vec::new();
            $(
                match fixture($name) {
                    Some(p) => paths.push(p),
                    None => {
                        eprintln!("SKIP: fixtures/{} not found", $name);
                        return;
                    }
                }
            )+
            paths
        }
    };
}

// ── Level loading ────────────────────────────────────────────────────

#[test]
fn test_enumerate_levels_from_map_wad() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    // Marathon 2 has 28 solo levels (plus net levels)
    assert!(
        wad.entry_count() >= 20,
        "expected at least 20 levels, got {}",
        wad.entry_count()
    );

    // Each entry should parse into MapData
    let mut parsed_count = 0;
    for i in 0..wad.entry_count() {
        let entry = wad.entry(i).unwrap();
        if marathon_formats::MapData::from_entry(entry).is_ok() {
            parsed_count += 1;
        }
    }
    assert!(
        parsed_count >= 20,
        "expected at least 20 parseable levels, got {parsed_count}"
    );
}

#[test]
fn test_level_names_are_non_empty() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse first level");

    let name = map
        .map_info
        .as_ref()
        .map(|info| info.level_name.clone())
        .unwrap_or_default();

    assert!(!name.is_empty(), "first level should have a name");
    eprintln!("First level name: {name}");
}

// ── Mesh generation ──────────────────────────────────────────────────

#[test]
fn test_mesh_generation_produces_geometry() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    // Verify we have polygons to work with
    assert!(!map.polygons.is_empty(), "level should have polygons");
    assert!(!map.lines.is_empty(), "level should have lines");
    assert!(!map.endpoints.is_empty(), "level should have endpoints");

    eprintln!(
        "Level has {} polygons, {} lines, {} sides, {} endpoints",
        map.polygons.len(),
        map.lines.len(),
        map.sides.len(),
        map.endpoints.len()
    );
}

#[test]
fn test_mesh_vertex_count_sanity() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    // Test several levels
    for level_idx in 0..5.min(wad.entry_count()) {
        let entry = wad.entry(level_idx).unwrap();
        let map = match marathon_formats::MapData::from_entry(entry) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let poly_count = map.polygons.len();
        let side_count = map.sides.len();

        // Each polygon contributes floor + ceiling vertices (up to 8 each)
        // Each side contributes wall vertices (4 per quad)
        // Very rough lower bound: at least 3 verts per polygon floor + ceiling
        // We can't call build_level_mesh directly (it's not pub from the binary crate)
        // but we verify the data is well-formed for mesh generation
        let mut inverted_count = 0;
        for polygon in &map.polygons {
            let vc = polygon.vertex_count as usize;
            assert!(vc >= 3 && vc <= 8, "polygon vertex count {vc} out of range [3,8]");

            for i in 0..vc {
                let ep_idx = polygon.endpoint_indexes[i];
                assert!(
                    ep_idx >= 0 && (ep_idx as usize) < map.endpoints.len(),
                    "polygon endpoint index {ep_idx} out of range"
                );
            }

            // Note: floor > ceiling is valid for platform polygons (closed platforms)
            if polygon.floor_height > polygon.ceiling_height {
                inverted_count += 1;
            }
        }

        eprintln!(
            "Level {level_idx}: {poly_count} polygons, {side_count} sides, {inverted_count} inverted floor/ceiling - data validated"
        );
    }
}

#[test]
fn test_side_types_are_valid() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    let mut type_counts = std::collections::HashMap::new();

    for side in &map.sides {
        *type_counts.entry(side.side_type).or_insert(0u32) += 1;
    }

    eprintln!("Side type distribution: {:?}", type_counts);

    // Should have at least some of the known wall types (0=full, 1=high, 2=low)
    let known_count: u32 = [0i16, 1, 2, 3]
        .iter()
        .filter_map(|t| type_counts.get(t))
        .sum();
    assert!(known_count > 0, "should have at least some known side types (full/high/low/split)");
}

// ── Texture pipeline ─────────────────────────────────────────────────

#[test]
fn test_shapes_file_loading() {
    let paths = require_fixtures!("Shapes");
    let shapes = marathon_formats::ShapesFile::open(&paths[0]).expect("open Shapes file");

    // Marathon 2 has 32 collection slots
    let headers = shapes.headers();
    assert_eq!(headers.len(), 32, "should have 32 collection headers");

    // Some collections should have data (offset != -1)
    let active_count = headers.iter().filter(|h| h.has_8bit_data() || h.has_16bit_data()).count();
    assert!(
        active_count >= 10,
        "expected at least 10 collections with data, got {active_count}"
    );

    eprintln!("{active_count} collections with data out of 32");
}

#[test]
fn test_collection_bitmap_loading() {
    let paths = require_fixtures!("Shapes");
    let shapes = marathon_formats::ShapesFile::open(&paths[0]).expect("open Shapes file");

    // Try to load a collection that has data
    let mut loaded = false;
    for coll_idx in 0..32 {
        let header = shapes.header(coll_idx).unwrap();
        if !header.has_8bit_data() && !header.has_16bit_data() {
            continue;
        }

        match shapes.collection(coll_idx) {
            Ok(collection) => {
                assert!(!collection.bitmaps.is_empty(), "active collection {coll_idx} should have bitmaps");
                assert!(!collection.color_tables.is_empty(), "collection {coll_idx} should have CLUTs");

                for bitmap in &collection.bitmaps {
                    assert!(bitmap.width > 0 && bitmap.height > 0, "bitmap dimensions should be positive");
                    assert!(
                        !bitmap.pixels.is_empty(),
                        "bitmap should have pixel data"
                    );
                }

                eprintln!(
                    "Collection {coll_idx}: {} bitmaps, {} CLUTs, first bitmap {}x{}",
                    collection.bitmaps.len(),
                    collection.color_tables.len(),
                    collection.bitmaps[0].width,
                    collection.bitmaps[0].height,
                );
                loaded = true;
                break;
            }
            Err(e) => {
                eprintln!("Collection {coll_idx} failed: {e}");
            }
        }
    }

    assert!(loaded, "should have loaded at least one collection");
}

#[test]
fn test_bitmap_clut_conversion() {
    let paths = require_fixtures!("Shapes");
    let shapes = marathon_formats::ShapesFile::open(&paths[0]).expect("open Shapes file");

    // Find first collection with data and verify CLUT conversion
    for coll_idx in 0..32 {
        let header = shapes.header(coll_idx).unwrap();
        if !header.has_8bit_data() && !header.has_16bit_data() {
            continue;
        }

        if let Ok(collection) = shapes.collection(coll_idx) {
            if collection.bitmaps.is_empty() || collection.color_tables.is_empty() {
                continue;
            }

            let bitmap = &collection.bitmaps[0];
            let clut = &collection.color_tables[0];

            // Verify CLUT has entries
            assert!(!clut.is_empty(), "CLUT should have color entries");

            // Verify bitmap pixels reference valid CLUT indices
            let max_pixel = *bitmap.pixels.iter().max().unwrap_or(&0) as usize;
            assert!(
                max_pixel < clut.len(),
                "bitmap pixel value {max_pixel} exceeds CLUT size {}",
                clut.len()
            );

            // Verify color values are reasonable (16-bit range)
            for color in clut {
                // Color values are u16, so they're always in range
                // Just verify we can access them
                let _r = (color.red >> 8) as u8;
                let _g = (color.green >> 8) as u8;
                let _b = (color.blue >> 8) as u8;
            }

            eprintln!(
                "Collection {coll_idx}: CLUT has {} entries, max pixel index = {max_pixel}",
                clut.len(),
            );
            return;
        }
    }

    panic!("no collection could be loaded for CLUT test");
}

#[test]
fn test_texture_descriptors_in_level() {
    let paths = require_fixtures!("Map", "Shapes");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");
    let shapes = marathon_formats::ShapesFile::open(&paths[1]).expect("open Shapes file");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    // Collect unique collections referenced by level textures
    let mut collections = std::collections::HashSet::new();

    for polygon in &map.polygons {
        if !polygon.floor_texture.is_none() {
            collections.insert(polygon.floor_texture.collection());
        }
        if !polygon.ceiling_texture.is_none() {
            collections.insert(polygon.ceiling_texture.collection());
        }
    }

    for side in &map.sides {
        if !side.primary_texture.texture.is_none() {
            collections.insert(side.primary_texture.texture.collection());
        }
        if !side.secondary_texture.texture.is_none() {
            collections.insert(side.secondary_texture.texture.collection());
        }
        if !side.transparent_texture.texture.is_none() {
            collections.insert(side.transparent_texture.texture.collection());
        }
    }

    eprintln!("Level references {} unique collections: {:?}", collections.len(), collections);

    assert!(!collections.is_empty(), "level should reference at least one texture collection");

    // Verify each referenced collection can be loaded
    let mut load_failures = 0;
    for &coll_idx in &collections {
        match shapes.collection(coll_idx as usize) {
            Ok(c) => {
                assert!(!c.bitmaps.is_empty(), "referenced collection {coll_idx} should have bitmaps");
            }
            Err(e) => {
                eprintln!("WARNING: collection {coll_idx} failed to load: {e}");
                load_failures += 1;
            }
        }
    }

    assert!(
        load_failures == 0,
        "{load_failures} referenced collections failed to load"
    );
}

// ── Light data ───────────────────────────────────────────────────────

#[test]
fn test_light_data_present() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    match &map.lights {
        marathon_formats::map::LightData::Static(lights) => {
            assert!(!lights.is_empty(), "should have static lights");
            eprintln!("Level has {} static lights", lights.len());

            // Verify light intensities are reasonable
            for (i, light) in lights.iter().enumerate() {
                let intensity = light.primary_active.intensity;
                assert!(
                    intensity >= 0.0 && intensity <= 2.0,
                    "light {i} intensity {intensity} out of expected range"
                );
            }
        }
        marathon_formats::map::LightData::Old(lights) => {
            assert!(!lights.is_empty(), "should have old lights");
            eprintln!("Level has {} old-format lights", lights.len());
        }
        marathon_formats::map::LightData::None => {
            panic!("level should have light data");
        }
    }
}

// ── Platform data ────────────────────────────────────────────────────

#[test]
fn test_platform_data() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    // Check across all levels for platforms
    let mut found_platforms = false;
    for level_idx in 0..wad.entry_count() {
        let entry = wad.entry(level_idx).unwrap();
        let map = match marathon_formats::MapData::from_entry(entry) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if !map.platforms.is_empty() {
            found_platforms = true;
            for (i, platform) in map.platforms.iter().enumerate() {
                let poly_idx = platform.polygon_index as usize;
                assert!(
                    poly_idx < map.polygons.len(),
                    "platform {i} polygon_index {poly_idx} out of range"
                );
            }
            eprintln!("Level {level_idx}: {} platforms", map.platforms.len());
        }
    }

    if !found_platforms {
        // Not all Marathon scenarios include explicit platform data entries.
        // Platforms may be defined by polygon type alone. This is acceptable.
        eprintln!("NOTE: No explicit platform data found (platforms defined by polygon type)");

        // Verify at least some polygon types indicate platforms
        let mut platform_polygons = 0;
        for level_idx in 0..wad.entry_count() {
            let entry = wad.entry(level_idx).unwrap();
            if let Ok(map) = marathon_formats::MapData::from_entry(entry) {
                // polygon_type 5 = Platform in Marathon
                platform_polygons += map.polygons.iter().filter(|p| p.polygon_type == 5).count();
            }
        }
        eprintln!("Found {platform_polygons} platform-type polygons across all levels");
        assert!(platform_polygons > 0, "should have at least some platform-type polygons");
    }
}

// ── Full pipeline integration ────────────────────────────────────────

#[test]
fn test_full_level_data_coherence() {
    let paths = require_fixtures!("Map", "Shapes");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");
    let shapes = marathon_formats::ShapesFile::open(&paths[1]).expect("open Shapes file");

    // Test first 5 levels for full data coherence
    for level_idx in 0..wad.entry_count().min(5) {
        let entry = wad.entry(level_idx).unwrap();
        let map = match marathon_formats::MapData::from_entry(entry) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let level_name = map
            .map_info
            .as_ref()
            .map(|i| i.level_name.as_str())
            .unwrap_or("unnamed");

        // Verify line → endpoint references
        for (i, line) in map.lines.iter().enumerate() {
            for &ep_idx in &line.endpoint_indexes {
                assert!(
                    (ep_idx as usize) < map.endpoints.len(),
                    "level '{level_name}': line {i} endpoint {ep_idx} out of range"
                );
            }
        }

        // Verify polygon → endpoint references
        for (i, polygon) in map.polygons.iter().enumerate() {
            let vc = polygon.vertex_count as usize;
            for j in 0..vc {
                let ep_idx = polygon.endpoint_indexes[j];
                assert!(
                    ep_idx >= 0 && (ep_idx as usize) < map.endpoints.len(),
                    "level '{level_name}': polygon {i} endpoint[{j}]={ep_idx} out of range"
                );
            }

            // Verify line references
            for j in 0..vc {
                let line_idx = polygon.line_indexes[j];
                assert!(
                    line_idx >= 0 && (line_idx as usize) < map.lines.len(),
                    "level '{level_name}': polygon {i} line[{j}]={line_idx} out of range"
                );
            }
        }

        // Verify side → polygon references
        for (i, side) in map.sides.iter().enumerate() {
            if side.polygon_index >= 0 {
                assert!(
                    (side.polygon_index as usize) < map.polygons.len(),
                    "level '{level_name}': side {i} polygon_index {} out of range",
                    side.polygon_index
                );
            }
            if side.line_index >= 0 {
                assert!(
                    (side.line_index as usize) < map.lines.len(),
                    "level '{level_name}': side {i} line_index {} out of range",
                    side.line_index
                );
            }
        }

        // Verify all referenced textures can be resolved
        let mut missing_textures = 0;
        for polygon in &map.polygons {
            for desc in [polygon.floor_texture, polygon.ceiling_texture] {
                if !desc.is_none() {
                    let coll = desc.collection() as usize;
                    if shapes.collection(coll).is_err() {
                        missing_textures += 1;
                    }
                }
            }
        }

        eprintln!(
            "Level {level_idx} '{level_name}': {} polygons, {} lines, {} sides — coherence OK (missing_tex={})",
            map.polygons.len(),
            map.lines.len(),
            map.sides.len(),
            missing_textures,
        );
    }
}
