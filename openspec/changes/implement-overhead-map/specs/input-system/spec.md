## MODIFIED Requirements

### Requirement: Translate gameplay input to Marathon action flags
The input system SHALL map the Tab key to the TOGGLE_MAP action flag in the Gameplay input context. On desktop (winit), pressing `KeyCode::Tab` SHALL set the TOGGLE_MAP flag for the current tick. On web (JavaScript), the existing Tab keydown handler SHALL set the toggle_map flag in the input state. Additionally, the + (or =) key SHALL trigger zoom-in and the - key SHALL trigger zoom-out for the overhead map when it is visible.

#### Scenario: Tab key sets TOGGLE_MAP on desktop
- **WHEN** the Tab key is pressed in the desktop build during Gameplay context
- **THEN** the TOGGLE_MAP action flag SHALL be set for the current tick's input

#### Scenario: Tab key sets toggle_map on web
- **WHEN** the Tab key is pressed in the web build during gameplay
- **THEN** the input state toggle_map flag SHALL be set, triggering overhead map visibility toggle

#### Scenario: Plus key zooms in on desktop
- **WHEN** the + or = key is pressed while the overhead map is visible in the desktop build
- **THEN** the system SHALL increase the overhead map zoom level

#### Scenario: Minus key zooms out on desktop
- **WHEN** the - key is pressed while the overhead map is visible in the desktop build
- **THEN** the system SHALL decrease the overhead map zoom level

#### Scenario: Plus key zooms in on web
- **WHEN** the + or = key is pressed while the overhead map is visible in the web build
- **THEN** the system SHALL increase the overhead map zoom level

#### Scenario: Minus key zooms out on web
- **WHEN** the - key is pressed while the overhead map is visible in the web build
- **THEN** the system SHALL decrease the overhead map zoom level
