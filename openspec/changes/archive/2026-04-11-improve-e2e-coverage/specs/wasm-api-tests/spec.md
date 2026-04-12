## ADDED Requirements

### Requirement: marathon-web level module is testable via wasm-bindgen-test
The `marathon-web` crate SHALL have a `wasm-bindgen-test` test suite that exercises the `level` module's non-GPU functions.

#### Scenario: enumerate_levels returns entries from a valid WAD
- **WHEN** a WAD file containing at least one parseable level is passed to `level::enumerate_levels`
- **THEN** the returned `Vec<LevelInfo>` SHALL have length >= 1, and each entry SHALL have a non-empty `name`

#### Scenario: load_level returns map data for valid index
- **WHEN** `level::load_level` is called with a valid WAD and index 0
- **THEN** it SHALL return `Ok(LoadedLevel)` with non-empty `map.polygons`

#### Scenario: load_level returns error for invalid index
- **WHEN** `level::load_level` is called with index 9999
- **THEN** it SHALL return `Err` containing "out of range"

### Requirement: marathon-web texture utility is testable via wasm-bindgen-test
The `marathon-web` crate SHALL have tests for the `texture::pad_layer_count_for_webgl` function.

#### Scenario: pad_layer_count avoids 1 (would map to D2)
- **WHEN** `pad_layer_count_for_webgl(1)` is called
- **THEN** the result SHALL be >= 2

#### Scenario: pad_layer_count avoids 6 (would map to Cube)
- **WHEN** `pad_layer_count_for_webgl(6)` is called
- **THEN** the result SHALL be 7 (padded past 6)

#### Scenario: pad_layer_count avoids multiples of 6 above 6
- **WHEN** `pad_layer_count_for_webgl(12)` is called
- **THEN** the result SHALL be 13 (padded past 12)

#### Scenario: pad_layer_count passes safe values unchanged
- **WHEN** `pad_layer_count_for_webgl(5)` is called
- **THEN** the result SHALL be 5

### Requirement: marathon-web mesh module produces valid geometry
The `marathon-web` crate SHALL have wasm-bindgen tests verifying that `build_level_mesh` produces valid output from synthetic map data.

#### Scenario: build_level_mesh from a single-polygon map
- **WHEN** `build_level_mesh` is called with a MapData containing one 4-vertex polygon with floor and ceiling
- **THEN** the resulting `LevelMesh` SHALL have non-empty `vertices` and `indices`, index count SHALL be a multiple of 3, and all indices SHALL be less than the vertex count
