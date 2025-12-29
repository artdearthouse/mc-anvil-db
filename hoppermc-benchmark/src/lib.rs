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

    pub fn generate_report(&self) -> String {
        let uptime = self.start_time.unwrap_or_else(Instant::now).elapsed();
        let generated = self.total_chunks_generated.load(Ordering::Relaxed);
        let gen_time_total = self.total_generation_time_us.load(Ordering::Relaxed) as f64 / 1000.0; // ms
        let gen_max = self.max_generation_time_us.load(Ordering::Relaxed) as f64 / 1000.0; // ms
        let gen_avg = if generated > 0 { gen_time_total / generated as f64 } else { 0.0 };

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
             Max Time: {:.2} ms\n\n\
             [Storage Read]\n\
             Chunks Loaded: {}\n\
             Avg Time: {:.2} ms/chunk\n\n\
             [Storage Write]\n\
             Chunks Saved: {}\n\
             Avg Time: {:.2} ms/chunk\n",
            uptime,
            generated, gen_time_total, gen_avg, gen_max,
            loaded, load_avg,
            saved, save_avg
        )
    }
}
