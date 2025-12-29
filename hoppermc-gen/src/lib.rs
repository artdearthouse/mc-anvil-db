use anyhow::Result;
use tokio::runtime::Handle;

pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, x: i32, z: i32, rt: &Handle) -> Result<Vec<u8>>;
}

pub mod flat;
pub mod vanilla;
pub mod builder;