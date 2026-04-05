use std::io::Cursor;

use binrw::BinRead;
use bitflags::bitflags;

use crate::error::MapError;
use crate::tags::WadTag;
use crate::types::{fixed_to_f32, ShapeDescriptor, SideTexture, WorldPoint2d, WorldPoint3d};
use crate::wad::WadEntry;

// Struct sizes for tag data length validation
const ENDPOINT_SIZE: usize = 16;
const POINT_SIZE: usize = 4;
const LINE_SIZE: usize = 32;
const SIDE_SIZE: usize = 64;
const POLYGON_SIZE: usize = 128;
const MAP_OBJECT_SIZE: usize = 16;
const STATIC_LIGHT_SIZE: usize = 100;
const OLD_LIGHT_SIZE: usize = 32;
const PLATFORM_SIZE: usize = 32;
const MEDIA_SIZE: usize = 32;
const ANNOTATION_SIZE: usize = 72;
const AMBIENT_SOUND_SIZE: usize = 16;
const RANDOM_SOUND_SIZE: usize = 32;
const MAP_INFO_SIZE: usize = 88;
const ITEM_PLACEMENT_SIZE: usize = 12;

fn bytes_to_string(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

fn validate_tag_length(
    tag_name: &str,
    data_len: usize,
    struct_size: usize,
) -> Result<(), MapError> {
    if !data_len.is_multiple_of(struct_size) {
        return Err(MapError::InvalidTagLength {
            tag: tag_name.to_string(),
            length: data_len,
            struct_size,
        });
    }
    Ok(())
}

fn parse_array<T: for<'a> BinRead<Args<'a> = ()> + binrw::meta::ReadEndian>(
    data: &[u8],
    struct_size: usize,
) -> Result<Vec<T>, MapError> {
    let count = data.len() / struct_size;
    let mut result = Vec::with_capacity(count);
    let mut cursor = Cursor::new(data);
    for _ in 0..count {
        result.push(T::read(&mut cursor)?);
    }
    Ok(result)
}

// =============================================
// Enums
// =============================================

/// Polygon type discriminator (24 variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonType {
    Normal,
    ItemImpassable,
    MonsterImpassable,
    Hill,
    Base,
    Platform,
    LightOnTrigger,
    PlatformOnTrigger,
    LightOffTrigger,
    PlatformOffTrigger,
    Teleporter,
    ZoneBorder,
    Goal,
    VisibleMonsterTrigger,
    InvisibleMonsterTrigger,
    DualMonsterTrigger,
    ItemTrigger,
    MustBeExplored,
    AutomaticExit,
    MinorOuch,
    MajorOuch,
    Glue,
    GlueTrigger,
    Superglue,
    Unknown(i16),
}

impl From<i16> for PolygonType {
    fn from(v: i16) -> Self {
        match v {
            0 => Self::Normal,
            1 => Self::ItemImpassable,
            2 => Self::MonsterImpassable,
            3 => Self::Hill,
            4 => Self::Base,
            5 => Self::Platform,
            6 => Self::LightOnTrigger,
            7 => Self::PlatformOnTrigger,
            8 => Self::LightOffTrigger,
            9 => Self::PlatformOffTrigger,
            10 => Self::Teleporter,
            11 => Self::ZoneBorder,
            12 => Self::Goal,
            13 => Self::VisibleMonsterTrigger,
            14 => Self::InvisibleMonsterTrigger,
            15 => Self::DualMonsterTrigger,
            16 => Self::ItemTrigger,
            17 => Self::MustBeExplored,
            18 => Self::AutomaticExit,
            19 => Self::MinorOuch,
            20 => Self::MajorOuch,
            21 => Self::Glue,
            22 => Self::GlueTrigger,
            23 => Self::Superglue,
            _ => Self::Unknown(v),
        }
    }
}

/// Map object type discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectType {
    Monster,
    Scenery,
    Item,
    Player,
    Goal,
    SoundSource,
    Unknown(i16),
}

impl From<i16> for MapObjectType {
    fn from(v: i16) -> Self {
        match v {
            0 => Self::Monster,
            1 => Self::Scenery,
            2 => Self::Item,
            3 => Self::Player,
            4 => Self::Goal,
            5 => Self::SoundSource,
            _ => Self::Unknown(v),
        }
    }
}

/// Side type (wall surface type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideType {
    Full,
    High,
    Low,
    Composite,
    Split,
    Unknown(i16),
}

impl From<i16> for SideType {
    fn from(v: i16) -> Self {
        match v {
            0 => Self::Full,
            1 => Self::High,
            2 => Self::Low,
            3 => Self::Composite,
            4 => Self::Split,
            _ => Self::Unknown(v),
        }
    }
}

/// Media (liquid) type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaTypeEnum {
    Water,
    Lava,
    Goo,
    Sewage,
    Jjaro,
    Unknown(i16),
}

impl From<i16> for MediaTypeEnum {
    fn from(v: i16) -> Self {
        match v {
            0 => Self::Water,
            1 => Self::Lava,
            2 => Self::Goo,
            3 => Self::Sewage,
            4 => Self::Jjaro,
            _ => Self::Unknown(v),
        }
    }
}

// =============================================
// Bitflags
// =============================================

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LineFlags: u16 {
        const SOLID = 0x4000;
        const TRANSPARENT = 0x2000;
        const LANDSCAPE = 0x1000;
        const ELEVATION = 0x0800;
        const VARIABLE_ELEVATION = 0x0400;
        const HAS_TRANSPARENT_SIDE = 0x0200;
        const DECORATIVE = 0x0100;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SideFlags: u16 {
        const CONTROL_PANEL_STATUS = 0x0001;
        const IS_CONTROL_PANEL = 0x0002;
        const IS_REPAIR_SWITCH = 0x0004;
        const IS_DESTRUCTIVE_SWITCH = 0x0008;
        const IS_LIGHTED_SWITCH = 0x0010;
        const SWITCH_CAN_BE_DESTROYED = 0x0020;
        const SWITCH_CAN_ONLY_BE_HIT_BY_PROJECTILES = 0x0040;
        const ITEM_IS_OPTIONAL = 0x0080;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MapObjectFlags: u16 {
        const INVISIBLE = 0x0001;
        const HANGING_FROM_CEILING = 0x0002;
        const BLIND = 0x0004;
        const DEAF = 0x0008;
        const FLOATS = 0x0010;
        const NETWORK_ONLY = 0x0020;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MissionFlags: i16 {
        const EXTERMINATION = 0x0001;
        const EXPLORATION = 0x0002;
        const RETRIEVAL = 0x0004;
        const REPAIR = 0x0008;
        const RESCUE = 0x0010;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EnvironmentFlags: i16 {
        const VACUUM = 0x0001;
        const MAGNETIC = 0x0002;
        const REBELLION = 0x0004;
        const LOW_GRAVITY = 0x0008;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EntryPointFlags: u32 {
        const SINGLE_PLAYER = 0x01;
        const MULTIPLAYER_COOPERATIVE = 0x02;
        const MULTIPLAYER_CARNAGE = 0x04;
        const KILL_THE_MAN_WITH_THE_BALL = 0x08;
        const KING_OF_THE_HILL = 0x10;
        const DEFENSE = 0x20;
        const RUGBY = 0x40;
        const CAPTURE_THE_FLAG = 0x80;
    }
}

// =============================================
// Endpoint (EPNT 16 bytes / PNTS 4 bytes)
// =============================================

/// Endpoint/vertex data from EPNT tag (16 bytes) or derived from legacy PNTS tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct Endpoint {
    pub flags: u16,
    pub highest_adjacent_floor_height: i16,
    pub lowest_adjacent_ceiling_height: i16,
    pub vertex: WorldPoint2d,
    pub transformed: WorldPoint2d,
    pub supporting_polygon_index: i16,
}

impl Endpoint {
    /// Create an endpoint from legacy PNTS data (4 bytes: x, y).
    pub fn from_point(x: i16, y: i16) -> Self {
        Self {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 0,
            vertex: WorldPoint2d { x, y },
            transformed: WorldPoint2d { x, y },
            supporting_polygon_index: -1,
        }
    }
}

// =============================================
// Line (LINS 32 bytes)
// =============================================

/// Line (edge) data from LINS tag (32 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct Line {
    pub endpoint_indexes: [i16; 2],
    pub flags: u16,
    pub length: i16,
    pub highest_adjacent_floor: i16,
    pub lowest_adjacent_ceiling: i16,
    pub clockwise_polygon_side_index: i16,
    pub counterclockwise_polygon_side_index: i16,
    pub clockwise_polygon_owner: i16,
    #[br(pad_after = 12)] // 6 unused i16
    pub counterclockwise_polygon_owner: i16,
}

impl Line {
    pub fn line_flags(&self) -> LineFlags {
        LineFlags::from_bits_truncate(self.flags)
    }
}

// =============================================
// Side (SIDS 64 bytes)
// =============================================

/// Side (wall surface) data from SIDS tag (64 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct Side {
    pub side_type: i16,
    pub flags: u16,
    pub primary_texture: SideTexture,
    pub secondary_texture: SideTexture,
    pub transparent_texture: SideTexture,
    pub exclusion_zone: [WorldPoint2d; 4], // runtime-only
    pub control_panel_type: i16,
    pub control_panel_permutation: i16,
    pub primary_transfer_mode: i16,
    pub secondary_transfer_mode: i16,
    pub transparent_transfer_mode: i16,
    pub polygon_index: i16,
    pub line_index: i16,
    pub primary_lightsource_index: i16,
    pub secondary_lightsource_index: i16,
    pub transparent_lightsource_index: i16,
    #[br(pad_after = 2)] // 1 unused i16
    pub ambient_delta: i32,
}

impl Side {
    pub fn side_flags(&self) -> SideFlags {
        SideFlags::from_bits_truncate(self.flags)
    }

    pub fn side_type_enum(&self) -> SideType {
        SideType::from(self.side_type)
    }
}

// =============================================
// Polygon (POLY 128 bytes)
// =============================================

/// Polygon (room/space) data from POLY tag (128 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct Polygon {
    pub polygon_type: i16,
    pub flags: u16,
    pub permutation: i16,
    pub vertex_count: u16,
    pub endpoint_indexes: [i16; 8],
    pub line_indexes: [i16; 8],
    pub floor_texture: ShapeDescriptor,
    pub ceiling_texture: ShapeDescriptor,
    pub floor_height: i16,
    pub ceiling_height: i16,
    pub floor_lightsource_index: i16,
    pub ceiling_lightsource_index: i16,
    // pad 8: first_object, first_exclusion_zone_index, line/point_exclusion_zone_count (runtime)
    #[br(pad_after = 8)]
    pub area: i32,
    pub floor_transfer_mode: i16,
    pub ceiling_transfer_mode: i16,
    // pad 4: first_neighbor_index, neighbor_count (runtime)
    #[br(pad_after = 4)]
    pub adjacent_polygon_indexes: [i16; 8],
    pub center: WorldPoint2d,
    pub side_indexes: [i16; 8],
    pub floor_origin: WorldPoint2d,
    pub ceiling_origin: WorldPoint2d,
    pub media_index: i16,
    pub media_lightsource_index: i16,
    pub sound_source_indexes: i16,
    pub ambient_sound_image_index: i16,
    #[br(pad_after = 2)] // 1 unused i16
    pub random_sound_image_index: i16,
}

impl Polygon {
    pub fn polygon_type_enum(&self) -> PolygonType {
        PolygonType::from(self.polygon_type)
    }
}

// =============================================
// MapObject (OBJS 16 bytes)
// =============================================

/// Placed map object from OBJS tag (16 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct MapObject {
    pub object_type: i16,
    pub index: i16,
    pub facing: i16,
    pub polygon_index: i16,
    pub location: WorldPoint3d,
    pub flags: u16,
}

impl MapObject {
    pub fn object_type_enum(&self) -> MapObjectType {
        MapObjectType::from(self.object_type)
    }

    pub fn object_flags(&self) -> MapObjectFlags {
        MapObjectFlags::from_bits_truncate(self.flags)
    }

    /// Top 4 bits (12-15) are the activation bias for monsters.
    pub fn activation_bias(&self) -> u8 {
        ((self.flags >> 12) & 0x0F) as u8
    }
}

// =============================================
// Light (LITE 100 bytes new / 32 bytes old)
// =============================================

/// Lighting function specification (14 bytes), used in StaticLightData.
#[derive(Debug, Clone, Copy, PartialEq, BinRead)]
#[br(big)]
pub struct LightingFunctionSpec {
    pub function: i16,
    pub period: i16,
    pub delta_period: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub intensity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub delta_intensity: f32,
}

/// Marathon 2/Infinity static light data from LITE tag (100 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, BinRead)]
#[br(big)]
pub struct StaticLightData {
    pub light_type: i16,
    pub flags: u16,
    pub phase: i16,
    pub primary_active: LightingFunctionSpec,
    pub secondary_active: LightingFunctionSpec,
    pub becoming_active: LightingFunctionSpec,
    pub primary_inactive: LightingFunctionSpec,
    pub secondary_inactive: LightingFunctionSpec,
    pub becoming_inactive: LightingFunctionSpec,
    #[br(pad_after = 8)] // 4 unused i16
    pub tag: i16,
}

/// Marathon 1 old light data from LITE tag (32 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, BinRead)]
#[br(big)]
pub struct OldLightData {
    pub flags: u16,
    pub light_type: i16,
    pub mode: i16,
    pub phase: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub minimum_intensity: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub maximum_intensity: f32,
    pub period: i16,
    #[br(map = |v: i32| fixed_to_f32(v), pad_after = 10)] // 5 unused i16
    pub intensity: f32,
}

/// Parsed light data, either new or old format.
#[derive(Debug, Clone)]
pub enum LightData {
    Static(Vec<StaticLightData>),
    Old(Vec<OldLightData>),
    None,
}

// =============================================
// Platform (plat 32 bytes)
// =============================================

/// Static platform (elevator/door) data from plat tag (32 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct StaticPlatformData {
    pub platform_type: i16,
    pub speed: i16,
    pub delay: i16,
    pub maximum_height: i16,
    pub minimum_height: i16,
    pub static_flags: u32,
    pub polygon_index: i16,
    #[br(pad_after = 14)] // 7 unused i16
    pub tag: i16,
}

// =============================================
// Media (medi 32 bytes)
// =============================================

/// Media (liquid) data from medi tag (32 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, BinRead)]
#[br(big)]
pub struct MediaData {
    pub media_type: i16,
    pub flags: u16,
    pub light_index: i16,
    pub current_direction: i16,
    pub current_magnitude: i16,
    pub low: i16,
    pub high: i16,
    pub origin: WorldPoint2d,
    pub height: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub minimum_light_intensity: f32,
    pub texture: ShapeDescriptor,
    #[br(pad_after = 4)] // 2 unused i16
    pub transfer_mode: i16,
}

impl MediaData {
    pub fn media_type_enum(&self) -> MediaTypeEnum {
        MediaTypeEnum::from(self.media_type)
    }
}

// =============================================
// Annotation (NOTE 72 bytes)
// =============================================

/// Map annotation from NOTE tag (72 bytes per record).
#[derive(Debug, Clone, PartialEq, BinRead)]
#[br(big)]
pub struct MapAnnotation {
    pub annotation_type: i16,
    pub location: WorldPoint2d,
    pub polygon_index: i16,
    #[br(count = 64, map = |b: Vec<u8>| bytes_to_string(&b))]
    pub text: String,
}

// =============================================
// Ambient Sound (ambi 16 bytes)
// =============================================

/// Ambient sound image from ambi tag (16 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct AmbientSoundImage {
    pub flags: u16,
    pub sound_index: i16,
    #[br(pad_after = 10)] // 5 unused i16
    pub volume: i16,
}

// =============================================
// Random Sound (bonk 32 bytes)
// =============================================

/// Random sound image from bonk tag (32 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, BinRead)]
#[br(big)]
pub struct RandomSoundImage {
    pub flags: u16,
    pub sound_index: i16,
    pub volume: i16,
    pub delta_volume: i16,
    pub period: i16,
    pub delta_period: i16,
    pub direction: i16,
    pub delta_direction: i16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub pitch: f32,
    #[br(map = |v: i32| fixed_to_f32(v), pad_after = 8)] // phase (runtime) + 3 unused i16
    pub delta_pitch: f32,
}

impl RandomSoundImage {
    /// Bit 0: non-directional flag (direction field is ignored).
    pub fn is_non_directional(&self) -> bool {
        self.flags & 0x0001 != 0
    }
}

// =============================================
// MapInfo (Minf 88 bytes)
// =============================================

/// Static map metadata from Minf tag (88 bytes).
#[derive(Debug, Clone, PartialEq, BinRead)]
#[br(big)]
pub struct MapInfo {
    pub environment_code: i16,
    pub physics_model: i16,
    pub song_index: i16,
    pub mission_flags: i16,
    pub environment_flags: i16,
    #[br(pad_after = 7)] // 1 unused byte + 3 unused i16
    pub ball_in_play: u8,
    #[br(count = 66, map = |b: Vec<u8>| bytes_to_string(&b))]
    pub level_name: String,
    pub entry_point_flags: u32,
}

impl MapInfo {
    pub fn mission_flags_bits(&self) -> MissionFlags {
        MissionFlags::from_bits_truncate(self.mission_flags)
    }

    pub fn environment_flags_bits(&self) -> EnvironmentFlags {
        EnvironmentFlags::from_bits_truncate(self.environment_flags)
    }

    pub fn entry_point_flags_bits(&self) -> EntryPointFlags {
        EntryPointFlags::from_bits_truncate(self.entry_point_flags)
    }
}

// =============================================
// Item Placement (plac 12 bytes)
// =============================================

/// Object frequency definition from plac tag (12 bytes per record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct ObjectFrequencyDefinition {
    pub flags: u16,
    pub initial_count: i16,
    pub minimum_count: i16,
    pub maximum_count: i16,
    pub random_count: i16,
    pub random_chance: u16,
}

impl ObjectFrequencyDefinition {
    /// Bit 0: whether the object reappears in a random location.
    pub fn reappears_randomly(&self) -> bool {
        self.flags & 0x0001 != 0
    }
}

// =============================================
// Terminal (term, variable length)
// =============================================

/// Parsed terminal data from a term tag entry.
#[derive(Debug, Clone)]
pub struct TerminalData {
    pub flags: i16,
    pub lines_per_page: i16,
    pub groupings: Vec<TerminalGrouping>,
    pub font_changes: Vec<TerminalFontChange>,
    pub text: Vec<u8>,
}

/// Terminal grouping record (12 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct TerminalGrouping {
    pub flags: i16,
    pub grouping_type: i16,
    pub permutation: i16,
    pub start_index: i16,
    pub length: i16,
    pub maximum_line_count: i16,
}

/// Terminal font change record (6 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, BinRead)]
#[br(big)]
pub struct TerminalFontChange {
    pub index: i16,
    pub face: i16,
    pub color: i16,
}

fn parse_terminals(data: &[u8]) -> Result<Vec<TerminalData>, MapError> {
    let mut terminals = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        if offset + 10 > data.len() {
            break;
        }

        let mut cursor = Cursor::new(&data[offset..]);
        let total_length = i16::read_be(&mut cursor)? as usize;
        let flags = i16::read_be(&mut cursor)?;
        let lines_per_page = i16::read_be(&mut cursor)?;
        let grouping_count = i16::read_be(&mut cursor)? as usize;
        let font_changes_count = i16::read_be(&mut cursor)? as usize;

        let mut groupings = Vec::with_capacity(grouping_count);
        for _ in 0..grouping_count {
            groupings.push(TerminalGrouping::read(&mut cursor)?);
        }

        let mut font_changes = Vec::with_capacity(font_changes_count);
        for _ in 0..font_changes_count {
            font_changes.push(TerminalFontChange::read(&mut cursor)?);
        }

        let consumed = 10 + grouping_count * 12 + font_changes_count * 6;
        let terminal_end = offset + total_length.min(data.len() - offset);
        let text = if consumed < total_length && offset + consumed < data.len() {
            data[offset + consumed..terminal_end].to_vec()
        } else {
            Vec::new()
        };

        terminals.push(TerminalData {
            flags,
            lines_per_page,
            groupings,
            font_changes,
            text,
        });

        if total_length == 0 {
            break;
        }
        offset += total_length;
    }

    Ok(terminals)
}

// =============================================
// MapData - convenience aggregate
// =============================================

/// All parsed map data from a single WAD entry.
#[derive(Debug, Clone)]
pub struct MapData {
    pub endpoints: Vec<Endpoint>,
    pub lines: Vec<Line>,
    pub sides: Vec<Side>,
    pub polygons: Vec<Polygon>,
    pub objects: Vec<MapObject>,
    pub lights: LightData,
    pub platforms: Vec<StaticPlatformData>,
    pub media: Vec<MediaData>,
    pub annotations: Vec<MapAnnotation>,
    pub terminals: Vec<TerminalData>,
    pub ambient_sounds: Vec<AmbientSoundImage>,
    pub random_sounds: Vec<RandomSoundImage>,
    pub map_info: Option<MapInfo>,
    pub item_placement: Vec<ObjectFrequencyDefinition>,
    pub guard_paths: Option<Vec<u8>>,
}

impl MapData {
    /// Parse all known map tags from a WAD entry.
    /// PNTS tag takes precedence over EPNT when both are present.
    pub fn from_entry(entry: &WadEntry) -> Result<Self, MapError> {
        // Endpoints: PNTS takes precedence over EPNT
        let endpoints = if let Some(pnts_data) = entry.get_tag_data(WadTag::Points) {
            validate_tag_length("PNTS", pnts_data.len(), POINT_SIZE)?;
            let count = pnts_data.len() / POINT_SIZE;
            let mut eps = Vec::with_capacity(count);
            let mut cursor = Cursor::new(pnts_data);
            for _ in 0..count {
                let p = WorldPoint2d::read(&mut cursor)?;
                eps.push(Endpoint::from_point(p.x, p.y));
            }
            eps
        } else if let Some(epnt_data) = entry.get_tag_data(WadTag::Endpoints) {
            validate_tag_length("EPNT", epnt_data.len(), ENDPOINT_SIZE)?;
            parse_array::<Endpoint>(epnt_data, ENDPOINT_SIZE)?
        } else {
            Vec::new()
        };

        let lines = if let Some(data) = entry.get_tag_data(WadTag::Lines) {
            validate_tag_length("LINS", data.len(), LINE_SIZE)?;
            parse_array::<Line>(data, LINE_SIZE)?
        } else {
            Vec::new()
        };

        let sides = if let Some(data) = entry.get_tag_data(WadTag::Sides) {
            validate_tag_length("SIDS", data.len(), SIDE_SIZE)?;
            parse_array::<Side>(data, SIDE_SIZE)?
        } else {
            Vec::new()
        };

        let polygons = if let Some(data) = entry.get_tag_data(WadTag::Polygons) {
            validate_tag_length("POLY", data.len(), POLYGON_SIZE)?;
            parse_array::<Polygon>(data, POLYGON_SIZE)?
        } else {
            Vec::new()
        };

        let objects = if let Some(data) = entry.get_tag_data(WadTag::Objects) {
            validate_tag_length("OBJS", data.len(), MAP_OBJECT_SIZE)?;
            parse_array::<MapObject>(data, MAP_OBJECT_SIZE)?
        } else {
            Vec::new()
        };

        // Detect light format by data length divisibility
        let lights = if let Some(data) = entry.get_tag_data(WadTag::Lights) {
            if data.len() % STATIC_LIGHT_SIZE == 0 {
                LightData::Static(parse_array::<StaticLightData>(data, STATIC_LIGHT_SIZE)?)
            } else if data.len() % OLD_LIGHT_SIZE == 0 {
                LightData::Old(parse_array::<OldLightData>(data, OLD_LIGHT_SIZE)?)
            } else {
                return Err(MapError::InvalidTagLength {
                    tag: "LITE".to_string(),
                    length: data.len(),
                    struct_size: STATIC_LIGHT_SIZE,
                });
            }
        } else {
            LightData::None
        };

        let platforms = if let Some(data) = entry.get_tag_data(WadTag::Platforms) {
            validate_tag_length("plat", data.len(), PLATFORM_SIZE)?;
            parse_array::<StaticPlatformData>(data, PLATFORM_SIZE)?
        } else {
            Vec::new()
        };

        let media = if let Some(data) = entry.get_tag_data(WadTag::Media) {
            validate_tag_length("medi", data.len(), MEDIA_SIZE)?;
            parse_array::<MediaData>(data, MEDIA_SIZE)?
        } else {
            Vec::new()
        };

        let annotations = if let Some(data) = entry.get_tag_data(WadTag::Annotations) {
            validate_tag_length("NOTE", data.len(), ANNOTATION_SIZE)?;
            parse_array::<MapAnnotation>(data, ANNOTATION_SIZE)?
        } else {
            Vec::new()
        };

        let terminals = if let Some(data) = entry.get_tag_data(WadTag::Terminals) {
            parse_terminals(data)?
        } else {
            Vec::new()
        };

        let ambient_sounds = if let Some(data) = entry.get_tag_data(WadTag::AmbientSounds) {
            validate_tag_length("ambi", data.len(), AMBIENT_SOUND_SIZE)?;
            parse_array::<AmbientSoundImage>(data, AMBIENT_SOUND_SIZE)?
        } else {
            Vec::new()
        };

        let random_sounds = if let Some(data) = entry.get_tag_data(WadTag::RandomSounds) {
            validate_tag_length("bonk", data.len(), RANDOM_SOUND_SIZE)?;
            parse_array::<RandomSoundImage>(data, RANDOM_SOUND_SIZE)?
        } else {
            Vec::new()
        };

        let map_info = if let Some(data) = entry.get_tag_data(WadTag::MapInfo) {
            validate_tag_length("Minf", data.len(), MAP_INFO_SIZE)?;
            let mut cursor = Cursor::new(data);
            Some(MapInfo::read(&mut cursor)?)
        } else {
            None
        };

        let item_placement = if let Some(data) = entry.get_tag_data(WadTag::ItemPlacement) {
            validate_tag_length("plac", data.len(), ITEM_PLACEMENT_SIZE)?;
            parse_array::<ObjectFrequencyDefinition>(data, ITEM_PLACEMENT_SIZE)?
        } else {
            Vec::new()
        };

        let guard_paths = entry.get_tag_data(WadTag::GuardPaths).map(|d| d.to_vec());

        Ok(Self {
            endpoints,
            lines,
            sides,
            polygons,
            objects,
            lights,
            platforms,
            media,
            annotations,
            terminals,
            ambient_sounds,
            random_sounds,
            map_info,
            item_placement,
            guard_paths,
        })
    }

    /// Validate cross-references between geometry structures.
    /// Returns a list of validation errors without aborting.
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let ep_count = self.endpoints.len();
        let line_count = self.lines.len();
        let poly_count = self.polygons.len();
        let side_count = self.sides.len();

        // Polygon references
        for (pi, poly) in self.polygons.iter().enumerate() {
            let vc = (poly.vertex_count as usize).min(8);
            for i in 0..vc {
                let ep_idx = poly.endpoint_indexes[i];
                if ep_idx != -1 && (ep_idx < 0 || ep_idx as usize >= ep_count) {
                    errors.push(ValidationError {
                        message: format!(
                            "polygon {} endpoint_indexes[{}]={} out of range (0..{})",
                            pi, i, ep_idx, ep_count
                        ),
                    });
                }

                let ln_idx = poly.line_indexes[i];
                if ln_idx != -1 && (ln_idx < 0 || ln_idx as usize >= line_count) {
                    errors.push(ValidationError {
                        message: format!(
                            "polygon {} line_indexes[{}]={} out of range (0..{})",
                            pi, i, ln_idx, line_count
                        ),
                    });
                }

                let adj_idx = poly.adjacent_polygon_indexes[i];
                if adj_idx != -1 && (adj_idx < 0 || adj_idx as usize >= poly_count) {
                    errors.push(ValidationError {
                        message: format!(
                            "polygon {} adjacent_polygon_indexes[{}]={} out of range (0..{})",
                            pi, i, adj_idx, poly_count
                        ),
                    });
                }

                let side_idx = poly.side_indexes[i];
                if side_idx != -1 && (side_idx < 0 || side_idx as usize >= side_count) {
                    errors.push(ValidationError {
                        message: format!(
                            "polygon {} side_indexes[{}]={} out of range (0..{})",
                            pi, i, side_idx, side_count
                        ),
                    });
                }
            }
        }

        // Line references
        for (li, line) in self.lines.iter().enumerate() {
            for (ei, &ep_idx) in line.endpoint_indexes.iter().enumerate() {
                if ep_idx < 0 || ep_idx as usize >= ep_count {
                    errors.push(ValidationError {
                        message: format!(
                            "line {} endpoint_indexes[{}]={} out of range (0..{})",
                            li, ei, ep_idx, ep_count
                        ),
                    });
                }
            }

            if line.clockwise_polygon_owner != -1
                && (line.clockwise_polygon_owner < 0
                    || line.clockwise_polygon_owner as usize >= poly_count)
            {
                errors.push(ValidationError {
                    message: format!(
                        "line {} clockwise_polygon_owner={} out of range (0..{})",
                        li, line.clockwise_polygon_owner, poly_count
                    ),
                });
            }

            if line.counterclockwise_polygon_owner != -1
                && (line.counterclockwise_polygon_owner < 0
                    || line.counterclockwise_polygon_owner as usize >= poly_count)
            {
                errors.push(ValidationError {
                    message: format!(
                        "line {} counterclockwise_polygon_owner={} out of range (0..{})",
                        li, line.counterclockwise_polygon_owner, poly_count
                    ),
                });
            }
        }

        // Side back-references
        for (si, side) in self.sides.iter().enumerate() {
            if side.polygon_index < 0 || side.polygon_index as usize >= poly_count {
                errors.push(ValidationError {
                    message: format!(
                        "side {} polygon_index={} out of range (0..{})",
                        si, side.polygon_index, poly_count
                    ),
                });
            }

            if side.line_index < 0 || side.line_index as usize >= line_count {
                errors.push(ValidationError {
                    message: format!(
                        "side {} line_index={} out of range (0..{})",
                        si, side.line_index, line_count
                    ),
                });
            }
        }

        // Object polygon references
        for (oi, obj) in self.objects.iter().enumerate() {
            if obj.polygon_index < 0 || obj.polygon_index as usize >= poly_count {
                errors.push(ValidationError {
                    message: format!(
                        "object {} polygon_index={} out of range (0..{})",
                        oi, obj.polygon_index, poly_count
                    ),
                });
            }
        }

        errors
    }
}

/// A cross-reference validation error.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

// =============================================
// Tests
// =============================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use crate::wad::WadFile;

    // -- Endpoint tests --

    #[test]
    fn test_map_endpoint_from_epnt_data() {
        let data = BinaryWriter::new()
            .write_u16(0x0001) // flags
            .write_i16(100) // highest_adjacent_floor_height
            .write_i16(200) // lowest_adjacent_ceiling_height
            .write_i16(50) // vertex.x
            .write_i16(75) // vertex.y
            .write_i16(50) // transformed.x
            .write_i16(75) // transformed.y
            .write_i16(3) // supporting_polygon_index
            .into_bytes();
        assert_eq!(data.len(), ENDPOINT_SIZE);

        let mut cursor = Cursor::new(&data);
        let ep = Endpoint::read(&mut cursor).unwrap();
        assert_eq!(ep.flags, 1);
        assert_eq!(ep.highest_adjacent_floor_height, 100);
        assert_eq!(ep.lowest_adjacent_ceiling_height, 200);
        assert_eq!(ep.vertex, WorldPoint2d { x: 50, y: 75 });
        assert_eq!(ep.supporting_polygon_index, 3);
    }

    #[test]
    fn test_map_endpoint_from_pnts_data() {
        let ep = Endpoint::from_point(100, -200);
        assert_eq!(ep.flags, 0);
        assert_eq!(ep.highest_adjacent_floor_height, 0);
        assert_eq!(ep.lowest_adjacent_ceiling_height, 0);
        assert_eq!(ep.vertex, WorldPoint2d { x: 100, y: -200 });
        assert_eq!(ep.supporting_polygon_index, -1);
    }

    // -- Line tests --

    #[test]
    fn test_map_line_parsing() {
        let data = BinaryWriter::new()
            .write_i16(0) // endpoint_indexes[0]
            .write_i16(1) // endpoint_indexes[1]
            .write_u16(0x6000) // flags: SOLID | TRANSPARENT
            .write_i16(1024) // length
            .write_i16(0) // highest_adjacent_floor
            .write_i16(2048) // lowest_adjacent_ceiling
            .write_i16(0) // cw_poly_side_index
            .write_i16(-1) // ccw_poly_side_index
            .write_i16(0) // cw_polygon_owner
            .write_i16(-1) // ccw_polygon_owner
            .write_padding(12) // 6 unused i16
            .into_bytes();
        assert_eq!(data.len(), LINE_SIZE);

        let mut cursor = Cursor::new(&data);
        let line = Line::read(&mut cursor).unwrap();
        assert_eq!(line.endpoint_indexes, [0, 1]);
        assert_eq!(line.line_flags(), LineFlags::SOLID | LineFlags::TRANSPARENT);
        assert_eq!(line.length, 1024);
        assert_eq!(line.clockwise_polygon_owner, 0);
        assert_eq!(line.counterclockwise_polygon_owner, -1);
    }

    #[test]
    fn test_map_line_flags() {
        assert_eq!(LineFlags::SOLID.bits(), 0x4000);
        assert_eq!(LineFlags::DECORATIVE.bits(), 0x0100);
        let flags = LineFlags::from_bits_truncate(0x4200);
        assert!(flags.contains(LineFlags::SOLID));
        assert!(flags.contains(LineFlags::HAS_TRANSPARENT_SIDE));
        assert!(!flags.contains(LineFlags::LANDSCAPE));
    }

    // -- Side tests --

    #[test]
    fn test_map_side_parsing() {
        let data = BinaryWriter::new()
            .write_i16(0) // side_type = Full
            .write_u16(0x0002) // flags: IS_CONTROL_PANEL
            // primary_texture (SideTexture: 6 bytes)
            .write_i16(10) // x0
            .write_i16(20) // y0
            .write_u16(0x0500) // texture (ShapeDescriptor)
            // secondary_texture
            .write_i16(0)
            .write_i16(0)
            .write_u16(0xFFFF) // none
            // transparent_texture
            .write_i16(0)
            .write_i16(0)
            .write_u16(0xFFFF) // none
            // exclusion_zone: 4 * WorldPoint2d = 16 bytes
            .write_padding(16)
            .write_i16(5) // control_panel_type
            .write_i16(0) // control_panel_permutation
            .write_i16(0) // primary_transfer_mode
            .write_i16(0) // secondary_transfer_mode
            .write_i16(0) // transparent_transfer_mode
            .write_i16(2) // polygon_index
            .write_i16(3) // line_index
            .write_i16(0) // primary_lightsource_index
            .write_i16(0) // secondary_lightsource_index
            .write_i16(0) // transparent_lightsource_index
            .write_i32(0) // ambient_delta
            .write_padding(2) // 1 unused i16
            .into_bytes();
        assert_eq!(data.len(), SIDE_SIZE);

        let mut cursor = Cursor::new(&data);
        let side = Side::read(&mut cursor).unwrap();
        assert_eq!(side.side_type_enum(), SideType::Full);
        assert!(side.side_flags().contains(SideFlags::IS_CONTROL_PANEL));
        assert_eq!(side.primary_texture.x0, 10);
        assert_eq!(side.primary_texture.y0, 20);
        assert_eq!(side.polygon_index, 2);
        assert_eq!(side.line_index, 3);
        assert_eq!(side.control_panel_type, 5);
    }

    #[test]
    fn test_map_side_flags() {
        let flags = SideFlags::from_bits_truncate(0x0043);
        assert!(flags.contains(SideFlags::CONTROL_PANEL_STATUS));
        assert!(flags.contains(SideFlags::IS_CONTROL_PANEL));
        assert!(flags.contains(SideFlags::SWITCH_CAN_ONLY_BE_HIT_BY_PROJECTILES));
    }

    // -- Polygon tests --

    #[test]
    fn test_map_polygon_parsing() {
        let mut w = BinaryWriter::new()
            .write_i16(5) // type = Platform
            .write_u16(0) // flags
            .write_i16(7) // permutation
            .write_u16(3); // vertex_count

        // endpoint_indexes: 8 i16
        for i in 0..3 {
            w = w.write_i16(i);
        }
        for _ in 3..8 {
            w = w.write_i16(-1);
        }
        // line_indexes: 8 i16
        for i in 0..3 {
            w = w.write_i16(i);
        }
        for _ in 3..8 {
            w = w.write_i16(-1);
        }

        w = w
            .write_u16(0x0500) // floor_texture
            .write_u16(0x0600) // ceiling_texture
            .write_i16(0) // floor_height
            .write_i16(1024) // ceiling_height
            .write_i16(0) // floor_lightsource_index
            .write_i16(0) // ceiling_lightsource_index
            .write_i32(5000) // area
            .write_padding(8) // runtime fields
            .write_i16(0) // floor_transfer_mode
            .write_i16(0); // ceiling_transfer_mode

        // adjacent_polygon_indexes: 8 i16
        for _ in 0..8 {
            w = w.write_i16(-1);
        }
        w = w
            .write_padding(4) // runtime fields
            .write_i16(100) // center.x
            .write_i16(200); // center.y

        // side_indexes: 8 i16
        for _ in 0..8 {
            w = w.write_i16(-1);
        }

        w = w
            .write_i16(0) // floor_origin.x
            .write_i16(0) // floor_origin.y
            .write_i16(0) // ceiling_origin.x
            .write_i16(0) // ceiling_origin.y
            .write_i16(-1) // media_index
            .write_i16(-1) // media_lightsource_index
            .write_i16(-1) // sound_source_indexes
            .write_i16(-1) // ambient_sound_image_index
            .write_i16(-1) // random_sound_image_index
            .write_padding(2); // unused

        let data = w.into_bytes();
        assert_eq!(data.len(), POLYGON_SIZE);

        let mut cursor = Cursor::new(&data);
        let poly = Polygon::read(&mut cursor).unwrap();
        assert_eq!(poly.polygon_type_enum(), PolygonType::Platform);
        assert_eq!(poly.vertex_count, 3);
        assert_eq!(poly.endpoint_indexes[0], 0);
        assert_eq!(poly.endpoint_indexes[1], 1);
        assert_eq!(poly.endpoint_indexes[2], 2);
        assert_eq!(poly.endpoint_indexes[3], -1);
        assert_eq!(poly.permutation, 7);
        assert_eq!(poly.ceiling_height, 1024);
        assert_eq!(poly.center, WorldPoint2d { x: 100, y: 200 });
        assert_eq!(poly.area, 5000);
    }

    #[test]
    fn test_map_polygon_type_enum() {
        assert_eq!(PolygonType::from(0), PolygonType::Normal);
        assert_eq!(PolygonType::from(5), PolygonType::Platform);
        assert_eq!(PolygonType::from(10), PolygonType::Teleporter);
        assert_eq!(PolygonType::from(23), PolygonType::Superglue);
        assert_eq!(PolygonType::from(99), PolygonType::Unknown(99));
    }

    // -- MapObject tests --

    #[test]
    fn test_map_object_parsing() {
        let data = BinaryWriter::new()
            .write_i16(0) // type: Monster
            .write_i16(5) // index
            .write_i16(128) // facing
            .write_i16(3) // polygon_index
            .write_i16(100) // location.x
            .write_i16(200) // location.y
            .write_i16(50) // location.z
            .write_u16(0x0006) // flags: HANGING_FROM_CEILING | BLIND
            .into_bytes();
        assert_eq!(data.len(), MAP_OBJECT_SIZE);

        let mut cursor = Cursor::new(&data);
        let obj = MapObject::read(&mut cursor).unwrap();
        assert_eq!(obj.object_type_enum(), MapObjectType::Monster);
        assert_eq!(obj.index, 5);
        assert_eq!(obj.facing, 128);
        assert_eq!(obj.polygon_index, 3);
        assert_eq!(
            obj.location,
            WorldPoint3d {
                x: 100,
                y: 200,
                z: 50
            }
        );
        let flags = obj.object_flags();
        assert!(!flags.contains(MapObjectFlags::INVISIBLE));
        assert!(flags.contains(MapObjectFlags::HANGING_FROM_CEILING));
        assert!(flags.contains(MapObjectFlags::BLIND));
    }

    #[test]
    fn test_map_object_type_enum() {
        assert_eq!(MapObjectType::from(0), MapObjectType::Monster);
        assert_eq!(MapObjectType::from(3), MapObjectType::Player);
        assert_eq!(MapObjectType::from(5), MapObjectType::SoundSource);
        assert_eq!(MapObjectType::from(42), MapObjectType::Unknown(42));
    }

    #[test]
    fn test_map_object_activation_bias() {
        let data = BinaryWriter::new()
            .write_i16(0)
            .write_i16(0)
            .write_i16(0)
            .write_i16(0)
            .write_i16(0)
            .write_i16(0)
            .write_i16(0)
            .write_u16(0xA000) // bits 12-15 = 0xA = 10
            .into_bytes();
        let mut cursor = Cursor::new(&data);
        let obj = MapObject::read(&mut cursor).unwrap();
        assert_eq!(obj.activation_bias(), 0x0A);
    }

    // -- Light tests --

    #[test]
    fn test_map_static_light_parsing() {
        let mut w = BinaryWriter::new()
            .write_i16(1) // light_type
            .write_u16(0) // flags
            .write_i16(0); // phase

        // 6 LightingFunctionSpec blocks (14 bytes each)
        for _ in 0..6 {
            w = w
                .write_i16(0) // function
                .write_i16(30) // period
                .write_i16(0) // delta_period
                .write_i32(0x10000) // intensity = 1.0 as fixed
                .write_i32(0); // delta_intensity
        }

        w = w
            .write_i16(42) // tag
            .write_padding(8); // 4 unused i16

        let data = w.into_bytes();
        assert_eq!(data.len(), STATIC_LIGHT_SIZE);

        let mut cursor = Cursor::new(&data);
        let light = StaticLightData::read(&mut cursor).unwrap();
        assert_eq!(light.light_type, 1);
        assert_eq!(light.tag, 42);
        assert!((light.primary_active.intensity - 1.0).abs() < 0.001);
        assert_eq!(light.primary_active.period, 30);
    }

    #[test]
    fn test_map_old_light_parsing() {
        let data = BinaryWriter::new()
            .write_u16(0) // flags
            .write_i16(0) // light_type
            .write_i16(0) // mode
            .write_i16(0) // phase
            .write_i32(0x8000) // minimum_intensity = 0.5
            .write_i32(0x10000) // maximum_intensity = 1.0
            .write_i16(60) // period
            .write_i32(0xC000) // intensity = 0.75 (runtime)
            .write_padding(10) // 5 unused i16
            .into_bytes();
        assert_eq!(data.len(), OLD_LIGHT_SIZE);

        let mut cursor = Cursor::new(&data);
        let light = OldLightData::read(&mut cursor).unwrap();
        assert!((light.minimum_intensity - 0.5).abs() < 0.001);
        assert!((light.maximum_intensity - 1.0).abs() < 0.001);
        assert_eq!(light.period, 60);
    }

    // -- Platform tests --

    #[test]
    fn test_map_platform_parsing() {
        let data = BinaryWriter::new()
            .write_i16(1) // platform_type
            .write_i16(100) // speed
            .write_i16(30) // delay
            .write_i16(2048) // maximum_height
            .write_i16(0) // minimum_height
            .write_u32(0x00000001) // static_flags
            .write_i16(5) // polygon_index
            .write_i16(10) // tag
            .write_padding(14) // 7 unused i16
            .into_bytes();
        assert_eq!(data.len(), PLATFORM_SIZE);

        let mut cursor = Cursor::new(&data);
        let plat = StaticPlatformData::read(&mut cursor).unwrap();
        assert_eq!(plat.platform_type, 1);
        assert_eq!(plat.speed, 100);
        assert_eq!(plat.delay, 30);
        assert_eq!(plat.maximum_height, 2048);
        assert_eq!(plat.polygon_index, 5);
        assert_eq!(plat.tag, 10);
        assert_eq!(plat.static_flags, 1);
    }

    // -- Media tests --

    #[test]
    fn test_map_media_parsing() {
        let data = BinaryWriter::new()
            .write_i16(1) // media_type = Lava
            .write_u16(0) // flags
            .write_i16(0) // light_index
            .write_i16(0) // current_direction
            .write_i16(0) // current_magnitude
            .write_i16(0) // low
            .write_i16(512) // high
            .write_i16(100) // origin.x
            .write_i16(200) // origin.y
            .write_i16(256) // height
            .write_i32(0x8000) // minimum_light_intensity = 0.5
            .write_u16(0x0300) // texture
            .write_i16(0) // transfer_mode
            .write_padding(4) // 2 unused i16
            .into_bytes();
        assert_eq!(data.len(), MEDIA_SIZE);

        let mut cursor = Cursor::new(&data);
        let media = MediaData::read(&mut cursor).unwrap();
        assert_eq!(media.media_type_enum(), MediaTypeEnum::Lava);
        assert_eq!(media.high, 512);
        assert_eq!(media.origin, WorldPoint2d { x: 100, y: 200 });
        assert!((media.minimum_light_intensity - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_map_media_type_enum() {
        assert_eq!(MediaTypeEnum::from(0), MediaTypeEnum::Water);
        assert_eq!(MediaTypeEnum::from(4), MediaTypeEnum::Jjaro);
        assert_eq!(MediaTypeEnum::from(99), MediaTypeEnum::Unknown(99));
    }

    // -- Annotation tests --

    #[test]
    fn test_map_annotation_parsing() {
        let mut data = BinaryWriter::new()
            .write_i16(0) // annotation_type
            .write_i16(500) // location.x
            .write_i16(600) // location.y
            .write_i16(2) // polygon_index
            .into_bytes();
        // 64-byte text field
        let mut text = b"Hello, Marathon!".to_vec();
        text.resize(64, 0);
        data.extend_from_slice(&text);
        assert_eq!(data.len(), ANNOTATION_SIZE);

        let mut cursor = Cursor::new(&data);
        let note = MapAnnotation::read(&mut cursor).unwrap();
        assert_eq!(note.annotation_type, 0);
        assert_eq!(note.location, WorldPoint2d { x: 500, y: 600 });
        assert_eq!(note.polygon_index, 2);
        assert_eq!(note.text, "Hello, Marathon!");
    }

    // -- Ambient/Random sound tests --

    #[test]
    fn test_map_ambient_sound_parsing() {
        let data = BinaryWriter::new()
            .write_u16(0) // flags
            .write_i16(42) // sound_index
            .write_i16(256) // volume
            .write_padding(10) // 5 unused i16
            .into_bytes();
        assert_eq!(data.len(), AMBIENT_SOUND_SIZE);

        let mut cursor = Cursor::new(&data);
        let amb = AmbientSoundImage::read(&mut cursor).unwrap();
        assert_eq!(amb.sound_index, 42);
        assert_eq!(amb.volume, 256);
    }

    #[test]
    fn test_map_random_sound_parsing() {
        let data = BinaryWriter::new()
            .write_u16(0x0001) // flags: non-directional
            .write_i16(10) // sound_index
            .write_i16(200) // volume
            .write_i16(50) // delta_volume
            .write_i16(60) // period
            .write_i16(10) // delta_period
            .write_i16(0) // direction
            .write_i16(0) // delta_direction
            .write_i32(0x10000) // pitch = 1.0
            .write_i32(0x8000) // delta_pitch = 0.5
            .write_padding(8) // phase + 3 unused i16
            .into_bytes();
        assert_eq!(data.len(), RANDOM_SOUND_SIZE);

        let mut cursor = Cursor::new(&data);
        let rs = RandomSoundImage::read(&mut cursor).unwrap();
        assert!(rs.is_non_directional());
        assert_eq!(rs.sound_index, 10);
        assert_eq!(rs.volume, 200);
        assert!((rs.pitch - 1.0).abs() < 0.001);
        assert!((rs.delta_pitch - 0.5).abs() < 0.001);
    }

    // -- MapInfo tests --

    #[test]
    fn test_map_info_parsing() {
        let mut data = BinaryWriter::new()
            .write_i16(1) // environment_code
            .write_i16(0) // physics_model
            .write_i16(3) // song_index
            .write_i16(0x0003) // mission_flags: extermination + exploration
            .write_i16(0x0001) // environment_flags: vacuum
            .write_bytes(&[1]) // ball_in_play
            .write_padding(7) // unused byte + 3 unused i16
            .into_bytes();
        // 66-byte level_name
        let mut name = b"Test Level".to_vec();
        name.resize(66, 0);
        data.extend_from_slice(&name);
        // entry_point_flags
        data.extend_from_slice(&0x03u32.to_be_bytes());
        assert_eq!(data.len(), MAP_INFO_SIZE);

        let mut cursor = Cursor::new(&data);
        let info = MapInfo::read(&mut cursor).unwrap();
        assert_eq!(info.environment_code, 1);
        assert_eq!(info.song_index, 3);
        assert_eq!(info.level_name, "Test Level");
        assert_eq!(info.ball_in_play, 1);

        let mf = info.mission_flags_bits();
        assert!(mf.contains(MissionFlags::EXTERMINATION));
        assert!(mf.contains(MissionFlags::EXPLORATION));
        assert!(!mf.contains(MissionFlags::RETRIEVAL));

        let ef = info.environment_flags_bits();
        assert!(ef.contains(EnvironmentFlags::VACUUM));
        assert!(!ef.contains(EnvironmentFlags::LOW_GRAVITY));

        let epf = info.entry_point_flags_bits();
        assert!(epf.contains(EntryPointFlags::SINGLE_PLAYER));
        assert!(epf.contains(EntryPointFlags::MULTIPLAYER_COOPERATIVE));
        assert!(!epf.contains(EntryPointFlags::CAPTURE_THE_FLAG));
    }

    // -- Item placement tests --

    #[test]
    fn test_map_item_placement_parsing() {
        let data = BinaryWriter::new()
            .write_u16(0x0001) // flags: reappears randomly
            .write_i16(5) // initial_count
            .write_i16(2) // minimum_count
            .write_i16(10) // maximum_count
            .write_i16(3) // random_count
            .write_u16(100) // random_chance
            .into_bytes();
        assert_eq!(data.len(), ITEM_PLACEMENT_SIZE);

        let mut cursor = Cursor::new(&data);
        let plac = ObjectFrequencyDefinition::read(&mut cursor).unwrap();
        assert!(plac.reappears_randomly());
        assert_eq!(plac.initial_count, 5);
        assert_eq!(plac.minimum_count, 2);
        assert_eq!(plac.maximum_count, 10);
        assert_eq!(plac.random_count, 3);
        assert_eq!(plac.random_chance, 100);
    }

    // -- Terminal tests --

    #[test]
    fn test_map_terminal_parsing() {
        let text_body = b"Welcome to Marathon!";
        let grouping_count: usize = 1;
        let font_changes_count: usize = 1;
        let total_length = 10 + grouping_count * 12 + font_changes_count * 6 + text_body.len();

        let mut data = BinaryWriter::new()
            .write_i16(total_length as i16) // total_length
            .write_i16(0) // flags
            .write_i16(22) // lines_per_page
            .write_i16(grouping_count as i16)
            .write_i16(font_changes_count as i16)
            // grouping
            .write_i16(0) // flags
            .write_i16(5) // type: information
            .write_i16(0) // permutation
            .write_i16(0) // start_index
            .write_i16(text_body.len() as i16) // length
            .write_i16(10) // maximum_line_count
            // font change
            .write_i16(0) // index
            .write_i16(1) // face
            .write_i16(0) // color
            .into_bytes();
        data.extend_from_slice(text_body);

        let terminals = parse_terminals(&data).unwrap();
        assert_eq!(terminals.len(), 1);
        assert_eq!(terminals[0].lines_per_page, 22);
        assert_eq!(terminals[0].groupings.len(), 1);
        assert_eq!(terminals[0].groupings[0].grouping_type, 5);
        assert_eq!(terminals[0].font_changes.len(), 1);
        assert_eq!(terminals[0].font_changes[0].face, 1);
        assert_eq!(terminals[0].text, text_body);
    }

    // -- MapData integration tests --

    #[test]
    fn test_map_data_pnts_takes_precedence_over_epnt() {
        // Build PNTS data (4 bytes per point)
        let pnts_data = BinaryWriter::new()
            .write_i16(100)
            .write_i16(200)
            .write_i16(300)
            .write_i16(400)
            .into_bytes();

        // Build EPNT data (16 bytes per endpoint) with different coords
        let epnt_data = BinaryWriter::new()
            .write_u16(0)
            .write_i16(0)
            .write_i16(0)
            .write_i16(999)
            .write_i16(999)
            .write_i16(999)
            .write_i16(999)
            .write_i16(-1)
            .into_bytes();

        // Build WAD with both PNTS and EPNT tags (PNTS should win)
        let wad_data = WadBuilder::new()
            .add_entry(
                0,
                vec![
                    TagData::new(WadTag::Points, pnts_data),
                    TagData::new(WadTag::Endpoints, epnt_data),
                ],
            )
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let map = MapData::from_entry(entry).unwrap();

        assert_eq!(map.endpoints.len(), 2);
        // PNTS data used, not EPNT
        assert_eq!(map.endpoints[0].vertex, WorldPoint2d { x: 100, y: 200 });
        assert_eq!(map.endpoints[1].vertex, WorldPoint2d { x: 300, y: 400 });
        assert_eq!(map.endpoints[0].supporting_polygon_index, -1); // default from PNTS
    }

    #[test]
    fn test_map_data_invalid_tag_length() {
        // 33 bytes is not a multiple of LINE_SIZE (32)
        let bad_line_data = vec![0u8; 33];
        let wad_data = WadBuilder::new()
            .add_entry(0, vec![TagData::new(WadTag::Lines, bad_line_data)])
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let result = MapData::from_entry(entry);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MapError::InvalidTagLength { .. }));
    }

    #[test]
    fn test_map_data_validation_invalid_references() {
        // Build endpoints, lines, polygon with out-of-range references
        let endpoints = MapDataBuilder::endpoints(&[(0, 0), (100, 0), (100, 100)]);
        let lines = MapDataBuilder::lines(&[(0, 1, 0, -1), (1, 2, 0, -1), (2, 0, 0, -1)]);

        // Polygon referencing endpoint index 99 (out of range)
        let poly = MapDataBuilder::polygon(3, &[0, 1, 99], &[0, 1, 2]);

        let wad_data = WadBuilder::new()
            .add_entry(
                0,
                vec![
                    TagData::new(WadTag::Endpoints, endpoints),
                    TagData::new(WadTag::Lines, lines),
                    TagData::new(WadTag::Polygons, poly),
                ],
            )
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let map = MapData::from_entry(entry).unwrap();

        let errors = map.validate();
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.message.contains("endpoint_indexes")));
    }

    #[test]
    fn test_map_data_validation_valid_triangle() {
        let endpoints = MapDataBuilder::endpoints(&[(0, 0), (100, 0), (100, 100)]);
        let lines = MapDataBuilder::lines(&[(0, 1, 0, -1), (1, 2, 0, -1), (2, 0, 0, -1)]);
        let poly = MapDataBuilder::polygon(3, &[0, 1, 2], &[0, 1, 2]);

        let wad_data = WadBuilder::new()
            .add_entry(
                0,
                vec![
                    TagData::new(WadTag::Endpoints, endpoints),
                    TagData::new(WadTag::Lines, lines),
                    TagData::new(WadTag::Polygons, poly),
                ],
            )
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let map = MapData::from_entry(entry).unwrap();

        let errors = map.validate();
        assert!(
            errors.is_empty(),
            "Expected no validation errors: {:?}",
            errors
        );
    }

    #[test]
    fn test_map_data_guard_paths_opaque() {
        let guard_data = vec![1u8, 2, 3, 4, 5];
        let wad_data = WadBuilder::new()
            .add_entry(
                0,
                vec![TagData::new(WadTag::GuardPaths, guard_data.clone())],
            )
            .build();

        let wad = WadFile::from_bytes(&wad_data).unwrap();
        let entry = wad.entry(0).unwrap();
        let map = MapData::from_entry(entry).unwrap();

        assert_eq!(map.guard_paths, Some(guard_data));
    }
}
