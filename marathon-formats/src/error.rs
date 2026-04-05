use thiserror::Error;

/// Top-level parse error for all Marathon format parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("WAD error: {0}")]
    Wad(#[from] WadError),
    #[error("Map error: {0}")]
    Map(#[from] MapError),
    #[error("Shape error: {0}")]
    Shape(#[from] ShapeError),
    #[error("Sound error: {0}")]
    Sound(#[from] SoundError),
    #[error("Physics error: {0}")]
    Physics(#[from] PhysicsError),
    #[error("MML error: {0}")]
    Mml(#[from] MmlError),
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum WadError {
    #[error("invalid WAD header: expected 128 bytes, got {0}")]
    HeaderTooShort(usize),
    #[error("unsupported WAD version {0} (expected 0-4)")]
    UnsupportedVersion(i16),
    #[error("directory offset {offset} exceeds file size {file_size}")]
    DirectoryOutOfBounds { offset: i32, file_size: usize },
    #[error("entry data out of bounds: offset {offset}, length {length}, file size {file_size}")]
    EntryOutOfBounds {
        offset: usize,
        length: usize,
        file_size: usize,
    },
    #[error("negative wad_count: {0}")]
    NegativeWadCount(i16),
    #[error("cyclic tag chain detected at offset {0}")]
    CyclicTagChain(usize),
    #[error("failed to parse WAD: {0}")]
    BinRw(#[from] binrw::Error),
}

#[derive(Debug, Error)]
pub enum MapError {
    #[error("tag data length {length} is not a multiple of struct size {struct_size} for tag {tag}")]
    InvalidTagLength {
        tag: String,
        length: usize,
        struct_size: usize,
    },
    #[error("invalid cross-reference: {0}")]
    InvalidReference(String),
    #[error("failed to parse map data: {0}")]
    BinRw(#[from] binrw::Error),
}

#[derive(Debug, Error)]
pub enum ShapeError {
    #[error("invalid collection version {0} (expected 3)")]
    InvalidCollectionVersion(i16),
    #[error("collection index {0} out of range (0-31)")]
    CollectionOutOfRange(usize),
    #[error("bitmap decompression error: {0}")]
    BitmapDecompression(String),
    #[error("failed to parse shapes: {0}")]
    BinRw(#[from] binrw::Error),
}

#[derive(Debug, Error)]
pub enum SoundError {
    #[error("invalid sound file tag: expected 0x736E6432 ('snd2'), got {0:#010x}")]
    InvalidTag(i32),
    #[error("invalid sound file version: {0}")]
    InvalidVersion(i32),
    #[error("permutation index {index} out of range (max {max})")]
    PermutationOutOfRange { index: usize, max: usize },
    #[error("audio data offset {offset} exceeds file size {file_size}")]
    AudioDataOutOfBounds { offset: usize, file_size: usize },
    #[error("failed to parse sounds: {0}")]
    BinRw(#[from] binrw::Error),
}

#[derive(Debug, Error)]
pub enum PhysicsError {
    #[error("tag data length {length} is not a multiple of record size {record_size} for tag {tag}")]
    InvalidTagLength {
        tag: String,
        length: usize,
        record_size: usize,
    },
    #[error("failed to parse physics: {0}")]
    BinRw(#[from] binrw::Error),
}

#[derive(Debug, Error)]
pub enum MmlError {
    #[error("invalid root element: expected 'marathon', got '{0}'")]
    InvalidRootElement(String),
    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("XML parse error in {source}: {message}")]
    XmlWithContext {
        source: String,
        #[source]
        message: XmlContextMessage,
    },
}

#[derive(Debug)]
pub struct XmlContextMessage(pub String);

impl std::fmt::Display for XmlContextMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for XmlContextMessage {
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("missing required 'name' attribute in Plugin.xml")]
    MissingName,
    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
