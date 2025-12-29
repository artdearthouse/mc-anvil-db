use crate::WorldGenerator;
use pumpkin_world::generation::proto_chunk::{ProtoChunk, TerrainCache};
use pumpkin_world::generation::settings::{GeneratorSetting, GENERATION_SETTINGS};
use pumpkin_world::generation::{GlobalRandomConfig, biome_coords};
use pumpkin_world::generation::noise::router::proto_noise_router::ProtoNoiseRouters;
use pumpkin_world::biome::hash_seed;
use pumpkin_world::chunk::{ChunkData, ChunkSections, SubChunk, ChunkLight, ChunkHeightmaps};
use pumpkin_world::chunk::format::LightContainer;
use pumpkin_world::chunk::format::anvil::SingleChunkDataSerializer;
use pumpkin_world::chunk::palette::{BlockPalette, BiomePalette};
use pumpkin_world::dimension::Dimension;
use pumpkin_data::chunk::ChunkStatus;
use pumpkin_data::noise_router::{OVERWORLD_BASE_NOISE_ROUTER, NETHER_BASE_NOISE_ROUTER, END_BASE_NOISE_ROUTER};
use anyhow::Result;
use std::collections::HashMap;

/// Vanilla-style world generator using Pumpkin's VanillaGenerator
/// Generates realistic Minecraft terrain with biomes, caves, ores, etc.
pub struct VanillaWorldGenerator {
    dimension: Dimension,
    random_config: GlobalRandomConfig,
    noise_router: ProtoNoiseRouters,
    terrain_cache: TerrainCache,
}

impl VanillaWorldGenerator {
    pub fn new(seed: u64) -> Self {
        Self::with_dimension(seed, Dimension::Overworld)
    }
    
    pub fn with_dimension(seed: u64, dimension: Dimension) -> Self {
        // Initialize noise configuration (cached, reused for all chunks)
        let random_config = GlobalRandomConfig::new(seed, false);
        
        // Get base noise router for dimension
        let base_router = match dimension {
            Dimension::Overworld => &OVERWORLD_BASE_NOISE_ROUTER,
            Dimension::Nether => &NETHER_BASE_NOISE_ROUTER,
            Dimension::End => &END_BASE_NOISE_ROUTER,
        };
        let noise_router = ProtoNoiseRouters::generate(base_router, &random_config);
        
        // Create terrain cache for surface generation
        let terrain_cache = TerrainCache::from_random(&random_config);
        
        Self {
            dimension,
            random_config,
            noise_router,
            terrain_cache,
        }
    }
    
    fn get_settings(&self) -> &'static pumpkin_world::generation::settings::GenerationSettings {
        let setting = match self.dimension {
            Dimension::Overworld => GeneratorSetting::Overworld,
            Dimension::Nether => GeneratorSetting::Nether,
            Dimension::End => GeneratorSetting::End,
        };
        GENERATION_SETTINGS.get(&setting).expect("Generation settings not found")
    }
}

impl WorldGenerator for VanillaWorldGenerator {
    /// Generates a chunk at the specified coordinates using Pumpkin's staged generation.
    /// 
    /// # Arguments
    /// * `x` - Chunk X coordinate (in chunk coordinates, not blocks)
    /// * `z` - Chunk Z coordinate (in chunk coordinates, not blocks)
    /// * `rt` - Tokio runtime handle for async serialization (reused, not created per-chunk)
    /// 
    /// # Generation Pipeline
    /// 1. Create ProtoChunk with default block (stone/netherrack/end_stone)
    /// 2. `step_to_biomes()` - Populate biome data using noise sampling
    /// 3. `step_to_noise()` - Generate terrain heightmap and 3D density
    /// 4. `step_to_surface()` - Apply surface rules (grass, sand, snow, etc.)
    /// 5. Convert `ProtoChunk` → `ChunkData` (copy blocks/biomes to sections)
    /// 6. Serialize to NBT bytes
    fn generate_chunk(&self, x: i32, z: i32, rt: &tokio::runtime::Handle, benchmark: Option<&hoppermc_benchmark::BenchmarkMetrics>) -> Result<Vec<u8>> {
        let start_noise = std::time::Instant::now();
        let settings = self.get_settings();
        let default_block = settings.default_block.get_state();
        let biome_mixer_seed = hash_seed(self.random_config.seed);
        
        // Create ProtoChunk for generation
        let mut proto_chunk = ProtoChunk::new(x, z, settings, default_block, biome_mixer_seed);
        
        // Step 1: Populate biomes
        proto_chunk.step_to_biomes(self.dimension.clone(), &self.noise_router);
        
        // Step 2: Populate noise (terrain)
        proto_chunk.step_to_noise(settings, &self.random_config, &self.noise_router);
        
        // Step 3: Build surface (grass, sand, etc.)
        proto_chunk.step_to_surface(settings, &self.random_config, &self.terrain_cache, &self.noise_router);
        
        // Convert ProtoChunk to ChunkData (adapted from Pumpkin's upgrade_to_level_chunk)
        let chunk_data = self.proto_to_chunk_data(&proto_chunk, settings);
        
        if let Some(bench) = benchmark {
            bench.record_generation_noise(start_noise.elapsed());
        }

        // Serialize to bytes using passed runtime handle (no per-chunk runtime creation!)
        let start_ser = std::time::Instant::now();
        let bytes = rt.block_on(async move {
            chunk_data.to_bytes().await
        }).map_err(|e| anyhow::anyhow!("Serialization error: {:?}", e))?;

        if let Some(bench) = benchmark {
             bench.record_serialization(start_ser.elapsed());
        }

        Ok(bytes.to_vec())
    }
}

impl VanillaWorldGenerator {
    /// Converts ProtoChunk to ChunkData (adapted from Pumpkin's upgrade_to_level_chunk)
    fn proto_to_chunk_data(
        &self,
        proto_chunk: &ProtoChunk,
        settings: &pumpkin_world::generation::settings::GenerationSettings,
    ) -> ChunkData {
        let sub_chunks = settings.shape.height as usize / BlockPalette::SIZE;
        let sections_vec: Vec<SubChunk> = (0..sub_chunks).map(|_| SubChunk::default()).collect();
        let mut sections = ChunkSections::new(sections_vec.into_boxed_slice(), settings.shape.min_y as i32);

        // Copy biomes from proto_chunk to sections
        for y in 0..biome_coords::from_block(settings.shape.height) {
            let relative_y = y as usize;
            let section_index = relative_y / BiomePalette::SIZE;
            let relative_y_in_section = relative_y % BiomePalette::SIZE;
            
            if let Some(section) = sections.sections.get_mut(section_index) {
                for z in 0..BiomePalette::SIZE {
                    for local_x in 0..BiomePalette::SIZE {
                        let absolute_y = biome_coords::from_block(settings.shape.min_y as i32) + y as i32;
                        let biome = proto_chunk.get_biome(local_x as i32, absolute_y, z as i32);
                        section.biomes.set(local_x, relative_y_in_section, z, biome.id);
                    }
                }
            }
        }
        
        // Copy blocks from proto_chunk to sections
        for y in 0..settings.shape.height {
            let relative_y = y as usize;
            let section_index = relative_y / BlockPalette::SIZE;
            let relative_y_in_section = relative_y % BlockPalette::SIZE;
            
            if let Some(section) = sections.sections.get_mut(section_index) {
                for z in 0..BlockPalette::SIZE {
                    for local_x in 0..BlockPalette::SIZE {
                        let block = proto_chunk.get_block_state_raw(local_x as i32, y as i32, z as i32);
                        section.block_states.set(local_x, relative_y_in_section, z, block);
                    }
                }
            }
        }

        // Create ChunkData
        let mut chunk = ChunkData {
            section: sections,
            heightmap: ChunkHeightmaps::default(),
            x: proto_chunk.x,
            z: proto_chunk.z,
            block_ticks: Default::default(),
            fluid_ticks: Default::default(),
            block_entities: HashMap::new(),
            light_engine: ChunkLight {
                sky_light: std::iter::repeat_with(|| LightContainer::new_filled(15))
                    .take(sub_chunks)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                block_light: std::iter::repeat_with(|| LightContainer::new_empty(0))
                    .take(sub_chunks)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            },
            status: ChunkStatus::Full,
            dirty: false,
        };

        // Calculate heightmap
        chunk.heightmap = chunk.calculate_heightmap();
        chunk
    }
}
