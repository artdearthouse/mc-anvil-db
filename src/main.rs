//! mc-anvil-db: FUSE filesystem for procedural Minecraft world generation.
//!
//! This program mounts a virtual filesystem that serves Minecraft region files
//! (.mca) with procedurally generated chunk data.

mod chunk;
mod fuse;
mod nbt;
mod region;
mod storage;

use std::sync::Arc;
use fuser::MountOption;

use crate::fuse::AnvilFS;
use crate::storage::MemoryStorage;

fn main() {
    env_logger::init();

    let mountpoint = "/mnt/region";

    let options = vec![
        MountOption::FSName("mc-anvil-db".to_string()),
        MountOption::AutoUnmount,
        MountOption::AllowOther,
    ];

    // Create storage backend based on environment
    let storage: Arc<dyn crate::storage::ChunkStorage> = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            log::info!("Using PostgreSQL storage: {}", url);
            Arc::new(crate::storage::PostgresStorage::new(&url))
        },
        Err(_) => {
            log::warn!("DATABASE_URL not found. Using in-memory storage (data will be lost on exit!)");
            Arc::new(MemoryStorage::new())
        }
    };

    // Create filesystem
    let fs = AnvilFS::new(storage);

    println!("Starting mc-anvil-db FUSE mount at {}...", mountpoint);

    fuser::mount2(fs, mountpoint, &options).expect("Failed to mount FUSE filesystem");
}
