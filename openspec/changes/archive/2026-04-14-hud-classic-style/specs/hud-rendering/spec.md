## MODIFIED Requirements

### Requirement: Composite HUD as 2D overlay on 3D scene
The system SHALL render the HUD as a DOM overlay panel positioned below the 3D canvas. The 3D canvas SHALL be resized so its bottom edge meets the top edge of the HUD panel (no overlap). The HUD panel SHALL be opaque with a dark metallic background (#1a1a1a or similar). The HUD panel height SHALL be approximately 128px. The HUD render layer SHALL have a z-index above the canvas.

#### Scenario: HUD over gameplay
- **WHEN** a frame is rendered during the Playing state
- **THEN** the 3D canvas SHALL occupy the screen area above the HUD, and the HUD panel SHALL occupy the bottom 128px

#### Scenario: 3D viewport does not extend behind HUD
- **WHEN** the HUD is visible
- **THEN** the 3D canvas height SHALL be `100vh - 128px`, so no 3D content is rendered behind the opaque HUD

#### Scenario: HUD background is opaque
- **WHEN** the HUD is visible
- **THEN** the HUD background SHALL be fully opaque (no transparency showing the page behind it)

### Requirement: HUD uses three-column layout
The system SHALL arrange HUD elements in a three-column layout using CSS Grid or equivalent. The left column SHALL contain the motion sensor. The center column SHALL contain health, shield, and oxygen vitals. The right column SHALL contain weapon information. Columns SHALL be visually balanced with subtle dividers or spacing.

#### Scenario: Three columns visible
- **WHEN** the HUD is rendered
- **THEN** three distinct visual columns SHALL be visible: motion sensor on left, vitals in center, weapon info on right

#### Scenario: Column proportions
- **WHEN** the HUD is rendered at any supported width
- **THEN** the left and right columns SHALL have equal width, and the center column SHALL be wider to accommodate vitals display

### Requirement: Render health and shield bars
The system SHALL render the player's health and shield values as horizontal bars in the center column of the HUD. Bars SHALL have a segmented visual appearance (using CSS repeating gradients or similar). Each bar SHALL display its numeric value alongside the bar. The health bar SHALL use green coloring with tier-based color changes. The shield bar SHALL use blue coloring.

#### Scenario: Full health display
- **WHEN** the player has 150 health out of 150 maximum
- **THEN** the health bar SHALL render at full width with green coloring and "150" displayed

#### Scenario: Partial shield with tier coloring
- **WHEN** the player has double shield strength (shield value in the 2x range)
- **THEN** the shield bar SHALL render with the double-shield color up to the current value

#### Scenario: Zero health
- **WHEN** the player's health is 0
- **THEN** the health bar SHALL render as empty (zero width) and display "0"

#### Scenario: Low health warning
- **WHEN** the player's health is below 33% of maximum
- **THEN** the health bar SHALL render with a red/warning color

### Requirement: Render oxygen meter
The system SHALL render the player's oxygen level as a bar in the center column. The oxygen display SHALL be hidden when the player is in a normal atmosphere (oxygen at maximum). The oxygen bar SHALL use cyan coloring.

#### Scenario: Oxygen depleting underwater
- **WHEN** the player is submerged and oxygen is at 50% of maximum
- **THEN** the oxygen meter SHALL be visible and render at half width

#### Scenario: Normal atmosphere
- **WHEN** the player is in a polygon with normal atmosphere
- **THEN** the oxygen meter SHALL be hidden

#### Scenario: Oxygen critically low
- **WHEN** the player's oxygen is below 25% of maximum
- **THEN** the oxygen meter SHALL render with a warning visual indicator (flashing or color change)

### Requirement: HUD uses retro visual styling
The system SHALL style HUD elements with a retro/tech aesthetic matching Marathon 2. Text SHALL use a monospace font. The background SHALL be dark (#1a1a1a) with subtle border or bevel effects. Bar segments and labels SHALL evoke an 8-bit/16-bit era control panel look.

#### Scenario: Font styling
- **WHEN** the HUD is rendered
- **THEN** all HUD text (labels, values, weapon name) SHALL use a monospace or pixel-style font

#### Scenario: Dark panel aesthetic
- **WHEN** the HUD is rendered
- **THEN** the HUD background SHALL be a dark opaque color with subtle border effects suggesting a tech panel
