# mc-anvil-db

FUSE-based virtual filesystem for Minecraft that emulate *.mca region files but instead of storing real .mca files on disk, chunks are read from Memory, Redis or PostgreSQL.


## Overview

**Key Features:**
- [] 🚀 Infinite World Feature without real .mca files on disk
- [] 📁 Anvil format compatibility (potentially works with any Minecraft Java Edition, but 1.21.11 considered as a goal for now)
- [] 🔌 Pluggable abstract storage backends (Memory, Redis, PostgreSQL) so Minecraft can read chunks from any storage backend without knowing the storage backend.
- [] 🐳 Docker-first with proper FUSE support
- [] Minecraft world is potentially queriable via PostgreSQL 
- [] Potentially we can use same map on different servers

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Minecraft Server                    │
│                    (Paper 1.21.11)                    │
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
│     Memory (L1) │ Redis (L2) │ PostgreSQL (L3)       │
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

## Storage Backends

| Backend | Status | Use Case |
|---------|--------|----------|
| `MemoryStorage` | 🚧 TODO | Development, testing |
| `RedisStorage` | 🚧 TODO | Distributed cache |
| `PostgresStorage` | 🚧 TODO | Persistent storage |

## How It Works

1. **Minecraft requests `r.0.0.mca`** → FUSE intercepts
2. **FUSE checks storage** → Is chunk in storage? (Memory, Redis, PostgreSQL)

3.1 **If not in storage** → Tell Minecraft to generate chunk
3.2. **If in storage** → Give minecraft .mca file

4. **Minecraft saves changes** → We capture them and store them in storage


## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level |
| `REDIS_URL` | `redis://redis:6379` | Redis connection (future) |

## Requirements

- Rust 1.92+
- FUSE 3 (`libfuse3-dev` on Debian/Ubuntu)
- Docker & Docker Compose (for containerized setup)

## License

MIT
