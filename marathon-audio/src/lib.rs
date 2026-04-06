pub mod ambient;
pub mod channel;
pub mod engine;
pub mod music;
pub mod spatial;
pub mod types;

// Re-export the main public API
pub use engine::{AudioEngine, AudioError};
pub use types::{AudioConfig, AudioEvent, ChannelId, ListenerState, PlaySoundRequest};
