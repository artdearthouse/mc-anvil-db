use std::collections::HashMap;

#[derive(Default)]
pub struct ChunkBuilder {
    // We store blocks in a sparse map for simplicity in MVP.
    // Key: (x, y, z), Value: Block Name
    // This isn't the most efficient (VoxelGrid is faster), but it's the easiest to write "set_block".
    // For full layers we will handle efficient filling during build().
    custom_blocks: HashMap<(u8, i32, u8), String>,
    
    // Optimisation for layers:
    // Key: y, Value: Block Name
    full_layers: HashMap<i32, String>,
}

impl ChunkBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a single block at chunk-local coordinates (x: 0..15, z: 0..15)
    pub fn set_block(&mut self, x: u8, y: i32, z: u8, name: &str) {
        if x < 16 && z < 16 {
            self.custom_blocks.insert((x, y, z), name.to_string());
        }
    }

    /// Fill an entire Y-layer with a block (efficiently)
    pub fn fill_layer(&mut self, y: i32, name: &str) {
        self.full_layers.insert(y, name.to_string());
        // Remove individual blocks at this Y to save memory/logic, they are overwritten
        self.custom_blocks.retain(|(_, by, _), _| *by != y);
    }

    pub fn build(self, chunk_x: i32, chunk_z: i32, rt: &tokio::runtime::Handle) -> anyhow::Result<Vec<u8>> {
        use pumpkin_world::chunk::{ChunkData, ChunkSections, SubChunk, ChunkHeightmaps, ChunkLight};
        use pumpkin_world::chunk::format::LightContainer;
        use pumpkin_data::chunk::ChunkStatus; // Explicitly import Status
        use pumpkin_data::Block; // Correct import
        use pumpkin_world::chunk::format::anvil::SingleChunkDataSerializer;
        use std::collections::HashMap;

        // 1. Create Sections
        // -64 to 320 = 384 blocks = 24 sections
        let sections_vec: Vec<SubChunk> = std::iter::repeat_with(|| SubChunk::default())
            .take(24)
            .collect();
        let mut sections = ChunkSections::new(sections_vec.into_boxed_slice(), -64);

        // 2. Apply Full Layers
        for (y, name) in &self.full_layers {
            // Strip namespace if needed
            let name_key = name.strip_prefix("minecraft:").unwrap_or(name);
            
            if let Some(block) = Block::from_registry_key(name_key) {
                let state_id = block.default_state.id;
                // Set for 16x16
                for x in 0..16 {
                    for z in 0..16 {
                         sections.set_block_absolute_y(x, *y, z, state_id);
                    }
                }
            }
        }

        // 3. Apply Custom Blocks
        for ((x, y, z), name) in &self.custom_blocks {
            let name_key = name.strip_prefix("minecraft:").unwrap_or(name);
            if let Some(block) = Block::from_registry_key(name_key) {
                sections.set_block_absolute_y(*x as usize, *y, *z as usize, block.default_state.id);
            }
        }

        // 4. Construct ChunkData
        let chunk_data = ChunkData {
            section: sections,
            heightmap: ChunkHeightmaps::default(), // TODO: Calculate?
            x: chunk_x,
            z: chunk_z,
            block_ticks: Default::default(),
            fluid_ticks: Default::default(),
            block_entities: HashMap::new(),
            light_engine: ChunkLight {
                sky_light: std::iter::repeat_with(LightContainer::default)
                    .take(24)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                block_light: std::iter::repeat_with(LightContainer::default)
                    .take(24)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            },
            status: ChunkStatus::Full, // Correct enum?
            dirty: false,
        };

        // 5. Serialize using passed runtime handle (no new runtime creation!)
        let bytes = rt.block_on(async move {
            chunk_data.to_bytes().await
        }).map_err(|e| anyhow::anyhow!("Serialization error: {:?}", e))?;

        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_builds_valid_pumpkin_chunk() {
        let mut builder = ChunkBuilder::new();
        builder.fill_layer(0, "minecraft:stone");
        
        let result = builder.build(0, 0);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
    
    #[test]
    fn test_coordinate_serialization() {
        let mut builder = ChunkBuilder::new();
        // Coords from the bug
        // Expected (32, 13), Found (37, 28)
        let x = 32;
        let z = 13;
        let result = builder.build(x, z).unwrap();
        
        let nbt: fastnbt::Value = fastnbt::from_bytes(result.as_slice()).expect("Should be valid NBT");
        println!("NBT: {:?}", nbt);
        
        if let fastnbt::Value::Compound(map) = nbt {
             let x_tag = map.get("xPos").expect("Should have xPos");
             let z_tag = map.get("zPos").expect("Should have zPos");
             
             let x_val = x_tag.as_i64().expect("xPos should be int");
             let z_val = z_tag.as_i64().expect("zPos should be int");
             
             assert_eq!(x_val, x as i64, "xPos mismatch");
             assert_eq!(z_val, z as i64, "zPos mismatch");
        } else {
             panic!("Root should be Compound");
        }
    }
}
