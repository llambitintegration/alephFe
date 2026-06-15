## MODIFIED Requirements

### Requirement: Control panel activation

The system SHALL detect when the player activates a control panel (action key pressed while facing a side with a control panel texture). Control panels SHALL trigger their associated action: platform activation, light toggle, or terminal display. Panel activation SHALL respect line-of-sight and distance requirements. Action-key activation — for both control panels and adjacent door/platform polygons — SHALL be edge-triggered: it SHALL fire at most once per key press (on the tick the ACTION flag transitions from clear to set) and SHALL NOT re-fire on subsequent ticks while the key remains held.

#### Scenario: Activate platform control panel

- **WHEN** the player presses the action key facing a control panel side linked to a platform
- **THEN** the linked platform SHALL begin its activation sequence

#### Scenario: Activate terminal

- **WHEN** the player presses the action key facing a terminal control panel
- **THEN** the system SHALL emit a terminal activation event with the terminal index

#### Scenario: Panel out of reach

- **WHEN** the player presses the action key but is too far from the control panel
- **THEN** no activation SHALL occur

#### Scenario: Held action key does not re-trigger

- **WHEN** the player presses and holds the action key facing a door for multiple consecutive ticks
- **THEN** the door SHALL be activated exactly once on the first tick of the press, and no further activation SHALL occur until the key is released and pressed again
