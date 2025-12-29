# Configuration Guide

This document explains the environment variables and configuration options available in HopperMC. These can be set in your `.env` file or passed as environment variables to the Docker container.

## Storage Configuration

### `STORAGE`
Defines how chunk data is persisted or generated.
- `nostorage`: **Stateless Mode**. Chunks are generated on-the-fly and never saved. Ideal for testing or purely procedural worlds.
- `pg_raw`: **Binary Persistence**. (Default) Chunks are saved as NBT binary blobs in PostgreSQL. High performance and full data integrity.
- `pg_jsonb`: **Structured Persistence**. Chunks are converted to JSON and stored in a indexed `JSONB` column. Enables powerful SQL queries (e.g., searching for blocks/entities).

### `DATABASE_URL`
The PostgreSQL connection string. 
- Example: `postgres://user:password@db:5432/hoppermc`

### `COMPOSE_PROFILES`
Controls which services start in Docker.
- `storage`: Starts PostgreSQL alongside the filesystem. (Required for `pg_raw` and `pg_jsonb`).
- Leave empty for `nostorage`.

---

## Generator Configuration

### `GENERATOR`
Selects the world generation algorithm.
- `flat`: A fast, simple flat world (Grass/Dirt/Stone/Bedrock).
- `vanilla`: (Experimental) Realistic terrain generation using the Pumpkin-MC engine. Includes biomes, caves, and ores.

### `SEED`
The numerical seed for the world generator.
- Used by both `flat` and `vanilla` generators to ensure reproducibility.
- Example: `SEED=123456789`

---

## Performance Tuning

### `CACHE_SIZE`
Number of chunks to keep in the in-memory LRU cache.
- **Default**: `500`
- Higher values reduce DB/Generator load but increase RAM usage. Each chunk is roughly 4KB–100KB.

### `PREFETCH_RADIUS`
The radius (in chunks) around a player to pre-generate/load.
- **Default**: `0` (Disabled)
- **Recommended**: `1` or `2`
- When a player enters a chunk, HopperMC will trigger background generation for neighbors within this radius. This significantly reduces "transparent chunks" when flying.

---

## Technical Defaults

### `MC_DATA_VERSION`
The Minecraft DataVersion used for NBT structures.
- **Default**: `4671` (Minecraft 1.21.4)

### `RUST_LOG`
Controls logging verbosity.
- Options: `error`, `warn`, `info`, `debug`, `trace`
- Example: `RUST_LOG=hoppermc=debug,hoppermc_fs=info`
