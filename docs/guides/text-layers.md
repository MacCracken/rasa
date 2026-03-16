# Text Layers

Text layers allow you to add typographic content to a Rasa document. They are
rendered on-the-fly during compositing and remain resolution-independent in the
document model.

## How text layers work

A text layer stores its content as a Unicode string along with styling
properties. During compositing, the text engine rasterises the content into a
`PixelBuffer` at the document's resolution, which is then blended onto the
canvas like any other raster layer.

Because rendering happens on demand, changing a text layer's content or style
does not allocate pixel storage until the next composite pass.

## Supported properties

| Property      | Type        | Default   | Description                                    |
|---------------|-------------|-----------|------------------------------------------------|
| `content`     | `String`    | `""`      | The text to render. Supports `\n` for newlines.|
| `font_family` | `String`    | —         | Font family name (for future font selection).  |
| `font_size`   | `f32`       | —         | Size in pixels.                                |
| `color`       | `Color`     | black     | Text color in linear RGBA.                     |
| `alignment`   | `TextAlign` | `Left`    | Horizontal alignment: `Left`, `Center`, `Right`.|
| `line_height` | `f32`       | `1.2`     | Line spacing as a multiplier of `font_size`.   |

## Compositor integration

The compositor (`rasa-engine/src/compositor.rs`) has an explicit match arm for
`LayerKind::Text`. It calls `render_text_layer()` to produce a pixel buffer,
then composites it using the layer's blend mode and opacity -- exactly the same
as raster layers.

```
LayerKind::Text(text_layer) => {
    let text_buf = render_text_layer(text_layer, w, h);
    composite_layer(dst, &text_buf, layer.blend_mode, layer.opacity);
}
```

## Rendering with a custom font

The default `render_text_layer` function currently returns a transparent buffer
because no font is bundled with the engine. To render actual glyphs, use the
explicit font API:

```rust
use rasa_engine::text::render_text_layer_with_font;

let font_data = std::fs::read("path/to/font.ttf").unwrap();
let buf = render_text_layer_with_font(&text_layer, width, height, &font_data);
```

This accepts any TrueType or OpenType font as raw bytes.

## Current limitations

- **Single built-in font:** No font is bundled yet. `render_text_layer`
  produces transparent output until a default font is embedded.
- **No paragraph wrapping:** Text does not wrap at the layer boundary. Long
  lines extend beyond the buffer width.
- **No rich text:** All text in a layer shares the same style. Per-character
  or per-run styling is not supported.
- **No OpenType shaping:** Advanced features like ligatures, contextual
  alternates, and bidirectional text are not handled.
- **CPU only:** Text is rasterised on the CPU. GPU-accelerated rendering is
  planned for a future release.
