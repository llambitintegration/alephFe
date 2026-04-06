## ADDED Requirements

### Requirement: Play a positioned sound from a sound definition
The system SHALL accept a sound play request containing a sound definition index, a world position (x, y), and an optional source entity identifier. The system SHALL look up the `SoundDefinition` from the loaded `SoundsFile`, verify the definition is not empty (`is_empty() == false`), and initiate playback of the sound at the specified position in the spatial mix.

#### Scenario: Valid sound play request
- **WHEN** a sound play request is received with sound definition index 5, position (1024, 2048), and source entity 42
- **THEN** the system SHALL look up definition index 5 from the loaded sound data, confirm it is non-empty, and begin spatial playback at position (1024, 2048) associated with entity 42

#### Scenario: Empty sound definition slot
- **WHEN** a sound play request references a sound definition where `is_empty()` returns true
- **THEN** the system SHALL silently ignore the request and not initiate playback

#### Scenario: Sound definition index out of range
- **WHEN** a sound play request references a sound definition index that does not exist in the loaded sound data
- **THEN** the system SHALL silently ignore the request and not initiate playback

### Requirement: Select a random permutation avoiding immediate repeat
The system SHALL select a permutation randomly from the available permutations (0 to `permutation_count() - 1`) for each playback. If the sound definition has more than one permutation, the system SHALL exclude the most recently played permutation index for that definition from the selection pool.

#### Scenario: Multiple permutations available
- **WHEN** a sound definition has 3 permutations and permutation 1 was most recently played
- **THEN** the system SHALL randomly select from permutations 0 and 2 only

#### Scenario: Single permutation
- **WHEN** a sound definition has exactly 1 permutation
- **THEN** the system SHALL always select permutation 0

#### Scenario: First play of a definition
- **WHEN** a sound definition has not been played before in the current session
- **THEN** the system SHALL select from all available permutations with equal probability

### Requirement: Apply volume and pitch randomization from sound definition
The system SHALL compute the playback volume as a random value between the base volume and the base volume plus the delta range defined by the sound definition. The system SHALL compute the playback pitch as a random value within the range defined by `low_pitch` to `high_pitch` from the `SoundDefinition`.

#### Scenario: Volume randomization
- **WHEN** a sound definition specifies a base volume and a volume delta
- **THEN** the playback volume SHALL be a random value in the range [base, base + delta], before distance attenuation is applied

#### Scenario: Pitch randomization
- **WHEN** a sound definition has `low_pitch` of 0.8 and `high_pitch` of 1.2
- **THEN** the playback pitch SHALL be a random value in the range [0.8, 1.2]

#### Scenario: Pitch randomization with RESISTS_PITCH_CHANGES flag
- **WHEN** a sound definition has the `RESISTS_PITCH_CHANGES` flag set
- **THEN** the system SHALL still apply pitch randomization from the definition's own range, but SHALL NOT apply any environmental pitch modifications

### Requirement: Apply chance-based play gating
The system SHALL evaluate the `chance` field of the `SoundDefinition` before playing a sound. If `chance` is 0 (or the maximum value indicating "always"), the sound SHALL always play. Otherwise, the system SHALL generate a random value and only play the sound if the roll is within the chance threshold.

#### Scenario: Chance is zero (always play)
- **WHEN** a sound definition has `chance` value of 0
- **THEN** the system SHALL always proceed with playback

#### Scenario: Chance gating rejects playback
- **WHEN** a sound definition has a non-zero `chance` value and the random roll exceeds the threshold
- **THEN** the system SHALL not play the sound

### Requirement: Distance-based volume attenuation with behavior curves
The system SHALL attenuate sound volume based on the 2D distance (XY plane) between the sound source position and the listener position. The attenuation curve SHALL be determined by the sound definition's `SoundBehavior`:
- **Quiet**: short maximum range, steep falloff
- **Normal**: medium maximum range, linear falloff
- **Loud**: long maximum range, gradual falloff

Sounds beyond the maximum range for their behavior type SHALL have zero volume and MAY be culled from active playback.

#### Scenario: Sound within range using Normal behavior
- **WHEN** a sound with Normal behavior is at 2D distance 5 units from the listener, and Normal max range is 10 units
- **THEN** the volume SHALL be attenuated proportionally to the distance within the Normal falloff curve

#### Scenario: Sound beyond maximum range
- **WHEN** a sound with Quiet behavior is at a 2D distance exceeding the Quiet max range
- **THEN** the effective volume SHALL be zero

#### Scenario: Sound at listener position
- **WHEN** a sound source is at the same position as the listener (distance = 0)
- **THEN** the volume SHALL be at full level (no distance attenuation)

### Requirement: Directional stereo panning based on listener facing
The system SHALL compute the angle between the listener's facing direction and the vector from the listener to the sound source. This angle SHALL be mapped to a stereo panning value: sounds directly ahead are centered, sounds to the left are panned left, and sounds to the right are panned right. Sounds directly behind the listener SHALL be centered with reduced volume.

#### Scenario: Sound directly ahead
- **WHEN** a sound source is directly in front of the listener's facing direction
- **THEN** the panning SHALL be centered (equal left and right)

#### Scenario: Sound to the right
- **WHEN** a sound source is 90 degrees to the right of the listener's facing direction
- **THEN** the panning SHALL be fully right

#### Scenario: Sound directly behind
- **WHEN** a sound source is 180 degrees from the listener's facing direction
- **THEN** the panning SHALL be centered with a rear attenuation factor applied

### Requirement: Wall obstruction via polygon adjacency traversal
The system SHALL compute sound obstruction by tracing a path from the source polygon to the listener polygon through the map's polygon adjacency graph. For each line crossed during traversal:
- Lines with `SOLID` flag and no transparent side SHALL add full obstruction
- Lines with `TRANSPARENT` or `HAS_TRANSPARENT_SIDE` flags SHALL add partial obstruction

Total obstruction SHALL be clamped to a maximum value and applied as volume reduction and low-pass filtering on the sound.

#### Scenario: Direct line of sound (same polygon)
- **WHEN** the sound source and listener are in the same polygon
- **THEN** no wall obstruction SHALL be applied

#### Scenario: One solid wall between source and listener
- **WHEN** the path from source to listener crosses one line with `SOLID` flag and no transparent side
- **THEN** full obstruction SHALL be applied for that wall crossing (volume reduction + low-pass filter)

#### Scenario: Path through transparent line
- **WHEN** the path crosses a line with `HAS_TRANSPARENT_SIDE` flag
- **THEN** partial obstruction SHALL be applied (less than a fully solid wall)

#### Scenario: Sound with CANNOT_BE_OBSTRUCTED flag
- **WHEN** a sound definition has the `CANNOT_BE_OBSTRUCTED` flag set
- **THEN** the system SHALL skip wall obstruction computation entirely and apply no obstruction

#### Scenario: Obstruction caching across frames
- **WHEN** the listener remains in the same polygon between updates
- **THEN** the system SHALL reuse cached obstruction values for sound sources whose polygons have not changed

### Requirement: Media obstruction when submerged
The system SHALL detect when the listener or sound source is submerged in a media surface by checking if the polygon's `media_index` references a `MediaData` whose `height` exceeds the entity's vertical position. When submerged, the system SHALL apply a low-pass filter with cutoff frequency determined by media type:
- Water and Sewage: moderate muffling
- Lava, Goo, and Jjaro: heavy muffling

#### Scenario: Listener submerged in water
- **WHEN** the listener is in a polygon with water media whose height exceeds the listener's vertical position
- **THEN** all sounds SHALL have a moderate low-pass filter applied

#### Scenario: Source submerged in lava
- **WHEN** a sound source is in a polygon with lava media and is submerged
- **THEN** that sound SHALL have a heavy low-pass filter applied

#### Scenario: Neither submerged
- **WHEN** neither the listener nor the sound source is submerged in any media
- **THEN** no media obstruction filter SHALL be applied

#### Scenario: Sound with CANNOT_BE_MEDIA_OBSTRUCTED flag
- **WHEN** a sound definition has the `CANNOT_BE_MEDIA_OBSTRUCTED` flag set
- **THEN** the system SHALL skip media obstruction processing for that sound

### Requirement: CANNOT_BE_RESTARTED flag prevents retriggering
The system SHALL track active sound instances by their sound definition index. When a new play request arrives for a sound definition with the `CANNOT_BE_RESTARTED` flag set, and an instance of that definition is already playing, the system SHALL ignore the new request.

#### Scenario: Retriggering a non-restartable sound
- **WHEN** a play request for sound definition 10 arrives, definition 10 has `CANNOT_BE_RESTARTED`, and an instance is currently playing
- **THEN** the new request SHALL be ignored

#### Scenario: Retriggering after previous instance completes
- **WHEN** a play request for sound definition 10 arrives, definition 10 has `CANNOT_BE_RESTARTED`, and no instance is currently playing
- **THEN** the sound SHALL play normally

### Requirement: DOES_NOT_SELF_ABORT allows overlapping instances
The system SHALL allow multiple simultaneous instances of the same sound definition when the `DOES_NOT_SELF_ABORT` flag is set. When this flag is NOT set and a new instance of the same definition is triggered, the system SHALL stop the existing instance before starting the new one.

#### Scenario: Overlapping instances allowed
- **WHEN** sound definition 20 has `DOES_NOT_SELF_ABORT` set and a new play request arrives while an instance is playing
- **THEN** both instances SHALL play simultaneously

#### Scenario: Self-abort when flag is not set
- **WHEN** sound definition 20 does NOT have `DOES_NOT_SELF_ABORT` set and a new play request arrives while an instance is playing
- **THEN** the existing instance SHALL be stopped and the new instance SHALL begin

### Requirement: Sound channel pool with priority eviction
The system SHALL maintain a configurable pool of sound channels (default 32). When all channels are in use and a new sound is requested, the system SHALL evict the active sound with the lowest effective volume (after distance attenuation). Sounds with the `CANNOT_BE_RESTARTED` flag SHALL be evicted only as a last resort.

#### Scenario: Channel pool full, eviction needed
- **WHEN** all 32 channels are in use and a new sound request arrives
- **THEN** the system SHALL stop the active sound with the lowest effective volume and assign its channel to the new sound

#### Scenario: All channels have CANNOT_BE_RESTARTED
- **WHEN** all channels are occupied by sounds with `CANNOT_BE_RESTARTED` and a new request arrives for a sound without that flag
- **THEN** the system SHALL evict the lowest-volume `CANNOT_BE_RESTARTED` sound

### Requirement: Update listener state each tick
The system SHALL accept a `ListenerState` containing the listener's world position (x, y, z), facing angle, and current polygon index. This state SHALL be provided by the caller each update tick and used for all spatial calculations (attenuation, panning, obstruction, media detection) until the next update.

#### Scenario: Listener state update
- **WHEN** the caller provides a new `ListenerState` with position (512, 768, 100), facing angle 90 degrees, and polygon index 5
- **THEN** all subsequent spatial calculations SHALL use this listener state until the next update call

### Requirement: Sound source position tracking for moving entities
The system SHALL support updating the position of an active sound tied to a source entity. When the caller provides updated entity positions, all active sounds associated with those entities SHALL have their spatial parameters (distance, panning, obstruction) recalculated.

#### Scenario: Moving entity sound
- **WHEN** entity 42 has an active sound and the caller updates entity 42's position from (100, 200) to (150, 250)
- **THEN** the sound's attenuation, panning, and obstruction SHALL be recalculated using the new position
