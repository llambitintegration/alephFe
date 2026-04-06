## Why

Marathon's audio system is inseparable from its gameplay -- sound positioning creates spatial awareness of off-screen threats, obstruction through walls and underwater media communicates level topology, and ambient/random sound sources give each area of a map a distinct acoustic identity. The `marathon-formats` crate can already parse sound definitions and permutation metadata, and `marathon-sim` will provide entity positions and game events, but there is no component that turns this data into actual audio output. A dedicated `marathon-audio` crate that handles spatial sound playback, environmental audio, and music will complete the sensory feedback loop that Marathon's game design depends on.

Building audio as a separate crate (rather than embedding it in simulation or rendering) is the right boundary because audio has its own update cadence, its own spatial model (distance attenuation, obstruction, directional panning are distinct from visual rendering), and its own platform concerns (mixer management, streaming, output device handling). The `kira` crate provides the spatial audio primitives, mixer graph, and tween system needed to implement Marathon's audio model without writing a low-level audio engine from scratch.

## What Changes

- Create a new `marathon-audio` crate that plays Marathon sound content with full spatial, environmental, and behavioral fidelity
- Implement spatial audio positioning: sounds placed in 3D world space relative to the listener (player), with distance-based volume attenuation using Marathon's three behavior types (`quiet`, `normal`, `loud`) that define different falloff curves
- Support sound definitions with up to 5 permutations per sound, randomly selecting a variation on each play to avoid repetitive audio
- Apply volume and pitch variation per playback: base volume/pitch plus a random delta range from the sound definition
- Implement directional sound via angle-based panning, attenuating sounds based on the angle between the listener's facing direction and the sound source
- Play ambient sound sources: looping sounds tied to specific map polygons, continuously active while the player is within audible range
- Play random sound sources: periodic sounds tied to map polygons that fire at intervals with per-definition variance in period, volume, direction, and pitch
- Compute sound obstruction through walls: trace a path from source to listener through the map's polygon connectivity, accumulating obstruction when solid lines (walls) intervene, and apply a low-pass filter or volume reduction accordingly
- Apply media obstruction effects: when the listener or source is submerged in a media surface (water, lava, etc.), muffle audio with frequency filtering to simulate underwater acoustics
- Respect sound definition flags: `cannot_be_restarted` (prevent retriggering while playing), `does_not_self_abort` (allow overlapping instances of the same sound), `resists_pitch_changes` (ignore Doppler or environmental pitch shifts), and `is_ambient` (mark as ambient-class for separate volume control)
- Manage music playback: background music start/stop/crossfade, separate from spatial sound, with its own volume control independent of sound effects

## Capabilities

### New Capabilities

- `spatial-audio`: 3D positioned sound playback with distance-based volume attenuation, directional panning, and obstruction modeling. Covers the core sound-playing pipeline: receiving a play request (sound definition index + world position + source entity), selecting a random permutation, applying volume/pitch deltas, positioning the sound in the spatial mix relative to the listener, computing wall obstruction and media effects along the sound path, and managing the sound instance lifecycle including the behavioral flags (cannot_be_restarted, does_not_self_abort, resists_pitch_changes). Also handles the three distance falloff behavior types (quiet, normal, loud) that control how quickly sounds attenuate over distance.

- `ambient-sound`: Looping environmental audio tied to map polygons. Each polygon in the map can reference an ambient sound definition, and the system continuously plays those loops with volume scaled by the listener's distance to the polygon. Handles activation/deactivation as polygons enter or leave audible range, smooth volume transitions to avoid pops, and proper cleanup on level transitions. Also covers random sound sources -- periodic (non-looping) sounds tied to polygons that fire at randomized intervals with per-definition variance in period, volume, direction, and pitch.

- `music-playback`: Background music management independent of spatial sound. Handles starting, stopping, and crossfading music tracks, maintaining a separate volume channel from sound effects, and responding to game events (level transitions, cutscenes) that trigger music changes.

### Modified Capabilities

None. This is a new crate with no modifications to existing capabilities.

## Impact

- **`marathon-formats`** (dependency): The audio crate depends on `marathon-formats` for sound definition data -- sound headers (behavior type, flags, permutation count, volume/pitch base and delta values), permutation metadata (offset, length into sound data), and map-level ambient/random sound source definitions (polygon assignments, period, delta fields). This will be the first consumer of the sound-related parsing and may surface gaps or needed API refinements.

- **`marathon-sim`** (dependency): The audio crate needs entity positions (for spatial sound source tracking), listener/player position and facing angle (for spatial mix computation), map polygon connectivity (for obstruction path tracing), and game event notifications (weapon fired, monster alerted, projectile impact, etc.) that trigger sound playback. The interface between sim and audio will need to be defined -- likely an event/command channel where the simulation emits sound requests and the audio system processes them independently.

- **New crate**: `marathon-audio` added to the workspace, depending on `kira` for the audio backend (spatial emitters, mixer tracks, tweens, streaming), `marathon-formats` for content data, and `marathon-sim` for world state.

- **Future integration** (`marathon-integration`): The integration crate will own the audio system's lifecycle -- creating it at game start, feeding it listener updates each frame, and shutting it down cleanly. The audio crate itself is a library with no main loop or windowing concerns.
