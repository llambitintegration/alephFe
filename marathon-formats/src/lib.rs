pub mod error;
pub mod map;
pub mod mml;
pub mod physics;
pub mod plugin;
pub mod shapes;
pub mod sounds;
pub mod tags;
/// Test helpers for constructing synthetic Marathon binary data.
/// Available when the `test-helpers` feature is enabled or during `cargo test`.
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;
pub mod types;
pub mod wad;

pub use error::ParseError;
pub use map::{
    AmbientSoundImage, Endpoint, Line, MapAnnotation, MapData, MapInfo, MapObject, MediaData,
    ObjectFrequencyDefinition, Polygon, RandomSoundImage, Side, StaticLightData,
    StaticPlatformData,
};
pub use mml::{MmlDocument, MmlElement, MmlSection};
pub use physics::{
    AttackDefinition, EffectDefinition, MonsterDefinition, PhysicsConstants, PhysicsData,
    ProjectileDefinition, TriggerDefinition, WeaponDefinition,
};
pub use plugin::{
    MapPatch, MapResource, PluginMetadata, ScenarioRequirement, ShapesPatch, SoloLuaWriteAccess,
};
pub use shapes::{
    Bitmap, Collection, CollectionDefinition, CollectionHeader, CollectionType, ColorValue,
    HighLevelShape, LowLevelShape, LowLevelShapeFlags, ShapesFile,
};
pub use sounds::{SoundBehavior, SoundDefinition, SoundFileHeader, SoundFlags, SoundsFile};
pub use tags::WadTag;
pub use types::{DamageDefinition, ShapeDescriptor, SideTexture, WorldPoint2d, WorldPoint3d};
pub use wad::{WadEntry, WadFile, WadHeader};
