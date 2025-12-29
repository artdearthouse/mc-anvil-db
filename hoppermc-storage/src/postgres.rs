use crate::nbt_json::{json_to_nbt, nbt_to_json};
use crate::{ChunkStorage, StorageMode};
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

pub struct PostgresStorage {
    pool: Pool,
    mode: StorageMode,
}

impl PostgresStorage {
    pub async fn new(connection_string: &str, mode: StorageMode) -> Result<Self> {
        let mut cfg = Config::new();
        cfg.url = Some(connection_string.to_string());
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)
            .context("Failed to create Postgres pool")?;

        // Ensure connections work and schema exists
        let storage = Self { pool, mode };
        storage.init_schema().await?;
        
        Ok(storage)
    }

    async fn init_schema(&self) -> Result<()> {
        let client = self.pool.get().await.context("Failed to get DB connection")?;
        
        match self.mode {
            StorageMode::PgRaw => {
                client.batch_execute("
                    CREATE TABLE IF NOT EXISTS chunks_raw (
                        x INT,
                        z INT,
                        data BYTEA,
                        updated_at TIMESTAMP DEFAULT NOW(),
                        PRIMARY KEY (x, z)
                    );
                ").await.context("Failed to init raw schema")?;
            }
            StorageMode::PgJsonb => {
                client.batch_execute("
                    CREATE TABLE IF NOT EXISTS chunks_jsonb (
                        x INT,
                        z INT,
                        data JSONB,
                        updated_at TIMESTAMP DEFAULT NOW(),
                        PRIMARY KEY (x, z)
                    );
                    CREATE INDEX IF NOT EXISTS idx_chunks_jsonb_data ON chunks_jsonb USING GIN (data);
                ").await.context("Failed to init jsonb schema")?;
            }
            _ => {
                log::warn!("Schema init for mode {:?} not yet implemented", self.mode);
            }
        }
        Ok(())
    }
}


#[async_trait]
impl ChunkStorage for PostgresStorage {
    async fn save_chunk(&self, x: i32, z: i32, data: &[u8]) -> Result<()> {
        let client = self.pool.get().await.context("Failed to get DB connection")?;

        match self.mode {
            StorageMode::PgRaw => {
                // Upsert logic
                client.execute(
                    "INSERT INTO chunks_raw (x, z, data, updated_at) 
                     VALUES ($1, $2, $3, NOW())
                     ON CONFLICT (x, z) DO UPDATE SET data = $3, updated_at = NOW()",
                    &[&x, &z, &data],
                ).await.context("Failed to insert chunk raw")?;
            }
            StorageMode::PgJsonb => {
                match fastnbt::from_bytes::<fastnbt::Value>(data) {
                    Ok(nbt_value) => {
                        let json_value = nbt_to_json(nbt_value);
                        client.execute(
                            "INSERT INTO chunks_jsonb (x, z, data, updated_at) 
                             VALUES ($1, $2, $3, NOW())
                             ON CONFLICT (x, z) DO UPDATE SET data = $3, updated_at = NOW()",
                            &[&x, &z, &json_value],
                        ).await.context("Failed to insert chunk jsonb")?;
                    }
                    Err(e) => {
                        log::error!("Failed to parse NBT for ({}, {}): {:?}", x, z, e);
                    }
                }
            }
            _ => anyhow::bail!("Save not implemented for mode {:?}", self.mode),
        }

        Ok(())
    }

    async fn load_chunk(&self, x: i32, z: i32) -> Result<Option<Vec<u8>>> {
        let client = self.pool.get().await.context("Failed to get DB connection")?;
        
        match self.mode {
             StorageMode::PgRaw => {
                 let rows = client.query(
                     "SELECT data FROM chunks_raw WHERE x = $1 AND z = $2",
                     &[&x, &z]
                 ).await?;
                 
                 if let Some(row) = rows.first() {
                     let data: Vec<u8> = row.get(0);
                     Ok(Some(data))
                 } else {
                     Ok(None)
                 }
             },
             StorageMode::PgJsonb => {
                 let row = client.query_opt("SELECT data FROM chunks_jsonb WHERE x = $1 AND z = $2", &[&x, &z]).await?;
                 if let Some(row) = row {
                     let json_value: serde_json::Value = row.get(0);
                     let nbt_value = json_to_nbt(json_value);
                     match fastnbt::to_bytes(&nbt_value) {
                         Ok(nbt_data) => Ok(Some(nbt_data)),
                         Err(e) => {
                             log::error!("Failed to encode NBT for ({}, {}): {:?}", x, z, e);
                             Ok(None)
                         }
                     }
                 } else {
                     Ok(None)
                 }
             }
             _ => Ok(None)
        }
    }

    async fn get_total_size(&self) -> Result<u64> {
        let client = self.pool.get().await.context("Failed to get DB connection")?;
        
        match self.mode {
            StorageMode::PgRaw => {
                let row = client.query_one("SELECT pg_total_relation_size('chunks_raw')", &[]).await?;
                let size: i64 = row.get(0);
                Ok(size as u64)
            }
            StorageMode::PgJsonb => {
                let row = client.query_one("SELECT pg_total_relation_size('chunks_jsonb')", &[]).await?;
                let size: i64 = row.get(0);
                Ok(size as u64)
            }
            _ => Ok(0)
        }
    }
}
