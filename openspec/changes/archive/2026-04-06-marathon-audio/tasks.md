## 1. Crate Setup

- [x] 1.1 Create `marathon-audio/` crate directory with `Cargo.toml` (depends on `kira`, `marathon-formats` as path dep) and add to workspace `Cargo.toml`
- [x] 1.2 Create `src/lib.rs` with module declarations (`engine`, `spatial`, `ambient`, `music`, `channel`, `types`) and public API re-exports
- [x] 1.3 Create `src/types.rs` with shared types: `SoundId`, `ChannelId`, `ListenerState` (position, facing angle, polygon index), `AudioEvent` enum (PlaySound, StopSound, UpdateListener, UpdateEntityPosition, PlayMusic, StopMusic, SetMusicVolume, SetSfxVolume, LevelTransition)

## 2. Sound Channel Pool

- [x] 2.1 Create `src/channel.rs` with `ChannelPool` struct: configurable capacity (default 32), stores active `SoundInstance` entries (definition index, entity id, kira sound handle, effective volume, flags)
- [x] 2.2 Implement `ChannelPool::allocate()`: find free channel or evict lowest effective-volume sound, respecting `CANNOT_BE_RESTARTED` eviction priority
- [x] 2.3 Implement `ChannelPool::find_by_definition()`: look up active instances by sound definition index (needed for `CANNOT_BE_RESTARTED` and `DOES_NOT_SELF_ABORT` checks)
- [x] 2.4 Implement `ChannelPool::release()`: free a channel when its sound finishes, and `update_volumes()`: refresh effective volumes for eviction priority
- [x] 2.5 Write unit tests for channel allocation, eviction priority, and flag-based behavior

## 3. Spatial Audio Core

- [x] 3.1 Create `src/spatial.rs` with distance attenuation functions for each `SoundBehavior` (Quiet, Normal, Loud) — takes 2D distance, returns volume multiplier (0.0–1.0)
- [x] 3.2 Implement directional stereo panning: compute angle from listener facing to sound source, map to pan value (-1.0 left to 1.0 right) with rear attenuation
- [x] 3.3 Implement permutation selection: track last-played index per definition, select random excluding last (wrapping for single permutation)
- [x] 3.4 Implement volume/pitch randomization from `SoundDefinition` fields (low_pitch/high_pitch range, volume base + delta)
- [x] 3.5 Implement chance-based play gating using `SoundDefinition.chance` field
- [x] 3.6 Write unit tests for attenuation curves, panning math, permutation selection, and randomization ranges

## 4. Wall Obstruction

- [x] 4.1 Implement BFS/path trace through polygon adjacency graph: given source polygon index and listener polygon index, traverse `adjacent_polygon_indexes` and accumulate obstruction from crossed lines
- [x] 4.2 Implement obstruction scoring per line: full obstruction for `SOLID` without transparent side, partial for `TRANSPARENT`/`HAS_TRANSPARENT_SIDE`
- [x] 4.3 Implement obstruction cache keyed by (source_polygon, listener_polygon) pair, invalidated when listener polygon changes
- [x] 4.4 Apply obstruction as volume reduction and low-pass filter parameters (to be passed to kira filter effect)
- [x] 4.5 Skip obstruction for sounds with `CANNOT_BE_OBSTRUCTED` flag
- [x] 4.6 Write unit tests for BFS traversal, obstruction accumulation, cache invalidation, and flag bypass

## 5. Media Obstruction

- [x] 5.1 Implement submersion detection: check polygon's `media_index` against `MediaData` entries, compare media `height` to entity vertical position
- [x] 5.2 Implement low-pass filter parameter selection by `MediaTypeEnum`: Water/Sewage → moderate cutoff, Lava/Goo/Jjaro → heavy cutoff
- [x] 5.3 Skip media obstruction for sounds with `CANNOT_BE_MEDIA_OBSTRUCTED` flag
- [x] 5.4 Write unit tests for submersion detection and per-media-type filter values

## 6. Ambient Sound System

- [x] 6.1 Create `src/ambient.rs` with `AmbientManager` struct: holds state for all ambient loops (active/inactive, kira handle, current volume, polygon index)
- [x] 6.2 Implement `AmbientManager::init_level()`: read `AmbientSoundImage` entries and polygon `ambient_sound_image_index` fields from map data, build initial state
- [x] 6.3 Implement ambient activation/deactivation based on listener distance to polygon center vs. behavior max range, with smooth fade in/out transitions
- [x] 6.4 Implement ambient volume scaling by listener distance using behavior-based attenuation curves
- [x] 6.5 Implement `AmbientManager::cleanup()`: stop all loops and release resources on level transition
- [x] 6.6 Write unit tests for activation range logic, volume scaling, and level cleanup

## 7. Random Sound Sources

- [x] 7.1 Implement `RandomSoundManager` in `src/ambient.rs`: holds per-polygon timer state, reads `RandomSoundImage` entries and polygon `random_sound_image_index` fields
- [x] 7.2 Implement periodic trigger logic: decrement timer each tick, fire sound when timer reaches zero, reset with `period + random(0, delta_period)`
- [x] 7.3 Apply per-playback variance: volume from `volume + random(0, delta_volume)`, pitch from `pitch + random(0, delta_pitch)`
- [x] 7.4 Apply directional positioning: offset from polygon center by `direction + random(0, delta_direction)` angle, skip if non-directional flag is set
- [x] 7.5 Gate random sounds by listener audible range: skip timer processing for out-of-range polygons
- [x] 7.6 Ensure random sound playback respects `SoundFlags` from the referenced definition (`CANNOT_BE_RESTARTED`, `DOES_NOT_SELF_ABORT`, etc.)
- [x] 7.7 Write unit tests for timer logic, variance application, directional positioning, and flag behavior

## 8. Music Playback

- [x] 8.1 Create `src/music.rs` with `MusicPlayer` struct: holds kira sub-track handle, current song index, volume level
- [x] 8.2 Implement `MusicPlayer::play(song_index)`: start music track, crossfade from current if one is playing (~2 second kira tween)
- [x] 8.3 Implement `MusicPlayer::stop()`: fade out current track over ~2 seconds and release handle
- [x] 8.4 Implement `MusicPlayer::set_volume(f32)`: adjust music channel volume independently from SFX
- [x] 8.5 Implement level transition logic: crossfade to new song, continue if same song, fade out if new level has no music (song_index -1)
- [x] 8.6 Write unit tests for crossfade transitions, volume control, and no-music level handling

## 9. Audio Engine Integration

- [x] 9.1 Create `src/engine.rs` with `AudioEngine` struct: owns kira `AudioManager`, `ChannelPool`, `AmbientManager`, `RandomSoundManager`, `MusicPlayer`, and spatial state
- [x] 9.2 Implement `AudioEngine::new(config)`: initialize kira audio manager, create mixer sub-tracks for SFX and music, instantiate sub-systems with configurable channel count
- [x] 9.3 Implement `AudioEngine::load_level(map_data, sounds_file)`: initialize ambient/random managers from map data, set up music from `MapInfo.song_index`
- [x] 9.4 Implement `AudioEngine::update(dt, listener_state, events)`: process `AudioEvent` list — dispatch PlaySound (with permutation selection, chance gating, flag checks, spatial positioning, obstruction), StopSound, music events; update ambient/random managers; refresh spatial parameters for moving sources
- [x] 9.5 Implement `AudioEngine::shutdown()`: stop all sounds, release kira resources
- [x] 9.6 Write integration tests using mock/stub kira backend (or kira's mock audio manager if available) to verify event processing, level load/unload, and engine lifecycle

## 10. Sound Data Bridge

- [x] 10.1 Implement audio data decoding: convert raw 8-bit unsigned mono 22050 Hz PCM bytes from `SoundsFile::audio_data()` into kira-compatible `StaticSoundData`
- [x] 10.2 Implement sound definition cache: pre-decode all non-empty sound definitions from the loaded `SoundsFile` into kira sound data on level load, keyed by (source_index, sound_index)
- [x] 10.3 Write tests to verify PCM conversion produces correct sample format and duration for known input data
