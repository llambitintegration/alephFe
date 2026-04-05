use std::io::Cursor;

use binrw::BinRead;
use bitflags::bitflags;

use crate::error::ShapeError;
use crate::types::fixed_to_f32;

// ─── Constants ──────────────────────────────────────────────────────────────

const MAXIMUM_COLLECTIONS: usize = 32;
const COLLECTION_VERSION: i16 = 3;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn bytes_to_string(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

// ─── Collection Header (32 bytes) ───────────────────────────────────────────

/// Header for a single collection in the Shapes file.
/// There are exactly 32 headers at the start of every Shapes file (1024 bytes total).
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct CollectionHeader {
    pub status: i16,
    pub flags: u16,
    /// Byte offset to 8-bit color depth data (-1 = no data).
    pub offset: i32,
    /// Length of 8-bit color depth data block.
    pub length: i32,
    /// Byte offset to 16-bit color depth data (-1 = no data).
    pub offset16: i32,
    /// Length of 16-bit color depth data block.
    #[br(pad_after = 12)]
    pub length16: i32,
}

impl CollectionHeader {
    pub fn has_8bit_data(&self) -> bool {
        self.offset != -1
    }

    pub fn has_16bit_data(&self) -> bool {
        self.offset16 != -1
    }

    /// Returns (offset, length) for the requested bit depth,
    /// falling back from 16-bit to 8-bit when 16-bit is unavailable.
    pub fn data_offset(&self, prefer_16bit: bool) -> Option<(i32, i32)> {
        if prefer_16bit && self.has_16bit_data() {
            Some((self.offset16, self.length16))
        } else if self.has_8bit_data() {
            Some((self.offset, self.length))
        } else {
            None
        }
    }
}

// ─── Collection Type ────────────────────────────────────────────────────────

/// The type of a shape collection, determining bitmap storage format.
/// Wall and Interface use raw uncompressed storage; Object and Scenery use RLE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionType {
    Unused = 0,
    Wall = 1,
    Object = 2,
    Interface = 3,
    Scenery = 4,
}

impl CollectionType {
    pub fn from_i16(v: i16) -> Self {
        match v {
            1 => Self::Wall,
            2 => Self::Object,
            3 => Self::Interface,
            4 => Self::Scenery,
            _ => Self::Unused,
        }
    }
}

// ─── Collection Definition (544 bytes) ──────────────────────────────────────

/// Definition of a shape collection's contents and layout.
/// Version must equal 3 (COLLECTION_VERSION).
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct CollectionDefinition {
    pub version: i16,
    pub collection_type: i16,
    pub flags: u16,
    pub color_count: i16,
    pub clut_count: i16,
    pub color_table_offset: i32,
    pub high_level_shape_count: i16,
    pub high_level_shape_offset_table_offset: i32,
    pub low_level_shape_count: i16,
    pub low_level_shape_offset_table_offset: i32,
    pub bitmap_count: i16,
    pub bitmap_offset_table_offset: i32,
    pub pixels_to_world: i16,
    #[br(pad_after = 506)]
    pub size: i32,
}

impl CollectionDefinition {
    pub fn get_type(&self) -> CollectionType {
        CollectionType::from_i16(self.collection_type)
    }
}

// ─── Color Value (8 bytes) ──────────────────────────────────────────────────

/// An RGB color entry in a color lookup table (CLUT).
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct ColorValue {
    pub flags: u8,
    pub value: u8,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

impl ColorValue {
    /// Returns true if this color is self-luminescent (flag bit 0x80).
    pub fn is_self_luminescent(&self) -> bool {
        self.flags & 0x80 != 0
    }
}

// ─── Low-Level Shape Flags ──────────────────────────────────────────────────

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LowLevelShapeFlags: u16 {
        const X_MIRRORED = 0x8000;
        const Y_MIRRORED = 0x4000;
        const KEYPOINT_OBSCURED = 0x2000;
    }
}

// ─── Low-Level Shape (36 bytes) ─────────────────────────────────────────────

/// A single frame definition with spatial metadata.
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct LowLevelShape {
    pub flags: u16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub minimum_light_intensity: f32,
    pub bitmap_index: i16,
    pub origin_x: i16,
    pub origin_y: i16,
    pub key_x: i16,
    pub key_y: i16,
    pub world_left: i16,
    pub world_right: i16,
    pub world_top: i16,
    pub world_bottom: i16,
    pub world_x0: i16,
    #[br(pad_after = 8)]
    pub world_y0: i16,
}

impl LowLevelShape {
    pub fn shape_flags(&self) -> LowLevelShapeFlags {
        LowLevelShapeFlags::from_bits_truncate(self.flags)
    }

    pub fn is_x_mirrored(&self) -> bool {
        self.shape_flags().contains(LowLevelShapeFlags::X_MIRRORED)
    }

    pub fn is_y_mirrored(&self) -> bool {
        self.shape_flags().contains(LowLevelShapeFlags::Y_MIRRORED)
    }

    pub fn is_keypoint_obscured(&self) -> bool {
        self.shape_flags()
            .contains(LowLevelShapeFlags::KEYPOINT_OBSCURED)
    }
}

// ─── High-Level Shape Header (90 bytes, internal) ───────────────────────────

#[derive(Debug, Clone, BinRead)]
#[br(big)]
struct HighLevelShapeHeader {
    shape_type: i16,
    flags: u16,
    #[br(count = 34, map = |b: Vec<u8>| bytes_to_string(&b))]
    name: String,
    number_of_views: i16,
    frames_per_view: i16,
    ticks_per_frame: i16,
    key_frame: i16,
    transfer_mode: i16,
    transfer_mode_period: i16,
    first_frame_sound: i16,
    key_frame_sound: i16,
    last_frame_sound: i16,
    pixels_to_world: i16,
    #[br(pad_after = 28)]
    loop_frame: i16,
}

// ─── High-Level Shape ───────────────────────────────────────────────────────

/// An animation sequence definition with frame indices.
#[derive(Debug, Clone)]
pub struct HighLevelShape {
    pub shape_type: i16,
    pub flags: u16,
    pub name: String,
    pub number_of_views: i16,
    pub frames_per_view: i16,
    pub ticks_per_frame: i16,
    pub key_frame: i16,
    pub transfer_mode: i16,
    pub transfer_mode_period: i16,
    pub first_frame_sound: i16,
    pub key_frame_sound: i16,
    pub last_frame_sound: i16,
    pub pixels_to_world: i16,
    pub loop_frame: i16,
    pub low_level_shape_indexes: Vec<i16>,
}

/// Compute the actual number of rendered views from the `number_of_views` field.
pub fn actual_view_count(number_of_views: i16) -> i16 {
    match number_of_views {
        1 | 10 => 1,    // animated1, unanimated
        3 | 4 => 4,     // animated3to4, animated4
        9 | 11 => 5,    // animated3to5, animated5
        2 | 5 | 8 => 8, // animated2to8, animated5to8, animated8
        other => other,
    }
}

// ─── Bitmap Header (26 bytes, internal) ─────────────────────────────────────

#[derive(Debug, Clone, BinRead)]
#[br(big)]
struct BitmapHeader {
    width: i16,
    height: i16,
    bytes_per_row: i16,
    flags: u16,
    #[br(pad_after = 16)]
    #[allow(dead_code)]
    bit_depth: i16,
}

impl BitmapHeader {
    fn is_column_order(&self) -> bool {
        self.flags & 0x8000 != 0
    }

    fn is_transparent(&self) -> bool {
        self.flags & 0x4000 != 0
    }

    fn is_rle(&self) -> bool {
        self.bytes_per_row == -1
    }

    fn row_count(&self) -> usize {
        if self.is_column_order() {
            self.width as usize
        } else {
            self.height as usize
        }
    }
}

// ─── Bitmap ─────────────────────────────────────────────────────────────────

/// Decompressed bitmap with pixel data (8-bit indexed color).
#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: i16,
    pub height: i16,
    pub column_order: bool,
    pub transparent: bool,
    pub pixels: Vec<u8>,
}

// ─── Collection ─────────────────────────────────────────────────────────────

/// A fully parsed shape collection with all components.
#[derive(Debug, Clone)]
pub struct Collection {
    pub definition: CollectionDefinition,
    pub color_tables: Vec<Vec<ColorValue>>,
    pub high_level_shapes: Vec<HighLevelShape>,
    pub low_level_shapes: Vec<LowLevelShape>,
    pub bitmaps: Vec<Bitmap>,
}

// ─── ShapesFile ─────────────────────────────────────────────────────────────

/// Parser for Marathon Shapes files containing sprite collections.
pub struct ShapesFile {
    data: Vec<u8>,
    headers: Vec<CollectionHeader>,
}

impl ShapesFile {
    /// Parse a Shapes file from a byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self, ShapeError> {
        if data.len() < MAXIMUM_COLLECTIONS * 32 {
            return Err(ShapeError::BinRw(binrw::Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "shapes file too short for collection headers",
            ))));
        }

        let mut cursor = Cursor::new(data);
        let mut headers = Vec::with_capacity(MAXIMUM_COLLECTIONS);
        for _ in 0..MAXIMUM_COLLECTIONS {
            headers.push(CollectionHeader::read(&mut cursor)?);
        }

        Ok(Self {
            data: data.to_vec(),
            headers,
        })
    }

    /// Open a Shapes file from the filesystem.
    pub fn open(path: &std::path::Path) -> Result<Self, crate::error::ParseError> {
        let data = std::fs::read(path)?;
        Ok(Self::from_bytes(&data)?)
    }

    /// Get a collection header by index (0-31).
    pub fn header(&self, index: usize) -> Option<&CollectionHeader> {
        self.headers.get(index)
    }

    /// Get all 32 collection headers.
    pub fn headers(&self) -> &[CollectionHeader] {
        &self.headers
    }

    /// Parse a collection using 8-bit color depth.
    pub fn collection(&self, index: usize) -> Result<Collection, ShapeError> {
        self.collection_with_depth(index, false)
    }

    /// Parse a collection, optionally preferring 16-bit color depth.
    /// Falls back to 8-bit if 16-bit data is unavailable.
    pub fn collection_with_depth(
        &self,
        index: usize,
        prefer_16bit: bool,
    ) -> Result<Collection, ShapeError> {
        if index >= MAXIMUM_COLLECTIONS {
            return Err(ShapeError::CollectionOutOfRange(index));
        }

        let header = &self.headers[index];
        let (offset, length) = header
            .data_offset(prefer_16bit)
            .ok_or(ShapeError::CollectionOutOfRange(index))?;

        let start = offset as usize;
        let end = start + length as usize;
        if end > self.data.len() {
            return Err(ShapeError::BitmapDecompression(
                "collection data extends past end of file".into(),
            ));
        }

        parse_collection(&self.data[start..end])
    }
}

// ─── Internal Parsing ───────────────────────────────────────────────────────

fn parse_collection(data: &[u8]) -> Result<Collection, ShapeError> {
    let mut cursor = Cursor::new(data);
    let definition = CollectionDefinition::read(&mut cursor)?;

    if definition.version != COLLECTION_VERSION {
        return Err(ShapeError::InvalidCollectionVersion(definition.version));
    }

    let color_tables = parse_color_tables(data, &definition)?;
    let high_level_shapes = parse_high_level_shapes(data, &definition)?;
    let low_level_shapes = parse_low_level_shapes(data, &definition)?;
    let bitmaps = parse_bitmaps(data, &definition)?;

    Ok(Collection {
        definition,
        color_tables,
        high_level_shapes,
        low_level_shapes,
        bitmaps,
    })
}

fn read_offset_table(data: &[u8], offset: i32, count: i16) -> Result<Vec<i32>, ShapeError> {
    if count <= 0 {
        return Ok(Vec::new());
    }
    let mut cursor = Cursor::new(data);
    cursor.set_position(offset as u64);
    let mut offsets = Vec::with_capacity(count as usize);
    for _ in 0..count {
        offsets.push(<i32 as BinRead>::read_be(&mut cursor)?);
    }
    Ok(offsets)
}

fn parse_color_tables(
    data: &[u8],
    def: &CollectionDefinition,
) -> Result<Vec<Vec<ColorValue>>, ShapeError> {
    if def.clut_count <= 0 || def.color_count <= 0 {
        return Ok(Vec::new());
    }

    let mut cursor = Cursor::new(data);
    cursor.set_position(def.color_table_offset as u64);

    let mut tables = Vec::with_capacity(def.clut_count as usize);
    for _ in 0..def.clut_count {
        let mut colors = Vec::with_capacity(def.color_count as usize);
        for _ in 0..def.color_count {
            colors.push(ColorValue::read(&mut cursor)?);
        }
        tables.push(colors);
    }

    Ok(tables)
}

fn parse_high_level_shapes(
    data: &[u8],
    def: &CollectionDefinition,
) -> Result<Vec<HighLevelShape>, ShapeError> {
    if def.high_level_shape_count <= 0 {
        return Ok(Vec::new());
    }

    let offsets = read_offset_table(
        data,
        def.high_level_shape_offset_table_offset,
        def.high_level_shape_count,
    )?;

    let mut shapes = Vec::with_capacity(offsets.len());
    for &offset in &offsets {
        let mut cursor = Cursor::new(data);
        cursor.set_position(offset as u64);

        let header = HighLevelShapeHeader::read(&mut cursor)?;
        let views = actual_view_count(header.number_of_views);
        let index_count = if views > 0 && header.frames_per_view > 0 {
            views as usize * header.frames_per_view as usize
        } else {
            0
        };

        let mut indexes = Vec::with_capacity(index_count);
        for _ in 0..index_count {
            indexes.push(<i16 as BinRead>::read_be(&mut cursor)?);
        }

        shapes.push(HighLevelShape {
            shape_type: header.shape_type,
            flags: header.flags,
            name: header.name,
            number_of_views: header.number_of_views,
            frames_per_view: header.frames_per_view,
            ticks_per_frame: header.ticks_per_frame,
            key_frame: header.key_frame,
            transfer_mode: header.transfer_mode,
            transfer_mode_period: header.transfer_mode_period,
            first_frame_sound: header.first_frame_sound,
            key_frame_sound: header.key_frame_sound,
            last_frame_sound: header.last_frame_sound,
            pixels_to_world: header.pixels_to_world,
            loop_frame: header.loop_frame,
            low_level_shape_indexes: indexes,
        });
    }

    Ok(shapes)
}

fn parse_low_level_shapes(
    data: &[u8],
    def: &CollectionDefinition,
) -> Result<Vec<LowLevelShape>, ShapeError> {
    if def.low_level_shape_count <= 0 {
        return Ok(Vec::new());
    }

    let offsets = read_offset_table(
        data,
        def.low_level_shape_offset_table_offset,
        def.low_level_shape_count,
    )?;

    let mut shapes = Vec::with_capacity(offsets.len());
    for &offset in &offsets {
        let mut cursor = Cursor::new(data);
        cursor.set_position(offset as u64);
        shapes.push(LowLevelShape::read(&mut cursor)?);
    }

    Ok(shapes)
}

fn parse_bitmaps(data: &[u8], def: &CollectionDefinition) -> Result<Vec<Bitmap>, ShapeError> {
    if def.bitmap_count <= 0 {
        return Ok(Vec::new());
    }

    let offsets = read_offset_table(data, def.bitmap_offset_table_offset, def.bitmap_count)?;

    let mut bitmaps = Vec::with_capacity(offsets.len());
    for &offset in &offsets {
        let mut cursor = Cursor::new(data);
        cursor.set_position(offset as u64);

        let header = BitmapHeader::read(&mut cursor)?;
        let row_count = header.row_count();

        // Skip row/column address pointers: (row_count + 1) * 4 bytes
        let skip = (row_count + 1) * 4;
        cursor.set_position(cursor.position() + skip as u64);

        let pixels = if header.is_rle() {
            decompress_rle(&mut cursor, &header)?
        } else {
            read_raw_pixels(&mut cursor, &header)?
        };

        bitmaps.push(Bitmap {
            width: header.width,
            height: header.height,
            column_order: header.is_column_order(),
            transparent: header.is_transparent(),
            pixels,
        });
    }

    Ok(bitmaps)
}

fn read_raw_pixels(
    cursor: &mut Cursor<&[u8]>,
    header: &BitmapHeader,
) -> Result<Vec<u8>, ShapeError> {
    let row_count = header.row_count();
    let bytes_per_row = header.bytes_per_row as usize;
    let total = row_count * bytes_per_row;

    let pos = cursor.position() as usize;
    let data = cursor.get_ref();

    if pos + total > data.len() {
        return Err(ShapeError::BitmapDecompression(
            "raw bitmap data extends past end of collection".into(),
        ));
    }

    Ok(data[pos..pos + total].to_vec())
}

fn decompress_rle(
    cursor: &mut Cursor<&[u8]>,
    header: &BitmapHeader,
) -> Result<Vec<u8>, ShapeError> {
    let row_count = header.row_count();
    let scanline_length = if header.is_column_order() {
        header.height as usize
    } else {
        header.width as usize
    };

    let mut pixels = vec![0u8; row_count * scanline_length];

    for i in 0..row_count {
        let first = <i16 as BinRead>::read_be(cursor)? as usize;
        let last = <i16 as BinRead>::read_be(cursor)? as usize;

        if last < first {
            return Err(ShapeError::BitmapDecompression(format!(
                "RLE scanline {i}: last ({last}) < first ({first})"
            )));
        }

        let opaque_count = last - first;
        let base = i * scanline_length;

        if opaque_count > 0 {
            let pos = cursor.position() as usize;
            let data = cursor.get_ref();

            if pos + opaque_count > data.len() {
                return Err(ShapeError::BitmapDecompression(
                    "RLE pixel data extends past end of collection".into(),
                ));
            }

            if first + opaque_count <= scanline_length {
                pixels[base + first..base + first + opaque_count]
                    .copy_from_slice(&data[pos..pos + opaque_count]);
            }

            cursor.set_position((pos + opaque_count) as u64);
        }
    }

    Ok(pixels)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::BinaryWriter;

    fn build_header(offset: i32, length: i32, offset16: i32, length16: i32) -> Vec<u8> {
        BinaryWriter::new()
            .write_i16(0)
            .write_u16(0)
            .write_i32(offset)
            .write_i32(length)
            .write_i32(offset16)
            .write_i32(length16)
            .write_padding(12)
            .build()
    }

    fn build_empty_header() -> Vec<u8> {
        build_header(-1, 0, -1, 0)
    }

    fn build_collection_def(
        version: i16,
        ctype: i16,
        color_count: i16,
        clut_count: i16,
        color_table_offset: i32,
        hl_count: i16,
        hl_offset: i32,
        ll_count: i16,
        ll_offset: i32,
        bm_count: i16,
        bm_offset: i32,
    ) -> Vec<u8> {
        BinaryWriter::new()
            .write_i16(version)
            .write_i16(ctype)
            .write_u16(0)
            .write_i16(color_count)
            .write_i16(clut_count)
            .write_i32(color_table_offset)
            .write_i16(hl_count)
            .write_i32(hl_offset)
            .write_i16(ll_count)
            .write_i32(ll_offset)
            .write_i16(bm_count)
            .write_i32(bm_offset)
            .write_i16(0)
            .write_i32(0)
            .write_padding(506)
            .build()
    }

    fn build_color(flags: u8, r: u16, g: u16, b: u16) -> Vec<u8> {
        BinaryWriter::new()
            .write_bytes(&[flags, 0])
            .write_u16(r)
            .write_u16(g)
            .write_u16(b)
            .build()
    }

    fn build_low_level_shape(flags: u16, bitmap_index: i16) -> Vec<u8> {
        BinaryWriter::new()
            .write_u16(flags)
            .write_fixed(0.5)
            .write_i16(bitmap_index)
            .write_i16(10)
            .write_i16(20)
            .write_i16(5)
            .write_i16(15)
            .write_i16(-100)
            .write_i16(100)
            .write_i16(-50)
            .write_i16(50)
            .write_i16(0)
            .write_i16(0)
            .write_padding(8)
            .build()
    }

    fn build_bitmap_header(width: i16, height: i16, bytes_per_row: i16, flags: u16) -> Vec<u8> {
        BinaryWriter::new()
            .write_i16(width)
            .write_i16(height)
            .write_i16(bytes_per_row)
            .write_u16(flags)
            .write_i16(8)
            .write_padding(16)
            .build()
    }

    fn build_minimal_shapes_file() -> Vec<u8> {
        let collection_data_offset = (MAXIMUM_COLLECTIONS * 32) as i32;
        let collection_data = build_collection_def(3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        let collection_len = collection_data.len() as i32;

        let mut file_data = Vec::new();
        file_data.extend_from_slice(&build_header(collection_data_offset, collection_len, -1, 0));
        for _ in 1..MAXIMUM_COLLECTIONS {
            file_data.extend_from_slice(&build_empty_header());
        }
        file_data.extend_from_slice(&collection_data);

        file_data
    }

    // ─── Collection Header ──────────────────────────────────────────────────

    #[test]
    fn test_collection_header_parsing() {
        let data = build_header(1024, 2048, 3072, 4096);
        let mut cursor = Cursor::new(&data[..]);
        let header = CollectionHeader::read(&mut cursor).unwrap();

        assert_eq!(header.offset, 1024);
        assert_eq!(header.length, 2048);
        assert_eq!(header.offset16, 3072);
        assert_eq!(header.length16, 4096);
        assert!(header.has_8bit_data());
        assert!(header.has_16bit_data());
    }

    #[test]
    fn test_collection_header_no_data() {
        let data = build_header(-1, 0, -1, 0);
        let mut cursor = Cursor::new(&data[..]);
        let header = CollectionHeader::read(&mut cursor).unwrap();

        assert!(!header.has_8bit_data());
        assert!(!header.has_16bit_data());
        assert!(header.data_offset(false).is_none());
        assert!(header.data_offset(true).is_none());
    }

    #[test]
    fn test_collection_header_fallback_16_to_8() {
        let data = build_header(1024, 2048, -1, 0);
        let mut cursor = Cursor::new(&data[..]);
        let header = CollectionHeader::read(&mut cursor).unwrap();

        let (offset, length) = header.data_offset(true).unwrap();
        assert_eq!(offset, 1024);
        assert_eq!(length, 2048);
    }

    // ─── Collection Definition ──────────────────────────────────────────────

    #[test]
    fn test_collection_definition_valid() {
        let data = build_collection_def(3, 2, 16, 1, 544, 0, 0, 0, 0, 0, 0);
        let mut cursor = Cursor::new(&data[..]);
        let def = CollectionDefinition::read(&mut cursor).unwrap();

        assert_eq!(def.version, 3);
        assert_eq!(def.collection_type, 2);
        assert_eq!(def.get_type(), CollectionType::Object);
        assert_eq!(def.color_count, 16);
        assert_eq!(def.clut_count, 1);
        assert_eq!(def.color_table_offset, 544);
    }

    #[test]
    fn test_collection_definition_version_validation() {
        let data = build_collection_def(5, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        let result = parse_collection(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShapeError::InvalidCollectionVersion(v) => assert_eq!(v, 5),
            other => panic!("expected InvalidCollectionVersion, got {other:?}"),
        }
    }

    // ─── Collection Type ────────────────────────────────────────────────────

    #[test]
    fn test_collection_types() {
        assert_eq!(CollectionType::from_i16(0), CollectionType::Unused);
        assert_eq!(CollectionType::from_i16(1), CollectionType::Wall);
        assert_eq!(CollectionType::from_i16(2), CollectionType::Object);
        assert_eq!(CollectionType::from_i16(3), CollectionType::Interface);
        assert_eq!(CollectionType::from_i16(4), CollectionType::Scenery);
        assert_eq!(CollectionType::from_i16(99), CollectionType::Unused);
    }

    // ─── CLUT Parsing ───────────────────────────────────────────────────────

    #[test]
    fn test_clut_parsing() {
        let mut data = build_collection_def(3, 1, 2, 1, 544, 0, 0, 0, 0, 0, 0);
        data.extend_from_slice(&build_color(0x80, 0xFFFF, 0, 0));
        data.extend_from_slice(&build_color(0x00, 0, 0xFFFF, 0));

        let collection = parse_collection(&data).unwrap();
        assert_eq!(collection.color_tables.len(), 1);
        assert_eq!(collection.color_tables[0].len(), 2);

        assert!(collection.color_tables[0][0].is_self_luminescent());
        assert_eq!(collection.color_tables[0][0].red, 0xFFFF);

        assert!(!collection.color_tables[0][1].is_self_luminescent());
        assert_eq!(collection.color_tables[0][1].green, 0xFFFF);
    }

    #[test]
    fn test_clut_empty() {
        let data = build_collection_def(3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        let collection = parse_collection(&data).unwrap();
        assert!(collection.color_tables.is_empty());
    }

    #[test]
    fn test_clut_multiple_tables() {
        let mut data = build_collection_def(3, 1, 1, 2, 544, 0, 0, 0, 0, 0, 0);
        // CLUT 0: 1 color (red)
        data.extend_from_slice(&build_color(0x00, 0xFFFF, 0, 0));
        // CLUT 1: 1 color (blue)
        data.extend_from_slice(&build_color(0x00, 0, 0, 0xFFFF));

        let collection = parse_collection(&data).unwrap();
        assert_eq!(collection.color_tables.len(), 2);
        assert_eq!(collection.color_tables[0][0].red, 0xFFFF);
        assert_eq!(collection.color_tables[1][0].blue, 0xFFFF);
    }

    // ─── View Count ─────────────────────────────────────────────────────────

    #[test]
    fn test_view_count_computation() {
        assert_eq!(actual_view_count(1), 1);
        assert_eq!(actual_view_count(10), 1);
        assert_eq!(actual_view_count(3), 4);
        assert_eq!(actual_view_count(4), 4);
        assert_eq!(actual_view_count(9), 5);
        assert_eq!(actual_view_count(11), 5);
        assert_eq!(actual_view_count(2), 8);
        assert_eq!(actual_view_count(5), 8);
        assert_eq!(actual_view_count(8), 8);
        assert_eq!(actual_view_count(6), 6);
        assert_eq!(actual_view_count(12), 12);
    }

    // ─── Low-Level Shape ────────────────────────────────────────────────────

    #[test]
    fn test_low_level_shape_parsing() {
        let data = build_low_level_shape(0, 0);
        let mut cursor = Cursor::new(&data[..]);
        let shape = LowLevelShape::read(&mut cursor).unwrap();

        assert_eq!(shape.bitmap_index, 0);
        assert_eq!(shape.origin_x, 10);
        assert_eq!(shape.origin_y, 20);
        assert_eq!(shape.key_x, 5);
        assert_eq!(shape.key_y, 15);
        assert_eq!(shape.world_left, -100);
        assert_eq!(shape.world_right, 100);
        assert_eq!(shape.world_top, -50);
        assert_eq!(shape.world_bottom, 50);
        assert!((shape.minimum_light_intensity - 0.5).abs() < 0.001);
        assert!(!shape.is_x_mirrored());
        assert!(!shape.is_y_mirrored());
        assert!(!shape.is_keypoint_obscured());
    }

    #[test]
    fn test_low_level_shape_flags() {
        // X_MIRRORED only
        let data = build_low_level_shape(0x8000, 0);
        let mut cursor = Cursor::new(&data[..]);
        let shape = LowLevelShape::read(&mut cursor).unwrap();
        assert!(shape.is_x_mirrored());
        assert!(!shape.is_y_mirrored());
        assert!(!shape.is_keypoint_obscured());

        // Y_MIRRORED only
        let data = build_low_level_shape(0x4000, 0);
        let mut cursor = Cursor::new(&data[..]);
        let shape = LowLevelShape::read(&mut cursor).unwrap();
        assert!(shape.is_y_mirrored());
        assert!(!shape.is_x_mirrored());

        // KEYPOINT_OBSCURED only
        let data = build_low_level_shape(0x2000, 0);
        let mut cursor = Cursor::new(&data[..]);
        let shape = LowLevelShape::read(&mut cursor).unwrap();
        assert!(shape.is_keypoint_obscured());

        // All flags combined
        let data = build_low_level_shape(0xE000, 0);
        let mut cursor = Cursor::new(&data[..]);
        let shape = LowLevelShape::read(&mut cursor).unwrap();
        assert!(shape.is_x_mirrored());
        assert!(shape.is_y_mirrored());
        assert!(shape.is_keypoint_obscured());
    }

    // ─── High-Level Shape ───────────────────────────────────────────────────

    #[test]
    fn test_high_level_shape_parsing() {
        let hl_offset_table_pos = 544i32;
        let hl_data_pos = 548i32;

        let mut data = build_collection_def(3, 1, 0, 0, 0, 1, hl_offset_table_pos, 0, 0, 0, 0);

        // Offset table entry
        data.extend_from_slice(&hl_data_pos.to_be_bytes());

        // High-level shape header (90 bytes)
        let hl = BinaryWriter::new()
            .write_i16(0)
            .write_u16(0)
            .write_bytes(b"TestShape\0")
            .write_padding(24)
            .write_i16(10) // unanimated → 1 view
            .write_i16(2) // frames_per_view
            .write_i16(4) // ticks_per_frame
            .write_i16(0)
            .write_i16(1) // transfer_mode
            .write_i16(10)
            .write_i16(-1)
            .write_i16(-1)
            .write_i16(-1)
            .write_i16(0)
            .write_i16(0)
            .write_padding(28)
            .build();
        data.extend_from_slice(&hl);

        // 1 view * 2 frames = 2 indexes
        data.extend_from_slice(&0_i16.to_be_bytes());
        data.extend_from_slice(&1_i16.to_be_bytes());

        let collection = parse_collection(&data).unwrap();
        assert_eq!(collection.high_level_shapes.len(), 1);

        let shape = &collection.high_level_shapes[0];
        assert_eq!(shape.name, "TestShape");
        assert_eq!(shape.number_of_views, 10);
        assert_eq!(shape.frames_per_view, 2);
        assert_eq!(shape.ticks_per_frame, 4);
        assert_eq!(shape.transfer_mode, 1);
        assert_eq!(shape.low_level_shape_indexes, vec![0, 1]);
    }

    // ─── Raw Bitmap ─────────────────────────────────────────────────────────

    #[test]
    fn test_raw_bitmap_parsing() {
        let bm_offset_table_pos = 544i32;
        let bm_data_pos = 548i32;

        let mut data = build_collection_def(3, 1, 0, 0, 0, 0, 0, 0, 0, 1, bm_offset_table_pos);

        data.extend_from_slice(&bm_data_pos.to_be_bytes());
        data.extend_from_slice(&build_bitmap_header(4, 3, 4, 0));

        // Row address pointers: (3 + 1) * 4 = 16 bytes
        data.extend_from_slice(&vec![0u8; 16]);

        let pixel_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        data.extend_from_slice(&pixel_data);

        let collection = parse_collection(&data).unwrap();
        assert_eq!(collection.bitmaps.len(), 1);

        let bm = &collection.bitmaps[0];
        assert_eq!(bm.width, 4);
        assert_eq!(bm.height, 3);
        assert!(!bm.column_order);
        assert!(!bm.transparent);
        assert_eq!(bm.pixels, pixel_data);
    }

    // ─── RLE Bitmap ─────────────────────────────────────────────────────────

    #[test]
    fn test_rle_bitmap_decompression() {
        let bm_offset_table_pos = 544i32;
        let bm_data_pos = 548i32;

        let mut data = build_collection_def(3, 2, 0, 0, 0, 0, 0, 0, 0, 1, bm_offset_table_pos);

        data.extend_from_slice(&bm_data_pos.to_be_bytes());
        data.extend_from_slice(&build_bitmap_header(4, 3, -1, 0));

        // Row address pointers: (3 + 1) * 4 = 16 bytes
        data.extend_from_slice(&vec![0u8; 16]);

        // Row 0: first=1, last=3 → pixels [0, 0xAA, 0xBB, 0]
        data.extend_from_slice(&1_i16.to_be_bytes());
        data.extend_from_slice(&3_i16.to_be_bytes());
        data.extend_from_slice(&[0xAA, 0xBB]);

        // Row 1: first=0, last=4 → fully opaque
        data.extend_from_slice(&0_i16.to_be_bytes());
        data.extend_from_slice(&4_i16.to_be_bytes());
        data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);

        // Row 2: first=0, last=0 → fully transparent
        data.extend_from_slice(&0_i16.to_be_bytes());
        data.extend_from_slice(&0_i16.to_be_bytes());

        let collection = parse_collection(&data).unwrap();
        let bm = &collection.bitmaps[0];

        assert_eq!(bm.width, 4);
        assert_eq!(bm.height, 3);
        assert_eq!(&bm.pixels[0..4], &[0, 0xAA, 0xBB, 0]);
        assert_eq!(&bm.pixels[4..8], &[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(&bm.pixels[8..12], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_rle_column_order() {
        let bm_offset_table_pos = 544i32;
        let bm_data_pos = 548i32;

        let mut data = build_collection_def(3, 2, 0, 0, 0, 0, 0, 0, 0, 1, bm_offset_table_pos);

        data.extend_from_slice(&bm_data_pos.to_be_bytes());
        // 2x3, column order, RLE
        data.extend_from_slice(&build_bitmap_header(2, 3, -1, 0x8000));

        // Column-order: row_count = width = 2
        // Row address pointers: (2 + 1) * 4 = 12 bytes
        data.extend_from_slice(&vec![0u8; 12]);

        // Column 0: fully transparent
        data.extend_from_slice(&0_i16.to_be_bytes());
        data.extend_from_slice(&0_i16.to_be_bytes());

        // Column 1: single pixel at index 1
        data.extend_from_slice(&1_i16.to_be_bytes());
        data.extend_from_slice(&2_i16.to_be_bytes());
        data.extend_from_slice(&[0xFF]);

        let collection = parse_collection(&data).unwrap();
        let bm = &collection.bitmaps[0];

        assert!(bm.column_order);
        assert_eq!(&bm.pixels[0..3], &[0, 0, 0]);
        assert_eq!(&bm.pixels[3..6], &[0, 0xFF, 0]);
    }

    #[test]
    fn test_rle_single_pixel_span() {
        let bm_offset_table_pos = 544i32;
        let bm_data_pos = 548i32;

        let mut data = build_collection_def(3, 2, 0, 0, 0, 0, 0, 0, 0, 1, bm_offset_table_pos);

        data.extend_from_slice(&bm_data_pos.to_be_bytes());
        // 5x1, row-order, RLE
        data.extend_from_slice(&build_bitmap_header(5, 1, -1, 0));

        // Row address pointers: (1 + 1) * 4 = 8 bytes
        data.extend_from_slice(&vec![0u8; 8]);

        // Single row: first=2, last=3 → one pixel
        data.extend_from_slice(&2_i16.to_be_bytes());
        data.extend_from_slice(&3_i16.to_be_bytes());
        data.extend_from_slice(&[0x42]);

        let collection = parse_collection(&data).unwrap();
        let bm = &collection.bitmaps[0];

        assert_eq!(bm.pixels, vec![0, 0, 0x42, 0, 0]);
    }

    #[test]
    fn test_rle_fully_opaque() {
        let bm_offset_table_pos = 544i32;
        let bm_data_pos = 548i32;

        let mut data = build_collection_def(3, 2, 0, 0, 0, 0, 0, 0, 0, 1, bm_offset_table_pos);

        data.extend_from_slice(&bm_data_pos.to_be_bytes());
        // 3x1, row-order, RLE
        data.extend_from_slice(&build_bitmap_header(3, 1, -1, 0));

        // Row address pointers: (1 + 1) * 4 = 8 bytes
        data.extend_from_slice(&vec![0u8; 8]);

        // Fully opaque: first=0, last=3
        data.extend_from_slice(&0_i16.to_be_bytes());
        data.extend_from_slice(&3_i16.to_be_bytes());
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC]);

        let collection = parse_collection(&data).unwrap();
        let bm = &collection.bitmaps[0];

        assert_eq!(bm.pixels, vec![0xAA, 0xBB, 0xCC]);
    }

    // ─── ShapesFile ─────────────────────────────────────────────────────────

    #[test]
    fn test_shapes_file_from_bytes() {
        let data = build_minimal_shapes_file();
        let shapes = ShapesFile::from_bytes(&data).unwrap();

        assert_eq!(shapes.headers().len(), 32);
        assert!(shapes.header(0).unwrap().has_8bit_data());
        assert!(!shapes.header(1).unwrap().has_8bit_data());
    }

    #[test]
    fn test_shapes_file_collection_parsing() {
        let data = build_minimal_shapes_file();
        let shapes = ShapesFile::from_bytes(&data).unwrap();

        let collection = shapes.collection(0).unwrap();
        assert_eq!(collection.definition.version, 3);
        assert_eq!(collection.definition.get_type(), CollectionType::Wall);
        assert!(collection.color_tables.is_empty());
        assert!(collection.high_level_shapes.is_empty());
        assert!(collection.low_level_shapes.is_empty());
        assert!(collection.bitmaps.is_empty());
    }

    #[test]
    fn test_shapes_file_collection_out_of_range() {
        let data = build_minimal_shapes_file();
        let shapes = ShapesFile::from_bytes(&data).unwrap();
        assert!(shapes.collection(32).is_err());
    }

    #[test]
    fn test_shapes_file_no_data_collection() {
        let data = build_minimal_shapes_file();
        let shapes = ShapesFile::from_bytes(&data).unwrap();
        assert!(shapes.collection(1).is_err());
    }

    #[test]
    fn test_shapes_file_too_short() {
        let data = vec![0u8; 100];
        assert!(ShapesFile::from_bytes(&data).is_err());
    }

    #[test]
    fn test_shapes_file_16bit_fallback() {
        let data = build_minimal_shapes_file();
        let shapes = ShapesFile::from_bytes(&data).unwrap();

        let collection = shapes.collection_with_depth(0, true).unwrap();
        assert_eq!(collection.definition.version, 3);
    }

    // ─── Full Collection ────────────────────────────────────────────────────

    #[test]
    fn test_full_collection_with_all_components() {
        let color_table_offset = 544i32;
        let ll_offset_table_pos = 560i32;
        let ll_data_pos = 564i32;
        let bm_offset_table_pos = 600i32;
        let bm_data_pos = 604i32;

        let mut data = build_collection_def(
            3,
            1,
            2,
            1,
            color_table_offset,
            0,
            0,
            1,
            ll_offset_table_pos,
            1,
            bm_offset_table_pos,
        );

        // 2 colors
        data.extend_from_slice(&build_color(0x00, 0xFFFF, 0, 0));
        data.extend_from_slice(&build_color(0x80, 0, 0, 0xFFFF));

        // Low-level shape offset table + data
        data.extend_from_slice(&ll_data_pos.to_be_bytes());
        data.extend_from_slice(&build_low_level_shape(0x8000, 0));

        // Bitmap offset table + data
        data.extend_from_slice(&bm_data_pos.to_be_bytes());
        data.extend_from_slice(&build_bitmap_header(4, 1, 4, 0));
        data.extend_from_slice(&vec![0u8; 8]); // row ptrs: (1+1)*4
        data.extend_from_slice(&[10, 20, 30, 40]);

        let collection = parse_collection(&data).unwrap();

        // CLUTs
        assert_eq!(collection.color_tables.len(), 1);
        assert_eq!(collection.color_tables[0].len(), 2);
        assert!(!collection.color_tables[0][0].is_self_luminescent());
        assert!(collection.color_tables[0][1].is_self_luminescent());

        // Low-level shapes
        assert_eq!(collection.low_level_shapes.len(), 1);
        assert!(collection.low_level_shapes[0].is_x_mirrored());
        assert_eq!(collection.low_level_shapes[0].bitmap_index, 0);

        // Bitmaps
        assert_eq!(collection.bitmaps.len(), 1);
        assert_eq!(collection.bitmaps[0].pixels, vec![10, 20, 30, 40]);
    }
}
