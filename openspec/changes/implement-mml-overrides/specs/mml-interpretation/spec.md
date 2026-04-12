## ADDED Requirements

### Requirement: Interpret monster section into typed overrides
The system SHALL read a merged `<monsters>` MML section and produce a list of `MonsterOverride` structs. Each `<monster>` element's `index` attribute SHALL identify which monster definition to override. Each recognized attribute (`vitality`, `immunities`, `weaknesses`, `flags`, `class`, `friends`, `enemies`, `sound_pitch`, `speed`, `radius`, `height`, `visual_range`, `dark_visual_range`, `half_visual_arc`, `half_vertical_visual_arc`, `intelligence`, `carrying_item_type`, `must_be_exterminated`, etc.) SHALL be parsed into the corresponding typed `Option<T>` field. Unrecognized attributes SHALL be silently ignored.

#### Scenario: Monster override with subset of attributes
- **WHEN** the merged MML contains `<monster index="5" vitality="300" speed="10"/>`
- **THEN** the interpreter SHALL produce a `MonsterOverride` with `index=5`, `vitality=Some(300)`, `speed=Some(10)`, and all other fields `None`

#### Scenario: Monster override with no index
- **WHEN** the merged MML contains `<monster vitality="100"/>` with no `index` attribute
- **THEN** the interpreter SHALL skip the element and log a warning

#### Scenario: Monster override with malformed attribute value
- **WHEN** the merged MML contains `<monster index="5" vitality="not_a_number"/>`
- **THEN** the interpreter SHALL produce a `MonsterOverride` with `index=5` and `vitality=None`, logging a warning about the unparseable value

### Requirement: Interpret weapon section into typed overrides
The system SHALL read a merged `<weapons>` MML section and produce weapon-related override data including `<shell_casings>` definitions and `<order>` (weapon cycling order). Shell casing elements SHALL be matched by `index` and interpreted into typed `ShellCasingOverride` structs with optional fields for `coll`, `seq`, `x0`, `y0`, `vx0`, `vy0`, `dvx`, `dvy`. Weapon order elements SHALL produce an ordered list of weapon indices.

#### Scenario: Shell casing override
- **WHEN** the merged MML contains `<weapons><shell_casings index="0" coll="14" seq="2"/></weapons>`
- **THEN** the interpreter SHALL produce a `ShellCasingOverride` with `index=0`, `collection=Some(14)`, `sequence=Some(2)`, and position/velocity fields `None`

#### Scenario: Weapon order definition
- **WHEN** the merged MML contains `<weapons><order index="0" weapon="3"/><order index="1" weapon="0"/></weapons>`
- **THEN** the interpreter SHALL produce a weapon order mapping from slot index to weapon index

### Requirement: Interpret projectile section into typed overrides
The system SHALL read a merged `<projectiles>` MML section (if the MML document contains one at a future date, or through the physics override mechanism) and produce `ProjectileOverride` structs. Each override SHALL support optional fields for all `ProjectileDefinition` fields: `collection`, `shape`, `detonation_effect`, `media_detonation_effect`, `contrail_effect`, `ticks_between_contrails`, `maximum_contrails`, `radius`, `area_of_effect`, `damage`, `flags`, `speed`, `maximum_range`, `sound_pitch`, `flyby_sound`, `rebound_sound`.

#### Scenario: Projectile speed and damage override
- **WHEN** the MML contains a projectile override element with `index="3" speed="50" area_of_effect="512"`
- **THEN** the interpreter SHALL produce a `ProjectileOverride` with `index=3`, `speed=Some(50)`, `area_of_effect=Some(512)`, and other fields `None`

### Requirement: Interpret effect section into typed overrides
The system SHALL read effect override elements and produce `EffectOverride` structs with optional fields for `collection`, `shape`, `sound_pitch`, `flags`, `delay`, `delay_sound`.

#### Scenario: Effect override
- **WHEN** the MML contains an effect override with `index="2" delay="5"`
- **THEN** the interpreter SHALL produce an `EffectOverride` with `index=2`, `delay=Some(5)`, and other fields `None`

### Requirement: Interpret physics constants section into typed overrides
The system SHALL read a merged `<player>` MML section's physics-related attributes and produce overrides for `PhysicsConstants` fields. Attributes such as `energy`, `oxygen`, `light`, `oxygen_deplete`, `oxygen_replenish`, `half_visual_arc`, `half_vertical_visual_arc`, `visual_range`, `dark_visual_range`, `single_energy`, `double_energy`, `triple_energy`, `can_swim` SHALL be interpreted into a `PlayerOverride` struct.

#### Scenario: Player energy and oxygen overrides
- **WHEN** the merged MML contains `<player energy="200" oxygen="5400"/>`
- **THEN** the interpreter SHALL produce a `PlayerOverride` with `energy=Some(200)`, `oxygen=Some(5400)`, and other fields `None`

### Requirement: Interpret dynamic limits section
The system SHALL read a merged `<dynamic_limits>` section and produce a `DynamicLimitsOverride` struct with optional fields for `objects`, `monsters`, `paths`, `projectiles`, `effects`, `rendered`, `local_collision`, `global_collision`. Each child element's text content SHALL be parsed as an integer.

#### Scenario: Override monster and projectile limits
- **WHEN** the merged MML contains `<dynamic_limits><monsters>1024</monsters><projectiles>256</projectiles></dynamic_limits>`
- **THEN** the interpreter SHALL produce a `DynamicLimitsOverride` with `monsters=Some(1024)`, `projectiles=Some(256)`, and other fields `None`

### Requirement: Interpret item section into typed overrides
The system SHALL read a merged `<items>` section and produce `ItemOverride` structs with optional fields for `type`, `singular` (name), `plural` (name), `maximum`, and `invalid` (boolean).

#### Scenario: Item maximum override
- **WHEN** the merged MML contains `<items><item index="7" maximum="5"/></items>`
- **THEN** the interpreter SHALL produce an `ItemOverride` with `index=7`, `maximum=Some(5)`, and other fields `None`

### Requirement: Interpret landscape section into typed overrides
The system SHALL read a merged `<landscapes>` section and produce `LandscapeOverride` structs and `<clear>` directives. Each `<landscape>` element SHALL support optional fields for `coll`, `frame`, `horiz_exp`, `vert_exp`, `ogl_asprat_exp`, `vert_repeat`, `azimuth`. Each `<clear>` element SHALL indicate which collection to reset.

#### Scenario: Landscape assignment override
- **WHEN** the merged MML contains `<landscapes><landscape coll="27" frame="0" horiz_exp="1"/></landscapes>`
- **THEN** the interpreter SHALL produce a `LandscapeOverride` with `collection=Some(27)`, `frame=Some(0)`, `horiz_exp=Some(1)`, and other fields `None`

### Requirement: Interpret texture loading section
The system SHALL read a merged `<texture_loading>` section and produce `TextureLoadingOverride` data including the `landscapes` boolean attribute and `<texture_env>` entries with `index`, `which`, and `coll` fields.

#### Scenario: Texture environment override
- **WHEN** the merged MML contains `<texture_loading landscapes="true"><texture_env index="0" which="1" coll="17"/></texture_loading>`
- **THEN** the interpreter SHALL produce a `TextureLoadingOverride` with `landscapes=Some(true)` and one texture environment entry

### Requirement: Interpret string set section
The system SHALL read a merged `<stringset>` section and produce string override entries keyed by `(resource_id, string_index)`. Each `<stringset>` element's `index` attribute identifies the resource ID (128-149). Each child `<string>` element's `index` attribute identifies the string position, and the element's text content is the replacement string.

#### Scenario: Override an error message string
- **WHEN** the merged MML contains `<stringset index="128"><string index="0">Custom error</string></stringset>`
- **THEN** the interpreter SHALL produce a string override mapping `(128, 0)` to `"Custom error"`

### Requirement: Parse MML attribute values with AlephOne-compatible type rules
The system SHALL parse MML attribute values using these rules: integers accept decimal and hexadecimal (0x prefix) notation. Booleans accept `1`, `t`, `true` as true and `0`, `f`, `false` as false. Fixed-point values (where indicated by the section schema) SHALL be parsed as floating-point and converted to the engine's internal representation. Malformed values SHALL produce `None` (no override) and log a warning, never causing a parse failure for the entire document.

#### Scenario: Hexadecimal attribute value
- **WHEN** an MML attribute contains `flags="0x1F"`
- **THEN** the system SHALL parse the value as integer 31

#### Scenario: Boolean true variants
- **WHEN** an MML attribute contains `must_be_exterminated="true"` or `must_be_exterminated="1"` or `must_be_exterminated="t"`
- **THEN** the system SHALL parse all three as boolean `true`

#### Scenario: Invalid value does not fail the document
- **WHEN** an MML monster element contains `vitality="abc"`
- **THEN** the system SHALL log a warning, set that field to `None`, and continue interpreting the rest of the document
