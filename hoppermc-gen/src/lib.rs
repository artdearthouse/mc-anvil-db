use anyhow::Result;
use tokio::runtime::Handle;

use hoppermc_benchmark::BenchmarkMetrics;


pub trait WorldGenerator: Send + Sync {
    fn generate_chunk(&self, x: i32, z: i32, rt: &Handle, benchmark: Option<&BenchmarkMetrics>) -> Result<Vec<u8>>;
}

pub mod flat;
pub mod vanilla;
pub mod builder;