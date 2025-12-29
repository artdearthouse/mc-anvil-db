use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct BenchmarkMetrics {
    // Generation Stats
    pub total_chunks_generated: AtomicUsize,
    pub total_generation_time_us: AtomicU64,
    pub max_generation_time_us: AtomicU64,
    
    // Storage Stats
    pub total_chunks_loaded: AtomicUsize,
    pub total_load_time_us: AtomicU64,
    pub total_chunks_saved: AtomicUsize,
    pub total_save_time_us: AtomicU64,

    // Detailed Breakdown
    pub total_generation_noise_us: AtomicU64,
    pub total_serialization_us: AtomicU64,
    pub total_compression_us: AtomicU64,
    
    // Cache
    pub total_cache_hits: AtomicUsize,
    pub total_cache_misses: AtomicUsize,

    // Session
    pub start_time: Option<Instant>,
}

impl BenchmarkMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Some(Instant::now()),
            ..Default::default()
        }
    }

    pub fn record_generation(&self, duration: Duration) {
        self.total_chunks_generated.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.total_generation_time_us.fetch_add(us, Ordering::Relaxed);
        self.max_generation_time_us.fetch_max(us, Ordering::Relaxed);
    }

    pub fn record_load(&self, duration: Duration) {
        self.total_chunks_loaded.fetch_add(1, Ordering::Relaxed);
        self.total_load_time_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_save(&self, duration: Duration) {
        self.total_chunks_saved.fetch_add(1, Ordering::Relaxed);
        self.total_save_time_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_generation_noise(&self, duration: Duration) {
        self.total_generation_noise_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_serialization(&self, duration: Duration) {
        self.total_serialization_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_compression(&self, duration: Duration) {
        self.total_compression_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.total_cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.total_cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn generate_report(&self) -> String {
        let uptime = self.start_time.unwrap_or_else(Instant::now).elapsed();
        let generated = self.total_chunks_generated.load(Ordering::Relaxed);
        let gen_time_total = self.total_generation_time_us.load(Ordering::Relaxed) as f64 / 1000.0; // ms
        let gen_max = self.max_generation_time_us.load(Ordering::Relaxed) as f64 / 1000.0; // ms
        let gen_avg = if generated > 0 { gen_time_total / generated as f64 } else { 0.0 };
        
        // Granular stats
        let noise_time = self.total_generation_noise_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let ser_time = self.total_serialization_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let comp_time = self.total_compression_us.load(Ordering::Relaxed) as f64 / 1000.0;
        
        let noise_avg = if generated > 0 { noise_time / generated as f64 } else { 0.0 };
        let ser_avg = if generated > 0 { ser_time / generated as f64 } else { 0.0 };
        let comp_avg = if generated > 0 { comp_time / generated as f64 } else { 0.0 };

        // Cache stats
        let hits = self.total_cache_hits.load(Ordering::Relaxed);
        let misses = self.total_cache_misses.load(Ordering::Relaxed);
        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 { (hits as f64 / total_requests as f64) * 100.0 } else { 0.0 };

        let loaded = self.total_chunks_loaded.load(Ordering::Relaxed);
        let load_time = self.total_load_time_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let load_avg = if loaded > 0 { load_time / loaded as f64 } else { 0.0 };
        
        let saved = self.total_chunks_saved.load(Ordering::Relaxed);
        let save_time = self.total_save_time_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let save_avg = if saved > 0 { save_time / saved as f64 } else { 0.0 };

        format!(
            "HopperMC Benchmark Report\n\
             =========================\n\
             Session Duration: {:.2?}\n\n\
             [Generation]\n\
             Chunks Generated: {}\n\
             Total Time: {:.2} ms\n\
             Avg Time: {:.2} ms/chunk\n\
             Max Time: {:.2} ms\n\
               - Noise/Logic: {:.2} ms/chunk\n\
               - Serialization: {:.2} ms/chunk\n\
               - Compression: {:.2} ms/chunk\n\n\
             [Storage Read]\n\
             Chunks Loaded: {}\n\
             Avg Time: {:.2} ms/chunk\n\n\
             [Storage Write]\n\
             Chunks Saved: {}\n\
             Avg Time: {:.2} ms/chunk\n\n\
             [Cache]\n\
             Hits: {}\n\
             Misses: {}\n\
             Hit Rate: {:.1}%\n",
            uptime,
            generated, gen_time_total, gen_avg, gen_max,
            noise_avg, ser_avg, comp_avg,
            loaded, load_avg,
            saved, save_avg,
            hits, misses, hit_rate
        )
    }
}
