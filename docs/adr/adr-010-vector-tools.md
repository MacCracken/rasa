# ADR-010: Vector Tools

- **Status:** Accepted
- **Date:** 2026-03-16

## Context

Rasa's layer model included a `LayerKind::Vector` stub with no backing data
structure or rendering path. Users need the ability to create resolution-
independent shapes (rectangles, ellipses, lines, arbitrary bezier paths) that
are stored as geometry rather than pixels.

We need:
1. A data model for vector paths with fill and stroke styling.
2. A rendering pipeline that rasterises vector data on demand during
   compositing.
3. A 2D geometry library for bezier math (containment tests, distance
   queries, path evaluation).

## Decision

### kurbo for 2D path math

We adopt **kurbo** (`0.13`) from the Linebender project as the 2D geometry and
bezier library. kurbo is pure Rust, has no unsafe code, and provides the
primitives we need: `BezPath`, `Shape::contains()`, `ParamCurveNearest`, and
segment evaluation.

Alternatives considered:
- **lyon**: Full tessellation library. More than we need for the MVP and brings
  a larger dependency tree.
- **bezier-rs**: Smaller but less mature and less widely adopted.
- **Hand-rolled**: Bezier math is well-known but error-prone to implement
  correctly, especially for cubic curves and winding-number containment tests.

### Vector layers store paths as data

`LayerKind::Vector` now wraps a `VectorData` struct containing a list of
`VectorPath` values. Each path has:
- A sequence of `PathSegment` values (MoveTo, LineTo, QuadTo, CubicTo).
- A `closed` flag.
- Optional `FillStyle` (currently only `Solid` color).
- Optional `StrokeStyle` (color, width, cap, join).

This data is purely declarative and lives in `rasa-core`, which remains
zero-I/O. The data serialises cleanly with serde for document persistence.

### On-demand rasterisation during compositing

Vector layers are not pre-rasterised. During compositing, the compositor calls
`render_vector_layer()` to produce a `PixelBuffer` from the `VectorData` at
the document's resolution. This buffer is then blended onto the canvas like
any other raster layer.

Benefits:
- Resolution independence: re-rendering at any zoom or export resolution is
  trivial.
- No pixel storage for vector layers until composite time.
- Matches the pattern already established for text layers.

### Rendering approach

The MVP renderer uses a per-pixel approach:
- **Fill**: For each pixel center, test containment using kurbo's
  `Shape::contains()` (winding-number rule).
- **Stroke**: For each pixel center, compute the minimum distance to the path
  using `ParamCurveNearest`. If within half the stroke width, the pixel is
  painted.

This is O(pixels x segments), which is acceptable for the MVP. Future
optimisations (scanline rasterisation, signed-distance-field caching, GPU
tessellation) can replace the inner loop without changing the public API.

## Consequences

- **New dependency**: `kurbo = "0.13"` added to the workspace and to both
  `rasa-core` and `rasa-engine`. kurbo is a lightweight, well-maintained crate.
- **Breaking change to LayerKind**: `Vector` changes from a unit variant to
  `Vector(VectorData)`. All pattern matches and serde roundtrip tests are
  updated.
- **Performance**: The per-pixel renderer is adequate for small-to-medium
  paths. Large documents with many complex vector paths will benefit from the
  planned scanline or GPU-accelerated rasteriser.
- **Future work**:
  - Gradient fills and pattern fills.
  - Text-on-path.
  - Anti-aliasing via multi-sample or analytic coverage.
  - Interactive vector editing tools (pen tool, node manipulation).
  - GPU-accelerated tessellation and rendering.
