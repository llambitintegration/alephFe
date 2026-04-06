## ADDED Requirements

### Requirement: Play background music on a dedicated channel
The system SHALL play music on a dedicated audio channel (kira sub-track) that is independent from the spatial sound effects channel. Music volume SHALL be controlled separately from the sound effects volume.

#### Scenario: Music plays independently from sound effects
- **WHEN** music is playing and sound effects are also playing
- **THEN** music and sound effects SHALL be mixed independently, each with their own volume level

#### Scenario: Music volume adjustment does not affect sound effects
- **WHEN** the music volume is changed from 0.8 to 0.3
- **THEN** sound effect volumes SHALL remain unchanged

### Requirement: Start music by song index
The system SHALL accept a song index (from `MapInfo.song_index`) and begin playback of the corresponding music track. If a music track is already playing, the system SHALL crossfade from the current track to the new one.

#### Scenario: Start music on level load
- **WHEN** a level is loaded with `MapInfo.song_index` of 3
- **THEN** the system SHALL begin playing music track 3

#### Scenario: Song index is invalid or none
- **WHEN** a level has `song_index` set to -1 or an out-of-range value
- **THEN** no music SHALL play and any currently playing music SHALL fade out

### Requirement: Crossfade between music tracks
The system SHALL crossfade when transitioning between music tracks. The outgoing track SHALL fade out while the incoming track fades in simultaneously. The crossfade duration SHALL be approximately 2 seconds.

#### Scenario: Level transition with different music
- **WHEN** the game transitions from a level with song 3 to a level with song 7
- **THEN** song 3 SHALL fade out over ~2 seconds while song 7 fades in over ~2 seconds

#### Scenario: Level transition with same music
- **WHEN** the game transitions to a level with the same `song_index` as the current level
- **THEN** the music SHALL continue playing without interruption or crossfade

### Requirement: Stop music with fade out
The system SHALL support stopping music with a smooth fade out. This SHALL be triggered by game events such as entering a cutscene, game over, or explicit stop requests.

#### Scenario: Music stop requested
- **WHEN** a music stop event is received
- **THEN** the current music track SHALL fade out smoothly over ~2 seconds and then stop

#### Scenario: Stop when no music is playing
- **WHEN** a music stop event is received but no music is currently playing
- **THEN** the system SHALL do nothing

### Requirement: Set music volume independently
The system SHALL expose a music volume control (0.0 to 1.0) that scales the music channel's output level independently from sound effects. This volume SHALL be adjustable at any time during playback.

#### Scenario: Set music volume during playback
- **WHEN** music is playing and the music volume is set to 0.5
- **THEN** the music output level SHALL be scaled to 50% of the track's natural volume

#### Scenario: Set music volume to zero
- **WHEN** music volume is set to 0.0
- **THEN** the music SHALL continue playing but produce no audible output (muted, not stopped)

### Requirement: Clean up music on level transition
The system SHALL handle music cleanup as part of level transitions. If the new level has a different `song_index`, the crossfade requirement applies. If the new level has no music (`song_index` is -1), the current track SHALL fade out and stop.

#### Scenario: Transition to level with no music
- **WHEN** transitioning from a level with music to a level with `song_index` of -1
- **THEN** the current music SHALL fade out and stop, with no new track started

#### Scenario: Transition to level with music
- **WHEN** transitioning from a level with `song_index` 3 to a level with `song_index` 5
- **THEN** track 3 SHALL crossfade to track 5 per the crossfade requirement
