## ADDED Requirements

### Requirement: Health bar display
The web HUD SHALL display the player's current health as a horizontal bar. The bar color SHALL change based on health tier: green above 66%, yellow between 33-66%, red below 33%.

#### Scenario: Health bar reflects current health
- **WHEN** the player has 150 health out of 150 maximum
- **THEN** the HUD SHALL show a full green health bar

#### Scenario: Health bar changes color at low health
- **WHEN** the player's health drops below 33% of maximum
- **THEN** the health bar SHALL display in red

### Requirement: Shield bar display
The web HUD SHALL display the player's current shield as a horizontal bar alongside the health bar. The shield bar SHALL use a blue/cyan color scheme.

#### Scenario: Shield bar reflects current shield
- **WHEN** the player has 100 shield out of 150 maximum
- **THEN** the HUD SHALL show the shield bar at approximately 66% fill

### Requirement: Oxygen meter display
The web HUD SHALL display an oxygen meter only when the player is submerged in media or in a vacuum environment. The oxygen meter SHALL be hidden during normal atmospheric conditions.

#### Scenario: Oxygen meter appears when submerged
- **WHEN** the player enters a water polygon
- **THEN** the HUD SHALL display the oxygen meter showing remaining oxygen

#### Scenario: Oxygen meter hidden in normal atmosphere
- **WHEN** the player is in a normal atmospheric polygon
- **THEN** the HUD SHALL NOT display the oxygen meter

### Requirement: HUD updates from sim state
The web HUD SHALL read player health, shield, and oxygen values from the sim via wasm-bindgen exported functions. Updates SHALL occur at a throttled rate (no more than 10 times per second) to avoid excessive DOM manipulation.

#### Scenario: HUD values stay in sync with sim
- **WHEN** the player takes damage reducing health from 150 to 100
- **THEN** the HUD health bar SHALL update to reflect the new value within 100ms
