## ADDED Requirements

### Requirement: Automap toggle
The web build SHALL toggle the automap display when the player presses the Tab key. When visible, the automap SHALL render as a 2D overlay on top of the 3D viewport.

#### Scenario: Tab toggles automap visibility
- **WHEN** the automap is hidden and the player presses Tab
- **THEN** the automap overlay SHALL become visible

#### Scenario: Tab hides automap when visible
- **WHEN** the automap is visible and the player presses Tab
- **THEN** the automap overlay SHALL be hidden

### Requirement: Automap renders polygon edges
The automap SHALL render the edges of all polygons in the current level as colored lines on a 2D canvas. Walls SHALL be drawn as solid lines. The map SHALL be centered on the player's current position.

#### Scenario: Automap shows level layout
- **WHEN** the automap is visible
- **THEN** all polygon edges of the current level SHALL be drawn as lines on the 2D overlay

### Requirement: Automap shows player position
The automap SHALL display a marker (arrow or dot) at the player's current position, oriented in the player's facing direction.

#### Scenario: Player marker tracks position
- **WHEN** the player moves and the automap is visible
- **THEN** the player marker position SHALL update to reflect the player's current map coordinates
