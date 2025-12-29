# Changelog

All notable changes to this project will be documented in this file.

## [0.0.6-pre3] - 2025-12-30 (Unreleased)

### Added
-   **Configurable Pre-generation**: New `--prefetch-radius` arg to proactively generate chunks around the player.
-   **Concurrent Prefetching**: Background pre-generation tasks now run concurrently (up to 2 parallel tasks) for faster world warming.

### Performance
-   **Internal Parallelism (Rayon)**: Integrated `rayon` into `hoppermc-gen` to parallelize data conversion loops (biomes/blocks), effectively utilizing all CPU cores.
-   **Docker Build Optimization**: 
    -   Added cache mounts for Git dependencies (`/usr/local/cargo/git`), avoiding redundant Pumpkin downloads.
    -   Fixed `RUSTFLAGS` mismatch between dependency caching and final build, ensuring consistent cache usage.
-   **LRU Chunk Cache**: Implemented in-memory LRU cache (`hoppermc-fs`) to store generated/loaded chunk blobs. Reduces redundant generation and I/O. Configurable via `--cache-size` (default: 500 chunks).
-   **Parallel FUSE I/O**: Refactored I/O handling to use a thread-per-request model, preventing file system operations from blocking the main loop. Significantly reduces "transparent chunk" issues during flight.
-   **Runtime Optimization**: Eliminated per-chunk `tokio::runtime` creation overhead.

### Storage & Metrics
-   **World Weight Tracking**: Benchmark reports now include:
    *   **Estimated MCA Size**: Calculated based on standard Minecraft Anvil format overhead.
    *   **Actual DB Size**: Real table size from PostgreSQL.
    *   **Efficiency Ratio**: Comparison of storage density between DB and MCA.
-   **Postgres Size Query**: Added `get_total_size` support to `ChunkStorage` trait and `PostgresStorage`.
-   **Granular Logic**: Logic time is now broken down into `Biomes`, `Noise` (Terrain), `Surface Rules`, and `Data Conversion` to pinpoint generator bottlenecks.
-   **FUSE Profiling**: Added direct measurement of filesystem `read_at` Latency, Throughput (MB/s), and Compression Ratio.
-   **Storage Mode Selection**: choose between `raw` (Postgres) and `nostorage` (Stateless).

---

## [0.0.6-pre2] - 2025-12-29

### Added
-   **LZ4 Decompression Support**: Added support for LZ4 compressed chunks (Minecraft 24w04a+) via `lz4-java-wrc`.
-   **Compression Constants Module**: New `hoppermc_anvil::compression` module with GZIP, ZLIB, NONE, LZ4 constants.
-   **Vanilla World Generator (Experimental)**: Full integration with Pumpkin's `VanillaGenerator` for realistic Minecraft terrain with biomes, caves, and surface rules. Uses staged generation (biomes → noise → surface) with `ProtoChunk` → `ChunkData` conversion.
    -   Select via `--generator vanilla` CLI flag or `GENERATOR=vanilla` env var.
    -   Seed configurable via `--seed N` or `SEED=N`.

### Changed
-   **Dependency Cleanup**: Removed unused workspace dependencies (`thiserror`, `postgis`, `hex`, `pumpkin-nbt`, `pumpkin-util`).
-   **Docker Compose**: Now passes `GENERATOR` and `SEED` environment variables from `.env` to the hoppermc container.

### Known Issues
-   ⚠️ **Vanilla Generator Performance**: The vanilla generator is **very slow** — chunk loading may appear frozen for 30+ seconds on initial spawn. This is expected due to complex noise sampling. Optimization planned for future releases.

## [0.0.5] - 2025-12-29

### Changed
-   **Project Rename**: Renamed project from `mc-anvil-db` to **HopperMC** (`hoppermc`).
-   **Workspace Restructuring**: Refactored the monolithic structure into a Cargo Workspace with modular crates:
    -   `hoppermc`: CLI and FUSE mount.
    -   `hoppermc-fs`: FUSE filesystem implementation.
    -   `hoppermc-gen`: World generation logic.
    -   `hoppermc-anvil`: Anvil format utilities.
    -   `hoppermc-storage`: Storage interfaces.

## [0.0.4] - 2025-12-28

### Added
-   **Infinite World Interception**: The FUSE layer now intercepts read/write requests for the *entire* world (all region coordinates `r.x.z.mca`), not just `r.0.0.mca`. This allows for infinite exploration.
-   **Pumpkin Backend**: Completely replaced the custom legacy NBT generation logic with the [Pumpkin-MC](https://github.com/Pumpkin-MC/Pumpkin) library. This ensures correct chunk structure, block ID resolution, and standardized NBT serialization.
-   **Docker Build Caching**: Initialized Docker BuildKit `cache mounts` for Cargo registry/git and target directories. This drastically reduces incremental build times (from minutes to seconds).

### Fixed
-   **Paper Console Errors**: Eliminated I/O errors and warnings in the Paper server console. The FUSE layer now correctly mocks file simulates file operations (writes, locks) to satisfy the server's requirements without requiring actual disk storage.
