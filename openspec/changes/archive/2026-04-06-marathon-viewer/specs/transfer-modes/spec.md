## ADDED Requirements

### Requirement: Normal transfer mode
The system SHALL render surfaces with transfer mode 0 (normal) by sampling the texture at the computed UV coordinates without modification. This is the default rendering path.

#### Scenario: Standard textured surface
- **WHEN** a surface has transfer_mode=0
- **THEN** the texture is sampled at the vertex UV coordinates with no modification

### Requirement: Landscape transfer mode
The system SHALL render surfaces with landscape transfer mode by mapping the texture to the view angle rather than world geometry. The U coordinate SHALL be derived from the horizontal view angle (yaw relative to surface), and the V coordinate SHALL be derived from the vertical view angle (pitch). This produces a sky/horizon effect where the texture appears fixed to the viewing direction.

#### Scenario: Sky rendering
- **WHEN** a ceiling surface has landscape transfer mode
- **THEN** the texture scrolls with camera rotation but does not move with camera translation, creating a sky dome effect

### Requirement: Slide transfer mode
The system SHALL render surfaces with slide transfer mode by offsetting the UV coordinates by a time-varying amount. The offset SHALL increase linearly with elapsed time, causing the texture to scroll across the surface. The scroll direction and speed SHALL be derived from the texture offset values.

#### Scenario: Scrolling texture
- **WHEN** a wall surface has slide transfer mode
- **THEN** the texture continuously scrolls across the surface over time

### Requirement: Pulsate transfer mode
The system SHALL render surfaces with pulsate transfer mode by periodically scaling the UV coordinates toward/away from the surface center using a sinusoidal function of elapsed time. This creates a pulsing/breathing effect on the texture.

#### Scenario: Pulsating surface
- **WHEN** a surface has pulsate transfer mode
- **THEN** the texture periodically zooms in and out with a smooth sinusoidal cycle

### Requirement: Wobble transfer mode
The system SHALL render surfaces with wobble transfer mode by applying a periodic UV distortion using sinusoidal offsets in both U and V based on elapsed time and position. This creates a wavy, liquid-like distortion effect.

#### Scenario: Wobbling surface
- **WHEN** a surface has wobble transfer mode
- **THEN** the texture appears to wave and distort with a smooth periodic animation

### Requirement: Static transfer mode
The system SHALL render surfaces with static transfer mode by replacing the texture with randomized noise. The noise pattern SHALL change each frame to produce a TV-static visual effect.

#### Scenario: Static noise surface
- **WHEN** a surface has static transfer mode
- **THEN** the surface displays rapidly changing random noise instead of a texture

### Requirement: Transfer mode uniform data
The system SHALL pass transfer mode parameters to the fragment shader via the per-polygon storage buffer. Each surface's entry SHALL include: transfer mode ID (u32), elapsed time (f32), and transfer mode-specific parameters (texture offsets for scroll direction/speed). The fragment shader SHALL branch on the transfer mode ID to apply the appropriate effect.

#### Scenario: Shader receives mode data
- **WHEN** the fragment shader processes a surface with slide transfer mode
- **THEN** it reads the transfer mode ID and elapsed time from the storage buffer and applies the sliding UV offset

### Requirement: Transfer mode constants
The system SHALL define transfer mode ID constants matching Marathon's values: normal=0, pulsate=1, wobble=2, slide=4, static=6, landscape=9. Unknown transfer mode IDs SHALL fall back to normal rendering.

#### Scenario: Unknown transfer mode
- **WHEN** a surface has an unrecognized transfer_mode value
- **THEN** the system renders it using normal (mode 0) behavior
