## Why

The Rust codebase defines transfer mode constants with wrong numeric values that do not match alephone's `map.h` enum. For example, `TRANSFER_PULSATE` is 1 but alephone defines `_xfer_pulsate` as 4. `TRANSFER_WOBBLE` is 2 instead of 5. `TRANSFER_SLIDE` is 4 instead of 15. `TRANSFER_STATIC` is 6 instead of 7. Only `TRANSFER_NORMAL` (0) and `TRANSFER_LANDSCAPE` (9) are correct. Since transfer mode IDs are read directly from Marathon map files, wrong constants mean the shader applies the wrong effect to every surface that uses a non-zero, non-landscape transfer mode. Additionally, only 6 of 28 transfer modes are defined, so the remaining 22 modes silently fall back to normal rendering instead of their intended visual effect.

## What Changes

- Correct all 6 existing transfer mode constants in `transfer.rs` and all three `shader.wgsl` files (marathon-viewer, marathon-game, marathon-web) to match alephone's `map.h` enum values (0-27)
- Add the 22 missing transfer mode constants covering: fade_out_to_black (1), invisibility (2), subtle_invisibility (3), fast_wobble (6), 50percent_static (8), smear (10), fade_out_static (11), pulsating_static (12), fold_in (13), fold_out (14), fast_horizontal_slide (16), vertical_slide (17), fast_vertical_slide (18), wander (19), fast_wander (20), big_landscape (21), reverse_horizontal_slide (22), reverse_fast_horizontal_slide (23), reverse_vertical_slide (24), reverse_fast_vertical_slide (25), 2x (26), 4x (27)
- Implement shader branches for the visually impactful modes: fast_wobble, vertical_slide, fast_horizontal_slide, fast_vertical_slide, reverse slide variants, wander, fast_wander, big_landscape, 50percent_static, 2x, 4x
- Unimplemented modes (fade, invisibility, fold, smear) fall back to normal rendering with a documented TODO
- Rename constants from `TRANSFER_SLIDE` to `TRANSFER_HORIZONTAL_SLIDE` to match alephone naming

## Capabilities

### New Capabilities

- `fast-wobble`: Higher-frequency UV wobble distortion (mode 6)
- `vertical-slide`: Texture scrolls vertically (mode 17), with fast variant (mode 18)
- `reverse-slides`: Texture scrolls in reverse for horizontal (22, 23) and vertical (24, 25)
- `wander`: Pseudo-random UV drift (mode 19), with fast variant (mode 20)
- `big-landscape`: Wider FOV landscape projection (mode 21)
- `50percent-static`: Half-pixel noise overlay blended with base texture (mode 8)
- `texture-scaling`: 2x (mode 26) and 4x (mode 27) texture coordinate scaling

### Modified Capabilities

- `transfer-modes`: All constant values corrected to match alephone enum; `TRANSFER_SLIDE` renamed to `TRANSFER_HORIZONTAL_SLIDE` (value changed from 4 to 15)
- `texture-pipeline`: Shader `apply_transfer_mode()` expanded from 5 branches to 16+ branches

## Impact

- `marathon-viewer/src/transfer.rs` -- Correct existing constants, add 22 new constants
- `marathon-viewer/src/shader.wgsl` -- Fix constant values, add new shader branches
- `marathon-game/src/shader.wgsl` -- Same fixes (shared shader pattern)
- `marathon-web/src/shader.wgsl` -- Same fixes (different shader structure but same constants and apply_transfer_mode function)
- `openspec/specs/transfer-modes/spec.md` -- Update "Transfer mode constants" requirement from wrong values to correct alephone values; add requirements for new modes
- Any Rust code referencing `TRANSFER_SLIDE` must update to `TRANSFER_HORIZONTAL_SLIDE`
- No API or data format changes -- map file parsing already reads the correct values from disk; only the interpretation constants were wrong
