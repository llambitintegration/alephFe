use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, Decibels, Tween};
use marathon_formats::map::{AmbientSoundImage, Polygon, RandomSoundImage};
use marathon_formats::sounds::{SoundBehavior, SoundDefinition, SoundFlags};
use rand::Rng;

/// Convert a linear amplitude (0.0–1.0) to kira Decibels.
fn amplitude_to_db(amplitude: f32) -> Decibels {
    if amplitude <= 0.0 {
        Decibels(-100.0)
    } else {
        Decibels(20.0 * amplitude.log10())
    }
}

use crate::spatial::{self, distance_2d, is_within_range};
use crate::types::ListenerState;

// ─── Ambient Sound Manager ─────────────────────────────────────────────────

/// State for a single ambient sound loop tied to a polygon.
struct AmbientLoop {
    /// Sound definition index.
    sound_index: usize,
    /// Polygon center (for distance calculation).
    center_x: f32,
    center_y: f32,
    /// Sound behavior for attenuation.
    behavior: SoundBehavior,
    /// Base volume from the ambient image.
    base_volume: f32,
    /// Whether this loop is currently active (playing).
    active: bool,
    /// Kira sound handle (None when inactive).
    handle: Option<StaticSoundHandle>,
    /// Current volume level (for smooth transitions).
    current_volume: f32,
}

/// Duration of fade-in/out transitions for ambient sounds (in seconds).
const AMBIENT_FADE_DURATION: f64 = 0.5;

/// Manages ambient sound loops tied to map polygons.
pub struct AmbientManager {
    loops: Vec<AmbientLoop>,
}

impl AmbientManager {
    pub fn new() -> Self {
        Self { loops: Vec::new() }
    }

    /// Initialize ambient sounds from map data.
    ///
    /// Scans all polygons for ambient_sound_image_index references and sets up
    /// loop state for each valid one.
    pub fn init_level(
        &mut self,
        polygons: &[Polygon],
        ambient_images: &[AmbientSoundImage],
        sound_defs: &[SoundDefinition],
    ) {
        self.loops.clear();

        for polygon in polygons {
            let img_idx = polygon.ambient_sound_image_index;
            if img_idx < 0 {
                continue;
            }
            let img_idx = img_idx as usize;
            if img_idx >= ambient_images.len() {
                continue;
            }

            let image = &ambient_images[img_idx];
            let sound_idx = image.sound_index as usize;
            if sound_idx >= sound_defs.len() || sound_defs[sound_idx].is_empty() {
                continue;
            }

            let behavior = sound_defs[sound_idx].behavior();

            self.loops.push(AmbientLoop {
                sound_index: sound_idx,
                center_x: polygon.center.x as f32,
                center_y: polygon.center.y as f32,
                behavior,
                base_volume: image.volume as f32 / 256.0,
                active: false,
                handle: None,
                current_volume: 0.0,
            });
        }
    }

    /// Update ambient sounds based on listener position.
    ///
    /// Activates loops that come into range, deactivates those that go out,
    /// and adjusts volumes based on distance.
    pub fn update(
        &mut self,
        listener: &ListenerState,
        audio_manager: &mut AudioManager,
        sound_cache: &HashMap<usize, StaticSoundData>,
    ) {
        for ambient_loop in &mut self.loops {
            let distance = distance_2d(
                listener.x,
                listener.y,
                ambient_loop.center_x,
                ambient_loop.center_y,
            );

            let in_range = is_within_range(distance, ambient_loop.behavior);

            if in_range {
                let target_volume = ambient_loop.base_volume
                    * spatial::distance_attenuation(distance, ambient_loop.behavior);

                if !ambient_loop.active {
                    // Activate: start playing with fade-in
                    if let Some(sound_data) = sound_cache.get(&ambient_loop.sound_index) {
                        let data = sound_data
                            .clone()
                            .loop_region(..)
                            .volume(amplitude_to_db(target_volume))
                            .fade_in_tween(Tween {
                                duration: std::time::Duration::from_secs_f64(AMBIENT_FADE_DURATION),
                                ..Default::default()
                            });
                        if let Ok(handle) = audio_manager.play(data) {
                            ambient_loop.handle = Some(handle);
                            ambient_loop.active = true;
                            ambient_loop.current_volume = target_volume;
                        }
                    }
                } else if let Some(ref mut handle) = ambient_loop.handle {
                    // Update volume smoothly
                    if (target_volume - ambient_loop.current_volume).abs() > 0.01 {
                        handle.set_volume(
                            amplitude_to_db(target_volume),
                            Tween {
                                duration: std::time::Duration::from_millis(50),
                                ..Default::default()
                            },
                        );
                        ambient_loop.current_volume = target_volume;
                    }
                }
            } else if ambient_loop.active {
                // Deactivate: fade out and stop
                if let Some(ref mut handle) = ambient_loop.handle {
                    handle.stop(Tween {
                        duration: std::time::Duration::from_secs_f64(AMBIENT_FADE_DURATION),
                        ..Default::default()
                    });
                }
                ambient_loop.handle = None;
                ambient_loop.active = false;
                ambient_loop.current_volume = 0.0;
            }
        }
    }

    /// Stop all ambient loops and release resources.
    pub fn cleanup(&mut self) {
        for ambient_loop in &mut self.loops {
            if let Some(ref mut handle) = ambient_loop.handle {
                let _ = handle.stop(Tween::default());
            }
            ambient_loop.handle = None;
            ambient_loop.active = false;
            ambient_loop.current_volume = 0.0;
        }
        self.loops.clear();
    }
}

// ─── Random Sound Manager ──────────────────────────────────────────────────

use std::collections::HashMap;

/// State for a single random sound source tied to a polygon.
struct RandomSource {
    /// Sound definition index.
    sound_index: usize,
    /// Polygon center.
    center_x: f32,
    center_y: f32,
    /// Sound behavior for range checking.
    behavior: SoundBehavior,
    /// Sound flags from the definition.
    flags: SoundFlags,
    /// Base volume from the random image.
    base_volume: f32,
    /// Volume variance.
    delta_volume: f32,
    /// Base period (ticks).
    period: i16,
    /// Period variance.
    delta_period: i16,
    /// Base direction (Marathon angle units 0-511).
    direction: i16,
    /// Direction variance.
    delta_direction: i16,
    /// Base pitch.
    pitch: f32,
    /// Pitch variance.
    delta_pitch: f32,
    /// Whether the sound is non-directional (flag bit 0).
    non_directional: bool,
    /// Ticks remaining until next trigger.
    timer: i32,
}

/// Manages random/periodic sound sources tied to map polygons.
pub struct RandomSoundManager {
    sources: Vec<RandomSource>,
}

impl RandomSoundManager {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Initialize random sounds from map data.
    pub fn init_level(
        &mut self,
        polygons: &[Polygon],
        random_images: &[RandomSoundImage],
        sound_defs: &[SoundDefinition],
    ) {
        self.sources.clear();
        let mut rng = rand::thread_rng();

        for polygon in polygons {
            let img_idx = polygon.random_sound_image_index;
            if img_idx < 0 {
                continue;
            }
            let img_idx = img_idx as usize;
            if img_idx >= random_images.len() {
                continue;
            }

            let image = &random_images[img_idx];
            let sound_idx = image.sound_index as usize;
            if sound_idx >= sound_defs.len() || sound_defs[sound_idx].is_empty() {
                continue;
            }

            let def = &sound_defs[sound_idx];
            let initial_timer =
                image.period as i32 + rng.gen_range(0..=image.delta_period.max(0) as i32);

            self.sources.push(RandomSource {
                sound_index: sound_idx,
                center_x: polygon.center.x as f32,
                center_y: polygon.center.y as f32,
                behavior: def.behavior(),
                flags: def.sound_flags(),
                base_volume: image.volume as f32 / 256.0,
                delta_volume: image.delta_volume as f32 / 256.0,
                period: image.period,
                delta_period: image.delta_period,
                direction: image.direction,
                delta_direction: image.delta_direction,
                pitch: image.pitch,
                delta_pitch: image.delta_pitch,
                non_directional: (image.flags & 0x01) != 0,
                timer: initial_timer,
            });
        }
    }

    /// Update random sound timers and trigger sounds when ready.
    ///
    /// Returns a list of sound play requests (sound_index, x, y, volume, pitch).
    pub fn update(
        &mut self,
        listener: &ListenerState,
    ) -> Vec<RandomSoundTrigger> {
        let mut rng = rand::thread_rng();
        let mut triggers = Vec::new();

        for source in &mut self.sources {
            // Skip if out of range
            let distance = distance_2d(
                listener.x,
                listener.y,
                source.center_x,
                source.center_y,
            );
            if !is_within_range(distance, source.behavior) {
                continue;
            }

            // Decrement timer
            source.timer -= 1;
            if source.timer > 0 {
                continue;
            }

            // Reset timer for next trigger
            source.timer =
                source.period as i32 + rng.gen_range(0..=source.delta_period.max(0) as i32);

            // Compute per-playback variance
            let volume = source.base_volume + rng.gen::<f32>() * source.delta_volume;
            let pitch = source.pitch + rng.gen::<f32>() * source.delta_pitch;

            // Compute position with directional offset
            let (x, y) = if source.non_directional {
                (source.center_x, source.center_y)
            } else {
                let dir = source.direction as f32
                    + rng.gen::<f32>() * source.delta_direction as f32;
                // Marathon angle: 0-511 maps to 0-2pi
                let angle = dir * std::f32::consts::TAU / 512.0;
                // Offset a small distance from center in the direction
                let offset = 256.0; // ~0.25 world units
                (
                    source.center_x + angle.cos() * offset,
                    source.center_y + angle.sin() * offset,
                )
            };

            triggers.push(RandomSoundTrigger {
                sound_index: source.sound_index,
                x,
                y,
                volume: volume.clamp(0.0, 1.0),
                pitch,
                flags: source.flags,
            });
        }

        triggers
    }

    /// Stop all random sound timers and clear state.
    pub fn cleanup(&mut self) {
        self.sources.clear();
    }
}

/// A random sound that should be triggered this tick.
#[derive(Debug)]
pub struct RandomSoundTrigger {
    pub sound_index: usize,
    pub x: f32,
    pub y: f32,
    pub volume: f32,
    pub pitch: f32,
    pub flags: SoundFlags,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ambient_manager_new() {
        let mgr = AmbientManager::new();
        assert!(mgr.loops.is_empty());
    }

    #[test]
    fn test_ambient_cleanup() {
        let mut mgr = AmbientManager::new();
        mgr.cleanup();
        assert!(mgr.loops.is_empty());
    }

    #[test]
    fn test_random_sound_manager_new() {
        let mgr = RandomSoundManager::new();
        assert!(mgr.sources.is_empty());
    }

    #[test]
    fn test_random_sound_cleanup() {
        let mut mgr = RandomSoundManager::new();
        mgr.cleanup();
        assert!(mgr.sources.is_empty());
    }
}
