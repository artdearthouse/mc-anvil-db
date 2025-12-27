use clap::Parser;
use std::path::PathBuf;

mod fuse;
use fuse::McFUSE;

#[derive(Parser)]
#[command(name = "mc-anvil-db", about = "FUSE-based virtual filesystem for Minecraft with Storage Backends")]
pub struct Args {
    #[arg(short, long, default_value = "/mnt/world")]
    pub mountpoint: PathBuf,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();


    let options = vec![];

    println!("Mounting FUSE to {:?} (Background)", args.mountpoint);
    
    let _session = fuser::spawn_mount2(McFUSE, &args.mountpoint, &options).unwrap();

    println!("Mounted successfully! Press Ctrl+C to unmount");
    
    tokio::signal::ctrl_c().await.expect("failed to install CTRL+C signal handler");
}