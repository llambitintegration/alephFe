# Spec: physics-formats

Capability for parsing Marathon physics model files. Physics files are WAD containers whose single entry holds tagged chunks for up to five physics types: monster definitions (tag `MNpx`), effect definitions (tag `FXpx`), projectile definitions (tag `PRpx`), player physics constants (tag `PXpx`), and weapon definitions (tag `WPpx`). Marathon 1 compatibility uses alternate four-character tags (`mons`, `effe`, `proj`, `phys`, `weap`). Each tag's data is an array of fixed-size records packed in big-endian byte order.

---

### Requirement: ADDED -- Parse player physics constants

The parser must read `PXpx` (or `phys` for Marathon 1) tagged data as an array of `physics_constants` records. Each record is 104 bytes containing 26 fixed-point 16.16 (`_fixed` = `i32`) fields. The parser must convert every fixed-point value to `f32` by dividing the raw `i32` by 65536.0. The standard physics file contains 2 records (walking and running models).

Fields in serialization order:
- `maximum_forward_velocity`, `maximum_backward_velocity`, `maximum_perpendicular_velocity`
- `acceleration`, `deceleration`, `airborne_deceleration`
- `gravitational_acceleration`, `climbing_acceleration`, `terminal_velocity`
- `external_deceleration`
- `angular_acceleration`, `angular_deceleration`, `maximum_angular_velocity`, `angular_recentering_velocity`
- `fast_angular_velocity`, `fast_angular_maximum`
- `maximum_elevation`
- `external_angular_deceleration`
- `step_delta`, `step_amplitude`
- `radius`, `height`, `dead_height`, `camera_height`, `splash_height`
- `half_camera_separation`

#### Scenario: WHEN parsing a PXpx tag containing 208 bytes of data THEN the parser produces 2 physics_constants records (208 / 104 = 2), one for walking and one for running

#### Scenario: WHEN a fixed-point field contains the raw value 0x00010000 (FIXED_ONE = 65536) THEN the parsed f32 value is 1.0

#### Scenario: WHEN a fixed-point field contains the raw value 0xFFFF0000 (-65536) THEN the parsed f32 value is -1.0

#### Scenario: WHEN a fixed-point field contains the raw value 0x00008000 (0.5 in 16.16) THEN the parsed f32 value is 0.5

#### Scenario: WHEN the walking model has maximum_forward_velocity raw value of FIXED_ONE/14 (4681) THEN the parsed f32 is approximately 0.07143

---

### Requirement: ADDED -- Parse monster definitions from physics WAD

The parser must read `MNpx` (or `mons` for Marathon 1) tagged data as an array of `monster_definition` records. Each record is 156 bytes. The parser must produce a struct containing all monster fields.

Fields in serialization order:
- `collection` (i16) -- shape collection index
- `vitality` (i16)
- `immunities` (u32), `weaknesses` (u32) -- damage type bitmasks
- `flags` (u32) -- behavioral flags (omniscient, flies, alien, major/minor, etc.)
- `_class` (i32) -- monster class bitmask
- `friends` (i32), `enemies` (i32) -- class bitmasks for friend/foe identification
- `sound_pitch` (i32, fixed-point)
- `activation_sound`, `friendly_activation_sound`, `clear_sound`, `kill_sound`, `apology_sound`, `friendly_fire_sound` (each i16) -- 6 sound indices
- `flaming_sound` (i16)
- `random_sound` (i16), `random_sound_mask` (i16)
- `carrying_item_type` (i16)
- `radius` (i16, world_distance), `height` (i16, world_distance)
- `preferred_hover_height` (i16, world_distance)
- `minimum_ledge_delta` (i16), `maximum_ledge_delta` (i16)
- `external_velocity_scale` (i32, fixed-point)
- `impact_effect` (i16), `melee_impact_effect` (i16), `contrail_effect` (i16)
- `half_visual_arc` (i16), `half_vertical_visual_arc` (i16)
- `visual_range` (i16, world_distance), `dark_visual_range` (i16, world_distance)
- `intelligence` (i16)
- `speed` (i16), `gravity` (i16), `terminal_velocity` (i16)
- `door_retry_mask` (i16)
- `shrapnel_radius` (i16)
- `shrapnel_damage` -- embedded damage_definition: `type` (i16), `flags` (i16), `base` (i16), `random` (i16), `scale` (i32 fixed-point) -- 12 bytes
- `hit_shapes` (u16, shape_descriptor)
- `hard_dying_shape` (u16), `soft_dying_shape` (u16)
- `hard_dead_shapes` (u16), `soft_dead_shapes` (u16)
- `stationary_shape` (u16), `moving_shape` (u16)
- `teleport_in_shape` (u16), `teleport_out_shape` (u16)
- `attack_frequency` (i16)
- `melee_attack` -- embedded attack_definition: `type` (i16), `repetitions` (i16), `error` (i16, angle), `range` (i16, world_distance), `attack_shape` (i16), `dx` (i16), `dy` (i16), `dz` (i16) -- 16 bytes
- `ranged_attack` -- embedded attack_definition (same layout as melee_attack) -- 16 bytes

#### Scenario: WHEN parsing a MNpx tag containing 4680 bytes THEN the parser produces 30 monster definitions (4680 / 156 = 30)

#### Scenario: WHEN a monster definition has flags value 0x0002 THEN the monster has the _monster_flys flag set

#### Scenario: WHEN a monster has carrying_item_type of -1 (NONE) THEN the parsed value indicates no carried item

#### Scenario: WHEN parsing attack_definition fields where type is -1 (NONE) THEN the attack is treated as absent (no melee or ranged capability)

---

### Requirement: ADDED -- Parse projectile definitions

The parser must read `PRpx` (or `proj` for Marathon 1) tagged data as an array of `projectile_definition` records. Each record is 48 bytes.

Fields in serialization order:
- `collection` (i16), `shape` (i16) -- collection can be NONE (-1) for invisible projectiles
- `detonation_effect` (i16), `media_detonation_effect` (i16)
- `contrail_effect` (i16), `ticks_between_contrails` (i16), `maximum_contrails` (i16) -- NONE means infinite
- `media_projectile_promotion` (i16)
- `radius` (i16, world_distance) -- can be zero and still hit
- `area_of_effect` (i16, world_distance) -- zero means single target
- `damage` -- embedded damage_definition: `type` (i16), `flags` (i16), `base` (i16), `random` (i16), `scale` (i32 fixed-point) -- 12 bytes
- `flags` (u32) -- projectile behavioral flags (guided, persistent, affected_by_gravity, etc.)
- `speed` (i16, world_distance per tick)
- `maximum_range` (i16, world_distance)
- `sound_pitch` (i32, fixed-point)
- `flyby_sound` (i16), `rebound_sound` (i16)

#### Scenario: WHEN parsing a PRpx tag containing 1056 bytes THEN the parser produces 22 projectile definitions (1056 / 48 = 22)

#### Scenario: WHEN a projectile has flags value 0x0010 THEN the _affected_by_gravity flag is set

#### Scenario: WHEN a projectile has collection value -1 (NONE) THEN the projectile is invisible (no rendered sprite)

#### Scenario: WHEN a projectile has area_of_effect of 0 THEN it deals damage to a single target only

---

### Requirement: ADDED -- Parse effect definitions

The parser must read `FXpx` (or `effe` for Marathon 1) tagged data as an array of `effect_definition` records. Each record is 14 bytes.

Fields in serialization order:
- `collection` (i16), `shape` (i16) -- shape collection and shape index
- `sound_pitch` (i32, fixed-point)
- `flags` (u16) -- effect behavioral flags (end_when_animation_loops, sound_only, etc.)
- `delay` (i16) -- ticks before effect appears
- `delay_sound` (i16) -- sound to play during delay, NONE if no delay sound

#### Scenario: WHEN parsing an FXpx tag containing 406 bytes THEN the parser produces 29 effect definitions (406 / 14 = 29)

#### Scenario: WHEN an effect has flags value 0x0001 THEN the _end_when_animation_loops flag is set

#### Scenario: WHEN an effect has delay of 0 and delay_sound of -1 (NONE) THEN the effect appears immediately with no delay sound

---

### Requirement: ADDED -- Parse weapon definitions including dual triggers

The parser must read `WPpx` (or `weap` for Marathon 1) tagged data as an array of `weapon_definition` records. Each record is 134 bytes. Each weapon contains exactly 2 trigger definitions (primary and secondary), supporting dual-wielded and multi-function weapons.

Fields in serialization order:
- `item_type` (i16) -- corresponding item pickup type
- `powerup_type` (i16) -- NONE if no powerup variant
- `weapon_class` (i16) -- melee, normal, dual_function, twofisted_pistol, or multipurpose
- `flags` (i16) -- weapon behavioral flags (automatic, disappears_after_use, overloads, etc.)
- `firing_light_intensity` (i32, fixed-point)
- `firing_intensity_decay_ticks` (i16)
- `idle_height` (i32, fixed-point), `bob_amplitude` (i32, fixed-point), `kick_height` (i32, fixed-point), `reload_height` (i32, fixed-point)
- `idle_width` (i32, fixed-point), `horizontal_amplitude` (i32, fixed-point)
- `collection` (i16) -- weapon-in-hand shape collection
- `idle_shape` (i16), `firing_shape` (i16), `reloading_shape` (i16)
- `unused` (i16)
- `charging_shape` (i16), `charged_shape` (i16)
- `ready_ticks` (i16), `await_reload_ticks` (i16), `loading_ticks` (i16), `finish_loading_ticks` (i16), `powerup_ticks` (i16)
- `weapons_by_trigger[0]` (primary trigger) -- trigger_definition, 38 bytes
- `weapons_by_trigger[1]` (secondary trigger) -- trigger_definition, 38 bytes

Each trigger_definition contains in serialization order:
- `rounds_per_magazine` (i16)
- `ammunition_type` (i16) -- NONE for unlimited/melee
- `ticks_per_round` (i16), `recovery_ticks` (i16), `charging_ticks` (i16)
- `recoil_magnitude` (i16, world_distance)
- `firing_sound` (i16), `click_sound` (i16), `charging_sound` (i16), `shell_casing_sound` (i16), `reloading_sound` (i16), `charged_sound` (i16)
- `projectile_type` (i16)
- `theta_error` (i16)
- `dx` (i16), `dz` (i16)
- `shell_casing_type` (i16)
- `burst_count` (i16)
- `sound_activation_range` (i16) -- Marathon 1 compatibility field

#### Scenario: WHEN parsing a WPpx tag containing 1340 bytes THEN the parser produces 10 weapon definitions (1340 / 134 = 10)

#### Scenario: WHEN a weapon has weapon_class value 3 (_twofisted_pistol_class) THEN both primary and secondary triggers are independently active for dual-wielded operation

#### Scenario: WHEN a trigger has ammunition_type of -1 (NONE) THEN the weapon uses no consumable ammunition (e.g., fist melee)

#### Scenario: WHEN a trigger has burst_count of 0 THEN the weapon fires single shots (no burst mode)

#### Scenario: WHEN a weapon has powerup_type of -1 (NONE) THEN the weapon has no powered-up variant

---

### Requirement: ADDED -- Support both Marathon 2/Infinity tags and Marathon 1 tags

The parser must recognize both Marathon 2/Infinity four-character tag codes and Marathon 1 tag codes for all five physics types, selecting the appropriate parsing path based on which tags are present in the WAD entry.

Tag mapping:
| Physics Type   | Marathon 2/Infinity Tag | Marathon 1 Tag |
|----------------|------------------------|----------------|
| Monster        | `MNpx`                 | `mons`         |
| Effect         | `FXpx`                 | `effe`         |
| Projectile     | `PRpx`                 | `proj`         |
| Player Physics | `PXpx`                 | `phys`         |
| Weapon         | `WPpx`                 | `weap`         |

#### Scenario: WHEN a physics WAD entry contains a tag with code `MNpx` THEN monster definitions are parsed using the Marathon 2/Infinity record layout (156 bytes per record)

#### Scenario: WHEN a physics WAD entry contains a tag with code `mons` THEN monster definitions are parsed using the Marathon 1 record layout

#### Scenario: WHEN a physics WAD entry contains `PXpx` tagged data THEN player physics records are 104 bytes each with all 26 fixed-point fields

#### Scenario: WHEN a physics WAD entry contains `phys` tagged data THEN the first 100-byte record is an editor record that must be skipped, and the remaining records contain player physics constants

#### Scenario: WHEN both Marathon 2/Infinity and Marathon 1 tags for the same physics type are present THEN the Marathon 2/Infinity tag takes precedence

---

### Requirement: ADDED -- Handle physics files with partial content

The parser must successfully parse physics files that contain only a subset of the five tag types. Not all physics files include every physics type; a valid physics file may contain as few as one tagged chunk. The parser must not require the presence of any specific tag and must return parsed results only for the tags that are present.

#### Scenario: WHEN a physics WAD entry contains only a PXpx tag and no other physics tags THEN the parser returns player physics data and indicates that monster, effect, projectile, and weapon data are absent

#### Scenario: WHEN a physics WAD entry contains MNpx and WPpx tags but no FXpx, PRpx, or PXpx tags THEN the parser returns monster and weapon data and indicates the other three types are absent

#### Scenario: WHEN a physics WAD entry contains zero recognized physics tags THEN the parser returns a result with all five physics types absent, without producing an error

#### Scenario: WHEN a tag's data length is not evenly divisible by the expected record size THEN the parser returns an error for that specific physics type rather than silently truncating
