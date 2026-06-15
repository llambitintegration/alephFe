# Tasks: fix-transfer-mode-enums

## Phase A: Fix constant values

- [x] **Fix Rust constants in `marathon-viewer/src/transfer.rs`**: Correct existing 6 constants to match Alephone values (TRANSFER_PULSATE=4, TRANSFER_WOBBLE=5, TRANSFER_STATIC=7). Rename TRANSFER_SLIDE to TRANSFER_HORIZONTAL_SLIDE (value=15). Add all 22 missing constants (fade_out_to_black=1 through 4x=27).

- [x] **Fix WGSL constants in `marathon-viewer/src/shader.wgsl`**: Update the `// Transfer mode constants` block to define all 28 modes with correct Alephone values. Rename TRANSFER_SLIDE to TRANSFER_HORIZONTAL_SLIDE.

- [x] **Fix WGSL constants in `marathon-game/src/shader.wgsl`**: Same constant updates as marathon-viewer shader.

- [x] **Fix WGSL constants in `marathon-web/src/shader.wgsl`**: Same constant updates as marathon-web shader.

- [x] **Update all Rust code referencing `TRANSFER_SLIDE`**: Find and replace with `TRANSFER_HORIZONTAL_SLIDE` in any Rust source files that reference the old name.

## Phase B: Add shader branches for new modes

- [ ] **Add fast_wobble branch (6)** in all three shader files: Same UV distortion as wobble but with 2x frequency (time * 4.0 instead of time * 2.0).

- [ ] **Add 50percent_static handling (8)** in all three shader files: Use hash function to determine per-pixel whether to show noise or base texture (50% probability).

- [ ] **Add pulsating_static handling (12)** in all three shader files: Generate noise with sinusoidal intensity modulation over time.

- [ ] **Update horizontal_slide branch (15)** in all three shader files: Existing slide logic at new correct constant value. Ensure it uses `time * 0.5` for U offset.

- [ ] **Add fast_horizontal_slide branch (16)** in all three shader files: Same as horizontal_slide with `time * 1.0` (doubled speed).

- [ ] **Add vertical_slide branch (17)** in all three shader files: Offset V coordinate by `time * 0.5`.

- [ ] **Add fast_vertical_slide branch (18)** in all three shader files: Offset V coordinate by `time * 1.0` (doubled speed).

- [ ] **Add wander branch (19)** in all three shader files: Pseudo-random UV drift using layered sine waves at incommensurate frequencies.

- [ ] **Add fast_wander branch (20)** in all three shader files: Same as wander with 2x speed multiplier.

- [ ] **Add big_landscape branch (21)** in all three shader files: Same view-angle projection as landscape but with wider FOV scaling on U coordinate.

- [ ] **Add reverse slide branches (22-25)** in all three shader files: Negate the direction of the corresponding forward slide mode.

- [ ] **Add 2x branch (26)** in all three shader files: Return `uv * 2.0`.

- [ ] **Add 4x branch (27)** in all three shader files: Return `uv * 4.0`.

## Phase C: Fallback stubs for unimplemented modes

- [ ] **Add TODO fallback comments** in all three shader files for: fade_out_to_black (1), invisibility (2), subtle_invisibility (3), smear (10), fade_out_static (11), fold_in (13), fold_out (14). These fall through to the default (normal) case with a comment documenting the intended behavior.

## Phase D: Testing and verification

- [ ] **Visual test: pulsate surfaces** render with correct pulsating animation (not at old enum value 1 which was fade_out_to_black).

- [ ] **Visual test: wobble surfaces** render with correct wobble animation (not at old enum value 2 which was invisibility).

- [ ] **Visual test: static surfaces** render noise (not at old enum value 6 which was fast_wobble).

- [ ] **Visual test: slide surfaces** scroll horizontally at correct speed (constant renamed and value changed from 4 to 15).

- [ ] **Visual test: vertical slide** on a map with vertical scrolling textures (e.g., waterfalls).

- [ ] **Visual test: 2x/4x scaling** produces visible texture tiling increase.

- [ ] **Build all three crates** (marathon-viewer, marathon-game, marathon-web) to verify no compilation errors from renamed constant.
