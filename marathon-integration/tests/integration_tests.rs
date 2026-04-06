//! Integration tests for marathon-integration.
//!
//! Tests that require real Marathon data files (WAD, Shapes) are skipped
//! when the files are absent. See marathon-formats/tests/fixtures/README.md
//! for instructions on obtaining test data.

use std::path::PathBuf;

use marathon_formats::wad::WadFile;

use marathon_integration::shell::film::{
    deserialize_film, serialize_film, FilmPlayer, FilmRecorder,
};
use marathon_integration::shell::level::{load_level_map, LevelLoadError};
use marathon_integration::shell::save::SaveManager;
use marathon_integration::shell::states::{is_valid_transition, TickAccumulator};
use marathon_integration::types::{ActionFlags, Difficulty, GameConfig, GameModeType, GameState};

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

// ── 11.1: Load real Marathon level and verify state transitions ──────

#[test]
fn load_real_marathon_level_and_verify_state_transitions() {
    let path = match fixture("Map") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: marathon-formats/tests/fixtures/Map not found (real data required)");
            return;
        }
    };

    let wad = WadFile::open(&path).expect("failed to open Map WAD file");
    assert!(wad.entry_count() > 0, "WAD should have at least one level");

    // Load the first level
    let loaded = load_level_map(&wad, 0).expect("failed to load level 0");
    assert_eq!(loaded.level_index, 0);

    // Verify the map data has essential geometry
    assert!(
        !loaded.map_data.endpoints.is_empty(),
        "loaded level should have endpoints"
    );
    assert!(
        !loaded.map_data.lines.is_empty(),
        "loaded level should have lines"
    );
    assert!(
        !loaded.map_data.polygons.is_empty(),
        "loaded level should have polygons"
    );

    // Verify game state transitions for a typical play session
    // Loading -> Playing (level loaded)
    assert!(is_valid_transition(GameState::Loading, GameState::Playing));
    // Playing -> Paused (player pauses)
    assert!(is_valid_transition(GameState::Playing, GameState::Paused));
    // Paused -> Playing (player resumes)
    assert!(is_valid_transition(GameState::Paused, GameState::Playing));
    // Playing -> Terminal (player activates terminal)
    assert!(is_valid_transition(GameState::Playing, GameState::Terminal));
    // Terminal -> Playing (player exits terminal)
    assert!(is_valid_transition(GameState::Terminal, GameState::Playing));
    // Playing -> Intermission (level complete)
    assert!(is_valid_transition(
        GameState::Playing,
        GameState::Intermission
    ));
    // Intermission -> Loading (next level)
    assert!(is_valid_transition(
        GameState::Intermission,
        GameState::Loading
    ));
}

#[test]
fn load_invalid_level_index_fails() {
    let path = match fixture("Map") {
        Some(p) => p,
        None => {
            eprintln!("SKIP: marathon-formats/tests/fixtures/Map not found");
            return;
        }
    };

    let wad = WadFile::open(&path).expect("failed to open Map WAD file");
    let result = load_level_map(&wad, 9999);
    assert!(
        matches!(result, Err(LevelLoadError::InvalidLevelIndex { .. })),
        "loading out-of-range level should fail with InvalidLevelIndex"
    );
}

#[test]
fn tick_accumulator_simulation_timing() {
    // Simulate a sequence of frames at ~60 FPS and verify tick counting
    let mut accumulator = TickAccumulator::new();
    let frame_time_micros = 16_667; // ~60 FPS

    // First frame: 16.667ms < 33.333ms tick duration -> 0 ticks
    let ticks = accumulator.accumulate(frame_time_micros);
    assert_eq!(ticks, 0);

    // Second frame: 33.334ms accumulated -> 1 tick
    let ticks = accumulator.accumulate(frame_time_micros);
    assert_eq!(ticks, 1);

    // Interpolation factor should be near 0.0 (just after a tick)
    let factor = accumulator.interpolation_factor();
    assert!(factor < 0.1, "interpolation should be near 0 right after tick");

    // Simulate a slow frame (100ms) -> should catch up with 3 ticks
    let ticks = accumulator.accumulate(100_000);
    assert!(
        ticks >= 2,
        "slow frame should produce multiple catch-up ticks"
    );
}

// ── 11.2: Full save/load/film round-trip with simulated play ─────────

#[test]
fn save_load_film_round_trip() {
    // --- Step 1: Simulate a short play session, recording a film ---
    let level_index = 3;
    let difficulty = Difficulty::MajorDamage;
    let game_mode = GameModeType::Campaign;
    let random_seed = 42u64;

    let mut recorder = FilmRecorder::new(level_index, difficulty, game_mode, random_seed);

    // Simulate 90 ticks (3 seconds at 30 tps)
    let simulated_ticks: Vec<ActionFlags> = vec![
        ActionFlags::MOVE_FORWARD,
        ActionFlags::MOVE_FORWARD | ActionFlags::STRAFE_LEFT,
        ActionFlags::MOVE_FORWARD | ActionFlags::FIRE_PRIMARY,
        ActionFlags::TURN_RIGHT,
        ActionFlags::MOVE_BACKWARD,
        ActionFlags::ACTION,
        ActionFlags::empty(),
        ActionFlags::FIRE_SECONDARY,
        ActionFlags::CYCLE_WEAPON_FWD,
        ActionFlags::MOVE_FORWARD | ActionFlags::LOOK_UP,
    ];

    // Record 90 ticks cycling through the simulated actions
    for i in 0..90 {
        recorder.record_tick(simulated_ticks[i % simulated_ticks.len()]);
    }

    assert_eq!(recorder.tick_count(), 90);
    assert!(recorder.is_recording());

    let film = recorder.finish();

    // --- Step 2: Serialize and deserialize the film ---
    let film_bytes = serialize_film(&film).expect("film serialization should succeed");
    assert!(!film_bytes.is_empty());

    let restored_film = deserialize_film(&film_bytes).expect("film deserialization should succeed");
    assert_eq!(restored_film.header.level_index, level_index);
    assert_eq!(restored_film.header.difficulty, difficulty);
    assert_eq!(restored_film.header.game_mode, game_mode);
    assert_eq!(restored_film.header.random_seed, random_seed);
    assert_eq!(restored_film.ticks.len(), 90);

    // --- Step 3: Play back and verify deterministic replay ---
    let mut player = FilmPlayer::new(restored_film);
    assert_eq!(player.header().level_index, level_index);
    assert_eq!(player.header().random_seed, random_seed);
    assert_eq!(player.total_ticks(), 90);

    // Verify each tick matches what was recorded
    for i in 0..90 {
        let expected = simulated_ticks[i % simulated_ticks.len()];
        let actual = player.next_tick().expect("should have tick available");
        assert_eq!(
            actual, expected,
            "film playback tick {i} should match recorded input"
        );
    }
    assert!(player.is_finished());
    assert_eq!(player.next_tick(), None);

    // --- Step 4: Save game state to disk and reload ---
    let save_dir = std::env::temp_dir().join(format!(
        "marathon_integration_test_{}",
        rand::random::<u32>()
    ));
    let save_manager = SaveManager::new(save_dir.clone());

    let terminals_read = vec![0, 2, 5, 7];
    let fake_sim_state = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

    let config = GameConfig {
        difficulty,
        game_mode,
        starting_level: 0,
        record_film: true,
    };

    let save_data = SaveManager::create_save_data(
        level_index,
        &config,
        terminals_read.clone(),
        fake_sim_state.clone(),
    );

    save_manager.save(0, &save_data).expect("save should succeed");

    // Load it back
    let loaded = save_manager.load(0).expect("load should succeed");
    assert_eq!(loaded.level_index, level_index);
    assert_eq!(loaded.difficulty, difficulty);
    assert_eq!(loaded.game_mode, game_mode);
    assert_eq!(loaded.terminals_read, terminals_read);
    assert_eq!(loaded.sim_state, fake_sim_state);

    // --- Step 5: Verify save slot listing ---
    let slots = save_manager.list_slots();
    assert!(slots[0].is_some());
    let slot_info = slots[0].as_ref().unwrap();
    assert_eq!(slot_info.level_index, level_index);
    assert_eq!(slot_info.difficulty, difficulty);

    // Remaining slots should be empty
    for slot in &slots[1..] {
        assert!(slot.is_none());
    }

    // --- Step 6: Overwrite and verify ---
    let updated_save = SaveManager::create_save_data(
        level_index + 1,
        &config,
        vec![0, 1, 2, 3, 4, 5],
        vec![255; 20],
    );
    save_manager
        .save(0, &updated_save)
        .expect("overwrite should succeed");

    let reloaded = save_manager.load(0).expect("reload should succeed");
    assert_eq!(reloaded.level_index, level_index + 1);
    assert_eq!(reloaded.terminals_read.len(), 6);

    // Cleanup
    let _ = std::fs::remove_dir_all(&save_dir);
}

// ── 11.3: Verify HUD rendering pipeline produces valid wgpu output ───

#[test]
fn hud_pipeline_produces_valid_draw_list_at_multiple_resolutions() {
    use marathon_integration::hud::pipeline::HudPipeline;
    use marathon_integration::hud::{HudState, InventoryItem, RadarEntity, RadarEntityType};

    let state = HudState {
        health: 100,
        max_health: 150,
        shield: 200,
        oxygen: 60,
        max_oxygen: 100,
        in_vacuum: true,
        weapon_icon_index: Some(3),
        primary_ammo: Some(52),
        secondary_ammo: Some(7),
        inventory_items: vec![
            InventoryItem { icon_index: 1, count: 2 },
            InventoryItem { icon_index: 5, count: 1 },
            InventoryItem { icon_index: 8, count: 4 },
        ],
        player_x: 500,
        player_y: 300,
        player_facing: 16384, // 90 degrees (north)
        nearby_entities: vec![
            RadarEntity { x: 510, y: 310, entity_type: RadarEntityType::Enemy },
            RadarEntity { x: 490, y: 305, entity_type: RadarEntityType::Ally },
            RadarEntity { x: 505, y: 295, entity_type: RadarEntityType::Item },
        ],
    };

    // Test at multiple resolutions
    let resolutions = [(640, 480), (1280, 720), (1920, 1080), (2560, 1440)];

    for (width, height) in resolutions {
        let pipeline = HudPipeline::new(width, height);
        let list = pipeline.build_draw_list(&state);

        // Validate structural requirements:

        // Must have quads (health bg, health fill, shield bg, shield fill,
        // oxygen bg, oxygen fill, 3 inventory bg)
        assert!(
            list.quads.len() >= 9,
            "resolution {width}x{height}: expected >= 9 quads (bars + oxygen + inventory), got {}",
            list.quads.len()
        );

        // Must have the radar circle
        assert_eq!(
            list.circles.len(),
            1,
            "resolution {width}x{height}: expected 1 radar circle"
        );

        // Must have radar dots for in-range entities
        assert!(
            !list.dots.is_empty(),
            "resolution {width}x{height}: expected radar dots for nearby entities"
        );

        // Must have sprites (weapon icon + 3 inventory item icons)
        assert!(
            list.sprites.len() >= 4,
            "resolution {width}x{height}: expected >= 4 sprites (weapon + inventory), got {}",
            list.sprites.len()
        );

        // Must have texts (primary ammo, secondary ammo, inventory counts for items with count > 1)
        assert!(
            list.texts.len() >= 2,
            "resolution {width}x{height}: expected >= 2 texts (ammo counts), got {}",
            list.texts.len()
        );

        // Validate all quads have positive dimensions
        for (i, quad) in list.quads.iter().enumerate() {
            assert!(
                quad.rect[2] >= 0.0 && quad.rect[3] >= 0.0,
                "resolution {width}x{height}: quad {i} has non-positive dimensions: {:?}",
                quad.rect
            );
        }

        // Validate all quads are within screen bounds (with some tolerance for bar fills at 0)
        for (i, quad) in list.quads.iter().enumerate() {
            assert!(
                quad.rect[0] >= -1.0 && quad.rect[1] >= -1.0,
                "resolution {width}x{height}: quad {i} position is off-screen: {:?}",
                quad.rect
            );
        }

        // Validate radar circle center is within screen
        let circle = &list.circles[0];
        assert!(
            circle.center[0] >= 0.0
                && circle.center[0] <= width as f32
                && circle.center[1] >= 0.0
                && circle.center[1] <= height as f32,
            "resolution {width}x{height}: radar circle center off-screen: {:?}",
            circle.center
        );
        assert!(circle.radius > 0.0, "radar circle must have positive radius");

        // Validate all colors have values in [0, 1] range
        for quad in &list.quads {
            for &component in &quad.color {
                assert!(
                    (0.0..=1.0).contains(&component),
                    "resolution {width}x{height}: quad color component out of range: {}",
                    component
                );
            }
        }

        // Validate text font sizes are positive
        for text in &list.texts {
            assert!(
                text.font_size > 0.0,
                "resolution {width}x{height}: text font size must be positive"
            );
        }
    }
}

#[test]
fn hud_pipeline_minimal_state() {
    use marathon_integration::hud::pipeline::HudPipeline;
    use marathon_integration::hud::HudState;

    // Minimal state: no weapon, no inventory, no enemies, no vacuum
    let state = HudState {
        health: 0,
        max_health: 150,
        shield: 0,
        oxygen: 100,
        max_oxygen: 100,
        in_vacuum: false,
        weapon_icon_index: None,
        primary_ammo: None,
        secondary_ammo: None,
        inventory_items: vec![],
        player_x: 0,
        player_y: 0,
        player_facing: 0,
        nearby_entities: vec![],
    };

    let pipeline = HudPipeline::new(640, 480);
    let list = pipeline.build_draw_list(&state);

    // Should still have health and shield bars (4 quads)
    assert!(list.quads.len() >= 4);
    // Radar circle should still render
    assert_eq!(list.circles.len(), 1);
    // No radar dots (no entities)
    assert!(list.dots.is_empty());
    // No weapon sprite (no weapon equipped)
    assert!(list.sprites.is_empty());
    // No ammo text
    assert!(list.texts.is_empty());
    // No oxygen meter (not in vacuum)
    assert_eq!(list.quads.len(), 4);
}

#[test]
fn hud_pipeline_resize_scales_elements() {
    use marathon_integration::hud::pipeline::HudPipeline;
    use marathon_integration::hud::HudState;

    let state = HudState {
        health: 100,
        max_health: 150,
        shield: 100,
        oxygen: 50,
        max_oxygen: 100,
        in_vacuum: false,
        weapon_icon_index: Some(1),
        primary_ammo: Some(10),
        secondary_ammo: None,
        inventory_items: vec![],
        player_x: 0,
        player_y: 0,
        player_facing: 0,
        nearby_entities: vec![],
    };

    let mut pipeline = HudPipeline::new(640, 480);
    let list_small = pipeline.build_draw_list(&state);

    pipeline.resize(1920, 1080);
    let list_large = pipeline.build_draw_list(&state);

    // Health bar should be wider at higher resolution
    let small_health_bg_width = list_small.quads[0].rect[2];
    let large_health_bg_width = list_large.quads[0].rect[2];
    assert!(
        large_health_bg_width > small_health_bg_width,
        "health bar should scale with resolution: {small_health_bg_width} vs {large_health_bg_width}"
    );

    // Radar should be larger
    assert!(
        list_large.circles[0].radius > list_small.circles[0].radius,
        "radar should scale with resolution"
    );
}
