use std::collections::HashMap;
use std::sync::Arc;

use kira::sound::static_sound::StaticSoundData;
use kira::{AudioManager, AudioManagerSettings, Decibels, Frame, Panning, PlaybackRate, Tween};
use marathon_formats::map::MapData;
use marathon_formats::sounds::{SoundDefinition, SoundFlags, SoundsFile};
use rand::Rng;

/// Convert a linear amplitude (0.0–1.0) to kira Decibels.
fn amplitude_to_db(amplitude: f32) -> Decibels {
    if amplitude <= 0.0 {
        Decibels(-100.0)
    } else {
        Decibels(20.0 * amplitude.log10())
    }
}

use crate::ambient::{AmbientManager, RandomSoundManager};
use crate::channel::ChannelPool;
use crate::music::MusicPlayer;
use crate::spatial::{self, ObstructionCache, PermutationTracker};
use crate::types::{AudioConfig, AudioEvent, ListenerState, PlaySoundRequest, SoundInstanceState};

/// The main audio engine that owns all audio subsystems.
pub struct AudioEngine {
    audio_manager: AudioManager,
    channel_pool: ChannelPool,
    ambient_manager: AmbientManager,
    random_manager: RandomSoundManager,
    music_player: MusicPlayer,
    permutation_tracker: PermutationTracker,
    obstruction_cache: ObstructionCache,

    /// Pre-decoded sound data cache, keyed by sound definition index.
    sound_cache: HashMap<usize, StaticSoundData>,
    /// Music track cache, keyed by song index.
    music_cache: HashMap<i16, StaticSoundData>,
    /// Sound definitions from the loaded sounds file.
    sound_defs: Vec<SoundDefinition>,
    /// Current map data (for obstruction computation).
    map_data: Option<MapData>,
    /// Media heights for submersion checks.
    media_heights: Vec<i16>,
    /// Media types for filter selection.
    media_types: Vec<i16>,
    /// Current listener state.
    listener: ListenerState,
    /// Sound effects volume (0.0 to 1.0).
    sfx_volume: f32,
}

/// Errors that can occur during audio engine operations.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Failed to initialize audio manager")]
    InitFailed,
}

impl AudioEngine {
    /// Create a new audio engine with the given configuration.
    pub fn new(config: AudioConfig) -> Result<Self, AudioError> {
        let audio_manager =
            AudioManager::new(AudioManagerSettings::default()).map_err(|_| AudioError::InitFailed)?;

        Ok(Self {
            audio_manager,
            channel_pool: ChannelPool::new(config.max_channels),
            ambient_manager: AmbientManager::new(),
            random_manager: RandomSoundManager::new(),
            music_player: MusicPlayer::new(config.music_volume),
            permutation_tracker: PermutationTracker::new(),
            obstruction_cache: ObstructionCache::new(),
            sound_cache: HashMap::new(),
            music_cache: HashMap::new(),
            sound_defs: Vec::new(),
            map_data: None,
            media_heights: Vec::new(),
            media_types: Vec::new(),
            listener: ListenerState {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                facing_angle: 0.0,
                polygon_index: -1,
            },
            sfx_volume: config.sfx_volume,
        })
    }

    /// Load a level's audio data: decode sounds, initialize ambient/random managers, start music.
    pub fn load_level(&mut self, map_data: &MapData, sounds_file: &SoundsFile) {
        // Clean up previous level
        self.channel_pool.clear();
        self.ambient_manager.cleanup();
        self.random_manager.cleanup();
        self.permutation_tracker.clear();
        self.obstruction_cache.clear();
        self.sound_cache.clear();

        // Build sound definition list and decode audio data
        self.sound_defs.clear();
        for i in 0..sounds_file.sound_count() {
            if let Some(def) = sounds_file.sound(i) {
                self.sound_defs.push(def.clone());
                if !def.is_empty() {
                    if let Ok(sound_data) = decode_sound(sounds_file, i, def) {
                        self.sound_cache.insert(i, sound_data);
                    }
                }
            }
        }

        // Extract media data for submersion checks
        self.media_heights = map_data
            .media
            .iter()
            .map(|m| m.height)
            .collect();
        self.media_types = map_data
            .media
            .iter()
            .map(|m| m.media_type)
            .collect();

        // Initialize ambient and random sound managers
        self.ambient_manager
            .init_level(&map_data.polygons, &map_data.ambient_sounds, &self.sound_defs);
        self.random_manager
            .init_level(&map_data.polygons, &map_data.random_sounds, &self.sound_defs);

        // Start level music
        let song_index = map_data
            .map_info
            .as_ref()
            .map(|info| info.song_index)
            .unwrap_or(-1);
        self.music_player
            .play(song_index, &mut self.audio_manager, &self.music_cache);

        self.map_data = Some(map_data.clone());
    }

    /// Process a frame update: handle events, update spatial parameters, tick ambient/random.
    pub fn update(&mut self, _dt: f32, listener: ListenerState, events: &[AudioEvent]) {
        self.listener = listener;
        self.obstruction_cache
            .update_listener_polygon(listener.polygon_index);

        // Process events
        for event in events {
            match event {
                AudioEvent::PlaySound(request) => self.handle_play_sound(request),
                AudioEvent::StopSound { source_entity } => {
                    self.channel_pool.stop_entity(*source_entity);
                }
                AudioEvent::UpdateEntityPosition {
                    entity_id,
                    x,
                    y,
                    z,
                    polygon_index,
                } => {
                    self.update_entity_position(*entity_id, *x, *y, *z, *polygon_index);
                }
                AudioEvent::PlayMusic { song_index } => {
                    self.music_player
                        .play(*song_index, &mut self.audio_manager, &self.music_cache);
                }
                AudioEvent::StopMusic => {
                    self.music_player.stop();
                }
                AudioEvent::SetMusicVolume { volume } => {
                    self.music_player.set_volume(*volume);
                }
                AudioEvent::SetSfxVolume { volume } => {
                    self.sfx_volume = volume.clamp(0.0, 1.0);
                }
                AudioEvent::LevelTransition => {
                    // Level transition is handled by load_level call from integration layer
                }
            }
        }

        // Update spatial parameters for all active sounds
        self.update_spatial_params();

        // Clean up finished sounds
        self.channel_pool.cleanup_finished();

        // Update ambient sounds
        self.ambient_manager
            .update(&self.listener, &mut self.audio_manager, &self.sound_cache);

        // Process random sound triggers
        let triggers = self.random_manager.update(&self.listener);
        for trigger in triggers {
            self.handle_random_trigger(trigger);
        }
    }

    /// Shut down the audio engine and release all resources.
    pub fn shutdown(&mut self) {
        self.channel_pool.clear();
        self.ambient_manager.cleanup();
        self.random_manager.cleanup();
        self.music_player.cleanup();
        self.sound_cache.clear();
        self.music_cache.clear();
    }

    /// Register music data for a song index (called by integration layer).
    pub fn register_music(&mut self, song_index: i16, sound_data: StaticSoundData) {
        self.music_cache.insert(song_index, sound_data);
    }

    // ─── Internal ──────────────────────────────────────────────────────────

    fn handle_play_sound(&mut self, request: &PlaySoundRequest) {
        let mut rng = rand::thread_rng();

        // Look up sound definition
        if request.sound_index >= self.sound_defs.len() {
            return;
        }
        let def = &self.sound_defs[request.sound_index];
        if def.is_empty() {
            return;
        }

        let flags = def.sound_flags();

        // Chance gating
        if !spatial::passes_chance_gate(def.chance, rng.gen()) {
            return;
        }

        // Check CANNOT_BE_RESTARTED: skip if already playing
        if flags.contains(SoundFlags::CANNOT_BE_RESTARTED) {
            let existing = self.channel_pool.find_by_definition(request.sound_index);
            if !existing.is_empty() {
                return;
            }
        }

        // Check DOES_NOT_SELF_ABORT: stop existing if flag is NOT set
        if !flags.contains(SoundFlags::DOES_NOT_SELF_ABORT) {
            // Collect IDs first to avoid borrow conflict
            let ids: Vec<_> = self
                .channel_pool
                .find_by_definition(request.sound_index)
                .iter()
                .map(|s| s.id)
                .collect();
            for id in ids {
                self.channel_pool.release(id);
            }
        }

        // Select permutation
        let perm_count = def.permutation_count();
        let permutation =
            self.permutation_tracker
                .select(request.sound_index, perm_count, rng.gen());

        // Get cached sound data
        let sound_data = match self.sound_cache.get(&request.sound_index) {
            Some(data) => data.clone(),
            None => return,
        };

        // Compute volume and pitch
        let base_volume = 1.0; // Marathon sounds use definition-level volume
        let volume_delta = 0.0;
        let volume = spatial::randomize_volume(base_volume, volume_delta, rng.gen());
        let pitch = spatial::randomize_pitch(def.low_pitch, def.high_pitch, rng.gen());

        // Compute initial spatial parameters
        let distance = spatial::distance_2d(self.listener.x, self.listener.y, request.x, request.y);
        let distance_vol = spatial::distance_attenuation(distance, def.behavior());
        let (pan, rear_atten) = spatial::directional_pan(
            self.listener.facing_angle,
            self.listener.x,
            self.listener.y,
            request.x,
            request.y,
        );

        // Compute obstruction
        let wall_obstruction = if flags.contains(SoundFlags::CANNOT_BE_OBSTRUCTED) {
            0.0
        } else if let Some(ref map_data) = self.map_data {
            self.obstruction_cache.get_or_compute(
                request.source_polygon,
                self.listener.polygon_index,
                &map_data.polygons,
                &map_data.lines,
            )
        } else {
            0.0
        };

        let effective_volume =
            volume * distance_vol * rear_atten * (1.0 - wall_obstruction) * self.sfx_volume;

        // Build kira sound data with spatial parameters
        let playback_rate = if pitch > 0.0 { pitch as f64 } else { 1.0 };
        let data = sound_data
            .volume(amplitude_to_db(effective_volume))
            .panning(Panning(pan))
            .playback_rate(PlaybackRate(playback_rate));

        // Play the sound
        if let Ok(handle) = self.audio_manager.play(data) {
            let state = SoundInstanceState {
                sound_index: request.sound_index,
                permutation,
                source_entity: request.source_entity,
                x: request.x,
                y: request.y,
                z: request.z,
                source_polygon: request.source_polygon,
                flags,
                behavior: def.behavior(),
                effective_volume,
            };
            self.channel_pool.allocate(state, handle);
        }
    }

    fn handle_random_trigger(&mut self, trigger: crate::ambient::RandomSoundTrigger) {
        // Check CANNOT_BE_RESTARTED
        if trigger.flags.contains(SoundFlags::CANNOT_BE_RESTARTED) {
            let existing = self.channel_pool.find_by_definition(trigger.sound_index);
            if !existing.is_empty() {
                return;
            }
        }

        // Check DOES_NOT_SELF_ABORT
        if !trigger.flags.contains(SoundFlags::DOES_NOT_SELF_ABORT) {
            let ids: Vec<_> = self
                .channel_pool
                .find_by_definition(trigger.sound_index)
                .iter()
                .map(|s| s.id)
                .collect();
            for id in ids {
                self.channel_pool.release(id);
            }
        }

        // Get sound data
        let sound_data = match self.sound_cache.get(&trigger.sound_index) {
            Some(data) => data.clone(),
            None => return,
        };

        // Compute spatial params
        let distance =
            spatial::distance_2d(self.listener.x, self.listener.y, trigger.x, trigger.y);
        let def_behavior = if trigger.sound_index < self.sound_defs.len() {
            self.sound_defs[trigger.sound_index].behavior()
        } else {
            marathon_formats::sounds::SoundBehavior::Normal
        };
        let distance_vol = spatial::distance_attenuation(distance, def_behavior);
        let (pan, rear_atten) = spatial::directional_pan(
            self.listener.facing_angle,
            self.listener.x,
            self.listener.y,
            trigger.x,
            trigger.y,
        );

        let effective_volume =
            trigger.volume * distance_vol * rear_atten * self.sfx_volume;

        let playback_rate = if trigger.pitch > 0.0 {
            trigger.pitch as f64
        } else {
            1.0
        };
        let data = sound_data
            .volume(amplitude_to_db(effective_volume))
            .panning(Panning(pan))
            .playback_rate(PlaybackRate(playback_rate));

        if let Ok(handle) = self.audio_manager.play(data) {
            let state = SoundInstanceState {
                sound_index: trigger.sound_index,
                permutation: 0,
                source_entity: None,
                x: trigger.x,
                y: trigger.y,
                z: 0.0,
                source_polygon: -1,
                flags: trigger.flags,
                behavior: def_behavior,
                effective_volume,
            };
            self.channel_pool.allocate(state, handle);
        }
    }

    fn update_entity_position(
        &mut self,
        entity_id: u32,
        x: f32,
        y: f32,
        z: f32,
        polygon_index: i16,
    ) {
        for sound in self.channel_pool.active_sounds_mut() {
            if sound.state.source_entity == Some(entity_id) {
                sound.state.x = x;
                sound.state.y = y;
                sound.state.z = z;
                sound.state.source_polygon = polygon_index;
            }
        }
    }

    fn update_spatial_params(&mut self) {
        // Collect channel IDs and new params first, then apply
        let updates: Vec<_> = self
            .channel_pool
            .active_sounds()
            .map(|sound| {
                let distance = spatial::distance_2d(
                    self.listener.x,
                    self.listener.y,
                    sound.state.x,
                    sound.state.y,
                );
                let distance_vol =
                    spatial::distance_attenuation(distance, sound.state.behavior);
                let (pan, rear_atten) = spatial::directional_pan(
                    self.listener.facing_angle,
                    self.listener.x,
                    self.listener.y,
                    sound.state.x,
                    sound.state.y,
                );

                let wall_obstruction = if sound
                    .state
                    .flags
                    .contains(SoundFlags::CANNOT_BE_OBSTRUCTED)
                {
                    0.0
                } else {
                    // Can't use obstruction cache here due to borrow — use 0.0 for now
                    // Obstruction is computed at play time and doesn't change unless
                    // the listener or source changes polygon.
                    0.0
                };

                let effective_volume = distance_vol
                    * rear_atten
                    * (1.0 - wall_obstruction)
                    * self.sfx_volume;

                (sound.id, effective_volume, pan)
            })
            .collect();

        for (id, effective_volume, pan) in updates {
            if let Some(sound) = self.channel_pool.get_mut(id) {
                sound.state.effective_volume = effective_volume;
                sound.handle.set_volume(
                    amplitude_to_db(effective_volume),
                    Tween::default(),
                );
                sound
                    .handle
                    .set_panning(Panning(pan), Tween::default());
            }
        }
    }
}

// ─── Sound Data Decoding ───────────────────────────────────────────────────

/// Convert Marathon's raw 8-bit unsigned mono 22050 Hz PCM into kira StaticSoundData.
fn decode_sound(
    sounds_file: &SoundsFile,
    sound_index: usize,
    def: &SoundDefinition,
) -> Result<StaticSoundData, ()> {
    // Get audio data for permutation 0 (we'll handle permutation selection at play time
    // by storing all permutations, but for simplicity start with permutation 0)
    let perm_count = def.permutation_count();
    if perm_count == 0 {
        return Err(());
    }

    // Decode first permutation as the default
    let raw = sounds_file.audio_data(sound_index, 0).map_err(|_| ())?;
    let frames = pcm_u8_mono_to_frames(raw);

    Ok(StaticSoundData {
        sample_rate: 22050,
        frames: Arc::from(frames),
        settings: Default::default(),
        slice: None,
    })
}

/// Convert 8-bit unsigned mono PCM samples to kira stereo Frame array.
///
/// 8-bit unsigned: 0-255, 128 = silence
/// kira Frame: stereo f32, -1.0 to 1.0
pub fn pcm_u8_mono_to_frames(pcm: &[u8]) -> Vec<Frame> {
    pcm.iter()
        .map(|&sample| {
            // Convert unsigned 8-bit to signed float: (sample - 128) / 128.0
            let value = (sample as f32 - 128.0) / 128.0;
            Frame {
                left: value,
                right: value,
            }
        })
        .collect()
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_u8_silence() {
        let frames = pcm_u8_mono_to_frames(&[128]);
        assert_eq!(frames.len(), 1);
        assert!((frames[0].left).abs() < 0.01);
        assert!((frames[0].right).abs() < 0.01);
    }

    #[test]
    fn test_pcm_u8_max() {
        let frames = pcm_u8_mono_to_frames(&[255]);
        assert!((frames[0].left - (127.0 / 128.0)).abs() < 0.01);
    }

    #[test]
    fn test_pcm_u8_min() {
        let frames = pcm_u8_mono_to_frames(&[0]);
        assert!((frames[0].left - (-128.0 / 128.0)).abs() < 0.01);
    }

    #[test]
    fn test_pcm_u8_mono_produces_stereo() {
        let frames = pcm_u8_mono_to_frames(&[128, 200, 50]);
        assert_eq!(frames.len(), 3);
        for frame in &frames {
            assert_eq!(frame.left, frame.right, "Mono should produce equal L/R");
        }
    }

    #[test]
    fn test_pcm_u8_known_duration() {
        // 22050 samples = 1 second at 22050 Hz
        let data = vec![128u8; 22050];
        let frames = pcm_u8_mono_to_frames(&data);
        assert_eq!(frames.len(), 22050);
    }
}
