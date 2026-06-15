## MODIFIED Requirements

### Requirement: Platform movement
The system SHALL animate platforms (moving floors and ceilings) based on `StaticPlatformData` parsed from the map. Each platform SHALL have a resting position and an extended position. Platform movement speed SHALL follow the defined speed. Platforms SHALL support multiple activation types: player entry (standing on the polygon), player action key, monster entry, and projectile impact. The tick loop SHALL check the `PLATFORM_ACTIVATE_ON_ACTION_KEY` flag when the ACTION input flag is set and the action-key ray-cast identifies a platform as the target. The platform activation handler SHALL: activate an inactive platform if the player has any required key item; reverse a moving platform if it supports reversal; or play an uncontrollable sound if the platform cannot be operated.

#### Scenario: Player-activated platform via action key
- **WHEN** the player presses the action key while facing a door platform with `PLATFORM_ACTIVATE_ON_ACTION_KEY` set and the platform is at rest
- **THEN** the platform SHALL begin moving from its resting position to its extended position at the defined speed

#### Scenario: Platform reversal on action key
- **WHEN** the player presses the action key on an active moving platform that supports direction reversal
- **THEN** the platform SHALL reverse its movement direction

#### Scenario: Platform requires key item
- **WHEN** the player presses the action key on a platform that requires a key item and the player has that item
- **THEN** the key item SHALL be consumed and the platform SHALL activate

#### Scenario: Platform reaches destination
- **WHEN** a platform reaches its extended position
- **THEN** the platform SHALL stop and, if configured, begin a delay timer before returning

#### Scenario: Platform crushes entity
- **WHEN** a ceiling platform descends and an entity is between the floor and ceiling
- **THEN** if the platform has the crush flag, the entity SHALL take damage; otherwise the platform SHALL reverse

### Requirement: Control panel activation
The system SHALL spawn control panel data from map side entries during level load. For each `Side` where `control_panel_type >= 0` and the side has the control panel flag set, the system SHALL create a control panel record mapping `control_panel_type` and `control_panel_permutation` to a panel action. The system SHALL detect when the player activates a control panel (action key pressed while the ray-cast identifies a panel side within 1.5 world units). Control panels SHALL trigger their associated action: platform activation (`control_panel_type` 5), light toggle (`control_panel_type` 4), tag switch for grouped platforms/lights (`control_panel_type` 6), or terminal activation event (`control_panel_type` 9). Panel activation SHALL respect the ray-cast facing and distance requirements.

#### Scenario: Activate platform control panel
- **WHEN** the player presses the action key facing a control panel side linked to a platform (type 5)
- **THEN** the linked platform SHALL begin its activation sequence

#### Scenario: Activate light switch panel
- **WHEN** the player presses the action key facing a light switch control panel (type 4)
- **THEN** the linked light SHALL toggle between its active and inactive states

#### Scenario: Activate tag switch panel
- **WHEN** the player presses the action key facing a tag switch control panel (type 6)
- **THEN** all platforms and lights with the matching tag SHALL toggle their states

#### Scenario: Activate terminal
- **WHEN** the player presses the action key facing a terminal control panel (type 9)
- **THEN** the system SHALL emit a terminal activation event with the terminal index

#### Scenario: Panel out of reach
- **WHEN** the player presses the action key but the nearest control panel is more than 1.5 world units away
- **THEN** no activation SHALL occur

#### Scenario: Control panels loaded from map
- **WHEN** a level is loaded and the map contains sides with control panel flags
- **THEN** the system SHALL create control panel records for each qualifying side, accessible during tick processing
