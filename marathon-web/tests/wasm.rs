//! Integration tests for marathon-web's non-GPU modules.
//!
//! These tests verify the level loading, texture utility, and mesh generation
//! APIs using synthetic data. They run natively via `cargo test` and can also
//! run as WASM tests via `wasm-pack test --headless --chrome`.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test_configure!(run_in_browser);

use marathon_formats::test_helpers::*;
use marathon_formats::tags::WadTag;
use marathon_formats::wad::WadFile;

// ── Level module tests ──────────────────────────────────────────────

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn enumerate_levels_returns_entries_from_valid_wad() {
    let endpoints = MapDataBuilder::endpoints(&[(0, 0), (1024, 0), (1024, 1024), (0, 1024)]);
    let lines = MapDataBuilder::lines(&[
        (0, 1, 0, -1),
        (1, 2, 0, -1),
        (2, 3, 0, -1),
        (3, 0, 0, -1),
    ]);
    let polygon = MapDataBuilder::polygon(4, &[0, 1, 2, 3], &[0, 1, 2, 3]);

    let wad_data = WadBuilder::new()
        .version(4)
        .file_name("Test Map")
        .add_entry(
            0,
            vec![
                TagData::new(WadTag::Endpoints, endpoints),
                TagData::new(WadTag::Lines, lines),
                TagData::new(WadTag::Polygons, polygon),
            ],
        )
        .build();

    let wad = WadFile::from_bytes(&wad_data).unwrap();
    let levels = marathon_web::level::enumerate_levels(&wad);

    assert!(!levels.is_empty(), "should enumerate at least one level");
    assert!(!levels[0].name.is_empty(), "level name should be non-empty");
    assert_eq!(levels[0].index, 0);
}

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn load_level_success_returns_map_with_polygons() {
    let endpoints = MapDataBuilder::endpoints(&[(0, 0), (1024, 0), (1024, 1024), (0, 1024)]);
    let lines = MapDataBuilder::lines(&[
        (0, 1, 0, -1),
        (1, 2, 0, -1),
        (2, 3, 0, -1),
        (3, 0, 0, -1),
    ]);
    let polygon = MapDataBuilder::polygon(4, &[0, 1, 2, 3], &[0, 1, 2, 3]);

    let wad_data = WadBuilder::new()
        .version(4)
        .file_name("Test Map")
        .add_entry(
            0,
            vec![
                TagData::new(WadTag::Endpoints, endpoints),
                TagData::new(WadTag::Lines, lines),
                TagData::new(WadTag::Polygons, polygon),
            ],
        )
        .build();

    let wad = WadFile::from_bytes(&wad_data).unwrap();
    let loaded = marathon_web::level::load_level(&wad, 0).expect("should load level 0");

    assert!(!loaded.map.polygons.is_empty(), "loaded level should have polygons");
}

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn load_level_invalid_index_returns_error() {
    let wad_data = WadBuilder::new()
        .version(4)
        .file_name("Empty Map")
        .add_entry(0, vec![])
        .build();

    let wad = WadFile::from_bytes(&wad_data).unwrap();
    let result = marathon_web::level::load_level(&wad, 9999);

    match result {
        Ok(_) => panic!("should fail for out-of-range index"),
        Err(err) => {
            assert!(
                err.contains("out of range"),
                "error should mention 'out of range', got: {err}"
            );
        }
    }
}

// ── Texture utility tests ───────────────────────────────────────────

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pad_layer_count_avoids_d2_for_single_layer() {
    let result = marathon_web::texture::pad_layer_count_for_webgl(1);
    assert!(result >= 2, "single layer should be padded to >= 2, got {result}");
}

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pad_layer_count_avoids_cube_for_six_layers() {
    let result = marathon_web::texture::pad_layer_count_for_webgl(6);
    assert_eq!(result, 7, "6 layers should be padded to 7 to avoid Cube target");
}

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pad_layer_count_avoids_cube_array_for_multiples_of_six() {
    let result = marathon_web::texture::pad_layer_count_for_webgl(12);
    assert_eq!(result, 13, "12 layers should be padded to 13 to avoid CubeArray");
}

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pad_layer_count_passes_safe_values_unchanged() {
    assert_eq!(marathon_web::texture::pad_layer_count_for_webgl(5), 5);
    assert_eq!(marathon_web::texture::pad_layer_count_for_webgl(7), 7);
    assert_eq!(marathon_web::texture::pad_layer_count_for_webgl(10), 10);
}

// ── Mesh module tests ───────────────────────────────────────────────

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn build_level_mesh_from_single_polygon() {
    use marathon_formats::map::LightData;
    use marathon_formats::*;
    use marathon_web::mesh::{build_level_mesh, PolygonInfo};

    let endpoints = vec![
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 1024,
            vertex: WorldPoint2d { x: 0, y: 0 },
            transformed: WorldPoint2d { x: 0, y: 0 },
            supporting_polygon_index: 0,
        },
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 1024,
            vertex: WorldPoint2d { x: 1024, y: 0 },
            transformed: WorldPoint2d { x: 1024, y: 0 },
            supporting_polygon_index: 0,
        },
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 1024,
            vertex: WorldPoint2d { x: 1024, y: 1024 },
            transformed: WorldPoint2d { x: 1024, y: 1024 },
            supporting_polygon_index: 0,
        },
        Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 1024,
            vertex: WorldPoint2d { x: 0, y: 1024 },
            transformed: WorldPoint2d { x: 0, y: 1024 },
            supporting_polygon_index: 0,
        },
    ];

    let polygon = Polygon {
        polygon_type: 0,
        flags: 0,
        permutation: 0,
        vertex_count: 4,
        endpoint_indexes: [0, 1, 2, 3, -1, -1, -1, -1],
        line_indexes: [-1; 8],
        floor_texture: ShapeDescriptor(0x0100),
        ceiling_texture: ShapeDescriptor(0x0100),
        floor_height: 0,
        ceiling_height: 1024,
        floor_lightsource_index: 0,
        ceiling_lightsource_index: 0,
        area: 1024 * 1024,
        floor_transfer_mode: 0,
        ceiling_transfer_mode: 0,
        adjacent_polygon_indexes: [-1; 8],
        center: WorldPoint2d { x: 512, y: 512 },
        side_indexes: [-1; 8],
        floor_origin: WorldPoint2d { x: 0, y: 0 },
        ceiling_origin: WorldPoint2d { x: 0, y: 0 },
        media_index: -1,
        media_lightsource_index: -1,
        sound_source_indexes: -1,
        ambient_sound_image_index: -1,
        random_sound_image_index: -1,
    };

    let map = MapData {
        endpoints,
        lines: vec![],
        sides: vec![],
        polygons: vec![polygon],
        objects: vec![],
        lights: LightData::None,
        platforms: vec![],
        media: vec![],
        annotations: vec![],
        terminals: vec![],
        ambient_sounds: vec![],
        random_sounds: vec![],
        map_info: None,
        item_placement: vec![],
        guard_paths: None,
    };

    let poly_info = vec![PolygonInfo {
        floor_light: 1.0,
        floor_transfer_mode: 0,
        ceiling_light: 1.0,
        ceiling_transfer_mode: 0,
    }];

    let mesh = build_level_mesh(&map, &poly_info);

    assert!(!mesh.vertices.is_empty(), "mesh should have vertices");
    assert!(!mesh.indices.is_empty(), "mesh should have indices");
    assert_eq!(
        mesh.indices.len() % 3,
        0,
        "index count must be multiple of 3 (triangles)"
    );

    let max_vertex = mesh.vertices.len() as u32;
    for &idx in &mesh.indices {
        assert!(
            idx < max_vertex,
            "index {idx} out of bounds (max {max_vertex})"
        );
    }
}
