# HopperMC Performance Optimization Plan

This document outlines potential performance optimizations for HopperMC, prioritized by impact.

## Current Baseline (v0.0.6-pre3)
- **RAM Usage**: ~400MB
- **CPU Usage**: Low (underutilized)
- **Issue**: Slow chunk loading, transparent chunks
- **Target**: <1GB RAM, responsive chunk loading

---

## Optimization Priority Table

### 🔴 Critical (Must Fix)

| # | Optimization | CPU Impact | RAM Impact | Complexity | Status |
|---|-------------|------------|------------|------------|--------|
| 1 | **Remove per-chunk Tokio runtime creation** | **-40-60%** | -50MB | Low | ✅ DONE |

### 🟠 High Priority

| # | Optimization | CPU Impact | RAM Impact | Complexity | Status |
|---|-------------|------------|------------|------------|--------|
| 3 | LRU chunk cache (avoid regeneration) | **-30-50%** | +100-300MB | Medium | ⬜ TODO |
| 4 | Parallel chunk generation (thread pool) | +CPU, **-70% latency** | +50-100MB | Medium | ⬜ TODO |
| 5 | Pre-generation (ahead of player) | +CPU burst, **-90% lag** | +200-500MB | High | ⬜ TODO |

### 🟡 Medium Priority

| # | Optimization | CPU Impact | RAM Impact | Complexity | Status |
|---|-------------|------------|------------|------------|--------|
| 6 | Lazy heightmap calculation | -5-10% | 0 | Low | ⬜ TODO |
| 7 | ProtoChunk object pool (reuse allocations) | -5-10% | -50MB | Medium | ⬜ TODO |
| 8 | Inline noise routers (avoid Arc overhead) | -3-5% | -10MB | Low | ⬜ TODO |

### 🟢 Low Priority

| # | Optimization | CPU Impact | RAM Impact | Complexity | Status |
|---|-------------|------------|------------|------------|--------|
| 9 | SIMD for noise sampling | -10-20% | 0 | High | ⬜ TODO |
| 10 | Remove unnecessary clone() calls | -1-3% | -5MB | Low | ⬜ TODO |

---

## Detailed Implementation Notes

### 1. Remove Per-Chunk Tokio Runtime (CRITICAL)

**Problem**: Both `builder.rs` and `vanilla.rs` create a new Tokio runtime for every chunk:

```rust
// CURRENT (BAD) - builder.rs:105-108, vanilla.rs:86-89
let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()  // VERY EXPENSIVE per chunk!
    .unwrap();
rt.block_on(async { chunk_data.to_bytes().await })
```

**Solution Options**:
- **A**: Pass `tokio::runtime::Handle` through `WorldGenerator` trait
- **B**: Make `to_bytes()` synchronous (investigate Pumpkin API)
- **C**: Use `futures::executor::block_on()` (lighter than full runtime)

**Recommended**: Option A - modify trait to accept runtime handle.

### 2. Release Builds

**Check Dockerfile**:
```dockerfile
# Should be:
RUN cargo build --release
# NOT:
RUN cargo build
```

### 3. LRU Chunk Cache

```rust
use lru::LruCache;

struct VirtualFile {
    chunk_cache: Mutex<LruCache<(i32, i32), Vec<u8>>>,
    // ...
}

// In read_at:
if let Some(cached) = self.chunk_cache.lock().unwrap().get(&(abs_x, abs_z)) {
    return cached.clone();
}
```

**Memory Budget**:
- 500 chunks × ~200KB = ~100MB
- 1000 chunks × ~200KB = ~200MB

### 4. Parallel Generation

```rust
use rayon::prelude::*;

// Generate multiple chunks in parallel
let chunks: Vec<_> = chunk_positions
    .par_iter()
    .map(|(x, z)| self.generator.generate_chunk(*x, *z))
    .collect();
```

---

## RAM Budget Scenarios

| Configuration | Estimated RAM |
|--------------|---------------|
| Base (no cache) | ~400MB |
| + LRU 500 chunks | ~500MB |
| + LRU 1000 chunks | ~600MB |
| + Pre-gen radius 5 | ~800MB |
| **Limit** | **<1GB** ✓ |

---

## Implementation Order

1. **Phase 1 (Quick Wins)**: #1 Runtime fix, #2 Release builds
2. **Phase 2 (Caching)**: #3 LRU cache
3. **Phase 3 (Parallelism)**: #4 Thread pool
4. **Phase 4 (Advanced)**: #5-10 as needed

---

## Metrics to Track

- Chunk generation time (ms/chunk)
- Memory usage (RSS)
- CPU utilization during generation
- Transparent chunk rate (should be 0%)
