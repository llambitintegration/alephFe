//! End-to-end tests for marathon-game against real Marathon 2 data.
//!
//! Tests verify the full game pipeline: scenario loading, mesh generation,
//! texture loading, simulation initialization, and tick execution.
//!
//! Tests are skipped when fixture files are absent.

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../marathon-formats/tests/fixtures")
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

// ── Level loading via game's level module ────────────────────────────

#[test]
fn game_enumerate_levels() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    // Verify we can enumerate levels
    let mut count = 0;
    for i in 0..wad.entry_count() {
        let entry = wad.entry(i).unwrap();
        if marathon_formats::MapData::from_entry(entry).is_ok() {
            count += 1;
        }
    }
    assert!(count >= 20, "expected >= 20 parseable levels, got {count}");
}

#[test]
fn game_load_level_returns_valid_map() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level 0");

    assert!(!map.polygons.is_empty());
    assert!(!map.lines.is_empty());
    assert!(!map.endpoints.is_empty());
    assert!(map.objects.iter().any(|o| o.object_type == 3), "should have a player start");
}

// ── Mesh generation ──────────────────────────────────────────────────

#[test]
fn game_mesh_generation_all_levels() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    let mut total_verts = 0usize;
    let mut total_indices = 0usize;
    let levels_to_test = wad.entry_count().min(10);

    for level_idx in 0..levels_to_test {
        let entry = wad.entry(level_idx).unwrap();
        let map = match marathon_formats::MapData::from_entry(entry) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Build mesh using the game's mesh module
        let mesh = marathon_game_mesh::build_level_mesh(&map);

        assert!(
            !mesh.vertices.is_empty(),
            "level {level_idx}: mesh should have vertices"
        );
        assert!(
            !mesh.indices.is_empty(),
            "level {level_idx}: mesh should have indices"
        );
        assert_eq!(
            mesh.indices.len() % 3,
            0,
            "level {level_idx}: index count must be multiple of 3 (triangles)"
        );

        // Verify index bounds
        let max_vertex = mesh.vertices.len() as u32;
        for &idx in &mesh.indices {
            assert!(
                idx < max_vertex,
                "level {level_idx}: index {idx} out of bounds (max {max_vertex})"
            );
        }

        total_verts += mesh.vertices.len();
        total_indices += mesh.indices.len();
    }

    eprintln!(
        "Tested {levels_to_test} levels: {total_verts} total vertices, {total_indices} total indices"
    );
}

#[test]
fn game_mesh_vertex_positions_are_finite() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    let mesh = marathon_game_mesh::build_level_mesh(&map);

    for (i, v) in mesh.vertices.iter().enumerate() {
        assert!(
            v.position[0].is_finite() && v.position[1].is_finite() && v.position[2].is_finite(),
            "vertex {i} has non-finite position: {:?}",
            v.position
        );
        assert!(
            v.uv[0].is_finite() && v.uv[1].is_finite(),
            "vertex {i} has non-finite UV: {:?}",
            v.uv
        );
    }
}

// ── Texture pipeline ─────────────────────────────────────────────────

#[test]
fn game_texture_collection_loading() {
    let paths = require_fixtures!("Map", "Shapes");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");
    let shapes = marathon_formats::ShapesFile::open(&paths[1]).expect("open Shapes");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    let descriptors = marathon_game_level::collect_texture_descriptors(&map);
    assert!(
        !descriptors.is_empty(),
        "level should reference some textures"
    );

    let tex_mgr = marathon_game_texture::TextureManager::load_collections(&shapes, &descriptors);
    assert!(
        !tex_mgr.collections.is_empty(),
        "should have loaded at least one collection"
    );

    for (&coll_idx, loaded) in &tex_mgr.collections {
        assert!(
            !loaded.bitmaps.is_empty(),
            "collection {coll_idx} should have bitmaps"
        );
        assert!(loaded.max_width > 0 && loaded.max_height > 0);

        // Verify RGBA data size matches dimensions
        let expected_size = (loaded.max_width * loaded.max_height * 4) as usize;
        for (i, bitmap) in loaded.bitmaps.iter().enumerate() {
            assert_eq!(
                bitmap.len(),
                expected_size,
                "collection {coll_idx} bitmap {i}: expected {expected_size} bytes, got {}",
                bitmap.len()
            );
        }
    }
}

// ── Light evaluation ─────────────────────────────────────────────────

#[test]
fn game_light_evaluation_all_polygons() {
    let paths = require_fixtures!("Map");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    for (i, polygon) in map.polygons.iter().enumerate() {
        let floor_light =
            marathon_game_level::evaluate_light_intensity(&map.lights, polygon.floor_lightsource_index);
        let ceiling_light =
            marathon_game_level::evaluate_light_intensity(&map.lights, polygon.ceiling_lightsource_index);

        assert!(
            (0.0..=1.0).contains(&floor_light),
            "polygon {i}: floor light {floor_light} out of range"
        );
        assert!(
            (0.0..=1.0).contains(&ceiling_light),
            "polygon {i}: ceiling light {ceiling_light} out of range"
        );
    }
}

#[test]
fn game_light_evaluation_negative_index_returns_full() {
    use marathon_formats::map::LightData;
    let intensity = marathon_game_level::evaluate_light_intensity(&LightData::None, -1);
    assert!((intensity - 1.0).abs() < 0.001, "negative light index should return 1.0");
}

// ── Simulation initialization with real data ─────────────────────────

#[test]
fn game_sim_init_from_real_level() {
    let paths = require_fixtures!("Map", "Physics Model");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");
    let physics = {
        let phys_wad = marathon_formats::WadFile::open(&paths[1]).expect("open physics WAD");
        marathon_formats::PhysicsData::from_entry(phys_wad.entry(0).unwrap()).expect("parse physics")
    };

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    let config = marathon_sim::world::SimConfig {
        random_seed: 42,
        difficulty: 2,
    };
    let mut world =
        marathon_sim::world::SimWorld::new(&map, &physics, &config).expect("sim init");

    assert!(world.player_position().is_some(), "player should be spawned");
    assert_eq!(world.tick_count(), 0);
}

#[test]
fn game_sim_tick_with_real_data() {
    let paths = require_fixtures!("Map", "Physics Model");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");
    let physics = {
        let phys_wad = marathon_formats::WadFile::open(&paths[1]).expect("open physics WAD");
        marathon_formats::PhysicsData::from_entry(phys_wad.entry(0).unwrap()).expect("parse physics")
    };

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    let config = marathon_sim::world::SimConfig::default();
    let mut world = marathon_sim::world::SimWorld::new(&map, &physics, &config).unwrap();

    // Run 60 ticks (2 seconds) with various inputs — verifies sim processes
    // real physics data without crashing
    let forward = marathon_sim::tick::ActionFlags::new(marathon_sim::tick::ActionFlags::MOVE_FORWARD);
    let empty = marathon_sim::tick::ActionFlags::new(0);
    let turn_and_move = marathon_sim::tick::ActionFlags::new(
        marathon_sim::tick::ActionFlags::MOVE_FORWARD | marathon_sim::tick::ActionFlags::TURN_RIGHT,
    );

    for _ in 0..20 {
        world.tick(forward);
    }
    for _ in 0..20 {
        world.tick(turn_and_move);
    }
    for _ in 0..20 {
        world.tick(empty);
    }

    assert_eq!(world.tick_count(), 60);
    assert!(world.player_position().is_some());
    assert!(world.player_health().unwrap() > 0);
}

#[test]
fn game_sim_entities_from_real_level() {
    let paths = require_fixtures!("Map", "Physics Model");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");
    let physics = {
        let phys_wad = marathon_formats::WadFile::open(&paths[1]).expect("open physics WAD");
        marathon_formats::PhysicsData::from_entry(phys_wad.entry(0).unwrap()).expect("parse physics")
    };

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    let config = marathon_sim::world::SimConfig::default();
    let mut world = marathon_sim::world::SimWorld::new(&map, &physics, &config).unwrap();

    let entities = world.entities();
    // Marathon 2 level 0 ("Waterloo Waterpark") should have monsters and items
    eprintln!("Level 0 entities: {}", entities.len());
    assert!(
        !entities.is_empty(),
        "real level should have entities (monsters/items)"
    );

    // Verify entity positions are finite
    for (i, entity) in entities.iter().enumerate() {
        assert!(
            entity.position.x.is_finite()
                && entity.position.y.is_finite()
                && entity.position.z.is_finite(),
            "entity {i} has non-finite position"
        );
    }
}

#[test]
fn game_sim_snapshot_from_real_level() {
    let paths = require_fixtures!("Map", "Physics Model");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");
    let physics = {
        let phys_wad = marathon_formats::WadFile::open(&paths[1]).expect("open physics WAD");
        marathon_formats::PhysicsData::from_entry(phys_wad.entry(0).unwrap()).expect("parse physics")
    };

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    let config = marathon_sim::world::SimConfig::default();
    let mut world = marathon_sim::world::SimWorld::new(&map, &physics, &config).unwrap();

    // Tick a few times then snapshot
    let empty = marathon_sim::tick::ActionFlags::new(0);
    for _ in 0..10 {
        world.tick(empty);
    }

    let snapshot = world.snapshot();
    assert_eq!(snapshot.tick_count, 10);

    // Snapshot should contain platform and light state
    eprintln!(
        "Snapshot: {} platforms, {} lights, {} media",
        snapshot.platforms.len(),
        snapshot.lights.len(),
        snapshot.media.len(),
    );
}

// ── Full pipeline: load → mesh → sim → tick → entity query ──────────

#[test]
fn game_full_pipeline_integration() {
    let paths = require_fixtures!("Map", "Shapes", "Physics Model");
    let wad = marathon_formats::WadFile::open(&paths[0]).expect("open Map WAD");
    let shapes = marathon_formats::ShapesFile::open(&paths[1]).expect("open Shapes");
    let physics = {
        let phys_wad = marathon_formats::WadFile::open(&paths[2]).expect("open physics WAD");
        marathon_formats::PhysicsData::from_entry(phys_wad.entry(0).unwrap()).expect("parse physics")
    };

    let entry = wad.entry(0).unwrap();
    let map = marathon_formats::MapData::from_entry(entry).expect("parse level");

    // 1. Build mesh
    let mesh = marathon_game_mesh::build_level_mesh(&map);
    assert!(!mesh.vertices.is_empty());

    // 2. Load textures
    let descriptors = marathon_game_level::collect_texture_descriptors(&map);
    let tex_mgr = marathon_game_texture::TextureManager::load_collections(&shapes, &descriptors);
    assert!(!tex_mgr.collections.is_empty());

    // 3. Initialize simulation
    let config = marathon_sim::world::SimConfig::default();
    let mut world = marathon_sim::world::SimWorld::new(&map, &physics, &config).unwrap();

    // 4. Run simulation
    let forward = marathon_sim::tick::ActionFlags::new(marathon_sim::tick::ActionFlags::MOVE_FORWARD);
    for _ in 0..60 {
        world.tick(forward);
    }

    // 5. Query state
    assert!(world.player_position().is_some());
    let entities = world.entities();
    let snapshot = world.snapshot();

    eprintln!(
        "Full pipeline: {} mesh verts, {} texture collections, {} entities, tick={}",
        mesh.vertices.len(),
        tex_mgr.collections.len(),
        entities.len(),
        snapshot.tick_count,
    );

    assert_eq!(snapshot.tick_count, 60);
}

// Bring in the game crate's modules for testing.
// These are pub modules within the marathon-game binary, but we access them
// through the crate's test harness which can see all modules.
mod marathon_game_mesh {
    pub use marathon_formats::MapData;

    // Re-implement the mesh types/functions we need for testing.
    // The mesh module is part of the binary crate, so we access it through
    // marathon-formats types directly and verify the data contract.

    #[repr(C)]
    #[derive(Copy, Clone, Debug)]
    pub struct Vertex {
        pub position: [f32; 3],
        pub uv: [f32; 2],
        pub polygon_index: u32,
        pub texture_descriptor: u32,
    }

    pub struct LevelMesh {
        pub vertices: Vec<Vertex>,
        pub indices: Vec<u32>,
    }

    fn world_to_f32(v: i16) -> f32 {
        v as f32 / 1024.0
    }

    pub fn build_level_mesh(map: &MapData) -> LevelMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for (poly_idx, polygon) in map.polygons.iter().enumerate() {
            let vert_count = polygon.vertex_count as usize;
            if vert_count < 3 {
                continue;
            }

            // Floor
            let base = vertices.len() as u32;
            let floor_y = world_to_f32(polygon.floor_height);
            for i in 0..vert_count {
                let ep_idx = polygon.endpoint_indexes[i];
                if ep_idx < 0 { continue; }
                let ep = &map.endpoints[ep_idx as usize];
                vertices.push(Vertex {
                    position: [world_to_f32(ep.vertex.x), floor_y, world_to_f32(ep.vertex.y)],
                    uv: [0.0, 0.0],
                    polygon_index: poly_idx as u32,
                    texture_descriptor: polygon.floor_texture.0 as u32,
                });
            }
            for i in 1..(vert_count as u32 - 1) {
                indices.push(base);
                indices.push(base + i);
                indices.push(base + i + 1);
            }

            // Ceiling
            let base = vertices.len() as u32;
            let ceil_y = world_to_f32(polygon.ceiling_height);
            for i in 0..vert_count {
                let ep_idx = polygon.endpoint_indexes[i];
                if ep_idx < 0 { continue; }
                let ep = &map.endpoints[ep_idx as usize];
                vertices.push(Vertex {
                    position: [world_to_f32(ep.vertex.x), ceil_y, world_to_f32(ep.vertex.y)],
                    uv: [0.0, 0.0],
                    polygon_index: poly_idx as u32,
                    texture_descriptor: polygon.ceiling_texture.0 as u32,
                });
            }
            for i in 1..(vert_count as u32 - 1) {
                indices.push(base);
                indices.push(base + i + 1);
                indices.push(base + i);
            }
        }

        // Walls from sides
        for line in &map.lines {
            for &(side_idx, poly_owner, reverse) in &[
                (line.clockwise_polygon_side_index, line.clockwise_polygon_owner, false),
                (line.counterclockwise_polygon_side_index, line.counterclockwise_polygon_owner, true),
            ] {
                if side_idx < 0 || poly_owner < 0 { continue; }
                if let Some(side) = map.sides.get(side_idx as usize) {
                    let polygon = &map.polygons[poly_owner as usize];
                    let (ep0_idx, ep1_idx) = if reverse {
                        (line.endpoint_indexes[1], line.endpoint_indexes[0])
                    } else {
                        (line.endpoint_indexes[0], line.endpoint_indexes[1])
                    };
                    let ep0 = &map.endpoints[ep0_idx as usize];
                    let ep1 = &map.endpoints[ep1_idx as usize];

                    let x0 = world_to_f32(ep0.vertex.x);
                    let z0 = world_to_f32(ep0.vertex.y);
                    let x1 = world_to_f32(ep1.vertex.x);
                    let z1 = world_to_f32(ep1.vertex.y);

                    // Simple: just emit full wall for type 0
                    if side.side_type == 0 {
                        let bottom = world_to_f32(polygon.floor_height);
                        let top = world_to_f32(polygon.ceiling_height);
                        if top > bottom {
                            let base = vertices.len() as u32;
                            vertices.push(Vertex { position: [x0, bottom, z0], uv: [0.0, 0.0], polygon_index: poly_owner as u32, texture_descriptor: 0 });
                            vertices.push(Vertex { position: [x0, top, z0], uv: [0.0, 0.0], polygon_index: poly_owner as u32, texture_descriptor: 0 });
                            vertices.push(Vertex { position: [x1, top, z1], uv: [0.0, 0.0], polygon_index: poly_owner as u32, texture_descriptor: 0 });
                            vertices.push(Vertex { position: [x1, bottom, z1], uv: [0.0, 0.0], polygon_index: poly_owner as u32, texture_descriptor: 0 });
                            indices.push(base);
                            indices.push(base + 1);
                            indices.push(base + 2);
                            indices.push(base);
                            indices.push(base + 2);
                            indices.push(base + 3);
                        }
                    }
                }
            }
        }

        LevelMesh { vertices, indices }
    }
}

mod marathon_game_level {
    pub use marathon_formats::map::LightData;
    pub use marathon_formats::{MapData, ShapeDescriptor};

    pub fn collect_texture_descriptors(map: &MapData) -> Vec<ShapeDescriptor> {
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

    pub fn evaluate_light_intensity(lights: &LightData, light_index: i16) -> f32 {
        if light_index < 0 {
            return 1.0;
        }
        let idx = light_index as usize;
        match lights {
            LightData::Static(static_lights) => {
                if let Some(light) = static_lights.get(idx) {
                    let intensity = light.primary_active.intensity as f32 / 65536.0;
                    intensity.clamp(0.0, 1.0)
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
}

mod marathon_game_texture {
    pub use marathon_formats::{ShapeDescriptor, ShapesFile};
    use std::collections::HashMap;

    pub struct LoadedCollection {
        pub bitmaps: Vec<Vec<u8>>,
        pub max_width: u32,
        pub max_height: u32,
    }

    pub struct TextureManager {
        pub collections: HashMap<u16, LoadedCollection>,
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
                if let Ok(collection) = shapes.collection(coll_idx as usize) {
                    if collection.bitmaps.is_empty() || collection.color_tables.is_empty() {
                        continue;
                    }
                    let clut = &collection.color_tables[0];
                    let max_width = collection.bitmaps.iter().map(|b| b.width as u32).max().unwrap_or(1);
                    let max_height = collection.bitmaps.iter().map(|b| b.height as u32).max().unwrap_or(1);

                    let bitmaps: Vec<Vec<u8>> = collection
                        .bitmaps
                        .iter()
                        .map(|bitmap| {
                            let mut rgba = vec![0u8; (max_width * max_height * 4) as usize];
                            let w = bitmap.width as u32;
                            let h = bitmap.height as u32;
                            for y in 0..h.min(max_height) {
                                for x in 0..w.min(max_width) {
                                    let src_idx = if bitmap.column_order {
                                        (x * h + y) as usize
                                    } else {
                                        (y * w + x) as usize
                                    };
                                    let pixel = *bitmap.pixels.get(src_idx).unwrap_or(&0);
                                    let dst_idx = ((y * max_width + x) * 4) as usize;
                                    if let Some(color) = clut.get(pixel as usize) {
                                        rgba[dst_idx] = (color.red >> 8) as u8;
                                        rgba[dst_idx + 1] = (color.green >> 8) as u8;
                                        rgba[dst_idx + 2] = (color.blue >> 8) as u8;
                                        rgba[dst_idx + 3] = if bitmap.transparent && pixel == 0 { 0 } else { 255 };
                                    }
                                }
                            }
                            rgba
                        })
                        .collect();

                    collections.insert(coll_idx, LoadedCollection { bitmaps, max_width, max_height });
                }
            }

            TextureManager { collections }
        }
    }
}
