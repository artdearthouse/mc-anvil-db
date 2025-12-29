// Sparse Files for Emulationg Real files (so minecraft will see weight of file)

use std::io::{Read, Write};
use flate2::write::ZlibEncoder;
use flate2::Compression as ZlibCompression;

pub const SECTOR_BYTES: u64 = 4096; // minecraft uses 4096 bytes per sector     
pub const HEADER_BYTES: u64 = 8192; // header is 8192 bytes (2 sectors 8kb) 


pub const SECTORS_PER_CHUNK: u64 = 64; // 256kb per chunk


pub fn get_chunk_file_offset(rel_x: i32, rel_z: i32) -> u64 {
    // 32x32 chunks in region. index from 0 to 1023.
    // Formula: x + z * 32
    let index = (rel_x & 31) + (rel_z & 31) * 32; 
    
    // Offset = Header + (Chunk index * Sector size)
    HEADER_BYTES + (index as u64 * SECTORS_PER_CHUNK * SECTOR_BYTES)
}

pub fn generate_header() -> Vec<u8> {
    let mut header = vec![0u8; HEADER_BYTES as usize];
    for i in 0..1024 {
        let rel_x = i % 32;
        let rel_z = i / 32;
        
        // Calculate where the chunk lies using our Sparse formula
        // Let's rely on the canonical get_chunk_file_offset to be safe
        let chunk_offset = get_chunk_file_offset(rel_x, rel_z);
        let sector_id = (chunk_offset / SECTOR_BYTES) as u32;
        let sector_count = SECTORS_PER_CHUNK as u8;

        // Minecraft stores: [Offset:3 bytes][Count:1 byte] (Big Endian)
        let loc_idx = (i as usize) * 4;
        header[loc_idx] = ((sector_id >> 16) & 0xFF) as u8;
        header[loc_idx + 1] = ((sector_id >> 8) & 0xFF) as u8;
        header[loc_idx + 2] = (sector_id & 0xFF) as u8;
        header[loc_idx + 3] = sector_count;
    }
    header
}

pub fn compress_and_wrap_chunk(nbt_data: &[u8]) -> Option<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), ZlibCompression::default());
    if encoder.write_all(nbt_data).is_ok() {
        if let Ok(compressed) = encoder.finish() {
            // Form the chunk "Packet": [Length: 4][Type: 1][Data...]
            // Type 2 = Zlib
            let total_len = (compressed.len() + 1) as u32; // +1 byte for Type
            let mut chunk_blob = Vec::new();
            chunk_blob.extend_from_slice(&total_len.to_be_bytes()); // Big Endian Length
            chunk_blob.push(2); 
            chunk_blob.extend_from_slice(&compressed);
            return Some(chunk_blob);
        }
    }
    None
}

/// Compression types used in Minecraft Anvil format
/// Same IDs as used by Pumpkin and vanilla Minecraft
pub mod compression {
    pub const GZIP: u8 = 1;
    pub const ZLIB: u8 = 2;
    pub const NONE: u8 = 3;
    pub const LZ4: u8 = 4;
}

/// Unwrap and decompress a chunk blob.
/// Supports GZip (1), ZLib (2), None (3), and LZ4 (4).
pub fn unwrap_and_decompress_chunk(chunk_blob: &[u8]) -> anyhow::Result<Vec<u8>> {
    if chunk_blob.len() < 5 {
        anyhow::bail!("Chunk blob too short");
    }
    
    // Parse header: [Length: 4 bytes][Type: 1 byte][Data...]
    let compression_type = chunk_blob[4];
    let compressed_data = &chunk_blob[5..];
    
    match compression_type {
        compression::ZLIB => {
            let mut decoder = flate2::read::ZlibDecoder::new(compressed_data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            Ok(decompressed)
        },
        compression::GZIP => {
            let mut decoder = flate2::read::GzDecoder::new(compressed_data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            Ok(decompressed)
        },
        compression::NONE => {
            Ok(compressed_data.to_vec())
        },
        compression::LZ4 => {
            // LZ4 using same library as Pumpkin (lz4-java-wrc)
            let mut decoder = lz4_java_wrc::Lz4BlockInput::new(compressed_data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            Ok(decompressed)
        },
        _ => anyhow::bail!("Unknown compression type: {}", compression_type),
    }
}

pub fn verify_chunk_coords(nbt_data: &[u8], expected_x: i32, expected_z: i32) -> anyhow::Result<()> {
    // Lightweight parse
    // Modern MC chunks are root compounds with xPos and zPos directly (since 1.18 or so)
    // Older (and maybe still valid) are { Level: { xPos, zPos } }
    
    // Attempt parsing
    let nbt: fastnbt::Value = fastnbt::from_bytes(nbt_data)?;
    
    if let fastnbt::Value::Compound(root) = nbt {
        // Check for direct xPos/zPos (Pumpkin/Modern)
        if let (Some(x_tag), Some(z_tag)) = (root.get("xPos"), root.get("zPos")) {
             let x = x_tag.as_i64().ok_or_else(|| anyhow::anyhow!("xPos is not an int"))? as i32;
             let z = z_tag.as_i64().ok_or_else(|| anyhow::anyhow!("zPos is not an int"))? as i32;
             
             if x != expected_x || z != expected_z {
                 anyhow::bail!("NBT Coords mismatch! Expected ({}, {}), Found ({}, {})", expected_x, expected_z, x, z);
             }
             return Ok(());
        }
        
        // Check for Level compound (Legacy/Vanilla)
        if let Some(level_tag) = root.get("Level") {
            if let fastnbt::Value::Compound(level) = level_tag {
                if let (Some(x_tag), Some(z_tag)) = (level.get("xPos"), level.get("zPos")) {
                     let x = x_tag.as_i64().ok_or_else(|| anyhow::anyhow!("Level.xPos is not an int"))? as i32;
                     let z = z_tag.as_i64().ok_or_else(|| anyhow::anyhow!("Level.zPos is not an int"))? as i32;
                     
                     if x != expected_x || z != expected_z {
                         anyhow::bail!("NBT Coords (Level) mismatch! Expected ({}, {}), Found ({}, {})", expected_x, expected_z, x, z);
                     }
                     return Ok(());
                }
            }
        }
        
        anyhow::bail!("Could not find xPos/zPos in NBT root or Level compound. Keys: {:?}", root.keys());
    } else {
        anyhow::bail!("NBT Root is not a Compound");
    }
}


pub fn get_chunk_coords_from_offset(offset: u64) -> Option<(i32, i32)> {
    if offset < HEADER_BYTES {
        return None; // Header, no chunks here
    }
    let data_offset = offset - HEADER_BYTES;
    let slot_size = SECTORS_PER_CHUNK * SECTOR_BYTES;
    
    let index = data_offset / slot_size;
    if index >= 1024 {
        return None; // Out of bounds
    }
    // Reverse math: x = index % 32, z = index / 32
    let rel_x = (index % 32) as i32;
    let rel_z = (index / 32) as i32;
    
    Some((rel_x, rel_z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_offset_0_0() {
        // 0,0 -> Index 0 -> Offset = Header (8192)
        let offset = get_chunk_file_offset(0, 0);
        assert_eq!(offset, HEADER_BYTES);
    }

    #[test]
    fn test_chunk_offset_31_0() {
        // 31,0 -> Index 31
        let offset = get_chunk_file_offset(31, 0);
        let expected = HEADER_BYTES + (31 * SECTORS_PER_CHUNK * SECTOR_BYTES);
        assert_eq!(offset, expected);
    }

    #[test]
    fn test_chunk_offset_0_1() {
        // 0,1 -> Index 32
        let offset = get_chunk_file_offset(0, 1);
        let expected = HEADER_BYTES + (32 * SECTORS_PER_CHUNK * SECTOR_BYTES);
        assert_eq!(offset, expected);
    }

    #[test]
    fn test_round_trip() {
        // Test all possible chunks in a region (32x32)
        for z in 0..32 {
            for x in 0..32 {
                let offset = get_chunk_file_offset(x, z);
                
                // Verify we point to the start of a chunk
                let (res_x, res_z) = get_chunk_coords_from_offset(offset).expect("Should find coords");
                assert_eq!(res_x, x, "Mismatch X");
                assert_eq!(res_z, z, "Mismatch Z");

                // Verify we point somewhere inside the chunk too
                let mid_offset = offset + 1234; // Random offset inside
                let (res_x_mid, res_z_mid) = get_chunk_coords_from_offset(mid_offset).expect("Should find coords inside");
                assert_eq!(res_x_mid, x);
                assert_eq!(res_z_mid, z);
            }
        }
    }

    #[test]
    fn test_out_of_bounds() {
        // Before header
        assert_eq!(get_chunk_coords_from_offset(0), None);
        assert_eq!(get_chunk_coords_from_offset(8191), None);

        // Way too far (Index 1024 starts at Header + 1024 * ChunkSize)
        let max_valid = HEADER_BYTES + (1024 * SECTORS_PER_CHUNK * SECTOR_BYTES);
        assert_eq!(get_chunk_coords_from_offset(max_valid), None); // First byte of next region (conceptually)
    }
}