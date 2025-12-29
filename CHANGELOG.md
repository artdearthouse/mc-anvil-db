# Changelog

All notable changes to this project will be documented in this file.

## [0.0.6-pre3] - 2025-12-29 (Unreleased)

### Added
-   **Storage Mode Selection**: New `STORAGE` env var / `--storage` CLI arg to choose storage backend.
    -   `raw` (default) — Persist chunks to PostgreSQL.
    -   `nostorage` — Fully stateless mode, all chunks generated on-the-fly. No DB required.

### Performance
-   **LRU Chunk Cache**: Implemented in-memory LRU cache (`hoppermc-fs`) to store generated/loaded chunk blobs. Reduces redundant generation and I/O. Configurable via `--cache-size` (default: 500 chunks).
-   **Parallel FUSE I/O**: Refactored I/O handling to use a thread-per-request model, preventing file system operations from blocking the main loop. Significantly reduces "transparent chunk" issues during flight.
-   **Runtime Optimization**: Eliminated per-chunk `tokio::runtime` creation overhead.

### Internal
-   **Benchmark System**:
    -   **Usage**: `BENCHMARK=true` prints report on exit.
    -   **Refactor**: Extracted core logic to `hoppermc-benchmark` crate for cross-crate usage.
    -   **Metrics**: Tracks generation time (avg/max), storage I/O, and chunk throughput.
    -   **Reporting**: Automatically saves session reports to `benchmarks/benchmark-{timestamp}.txt`.

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
