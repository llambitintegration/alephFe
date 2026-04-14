## ADDED Requirements

### Requirement: First-person weapon sprite rendering
The web renderer SHALL display the player's current weapon as a sprite at the bottom center of the viewport. The weapon sprite SHALL be loaded from the shapes file using the weapon's collection and shape indices. The sprite SHALL be rendered in a screen-space overlay pass after level geometry, respecting alpha transparency.

#### Scenario: Weapon displays at viewport bottom
- **WHEN** the player is holding the fist weapon (default starting weapon)
- **THEN** the renderer SHALL display the fist sprite centered at the bottom of the viewport

#### Scenario: Weapon sprite updates with animation frame
- **WHEN** the weapon is firing and the sim advances the weapon animation frame
- **THEN** the rendered weapon sprite SHALL change to match the current animation frame

### Requirement: Weapon sprite uses correct texture collection
The weapon sprite SHALL be loaded from the correct shapes collection for the current weapon. The sprite SHALL use the same texture conversion pipeline (CLUT lookup, RGBA conversion) as entity sprites.

#### Scenario: Weapon texture loads from shapes file
- **WHEN** the player equips a weapon with collection index N
- **THEN** the renderer SHALL load and display sprites from shapes collection N
