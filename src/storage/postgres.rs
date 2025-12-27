use std::sync::Arc;
use std::io::Write;
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;
use tokio::runtime::Runtime;
use tokio_postgres::NoTls;
use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod, Pool, Runtime as PoolRuntime};

use crate::storage::{ChunkPos, ChunkStorage};
use crate::nbt::ChunkData;

pub struct PostgresStorage {
    pool: Pool,
    rt: Arc<Runtime>,
}

impl PostgresStorage {
    pub fn new(database_url: &str) -> Self {
        let mut cfg = Config::new();
        cfg.url = Some(database_url.to_string());
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg.create_pool(Some(PoolRuntime::Tokio1), NoTls).unwrap();
        
        // Create a runtime for bridging async/sync
        let rt = Runtime::new().unwrap();

        // Initialize schema
        rt.block_on(async {
            let client = pool.get().await.expect("Failed to connect to Postgres");
            client.execute(
                "CREATE TABLE IF NOT EXISTS chunks (
                    x INT,
                    z INT,
                    data JSONB,
                    updated_at TIMESTAMP DEFAULT NOW(),
                    PRIMARY KEY (x, z)
                )",
                &[],
            ).await.expect("Failed to init schema");
        });

        Self {
            pool,
            rt: Arc::new(rt),
        }
    }

    fn compress_nbt(nbt_data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(nbt_data).expect("Compression failed");
        encoder.finish().expect("Compression finish failed")
    }

    fn decompress_nbt(compressed: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(compressed);
        let mut decoded = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decoded)?;
        Ok(decoded)
    }
}

impl ChunkStorage for PostgresStorage {
    fn get(&self, pos: ChunkPos) -> Option<Vec<u8>> {
        self.rt.block_on(async {
            let client = self.pool.get().await.ok()?;
            
            let row = client.query_opt(
                "SELECT data FROM chunks WHERE x = $1 AND z = $2",
                &[&pos.x, &pos.z],
            ).await.ok()?;

            if let Some(row) = row {
                let json_data: serde_json::Value = row.get(0);
                
                // Conversion: JSON -> Struct -> NBT -> Compressed Bytes
                let chunk: ChunkData = serde_json::from_value(json_data)
                    .map_err(|e| log::error!("JSON deserialize error: {}", e)).ok()?;
                
                let nbt_bytes = fastnbt::to_bytes(&chunk)
                    .map_err(|e| log::error!("NBT serialize error: {}", e)).ok()?;
                
                let compressed = Self::compress_nbt(&nbt_bytes);

                // Add MCA Header: [len:4][type:1][data]
                let mut result = Vec::with_capacity(5 + compressed.len());
                let total_len = (compressed.len() + 1) as u32;
                result.extend_from_slice(&total_len.to_be_bytes());
                result.push(2); // Zlib
                result.extend_from_slice(&compressed);

                Some(result)
            } else {
                None
            }
        })
    }

    fn set(&self, pos: ChunkPos, data: Vec<u8>) {
        // Parse incoming MCA bytes: [len:4][type:1][compressed_data]
        if data.len() < 5 {
            return;
        }
        
        // Skip header (5 bytes)
        let compressed = &data[5..];
        
        // Decompress
        match Self::decompress_nbt(compressed) {
            Ok(nbt_bytes) => {
                // NBT -> Struct
                match fastnbt::from_bytes::<ChunkData>(&nbt_bytes) {
                    Ok(chunk) => {
                        // Struct -> JSON
                        match serde_json::to_value(&chunk) {
                            Ok(json_data) => {
                                // Async Insert
                                self.rt.block_on(async {
                                    log::info!("Postgres: Inserting chunk ({}, {})", pos.x, pos.z);
                                    if let Ok(client) = self.pool.get().await {
                                        match client.execute(
                                            "INSERT INTO chunks (x, z, data) VALUES ($1, $2, $3)
                                             ON CONFLICT (x, z) DO UPDATE SET data = $3, updated_at = NOW()",
                                            &[&pos.x, &pos.z, &json_data],
                                        ).await {
                                            Ok(_) => log::info!("Postgres: Write success for ({}, {})", pos.x, pos.z),
                                            Err(e) => log::error!("Postgres: Write failed: {}", e),
                                        }
                                    } else {
                                        log::error!("Postgres: Failed to get connection from pool");
                                    }
                                });
                            },
                            Err(e) => log::error!("Failed to convert chunk to JSON: {}", e),
                        }
                    },
                    Err(e) => log::error!("Failed to parse NBT: {}", e),
                }
            },
            Err(e) => log::error!("Failed to decompress chunk: {}", e),
        }
    }

    fn delete(&self, pos: ChunkPos) {
        self.rt.block_on(async {
            if let Ok(client) = self.pool.get().await {
                let _ = client.execute(
                    "DELETE FROM chunks WHERE x = $1 AND z = $2",
                    &[&pos.x, &pos.z],
                ).await;
            }
        });
    }

    fn list_chunks(&self) -> Vec<ChunkPos> {
        // Warning: heavy operation
        self.rt.block_on(async {
            let mut chunks = Vec::new();
            if let Ok(client) = self.pool.get().await {
                if let Ok(rows) = client.query("SELECT x, z FROM chunks", &[]).await {
                    for row in rows {
                        chunks.push(ChunkPos::new(row.get(0), row.get(1)));
                    }
                }
            }
            chunks
        })
    }

    fn get_region_chunks(&self, region: crate::region::RegionPos) -> Vec<ChunkPos> {
        self.rt.block_on(async {
            let mut chunks = Vec::new();
            if let Ok(client) = self.pool.get().await {
                // Determine chunk coordinate range for this region
                // Region 0,0 -> Chunks 0..31
                // Region -1,-1 -> Chunks -32..-1
                let min_x = region.x * 32;
                let max_x = min_x + 31;
                let min_z = region.z * 32;
                let max_z = min_z + 31;

                if let Ok(rows) = client.query(
                    "SELECT x, z FROM chunks WHERE x >= $1 AND x <= $2 AND z >= $3 AND z <= $4", 
                    &[&min_x, &max_x, &min_z, &max_z]
                ).await {
                    for row in rows {
                        chunks.push(ChunkPos::new(row.get(0), row.get(1)));
                    }
                }
            }
            chunks
        })
    }
}
