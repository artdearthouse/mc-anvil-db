# mc-anvil-db

FUSE-based virtual filesystem for Minecraft that generates world chunks procedurally on-the-fly.

## Overview

This project creates a virtual filesystem that Minecraft servers can use as their `region/` folder. Instead of storing real `.mca` files on disk, chunks are generated procedurally when requested and served directly from memory.

**Key Features:**
- 🚀 Procedural chunk generation (infinite world potential)
- 📁 Anvil format compatibility (works with Paper/Spigot/Vanilla)
- 🔌 Pluggable storage backends (Memory, Redis, PostgreSQL)
- 🐳 Docker-ready with proper FUSE support

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Minecraft Server                    │
│                    (Paper 1.21+)                    │
└─────────────────────┬───────────────────────────────┘
                      │ reads/writes .mca files
                      ▼
┌─────────────────────────────────────────────────────┐
│                    FUSE Layer                        │
│              (src/fuse/mod.rs)                      │
├─────────────────────────────────────────────────────┤
│  Region Format    │  Chunk Provider                 │
│  (src/region/)    │  (src/chunk/)                   │
├───────────────────┴─────────────────────────────────┤
│                 Storage Backend                      │
│     Memory │ Redis (TODO) │ PostgreSQL (Done)       │
│              (src/storage/)                         │
└─────────────────────────────────────────────────────┘
```

## Project Structure

```
src/
├── main.rs           # Entry point
├── fuse/
│   ├── mod.rs        # FUSE filesystem implementation
│   └── inode.rs      # Inode ↔ Region mapping
├── region/
│   ├── mod.rs        # MCA format utilities
│   └── header.rs     # Region header generation
├── chunk/
│   ├── mod.rs        # Chunk provider (storage + generation)
│   └── generator.rs  # Procedural world generation
├── storage/
│   ├── mod.rs        # ChunkStorage trait
│   └── memory.rs     # In-memory storage (dev/testing)
└── nbt.rs            # NBT structures for Minecraft
```

## Quick Start

### With Docker (Recommended)

```bash
docker compose up --build
```

This starts:
- `mc-anvil-db` - FUSE filesystem
- `redis` - Cache/storage backend
- `minecraft` - Paper server on port 25565

### Local Development

```bash
# Build
cargo build --release

# Create mount point
mkdir -p /tmp/mc-region

# Run (requires FUSE permissions)
./target/release/mc-anvil-db
```

## Storage Backends

The `ChunkStorage` trait allows swappable backends:

```rust
pub trait ChunkStorage: Send + Sync {
    fn get(&self, pos: ChunkPos) -> Option<Vec<u8>>;
    fn set(&self, pos: ChunkPos, data: Vec<u8>);
    fn exists(&self, pos: ChunkPos) -> bool;
    fn delete(&self, pos: ChunkPos);
}
```

| Backend | Status | Use Case |
|---------|--------|----------|
| `MemoryStorage` | ✅ Done | Development, testing |
| `RedisStorage` | 🚧 TODO | Distributed cache |
| `PostgresStorage` | 🚧 TODO | Persistent storage |

## How It Works

1. **Minecraft requests `r.0.0.mca`** → FUSE intercepts
2. **FUSE checks storage** → Has this chunk been modified?
3. **If not in storage** → Generate procedurally
4. **Return MCA-formatted data** → Minecraft loads chunks
5. **Minecraft saves changes** → We capture and store them

Currently, basic read/write works with PostgreSQL, but complex chunks (with block data) may fail due to NBT serialization issues.
Persistence is enabled via `DATABASE_URL`.

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level |
| `REDIS_URL` | `redis://redis:6379` | Redis connection (future) |

## Requirements

- Rust 1.75+
- FUSE 3 (`libfuse3-dev` on Debian/Ubuntu)
- Docker & Docker Compose (for containerized setup)

## License

MIT
