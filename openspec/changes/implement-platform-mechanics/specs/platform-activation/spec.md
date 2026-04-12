## ADDED Requirements

### Requirement: Player-entry platform activation
The system SHALL check each tick whether the player's current polygon is a platform polygon. When the player occupies a platform polygon and the platform is at rest with the `ACTIVATE_ON_PLAYER_ENTRY` flag set, the system SHALL activate that platform.

#### Scenario: Player steps onto elevator
- **WHEN** the player moves into a polygon controlled by a platform that has `ACTIVATE_ON_PLAYER_ENTRY` set and is at rest
- **THEN** the platform SHALL transition to the Extending state and begin moving

#### Scenario: Player on already-moving platform
- **WHEN** the player is on a platform polygon but the platform is in the Extending state
- **THEN** no additional activation SHALL occur (the platform continues its current movement)

#### Scenario: Player on platform without entry activation
- **WHEN** the player is on a platform polygon that does NOT have `ACTIVATE_ON_PLAYER_ENTRY` set
- **THEN** the platform SHALL remain at rest

### Requirement: Action-key platform activation
The system SHALL activate a platform when the player presses the ACTION key while on a platform polygon that has the `ACTIVATE_ON_ACTION_KEY` flag set. If the platform is currently moving (Extending or Returning), pressing ACTION SHALL reverse the platform's direction.

#### Scenario: Action key activates resting platform
- **WHEN** the player presses ACTION on a platform polygon with `ACTIVATE_ON_ACTION_KEY` and the platform is at rest
- **THEN** the platform SHALL transition to Extending

#### Scenario: Action key reverses extending platform
- **WHEN** the player presses ACTION on a platform polygon with `ACTIVATE_ON_ACTION_KEY` and the platform is Extending
- **THEN** the platform SHALL transition to Returning

#### Scenario: Action key reverses returning platform
- **WHEN** the player presses ACTION on a platform polygon with `ACTIVATE_ON_ACTION_KEY` and the platform is Returning
- **THEN** the platform SHALL transition to Extending

#### Scenario: Action key on platform at extended position
- **WHEN** the player presses ACTION on a platform that is AtExtended (waiting to return)
- **THEN** the platform SHALL immediately transition to Returning, bypassing remaining delay

### Requirement: Control panel platform activation
The system SHALL activate a platform when the player activates a control panel linked to that platform via `PanelAction::ActivatePlatform`. The activation SHALL follow the same re-activation rules as action-key activation (reversing a moving platform).

#### Scenario: Panel activates linked platform
- **WHEN** the player activates a control panel whose action is `ActivatePlatform { platform_index: N }`
- **THEN** platform N SHALL be activated (or reversed if already moving)

### Requirement: Monster-entry platform activation
The system SHALL check each tick whether any monster entity's current polygon is a platform polygon. When a monster occupies a platform polygon and the platform is at rest with the `ACTIVATE_ON_MONSTER_ENTRY` flag set, the system SHALL activate that platform.

#### Scenario: Monster steps onto platform
- **WHEN** a monster's polygon index matches a platform polygon that has `ACTIVATE_ON_MONSTER_ENTRY` set and is at rest
- **THEN** the platform SHALL transition to Extending

#### Scenario: Monster on platform without monster activation
- **WHEN** a monster is on a platform polygon without `ACTIVATE_ON_MONSTER_ENTRY` set
- **THEN** the platform SHALL remain at rest

### Requirement: Projectile-impact platform activation
The system SHALL check each tick whether any projectile entity's current polygon is a platform polygon. When a projectile occupies a platform polygon and the platform is at rest with the `ACTIVATE_ON_PROJECTILE` flag set, the system SHALL activate that platform.

#### Scenario: Projectile hits platform polygon
- **WHEN** a projectile enters a polygon controlled by a platform that has `ACTIVATE_ON_PROJECTILE` set and is at rest
- **THEN** the platform SHALL transition to Extending

### Requirement: Linked platform cascading
The system SHALL process linked platform triggers when a platform reaches its extended or resting position. Each platform MAY reference zero or more linked platform indices. When a platform reaches AtExtended or AtRest, each linked platform SHALL be activated.

#### Scenario: Platform A triggers platform B on arrival
- **WHEN** platform A reaches AtExtended and has platform B in its linked platforms list
- **THEN** platform B SHALL be activated (transition to Extending if at rest)

#### Scenario: Chain of three linked platforms
- **WHEN** platform A links to B, and B links to C, and A reaches AtExtended
- **THEN** B SHALL activate on this tick, and C SHALL activate when B later reaches its destination

#### Scenario: Platform with no links reaches destination
- **WHEN** a platform with empty linked lists reaches AtExtended
- **THEN** no other platforms or lights SHALL be affected

### Requirement: Linked light toggling
The system SHALL process linked light triggers when a platform reaches its extended or resting position. Each platform MAY reference zero or more linked light indices. When a platform reaches AtExtended or AtRest, each linked light SHALL be toggled.

#### Scenario: Platform toggles light on arrival
- **WHEN** a platform reaches AtExtended and has light index 3 in its linked lights list
- **THEN** light 3 SHALL receive a toggle event

### Requirement: Platform sound events
The system SHALL emit sound events for platform movement. When a platform transitions from AtRest to Extending or from AtExtended to Returning, a start-movement sound SHALL be emitted. When a platform transitions to AtExtended or AtRest, a stop-movement sound SHALL be emitted. While a platform is in the Extending or Returning state, a looping movement sound SHALL be active.

#### Scenario: Elevator starts moving
- **WHEN** a platform transitions from AtRest to Extending
- **THEN** the system SHALL emit a `SimEvent::SoundTrigger` with the platform's start sound index at the platform polygon's center position

#### Scenario: Door finishes opening
- **WHEN** a platform reaches AtExtended
- **THEN** the system SHALL emit a `SimEvent::SoundTrigger` with the platform's stop sound index
