## MODIFIED Requirements

### Requirement: Camera system
The web renderer SHALL map the sim's player position to 3D camera coordinates using the same coordinate convention as the mesh builder: camera X = sim position X, camera Y = sim position Z + EYE_HEIGHT (vertical), camera Z = sim position Y. The camera yaw SHALL be read from sim player facing. The camera pitch SHALL be read from sim player vertical look angle.

#### Scenario: Camera positioned at player floor height
- **WHEN** the sim reports player position as Vec3(px, py, pz) where pz is the vertical floor height
- **THEN** the camera SHALL be placed at 3D position (px, pz + EYE_HEIGHT, py) matching the mesh coordinate system

#### Scenario: Camera yaw tracks player facing
- **WHEN** the sim reports player facing as angle θ
- **THEN** the camera yaw SHALL equal θ so the view direction matches the player's facing

#### Scenario: Camera pitch tracks vertical look
- **WHEN** the sim reports player vertical look as angle φ
- **THEN** the camera pitch SHALL equal φ, allowing the player to look up and down

### Requirement: Batched texture rendering
The web renderer SHALL draw level geometry in batches grouped by texture collection, binding the correct GPU texture array for each batch. Each batch SHALL correspond to a contiguous range of indices in the index buffer sharing the same collection index.

#### Scenario: Multiple texture collections render correctly
- **WHEN** the level uses polygons from collections 17 and 28
- **THEN** the renderer SHALL bind collection 17's texture for its batch and collection 28's texture for its batch, with no INVALID_ENUM errors
