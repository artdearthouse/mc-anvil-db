//! In-memory storage backend using HashMap.
//!
//! Useful for development and testing. Data is lost on restart.

use std::collections::HashMap;
use std::sync::RwLock;

use super::{ChunkPos, ChunkStorage};

/// In-memory chunk storage using a thread-safe HashMap.
///
/// This is the simplest storage backend - all data lives in RAM
/// and is lost when the process exits. Useful for:
/// - Development and testing
/// - Temporary chunk modifications before Redis/Postgres is set up
pub struct MemoryStorage {
    chunks: RwLock<HashMap<ChunkPos, Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            chunks: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkStorage for MemoryStorage {
    fn get(&self, pos: ChunkPos) -> Option<Vec<u8>> {
        self.chunks.read().unwrap().get(&pos).cloned()
    }

    fn set(&self, pos: ChunkPos, data: Vec<u8>) {
        self.chunks.write().unwrap().insert(pos, data);
    }

    fn delete(&self, pos: ChunkPos) {
        self.chunks.write().unwrap().remove(&pos);
    }

    fn list_chunks(&self) -> Vec<ChunkPos> {
        self.chunks.read().unwrap().keys().cloned().collect()
    }

    fn get_region_chunks(&self, region: crate::region::RegionPos) -> Vec<ChunkPos> {
        self.chunks.read().unwrap()
            .keys()
            .filter(|p| {
                crate::region::chunk_to_region(p.x) == region.x && 
                crate::region::chunk_to_region(p.z) == region.z
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let storage = MemoryStorage::new();
        let pos = ChunkPos::new(10, -5);
        let data = vec![1, 2, 3, 4, 5];

        assert!(!storage.exists(pos));
        storage.set(pos, data.clone());
        assert!(storage.exists(pos));
        assert_eq!(storage.get(pos), Some(data));

        storage.delete(pos);
        assert!(!storage.exists(pos));
    }
}
