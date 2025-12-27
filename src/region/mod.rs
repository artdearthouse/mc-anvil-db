//! Minecraft Anvil region file format (.mca).
//!
//! Region files contain 32x32 chunks in a specific binary format:
//! - Bytes 0-4095: Location table (1024 entries × 4 bytes)
//! - Bytes 4096-8191: Timestamp table (1024 entries × 4 bytes)
//! - Bytes 8192+: Chunk data (variable size sectors)

mod header;

pub use header::Header;

/// Size of one sector in bytes (4 KB).
pub const SECTOR_SIZE: usize = 4096;

/// Total header size (location table + timestamp table).
pub const HEADER_SIZE: usize = SECTOR_SIZE * 2; // 8192 bytes

/// Number of chunks per region dimension.
pub const REGION_SIZE: i32 = 32;

/// Number of sectors reserved per chunk in the virtual file.
/// 32 sectors * 4096 bytes = 128 KB per chunk.
/// This is the maximum size we support for reading without overlap/truncation.
/// Real chunks can be larger, but for MVP 128KB is a safe upper bound (avg is ~5KB).
pub const CHUNK_STRIDE: u32 = 32;

/// Convert chunk coordinates to local region coordinates (0-31).
#[inline]
pub fn chunk_to_local(chunk_coord: i32) -> i32 {
    chunk_coord.rem_euclid(REGION_SIZE)
}

/// Convert chunk coordinates to region coordinates.
#[inline]
pub fn chunk_to_region(chunk_coord: i32) -> i32 {
    chunk_coord.div_euclid(REGION_SIZE)
}

/// Calculate linear index for a chunk within a region (0-1023).
#[inline]
pub fn local_to_index(local_x: i32, local_z: i32) -> usize {
    (local_z * REGION_SIZE + local_x) as usize
}

/// Calculate local coordinates from linear index.
#[inline]
pub fn index_to_local(index: usize) -> (i32, i32) {
    let local_x = (index % REGION_SIZE as usize) as i32;
    let local_z = (index / REGION_SIZE as usize) as i32;
    (local_x, local_z)
}

/// Calculate file offset for a chunk given its sector number.
#[inline]
pub fn sector_to_offset(sector: u32) -> usize {
    sector as usize * SECTOR_SIZE
}

/// Region file coordinates (parsed from filename like "r.0.-1.mca").
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct RegionPos {
    pub x: i32,
    pub z: i32,
}

impl RegionPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Parse region position from filename (e.g., "r.0.-1.mca").
    pub fn from_filename(name: &str) -> Option<Self> {
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() == 4 && parts[0] == "r" && parts[3] == "mca" {
            let x = parts[1].parse().ok()?;
            let z = parts[2].parse().ok()?;
            Some(Self { x, z })
        } else {
            None
        }
    }

    /// Convert local chunk coordinates to world chunk coordinates.
    pub fn local_to_world(&self, local_x: i32, local_z: i32) -> (i32, i32) {
        (
            self.x * REGION_SIZE + local_x,
            self.z * REGION_SIZE + local_z,
        )
    }
}
