use clap::Parser;
use std::path::PathBuf;

use hoppermc_fs::McFUSE;
use hoppermc_gen::flat::FlatGenerator;
use hoppermc_gen::vanilla::VanillaWorldGenerator;
use hoppermc_gen::WorldGenerator;
use hoppermc_fs::virtual_file::VirtualFile;

#[derive(Parser)]
#[command(name = "hoppermc", about = "FUSE-based virtual filesystem for Minecraft with Storage Backends")]
pub struct Args {
    #[arg(short, long, default_value = "/mnt/region")]
    pub mountpoint: PathBuf,
    
    /// World generator: "flat" or "vanilla"
    #[arg(short, long, env = "GENERATOR", default_value = "flat")]
    pub generator: String,
    
    /// World seed (for vanilla generator)
    #[arg(long, env = "SEED", default_value = "0")]
    pub seed: u64,
    
    /// Storage mode: "nostorage" (stateless) or "raw" (PostgreSQL)
    #[arg(long, env = "STORAGE", default_value = "raw")]
    pub storage: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    
    use hoppermc_storage::{postgres::PostgresStorage, StorageMode, ChunkStorage};
    use std::sync::Arc;
    
    // Initialize storage based on mode
    let storage: Option<Arc<dyn ChunkStorage>> = match args.storage.to_lowercase().as_str() {
        "nostorage" | "none" | "stateless" => {
            println!("Storage mode: NOSTORAGE (stateless, all chunks generated on-the-fly)");
            None
        },
        "raw" | "postgres" | _ => {
            let database_url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@db:5432/hoppermc".to_string());
            
            println!("Storage mode: RAW (PostgreSQL)");
            println!("Connecting to storage at {}...", database_url);
            
            // Retry loop for DB connection
            let mut storage_backend = None;
            for i in 0..30 {
                match PostgresStorage::new(&database_url, StorageMode::Raw).await {
                    Ok(s) => {
                        storage_backend = Some(s);
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to storage: {}. Retrying {}/30 in 2s...", e, i + 1);
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
            }

            let backend = storage_backend.expect("FATAL: Could not connect to storage after 30 retries.");
            Some(Arc::new(backend) as Arc<dyn ChunkStorage>)
        }
    };

    use fuser::MountOption;
    let options = vec![MountOption::AllowOther, MountOption::RW];

    // Select generator based on CLI args
    let generator: Arc<dyn WorldGenerator> = match args.generator.as_str() {
        "vanilla" => {
            println!("Using Pumpkin VanillaGenerator with seed: {}", args.seed);
            Arc::new(VanillaWorldGenerator::new(args.seed))
        },
        "flat" | _ => {
            println!("Using FlatGenerator");
            Arc::new(FlatGenerator)
        },
    };

    // Initialize Benchmark
    use hoppermc_fs::benchmark::BenchmarkMetrics;
    let benchmark = if std::env::var("BENCHMARK").is_ok() {
        println!("BENCHMARK MODE ENABLED 🚀");
        Some(Arc::new(BenchmarkMetrics::new()))
    } else {
        None
    };

    let handle = tokio::runtime::Handle::current();
    // Clone Arc for VirtualFile, keep original for report
    let virtual_file = VirtualFile::new(generator, storage, handle, benchmark.clone());
    let fs = McFUSE { virtual_file };

    println!("Mounting HopperMC FUSE to {:?} (Background)", args.mountpoint);
    
    let _session = fuser::spawn_mount2(fs, &args.mountpoint, &options).unwrap();

    println!("Mounted successfully! Press Ctrl+C to unmount");
    
    tokio::signal::ctrl_c().await.expect("failed to install CTRL+C signal handler");

    // Write Benchmark Report
    if let Some(bench) = benchmark {
        let report = bench.generate_report();
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        
        // Ensure benchmarks directory exists
        if let Err(e) = std::fs::create_dir_all("benchmarks") {
             eprintln!("Failed to create benchmarks directory: {}", e);
        }
        
        let filename = format!("benchmarks/benchmark-{}.txt", timestamp);
        if let Err(e) = std::fs::write(&filename, &report) {
            eprintln!("Failed to write benchmark report: {}", e);
        } else {
            println!("Benchmark report written to {}", filename);
            println!("{}", report);
        }
    }
}