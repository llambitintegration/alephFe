## MODIFIED Requirements

### Requirement: Landscape transfer mode
The system SHALL render surfaces with landscape transfer mode by mapping the texture to the view angle rather than world geometry. The U coordinate SHALL be derived from the horizontal view angle (yaw relative to surface), and the V coordinate SHALL be derived from the vertical view angle (pitch). This produces a sky/horizon effect where the texture appears fixed to the viewing direction. The fragment shader SHALL sample landscape textures using the linear-filtering sampler to produce smooth sky rendering, while all other transfer modes SHALL continue using the nearest-neighbor sampler.

#### Scenario: Sky rendering
- **WHEN** a ceiling surface has landscape transfer mode
- **THEN** the texture scrolls with camera rotation but does not move with camera translation, creating a sky dome effect

#### Scenario: Landscape uses linear filtering
- **WHEN** a surface has transfer_mode == TRANSFER_LANDSCAPE (9)
- **THEN** the fragment shader samples the texture using the linear sampler, producing smooth bilinear-filtered output instead of blocky nearest-neighbor pixels

#### Scenario: Non-landscape modes use nearest filtering
- **WHEN** a surface has any transfer mode other than landscape (e.g., normal, slide, wobble)
- **THEN** the fragment shader samples the texture using the nearest-neighbor sampler, preserving the pixel-art aesthetic
