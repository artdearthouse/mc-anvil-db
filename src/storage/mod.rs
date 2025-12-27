mod memory;
mod postgres;


pub use memory::MemoryStorage;
pub use postgres::PostgresStorage;

/// Coordinates for a chunk in the world.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

/// Abstract storage interface for chunk data.
///
/// Implementations of this trait can store chunk data in various backends:
/// - `MemoryStorage` - In-memory HashMap (for testing/development)
/// - `RedisStorage` - Redis (for distributed caching) [future]
/// - `PostgresStorage` - PostgreSQL (for persistence) [future]
///
/// The storage deals with raw compressed chunk bytes, not parsed NBT.
pub trait ChunkStorage: Send + Sync {
    /// Retrieve chunk data by coordinates.
    /// Returns None if the chunk hasn't been modified/stored.
    fn get(&self, pos: ChunkPos) -> Option<Vec<u8>>;

    /// Store chunk data at the given coordinates.
    fn set(&self, pos: ChunkPos, data: Vec<u8>);

    /// Check if a chunk exists in storage.
    fn exists(&self, pos: ChunkPos) -> bool {
        self.get(pos).is_some()
    }

    /// Delete a chunk from storage.
    fn delete(&self, pos: ChunkPos);

    /// Get all stored chunk positions (for debugging/admin).
    fn list_chunks(&self) -> Vec<ChunkPos>;

    /// Get all existing chunks within a specific region.
    /// Used to generate the region header.
    fn get_region_chunks(&self, region: crate::region::RegionPos) -> Vec<ChunkPos>;
}
