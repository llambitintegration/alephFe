## ADDED Requirements

### Requirement: Play ambient sound loops tied to map polygons
The system SHALL read `AmbientSoundImage` entries from the map data and associate each with the polygons that reference it via `ambient_sound_image_index`. For each ambient sound image with a valid `sound_index`, the system SHALL play the corresponding sound definition as a continuous loop, with base volume set from the ambient image's `volume` field.

#### Scenario: Polygon with ambient sound enters audible range
- **WHEN** the listener moves within audible range of a polygon whose `ambient_sound_image_index` references a valid `AmbientSoundImage`
- **THEN** the system SHALL start playing the referenced sound definition as a continuous loop

#### Scenario: Polygon with no ambient sound
- **WHEN** a polygon has `ambient_sound_image_index` set to -1 (none)
- **THEN** no ambient loop SHALL be activated for that polygon

#### Scenario: Ambient sound with invalid sound index
- **WHEN** an `AmbientSoundImage` references a `sound_index` that maps to an empty or out-of-range sound definition
- **THEN** the system SHALL silently skip that ambient sound

### Requirement: Scale ambient volume by listener distance to polygon
The system SHALL compute the distance from the listener's position to the center of each polygon with an active ambient sound. The ambient loop's volume SHALL be scaled by this distance using the sound definition's behavior-based attenuation curve, with the `AmbientSoundImage` volume field as the base (maximum) volume.

#### Scenario: Listener at polygon center
- **WHEN** the listener is at the center of a polygon with an ambient sound
- **THEN** the ambient loop SHALL play at the full base volume defined by the `AmbientSoundImage`

#### Scenario: Listener at maximum range
- **WHEN** the listener is at or beyond the maximum range for the ambient sound's behavior type
- **THEN** the ambient loop's volume SHALL be zero or the loop SHALL be deactivated

#### Scenario: Listener moves closer
- **WHEN** the listener moves from a far position to a closer position relative to an ambient polygon
- **THEN** the ambient loop's volume SHALL increase smoothly over the update

### Requirement: Activate and deactivate ambient sounds based on audible range
The system SHALL activate ambient sound loops only when the listener is within the audible range of the polygon (determined by the sound definition's behavior-based maximum distance). When the listener moves beyond audible range, the system SHALL deactivate the loop. Activation and deactivation SHALL use smooth volume transitions (fade in/out) to avoid audio pops.

#### Scenario: Ambient sound activation
- **WHEN** the listener enters the audible range of a polygon with an ambient sound that is not currently playing
- **THEN** the system SHALL start the loop and fade the volume in smoothly

#### Scenario: Ambient sound deactivation
- **WHEN** the listener leaves the audible range of a polygon with an active ambient sound
- **THEN** the system SHALL fade the volume out smoothly and then stop the loop

#### Scenario: Multiple ambient sounds active simultaneously
- **WHEN** the listener is within range of 5 different polygons with ambient sounds
- **THEN** all 5 ambient loops SHALL play simultaneously with independent volume levels

### Requirement: Clean up ambient sounds on level transition
The system SHALL stop all active ambient sound loops and release their resources when a level transition occurs. On the new level, the system SHALL read the new map's ambient sound data and initialize fresh ambient sound state.

#### Scenario: Level transition cleanup
- **WHEN** the game transitions from level 1 to level 2
- **THEN** all active ambient loops from level 1 SHALL be stopped and level 2's ambient sounds SHALL be initialized from the new map data

### Requirement: Play random sound sources at periodic intervals
The system SHALL read `RandomSoundImage` entries from the map data and associate each with the polygons that reference it via `random_sound_image_index`. For each random sound image, the system SHALL trigger playback of the referenced sound definition at intervals determined by the `period` field plus a random variance from `delta_period`.

#### Scenario: Random sound fires at interval
- **WHEN** a polygon has a random sound image with period 300 ticks and delta_period 60 ticks
- **THEN** the system SHALL trigger the sound at intervals randomly varying between 240 and 360 ticks

#### Scenario: Random sound only fires within audible range
- **WHEN** the listener is beyond the audible range of a polygon with a random sound source
- **THEN** the system SHALL not trigger the random sound (skip the interval timer for that source)

### Requirement: Apply random sound volume and pitch variance
The system SHALL apply per-playback variance to random sounds using the `RandomSoundImage` fields: the playback volume SHALL be the image's `volume` plus a random value in the range [0, `delta_volume`], and the playback pitch SHALL be the image's `pitch` plus a random value in the range [0, `delta_pitch`].

#### Scenario: Volume variance applied
- **WHEN** a random sound image has volume 200 and delta_volume 50
- **THEN** each playback SHALL have volume randomly selected from [200, 250]

#### Scenario: Pitch variance applied
- **WHEN** a random sound image has pitch 1.0 and delta_pitch 0.3
- **THEN** each playback SHALL have pitch randomly selected from [1.0, 1.3]

### Requirement: Apply random sound directional positioning
The system SHALL position random sounds using the `RandomSoundImage` direction field (in Marathon angle units, 0-511). If the `non-directional` flag (bit 0 of `flags`) is NOT set, the sound SHALL be positioned at the polygon center offset in the specified direction (plus random variance from `delta_direction`). If the flag IS set, the sound SHALL be positioned at the polygon center with no directional offset.

#### Scenario: Directional random sound
- **WHEN** a random sound image has direction 128 (90 degrees), delta_direction 32, and the non-directional flag is NOT set
- **THEN** the sound SHALL be positioned at the polygon center offset in a direction randomly varying between angle 96 and 160

#### Scenario: Non-directional random sound
- **WHEN** a random sound image has the non-directional flag set
- **THEN** the sound SHALL be positioned at the polygon center regardless of the direction field

### Requirement: Random sound sources respect sound definition flags
Random sound playback SHALL respect all `SoundFlags` from the referenced sound definition, including `CANNOT_BE_RESTARTED` (do not retrigger while previous instance is still playing), `DOES_NOT_SELF_ABORT` (allow overlapping periodic triggers), and `CANNOT_BE_OBSTRUCTED` / `CANNOT_BE_MEDIA_OBSTRUCTED` (skip obstruction processing).

#### Scenario: Random sound with CANNOT_BE_RESTARTED
- **WHEN** a random sound's timer fires but the previous instance of the same definition is still playing, and the definition has `CANNOT_BE_RESTARTED`
- **THEN** the system SHALL skip this trigger and wait for the next interval

#### Scenario: Random sound with DOES_NOT_SELF_ABORT
- **WHEN** a random sound's timer fires and a previous instance is still playing, and the definition has `DOES_NOT_SELF_ABORT`
- **THEN** both instances SHALL play simultaneously
