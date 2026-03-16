# ADR-009: Parallel Compositing with Rayon

- **Status:** Accepted
- **Date:** 2026-03-16

## Context

The compositor and filter pipeline in rasa-engine process images pixel-by-pixel
in nested loops. For large images (e.g., 4000x3000 or higher), compositing and
filter operations become the dominant cost in the rendering pipeline. The
existing tile cache in the renderer (256x256 tiles) provides spatial locality
but tiles are still processed sequentially on a single thread.

Modern CPUs have many cores that sit idle during single-threaded compositing.
We need a parallelism strategy that is safe, deterministic, and provides
meaningful speedups without introducing complexity.

## Decision

We adopt **rayon** for data-parallel compositing and filter operations in
rasa-engine, using **row-level parallelism**.

### Why rayon

- Zero-unsafe API for data parallelism (`par_chunks_mut`, `par_iter_mut`).
- Work-stealing scheduler automatically balances load across cores.
- Widely adopted in the Rust ecosystem; well-tested and maintained.
- Graceful fallback to sequential execution on single-core machines.

### Why row-level parallelism (not tile-level)

- **Simpler implementation.** `par_chunks_mut` on the pixel slice, chunked by
  image width, gives one row per work unit with no index arithmetic changes.
- **Good cache locality.** Rows are contiguous in memory. Each thread reads a
  contiguous source row and writes a contiguous destination row.
- **No shared mutable state.** The `blend()` function is pure -- it takes two
  `Color` values by copy and returns a new `Color`. Each row's output pixels
  are disjoint, so no locks or atomics are needed.
- Tile-level parallelism remains a future option if we need finer-grained
  scheduling or GPU tile submission.

### What is parallelized

- `composite_layer` -- the core compositing loop that blends a source buffer
  onto a destination buffer.
- `gaussian_blur` -- both horizontal and vertical separable passes.
- `sharpen` -- the unsharp-mask difference step.
- `invert`, `grayscale` -- simple per-pixel transforms.
- Adjustment filters (`brightness/contrast`, `hue/saturation`, `curves`,
  `levels`) -- per-pixel operations.

### What is NOT parallelized

- Layer tree traversal (`composite_layer_tree`) -- inherently sequential due
  to layer ordering dependencies.
- rasa-core -- remains zero-dependency on rayon. It exposes `&mut [Color]`
  slices that rasa-engine parallelizes externally.

## Consequences

- **Performance:** Expected near-linear scaling with CPU core count for large
  images. Small images (< ~64x64) may see negligible benefit due to rayon's
  thread pool overhead, but rayon's adaptive scheduling avoids spawning work
  for trivially small inputs.
- **Correctness:** The blend function is pure and row outputs are disjoint, so
  parallelism does not change results. Tests verify that parallel output
  matches expected values.
- **Dependency:** Adds `rayon = "1"` to the workspace and to rasa-engine.
  rayon is a stable, widely-used crate with minimal transitive dependencies.
- **Future work:** GPU-accelerated compositing can coexist with CPU rayon
  parallelism -- the renderer can choose the GPU path when available and fall
  back to the parallel CPU path.
