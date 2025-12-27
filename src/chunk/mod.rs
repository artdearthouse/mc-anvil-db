//! Chunk generation and management.
//!
//! This module handles:
//! - Procedural chunk generation
//! - Chunk data serialization (NBT + compression)
//! - Chunk provider that combines storage and generation

mod generator;

pub use generator::Generator;

use std::sync::Arc;
use flate2::read::ZlibDecoder;
use crate::storage::{ChunkPos, ChunkStorage};

/// Provides chunks by checking storage first, then falling back to generation.
///
/// This is the main interface for getting chunk data:
/// 1. Check if chunk exists in storage (was modified by player)
/// 2. If not, generate it procedurally
pub struct ChunkProvider {
    storage: Arc<dyn ChunkStorage>,
    generator: Generator,
}

impl ChunkProvider {
    pub fn new(storage: Arc<dyn ChunkStorage>) -> Self {
        Self {
            storage,
            generator: Generator::new(),
        }
    }

    pub fn get_storage(&self) -> &dyn ChunkStorage {
        self.storage.as_ref()
    }

    /// Get chunk data (from storage only).
    /// Returns raw MCA-formatted bytes (length + compression type + compressed NBT).
    pub fn get_chunk(&self, pos: ChunkPos) -> std::io::Result<Vec<u8>> {
        // Check storage for persisted chunks
        if let Some(data) = self.storage.get(pos) {
            return Ok(data);
        }

        // Previously generated flat world, now we return empty to let server generate.
        Ok(Vec::new()) 
    }

    /// Save a raw chunk blob (header + compressed data) to storage.
    /// Parses the NBT to find the coordinates.
    pub fn save_chunk(&self, data: &[u8]) -> std::io::Result<()> {
        log::info!("ChunkProvider: Processing chunk blob of size {}", data.len());
        
        if data.len() < 5 {
            log::warn!("ChunkProvider: Data too short");
            return Ok(());
        }

        // Check compression type (only Zlib methods 1 or 2 supported)
        let method = data[4];
        if method != 2 && method != 1 {
            log::warn!("ChunkProvider: Unknown compression method {}", method);
            return Ok(()); 
        }

        // Decompress to find coordinates
        let compressed = &data[5..];
        let mut decoder = ZlibDecoder::new(compressed);
        let mut nbt_bytes = Vec::new();
        if let Err(e) = std::io::Read::read_to_end(&mut decoder, &mut nbt_bytes) {
            log::error!("ChunkProvider: Decompression failed: {}", e);
            return Err(e);
        }

        // Partial parse just to get coordinates
        let chunk: crate::nbt::ChunkData = match fastnbt::from_bytes(&nbt_bytes) {
            Ok(c) => c,
            Err(e) => {
                log::error!("ChunkProvider: NBT Parse failed: {}", e);
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e));
            }
        };

        log::info!("ChunkProvider: Found chunk at ({}, {}). Data size: {} bytes", 
            chunk.x_pos, chunk.z_pos, nbt_bytes.len());

        let pos = ChunkPos::new(chunk.x_pos, chunk.z_pos);
        self.storage.set(pos, data.to_vec());
        
        Ok(())
    }

    /// Check if chunk has been modified (exists in storage).
    pub fn is_modified(&self, pos: ChunkPos) -> bool {
        self.storage.exists(pos)
    }
}
