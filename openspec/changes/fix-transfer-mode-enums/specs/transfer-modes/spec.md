## MODIFIED Requirements

### Requirement: Transfer mode constants
The system SHALL define transfer mode ID constants matching Alephone's `map.h` enum for all 28 modes:
- normal=0, fade_out_to_black=1, invisibility=2, subtle_invisibility=3
- pulsate=4, wobble=5, fast_wobble=6, static=7, 50percent_static=8
- landscape=9, smear=10, fade_out_static=11, pulsating_static=12
- fold_in=13, fold_out=14
- horizontal_slide=15, fast_horizontal_slide=16, vertical_slide=17, fast_vertical_slide=18
- wander=19, fast_wander=20, big_landscape=21
- reverse_horizontal_slide=22, reverse_fast_horizontal_slide=23, reverse_vertical_slide=24, reverse_fast_vertical_slide=25
- 2x=26, 4x=27

The Rust constants in `transfer.rs` and WGSL constants in all three shader files SHALL use identical values. Unknown transfer mode IDs SHALL fall back to normal rendering. The constant `TRANSFER_SLIDE` SHALL be renamed to `TRANSFER_HORIZONTAL_SLIDE`.

#### Scenario: Pulsate mode has correct value
- **WHEN** a surface has transfer_mode=4 read from a Marathon map file
- **THEN** the system matches it to TRANSFER_PULSATE and applies the pulsate UV animation

#### Scenario: Static mode has correct value
- **WHEN** a surface has transfer_mode=7 read from a Marathon map file
- **THEN** the system matches it to TRANSFER_STATIC and renders noise

#### Scenario: Horizontal slide has correct value
- **WHEN** a surface has transfer_mode=15 read from a Marathon map file
- **THEN** the system matches it to TRANSFER_HORIZONTAL_SLIDE and scrolls the texture horizontally

#### Scenario: Unknown transfer mode
- **WHEN** a surface has a transfer_mode value >= 28 or an unrecognized value
- **THEN** the system renders it using normal (mode 0) behavior

### Requirement: Slide transfer mode
The system SHALL render surfaces with horizontal slide transfer mode (15) by offsetting the U coordinate by a time-varying amount. The offset SHALL increase linearly with elapsed time, causing the texture to scroll horizontally. `TRANSFER_SLIDE` is renamed to `TRANSFER_HORIZONTAL_SLIDE` to match Alephone naming and distinguish from vertical slide.

#### Scenario: Scrolling texture
- **WHEN** a wall surface has transfer_mode=15 (horizontal_slide)
- **THEN** the texture continuously scrolls horizontally across the surface over time

## ADDED Requirements

### Requirement: Fast wobble transfer mode
The system SHALL render surfaces with fast wobble transfer mode (6) using the same sinusoidal UV distortion as wobble (5) but with doubled frequency parameters. This creates a faster, more agitated liquid-like distortion.

#### Scenario: Fast wobbling surface
- **WHEN** a surface has transfer_mode=6
- **THEN** the texture distorts with the same wobble pattern but at twice the animation speed

### Requirement: 50 percent static transfer mode
The system SHALL render surfaces with 50percent_static transfer mode (8) by blending the base texture with random noise. Approximately 50% of pixels per frame SHALL show noise while the remaining 50% show the normal texture. The noise pattern SHALL change each frame.

#### Scenario: Partial static surface
- **WHEN** a surface has transfer_mode=8
- **THEN** roughly half the pixels show noise and half show the base texture, creating a flickering partial-static effect

### Requirement: Vertical slide transfer modes
The system SHALL render surfaces with vertical slide transfer mode (17) by offsetting the V coordinate linearly with elapsed time, causing the texture to scroll vertically. Fast vertical slide (18) SHALL use doubled scroll speed.

#### Scenario: Vertically scrolling texture
- **WHEN** a surface has transfer_mode=17
- **THEN** the texture continuously scrolls vertically across the surface

#### Scenario: Fast vertical scroll
- **WHEN** a surface has transfer_mode=18
- **THEN** the texture scrolls vertically at twice the normal vertical slide speed

### Requirement: Fast horizontal slide transfer mode
The system SHALL render surfaces with fast horizontal slide transfer mode (16) by applying the same horizontal UV offset as horizontal slide (15) but with doubled scroll speed.

#### Scenario: Fast horizontal scroll
- **WHEN** a surface has transfer_mode=16
- **THEN** the texture scrolls horizontally at twice the normal horizontal slide speed

### Requirement: Reverse slide transfer modes
The system SHALL render surfaces with reverse slide transfer modes (22-25) by applying the same UV offset as their forward counterparts but with negated direction. Reverse horizontal slide (22) negates horizontal slide (15). Reverse fast horizontal slide (23) negates fast horizontal slide (16). Reverse vertical slide (24) negates vertical slide (17). Reverse fast vertical slide (25) negates fast vertical slide (18).

#### Scenario: Reverse horizontal scroll
- **WHEN** a surface has transfer_mode=22
- **THEN** the texture scrolls horizontally in the opposite direction from mode 15

#### Scenario: Reverse fast vertical scroll
- **WHEN** a surface has transfer_mode=25
- **THEN** the texture scrolls vertically in the opposite direction from mode 18, at doubled speed

### Requirement: Wander transfer mode
The system SHALL render surfaces with wander transfer mode (19) by applying a pseudo-random UV drift using layered sine waves at incommensurate frequencies. Fast wander (20) SHALL use doubled drift speed. The drift SHALL be deterministic (same time produces same offset).

#### Scenario: Drifting texture
- **WHEN** a surface has transfer_mode=19
- **THEN** the texture drifts slowly in a pseudo-random pattern

#### Scenario: Fast drifting texture
- **WHEN** a surface has transfer_mode=20
- **THEN** the texture drifts at twice the speed of normal wander

### Requirement: Big landscape transfer mode
The system SHALL render surfaces with big landscape transfer mode (21) using the same view-angle-based projection as landscape (9) but with a wider effective field of view. The U coordinate SHALL be scaled less aggressively to cover a larger angular range.

#### Scenario: Wide landscape rendering
- **WHEN** a surface has transfer_mode=21
- **THEN** the texture maps to the view angle like landscape mode but covers a wider horizontal field of view

### Requirement: Texture scaling transfer modes
The system SHALL render surfaces with 2x transfer mode (26) by multiplying UV coordinates by 2.0, causing the texture to tile twice across the surface. The 4x transfer mode (27) SHALL multiply UV coordinates by 4.0.

#### Scenario: 2x texture tiling
- **WHEN** a surface has transfer_mode=26
- **THEN** the texture repeats twice in each direction across the surface

#### Scenario: 4x texture tiling
- **WHEN** a surface has transfer_mode=27
- **THEN** the texture repeats four times in each direction across the surface

### Requirement: Pulsating static transfer mode
The system SHALL render surfaces with pulsating static transfer mode (12) by generating noise that varies in intensity over time using a sinusoidal modulation. The noise pattern SHALL change each frame.

#### Scenario: Pulsating noise surface
- **WHEN** a surface has transfer_mode=12
- **THEN** the surface displays noise whose intensity oscillates smoothly over time

### Requirement: Unimplemented mode fallback
The system SHALL render surfaces with fade_out_to_black (1), invisibility (2), subtle_invisibility (3), smear (10), fade_out_static (11), fold_in (13), and fold_out (14) transfer modes using normal rendering as a fallback. These modes require pipeline features (alpha blending, post-processing) not yet available. Each fallback SHALL be documented with a TODO comment in the shader.

#### Scenario: Fade mode falls back to normal
- **WHEN** a surface has transfer_mode=1 (fade_out_to_black)
- **THEN** the system renders it as a normal textured surface (fallback)

#### Scenario: Fold mode falls back to normal
- **WHEN** a surface has transfer_mode=13 (fold_in)
- **THEN** the system renders it as a normal textured surface (fallback)
