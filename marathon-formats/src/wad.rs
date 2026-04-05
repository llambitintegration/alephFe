use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use binrw::BinRead;

use crate::error::WadError;
use crate::tags::WadTag;

const WAD_HEADER_SIZE: usize = 128;
const OLD_DIRECTORY_ENTRY_SIZE: usize = 8;
const OLD_ENTRY_HEADER_SIZE: usize = 12;

fn bytes_to_string(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

/// WAD file header (128 bytes, big-endian).
#[derive(Debug, Clone)]
pub struct WadHeader {
    pub version: i16,
    pub data_version: i16,
    pub file_name: String,
    pub checksum: u32,
    pub directory_offset: i32,
    pub wad_count: i16,
    pub application_specific_directory_data_size: i16,
    pub entry_header_size: i16,
    pub directory_entry_base_size: i16,
    pub parent_checksum: u32,
}

impl WadHeader {
    fn parse(data: &[u8]) -> Result<Self, WadError> {
        if data.len() < WAD_HEADER_SIZE {
            return Err(WadError::HeaderTooShort(data.len()));
        }
        let mut cursor = Cursor::new(data);
        let raw = RawWadHeader::read(&mut cursor).map_err(WadError::BinRw)?;

        if raw.version < 0 || raw.version > 4 {
            return Err(WadError::UnsupportedVersion(raw.version));
        }

        Ok(Self {
            version: raw.version,
            data_version: raw.data_version,
            file_name: bytes_to_string(&raw.file_name),
            checksum: raw.checksum,
            directory_offset: raw.directory_offset,
            wad_count: raw.wad_count,
            application_specific_directory_data_size: raw.application_specific_directory_data_size,
            entry_header_size: raw.entry_header_size,
            directory_entry_base_size: raw.directory_entry_base_size,
            parent_checksum: raw.parent_checksum,
        })
    }

    pub fn is_overlay(&self) -> bool {
        self.parent_checksum != 0
    }
}

#[derive(BinRead)]
#[br(big)]
struct RawWadHeader {
    version: i16,
    data_version: i16,
    #[br(count = 64)]
    file_name: Vec<u8>,
    checksum: u32,
    directory_offset: i32,
    wad_count: i16,
    application_specific_directory_data_size: i16,
    entry_header_size: i16,
    directory_entry_base_size: i16,
    parent_checksum: u32,
    #[br(count = 20)]
    _unused: Vec<i16>,
}

/// Raw tag data extracted from a WAD entry.
#[derive(Debug, Clone)]
pub struct RawTagData {
    pub tag: WadTag,
    pub data: Vec<u8>,
}

/// A single entry (level/item) within a WAD file.
#[derive(Debug, Clone)]
pub struct WadEntry {
    pub index: i16,
    pub application_data: Vec<u8>,
    tags: Vec<RawTagData>,
}

impl WadEntry {
    /// Get raw data for a specific tag, if present.
    pub fn get_tag_data(&self, tag: WadTag) -> Option<&[u8]> {
        self.tags.iter().find(|t| t.tag == tag).map(|t| t.data.as_slice())
    }

    /// Get all tags in this entry.
    pub fn all_tags(&self) -> &[RawTagData] {
        &self.tags
    }

    /// Parse a tag's data into a typed struct.
    pub fn parse_tag<T: for<'a> BinRead<Args<'a> = ()> + binrw::meta::ReadEndian>(&self, tag: WadTag) -> Result<T, WadError> {
        let data = self
            .get_tag_data(tag)
            .ok_or_else(|| WadError::BinRw(binrw::Error::AssertFail {
                pos: 0,
                message: format!("tag {:?} not found in entry", tag),
            }))?;
        let mut cursor = Cursor::new(data);
        T::read(&mut cursor).map_err(WadError::BinRw)
    }
}

/// A parsed Marathon WAD file.
#[derive(Debug, Clone)]
pub struct WadFile {
    pub header: WadHeader,
    entries: Vec<WadEntry>,
}

impl WadFile {
    /// Open and parse a WAD file from disk.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, WadError> {
        let data = std::fs::read(path).map_err(|e| WadError::BinRw(binrw::Error::Io(e)))?;
        Self::from_bytes(&data)
    }

    /// Parse a WAD file from a byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self, WadError> {
        let header = WadHeader::parse(data)?;

        if header.wad_count < 0 {
            return Err(WadError::NegativeWadCount(header.wad_count));
        }

        let dir_offset = header.directory_offset as usize;
        if dir_offset > data.len() {
            return Err(WadError::DirectoryOutOfBounds {
                offset: header.directory_offset,
                file_size: data.len(),
            });
        }

        let use_old_format = header.version < 1;
        let dir_entry_base_size = if use_old_format {
            OLD_DIRECTORY_ENTRY_SIZE
        } else {
            header.directory_entry_base_size as usize
        };
        let app_data_size = header.application_specific_directory_data_size as usize;
        let dir_entry_total_size = dir_entry_base_size + app_data_size;

        let entry_header_size = if use_old_format {
            OLD_ENTRY_HEADER_SIZE
        } else {
            header.entry_header_size as usize
        };

        let mut entries = Vec::with_capacity(header.wad_count as usize);

        for i in 0..header.wad_count as usize {
            let entry_dir_offset = dir_offset + i * dir_entry_total_size;
            if entry_dir_offset + dir_entry_base_size > data.len() {
                break;
            }

            let mut cursor = Cursor::new(&data[entry_dir_offset..]);

            let (offset_to_start, length, index) = if use_old_format {
                let o = i32::read_be(&mut cursor).map_err(WadError::BinRw)?;
                let l = i32::read_be(&mut cursor).map_err(WadError::BinRw)?;
                (o as usize, l as usize, i as i16)
            } else {
                let o = i32::read_be(&mut cursor).map_err(WadError::BinRw)?;
                let l = i32::read_be(&mut cursor).map_err(WadError::BinRw)?;
                let idx = i16::read_be(&mut cursor).map_err(WadError::BinRw)?;
                (o as usize, l as usize, idx)
            };

            let application_data = if app_data_size > 0 {
                let app_offset = entry_dir_offset + dir_entry_base_size;
                if app_offset + app_data_size <= data.len() {
                    data[app_offset..app_offset + app_data_size].to_vec()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            if offset_to_start + length > data.len() {
                return Err(WadError::EntryOutOfBounds {
                    offset: offset_to_start,
                    length,
                    file_size: data.len(),
                });
            }

            let entry_data = &data[offset_to_start..offset_to_start + length];
            let tags = parse_tag_chain(entry_data, entry_header_size)?;

            entries.push(WadEntry {
                index,
                application_data,
                tags,
            });
        }

        Ok(Self { header, entries })
    }

    /// Number of entries in this WAD file.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get an entry by positional index.
    pub fn entry(&self, index: usize) -> Option<&WadEntry> {
        self.entries.get(index)
    }

    /// Iterate over all entries.
    pub fn entries(&self) -> &[WadEntry] {
        &self.entries
    }

    /// Validate the file's CRC-32 checksum.
    pub fn validate_checksum(&self, data: &[u8]) -> bool {
        if self.header.checksum == 0 {
            return true; // zero checksum means skip validation
        }
        let computed = compute_crc32(data);
        computed == self.header.checksum
    }
}

fn parse_tag_chain(entry_data: &[u8], entry_header_size: usize) -> Result<Vec<RawTagData>, WadError> {
    let mut tags = Vec::new();
    let mut offset = 0usize;
    let mut visited = HashSet::new();

    while offset < entry_data.len() {
        if !visited.insert(offset) {
            return Err(WadError::CyclicTagChain(offset));
        }

        if offset + entry_header_size > entry_data.len() {
            break;
        }

        let mut cursor = Cursor::new(&entry_data[offset..]);
        let tag_code = u32::read_be(&mut cursor).map_err(WadError::BinRw)?;
        let next_offset = i32::read_be(&mut cursor).map_err(WadError::BinRw)?;
        let length = i32::read_be(&mut cursor).map_err(WadError::BinRw)?;

        // For new format, there's also an offset field but we read sequentially
        let data_start = offset + entry_header_size;
        let data_end = data_start + length as usize;

        if data_end > entry_data.len() {
            break;
        }

        tags.push(RawTagData {
            tag: WadTag::from(tag_code),
            data: entry_data[data_start..data_end].to_vec(),
        });

        if next_offset == 0 {
            break;
        }
        offset = next_offset as usize;
    }

    Ok(tags)
}

/// CRC-32 with polynomial 0xEDB88320 (same as standard CRC-32/ISO).
fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    // We need to zero out the checksum field (bytes 68-71) during computation
    for (i, &byte) in data.iter().enumerate() {
        let b = if (68..72).contains(&i) { 0u8 } else { byte };
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFFFFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_too_short() {
        let result = WadFile::from_bytes(&[0u8; 64]);
        assert!(matches!(result, Err(WadError::HeaderTooShort(64))));
    }

    #[test]
    fn test_invalid_version() {
        let mut data = [0u8; 128];
        // Set version to 99 (big-endian i16)
        data[0] = 0;
        data[1] = 99;
        let result = WadFile::from_bytes(&data);
        assert!(matches!(result, Err(WadError::UnsupportedVersion(99))));
    }

    #[test]
    fn test_empty_wad() {
        let mut data = [0u8; 128];
        // version = 4
        data[1] = 4;
        // wad_count = 0
        data[74] = 0;
        data[75] = 0;
        // directory_offset = 128 (right after header)
        data[68] = 0;
        data[69] = 0;
        data[70] = 0;
        data[71] = 128;
        // entry_header_size = 16
        data[78] = 0;
        data[79] = 16;
        // directory_entry_base_size = 10
        data[80] = 0;
        data[81] = 10;

        let wad = WadFile::from_bytes(&data).unwrap();
        assert_eq!(wad.entry_count(), 0);
        assert_eq!(wad.header.version, 4);
    }
}
