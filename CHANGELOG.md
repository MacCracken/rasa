# Changelog

All notable changes to Rasa will be documented in this file.

## [2026.3.13] — 2026-03-13

### Added
- Initial project scaffolding and workspace setup
- Core crate structure: rasa-core, rasa-gpu, rasa-engine, rasa-storage, rasa-ai, rasa-ui, rasa-mcp
- Project documentation: README, CONTRIBUTING, roadmap
- CI/CD pipeline configuration
- Makefile with standard build targets
- **rasa-core**: Document, Layer, Color, Geometry, Transform, Selection, PixelBuffer types
- **rasa-core**: Blend mode implementations (12 modes with Porter-Duff alpha compositing)
- **rasa-core**: Undo/redo command history system
- **rasa-core**: Error type hierarchy with domain-specific variants (layer, selection, transform, storage, AI, history errors)
- **rasa-core**: 109 unit tests across all modules (geometry, layer, color, transform, selection, pixel, blend, command, document, error)
- **rasa-core**: 32 serde round-trip integration tests for all serializable types
- **rasa-core**: sRGB/linear/HSL color space conversions
- **rasa-core**: 2D affine transform with composition and inverse
- **rasa-core**: Merge down operation — composites upper layer pixels onto lower layer
- **rasa-core**: Layer grouping and ungrouping with undo/redo support
- **rasa-engine**: Recursive group compositing — groups rendered to intermediate buffer then blended
- **rasa-engine**: CPU compositing pipeline (flatten all visible layers with blend modes and opacity)
- **rasa-engine**: Document renderer with sRGB/linear/Display P3 color space conversion
- **rasa-engine**: Filter pipeline: brightness/contrast, hue/saturation, curves, levels, gaussian blur, sharpen, invert, grayscale
- **rasa-engine**: Adjustment layer compositing — adjustment layers apply filters inline during compositing
- **rasa-engine**: Tile-based rendering (256x256 tiles) with dirty-region render cache
- **rasa-engine**: Region rendering for partial/incremental updates
- **rasa-engine**: RGBA u8 byte output for display/export
- **rasa-storage**: PNG, JPEG, WebP, TIFF, BMP, GIF import/export with sRGB/linear color conversion
- **rasa-storage**: JPEG quality settings wired to encoder (1-100 via `JpegEncoder::new_with_quality`)
- **rasa-storage**: Native `.rasa` project format (RASA magic, JSON header, binary pixel data)
- **rasa-storage**: Recent files catalog backed by SQLite (rusqlite) with upsert, ordering, limits
- **rasa-storage**: Format detection by file extension, alpha support queries, export settings
