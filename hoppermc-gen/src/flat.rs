

use crate::WorldGenerator;
use crate::builder::ChunkBuilder;
use tokio::runtime::Handle;

pub struct FlatGenerator;

impl WorldGenerator for FlatGenerator {
    fn generate_chunk(&self, x: i32, z: i32, rt: &Handle, _benchmark: Option<&hoppermc_benchmark::BenchmarkMetrics>) -> anyhow::Result<Vec<u8>> {
        let mut builder = ChunkBuilder::new();

        // 1. Bedrock Floor (Y=-64)
        builder.fill_layer(-64, "minecraft:bedrock");

        // 2. Stone Layers (-63..-4) - The logic missing previously
        for y in -63..-4 {
            builder.fill_layer(y, "minecraft:stone");
        }

        // 3. Dirt Layers (-4..=-1) - Inclusive range to fill Y=-1
        for y in -4..=-1 {
            builder.fill_layer(y, "minecraft:dirt");
        }

        // 4. Grass Block (Y=0)
        builder.fill_layer(0, "minecraft:grass_block");

        // 4. TEST: A Stone Pillar at (8, Y, 8) to prove it's 3D
        for y in 0..10 {
            builder.set_block(8, y, 8, "minecraft:stone");
        }

        builder.build(x, z, rt)
    }
}