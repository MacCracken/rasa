# Performance Guide

This guide covers how rasa handles large images efficiently and what to know
when working on performance-sensitive code.

## Parallel compositing

Compositing blends source layer pixels onto a destination buffer. rasa uses
**rayon** to parallelize this at the row level:

```
dst_pixels
    .par_chunks_mut(dst_width)   // one chunk = one row
    .enumerate()
    .for_each(|(y, dst_row)| {
        // blend each pixel in the row
    });
```

Each row is an independent unit of work. The `blend()` function is pure (takes
two `Color` values by copy, returns a new one), so rows can be processed on
different threads without synchronization.

Rayon's work-stealing scheduler distributes rows across available CPU cores
automatically. For a 4000x3000 image with 8 cores, each core processes roughly
375 rows per compositing call.

## Which operations are parallelized

| Operation            | Parallelism strategy        |
|----------------------|-----------------------------|
| Layer compositing    | Row-level `par_chunks_mut`  |
| Gaussian blur        | Row-level per pass          |
| Sharpen (unsharp)    | Per-pixel `par_iter_mut`    |
| Invert               | Per-pixel `par_iter_mut`    |
| Grayscale            | Per-pixel `par_iter_mut`    |
| Brightness/Contrast  | Per-pixel `par_iter_mut`    |
| Hue/Saturation       | Per-pixel `par_iter_mut`    |
| Curves               | Per-pixel `par_iter_mut`    |
| Levels               | Per-pixel `par_iter_mut`    |

Layer tree traversal remains sequential because layer ordering is inherently
dependent (each layer composites onto the result of all layers below it).

## Tile caching

The renderer maintains a tile cache with 256x256 pixel tiles. When a layer
changes, only the affected tiles are re-rendered. This reduces the number of
pixels that need compositing on incremental updates.

Tiles are currently composited sequentially (one tile at a time), but each
tile's internal compositing uses rayon parallelism. Future work may add
tile-level parallelism as well.

## GPU acceleration

Some filter operations have GPU compute shader implementations in rasa-gpu.
The renderer selects the GPU path when a compatible device is available and
falls back to the CPU path (with rayon parallelism) otherwise.

Compositing does not yet have a GPU path. This is planned future work -- the
blend function would be implemented as a compute shader operating on tile
textures.

## Tips for working with large images

1. **Prefer `par_chunks_mut` over `par_iter_mut` for 2D operations.** Chunking
   by row width preserves spatial locality and maps naturally to image rows.

2. **Keep the blend function pure.** It must remain free of shared mutable
   state so that parallel compositing is safe without locks.

3. **Avoid per-pixel allocations.** Allocate temporaries outside the parallel
   loop. For gaussian blur, we snapshot the source pixels into a `Vec<Color>`
   before the parallel pass.

4. **Test correctness explicitly.** Parallel execution can mask ordering bugs.
   The test suite verifies that parallel compositing and filter results match
   expected values pixel-by-pixel.

5. **Profile before optimizing.** Use `cargo bench` or a profiler to identify
   actual bottlenecks. Rayon adds negligible overhead for small images but the
   parallelism benefit scales with image size.
