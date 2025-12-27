//! Region file header generation.
//!
//! The header consists of two tables:
//! - Location table: where each chunk is stored
//! - Timestamp table: when each chunk was last saved

use super::{SECTOR_SIZE, HEADER_SIZE};

/// MCA file header generator.
///
/// Generates the 8KB header (location table + timestamp table)
/// for virtual region files.
pub struct Header;

impl Header {
    /// Generate sparse header based on present chunks.
    pub fn generate(present_chunks: &[usize]) -> Vec<u8> {
        let mut header = vec![0u8; HEADER_SIZE];

        // Location table (first 4096 bytes)
        for &chunk_index in present_chunks {
            // Calculate virtual sector offset.
            // We use a fixed stride to allow larger chunks.
            // Old generic: 2 + i. New: 2 + i * STRIDE.
            let sector_offset = 2 + chunk_index as u32 * crate::region::CHUNK_STRIDE;
            let sector_count: u8 = crate::region::CHUNK_STRIDE as u8; 

            let entry_offset = chunk_index * 4;
            header[entry_offset] = ((sector_offset >> 16) & 0xFF) as u8;
            header[entry_offset + 1] = ((sector_offset >> 8) & 0xFF) as u8;
            header[entry_offset + 2] = (sector_offset & 0xFF) as u8;
            header[entry_offset + 3] = sector_count;
        }

        header
    }

    /// Get a slice of the header for a specific byte range.
    pub fn get_range(present_chunks: &[usize], offset: usize, size: usize) -> Vec<u8> {
        let header = Self::generate(present_chunks);
        let end = std::cmp::min(offset + size, HEADER_SIZE);
        if offset >= HEADER_SIZE {
            vec![0u8; size]
        } else {
            let mut result = header[offset..end].to_vec();
            // Pad with zeros if request extends beyond header
            if result.len() < size {
                result.resize(size, 0);
            }
            result
        }
    }

    /// Calculate sector offset for a chunk index.
    #[inline]
    pub fn chunk_sector(chunk_index: usize) -> u32 {
        2 + chunk_index as u32
    }

    /// Calculate file offset for a chunk index.
    #[inline]
    pub fn chunk_offset(chunk_index: usize) -> usize {
        Self::chunk_sector(chunk_index) as usize * SECTOR_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        let header = Header::generate();
        assert_eq!(header.len(), 8192);
    }

    #[test]
    fn test_first_chunk_location() {
        let header = Header::generate();
        // First chunk (index 0) should be at sector 2
        assert_eq!(header[0], 0); // high byte
        assert_eq!(header[1], 0); // mid byte
        assert_eq!(header[2], 2); // low byte = sector 2
        assert_eq!(header[3], 1); // size = 1 sector
    }

    #[test]
    fn test_chunk_offset() {
        // Chunk 0 at sector 2 = byte 8192
        assert_eq!(Header::chunk_offset(0), 8192);
        // Chunk 1 at sector 3 = byte 12288
        assert_eq!(Header::chunk_offset(1), 12288);
    }
}
