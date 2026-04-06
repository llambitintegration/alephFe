use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, Decibels, Tween};
use std::collections::HashMap;
use std::time::Duration;

/// Convert a linear amplitude (0.0–1.0) to kira Decibels.
fn amplitude_to_db(amplitude: f32) -> Decibels {
    if amplitude <= 0.0 {
        Decibels(-100.0)
    } else {
        Decibels(20.0 * amplitude.log10())
    }
}

/// Duration of music crossfade transitions.
const CROSSFADE_DURATION: Duration = Duration::from_secs(2);

/// Manages background music playback on a dedicated channel.
pub struct MusicPlayer {
    /// Currently playing music handle.
    current_handle: Option<StaticSoundHandle>,
    /// Currently playing song index.
    current_song: Option<i16>,
    /// Music volume (0.0 to 1.0).
    volume: f32,
}

impl MusicPlayer {
    pub fn new(volume: f32) -> Self {
        Self {
            current_handle: None,
            current_song: None,
            volume,
        }
    }

    /// Start playing a music track by song index.
    ///
    /// If a track is already playing, crossfades to the new one.
    /// If `song_index` is -1 or invalid, stops music.
    pub fn play(
        &mut self,
        song_index: i16,
        audio_manager: &mut AudioManager,
        music_cache: &HashMap<i16, StaticSoundData>,
    ) {
        // Same song already playing — do nothing
        if self.current_song == Some(song_index) && self.current_handle.is_some() {
            return;
        }

        // Fade out current track
        self.fade_out_current();

        // Invalid song index — just stop
        if song_index < 0 {
            self.current_song = None;
            return;
        }

        // Start new track
        if let Some(sound_data) = music_cache.get(&song_index) {
            let data = sound_data
                .clone()
                .loop_region(..)
                .volume(amplitude_to_db(self.volume))
                .fade_in_tween(Tween {
                    duration: CROSSFADE_DURATION,
                    ..Default::default()
                });
            if let Ok(handle) = audio_manager.play(data) {
                self.current_handle = Some(handle);
                self.current_song = Some(song_index);
            }
        } else {
            self.current_song = None;
        }
    }

    /// Stop the current music with a fade out.
    pub fn stop(&mut self) {
        self.fade_out_current();
        self.current_song = None;
    }

    /// Set the music volume (0.0 to 1.0).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(ref mut handle) = self.current_handle {
            handle.set_volume(
                amplitude_to_db(self.volume),
                Tween {
                    duration: Duration::from_millis(50),
                    ..Default::default()
                },
            );
        }
    }

    /// Handle level transition: crossfade to new song or stop if no music.
    pub fn on_level_transition(
        &mut self,
        new_song_index: i16,
        audio_manager: &mut AudioManager,
        music_cache: &HashMap<i16, StaticSoundData>,
    ) {
        self.play(new_song_index, audio_manager, music_cache);
    }

    /// Get the currently playing song index.
    pub fn current_song(&self) -> Option<i16> {
        self.current_song
    }

    /// Fade out and release the current track.
    fn fade_out_current(&mut self) {
        if let Some(ref mut handle) = self.current_handle {
            handle.stop(Tween {
                duration: CROSSFADE_DURATION,
                ..Default::default()
            });
        }
        self.current_handle = None;
    }

    /// Clean up all music resources.
    pub fn cleanup(&mut self) {
        self.fade_out_current();
        self.current_song = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_music_player_new() {
        let player = MusicPlayer::new(0.8);
        assert_eq!(player.volume, 0.8);
        assert_eq!(player.current_song(), None);
    }

    #[test]
    fn test_music_player_set_volume_clamped() {
        let mut player = MusicPlayer::new(0.5);
        player.set_volume(1.5);
        assert_eq!(player.volume, 1.0);
        player.set_volume(-0.5);
        assert_eq!(player.volume, 0.0);
    }

    #[test]
    fn test_music_player_cleanup() {
        let mut player = MusicPlayer::new(1.0);
        player.current_song = Some(5);
        player.cleanup();
        assert_eq!(player.current_song(), None);
    }
}
