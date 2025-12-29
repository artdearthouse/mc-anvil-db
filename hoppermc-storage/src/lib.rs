use anyhow::Result;
use async_trait::async_trait;

pub mod nbt_json;
pub mod postgres;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    PgRaw,          // Phase 1: Blob (formerly Raw)
    PgJsonb,        // Phase 2: Json
    Hybrid,         // Phase 3: Structured
    Weightless      // Phase 4: Diffs
}

#[async_trait]
pub trait ChunkStorage: Send + Sync {
    /// Save a chunk to the storage backend.
    /// Data is expected to be Raw NBT (already decompressed if coming from FUSE write, or generated).
    async fn save_chunk(&self, x: i32, z: i32, data: &[u8]) -> Result<()>;

    /// Load a chunk from storage.
    /// Returns None if the chunk does not exist in the DB.
    async fn load_chunk(&self, x: i32, z: i32) -> Result<Option<Vec<u8>>>;
    async fn get_total_size(&self) -> Result<u64> { Ok(0) }
}
