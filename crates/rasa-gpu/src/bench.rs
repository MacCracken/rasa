use std::time::{Duration, Instant};

use rasa_core::color::{BlendMode, Color};
use rasa_core::pixel::PixelBuffer;

use crate::backend::{CpuBackend, RenderBackend};

/// Result of a single benchmark run.
#[derive(Debug, Clone)]
pub struct BenchResult {
    pub operation: String,
    pub backend: String,
    pub duration: Duration,
    pub pixels: u64,
    pub megapixels_per_sec: f64,
}

impl std::fmt::Display for BenchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<25} {:<6} {:>8.2}ms  ({:.1} MP/s)",
            self.operation,
            self.backend,
            self.duration.as_secs_f64() * 1000.0,
            self.megapixels_per_sec,
        )
    }
}

fn measure(
    operation: &str,
    backend: &dyn RenderBackend,
    iterations: u32,
    mut f: impl FnMut(&dyn RenderBackend),
) -> BenchResult {
    // Warmup
    f(backend);

    let start = Instant::now();
    for _ in 0..iterations {
        f(backend);
    }
    let total = start.elapsed();
    let avg = total / iterations;

    BenchResult {
        operation: operation.into(),
        backend: backend.name().into(),
        duration: avg,
        pixels: 0,
        megapixels_per_sec: 0.0,
    }
}

/// Run benchmarks comparing CPU backend performance on standard operations.
/// Returns results for each operation.
pub fn run_benchmarks(size: u32, iterations: u32) -> Vec<BenchResult> {
    let backend = CpuBackend;
    let pixels = (size as u64) * (size as u64);
    let mut results = Vec::new();

    // Composite Normal
    let mut r = measure("composite_normal", &backend, iterations, |b| {
        let mut dst = PixelBuffer::filled(size, size, Color::WHITE);
        let src = PixelBuffer::filled(size, size, Color::new(1.0, 0.0, 0.0, 0.5));
        b.composite(&mut dst, &src, BlendMode::Normal, 1.0);
    });
    r.pixels = pixels;
    r.megapixels_per_sec = pixels as f64 / r.duration.as_secs_f64() / 1_000_000.0;
    results.push(r);

    // Composite Multiply
    let mut r = measure("composite_multiply", &backend, iterations, |b| {
        let mut dst = PixelBuffer::filled(size, size, Color::WHITE);
        let src = PixelBuffer::filled(size, size, Color::new(0.5, 0.5, 0.5, 1.0));
        b.composite(&mut dst, &src, BlendMode::Multiply, 1.0);
    });
    r.pixels = pixels;
    r.megapixels_per_sec = pixels as f64 / r.duration.as_secs_f64() / 1_000_000.0;
    results.push(r);

    // Invert
    let mut r = measure("invert", &backend, iterations, |b| {
        let mut buf = PixelBuffer::filled(size, size, Color::new(0.3, 0.6, 0.9, 1.0));
        b.invert(&mut buf);
    });
    r.pixels = pixels;
    r.megapixels_per_sec = pixels as f64 / r.duration.as_secs_f64() / 1_000_000.0;
    results.push(r);

    // Grayscale
    let mut r = measure("grayscale", &backend, iterations, |b| {
        let mut buf = PixelBuffer::filled(size, size, Color::new(1.0, 0.0, 0.0, 1.0));
        b.grayscale(&mut buf);
    });
    r.pixels = pixels;
    r.megapixels_per_sec = pixels as f64 / r.duration.as_secs_f64() / 1_000_000.0;
    results.push(r);

    // Brightness/Contrast
    let mut r = measure("brightness_contrast", &backend, iterations, |b| {
        let mut buf = PixelBuffer::filled(size, size, Color::new(0.5, 0.5, 0.5, 1.0));
        b.brightness_contrast(&mut buf, 0.1, 0.2);
    });
    r.pixels = pixels;
    r.megapixels_per_sec = pixels as f64 / r.duration.as_secs_f64() / 1_000_000.0;
    results.push(r);

    // Gaussian Blur
    let mut r = measure("gaussian_blur_r3", &backend, iterations, |b| {
        let mut buf = PixelBuffer::filled(size, size, Color::new(0.5, 0.5, 0.5, 1.0));
        b.gaussian_blur(&mut buf, 3);
    });
    r.pixels = pixels;
    r.megapixels_per_sec = pixels as f64 / r.duration.as_secs_f64() / 1_000_000.0;
    results.push(r);

    results
}

/// Run GPU benchmarks if a GPU is available, comparing with CPU.
pub fn run_gpu_benchmarks(size: u32, iterations: u32) -> Vec<BenchResult> {
    let mut all = run_benchmarks(size, iterations);

    match crate::backend::select_backend(false).is_gpu() {
        true => {
            let gpu_backend = crate::backend::select_backend(false);
            let pixels = (size as u64) * (size as u64);

            let ops: Vec<(&str, Box<dyn Fn(&dyn RenderBackend)>)> = vec![
                (
                    "composite_normal",
                    Box::new(move |b: &dyn RenderBackend| {
                        let mut dst = PixelBuffer::filled(size, size, Color::WHITE);
                        let src = PixelBuffer::filled(size, size, Color::new(1.0, 0.0, 0.0, 0.5));
                        b.composite(&mut dst, &src, BlendMode::Normal, 1.0);
                    }),
                ),
                (
                    "invert",
                    Box::new(move |b: &dyn RenderBackend| {
                        let mut buf =
                            PixelBuffer::filled(size, size, Color::new(0.3, 0.6, 0.9, 1.0));
                        b.invert(&mut buf);
                    }),
                ),
                (
                    "grayscale",
                    Box::new(move |b: &dyn RenderBackend| {
                        let mut buf =
                            PixelBuffer::filled(size, size, Color::new(1.0, 0.0, 0.0, 1.0));
                        b.grayscale(&mut buf);
                    }),
                ),
            ];

            for (name, op) in &ops {
                let mut r = measure(name, gpu_backend.as_ref(), iterations, |b| op(b));
                r.pixels = pixels;
                r.megapixels_per_sec = pixels as f64 / r.duration.as_secs_f64() / 1_000_000.0;
                all.push(r);
            }
        }
        false => {
            // No GPU available — skip GPU benchmarks
        }
    }

    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_benchmarks_run() {
        let results = run_benchmarks(64, 2);
        assert!(!results.is_empty());
        for r in &results {
            assert!(r.duration.as_nanos() > 0);
            assert!(r.megapixels_per_sec > 0.0);
            assert_eq!(r.backend, "CPU");
        }
    }

    #[test]
    fn bench_result_display() {
        let r = BenchResult {
            operation: "test_op".into(),
            backend: "CPU".into(),
            duration: Duration::from_millis(5),
            pixels: 1_000_000,
            megapixels_per_sec: 200.0,
        };
        let s = format!("{r}");
        assert!(s.contains("test_op"));
        assert!(s.contains("CPU"));
        assert!(s.contains("5.00ms"));
    }

    #[test]
    fn gpu_benchmarks_dont_panic() {
        // May or may not have GPU — just ensure no panic
        let results = run_gpu_benchmarks(32, 1);
        assert!(!results.is_empty());
    }
}
