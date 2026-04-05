use crate::tags::WadTag;

// ─── BinaryWriter ────────────────────────────────────────────────────────────

/// A helper for constructing big-endian binary payloads in tests.
pub struct BinaryWriter {
    buf: Vec<u8>,
}

impl BinaryWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn write_i16(mut self, v: i16) -> Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn write_i32(mut self, v: i32) -> Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn write_u16(mut self, v: u16) -> Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn write_u32(mut self, v: u32) -> Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn write_bytes(mut self, data: &[u8]) -> Self {
        self.buf.extend_from_slice(data);
        self
    }

    pub fn write_padding(mut self, n: usize) -> Self {
        self.buf.resize(self.buf.len() + n, 0);
        self
    }

    pub fn write_fixed(self, v: f32) -> Self {
        self.write_i32((v * 65536.0) as i32)
    }

    pub fn build(self) -> Vec<u8> {
        self.buf
    }

    /// Alias for build() — both names work.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

// ─── TagData ─────────────────────────────────────────────────────────────────

/// A tag and its payload for use with WadBuilder.
pub struct TagData {
    pub tag: WadTag,
    pub payload: Vec<u8>,
}

impl TagData {
    pub fn new(tag: WadTag, payload: Vec<u8>) -> Self {
        Self { tag, payload }
    }
}

// ─── WadBuilder ──────────────────────────────────────────────────────────────

/// Fluent builder for constructing valid WAD binary data in tests.
pub struct WadBuilder {
    version: i16,
    data_version: i16,
    file_name: [u8; 64],
    checksum: u32,
    parent_checksum: u32,
    app_specific_dir_data_size: i16,
    entries: Vec<EntryBuilder>,
}

struct EntryBuilder {
    index: i16,
    application_data: Vec<u8>,
    tags: Vec<TagData>,
}

impl WadBuilder {
    pub fn new() -> Self {
        Self {
            version: 4,
            data_version: 0,
            file_name: [0u8; 64],
            checksum: 0,
            parent_checksum: 0,
            app_specific_dir_data_size: 0,
            entries: Vec::new(),
        }
    }

    pub fn version(mut self, v: i16) -> Self {
        self.version = v;
        self
    }

    pub fn data_version(mut self, v: i16) -> Self {
        self.data_version = v;
        self
    }

    pub fn file_name(mut self, name: &str) -> Self {
        self.file_name = [0u8; 64];
        let bytes = name.as_bytes();
        let len = bytes.len().min(63);
        self.file_name[..len].copy_from_slice(&bytes[..len]);
        self
    }

    pub fn checksum(mut self, c: u32) -> Self {
        self.checksum = c;
        self
    }

    pub fn parent_checksum(mut self, c: u32) -> Self {
        self.parent_checksum = c;
        self
    }

    pub fn application_specific_directory_data_size(mut self, s: i16) -> Self {
        self.app_specific_dir_data_size = s;
        self
    }

    pub fn add_entry(mut self, index: i16, tags: Vec<TagData>) -> Self {
        self.entries.push(EntryBuilder {
            index,
            application_data: vec![0u8; self.app_specific_dir_data_size as usize],
            tags,
        });
        self
    }

    pub fn add_entry_with_app_data(
        mut self,
        index: i16,
        tags: Vec<TagData>,
        app_data: Vec<u8>,
    ) -> Self {
        self.entries.push(EntryBuilder {
            index,
            application_data: app_data,
            tags,
        });
        self
    }

    pub fn build(self) -> Vec<u8> {
        if self.version < 1 {
            self.build_old_format()
        } else {
            self.build_new_format()
        }
    }

    fn build_new_format(self) -> Vec<u8> {
        let entry_header_size: i16 = 16;
        let dir_entry_base_size: i16 = 10;

        // First pass: serialize all entry data to compute offsets
        let mut entry_blobs: Vec<Vec<u8>> = Vec::new();
        for entry in &self.entries {
            let blob = Self::serialize_entry_tags(&entry.tags, entry_header_size as usize);
            entry_blobs.push(blob);
        }

        // Entry data starts right after the 128-byte header
        let mut data_offset = 128usize;
        let mut entry_offsets: Vec<(usize, usize)> = Vec::new(); // (offset, length)
        for blob in &entry_blobs {
            entry_offsets.push((data_offset, blob.len()));
            data_offset += blob.len();
        }

        // Directory starts after all entry data
        let directory_offset = data_offset as i32;

        // Build the full binary
        let mut w = BinaryWriter::new()
            // Header (128 bytes)
            .write_i16(self.version)
            .write_i16(self.data_version)
            .write_bytes(&self.file_name) // 64 bytes
            .write_u32(self.checksum) // offset 68
            .write_i32(directory_offset) // offset 72
            .write_i16(self.entries.len() as i16) // wad_count, offset 74
            .write_i16(self.app_specific_dir_data_size)
            .write_i16(entry_header_size)
            .write_i16(dir_entry_base_size)
            .write_u32(self.parent_checksum)
            .write_padding(40); // unused (20 * i16)

        // Entry data
        for blob in &entry_blobs {
            w = w.write_bytes(blob);
        }

        // Directory entries
        for (i, entry) in self.entries.iter().enumerate() {
            let (offset, length) = entry_offsets[i];
            w = w
                .write_i32(offset as i32) // offset_to_start
                .write_i32(length as i32) // length
                .write_i16(entry.index); // index

            // Application-specific directory data
            if self.app_specific_dir_data_size > 0 {
                let app_data = &entry.application_data;
                let expected = self.app_specific_dir_data_size as usize;
                if app_data.len() >= expected {
                    w = w.write_bytes(&app_data[..expected]);
                } else {
                    w = w
                        .write_bytes(app_data)
                        .write_padding(expected - app_data.len());
                }
            }
        }

        w.build()
    }

    fn build_old_format(self) -> Vec<u8> {
        let entry_header_size: usize = 12;

        // Serialize entry data
        let mut entry_blobs: Vec<Vec<u8>> = Vec::new();
        for entry in &self.entries {
            let blob = Self::serialize_entry_tags(&entry.tags, entry_header_size);
            entry_blobs.push(blob);
        }

        let mut data_offset = 128usize;
        let mut entry_offsets: Vec<(usize, usize)> = Vec::new();
        for blob in &entry_blobs {
            entry_offsets.push((data_offset, blob.len()));
            data_offset += blob.len();
        }

        let directory_offset = data_offset as i32;

        let mut w = BinaryWriter::new()
            .write_i16(self.version)
            .write_i16(self.data_version)
            .write_bytes(&self.file_name)
            .write_u32(self.checksum)
            .write_i32(directory_offset)
            .write_i16(self.entries.len() as i16)
            .write_i16(self.app_specific_dir_data_size)
            .write_i16(12) // entry_header_size (ignored for old format but in header)
            .write_i16(8) // directory_entry_base_size (ignored for old format)
            .write_u32(self.parent_checksum)
            .write_padding(40);

        for blob in &entry_blobs {
            w = w.write_bytes(blob);
        }

        // Old format directory: 8 bytes per entry (offset, length), no index
        for (offset, length) in &entry_offsets {
            w = w.write_i32(*offset as i32).write_i32(*length as i32);
        }

        w.build()
    }

    fn serialize_entry_tags(tags: &[TagData], entry_header_size: usize) -> Vec<u8> {
        if tags.is_empty() {
            return Vec::new();
        }

        // Compute where each tag's data will start and the next_offset for chaining
        struct TagLayout {
            tag_code: u32,
            data: Vec<u8>,
            offset_in_entry: usize,
        }

        let mut layouts: Vec<TagLayout> = Vec::new();
        let mut current_offset = 0usize;

        for tag_data in tags {
            let tag_code: u32 = tag_data.tag.into();
            layouts.push(TagLayout {
                tag_code,
                data: tag_data.payload.clone(),
                offset_in_entry: current_offset,
            });
            current_offset += entry_header_size + tag_data.payload.len();
        }

        // Serialize with correct next_offset chaining
        let mut result = Vec::new();
        for (i, layout) in layouts.iter().enumerate() {
            let next_offset = if i + 1 < layouts.len() {
                layouts[i + 1].offset_in_entry as i32
            } else {
                0 // last tag
            };

            // Tag header
            result.extend_from_slice(&layout.tag_code.to_be_bytes());
            result.extend_from_slice(&next_offset.to_be_bytes());
            result.extend_from_slice(&(layout.data.len() as i32).to_be_bytes());

            if entry_header_size == 16 {
                // New format has an additional offset field
                result.extend_from_slice(&0_i32.to_be_bytes());
            }

            // Tag payload
            result.extend_from_slice(&layout.data);
        }

        result
    }
}

// ─── MapDataBuilder ──────────────────────────────────────────────────────────

/// Convenience builder for constructing map geometry tag payloads.
pub struct MapDataBuilder;

impl MapDataBuilder {
    /// Build a single 16-byte EPNT record.
    pub fn endpoint(x: i16, y: i16) -> Vec<u8> {
        BinaryWriter::new()
            .write_u16(0) // flags
            .write_i16(0) // highest_adjacent_floor_height
            .write_i16(0) // lowest_adjacent_ceiling_height
            .write_i16(x) // vertex.x
            .write_i16(y) // vertex.y
            .write_i16(x) // transformed.x (same as vertex)
            .write_i16(y) // transformed.y (same as vertex)
            .write_i16(-1) // supporting_polygon_index (NONE)
            .build()
    }

    /// Build a complete EPNT tag payload from multiple points.
    pub fn endpoints(points: &[(i16, i16)]) -> Vec<u8> {
        let mut result = Vec::new();
        for &(x, y) in points {
            result.extend_from_slice(&Self::endpoint(x, y));
        }
        result
    }

    /// Build a single 32-byte LINS record.
    pub fn line(endpoint_a: i16, endpoint_b: i16, cw_poly: i16, ccw_poly: i16) -> Vec<u8> {
        BinaryWriter::new()
            .write_i16(endpoint_a) // endpoint_indexes[0]
            .write_i16(endpoint_b) // endpoint_indexes[1]
            .write_u16(0) // flags
            .write_i16(0) // length (world_distance)
            .write_i16(0) // highest_adjacent_floor
            .write_i16(0) // lowest_adjacent_ceiling
            .write_i16(cw_poly) // clockwise_polygon_owner
            .write_i16(ccw_poly) // counterclockwise_polygon_owner
            .write_i16(-1) // clockwise_polygon_side_index
            .write_i16(-1) // counterclockwise_polygon_side_index
            .write_padding(12) // remaining fields (32 - 20 = 12)
            .build()
    }

    /// Build a complete LINS tag payload from multiple line definitions.
    pub fn lines(defs: &[(i16, i16, i16, i16)]) -> Vec<u8> {
        let mut result = Vec::new();
        for &(a, b, cw, ccw) in defs {
            result.extend_from_slice(&Self::line(a, b, cw, ccw));
        }
        result
    }

    /// Build a single 128-byte POLY record.
    pub fn polygon(vertex_count: u16, endpoint_indexes: &[i16], line_indexes: &[i16]) -> Vec<u8> {
        let mut w = BinaryWriter::new()
            .write_i16(0) // polygon_type (normal)
            .write_u16(0) // flags
            .write_i16(0) // permutation
            .write_u16(vertex_count);

        // endpoint_indexes[8]
        for i in 0..8 {
            w = w.write_i16(endpoint_indexes.get(i).copied().unwrap_or(-1));
        }
        // line_indexes[8]
        for i in 0..8 {
            w = w.write_i16(line_indexes.get(i).copied().unwrap_or(-1));
        }

        w = w
            .write_u16(0xFFFF) // floor_texture (NONE)
            .write_u16(0xFFFF) // ceiling_texture (NONE)
            .write_i16(0) // floor_height
            .write_i16(1024) // ceiling_height
            .write_i16(0) // floor_lightsource_index
            .write_i16(0) // ceiling_lightsource_index
            .write_i32(0) // area
            .write_padding(8) // runtime: first_object, first_exclusion_zone_index, line/point_exclusion_zone_count
            .write_i16(0) // floor_transfer_mode
            .write_i16(0); // ceiling_transfer_mode

        // adjacent_polygon_indexes[8]
        for _ in 0..8 {
            w = w.write_i16(-1);
        }

        w = w
            .write_padding(4) // runtime: first_neighbor_index, neighbor_count
            .write_i16(0) // center.x
            .write_i16(0); // center.y

        // side_indexes[8] — all NONE
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

        w.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wad::WadFile;

    // ─── BinaryWriter tests ─────────────────────────────────────────────────

    #[test]
    fn test_binary_writer_i16_big_endian() {
        let bytes = BinaryWriter::new().write_i16(0x0102).build();
        assert_eq!(bytes, vec![0x01, 0x02]);
    }

    #[test]
    fn test_binary_writer_i32_big_endian() {
        let bytes = BinaryWriter::new().write_i32(0x01020304).build();
        assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_binary_writer_u16_big_endian() {
        let bytes = BinaryWriter::new().write_u16(0xABCD).build();
        assert_eq!(bytes, vec![0xAB, 0xCD]);
    }

    #[test]
    fn test_binary_writer_u32_big_endian() {
        let bytes = BinaryWriter::new().write_u32(0xDEADBEEF).build();
        assert_eq!(bytes, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_binary_writer_fixed_point() {
        // 1.0 in 16.16 = 0x00010000
        let bytes = BinaryWriter::new().write_fixed(1.0).build();
        assert_eq!(bytes, vec![0x00, 0x01, 0x00, 0x00]);

        // 0.5 in 16.16 = 0x00008000
        let bytes = BinaryWriter::new().write_fixed(0.5).build();
        assert_eq!(bytes, vec![0x00, 0x00, 0x80, 0x00]);

        // -1.0 in 16.16 = 0xFFFF0000
        let bytes = BinaryWriter::new().write_fixed(-1.0).build();
        assert_eq!(bytes, vec![0xFF, 0xFF, 0x00, 0x00]);
    }

    #[test]
    fn test_binary_writer_padding() {
        let bytes = BinaryWriter::new().write_padding(4).build();
        assert_eq!(bytes, vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_binary_writer_raw_bytes() {
        let bytes = BinaryWriter::new().write_bytes(&[0xCA, 0xFE]).build();
        assert_eq!(bytes, vec![0xCA, 0xFE]);
    }

    #[test]
    fn test_binary_writer_fluent_chaining() {
        let bytes = BinaryWriter::new()
            .write_i16(1)
            .write_i32(2)
            .write_padding(2)
            .write_bytes(&[0xFF])
            .build();
        assert_eq!(
            bytes,
            vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0xFF]
        );
    }

    // ─── WadBuilder tests ───────────────────────────────────────────────────

    #[test]
    fn test_wad_builder_empty() {
        let data = WadBuilder::new().version(4).build();
        let wad = WadFile::from_bytes(&data).unwrap();
        assert_eq!(wad.entry_count(), 0);
        assert_eq!(wad.header.version, 4);
    }

    #[test]
    fn test_wad_builder_single_entry_single_tag() {
        let payload = vec![0xAA; 88]; // 88-byte MapInfo-sized payload
        let data = WadBuilder::new()
            .version(4)
            .add_entry(0, vec![TagData::new(WadTag::MapInfo, payload.clone())])
            .build();

        let wad = WadFile::from_bytes(&data).unwrap();
        assert_eq!(wad.entry_count(), 1);

        let entry = wad.entry(0).unwrap();
        let tag_data = entry.get_tag_data(WadTag::MapInfo).unwrap();
        assert_eq!(tag_data, &payload[..]);
    }

    #[test]
    fn test_wad_builder_multi_entry_multi_tag() {
        let ep_payload = vec![0x11; 32]; // 2 endpoints (16 bytes each)
        let ln_payload = vec![0x22; 64]; // 2 lines (32 bytes each)
        let ob_payload = vec![0x33; 16]; // 1 object

        let data = WadBuilder::new()
            .version(4)
            .add_entry(
                0,
                vec![
                    TagData::new(WadTag::Endpoints, ep_payload.clone()),
                    TagData::new(WadTag::Lines, ln_payload.clone()),
                ],
            )
            .add_entry(1, vec![TagData::new(WadTag::Objects, ob_payload.clone())])
            .build();

        let wad = WadFile::from_bytes(&data).unwrap();
        assert_eq!(wad.entry_count(), 2);

        let e0 = wad.entry(0).unwrap();
        assert_eq!(e0.get_tag_data(WadTag::Endpoints).unwrap(), &ep_payload[..]);
        assert_eq!(e0.get_tag_data(WadTag::Lines).unwrap(), &ln_payload[..]);

        let e1 = wad.entry(1).unwrap();
        assert_eq!(e1.get_tag_data(WadTag::Objects).unwrap(), &ob_payload[..]);
    }

    #[test]
    fn test_wad_builder_overlay() {
        let data = WadBuilder::new()
            .version(4)
            .parent_checksum(0x12345678)
            .build();

        let wad = WadFile::from_bytes(&data).unwrap();
        assert!(wad.header.is_overlay());
        assert_eq!(wad.header.parent_checksum, 0x12345678);
    }

    #[test]
    fn test_wad_builder_version_0() {
        let payload = vec![0xFF; 16];
        let data = WadBuilder::new()
            .version(0)
            .add_entry(0, vec![TagData::new(WadTag::Points, payload.clone())])
            .build();

        let wad = WadFile::from_bytes(&data).unwrap();
        assert_eq!(wad.header.version, 0);
        assert_eq!(wad.entry_count(), 1);

        let entry = wad.entry(0).unwrap();
        let tag_data = entry.get_tag_data(WadTag::Points).unwrap();
        assert_eq!(tag_data, &payload[..]);
    }

    // ─── MapDataBuilder tests ───────────────────────────────────────────────

    #[test]
    fn test_map_data_builder_endpoints() {
        let data = MapDataBuilder::endpoints(&[(100, 200), (300, 400), (500, 600)]);
        assert_eq!(data.len(), 48); // 3 * 16 bytes

        // Check first endpoint x coord at bytes 6-7 (after flags[2] + heights[4])
        assert_eq!(data[6], 0x00);
        assert_eq!(data[7], 100);
    }

    #[test]
    fn test_map_data_builder_minimal_map_in_wad() {
        let endpoints = MapDataBuilder::endpoints(&[(0, 0), (1024, 0), (0, 1024)]);
        let lines = MapDataBuilder::lines(&[(0, 1, 0, -1), (1, 2, 0, -1), (2, 0, 0, -1)]);
        let polygon = MapDataBuilder::polygon(3, &[0, 1, 2], &[0, 1, 2]);

        let data = WadBuilder::new()
            .version(4)
            .add_entry(
                0,
                vec![
                    TagData::new(WadTag::Endpoints, endpoints),
                    TagData::new(WadTag::Lines, lines),
                    TagData::new(WadTag::Polygons, polygon),
                ],
            )
            .build();

        let wad = WadFile::from_bytes(&data).unwrap();
        assert_eq!(wad.entry_count(), 1);

        let entry = wad.entry(0).unwrap();
        assert!(entry.get_tag_data(WadTag::Endpoints).is_some());
        assert!(entry.get_tag_data(WadTag::Lines).is_some());
        assert!(entry.get_tag_data(WadTag::Polygons).is_some());

        // Verify sizes
        assert_eq!(entry.get_tag_data(WadTag::Endpoints).unwrap().len(), 48);
        assert_eq!(entry.get_tag_data(WadTag::Lines).unwrap().len(), 96);
        assert_eq!(entry.get_tag_data(WadTag::Polygons).unwrap().len(), 128);
    }
}
