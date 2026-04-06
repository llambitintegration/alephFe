## Context

The `marathon-formats` crate already provides complete parsing of Marathon's sound data: `SoundDefinition` with behavior types, flags, permutation offsets, and pitch/volume ranges; `AmbientSoundImage` and `RandomSoundImage` for map-level environmental audio; `Polygon` fields linking to ambient/random sound indexes; `MediaData` for underwater/lava media detection; and `Line`/`LineFlags` for wall obstruction tracing. The `marathon-sim` crate does not exist yet, so the audio crate must define its own input traits rather than depending on concrete sim types. Only `marathon-formats` is a workspace member today.

Marathon audio is 8-bit unsigned mono at 22050 Hz. All spatial math uses Marathon's 2D coordinate system (top-down XY plane; height differences exist but Marathon's original audio engine ignores vertical separation for attenuation purposes). The engine supports up to ~120 simultaneous sound channels in practice.

## Goals / Non-Goals

**Goals:**
- Provide a standalone `marathon-audio` library crate that turns parsed sound data + world state into real-time spatial audio output
- Faithfully reproduce Marathon's three distance attenuation curves (quiet/normal/loud), permutation selection, volume/pitch randomization, and behavioral flags
- Support ambient loops and random periodic sounds tied to map polygons
- Compute wall obstruction via polygon adjacency traversal and media-based muffling
- Handle music playback on a separate mix channel with crossfade support
- Define clean input interfaces (traits or event structs) so the crate can be integrated with `marathon-sim` later without API changes

**Non-Goals:**
- Implementing the game simulation or entity system (that's `marathon-sim`)
- 3D height-based attenuation (Marathon's original engine uses 2D distance only)
- Network audio synchronization for multiplayer
- Sound asset loading or format conversion (consumers provide decoded PCM via `marathon-formats`)
- Doppler effect (Marathon doesn't implement it; `RESISTS_PITCH_CHANGES` exists but for environmental pitch shifts only)
- Supporting non-Marathon audio formats (MP3, OGG streaming, etc.)

## Decisions

### 1. Audio backend: kira

Use the `kira` crate (v0.9+) as the audio backend.

**Rationale:** kira provides a mixer graph with sub-tracks, spatial emitter positioning, volume/pitch tweens, and low-latency real-time control -- all primitives Marathon audio needs. It runs its own audio thread, keeping the game loop decoupled. Alternatives considered:
- **rodio**: Simpler API but lacks a mixer graph, spatial emitters, and tween system. Would require building most spatial logic from scratch.
- **cpal + custom mixer**: Maximum control but massive implementation effort for a mixer, resampler, and spatial pipeline that kira already provides.
- **FMOD/Wwise bindings**: Non-Rust, non-open-source, heavyweight for a faithful retro engine recreation.

### 2. Input interface: event structs, not trait objects

Define the boundary between simulation and audio as plain data structs (`SoundEvent`, `ListenerState`, `AmbientUpdate`) rather than trait objects or direct ECS coupling.

**Rationale:** `marathon-sim` doesn't exist yet, and coupling to a specific ECS (bevy_ecs) would create a hard dependency. Event structs are:
- Testable without a running simulation
- Serializable for replay/debugging
- Compatible with any future sim architecture

The audio system exposes an `AudioEngine` that accepts these events via a `process_events(&mut self, events: &[AudioEvent])` method each tick.

### 3. Spatial model: 2D with per-behavior attenuation curves

Implement distance attenuation in 2D (XY plane only), matching the original engine. Each `SoundBehavior` maps to a different (max_distance, falloff_curve) pair:
- **Quiet**: ~5 world units max range, steep falloff
- **Normal**: ~10 world units max range, linear falloff
- **Loud**: ~20 world units max range, gradual falloff

(Exact values to be calibrated against the original C++ source constants during implementation.)

Directional panning uses the angle between the listener's facing direction and the vector to the sound source, mapped to stereo left/right balance.

**Alternatives considered:**
- Full 3D spatial audio with HRTF: Overkill for a 2.5D game engine; Marathon's original audio was 2D panning and that's what players expect.

### 4. Obstruction: BFS through polygon adjacency graph

Sound obstruction is computed by tracing a path from the source polygon to the listener polygon through the map's polygon adjacency graph (via shared lines). For each line crossed:
- If `LineFlags::SOLID` and the line has no transparent side: add full obstruction
- If `LineFlags::HAS_TRANSPARENT_SIDE` or `LineFlags::TRANSPARENT`: add partial obstruction based on opening size

Total obstruction is clamped and applied as volume reduction plus a simple low-pass filter (kira's filter effect on the sound's sub-track).

Sounds with `CANNOT_BE_OBSTRUCTED` flag skip this entirely.

**Alternatives considered:**
- Raycast-based obstruction: More physically accurate but Marathon's original engine uses polygon-graph traversal, and raycasting against BSP/polygon soup is more complex to implement for marginal fidelity gain in a retro engine.

### 5. Media obstruction: frequency filter by media type

When the listener or sound source is submerged (polygon's `media_index` references a `MediaData` whose `height` is above the entity), apply a low-pass filter. The cutoff frequency varies by media type:
- **Water/Sewage**: moderate muffling (~800 Hz cutoff)
- **Lava/Goo/Jjaro**: heavier muffling (~400 Hz cutoff)

Sounds with `CANNOT_BE_MEDIA_OBSTRUCTED` skip this.

### 6. Permutation selection: weighted random without immediate repeat

Select permutations using a simple strategy: track the last-played permutation index per sound definition and exclude it from the next random selection (unless only 1 permutation exists). This avoids the most obvious repetition without complex shuffle logic.

The `chance` field on `SoundDefinition` gates whether the sound plays at all (random roll against chance value before selecting a permutation).

### 7. Sound instance management: channel pool with priority

Maintain a fixed pool of sound channels (e.g., 32 simultaneous sounds). When the pool is full and a new sound is requested:
1. Evict the quietest (lowest effective volume after distance attenuation) sound
2. Respect `CANNOT_BE_RESTARTED`: don't evict sounds with this flag unless all channels have it
3. Respect `DOES_NOT_SELF_ABORT`: allow multiple simultaneous instances of the same sound definition

Each active sound is tracked with its source entity (if any), sound definition index, current kira handle, and start time.

### 8. Music: separate kira track with crossfade

Music plays on a dedicated kira sub-track with independent volume control. Crossfading between tracks uses kira's tween system (linear fade over ~2 seconds). The `song_index` from `MapInfo` drives which track to play on level load.

Music data comes from the same sound file as effects (Marathon stores music as sound definitions) or from external files if the integration layer provides them.

### 9. Crate structure: single lib crate, no features initially

```
marathon-audio/
  Cargo.toml        # depends on kira, marathon-formats
  src/
    lib.rs          # public API: AudioEngine, AudioEvent, ListenerState
    engine.rs       # AudioEngine implementation
    spatial.rs      # distance attenuation, panning, obstruction
    ambient.rs      # ambient + random sound source management
    music.rs        # music playback + crossfade
    channel.rs      # sound channel pool + instance tracking
    types.rs        # shared types (SoundId, ChannelId, etc.)
```

No feature flags initially. The crate is a library with no platform-specific code (kira handles platform abstraction).

### 10. Update cadence: driven by caller, not internal timer

The audio engine does not own a thread or timer. The integration layer calls `AudioEngine::update(dt, listener, events)` each frame (or each sim tick). This keeps the audio crate deterministic-friendly and avoids threading complexity. kira's internal audio thread handles actual sample output independently.

## Risks / Trade-offs

**[Attenuation curve fidelity]** The exact distance constants and curve shapes for quiet/normal/loud behaviors need to be extracted from the original C++ source. If the values are wrong, spatial audio will feel off.
  -> Mitigation: Cross-reference with Aleph One source code (`sound_definitions.h`, `_sound_behavior_definition` structs). Add calibration tests comparing output volumes at known distances.

**[kira API stability]** kira is pre-1.0 and its API may change between versions.
  -> Mitigation: Pin to a specific minor version. Wrap kira types behind internal abstractions in `engine.rs` so a backend swap only touches one module.

**[Obstruction BFS performance]** Large maps with many polygons could make per-sound BFS expensive if many sounds are active simultaneously.
  -> Mitigation: Cache obstruction values per (source_polygon, listener_polygon) pair and invalidate only when the listener moves to a new polygon. Most frames, the listener stays in the same polygon.

**[marathon-sim interface uncertainty]** The sim crate doesn't exist yet, so the event-based interface is speculative.
  -> Mitigation: Keep the interface minimal (position, facing, sound triggers). If the sim ends up using a different pattern, the event structs are cheap to adapt or wrap.

**[Channel pool size]** 32 channels may be too few for dense combat scenes or too many for weaker hardware.
  -> Mitigation: Make channel count configurable at `AudioEngine` construction time. 32 is a reasonable default matching the original engine's practical limits.

## Open Questions

- **Exact attenuation constants**: What are the precise max-distance and falloff exponent values for each `SoundBehavior`? Need to extract from Aleph One's `sound_definitions.h`.
- **Music source format**: Will music come from the sound file (as indexed sound definitions) or from separate audio files provided by the integration layer? The proposal mentions `song_index` from `MapInfo` but the actual data source path needs clarification.
- **Obstruction granularity**: Should partial obstruction (transparent lines) use a fixed reduction factor or scale by the line's opening height? The original engine's approach needs verification.
