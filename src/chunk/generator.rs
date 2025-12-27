//! Procedural chunk generation.
//!
//! Generates flat world chunks with configurable layers.

use std::io::Write;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::nbt::{ChunkData, Section, BlockStates, Biomes, BlockState, get_data_version};

/// Procedural chunk generator.
///
/// Currently generates a simple flat world with:
/// - Dirt layer at Y=-64 (section Y=-4)
/// - Air everywhere else
pub struct Generator {
    // Future: configuration for world generation
}

impl Generator {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate a chunk at the given world coordinates.
    /// Returns MCA-formatted bytes: [length:4][compression:1][compressed_nbt:N]
    pub fn generate(&self, chunk_x: i32, chunk_z: i32) -> std::io::Result<Vec<u8>> {
        let mut sections = Vec::with_capacity(24);

        // Generate sections from Y=-4 to Y=19 (total height: 384 blocks)
        for section_y in -4..20i8 {
            let block_name = if section_y == -4 {
                "minecraft:dirt"
            } else {
                "minecraft:air"
            };

            let palette = vec![BlockState {
                name: block_name.to_string(),
            }];

            sections.push(Section {
                y: section_y,
                block_states: Some(BlockStates { 
                    palette,
                    data: None, // Single entry palette, no data needed
                }),
                biomes: Some(Biomes {
                    palette: vec!["minecraft:plains".to_string()],
                    data: None,
                }),
            });
        }

        let chunk = ChunkData {
            data_version: get_data_version(),
            x_pos: chunk_x,
            z_pos: chunk_z,
            y_pos: -4,
            status: "minecraft:full".to_string(),
            last_update: 0,
            inhabited_time: 0,
            is_light_on: Some(1),
            sections,
        };

        // Serialize to NBT
        let nbt_data = fastnbt::to_bytes(&chunk)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Compress with Zlib
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&nbt_data)?;
        let compressed = encoder.finish()?;

        // Pack in MCA format: [length:4][type:1][data:N]
        let mut result = Vec::with_capacity(5 + compressed.len());
        let total_len = (compressed.len() + 1) as u32;
        result.extend_from_slice(&total_len.to_be_bytes());
        result.push(2); // Compression type 2 = Zlib
        result.extend_from_slice(&compressed);

        Ok(result)
    }
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}
