## ADDED Requirements

### Requirement: Fader state manager
The system SHALL maintain a fader state manager that tracks zero or more active full-screen effects. Each active fader SHALL have a color (RGBA), a blend mode (one of: tint, randomize, negate, dodge, burn, soft_tint), a current intensity (0.0 to 1.0), a remaining duration in ticks, and a total duration in ticks. The manager SHALL tick each frame, decaying intensity linearly (intensity = initial_intensity * remaining / total) and removing faders whose remaining duration reaches zero. Multiple faders SHALL be active simultaneously.

#### Scenario: Single fader lifecycle
- **WHEN** a damage fader is triggered with intensity 0.8 and duration 10 ticks
- **THEN** the fader SHALL start at intensity 0.8, decay linearly each tick, and be removed after 10 ticks

#### Scenario: Multiple simultaneous faders
- **WHEN** the player takes damage while invincibility is active
- **THEN** both the red damage tint fader and the gold invincibility glow fader SHALL be active and rendered in sequence

#### Scenario: Fader expiration
- **WHEN** a fader's remaining duration reaches 0
- **THEN** the fader SHALL be removed from the active list and no longer rendered

#### Scenario: Sustained fader refresh
- **WHEN** a sustained effect (invincibility, low oxygen) is active across multiple ticks
- **THEN** the fader manager SHALL maintain the fader with refreshed duration each tick rather than spawning duplicate faders

### Requirement: Fader post-process render pass
The system SHALL render active faders as a post-process pass using a fullscreen triangle (3 vertices computed from vertex_index, no vertex buffer). The pass SHALL sample the intermediate scene texture (containing the rendered 3D scene and sprites) and apply the fader's blend mode in the fragment shader. The output SHALL write to the swapchain surface. When no faders are active, the intermediate texture SHALL be copied to the swapchain without the fader shader.

#### Scenario: Fader pass renders after sprites
- **WHEN** a frame is rendered with one active fader
- **THEN** the 3D scene and sprites SHALL render to an intermediate texture, and the fader pass SHALL sample that texture and output to the swapchain

#### Scenario: No active faders
- **WHEN** no faders are active during a frame
- **THEN** the intermediate texture SHALL be blitted directly to the swapchain without invoking the fader shader

#### Scenario: Multiple faders in one frame
- **WHEN** two faders are active (damage tint and shield dodge)
- **THEN** the fader pass SHALL execute two draw calls, each with the respective fader's uniform data, compositing sequentially onto the swapchain

### Requirement: Tint blend mode
The tint blend mode (index 0) SHALL compute the output color as `mix(scene, scene * fader_color, intensity)`, pushing scene colors toward the fader color proportional to intensity. This is used for damage flashes and environmental tints.

#### Scenario: Damage flash tint
- **WHEN** a tint fader is active with color (1.0, 0.0, 0.0, 1.0) and intensity 0.5
- **THEN** each pixel SHALL be blended 50% toward a red-multiplied version of itself, producing a visible red overlay

#### Scenario: Zero intensity tint
- **WHEN** a tint fader has intensity 0.0
- **THEN** the output SHALL equal the unmodified scene color

### Requirement: Randomize blend mode
The randomize blend mode (index 1) SHALL compute the output color as `mix(scene, scene * fader_color, intensity * noise)`, where noise is a per-pixel pseudo-random value derived from the pixel coordinates and a time uniform. This produces a static/interference visual effect used for teleportation.

#### Scenario: Teleport static effect
- **WHEN** a randomize fader is active with white color and intensity 0.8
- **THEN** each pixel SHALL show a noise-modulated tint, with some pixels heavily tinted and others barely affected, producing a visual static pattern

#### Scenario: Randomize animation
- **WHEN** the randomize fader is active across multiple frames
- **THEN** the noise pattern SHALL change each frame (driven by the time uniform), producing animated static

### Requirement: Negate blend mode
The negate blend mode (index 2) SHALL compute the output color as `mix(scene, 1.0 - scene, intensity)`, inverting scene colors proportional to intensity. This produces a photographic negative effect.

#### Scenario: Full negate
- **WHEN** a negate fader is active with intensity 1.0
- **THEN** each pixel's RGB values SHALL be fully inverted (dark becomes light, red becomes cyan)

#### Scenario: Partial negate
- **WHEN** a negate fader is active with intensity 0.3
- **THEN** each pixel SHALL be 30% blended toward its inverted color, producing a subtle desaturation/inversion effect

### Requirement: Dodge blend mode
The dodge blend mode (index 3) SHALL compute the output color as `scene + fader_color * intensity`, additively brightening the scene. Values SHALL be clamped to 1.0. This is used for shield recharge flashes.

#### Scenario: Shield recharge flash
- **WHEN** a dodge fader is active with color (0.5, 0.5, 1.0, 1.0) and intensity 0.6
- **THEN** each pixel SHALL have (0.3, 0.3, 0.6) added to its color, brightening the scene with a blue-white cast

#### Scenario: Dodge clamp
- **WHEN** a dodge fader adds brightness to an already-bright pixel
- **THEN** the output SHALL be clamped to 1.0, not wrap around or produce artifacts

### Requirement: Burn blend mode
The burn blend mode (index 4) SHALL compute the output color as `scene - fader_color * intensity`, subtractively darkening the scene. Values SHALL be clamped to 0.0. This is used for lava damage tints.

#### Scenario: Lava burn effect
- **WHEN** a burn fader is active with color (1.0, 0.5, 0.0, 1.0) and intensity 0.4
- **THEN** each pixel SHALL have (0.4, 0.2, 0.0) subtracted from its color, producing a darkened warm-shifted overlay

#### Scenario: Burn clamp
- **WHEN** a burn fader subtracts brightness from an already-dark pixel
- **THEN** the output SHALL be clamped to 0.0, not wrap negative

### Requirement: Soft tint blend mode
The soft tint blend mode (index 5) SHALL compute the output color as `mix(scene, scene * fader_color, intensity * 0.5)`, applying a gentler version of the tint mode at half the effective intensity. This is used for sustained effects like invincibility glow and infravision overlay.

#### Scenario: Invincibility glow
- **WHEN** a soft tint fader is active with a gold-green color and intensity 0.6
- **THEN** the effective blend factor SHALL be 0.3 (0.6 * 0.5), producing a subtle sustained tint

#### Scenario: Infravision overlay
- **WHEN** an infravision soft tint fader is active with green color and intensity 1.0
- **THEN** the scene SHALL have a gentle green cast at 50% effective intensity

### Requirement: Damage flash triggered by player damage events
The system SHALL trigger a red tint fader when the player takes damage. The fader color SHALL be (1.0, 0.0, 0.0) with tint blend mode. The intensity SHALL be proportional to the damage taken relative to player maximum health (intensity = damage / max_health, clamped to 1.0). The duration SHALL be 8 ticks with linear decay.

#### Scenario: Minor damage flash
- **WHEN** the player takes 10 damage out of 150 max health
- **THEN** a red tint fader SHALL be triggered with intensity ~0.067, producing a brief subtle red flash

#### Scenario: Major damage flash
- **WHEN** the player takes 100 damage out of 150 max health
- **THEN** a red tint fader SHALL be triggered with intensity ~0.667, producing an intense red flash

#### Scenario: Damage while existing flash active
- **WHEN** the player takes damage while a damage flash is already active
- **THEN** a new damage fader SHALL be triggered (the previous one continues decaying independently), resulting in a stronger combined red overlay

### Requirement: Teleport effect triggered by level teleport events
The system SHALL trigger a white randomize fader when the player teleports. The fader color SHALL be (1.0, 1.0, 1.0). The duration SHALL be 15 ticks for the fade-out phase. The intensity SHALL start at 1.0.

#### Scenario: Intra-level teleport
- **WHEN** the player steps onto a teleport pad within the same level
- **THEN** a white randomize fader SHALL be triggered with intensity 1.0 and duration 15 ticks, producing a static-noise flash that fades over half a second

#### Scenario: Inter-level teleport
- **WHEN** the player teleports to a new level
- **THEN** a white randomize fader SHALL be triggered before the level transition, and a second randomize fader SHALL be triggered after the new level loads (fold-out and fold-in)

### Requirement: Invincibility glow while powerup active
The system SHALL maintain a gold-green soft tint fader while the invincibility powerup is active on the player. The fader color SHALL cycle between gold (1.0, 0.9, 0.2) and green (0.2, 1.0, 0.3) using a sinusoidal oscillation. The intensity SHALL pulse between 0.4 and 0.8.

#### Scenario: Invincibility pickup
- **WHEN** the player picks up an invincibility powerup
- **THEN** a soft tint fader SHALL begin with a cycling gold-green color and pulsing intensity

#### Scenario: Invincibility expiration
- **WHEN** the invincibility powerup timer expires
- **THEN** the invincibility fader SHALL be removed from the active fader list

### Requirement: Oxygen warning fader at low oxygen
The system SHALL trigger a blue-gray soft tint fader when the player's oxygen drops below 20% of maximum. The fader color SHALL be (0.3, 0.3, 0.7). The intensity SHALL increase as oxygen decreases: intensity = 1.0 - (oxygen / (max_oxygen * 0.2)). The fader SHALL persist as long as oxygen remains below the threshold.

#### Scenario: Oxygen at 15%
- **WHEN** the player's oxygen is at 15% of maximum
- **THEN** a blue-gray soft tint fader SHALL be active with intensity 0.25

#### Scenario: Oxygen at 0%
- **WHEN** the player's oxygen reaches 0
- **THEN** a blue-gray soft tint fader SHALL be active with intensity 1.0

#### Scenario: Oxygen recovered above threshold
- **WHEN** the player's oxygen rises above 20% of maximum
- **THEN** the oxygen warning fader SHALL be removed

### Requirement: Shield recharge flash
The system SHALL trigger a blue-white dodge fader when the player's shield recharges. The fader color SHALL be (0.5, 0.5, 1.0). The duration SHALL be 4 ticks. The intensity SHALL be 0.4.

#### Scenario: Shield recharge pulse
- **WHEN** the player stands on a shield recharge panel and shield increments
- **THEN** a blue-white dodge fader SHALL flash briefly, brightening the screen

### Requirement: Lava and goo damage tint while submerged
The system SHALL maintain a burn or tint fader while the player is submerged in damaging media. For lava, the color SHALL be (1.0, 0.4, 0.0) with burn blend mode. For goo, the color SHALL be (0.2, 0.8, 0.1) with tint blend mode. For Jjaro goo, the color SHALL be (0.4, 0.6, 1.0) with tint blend mode. Intensity SHALL be proportional to submersion depth.

#### Scenario: Lava submersion
- **WHEN** the player is submerged in lava
- **THEN** a burn fader with orange-red color SHALL darken and warm-shift the scene

#### Scenario: Goo submersion
- **WHEN** the player is submerged in goo
- **THEN** a tint fader with green color SHALL tint the scene green

#### Scenario: Exit from damaging media
- **WHEN** the player exits the damaging liquid
- **THEN** the media damage fader SHALL be removed

### Requirement: MML fader configuration
The system SHALL read the MML `faders` section at level load time and construct a fader configuration table. Each entry SHALL specify: fader type index, color (RGB floats), blend mode, duration in ticks, and base intensity. When a fader is triggered, the MML configuration SHALL provide default values that can be overridden by the trigger (e.g., damage intensity scales with damage amount). If no MML configuration is present, hardcoded defaults matching Marathon 2 behavior SHALL be used.

#### Scenario: MML overrides damage flash color
- **WHEN** the MML configuration sets fader index 0 to color (0.0, 0.0, 1.0) with tint mode
- **THEN** damage flashes SHALL appear blue instead of red

#### Scenario: No MML fader section
- **WHEN** the loaded scenario has no MML fader configuration
- **THEN** all faders SHALL use the hardcoded Marathon 2 defaults (red damage, white teleport, etc.)

#### Scenario: MML overrides fader blend mode
- **WHEN** the MML configuration changes the teleport fader from randomize to negate
- **THEN** teleportation SHALL produce a color-inversion effect instead of static noise

### Requirement: Fader uniform buffer
The system SHALL maintain a GPU uniform buffer for the active fader being rendered. The uniform layout SHALL contain: `color` (vec4<f32>), `intensity` (f32), `mode` (u32), `time` (f32), and padding to 32 bytes. The buffer SHALL be updated before each fader draw call with the current fader's parameters.

#### Scenario: Uniform updated per fader draw
- **WHEN** two faders are active in a frame
- **THEN** the uniform buffer SHALL be updated with the first fader's data, drawn, then updated with the second fader's data and drawn again

#### Scenario: Time uniform advances
- **WHEN** the randomize fader is active
- **THEN** the time uniform SHALL advance each frame to drive noise animation in the shader
