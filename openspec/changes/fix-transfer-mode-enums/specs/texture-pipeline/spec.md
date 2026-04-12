## MODIFIED Requirements

### Requirement: Transfer mode shader dispatch
The system SHALL dispatch transfer mode effects in the fragment shader's `apply_transfer_mode()` function using a switch statement on the transfer mode ID (u32). The switch SHALL handle at minimum 16 branches covering: normal (0), pulsate (4), wobble (5), fast_wobble (6), static (7), 50percent_static (8), landscape (9), pulsating_static (12), horizontal_slide (15), fast_horizontal_slide (16), vertical_slide (17), fast_vertical_slide (18), wander (19), fast_wander (20), big_landscape (21), reverse_horizontal_slide (22), reverse_fast_horizontal_slide (23), reverse_vertical_slide (24), reverse_fast_vertical_slide (25), 2x (26), 4x (27). Unrecognized mode IDs SHALL fall through to normal rendering.

#### Scenario: Expanded shader handles new slide modes
- **WHEN** a surface with transfer_mode=17 (vertical_slide) is rendered
- **THEN** the fragment shader's switch statement matches the vertical_slide branch and offsets the V coordinate by elapsed_time * speed

#### Scenario: Shader handles texture scaling
- **WHEN** a surface with transfer_mode=26 (2x) is rendered
- **THEN** the fragment shader's switch statement matches the 2x branch and multiplies UV by 2.0

#### Scenario: Static mode handled before texture sampling
- **WHEN** a surface with transfer_mode=7 (static) is rendered
- **THEN** the fragment shader returns noise color directly without sampling the texture, using the corrected constant value 7 (not the old incorrect value 6)

#### Scenario: All three shader files in sync
- **WHEN** transfer mode constants or branches are updated
- **THEN** marathon-viewer/shader.wgsl, marathon-game/shader.wgsl, and marathon-web/shader.wgsl all contain identical constant definitions and apply_transfer_mode logic
