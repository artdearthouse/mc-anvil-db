use std::sync::{Arc, Mutex};
use hoppermc_gen::WorldGenerator;
use hoppermc_anvil as region;
use hoppermc_storage::ChunkStorage;
use hoppermc_benchmark::BenchmarkMetrics;
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct VirtualFile {
    pub generator: Arc<dyn WorldGenerator>,
    pub storage: Option<Arc<dyn ChunkStorage>>,
    pub rt: tokio::runtime::Handle,
    pub benchmark: Option<Arc<BenchmarkMetrics>>,
    pub cache: Mutex<LruCache<(i32, i32), Vec<u8>>>,
}

impl VirtualFile {
    pub fn new(
        generator: Arc<dyn WorldGenerator>, 
        storage: Option<Arc<dyn ChunkStorage>>, 
        rt: tokio::runtime::Handle,
        benchmark: Option<Arc<BenchmarkMetrics>>,
        cache_size: usize,
    ) -> Self {
        let cap = NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(500).unwrap());
        Self { 
            generator, 
            storage, 
            rt, 
            benchmark,
            cache: Mutex::new(LruCache::new(cap)),
        }
    }

    pub fn read_at(&self, offset: u64, size: usize, region_x: i32, region_z: i32) -> Vec<u8> {
        let mut response_data = Vec::with_capacity(size);

        // --- 1. HEADER GENERATION (0..8192) ---
        if offset < region::HEADER_BYTES {
            let header = region::generate_header();
            
            // Debug: Log first few bytes
            if offset == 0 {
                log::info!("Header Generated. Bytes 0..16: {:02X?}", &header[0..16]);
            }

            let start_in_header = offset as usize;
            let end_in_header = std::cmp::min(start_in_header + size, region::HEADER_BYTES as usize);
            if start_in_header < region::HEADER_BYTES as usize {
                response_data.extend_from_slice(&header[start_in_header..end_in_header]);
            }
        }

        // --- 2. CHUNK DATA GENERATION (8192+) ---
        while response_data.len() < size {
            let current_len = response_data.len();
            let data_read_offset = offset + current_len as u64;
            let needed = size - current_len;

            if let Some((rel_x, rel_z)) = region::get_chunk_coords_from_offset(data_read_offset) {
                // Generate chunk with ABSOLUTE coordinates
                let abs_x = region_x * 32 + rel_x;
                let abs_z = region_z * 32 + rel_z;
                
                // Check Cache first
                let cached_blob: Option<Vec<u8>> = {
                    let mut cache = self.cache.lock().unwrap();
                    cache.get(&(abs_x, abs_z)).cloned()
                };
                
                let chunk_blob = if let Some(blob) = cached_blob {
                    blob
                } else {
                    // CACHE MISS - Load/Generate
                    
                    // 1. Try to load from Storage first (if storage is enabled)
                    let nbt_res = if let Some(storage) = &self.storage {
                        let start = std::time::Instant::now();
                        let storage_data = self.rt.block_on(async {
                            storage.load_chunk(abs_x, abs_z).await
                        });
                        if let Some(bench) = &self.benchmark {
                            bench.record_load(start.elapsed());
                        }

                        match storage_data {
                            Ok(Some(raw_nbt)) => {
                                // Found in DB! Verify consistency
                                if let Err(e) = region::verify_chunk_coords(&raw_nbt, abs_x, abs_z) {
                                    log::error!("CRITICAL: DB Corruption detected for ({}, {}). Error: {:?}. Discarding and regenerating.", abs_x, abs_z, e);
                                    // Generation Fallback
                                    let start_gen = std::time::Instant::now();
                                    let res = self.generator.generate_chunk(abs_x, abs_z, &self.rt);
                                    if let Some(bench) = &self.benchmark { bench.record_generation(start_gen.elapsed()); }
                                    res
                                } else {
                                    Ok(raw_nbt)
                                }
                            },
                            Ok(None) => {
                                // Not in DB, generate it
                                let start_gen = std::time::Instant::now();
                                let res = self.generator.generate_chunk(abs_x, abs_z, &self.rt);
                                if let Some(bench) = &self.benchmark { bench.record_generation(start_gen.elapsed()); }
                                res
                            },
                            Err(e) => {
                                log::error!("Error loading chunk from DB: {:?}", e);
                                let start_gen = std::time::Instant::now();
                                let res = self.generator.generate_chunk(abs_x, abs_z, &self.rt);
                                if let Some(bench) = &self.benchmark { bench.record_generation(start_gen.elapsed()); }
                                res
                            }
                        }
                    } else {
                        // No storage - always generate
                        let start_gen = std::time::Instant::now();
                        let res = self.generator.generate_chunk(abs_x, abs_z, &self.rt);
                        if let Some(bench) = &self.benchmark { bench.record_generation(start_gen.elapsed()); }
                        res
                    };

                    match nbt_res {
                        Ok(nbt_data) => {
                            // Verify generated/resultant consistency
                            if let Err(e) = region::verify_chunk_coords(&nbt_data, abs_x, abs_z) {
                                log::error!("CRITICAL: Generated chunk coords mismatch for ({}, {}): {:?}", abs_x, abs_z, e);
                                break; // Broken generator
                            }

                            if let Some(blob) = region::compress_and_wrap_chunk(&nbt_data) {
                                // Update Cache
                                self.cache.lock().unwrap().put((abs_x, abs_z), blob.clone());
                                blob
                            } else {
                                break; // Compression fail
                            }
                        },
                        Err(e) => {
                            log::error!("Failed to generate/load chunk: {:?}", e);
                            break;
                        }
                    }
                };
                
                // Now we have the chunk_blob (from cache or fresh)
                let chunk_start_file_offset = region::get_chunk_file_offset(rel_x, rel_z);
                
                if data_read_offset >= chunk_start_file_offset {
                    let local_offset = (data_read_offset - chunk_start_file_offset) as usize;
                    
                    if local_offset < chunk_blob.len() {
                        let available = chunk_blob.len() - local_offset;
                        let to_copy = std::cmp::min(available, needed);
                        response_data.extend_from_slice(&chunk_blob[local_offset..local_offset + to_copy]);
                        continue; 
                    } else {
                         // sparse filling
                        let chunk_end_offset = chunk_start_file_offset + (region::SECTORS_PER_CHUNK as u64 * region::SECTOR_BYTES);
                        let zeros_available = chunk_end_offset.saturating_sub(data_read_offset);
                        let zeros_to_give = std::cmp::min(zeros_available as usize, needed);
                        
                        response_data.resize(current_len + zeros_to_give, 0);
                        continue;
                    }
                } else {
                     break;
                } 
            }
            
            // If we are here, we failed to map to a chunk (EOF or Error) or Generation Failed
            break;
        }
        
        // Pad with zeros if something is missing
        if response_data.len() < size {
             response_data.resize(size, 0);
        }

        response_data
    }
    pub fn write_at(&self, offset: u64, data: &[u8], region_x: i32, region_z: i32) {
        // --- WRITE INTERCEPTION ---
        // If writing to header area (0..8192) -> Ignore (it's virtual).
        // If writing data area:
        if offset >= region::HEADER_BYTES {
             // 1. Identify which chunk this is
             if let Some((rel_x, rel_z)) = region::get_chunk_coords_from_offset(offset) {
                 // 2. We only support "full chunk writes" for now.
                 
                 // Check if data looks like a chunk:
                 // 4 bytes length + 1 byte type + data.
                 // We rely on unwrap_and_decompress_chunk to validate.
                 
                 if let Ok(raw_nbt) = region::unwrap_and_decompress_chunk(data) {
                     let abs_x = region_x * 32 + rel_x;
                     let abs_z = region_z * 32 + rel_z;
                     
                     // Verify consistency and correct if necessary
                     let (save_x, save_z) = match region::verify_chunk_coords(&raw_nbt, abs_x, abs_z) {
                         Ok(_) => {
                             // Correct coords
                             (abs_x, abs_z)
                         },
                         Err(_) => {
                             // Mismatch! Extract real coords from NBT to trust them.
                             let mut real_x = abs_x;
                             let mut real_z = abs_z;
                             
                             if let Ok(real_nbt) = fastnbt::from_bytes::<fastnbt::Value>(&raw_nbt) {
                                  if let fastnbt::Value::Compound(root) = &real_nbt {
                                      let (x, z) = if let (Some(x), Some(z)) = (root.get("xPos"), root.get("zPos")) {
                                            (x.as_i64(), z.as_i64())
                                      } else if let Some(fastnbt::Value::Compound(level)) = root.get("Level") {
                                            (
                                                level.get("xPos").and_then(|v| v.as_i64()), 
                                                level.get("zPos").and_then(|v| v.as_i64())
                                            )
                                      } else {
                                          (None, None)
                                      };
                                      
                                      if let (Some(rx), Some(rz)) = (x, z) {
                                          real_x = rx as i32;
                                          real_z = rz as i32;
                                      }
                                  }
                             }
                             log::debug!("CORRECTION: Intercepted write at offset for ({}, {}), but NBT contains ({}, {}). Saving to DB as ({}, {}).", abs_x, abs_z, real_x, real_z, real_x, real_z);
                             (real_x, real_z)
                         }
                     };
                     
                     log::info!("Intercepted write for Chunk ({}, {}). Size: {} bytes.", save_x, save_z, raw_nbt.len());
                     
                     // 3. Save to DB (if storage is enabled)
                     if let Some(storage) = &self.storage {
                         let start = std::time::Instant::now();
                         let result = self.rt.block_on(async {
                             storage.save_chunk(save_x, save_z, &raw_nbt).await
                         });
                         if let Some(bench) = &self.benchmark {
                            bench.record_save(start.elapsed());
                         }
                         
                         if let Err(e) = result {
                             log::error!("Failed to save chunk ({}, {}) to DB: {:?}", abs_x, abs_z, e);
                         } else {
                             log::debug!("Chunk ({}, {}) saved to DB successfully.", save_x, save_z);
                             
                             // Update Cache with NEW BLOB
                             if let Some(new_blob) = region::compress_and_wrap_chunk(&raw_nbt) {
                                 let mut cache = self.cache.lock().unwrap();
                                 cache.put((save_x, save_z), new_blob);
                             }
                         }
                     } else {
                         log::debug!("Storage disabled, skipping save for chunk ({}, {}).", save_x, save_z);
                     }
                 } else {
                     log::warn!("Write to chunk data area at offset {} (len {}) failed decompression/validation. Maybe partial write?", offset, data.len());
                 }
             }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use async_trait::async_trait;

    struct MockGenerator;
    impl WorldGenerator for MockGenerator {
        fn generate_chunk(&self, _x: i32, _z: i32) -> Result<Vec<u8>> {
            // Return dummy NBT data
            Ok(vec![1, 2, 3, 4])
        }
    }

    struct MockStorage;
    #[async_trait]
    impl ChunkStorage for MockStorage {
        async fn save_chunk(&self, _x: i32, _z: i32, _data: &[u8]) -> Result<()> {
            Ok(())
        }
        async fn load_chunk(&self, _x: i32, _z: i32) -> Result<Option<Vec<u8>>> {
            Ok(None)
        }
    }

    #[test]
    fn test_virtual_file_read_header() {
        let generator = Arc::new(MockGenerator);
        let storage = Arc::new(MockStorage);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vf = VirtualFile::new(generator, storage, rt.handle().clone());

        // Read first 10 bytes of header. Region 0,0
        let data = vf.read_at(0, 10, 0, 0);
        assert_eq!(data.len(), 10);
    }

    #[test]
    fn test_virtual_file_read_chunk_offset() {
        let generator = Arc::new(MockGenerator);
        let storage = Arc::new(MockStorage);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vf = VirtualFile::new(generator, storage, rt.handle().clone());

        // Calculate offset for chunk 0,0
        // Header is 8192 bytes
        let chunk_offset = region::get_chunk_file_offset(0, 0); 
        
        // Read 5 bytes from there. Region 0,0
        let data = vf.read_at(chunk_offset, 5, 0, 0);
        assert_eq!(data.len(), 5);
        
        // The first 4 bytes are length (big endian). 
        // Our mock returns 4 bytes [1,2,3,4]. Compressed it will be larger.
        // But we can check it's not all zeros.
        assert_ne!(data, vec![0, 0, 0, 0, 0]);
    }
}
