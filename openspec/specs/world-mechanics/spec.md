## ADDED Requirements

### Requirement: Platform movement
The system SHALL animate platforms (moving floors and ceilings) based on `StaticPlatformData` parsed from the map. Each platform SHALL have a resting position and an extended position. Platform movement speed SHALL follow the defined speed. Platforms SHALL support multiple activation types: player entry (standing on the polygon), player action key, monster entry, and projectile impact.

#### Scenario: Player-activated platform
- **WHEN** the player presses the action key while on a platform polygon with player-action activation
- **THEN** the platform SHALL begin moving from its resting position to its extended position at the defined speed

#### Scenario: Platform reaches destination
- **WHEN** a platform reaches its extended position
- **THEN** the platform SHALL stop and, if configured, begin a delay timer before returning

#### Scenario: Platform crushes entity
- **WHEN** a ceiling platform descends and an entity is between the floor and ceiling
- **THEN** if the platform has the crush flag, the entity SHALL take damage; otherwise the platform SHALL reverse

### Requirement: Platform triggers
The system SHALL support platforms that trigger other platforms or lights. When a platform reaches its extended or resting position, it SHALL activate linked platforms or toggle linked lights based on the map's trigger line definitions.

#### Scenario: Platform triggers linked platform
- **WHEN** platform A reaches its extended position and is linked to platform B via a trigger line
- **THEN** platform B SHALL begin its activation sequence

#### Scenario: Platform triggers light
- **WHEN** a platform activation triggers a light via a light trigger polygon
- **THEN** the targeted light SHALL toggle its state

### Requirement: Light animation
The system SHALL animate lights based on `StaticLightData` parsed from the map. Each light SHALL have a function type (constant, linear, smooth, flicker), a period (in ticks), a phase offset, and intensity bounds. The system SHALL compute each light's current intensity value each tick based on its function and elapsed time.

#### Scenario: Constant light
- **WHEN** a light has the constant function type
- **THEN** its intensity SHALL remain at the defined level regardless of tick count

#### Scenario: Smooth cycling light
- **WHEN** a light has the smooth function type with period 60 ticks
- **THEN** its intensity SHALL smoothly oscillate between minimum and maximum over 60-tick cycles using a cosine curve

#### Scenario: Flickering light
- **WHEN** a light has the flicker function type
- **THEN** its intensity SHALL randomly vary between minimum and maximum each tick

#### Scenario: Light phase offset
- **WHEN** two lights have the same function and period but different phase offsets
- **THEN** they SHALL animate identically but offset in time by their phase difference

### Requirement: Media simulation
The system SHALL simulate liquid media (water, lava, goo, sewage, jjaro) in polygons that reference a `MediaData` entry. Media height SHALL be derived from the light intensity of the media's associated light: as the light intensity varies, the liquid surface rises and falls between the media's defined low and high heights. Media SHALL apply current flow forces to entities standing in the liquid.

#### Scenario: Rising water
- **WHEN** the media's associated light intensity increases
- **THEN** the water surface height SHALL rise proportionally between the low and high bounds

#### Scenario: Lava damage
- **WHEN** the player is submerged in lava media
- **THEN** the player SHALL take environmental damage each tick based on the lava damage definition

#### Scenario: Media current pushes entity
- **WHEN** a player or monster stands in media with a defined current direction and magnitude
- **THEN** an external velocity SHALL be applied to the entity in the current direction

### Requirement: Item pickup and spawning
The system SHALL manage item entities in the world. When the player's collision radius overlaps an item entity, the item SHALL be picked up: the item entity is removed, and the player's inventory is updated (ammunition, weapon, health, shield, oxygen, or inventory item). Item spawn points defined in the map SHALL respawn items on a timer in multiplayer modes.

#### Scenario: Player picks up health powerup
- **WHEN** the player walks over a health item and player health is below maximum
- **THEN** the item entity SHALL be removed and the player's health SHALL increase by the item's defined amount

#### Scenario: Player picks up weapon
- **WHEN** the player walks over a weapon item that the player does not already have
- **THEN** the weapon SHALL be added to the player's inventory and the item entity SHALL be removed

#### Scenario: Player at max health ignores health item
- **WHEN** the player walks over a health item but health is already at maximum
- **THEN** the item SHALL remain in the world and not be picked up

#### Scenario: Item respawn in multiplayer
- **WHEN** an item is picked up and the game mode is multiplayer
- **THEN** the item SHALL respawn at its original position after a defined respawn delay

### Requirement: Control panel activation
The system SHALL detect when the player activates a control panel (action key pressed while facing a side with a control panel texture). Control panels SHALL trigger their associated action: platform activation, light toggle, or terminal display. Panel activation SHALL respect line-of-sight and distance requirements.

#### Scenario: Activate platform control panel
- **WHEN** the player presses the action key facing a control panel side linked to a platform
- **THEN** the linked platform SHALL begin its activation sequence

#### Scenario: Activate terminal
- **WHEN** the player presses the action key facing a terminal control panel
- **THEN** the system SHALL emit a terminal activation event with the terminal index

#### Scenario: Panel out of reach
- **WHEN** the player presses the action key but is too far from the control panel
- **THEN** no activation SHALL occur
