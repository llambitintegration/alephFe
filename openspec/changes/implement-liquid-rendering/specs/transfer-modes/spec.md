## MODIFIED Requirements

### Requirement: Wobble transfer mode
The system SHALL render surfaces with wobble transfer mode by applying a periodic UV distortion using sinusoidal offsets in both U and V based on elapsed time and position. This creates a wavy, liquid-like distortion effect. For media surfaces, wobble SHALL be applied by default regardless of the media's declared transfer_mode, producing the characteristic ripple effect on all liquid surfaces.

#### Scenario: Wobbling surface
- **WHEN** a surface has wobble transfer mode
- **THEN** the texture appears to wave and distort with a smooth periodic animation

#### Scenario: Media surface default wobble
- **WHEN** a media surface has any declared transfer_mode (including normal)
- **THEN** the fragment shader SHALL apply wobble UV distortion to the media surface texture, overriding the declared mode

### Requirement: Media UV flow scroll
The system SHALL apply an additional linear UV offset to media surfaces based on the media's current direction and magnitude. The offset SHALL be computed as `uv_offset = vec2(media_current_dx, media_current_dy) * elapsed_time` and added to the UV coordinates after wobble distortion is applied. This simulates liquid surface flow. The `media_current_dx` and `media_current_dy` values SHALL be read from the per-polygon storage buffer.

#### Scenario: Water with eastward current
- **WHEN** a water media surface has media_current_dx=0.3 and media_current_dy=0.0
- **THEN** the texture SHALL scroll in the +U direction at a rate of 0.3 world units per second, in addition to wobble distortion

#### Scenario: Sewage with diagonal current
- **WHEN** a sewage media surface has media_current_dx=0.2 and media_current_dy=0.15
- **THEN** the texture SHALL scroll diagonally at the combined rate, in addition to wobble distortion

#### Scenario: Media with no current
- **WHEN** a media surface has media_current_dx=0.0 and media_current_dy=0.0
- **THEN** only wobble distortion SHALL be applied, with no additional UV scrolling

### Requirement: Transfer mode uniform data
The system SHALL pass transfer mode parameters to the fragment shader via the per-polygon storage buffer. Each surface's entry SHALL include: transfer mode ID (u32), elapsed time (f32), and transfer mode-specific parameters (texture offsets for scroll direction/speed). For media surfaces, the entry SHALL additionally include media_current_dx (f32) and media_current_dy (f32) for flow scrolling. The fragment shader SHALL branch on the media vertex flag (bit 31) to apply wobble + flow scroll for media, or on the transfer mode ID for non-media surfaces.

#### Scenario: Shader receives media flow data
- **WHEN** the fragment shader processes a media surface vertex
- **THEN** it reads media_current_dx and media_current_dy from the storage buffer and applies flow UV offset in addition to wobble
