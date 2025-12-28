# Changelog

All notable changes to this project will be documented in this file.

## [0.0.4] - 2025-12-28

### Added
-   **Infinite World Interception**: The FUSE layer now intercepts read/write requests for the *entire* world (all region coordinates `r.x.z.mca`), not just `r.0.0.mca`. This allows for infinite exploration.
-   **Pumpkin Backend**: Completely replaced the custom legacy NBT generation logic with the [Pumpkin-MC](https://github.com/Pumpkin-MC/Pumpkin) library. This ensures correct chunk structure, block ID resolution, and standardized NBT serialization.
-   **Docker Build Caching**: Initialized Docker BuildKit `cache mounts` for Cargo registry/git and target directories. This drastically reduces incremental build times (from minutes to seconds).

### Fixed
-   **Paper Console Errors**: Eliminated I/O errors and warnings in the Paper server console. The FUSE layer now correctly mocks file simulates file operations (writes, locks) to satisfy the server's requirements without requiring actual disk storage.
