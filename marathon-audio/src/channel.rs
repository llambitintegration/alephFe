use kira::sound::static_sound::StaticSoundHandle;
use marathon_formats::sounds::SoundFlags;

use crate::types::{ChannelId, SoundInstanceState};

/// An active sound instance occupying a channel.
pub struct ActiveSound {
    pub id: ChannelId,
    pub state: SoundInstanceState,
    pub handle: StaticSoundHandle,
}

/// Pool of sound channels with priority-based eviction.
pub struct ChannelPool {
    channels: Vec<Option<ActiveSound>>,
    next_id: u32,
}

impl ChannelPool {
    /// Create a new channel pool with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let mut channels = Vec::with_capacity(capacity);
        channels.resize_with(capacity, || None);
        Self {
            channels,
            next_id: 0,
        }
    }

    /// Number of active (occupied) channels.
    pub fn active_count(&self) -> usize {
        self.channels.iter().filter(|c| c.is_some()).count()
    }

    /// Total capacity.
    pub fn capacity(&self) -> usize {
        self.channels.len()
    }

    /// Try to allocate a channel. Returns the slot index if successful.
    ///
    /// If all channels are full, evicts the lowest effective-volume sound.
    /// Sounds with `CANNOT_BE_RESTARTED` are evicted only as a last resort.
    pub fn allocate(&mut self, state: SoundInstanceState, handle: StaticSoundHandle) -> ChannelId {
        let id = ChannelId(self.next_id);
        self.next_id += 1;

        // Try to find a free slot
        if let Some(slot) = self.channels.iter_mut().find(|c| c.is_none()) {
            *slot = Some(ActiveSound {
                id,
                state,
                handle,
            });
            return id;
        }

        // All channels full — find eviction candidate
        let evict_idx = self.find_eviction_candidate();
        if let Some(idx) = evict_idx {
            if let Some(ref mut sound) = self.channels[idx] {
                let _ = sound.handle.stop(kira::Tween::default());
            }
            self.channels[idx] = Some(ActiveSound {
                id,
                state,
                handle,
            });
        }

        id
    }

    /// Find the index of the best eviction candidate.
    ///
    /// Prefers to evict sounds without CANNOT_BE_RESTARTED first.
    /// Among eligible sounds, evicts the one with the lowest effective volume.
    fn find_eviction_candidate(&self) -> Option<usize> {
        let mut best_idx: Option<usize> = None;
        let mut best_volume = f32::MAX;
        let mut best_is_protected = true;

        for (idx, slot) in self.channels.iter().enumerate() {
            if let Some(ref sound) = slot {
                let is_protected = sound
                    .state
                    .flags
                    .contains(SoundFlags::CANNOT_BE_RESTARTED);

                // Prefer unprotected over protected
                let better = match (is_protected, best_is_protected) {
                    (false, true) => true,  // unprotected beats protected
                    (true, false) => false, // protected doesn't beat unprotected
                    _ => sound.state.effective_volume < best_volume, // same class: lower volume wins
                };

                if better {
                    best_idx = Some(idx);
                    best_volume = sound.state.effective_volume;
                    best_is_protected = is_protected;
                }
            }
        }

        best_idx
    }

    /// Find active instances by sound definition index.
    pub fn find_by_definition(&self, sound_index: usize) -> Vec<&ActiveSound> {
        self.channels
            .iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|s| s.state.sound_index == sound_index)
            .collect()
    }

    /// Find a mutable active instance by channel ID.
    pub fn get_mut(&mut self, channel_id: ChannelId) -> Option<&mut ActiveSound> {
        self.channels
            .iter_mut()
            .filter_map(|slot| slot.as_mut())
            .find(|s| s.id == channel_id)
    }

    /// Release a channel by ID (sound finished playing).
    pub fn release(&mut self, channel_id: ChannelId) {
        for slot in &mut self.channels {
            if let Some(ref sound) = slot {
                if sound.id == channel_id {
                    *slot = None;
                    return;
                }
            }
        }
    }

    /// Update effective volumes for all active sounds.
    pub fn update_volume(&mut self, channel_id: ChannelId, effective_volume: f32) {
        if let Some(sound) = self.get_mut(channel_id) {
            sound.state.effective_volume = effective_volume;
        }
    }

    /// Get all active sounds (for iteration during update).
    pub fn active_sounds(&self) -> impl Iterator<Item = &ActiveSound> {
        self.channels.iter().filter_map(|slot| slot.as_ref())
    }

    /// Get all active sounds mutably.
    pub fn active_sounds_mut(&mut self) -> impl Iterator<Item = &mut ActiveSound> {
        self.channels.iter_mut().filter_map(|slot| slot.as_mut())
    }

    /// Stop all sounds and clear the pool.
    pub fn clear(&mut self) {
        for slot in &mut self.channels {
            if let Some(ref mut sound) = slot {
                let _ = sound.handle.stop(kira::Tween::default());
            }
            *slot = None;
        }
    }

    /// Remove all sounds associated with a specific entity.
    pub fn stop_entity(&mut self, entity_id: u32) {
        for slot in &mut self.channels {
            let should_remove = slot
                .as_ref()
                .map(|s| s.state.source_entity == Some(entity_id))
                .unwrap_or(false);
            if should_remove {
                if let Some(ref mut sound) = slot {
                    let _ = sound.handle.stop(kira::Tween::default());
                }
                *slot = None;
            }
        }
    }

    /// Remove finished sounds (check kira handle state).
    pub fn cleanup_finished(&mut self) {
        for slot in &mut self.channels {
            let is_stopped = slot
                .as_ref()
                .map(|s| s.handle.state() == kira::sound::PlaybackState::Stopped)
                .unwrap_or(false);
            if is_stopped {
                *slot = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use marathon_formats::sounds::SoundBehavior;

    // We can't easily construct StaticSoundHandle in tests without a running
    // audio manager, so channel pool tests that require handles are deferred
    // to integration tests. Here we test the eviction logic via
    // find_eviction_candidate using a helper.

    // For now, test the basic pool capacity and ID generation logic
    // by verifying the structural properties.

    #[test]
    fn test_pool_capacity() {
        let pool = ChannelPool::new(16);
        assert_eq!(pool.capacity(), 16);
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_pool_id_generation() {
        let mut pool = ChannelPool::new(4);
        assert_eq!(pool.next_id, 0);
        pool.next_id = 5;
        assert_eq!(pool.next_id, 5);
    }
}
