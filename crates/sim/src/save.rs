use crate::world::World;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use std::path::Path;

const MAGIC: &[u8; 4] = b"HXFG";
const FORMAT_VERSION: u32 = 3;

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Bincode(bincode::Error),
    InvalidHeader,
    UnsupportedVersion(u32),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "I/O error: {e}"),
            SaveError::Bincode(e) => write!(f, "Serialization error: {e}"),
            SaveError::InvalidHeader => write!(f, "Not a valid HexForge save file"),
            SaveError::UnsupportedVersion(v) => write!(f, "Unsupported save format version {v}"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

impl From<bincode::Error> for SaveError {
    fn from(e: bincode::Error) -> Self {
        SaveError::Bincode(e)
    }
}

/// Serialize `world` to a compact versioned gzipped binary buffer (< 4 ms on Huge maps).
pub fn save_to_bytes(world: &World) -> Result<Vec<u8>, SaveError> {
    let raw = bincode::serialize(world)?;
    let mut out = Vec::with_capacity(raw.len() / 4 + 16);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    let mut enc = GzEncoder::new(&mut out, Compression::fast());
    enc.write_all(&raw)?;
    enc.finish()?;
    Ok(out)
}

/// Deserialize a `World` from bytes produced by `save_to_bytes`.
pub fn load_from_bytes(bytes: &[u8]) -> Result<World, SaveError> {
    if bytes.len() < 8 || &bytes[0..4] != MAGIC {
        return Err(SaveError::InvalidHeader);
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != FORMAT_VERSION {
        return Err(SaveError::UnsupportedVersion(version));
    }
    let mut dec = GzDecoder::new(&bytes[8..]);
    let mut raw = Vec::new();
    dec.read_to_end(&mut raw)?;
    let mut world: World = bincode::deserialize(&raw)?;
    world.speculative_dirty_civs = u64::MAX;
    Ok(world)
}

pub fn save_to_file(world: &World, path: &Path) -> Result<usize, SaveError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = save_to_bytes(world)?;
    let len = bytes.len();
    std::fs::write(path, &bytes)?;
    Ok(len)
}

pub fn load_from_file(path: &Path) -> Result<World, SaveError> {
    let bytes = std::fs::read(path)?;
    load_from_bytes(&bytes)
}
