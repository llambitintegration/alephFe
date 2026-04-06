## ADDED Requirements

### Requirement: Render health and shield bars
The system SHALL render the player's health and shield values as horizontal bars on the HUD overlay. The health bar SHALL display the current health as a proportion of maximum health. The shield bar SHALL display the current shield value with color segments corresponding to shield strength tiers (single, double, triple). Bar positions SHALL be fixed relative to the screen bottom.

#### Scenario: Full health display
- **WHEN** the player has 150 health out of 150 maximum
- **THEN** the health bar SHALL render at full width

#### Scenario: Partial shield with tier coloring
- **WHEN** the player has double shield strength (shield value in the 2x range)
- **THEN** the shield bar SHALL render with the double-shield color up to the current value

#### Scenario: Zero health
- **WHEN** the player's health is 0
- **THEN** the health bar SHALL render as empty (zero width)

### Requirement: Render oxygen meter
The system SHALL render the player's oxygen level as a bar or indicator when the player is in an environment that consumes oxygen (vacuum or submerged). The oxygen display SHALL be hidden when the player is in a normal atmosphere.

#### Scenario: Oxygen depleting underwater
- **WHEN** the player is submerged and oxygen is at 50% of maximum
- **THEN** the oxygen meter SHALL be visible and render at half width

#### Scenario: Normal atmosphere
- **WHEN** the player is in a polygon with normal atmosphere
- **THEN** the oxygen meter SHALL be hidden

#### Scenario: Oxygen critically low
- **WHEN** the player's oxygen is below 25% of maximum
- **THEN** the oxygen meter SHALL render with a warning visual indicator (flashing or color change)

### Requirement: Render weapon and ammunition display
The system SHALL render the currently equipped weapon's icon and ammunition count on the HUD. For weapons with both primary and secondary triggers, the system SHALL display ammunition counts for both triggers. The weapon display SHALL use sprite frames from the ShapesFile interface collection.

#### Scenario: Weapon with dual triggers
- **WHEN** the player has a fusion pistol equipped with 42 primary rounds and 3 secondary charges
- **THEN** the HUD SHALL display the fusion pistol icon, "42" for primary ammo, and "3" for secondary ammo

#### Scenario: Weapon with infinite ammo
- **WHEN** the player has fists equipped (no ammunition)
- **THEN** the HUD SHALL display the fists icon with no ammunition counter

#### Scenario: Weapon switch
- **WHEN** the player switches from the assault rifle to the shotgun
- **THEN** the weapon display SHALL update to show the shotgun icon and its ammunition counts

### Requirement: Render motion sensor (radar)
The system SHALL render a motion sensor display showing nearby entities as dots relative to the player's position and facing direction. The motion sensor SHALL be a circular display. Entity dots SHALL be positioned based on their relative angle and distance from the player. Dot color SHALL distinguish entity types: allies (green), enemies (red), and items (yellow). Entities beyond the sensor's maximum range SHALL not appear.

#### Scenario: Enemy in range ahead
- **WHEN** an enemy is 3 world units ahead of the player within sensor range
- **THEN** a red dot SHALL appear near the top center of the motion sensor

#### Scenario: Ally to the left
- **WHEN** an ally is to the left of the player within sensor range
- **THEN** a green dot SHALL appear on the left side of the motion sensor

#### Scenario: Entity beyond range
- **WHEN** an enemy is 50 world units away and the sensor maximum range is 30 units
- **THEN** no dot SHALL appear for that enemy on the motion sensor

#### Scenario: Motion sensor updates with player rotation
- **WHEN** the player rotates 90 degrees clockwise
- **THEN** all entity dots on the motion sensor SHALL rotate 90 degrees counterclockwise relative to their previous positions

### Requirement: Render inventory panel
The system SHALL render collected inventory items (keycards, powerups, ammunition pickups) in a panel on the HUD. Each inventory item SHALL display its icon from the ShapesFile and a count if the player holds multiples.

#### Scenario: Single keycard
- **WHEN** the player has collected one uplink chip
- **THEN** the inventory panel SHALL display the uplink chip icon with count 1

#### Scenario: Multiple of same item
- **WHEN** the player has collected 3 alien energy cells
- **THEN** the inventory panel SHALL display the energy cell icon with count "3"

#### Scenario: Empty inventory
- **WHEN** the player has no inventory items
- **THEN** the inventory panel SHALL be empty or hidden

### Requirement: HUD layout adapts to display resolution
The system SHALL scale HUD element positions and sizes proportionally to the display resolution. The system SHALL maintain Marathon's original HUD aspect ratio and relative element positioning. HUD elements SHALL remain legible at all supported resolutions.

#### Scenario: Standard resolution
- **WHEN** the display resolution is 640x480
- **THEN** HUD elements SHALL render at their original pixel sizes matching Marathon's original layout

#### Scenario: High resolution
- **WHEN** the display resolution is 1920x1080
- **THEN** HUD elements SHALL scale proportionally so that relative positions and proportions match the original layout

### Requirement: HUD reads live state from marathon-sim
The system SHALL read player state (health, shield, oxygen, equipped weapon, ammunition, inventory, position, facing) from marathon-sim's exported game state each frame. The HUD SHALL reflect the most recent simulation state without delay.

#### Scenario: Damage taken updates health bar
- **WHEN** the player takes 20 damage and health decreases from 100 to 80
- **THEN** the health bar SHALL render at 80/150 on the next frame

#### Scenario: Ammunition consumed updates weapon display
- **WHEN** the player fires one primary round and ammo decreases from 42 to 41
- **THEN** the weapon ammo display SHALL show "41" on the next frame

### Requirement: Composite HUD as 2D overlay on 3D scene
The system SHALL render the HUD as a wgpu render pass that writes to the same framebuffer as the 3D scene, composited on top. The HUD render pass SHALL execute after the 3D scene render pass completes. HUD elements SHALL support alpha transparency.

#### Scenario: HUD over gameplay
- **WHEN** a frame is rendered during the Playing state
- **THEN** the 3D scene SHALL render first, followed by the HUD overlay pass which composites HUD elements on top

#### Scenario: Transparent HUD regions
- **WHEN** a HUD element has transparent pixels in its source sprite
- **THEN** the 3D scene SHALL be visible through those transparent regions
