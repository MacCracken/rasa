# ADR-008: Text Rendering Engine

**Status:** Accepted
**Date:** 2026-03-16

## Context

Rasa's layer model includes a `TextLayer` variant, but until now text layers
were silently ignored during compositing (caught by the `_ =>` catch-all in the
compositor). Users need the ability to add text overlays to their images with
control over font size, color, alignment, and line height.

We need a CPU-side text rasterisation solution that:

1. Fits into the existing raster pipeline (everything composites via `PixelBuffer`).
2. Does not introduce heavy system dependencies (e.g. FreeType, HarfBuzz).
3. Can be extended later with custom fonts and more advanced layout.

## Decision

### Use `ab_glyph` for font rasterisation

We chose `ab_glyph` because:

- It is a pure-Rust TrueType/OpenType rasteriser with no C dependencies.
- It is already a transitive dependency via `egui`, so it adds zero new
  dependency weight.
- It provides glyph-level control (scaling, positioning, outlining, coverage
  rasterisation) which is exactly what we need.

### Render text to PixelBuffer on demand

Text layers are rendered to a `PixelBuffer` during compositing, not stored as
pre-rasterised pixels. This means:

- Editing text content, size, or color does not require re-rasterising until
  the next composite pass.
- Text layers remain resolution-independent in the document model.
- The compositor calls `render_text_layer()` which returns a `PixelBuffer` that
  is then blended like any raster layer.

### Font embedding approach

Rather than bundling a font binary in the repository, the engine provides two
entry points:

- `render_text_layer(text, w, h)` -- uses a built-in default font (currently
  returns a transparent buffer as no font is bundled yet).
- `render_text_layer_with_font(text, w, h, font_data)` -- accepts arbitrary
  TrueType/OpenType font bytes for actual rendering.

This keeps the repository small while allowing full rendering when font data is
supplied at runtime.

### Extended TextLayer fields

The `TextLayer` struct was extended with `color` (linear RGBA), `alignment`
(Left/Center/Right), and `line_height` (multiplier) to give users meaningful
typographic control.

## Consequences

### Positive

- Text layers now composite correctly instead of being silently skipped.
- No new native dependencies -- `ab_glyph` is pure Rust and already in the
  dependency tree.
- The on-demand rendering model keeps the document model clean and
  resolution-independent.
- The `render_text_layer_with_font` API allows full rendering with any font.

### Negative

- Without a bundled font, `render_text_layer` produces transparent output. A
  default font should be embedded in a follow-up.
- No paragraph wrapping, rich text, or bidirectional text support yet.
- No font fallback chain -- only a single font is used per text layer.
- Kerning relies on the font's built-in kern table; no advanced OpenType
  shaping (ligatures, contextual alternates).

### Future work

- Bundle a small open-source font (e.g. Inter, Noto Sans) as the default.
- Add paragraph wrapping with configurable max width.
- Integrate GPU-accelerated text rendering via `rasa-gpu`.
- Support font selection from system-installed fonts.
- Add rich text support (per-run styling within a single text layer).
