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
fn nbt_to_json(nbt: fastnbt::Value) -> serde_json::Value {
    match nbt {
        fastnbt::Value::Compound(c) => {
            let mut map = serde_json::Map::new();
            for (k, v) in c {
                map.insert(k, nbt_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        fastnbt::Value::List(l) => {
            serde_json::Value::Array(l.into_iter().map(nbt_to_json).collect())
        }
        fastnbt::Value::String(s) => serde_json::Value::String(s),
        fastnbt::Value::Byte(b) => serde_json::Value::Number(b.into()),
        fastnbt::Value::Short(s) => serde_json::Value::Number(s.into()),
        fastnbt::Value::Int(i) => serde_json::Value::Number(i.into()),
        fastnbt::Value::Long(l) => serde_json::Value::Number(l.into()),
        fastnbt::Value::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(f as f64).unwrap_or(serde_json::Number::from(0))),
        fastnbt::Value::Double(d) => serde_json::Value::Number(serde_json::Number::from_f64(d).unwrap_or(serde_json::Number::from(0))),
        fastnbt::Value::ByteArray(ba) => {
            let mut map = serde_json::Map::new();
            map.insert("__fastnbt_byte_array".to_string(), serde_json::Value::Array(ba.iter().map(|&b| serde_json::Value::Number(b.into())).collect()));
            serde_json::Value::Object(map)
        }
        fastnbt::Value::IntArray(ia) => {
            let mut map = serde_json::Map::new();
            map.insert("__fastnbt_int_array".to_string(), serde_json::Value::Array(ia.iter().map(|&i| serde_json::Value::Number(i.into())).collect()));
            serde_json::Value::Object(map)
        }
        fastnbt::Value::LongArray(la) => {
            let mut map = serde_json::Map::new();
            map.insert("__fastnbt_long_array".to_string(), serde_json::Value::Array(la.iter().map(|&l| serde_json::Value::Number(l.into())).collect()));
            serde_json::Value::Object(map)
        }
    }
}

fn json_to_nbt(json: serde_json::Value) -> fastnbt::Value {
    match json {
        serde_json::Value::Object(mut map) => {
            // Check for special tags
            if map.len() == 1 {
                if let Some(serde_json::Value::Array(arr)) = map.get("__fastnbt_byte_array") {
                    let vec: Vec<i8> = arr.iter().filter_map(|v| v.as_i64().map(|i| i as i8)).collect();
                    return fastnbt::Value::ByteArray(fastnbt::ByteArray::new(vec));
                }
                if let Some(serde_json::Value::Array(arr)) = map.get("__fastnbt_int_array") {
                    let vec: Vec<i32> = arr.iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect();
                    return fastnbt::Value::IntArray(fastnbt::IntArray::new(vec));
                }
                if let Some(serde_json::Value::Array(arr)) = map.get("__fastnbt_long_array") {
                    let vec: Vec<i64> = arr.iter().filter_map(|v| v.as_i64()).collect();
                    return fastnbt::Value::LongArray(fastnbt::LongArray::new(vec));
                }
            }
            
            let mut compound = std::collections::HashMap::new();
            for (k, v) in map {
                compound.insert(k, json_to_nbt(v));
            }
            fastnbt::Value::Compound(compound)
        }
        serde_json::Value::Array(arr) => {
            fastnbt::Value::List(arr.into_iter().map(json_to_nbt).collect())
        }
        serde_json::Value::String(s) => fastnbt::Value::String(s),
        serde_json::Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                fastnbt::Value::Long(i)
            } else if let Some(f) = num.as_f64() {
                fastnbt::Value::Double(f)
            } else {
                fastnbt::Value::Double(0.0)
            }
        }
        serde_json::Value::Bool(b) => fastnbt::Value::Byte(if b { 1 } else { 0 }),
        serde_json::Value::Null => fastnbt::Value::Byte(0),
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
