# Vector Layers

Vector layers store resolution-independent geometry -- paths with fill and
stroke styling -- rather than pixel data. They are rasterised on demand during
compositing at the document's resolution.

## How vector layers work

A vector layer contains a `VectorData` payload: a list of `VectorPath` values.
Each path is a sequence of segments (lines, quadratic and cubic bezier curves)
with optional fill and stroke styling.

During compositing, the engine converts the vector data to a `PixelBuffer`
using `render_vector_layer()`, then blends it onto the canvas like any other
raster layer. Because rendering happens on demand, changing a path's geometry
or style does not allocate pixel storage until the next composite pass.

## Supported primitives

| Primitive  | Constructor                              | Description                                        |
|------------|------------------------------------------|----------------------------------------------------|
| Rectangle  | `VectorPath::rect(x, y, w, h, ...)`     | Axis-aligned rectangle from four line segments.    |
| Ellipse    | `VectorPath::ellipse(cx, cy, rx, ry, ...)`| Ellipse approximated with four cubic bezier arcs. |
| Line       | `VectorPath::line(x1, y1, x2, y2, ...)`  | A single straight stroke.                         |
| Custom     | Build `PathSegment` vec manually          | Any combination of MoveTo, LineTo, QuadTo, CubicTo.|

### Path segments

- `MoveTo(Point)` -- move the pen without drawing.
- `LineTo(Point)` -- straight line to the given point.
- `QuadTo { ctrl, end }` -- quadratic bezier curve.
- `CubicTo { ctrl1, ctrl2, end }` -- cubic bezier curve.

## Fill and stroke styling

### Fill

Currently supports solid color fills via `FillStyle::Solid(Color)`. Fills are
only rendered for closed paths.

### Stroke

`StrokeStyle` controls the path outline:

| Property | Type       | Default | Description                     |
|----------|------------|---------|---------------------------------|
| `color`  | `Color`    | --      | Stroke color in linear RGBA.    |
| `width`  | `f64`      | --      | Stroke width in pixels.         |
| `cap`    | `LineCap`  | `Butt`  | Endpoint style: Butt, Round, Square. |
| `join`   | `LineJoin` | `Miter` | Corner style: Miter, Round, Bevel.   |

## Compositor integration

The compositor (`rasa-engine/src/compositor.rs`) has an explicit match arm for
`LayerKind::Vector`. It calls `render_vector_layer()` to produce a pixel
buffer, then composites it using the layer's blend mode and opacity.

```
LayerKind::Vector(vector_data) => {
    let vec_buf = render_vector_layer(vector_data, w, h);
    composite_layer(dst, &vec_buf, layer.blend_mode, layer.opacity);
}
```

## How vectors are rasterised

The current renderer uses a per-pixel approach powered by the `kurbo` crate:

1. Each `VectorPath` is converted to a `kurbo::BezPath`.
2. For **fills**, each pixel center is tested for containment using kurbo's
   winding-number rule (`Shape::contains`).
3. For **strokes**, each pixel center's distance to the nearest path segment
   is computed. If within half the stroke width, the pixel is painted.

This produces correct results for all path types. Performance is proportional
to the number of pixels multiplied by the number of path segments.

## Creating a vector layer

```rust
use rasa_core::vector::{VectorData, VectorPath, FillStyle, StrokeStyle, LineCap, LineJoin};
use rasa_core::color::Color;
use rasa_core::layer::Layer;

let mut data = VectorData::new();

// Add a filled red rectangle.
data.add_path(VectorPath::rect(
    10.0, 10.0, 200.0, 100.0,
    Some(FillStyle::Solid(Color::new(1.0, 0.0, 0.0, 1.0))),
    None,
));

// Add a stroked ellipse.
data.add_path(VectorPath::ellipse(
    150.0, 150.0, 80.0, 50.0,
    None,
    Some(StrokeStyle {
        color: Color::BLACK,
        width: 3.0,
        cap: LineCap::Round,
        join: LineJoin::Round,
    }),
));

let layer = Layer::new_vector("Shapes", data, (800, 600));
```

## Current limitations

- **No gradient fills.** Only solid color fills are supported. Gradient and
  pattern fills are planned.
- **No text-on-path.** Text and vector paths are independent layer types.
- **Basic rasterisation.** The per-pixel renderer does not produce anti-aliased
  edges. A scanline or analytic-coverage rasteriser is planned.
- **No interactive editing.** There is no pen tool or node manipulation UI yet.
  Vector paths are created programmatically.
- **CPU only.** Rasterisation runs on the CPU. GPU-accelerated tessellation is
  planned for a future release.
