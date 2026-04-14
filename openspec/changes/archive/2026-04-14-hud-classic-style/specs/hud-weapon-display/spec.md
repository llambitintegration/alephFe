## ADDED Requirements

### Requirement: Display current weapon name
The system SHALL display the name of the currently equipped weapon in the right column of the HUD. The weapon name SHALL be displayed as text using a monospace or retro-styled font. The name SHALL update immediately when the player switches weapons.

#### Scenario: Weapon name shown
- **WHEN** the player has the Assault Rifle equipped
- **THEN** the text "Assault Rifle" SHALL be displayed in the weapon info column

#### Scenario: Weapon switch updates name
- **WHEN** the player switches from Magnum to Shotgun
- **THEN** the weapon name SHALL change from "Magnum" to "Shotgun" on the next frame

#### Scenario: Fists equipped
- **WHEN** the player has Fists equipped
- **THEN** the text "Fists" SHALL be displayed in the weapon info column

### Requirement: Display primary and secondary ammo counts
The system SHALL display numeric ammunition counts for the currently equipped weapon. For weapons with a primary trigger, the primary ammo count SHALL be displayed. For weapons with a secondary trigger, the secondary ammo count SHALL also be displayed. Weapons with no ammunition (e.g., Fists) SHALL show no ammo counters.

#### Scenario: Dual-trigger weapon
- **WHEN** the player has the Assault Rifle with 52 primary rounds and 7 grenades
- **THEN** the HUD SHALL display "52" as primary ammo and "7" as secondary ammo

#### Scenario: Single-trigger weapon
- **WHEN** the player has the Magnum with 8 rounds and no secondary trigger
- **THEN** the HUD SHALL display "8" as primary ammo and no secondary ammo indicator

#### Scenario: Infinite ammo weapon
- **WHEN** the player has Fists equipped
- **THEN** no ammo counters SHALL be displayed

#### Scenario: Ammo updates on fire
- **WHEN** the player fires one primary round and ammo decreases from 52 to 51
- **THEN** the primary ammo display SHALL show "51" on the next frame

### Requirement: Sim exposes weapon ammo data
The sim layer SHALL provide a method to query the current weapon's definition index, primary magazine count, and secondary magazine count. This method SHALL return None if no weapon is equipped.

#### Scenario: Weapon equipped with ammo
- **WHEN** the player has weapon definition_index=3 with 52 primary and 7 secondary rounds
- **THEN** the query SHALL return (3, 52, 7)

#### Scenario: No weapon equipped
- **WHEN** no weapon is equipped
- **THEN** the query SHALL return None
