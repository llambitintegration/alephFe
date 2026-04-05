# sound-formats

Specification for parsing Marathon sound files (M2/Infinity 'snd2' format). Sound files contain a fixed-size file header, an array of sound definitions organized by source and index, and raw audio data referenced by offset/length. Each sound definition carries behavioral metadata and up to 5 permutation slots pointing into the audio data block.

## References

- `Source_Files/Sound/sound_definitions.h` -- struct layouts, flag constants, behavior enum, sound chances
- `Source_Files/Sound/SoundFile.h` / `SoundFile.cpp` -- M2SoundFile open/unpack logic, SoundDefinition::Unpack
- `Source_Files/Sound/SoundManagerEnums.h` -- volume constants (MAXIMUM_SOUND_VOLUME = 256), sound source types

## Data Layout Summary

```
Offset  Size   Field
------  -----  -----
Sound file header (260 bytes):
  0       4    version (int32, big-endian)
  4       4    tag (int32, big-endian; must be 'snd2' = 0x736E6432)
  8       2    source_count (int16, big-endian; typically 2: 8-bit and 16-bit)
  10      2    sound_count (int16, big-endian)
  12    248    unused (124 x int16, reserved/padding)

Sound definition (64 bytes each, repeated source_count * sound_count times):
  0       2    sound_code (int16)
  2       2    behavior_index (int16; 0=quiet, 1=normal, 2=loud)
  4       2    flags (uint16, bitfield)
  6       2    chance (uint16; 0=always play)
  8       4    low_pitch (fixed-point 16.16)
  12      4    high_pitch (fixed-point 16.16)
  16      2    permutations (int16; 0..5)
  18      2    permutations_played (uint16; bitmask)
  20      4    group_offset (int32; absolute file offset to audio group)
  24      4    single_length (int32)
  28      4    total_length (int32)
  32     20    sound_offsets[5] (5 x int32; relative to group_offset)
  52      4    last_played (uint32; runtime only, ignored on load)
  56      8    padding (ptr + size; runtime only, ignored on load)
```

---

## ADDED Requirements

### Requirement: Parse sound file header with tag and version validation

The parser shall read the 260-byte sound file header and validate the tag and version fields before proceeding to sound definitions.

#### Scenario: Valid sound file with tag 'snd2' and version 1

WHEN a sound file is parsed whose first 260 bytes contain version=1, tag='snd2' (0x736E6432), source_count=2, sound_count=110, followed by 248 bytes of padding
THEN the parser returns a sound file header with version 1, tag 'snd2', source_count 2, and sound_count 110

#### Scenario: Valid sound file with version 0

WHEN a sound file is parsed whose header contains version=0 and tag='snd2'
THEN the parser accepts the header, because the original engine treats version 0 as valid

#### Scenario: Invalid tag value

WHEN a sound file is parsed whose header contains tag=0x00000000 (not 'snd2')
THEN the parser returns an error indicating the tag is invalid

#### Scenario: Invalid version value

WHEN a sound file is parsed whose header contains version=99 and tag='snd2'
THEN the parser returns an error indicating the version is unsupported

#### Scenario: Negative source_count or sound_count

WHEN a sound file header contains source_count=-1 or sound_count=-1
THEN the parser returns an error indicating invalid counts

#### Scenario: Zero sound_count with nonzero source_count triggers legacy layout

WHEN a sound file header contains sound_count=0 and source_count=N where N > 0
THEN the parser interprets the file using the legacy layout: sound_count is set to N and source_count is set to 1, matching the original engine's fallback behavior

---

### Requirement: Parse sound definition array with all behavioral fields

The parser shall read source_count * sound_count sound definition structures (64 bytes each) immediately following the file header, preserving all metadata fields.

#### Scenario: Parse a single sound definition with default values

WHEN a 64-byte sound definition is parsed with sound_code=10000, behavior_index=1, flags=0x0000, chance=0, low_pitch=0x00010000 (FIXED_ONE), high_pitch=0x00000000, permutations=3, permutations_played=0, group_offset=50000, single_length=8000, total_length=24000, sound_offsets=[0, 8000, 16000, 0, 0]
THEN the parser produces a sound definition struct with sound_code 10000, behavior_index Normal, flags empty, chance 0 (always), low_pitch FIXED_ONE, high_pitch 0, permutation_count 3, group_offset 50000, single_length 8000, total_length 24000, and sound_offsets [0, 8000, 16000, 0, 0]

#### Scenario: Parse the full definition array for multiple sources

WHEN a sound file has source_count=2 and sound_count=5
THEN the parser reads 10 sound definitions (2 sources x 5 sounds) in order: all definitions for source 0 first, then all definitions for source 1, each accessible by (source_index, sound_index)

#### Scenario: Sound definition with pitch range

WHEN a sound definition has low_pitch=0x0000C000 (0.75 in fixed-point) and high_pitch=0x00018000 (1.5 in fixed-point)
THEN the parser stores both pitch values preserving their fixed-point representation, and they can be converted to f64 as 0.75 and 1.5 respectively

#### Scenario: Sound definition with zero low_pitch

WHEN a sound definition has low_pitch=0 and high_pitch=0
THEN the parser stores both as zero; the runtime should interpret low_pitch=0 as FIXED_ONE (1.0) and high_pitch=0 as equal to low_pitch

#### Scenario: Sound definition with chance value

WHEN a sound definition has chance=16384 (approximately fifty percent: 32768*5/10)
THEN the parser stores the raw chance value 16384; playback logic uses the rule "play if random() >= chance"

#### Scenario: Ignore runtime-only fields

WHEN a sound definition is parsed, the last_played (4 bytes at offset 52) and pointer/size fields (8 bytes at offset 56) are present in the data
THEN the parser skips these 12 bytes, as they are runtime-only values not meaningful in a stored file

---

### Requirement: Parse permutation metadata (offsets and lengths)

Each sound definition contains up to 5 permutation slots. The parser shall read the sound_offsets array and use permutation_count to determine how many are valid.

#### Scenario: Sound with 3 permutations

WHEN a sound definition has permutations=3 and sound_offsets=[0, 4000, 9500, 0, 0]
THEN the parser reports 3 valid permutations with offsets [0, 4000, 9500] relative to group_offset

#### Scenario: Sound with 1 permutation

WHEN a sound definition has permutations=1 and sound_offsets=[0, 0, 0, 0, 0]
THEN the parser reports 1 valid permutation with offset 0 relative to group_offset

#### Scenario: Sound with maximum 5 permutations

WHEN a sound definition has permutations=5 and sound_offsets=[0, 2000, 4000, 6000, 8000]
THEN the parser reports all 5 permutations as valid with the corresponding offsets

#### Scenario: Sound with 0 permutations (empty/unused slot)

WHEN a sound definition has permutations=0
THEN the parser reports zero valid permutations; no audio data is associated with this definition

#### Scenario: Compute individual permutation byte lengths

WHEN a sound definition has permutations=3, sound_offsets=[0, 4000, 9500, 0, 0], and total_length=15000
THEN the length of permutation 0 is 4000 (offset[1] - offset[0]), the length of permutation 1 is 5500 (offset[2] - offset[1]), and the length of permutation 2 is 5500 (total_length - offset[2])

---

### Requirement: Extract raw audio data for individual permutations

The parser shall provide access to the raw audio bytes for any valid permutation of a sound definition by seeking to group_offset + sound_offsets[i] in the file.

#### Scenario: Extract audio data for a single permutation

WHEN the caller requests audio data for permutation 0 of a sound definition with group_offset=50000 and sound_offsets=[0, ...]
THEN the parser seeks to file offset 50000 and reads the audio data beginning at that position

#### Scenario: Extract audio data for a non-first permutation

WHEN the caller requests audio data for permutation 2 of a sound definition with group_offset=50000 and sound_offsets=[0, 4000, 9500, 0, 0]
THEN the parser seeks to file offset 59500 (50000 + 9500) and reads the audio data beginning at that position

#### Scenario: Audio data contains System 7 sound headers

WHEN audio data is read for a permutation, the bytes at the permutation offset contain a System 7 sound header (standard type 0x00, extended type 0xFF, or compressed type 0xFE) followed by raw sample data
THEN the parser reads the sound header to determine audio format (8-bit or 16-bit), sample rate, channel count, loop points, and data length, then provides access to the raw sample data following the header

#### Scenario: Permutation index out of range

WHEN the caller requests audio data for permutation index 3 but the sound definition has permutations=2
THEN the parser returns an error indicating the permutation index is out of range

---

### Requirement: Decode behavioral flags into typed enum and bitflags

The parser shall decode the 16-bit flags field and the behavior_index into strongly typed representations rather than exposing raw integer values.

#### Scenario: Decode behavior_index values

WHEN behavior_index is 0
THEN the parser produces a behavior type of Quiet

WHEN behavior_index is 1
THEN the parser produces a behavior type of Normal

WHEN behavior_index is 2
THEN the parser produces a behavior type of Loud

#### Scenario: Invalid behavior_index

WHEN behavior_index is 5 (outside the range 0..2)
THEN the parser returns an error or maps to a default behavior, indicating the value is not a recognized behavior type

#### Scenario: Decode single flag -- cannot_be_restarted

WHEN the flags field is 0x0001
THEN the decoded flags indicate cannot_be_restarted is set and all other flags are clear

#### Scenario: Decode single flag -- does_not_self_abort

WHEN the flags field is 0x0002
THEN the decoded flags indicate does_not_self_abort is set and all other flags are clear

#### Scenario: Decode single flag -- resists_pitch_changes

WHEN the flags field is 0x0004
THEN the decoded flags indicate resists_pitch_changes is set (0.5x external pitch effect) and all other flags are clear

#### Scenario: Decode single flag -- cannot_change_pitch

WHEN the flags field is 0x0008
THEN the decoded flags indicate cannot_change_pitch is set (no external pitch changes) and all other flags are clear

#### Scenario: Decode single flag -- cannot_be_obstructed

WHEN the flags field is 0x0010
THEN the decoded flags indicate cannot_be_obstructed is set (ignores line-of-sight obstructions) and all other flags are clear

#### Scenario: Decode single flag -- cannot_be_media_obstructed

WHEN the flags field is 0x0020
THEN the decoded flags indicate cannot_be_media_obstructed is set (ignores media obstructions) and all other flags are clear

#### Scenario: Decode single flag -- is_ambient

WHEN the flags field is 0x0040
THEN the decoded flags indicate is_ambient is set (sound is only loaded when ambient sounds are enabled) and all other flags are clear

#### Scenario: Decode combined flags

WHEN the flags field is 0x0043 (cannot_be_restarted | does_not_self_abort | is_ambient)
THEN the decoded flags indicate cannot_be_restarted, does_not_self_abort, and is_ambient are all set, and all other flags are clear

#### Scenario: Flags with unknown high bits set

WHEN the flags field is 0x8041 (is_ambient | an undefined bit 0x8000)
THEN the parser preserves the raw flags value and decodes the known bits (is_ambient set, cannot_be_restarted set), without failing on the unknown bit

---

### Requirement: Handle missing or empty sound slots gracefully

Sound files may contain unused entries or definitions that reference no audio data. The parser shall handle these without errors.

#### Scenario: Sound definition with sound_code of NONE (-1)

WHEN a sound definition has sound_code=-1 (NONE) and permutations=0
THEN the parser successfully parses the definition as an empty/unused slot with no associated audio data

#### Scenario: Sound definition with zero group_offset and zero total_length

WHEN a sound definition has group_offset=0, total_length=0, and permutations=0
THEN the parser treats this as a valid empty definition with no audio data to extract

#### Scenario: Sound file with zero sound_count (after legacy fixup)

WHEN a sound file header has source_count=0 and sound_count=0
THEN the parser returns a valid but empty sound file containing no sound definitions

#### Scenario: Truncated file with fewer definitions than expected

WHEN a sound file header declares sound_count=100 but the file ends after only 50 sound definitions
THEN the parser returns an error indicating unexpected end of file rather than silently producing partial results

#### Scenario: Sound definition with permutations > 0 but group_offset beyond file bounds

WHEN a sound definition has permutations=2 and group_offset=999999999 which exceeds the file size
THEN the parser successfully parses the definition metadata but returns an error when audio data extraction is attempted for any permutation
