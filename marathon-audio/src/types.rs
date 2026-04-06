use marathon_formats::sounds::{SoundBehavior, SoundFlags};

/// Unique identifier for an active sound instance in the channel pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelId(pub u32);

/// Listener state provided by the caller each update tick.
#[derive(Debug, Clone, Copy)]
pub struct ListenerState {
    /// World position X coordinate.
    pub x: f32,
    /// World position Y coordinate.
    pub y: f32,
    /// Vertical position (for media submersion checks).
    pub z: f32,
    /// Facing angle in radians (0 = east, counter-clockwise).
    pub facing_angle: f32,
    /// Index of the polygon the listener is currently in.
    pub polygon_index: i16,
}

/// A request to play a positioned sound.
#[derive(Debug, Clone)]
pub struct PlaySoundRequest {
    /// Index into the sound definitions array.
    pub sound_index: usize,
    /// World position of the sound source.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Optional entity that owns this sound (for position tracking).
    pub source_entity: Option<u32>,
    /// Polygon index of the sound source (for obstruction).
    pub source_polygon: i16,
}

/// Events sent to the audio engine each update tick.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// Play a positioned sound effect.
    PlaySound(PlaySoundRequest),
    /// Stop all sounds from a specific entity.
    StopSound { source_entity: u32 },
    /// Update an entity's position (for tracking moving sound sources).
    UpdateEntityPosition {
        entity_id: u32,
        x: f32,
        y: f32,
        z: f32,
        polygon_index: i16,
    },
    /// Start playing a music track by song index.
    PlayMusic { song_index: i16 },
    /// Stop the current music.
    StopMusic,
    /// Set the music volume (0.0 to 1.0).
    SetMusicVolume { volume: f32 },
    /// Set the sound effects volume (0.0 to 1.0).
    SetSfxVolume { volume: f32 },
    /// Notify that a level transition is happening.
    LevelTransition,
}

/// Configuration for the audio engine.
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Maximum number of simultaneous sound channels.
    pub max_channels: usize,
    /// Initial sound effects volume (0.0 to 1.0).
    pub sfx_volume: f32,
    /// Initial music volume (0.0 to 1.0).
    pub music_volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            max_channels: 32,
            sfx_volume: 1.0,
            music_volume: 1.0,
        }
    }
}

/// Distance attenuation parameters for a sound behavior type.
/// Extracted from Marathon/Aleph One source constants.
#[derive(Debug, Clone, Copy)]
pub struct AttenuationParams {
    /// Maximum audible distance in world units.
    pub max_distance: f32,
    /// Falloff exponent (1.0 = linear, >1 = steep, <1 = gradual).
    pub falloff_exponent: f32,
}

impl AttenuationParams {
    /// Get attenuation parameters for a sound behavior type.
    ///
    /// Values based on Marathon/Aleph One source:
    /// - Quiet: ~5 WU range, steep falloff
    /// - Normal: ~10 WU range, linear falloff
    /// - Loud: ~20 WU range, gradual falloff
    ///
    /// Marathon world units: 1 WU = 1024 internal units.
    pub fn for_behavior(behavior: SoundBehavior) -> Self {
        match behavior {
            SoundBehavior::Quiet => Self {
                max_distance: 5.0 * 1024.0,
                falloff_exponent: 2.0,
            },
            SoundBehavior::Normal => Self {
                max_distance: 10.0 * 1024.0,
                falloff_exponent: 1.0,
            },
            SoundBehavior::Loud => Self {
                max_distance: 20.0 * 1024.0,
                falloff_exponent: 0.5,
            },
        }
    }
}

/// Computed spatial parameters for a sound instance.
#[derive(Debug, Clone, Copy)]
pub struct SpatialParams {
    /// Volume multiplier from distance attenuation (0.0 to 1.0).
    pub distance_volume: f32,
    /// Stereo panning (-1.0 = full left, 0.0 = center, 1.0 = full right).
    pub pan: f32,
    /// Wall obstruction factor (0.0 = no obstruction, 1.0 = full obstruction).
    pub wall_obstruction: f32,
    /// Media obstruction low-pass cutoff frequency in Hz (None = no filter).
    pub media_filter_cutoff: Option<f32>,
}

/// Snapshot of an active sound instance's state for the channel pool.
#[derive(Debug, Clone)]
pub struct SoundInstanceState {
    /// Index into the sound definitions array.
    pub sound_index: usize,
    /// The permutation that was selected for this instance.
    pub permutation: usize,
    /// Source entity identifier (for position tracking).
    pub source_entity: Option<u32>,
    /// Current world position.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Source polygon index.
    pub source_polygon: i16,
    /// Sound flags from the definition.
    pub flags: SoundFlags,
    /// Behavior type from the definition.
    pub behavior: SoundBehavior,
    /// Current effective volume (after attenuation, used for eviction priority).
    pub effective_volume: f32,
}
