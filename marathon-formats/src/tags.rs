use crate::types::four_chars;

/// All known WAD tag types used in Marathon/Aleph One files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WadTag {
    // Map geometry tags
    Points,
    Endpoints,
    Lines,
    Sides,
    Polygons,
    Lights,
    Annotations,
    Objects,
    MapInfo,
    Platforms,
    Media,
    AmbientSounds,
    RandomSounds,
    Terminals,
    ItemPlacement,
    GuardPaths,

    // Physics tags (Marathon 2 / Infinity)
    MonsterPhysics,
    EffectsPhysics,
    ProjectilePhysics,
    PlayerPhysics,
    WeaponsPhysics,

    // Physics tags (Marathon 1 compatibility)
    M1MonsterPhysics,
    M1EffectsPhysics,
    M1ProjectilePhysics,
    M1PlayerPhysics,
    M1WeaponsPhysics,

    // Embedded content
    ShapePatch,
    SoundPatch,
    MmlScript,
    LuaScript,

    // Save game / film tags
    Players,
    DynamicWorld,
    MapObjects,
    Doors,
    MapIndexes,
    AutomapLines,
    AutomapPolygons,
    Monsters,
    Effects,
    Projectiles,
    PlatformState,
    WeaponState,
    TerminalState,
    LuaState,
    SaveMeta,
    SaveImage,

    Unknown(u32),
}

impl From<u32> for WadTag {
    fn from(v: u32) -> Self {
        match v {
            x if x == four_chars(b'P', b'N', b'T', b'S') => Self::Points,
            x if x == four_chars(b'E', b'P', b'N', b'T') => Self::Endpoints,
            x if x == four_chars(b'L', b'I', b'N', b'S') => Self::Lines,
            x if x == four_chars(b'S', b'I', b'D', b'S') => Self::Sides,
            x if x == four_chars(b'P', b'O', b'L', b'Y') => Self::Polygons,
            x if x == four_chars(b'L', b'I', b'T', b'E') => Self::Lights,
            x if x == four_chars(b'N', b'O', b'T', b'E') => Self::Annotations,
            x if x == four_chars(b'O', b'B', b'J', b'S') => Self::Objects,
            x if x == four_chars(b'M', b'i', b'n', b'f') => Self::MapInfo,
            x if x == four_chars(b'p', b'l', b'a', b't') => Self::Platforms,
            x if x == four_chars(b'm', b'e', b'd', b'i') => Self::Media,
            x if x == four_chars(b'a', b'm', b'b', b'i') => Self::AmbientSounds,
            x if x == four_chars(b'b', b'o', b'n', b'k') => Self::RandomSounds,
            x if x == four_chars(b't', b'e', b'r', b'm') => Self::Terminals,
            x if x == four_chars(b'p', b'l', b'a', b'c') => Self::ItemPlacement,
            x if x == four_chars(b'p', 0x8C, b't', b'h') => Self::GuardPaths,

            x if x == four_chars(b'M', b'N', b'p', b'x') => Self::MonsterPhysics,
            x if x == four_chars(b'F', b'X', b'p', b'x') => Self::EffectsPhysics,
            x if x == four_chars(b'P', b'R', b'p', b'x') => Self::ProjectilePhysics,
            x if x == four_chars(b'P', b'X', b'p', b'x') => Self::PlayerPhysics,
            x if x == four_chars(b'W', b'P', b'p', b'x') => Self::WeaponsPhysics,

            x if x == four_chars(b'm', b'o', b'n', b's') => Self::M1MonsterPhysics,
            x if x == four_chars(b'e', b'f', b'f', b'e') => Self::M1EffectsPhysics,
            x if x == four_chars(b'p', b'r', b'o', b'j') => Self::M1ProjectilePhysics,
            x if x == four_chars(b'p', b'h', b'y', b's') => Self::M1PlayerPhysics,
            x if x == four_chars(b'w', b'e', b'a', b'p') => Self::M1WeaponsPhysics,

            x if x == four_chars(b'S', b'h', b'P', b'a') => Self::ShapePatch,
            x if x == four_chars(b'S', b'n', b'P', b'a') => Self::SoundPatch,
            x if x == four_chars(b'M', b'M', b'L', b'S') => Self::MmlScript,
            x if x == four_chars(b'L', b'U', b'A', b'S') => Self::LuaScript,

            x if x == four_chars(b'p', b'l', b'y', b'r') => Self::Players,
            x if x == four_chars(b'd', b'w', b'o', b'l') => Self::DynamicWorld,
            x if x == four_chars(b'm', b'o', b'b', b'j') => Self::MapObjects,
            x if x == four_chars(b'd', b'o', b'o', b'r') => Self::Doors,
            x if x == four_chars(b'i', b'i', b'd', b'x') => Self::MapIndexes,
            x if x == four_chars(b'a', b'l', b'i', b'n') => Self::AutomapLines,
            x if x == four_chars(b'a', b'p', b'o', b'l') => Self::AutomapPolygons,
            x if x == four_chars(b'm', b'O', b'n', b's') => Self::Monsters,
            x if x == four_chars(b'f', b'x', b' ', b' ') => Self::Effects,
            x if x == four_chars(b'b', b'a', b'n', b'g') => Self::Projectiles,
            x if x == four_chars(b'P', b'L', b'A', b'T') => Self::PlatformState,
            x if x == four_chars(b'w', b'e', b'a', b'P') => Self::WeaponState,
            x if x == four_chars(b'c', b'i', b'n', b't') => Self::TerminalState,
            x if x == four_chars(b's', b'l', b'u', b'a') => Self::LuaState,
            x if x == four_chars(b'S', b'M', b'E', b'T') => Self::SaveMeta,
            x if x == four_chars(b'S', b'I', b'M', b'G') => Self::SaveImage,

            _ => Self::Unknown(v),
        }
    }
}

impl From<WadTag> for u32 {
    fn from(tag: WadTag) -> u32 {
        match tag {
            WadTag::Points => four_chars(b'P', b'N', b'T', b'S'),
            WadTag::Endpoints => four_chars(b'E', b'P', b'N', b'T'),
            WadTag::Lines => four_chars(b'L', b'I', b'N', b'S'),
            WadTag::Sides => four_chars(b'S', b'I', b'D', b'S'),
            WadTag::Polygons => four_chars(b'P', b'O', b'L', b'Y'),
            WadTag::Lights => four_chars(b'L', b'I', b'T', b'E'),
            WadTag::Annotations => four_chars(b'N', b'O', b'T', b'E'),
            WadTag::Objects => four_chars(b'O', b'B', b'J', b'S'),
            WadTag::MapInfo => four_chars(b'M', b'i', b'n', b'f'),
            WadTag::Platforms => four_chars(b'p', b'l', b'a', b't'),
            WadTag::Media => four_chars(b'm', b'e', b'd', b'i'),
            WadTag::AmbientSounds => four_chars(b'a', b'm', b'b', b'i'),
            WadTag::RandomSounds => four_chars(b'b', b'o', b'n', b'k'),
            WadTag::Terminals => four_chars(b't', b'e', b'r', b'm'),
            WadTag::ItemPlacement => four_chars(b'p', b'l', b'a', b'c'),
            WadTag::GuardPaths => four_chars(b'p', 0x8C, b't', b'h'),

            WadTag::MonsterPhysics => four_chars(b'M', b'N', b'p', b'x'),
            WadTag::EffectsPhysics => four_chars(b'F', b'X', b'p', b'x'),
            WadTag::ProjectilePhysics => four_chars(b'P', b'R', b'p', b'x'),
            WadTag::PlayerPhysics => four_chars(b'P', b'X', b'p', b'x'),
            WadTag::WeaponsPhysics => four_chars(b'W', b'P', b'p', b'x'),

            WadTag::M1MonsterPhysics => four_chars(b'm', b'o', b'n', b's'),
            WadTag::M1EffectsPhysics => four_chars(b'e', b'f', b'f', b'e'),
            WadTag::M1ProjectilePhysics => four_chars(b'p', b'r', b'o', b'j'),
            WadTag::M1PlayerPhysics => four_chars(b'p', b'h', b'y', b's'),
            WadTag::M1WeaponsPhysics => four_chars(b'w', b'e', b'a', b'p'),

            WadTag::ShapePatch => four_chars(b'S', b'h', b'P', b'a'),
            WadTag::SoundPatch => four_chars(b'S', b'n', b'P', b'a'),
            WadTag::MmlScript => four_chars(b'M', b'M', b'L', b'S'),
            WadTag::LuaScript => four_chars(b'L', b'U', b'A', b'S'),

            WadTag::Players => four_chars(b'p', b'l', b'y', b'r'),
            WadTag::DynamicWorld => four_chars(b'd', b'w', b'o', b'l'),
            WadTag::MapObjects => four_chars(b'm', b'o', b'b', b'j'),
            WadTag::Doors => four_chars(b'd', b'o', b'o', b'r'),
            WadTag::MapIndexes => four_chars(b'i', b'i', b'd', b'x'),
            WadTag::AutomapLines => four_chars(b'a', b'l', b'i', b'n'),
            WadTag::AutomapPolygons => four_chars(b'a', b'p', b'o', b'l'),
            WadTag::Monsters => four_chars(b'm', b'O', b'n', b's'),
            WadTag::Effects => four_chars(b'f', b'x', b' ', b' '),
            WadTag::Projectiles => four_chars(b'b', b'a', b'n', b'g'),
            WadTag::PlatformState => four_chars(b'P', b'L', b'A', b'T'),
            WadTag::WeaponState => four_chars(b'w', b'e', b'a', b'P'),
            WadTag::TerminalState => four_chars(b'c', b'i', b'n', b't'),
            WadTag::LuaState => four_chars(b's', b'l', b'u', b'a'),
            WadTag::SaveMeta => four_chars(b'S', b'M', b'E', b'T'),
            WadTag::SaveImage => four_chars(b'S', b'I', b'M', b'G'),

            WadTag::Unknown(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags_known_roundtrip() {
        let tags = [
            WadTag::Points,
            WadTag::Endpoints,
            WadTag::Lines,
            WadTag::Polygons,
            WadTag::MonsterPhysics,
            WadTag::MmlScript,
        ];
        for tag in tags {
            let code: u32 = tag.into();
            let back = WadTag::from(code);
            assert_eq!(back, tag);
        }
    }

    #[test]
    fn test_tags_unknown_preserved() {
        let tag = WadTag::from(0xDEADBEEF);
        assert_eq!(tag, WadTag::Unknown(0xDEADBEEF));
        let code: u32 = tag.into();
        assert_eq!(code, 0xDEADBEEF);
    }
}
