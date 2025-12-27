# mc-anvil-db

A FUSE-based virtual filesystem for Minecraft that intercepts and simulates `.mca` region files. It provides a programmable storage layer for the Anvil format, enabling on-the-fly chunk generation, ~~remote database backends~~, and virtualized world management with minimal local disk footprint.

This screenshot shows a **Stateless Infinite Flat World Generator** - world don't really exists as file, it's just a fully procedural generation of chunks on the fly.
![Infinite Flat World Demo](demo/infinity_flat_demo.png) 

## Overview

Currently, this project acts as a **Stateless Infinite Flat World Generator**.


**Key Features:**
- [x] 🚀 **Infinite World**: Generates chunks procedurally as Minecraft requests them (Stateless).
- [ ] 📁 **Anvil Format**: Emulates standard Minecraft region files (Works with 1.21+).
- [ ] 🌐 **Pluggable Storage**: Abstract storage backends so Minecraft can read chunks from any storage backend without knowing the implementation (Memory, Redis, PostgreSQL).
- [x] 🐳 **Docker-first**: Runs in a container with FUSE permissions.
- [ ] 🔍 **Queryable World**: Minecraft world is potentially queriable via PostgreSQL.
- [ ] 🌐 **Multi-server**: Potentially we can use same map on different servers simultaneously.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Minecraft Server                    │
│                    (Paper 1.21.11)                    │
└─────────────────────┬───────────────────────────────┘
                      │ reads "r.x.z.mca"
                      ▼
┌─────────────────────────────────────────────────────┐
│                    FUSE Layer                        │
│              (src/fuse/mod.rs)                      │
│            Intercepts File I/O                      │
├─────────────────────────────────────────────────────┤
│                 World Generator                      │
│            (src/generator/flat.rs)                  │
│          Generates chunks on-the-fly                │
└─────────────────────────────────────────────────────┘
```

## Project Structure

```
src/
├── main.rs           # Entry point & FUSE Mount
├── fuse/
│   ├── mod.rs        # FUSE Filesystem Logic (Read/Write interception)
│   └── ...
├── generator/
│   ├── mod.rs        # WorldGenerator Trait
│   ├── flat.rs       # Flat World Implementation
│   └── builder.rs    # Helper to build NBT chunks
├── chunk.rs          # NBT Data Structures (Chunk, Section, BlockStates)
├── storage/
│   └── mod.rs        # ChunkStorage Trait (Interface definition)
├── region/           # MCA header/offset calculations

```

## Storage Backends

The project is **infrastructure-ready** for persistent storage. The Docker Compose environment includes a PostgreSQL (PostGIS) container, and the Rust codebase includes the `ChunkStorage` interface, though the implementation is currently a work in progress.

| Backend | Status | Use Case |
|---------|--------|----------|
| **Stateless Generator** | ✅ **Active** | Infinite flat world, testing |
| `MemoryStorage` | 🚧 Planned | Fast temporary storage |
| `RedisStorage` | 🚧 Planned | Caching |
| `PostgresStorage` | 🛠 **Infra Ready** | Container running, `ChunkStorage` trait defined |

## Quick Start

### With Docker (Recommended)

```bash
docker compose up --build
```

This starts:
- `mc-anvil-db`: The FUSE filesystem mounting to `/mnt/region`.
- `minecraft`: A Paper server configured to use the FUSE mount.

**Note:** Any blocks you place or destroy **will NOT be saved** in the current version. The server "writes" the data, but the FUSE layer simply acknowledges the write without persisting it (`/dev/null` style).



## How It Works (Current)

1. **Minecraft requests `r.0.0.mca`**: FUSE intercepts the `open` and `read` calls.
2. **Header Generation**: FUSE calculates where chunks *would* be in a real file.
3. **Chunk Generation**: When Minecraft reads a specific sector, `FlatGenerator` builds the NBT data (Bedrock, Dirt, Grass) in RAM.
4. **Compression**: The chunk is Zlib-compressed and sent to Minecraft.
5. **Writes**: If Minecraft saves the world, FUSE accepts the bytes but discards them (for now).

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level |

## Requirements

- Linux
- Docker & Docker Compose
- FUSE 3 installed on host (required for passing through `/dev/fuse`)

## License

MIT
