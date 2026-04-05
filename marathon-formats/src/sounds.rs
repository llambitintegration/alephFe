use std::io::Cursor;

use binrw::BinRead;
use bitflags::bitflags;

use crate::error::SoundError;
use crate::types::fixed_to_f32;

// ─── Constants ──────────────────────────────────────────────────────────────

const SOUND_TAG: i32 = 0x736E6432; // 'snd2'
const MAXIMUM_PERMUTATIONS: usize = 5;

// ─── Sound File Header (260 bytes) ──────────────────────────────────────────

/// Header for a Marathon sound file.
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct SoundFileHeader {
    pub version: i32,
    pub tag: i32,
    pub source_count: i16,
    #[br(pad_after = 248)]
    pub sound_count: i16,
}

// ─── Sound Behavior ─────────────────────────────────────────────────────────

/// Volume behavior for a sound effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundBehavior {
    Quiet = 0,
    Normal = 1,
    Loud = 2,
}

impl SoundBehavior {
    pub fn from_i16(v: i16) -> Self {
        match v {
            0 => Self::Quiet,
            1 => Self::Normal,
            2 => Self::Loud,
            _ => Self::Normal,
        }
    }
}

// ─── Sound Flags ────────────────────────────────────────────────────────────

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SoundFlags: u16 {
        const CANNOT_BE_RESTARTED = 0x01;
        const DOES_NOT_SELF_ABORT = 0x02;
        const RESISTS_PITCH_CHANGES = 0x04;
        const CANNOT_CHANGE_PITCH = 0x08;
        const CANNOT_BE_OBSTRUCTED = 0x10;
        const CANNOT_BE_MEDIA_OBSTRUCTED = 0x20;
        const IS_AMBIENT = 0x40;
    }
}

// ─── Sound Definition (64 bytes) ────────────────────────────────────────────

/// A single sound definition with behavioral metadata and permutation offsets.
#[derive(Debug, Clone, BinRead)]
#[br(big)]
pub struct SoundDefinition {
    pub sound_code: i16,
    pub behavior_index: i16,
    pub flags: u16,
    pub chance: u16,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub low_pitch: f32,
    #[br(map = |v: i32| fixed_to_f32(v))]
    pub high_pitch: f32,
    pub permutations: i16,
    pub permutations_played: u16,
    pub group_offset: i32,
    pub single_length: i32,
    pub total_length: i32,
    #[br(pad_after = 12)]
    pub sound_offsets: [i32; 5],
}

impl SoundDefinition {
    pub fn behavior(&self) -> SoundBehavior {
        SoundBehavior::from_i16(self.behavior_index)
    }

    pub fn sound_flags(&self) -> SoundFlags {
        SoundFlags::from_bits_truncate(self.flags)
    }

    /// Returns true if this is an empty/unused sound slot.
    pub fn is_empty(&self) -> bool {
        self.sound_code == -1 || self.permutations <= 0
    }

    /// Number of valid permutations (clamped to 0..5).
    pub fn permutation_count(&self) -> usize {
        if self.permutations <= 0 {
            0
        } else {
            (self.permutations as usize).min(MAXIMUM_PERMUTATIONS)
        }
    }

    /// Compute the byte length of a specific permutation.
    pub fn permutation_length(&self, index: usize) -> Result<i32, SoundError> {
        let count = self.permutation_count();
        if index >= count {
            return Err(SoundError::PermutationOutOfRange { index, max: count });
        }
        if index + 1 < count {
            Ok(self.sound_offsets[index + 1] - self.sound_offsets[index])
        } else {
            Ok(self.total_length - self.sound_offsets[index])
        }
    }
}

// ─── SoundsFile ─────────────────────────────────────────────────────────────

/// Parser for Marathon sound files.
#[derive(Debug)]
pub struct SoundsFile {
    data: Vec<u8>,
    header: SoundFileHeader,
    source_count: usize,
    sound_count: usize,
    definitions: Vec<SoundDefinition>,
}

impl SoundsFile {
    /// Parse a sound file from a byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self, SoundError> {
        let mut cursor = Cursor::new(data);
        let header = SoundFileHeader::read(&mut cursor)?;

        if header.tag != SOUND_TAG {
            return Err(SoundError::InvalidTag(header.tag));
        }

        if !(0..=1).contains(&header.version) {
            return Err(SoundError::InvalidVersion(header.version));
        }

        if header.source_count < 0 || header.sound_count < 0 {
            return Err(SoundError::NegativeCounts {
                source_count: header.source_count,
                sound_count: header.sound_count,
            });
        }

        let mut source_count = header.source_count as usize;
        let mut sound_count = header.sound_count as usize;

        // Legacy layout: sound_count=0 with source_count>0
        if sound_count == 0 && source_count > 0 {
            sound_count = source_count;
            source_count = 1;
        }

        let total = source_count * sound_count;
        let mut definitions = Vec::with_capacity(total);
        for _ in 0..total {
            definitions.push(SoundDefinition::read(&mut cursor)?);
        }

        Ok(Self {
            data: data.to_vec(),
            header,
            source_count,
            sound_count,
            definitions,
        })
    }

    /// Open a sound file from the filesystem.
    pub fn open(path: &std::path::Path) -> Result<Self, crate::error::ParseError> {
        let data = std::fs::read(path)?;
        Ok(Self::from_bytes(&data)?)
    }

    pub fn header(&self) -> &SoundFileHeader {
        &self.header
    }

    pub fn source_count(&self) -> usize {
        self.source_count
    }

    pub fn sound_count(&self) -> usize {
        self.sound_count
    }

    /// Get a sound definition by flat index.
    pub fn sound(&self, index: usize) -> Option<&SoundDefinition> {
        self.definitions.get(index)
    }

    /// Get a sound definition by (source_index, sound_index).
    pub fn sound_by_source(&self, source: usize, sound_index: usize) -> Option<&SoundDefinition> {
        if source >= self.source_count || sound_index >= self.sound_count {
            return None;
        }
        self.definitions
            .get(source * self.sound_count + sound_index)
    }

    /// Extract raw audio bytes for a permutation of a sound definition.
    pub fn audio_data(&self, sound_index: usize, permutation: usize) -> Result<&[u8], SoundError> {
        let def = self
            .definitions
            .get(sound_index)
            .ok_or(SoundError::PermutationOutOfRange {
                index: sound_index,
                max: self.definitions.len(),
            })?;

        let count = def.permutation_count();
        if permutation >= count {
            return Err(SoundError::PermutationOutOfRange {
                index: permutation,
                max: count,
            });
        }

        let offset = def.group_offset as usize + def.sound_offsets[permutation] as usize;
        let length = def.permutation_length(permutation)? as usize;
        let end = offset + length;

        if end > self.data.len() {
            return Err(SoundError::AudioDataOutOfBounds {
                offset: end,
                file_size: self.data.len(),
            });
        }

        Ok(&self.data[offset..end])
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::BinaryWriter;

    fn build_sound_header(version: i32, tag: i32, source_count: i16, sound_count: i16) -> Vec<u8> {
        BinaryWriter::new()
            .write_i32(version)
            .write_i32(tag)
            .write_i16(source_count)
            .write_i16(sound_count)
            .write_padding(248)
            .build()
    }

    fn build_sound_definition(
        sound_code: i16,
        behavior: i16,
        flags: u16,
        chance: u16,
        low_pitch: f32,
        high_pitch: f32,
        permutations: i16,
        group_offset: i32,
        single_length: i32,
        total_length: i32,
        offsets: &[i32; 5],
    ) -> Vec<u8> {
        let mut w = BinaryWriter::new()
            .write_i16(sound_code)
            .write_i16(behavior)
            .write_u16(flags)
            .write_u16(chance)
            .write_fixed(low_pitch)
            .write_fixed(high_pitch)
            .write_i16(permutations)
            .write_u16(0) // permutations_played
            .write_i32(group_offset)
            .write_i32(single_length)
            .write_i32(total_length);
        for &off in offsets {
            w = w.write_i32(off);
        }
        w = w.write_padding(12); // runtime fields
        w.build()
    }

    fn build_empty_definition() -> Vec<u8> {
        build_sound_definition(-1, 0, 0, 0, 0.0, 0.0, 0, 0, 0, 0, &[0; 5])
    }

    fn build_minimal_sound_file(definitions: &[Vec<u8>], audio: &[u8]) -> Vec<u8> {
        let source_count = 1i16;
        let sound_count = definitions.len() as i16;

        let mut data = build_sound_header(1, SOUND_TAG, source_count, sound_count);
        for def in definitions {
            data.extend_from_slice(def);
        }
        data.extend_from_slice(audio);
        data
    }

    // ─── Header Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_sound_header_valid() {
        let def = build_empty_definition();
        let data = build_minimal_sound_file(&[def], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        assert_eq!(sf.header().version, 1);
        assert_eq!(sf.header().tag, SOUND_TAG);
        assert_eq!(sf.source_count(), 1);
        assert_eq!(sf.sound_count(), 1);
    }

    #[test]
    fn test_sound_header_version_0() {
        let data = build_sound_header(0, SOUND_TAG, 0, 0);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        assert_eq!(sf.header().version, 0);
    }

    #[test]
    fn test_sound_header_invalid_tag() {
        let data = build_sound_header(1, 0x00000000, 1, 0);
        let result = SoundsFile::from_bytes(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            SoundError::InvalidTag(t) => assert_eq!(t, 0),
            other => panic!("expected InvalidTag, got {other:?}"),
        }
    }

    #[test]
    fn test_sound_header_invalid_version() {
        let data = build_sound_header(99, SOUND_TAG, 1, 0);
        let result = SoundsFile::from_bytes(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            SoundError::InvalidVersion(v) => assert_eq!(v, 99),
            other => panic!("expected InvalidVersion, got {other:?}"),
        }
    }

    #[test]
    fn test_sound_header_negative_counts() {
        let data = build_sound_header(1, SOUND_TAG, -1, 5);
        let result = SoundsFile::from_bytes(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            SoundError::NegativeCounts {
                source_count,
                sound_count,
            } => {
                assert_eq!(source_count, -1);
                assert_eq!(sound_count, 5);
            }
            other => panic!("expected NegativeCounts, got {other:?}"),
        }
    }

    #[test]
    fn test_sound_header_legacy_layout() {
        // sound_count=0, source_count=5 → legacy: source_count=1, sound_count=5
        let mut data = build_sound_header(1, SOUND_TAG, 5, 0);
        for _ in 0..5 {
            data.extend_from_slice(&build_empty_definition());
        }
        let sf = SoundsFile::from_bytes(&data).unwrap();
        assert_eq!(sf.source_count(), 1);
        assert_eq!(sf.sound_count(), 5);
    }

    #[test]
    fn test_sound_header_empty_file() {
        let data = build_sound_header(1, SOUND_TAG, 0, 0);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        assert_eq!(sf.source_count(), 0);
        assert_eq!(sf.sound_count(), 0);
        assert!(sf.sound(0).is_none());
    }

    // ─── Definition Parsing ─────────────────────────────────────────────────

    #[test]
    fn test_sound_definition_parsing() {
        let offsets = [0, 4000, 9500, 0, 0];
        let def_data = build_sound_definition(
            10000, 1, 0x0000, 0, 1.0, 0.0, 3, 50000, 8000, 15000, &offsets,
        );
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();

        let def = sf.sound(0).unwrap();
        assert_eq!(def.sound_code, 10000);
        assert_eq!(def.behavior(), SoundBehavior::Normal);
        assert_eq!(def.chance, 0);
        assert!((def.low_pitch - 1.0).abs() < 0.001);
        assert!((def.high_pitch - 0.0).abs() < 0.001);
        assert_eq!(def.permutation_count(), 3);
        assert_eq!(def.group_offset, 50000);
        assert_eq!(def.single_length, 8000);
        assert_eq!(def.total_length, 15000);
        assert_eq!(def.sound_offsets[0], 0);
        assert_eq!(def.sound_offsets[1], 4000);
        assert_eq!(def.sound_offsets[2], 9500);
    }

    #[test]
    fn test_sound_definition_pitch_range() {
        let def_data = build_sound_definition(1, 0, 0, 0, 0.75, 1.5, 0, 0, 0, 0, &[0; 5]);
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        let def = sf.sound(0).unwrap();
        assert!((def.low_pitch - 0.75).abs() < 0.01);
        assert!((def.high_pitch - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_sound_definition_multiple_sources() {
        // 2 sources, 2 sounds each = 4 definitions
        let mut data = build_sound_header(1, SOUND_TAG, 2, 2);
        // Source 0, sound 0
        data.extend_from_slice(&build_sound_definition(
            100, 0, 0, 0, 1.0, 0.0, 0, 0, 0, 0, &[0; 5],
        ));
        // Source 0, sound 1
        data.extend_from_slice(&build_sound_definition(
            101, 1, 0, 0, 1.0, 0.0, 0, 0, 0, 0, &[0; 5],
        ));
        // Source 1, sound 0
        data.extend_from_slice(&build_sound_definition(
            200, 0, 0, 0, 1.0, 0.0, 0, 0, 0, 0, &[0; 5],
        ));
        // Source 1, sound 1
        data.extend_from_slice(&build_sound_definition(
            201, 2, 0, 0, 1.0, 0.0, 0, 0, 0, 0, &[0; 5],
        ));

        let sf = SoundsFile::from_bytes(&data).unwrap();
        assert_eq!(sf.source_count(), 2);
        assert_eq!(sf.sound_count(), 2);

        assert_eq!(sf.sound_by_source(0, 0).unwrap().sound_code, 100);
        assert_eq!(sf.sound_by_source(0, 1).unwrap().sound_code, 101);
        assert_eq!(sf.sound_by_source(1, 0).unwrap().sound_code, 200);
        assert_eq!(sf.sound_by_source(1, 1).unwrap().sound_code, 201);

        assert_eq!(
            sf.sound_by_source(1, 1).unwrap().behavior(),
            SoundBehavior::Loud
        );
    }

    // ─── Behavior & Flags ───────────────────────────────────────────────────

    #[test]
    fn test_sound_behavior_decoding() {
        assert_eq!(SoundBehavior::from_i16(0), SoundBehavior::Quiet);
        assert_eq!(SoundBehavior::from_i16(1), SoundBehavior::Normal);
        assert_eq!(SoundBehavior::from_i16(2), SoundBehavior::Loud);
        assert_eq!(SoundBehavior::from_i16(5), SoundBehavior::Normal); // default
    }

    #[test]
    fn test_sound_flags_decoding() {
        let f = SoundFlags::from_bits_truncate(0x0001);
        assert!(f.contains(SoundFlags::CANNOT_BE_RESTARTED));
        assert!(!f.contains(SoundFlags::DOES_NOT_SELF_ABORT));

        let f = SoundFlags::from_bits_truncate(0x0043);
        assert!(f.contains(SoundFlags::CANNOT_BE_RESTARTED));
        assert!(f.contains(SoundFlags::DOES_NOT_SELF_ABORT));
        assert!(f.contains(SoundFlags::IS_AMBIENT));
        assert!(!f.contains(SoundFlags::CANNOT_BE_OBSTRUCTED));
    }

    #[test]
    fn test_sound_flags_individual() {
        assert_eq!(
            SoundFlags::from_bits_truncate(0x02),
            SoundFlags::DOES_NOT_SELF_ABORT
        );
        assert_eq!(
            SoundFlags::from_bits_truncate(0x04),
            SoundFlags::RESISTS_PITCH_CHANGES
        );
        assert_eq!(
            SoundFlags::from_bits_truncate(0x08),
            SoundFlags::CANNOT_CHANGE_PITCH
        );
        assert_eq!(
            SoundFlags::from_bits_truncate(0x10),
            SoundFlags::CANNOT_BE_OBSTRUCTED
        );
        assert_eq!(
            SoundFlags::from_bits_truncate(0x20),
            SoundFlags::CANNOT_BE_MEDIA_OBSTRUCTED
        );
        assert_eq!(SoundFlags::from_bits_truncate(0x40), SoundFlags::IS_AMBIENT);
    }

    #[test]
    fn test_sound_flags_unknown_bits_preserved() {
        let def_data = build_sound_definition(1, 0, 0x8041, 0, 1.0, 0.0, 0, 0, 0, 0, &[0; 5]);
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        let def = sf.sound(0).unwrap();
        assert_eq!(def.flags, 0x8041);
        let f = def.sound_flags();
        assert!(f.contains(SoundFlags::CANNOT_BE_RESTARTED));
        assert!(f.contains(SoundFlags::IS_AMBIENT));
    }

    // ─── Permutation Lengths ────────────────────────────────────────────────

    #[test]
    fn test_permutation_length_computation() {
        let offsets = [0, 4000, 9500, 0, 0];
        let def_data = build_sound_definition(1, 0, 0, 0, 1.0, 0.0, 3, 0, 4000, 15000, &offsets);
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        let def = sf.sound(0).unwrap();

        assert_eq!(def.permutation_length(0).unwrap(), 4000);
        assert_eq!(def.permutation_length(1).unwrap(), 5500);
        assert_eq!(def.permutation_length(2).unwrap(), 5500);
    }

    #[test]
    fn test_permutation_single() {
        let offsets = [0, 0, 0, 0, 0];
        let def_data = build_sound_definition(1, 0, 0, 0, 1.0, 0.0, 1, 0, 8000, 8000, &offsets);
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        let def = sf.sound(0).unwrap();

        assert_eq!(def.permutation_count(), 1);
        assert_eq!(def.permutation_length(0).unwrap(), 8000);
    }

    #[test]
    fn test_permutation_max_five() {
        let offsets = [0, 2000, 4000, 6000, 8000];
        let def_data = build_sound_definition(1, 0, 0, 0, 1.0, 0.0, 5, 0, 2000, 10000, &offsets);
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        let def = sf.sound(0).unwrap();

        assert_eq!(def.permutation_count(), 5);
        assert_eq!(def.permutation_length(4).unwrap(), 2000);
    }

    // ─── Empty Slots ────────────────────────────────────────────────────────

    #[test]
    fn test_empty_slot_handling() {
        let def_data = build_empty_definition();
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        let def = sf.sound(0).unwrap();

        assert!(def.is_empty());
        assert_eq!(def.permutation_count(), 0);
        assert_eq!(def.sound_code, -1);
    }

    #[test]
    fn test_empty_slot_zero_group_offset() {
        let def_data = build_sound_definition(0, 0, 0, 0, 0.0, 0.0, 0, 0, 0, 0, &[0; 5]);
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();
        let def = sf.sound(0).unwrap();
        assert!(def.is_empty());
    }

    // ─── Audio Data Extraction ──────────────────────────────────────────────

    #[test]
    fn test_audio_data_extraction() {
        let audio = vec![0xAA; 4000];
        let audio2 = vec![0xBB; 5500];
        let audio3 = vec![0xCC; 5500];
        let mut all_audio = Vec::new();
        all_audio.extend_from_slice(&audio);
        all_audio.extend_from_slice(&audio2);
        all_audio.extend_from_slice(&audio3);

        let group_offset = 260 + 64; // header + 1 definition
        let offsets = [0, 4000, 9500, 0, 0];
        let def_data = build_sound_definition(
            1,
            0,
            0,
            0,
            1.0,
            0.0,
            3,
            group_offset as i32,
            4000,
            15000,
            &offsets,
        );

        let data = build_minimal_sound_file(&[def_data], &all_audio);
        let sf = SoundsFile::from_bytes(&data).unwrap();

        // Permutation 0
        let p0 = sf.audio_data(0, 0).unwrap();
        assert_eq!(p0.len(), 4000);
        assert!(p0.iter().all(|&b| b == 0xAA));

        // Permutation 1
        let p1 = sf.audio_data(0, 1).unwrap();
        assert_eq!(p1.len(), 5500);
        assert!(p1.iter().all(|&b| b == 0xBB));

        // Permutation 2
        let p2 = sf.audio_data(0, 2).unwrap();
        assert_eq!(p2.len(), 5500);
        assert!(p2.iter().all(|&b| b == 0xCC));
    }

    #[test]
    fn test_audio_data_permutation_out_of_range() {
        let def_data = build_sound_definition(1, 0, 0, 0, 1.0, 0.0, 2, 0, 0, 100, &[0; 5]);
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();

        assert!(sf.audio_data(0, 3).is_err());
        match sf.audio_data(0, 3).unwrap_err() {
            SoundError::PermutationOutOfRange { index, max } => {
                assert_eq!(index, 3);
                assert_eq!(max, 2);
            }
            other => panic!("expected PermutationOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn test_audio_data_out_of_bounds() {
        let offsets = [0, 0, 0, 0, 0];
        let def_data =
            build_sound_definition(1, 0, 0, 0, 1.0, 0.0, 1, 999_999_999, 1000, 1000, &offsets);
        let data = build_minimal_sound_file(&[def_data], &[]);
        let sf = SoundsFile::from_bytes(&data).unwrap();

        assert!(sf.audio_data(0, 0).is_err());
        match sf.audio_data(0, 0).unwrap_err() {
            SoundError::AudioDataOutOfBounds { .. } => {}
            other => panic!("expected AudioDataOutOfBounds, got {other:?}"),
        }
    }

    #[test]
    fn test_truncated_file() {
        // Header says 100 sounds but file only has header
        let data = build_sound_header(1, SOUND_TAG, 1, 100);
        let result = SoundsFile::from_bytes(&data);
        assert!(result.is_err());
    }
}
