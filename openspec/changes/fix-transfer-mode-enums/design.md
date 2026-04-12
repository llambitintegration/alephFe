# Design: fix-transfer-mode-enums

## Context

The Rust codebase defines 6 transfer mode constants in `transfer.rs` and the three `shader.wgsl` files with values that do not match Alephone's `map.h` enum. Since transfer mode IDs are read directly from Marathon map files, wrong constants mean the shader applies the wrong visual effect to every surface using a non-zero, non-landscape transfer mode. Additionally, 22 of 28 transfer modes are missing entirely, causing those surfaces to silently fall back to normal rendering.

### Current (wrong) values

| Constant            | Rust value | Alephone value | Status |
|---------------------|-----------|----------------|--------|
| TRANSFER_NORMAL     | 0         | 0              | Correct |
| TRANSFER_PULSATE    | 1         | 4              | WRONG |
| TRANSFER_WOBBLE     | 2         | 5              | WRONG |
| TRANSFER_SLIDE      | 4         | 15 (horizontal_slide) | WRONG value + wrong name |
| TRANSFER_STATIC     | 6         | 7              | WRONG |
| TRANSFER_LANDSCAPE  | 9         | 9              | Correct |

### Correct Alephone enum (all 28 modes)

| Value | Alephone name                       | Category        |
|-------|-------------------------------------|-----------------|
| 0     | `_xfer_normal`                      | Basic           |
| 1     | `_xfer_fade_out_to_black`           | Fade            |
| 2     | `_xfer_invisibility`                | Visibility      |
| 3     | `_xfer_subtle_invisibility`         | Visibility      |
| 4     | `_xfer_pulsate`                     | UV animation    |
| 5     | `_xfer_wobble`                      | UV animation    |
| 6     | `_xfer_fast_wobble`                 | UV animation    |
| 7     | `_xfer_static`                      | Noise           |
| 8     | `_xfer_50percent_static`            | Noise           |
| 9     | `_xfer_landscape`                   | Projection      |
| 10    | `_xfer_smear`                       | Solid fill      |
| 11    | `_xfer_fade_out_static`             | Noise+fade      |
| 12    | `_xfer_pulsating_static`            | Noise+anim      |
| 13    | `_xfer_fold_in`                     | Teleport FX     |
| 14    | `_xfer_fold_out`                    | Teleport FX     |
| 15    | `_xfer_horizontal_slide`            | Slide           |
| 16    | `_xfer_fast_horizontal_slide`       | Slide           |
| 17    | `_xfer_vertical_slide`              | Slide           |
| 18    | `_xfer_fast_vertical_slide`         | Slide           |
| 19    | `_xfer_wander`                      | Drift           |
| 20    | `_xfer_fast_wander`                 | Drift           |
| 21    | `_xfer_big_landscape`               | Projection      |
| 22    | `_xfer_reverse_horizontal_slide`    | Slide           |
| 23    | `_xfer_reverse_fast_horizontal_slide` | Slide         |
| 24    | `_xfer_reverse_vertical_slide`      | Slide           |
| 25    | `_xfer_reverse_fast_vertical_slide` | Slide           |
| 26    | `_xfer_2x`                          | Scaling         |
| 27    | `_xfer_4x`                          | Scaling         |

## Goals

1. Correct all 6 existing transfer mode constants to match Alephone's `map.h` values.
2. Add all 22 missing transfer mode constants.
3. Rename `TRANSFER_SLIDE` to `TRANSFER_HORIZONTAL_SLIDE` (value 15).
4. Implement shader branches for modes that have straightforward UV-based effects.
5. Document unimplemented modes (fade, invisibility, fold, smear) with fallback to normal.

## Non-Goals

- Glow / self-luminous two-pass rendering (separate change).
- Per-surface transfer mode routing (floor vs. ceiling vs. wall) -- separate change.
- Shape-level transfer modes from `LowLevelShape` -- separate change.
- Fold-in / fold-out framebuffer distortion effects -- deferred, requires post-processing pipeline.
- Fade/invisibility effects -- deferred, requires alpha/blending pipeline changes.

## Decisions

### D1: Phased approach -- fix values first, then add shader branches

**Phase A (correctness):** Fix the 6 existing constant values and rename `TRANSFER_SLIDE`. Add all 22 missing constants. Update the shader switch statement to use corrected values. This alone fixes every surface that uses pulsate, wobble, slide, or static.

**Phase B (new effects):** Add shader branches for modes that are pure UV transformations:
- `fast_wobble` (6) -- same as wobble with 2x frequency
- `50percent_static` (8) -- noise on 50% of pixels, texture on others
- `horizontal_slide` (15) -- existing slide logic, now at correct value
- `fast_horizontal_slide` (16) -- 2x speed horizontal slide
- `vertical_slide` (17) -- slide in V direction
- `fast_vertical_slide` (18) -- 2x speed vertical slide
- `wander` (19) -- pseudo-random UV drift using layered sine waves
- `fast_wander` (20) -- 2x speed wander
- `big_landscape` (21) -- landscape with wider FOV scaling
- `reverse_horizontal_slide` (22) -- negate horizontal slide direction
- `reverse_fast_horizontal_slide` (23) -- negate fast horizontal slide
- `reverse_vertical_slide` (24) -- negate vertical slide direction
- `reverse_fast_vertical_slide` (25) -- negate fast vertical slide
- `2x` (26) -- multiply UV by 2.0
- `4x` (27) -- multiply UV by 4.0
- `pulsating_static` (12) -- static noise with pulsating intensity

**Phase C (fallback stubs):** Modes that require features not yet in the pipeline fall back to normal rendering with a TODO comment:
- `fade_out_to_black` (1) -- needs alpha/color modulation over time
- `invisibility` (2) -- needs alpha blending / discard
- `subtle_invisibility` (3) -- needs partial transparency
- `smear` (10) -- needs solid color fill from first pixel
- `fade_out_static` (11) -- needs fade + static combination
- `fold_in` (13) -- needs post-process framebuffer distortion
- `fold_out` (14) -- needs post-process framebuffer distortion

### D2: Constant naming convention

Use `TRANSFER_` prefix (matching existing convention) with Alephone-style names:
- `TRANSFER_HORIZONTAL_SLIDE` (not `TRANSFER_SLIDE`)
- `TRANSFER_FAST_WOBBLE` (not `TRANSFER_WOBBLE_FAST`)
- `TRANSFER_50PERCENT_STATIC` (following Alephone's `_xfer_50percent_static`)
- `TRANSFER_2X`, `TRANSFER_4X`

### D3: Shader constants must mirror Rust constants

All three shader files (`marathon-viewer`, `marathon-game`, `marathon-web`) define their own WGSL constants. These must be updated in lockstep with `transfer.rs`. Each shader's `apply_transfer_mode()` function gets the same expanded switch statement.

### D4: Fast variants use speed multipliers, not separate logic

Fast wobble = wobble with 2x frequency. Fast slides = slides with 2x speed. Fast wander = wander with 2x speed. This keeps the shader DRY and ensures visual consistency.

### D5: Reverse slides negate direction

Reverse horizontal slide uses `-time * speed` instead of `+time * speed`. Same pattern for vertical. This matches Alephone's behavior.

### D6: Wander uses deterministic pseudo-random drift

Wander uses layered sine waves at incommensurate frequencies to approximate random drift. This is deterministic (same result for same time value) and avoids needing a random number generator in the shader.

```
wx = sin(time * 0.3) * 0.1 + cos(time * 0.17) * 0.05
wy = cos(time * 0.25) * 0.1 + sin(time * 0.13) * 0.05
```

### D7: Big landscape uses wider FOV scaling

`big_landscape` (21) uses the same view-angle projection as `landscape` (9) but with a wider effective FOV by scaling the U coordinate less aggressively. In Alephone, this is controlled by `LandscapeRescale`.

### D8: Texture scaling modes multiply UVs directly

`2x` mode: `uv * 2.0` -- texture repeats twice across the surface.
`4x` mode: `uv * 4.0` -- texture repeats four times across the surface.

## Risks and Trade-offs

### R1: Shader branch count increases from 5 to ~16

Expanding the switch statement increases shader complexity. GPU switch statements on u32 compile to jump tables and should not cause performance issues. If profiling later reveals a bottleneck, we can split into specialized shader variants.

### R2: Unimplemented modes fall back silently

Modes 1-3 (fade/invisibility), 10 (smear), 11 (fade_out_static), 13-14 (fold) render as normal. Players familiar with Marathon will notice missing effects on specific surfaces. This is acceptable as a phased approach -- each can be implemented in a follow-up change.

### R3: Three shader files must stay in sync

The three crates duplicate shader code. Any constant or branch added to one must be added to all three. This is a known tech debt; a shared shader module is a future improvement.

### R4: Wander approximation may differ from Alephone

Alephone's wander uses actual random walk state accumulated over frames. Our sine-wave approximation is visually similar but not frame-identical. This is acceptable for the Rust rebuild since we are not targeting frame-perfect parity with the original software renderer.
