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
- **rasa-core**: Selection combine operations (add, subtract, intersect via mask arithmetic)
- **rasa-engine**: Brush engine with round/square tips, hardness falloff, pressure sensitivity, spacing
- **rasa-engine**: Eraser tool (alpha reduction with brush dynamics)
- **rasa-engine**: Flood fill with tolerance, selection fill, linear gradient
- **rasa-engine**: Crop and affine transform with bilinear interpolation
- **rasa-engine**: Eyedropper / color picker (linear + sRGB)
- **rasa-gpu**: GPU compute pipeline — shader compilation, bind groups, dispatch, readback
- **rasa-gpu**: 9 WGSL compute shaders: composite (Normal/Multiply/Screen), invert, grayscale, brightness/contrast, blur H/V, brush dab
- **rasa-gpu**: GpuBackend wired to actual compute dispatch for compositing and per-pixel filters
- **rasa-gpu**: Performance benchmark framework with CPU baseline and GPU comparison
- **rasa-ai**: AI inference pipeline via hoosh/Synapse HTTP API (inpaint, upscale, segment, generate, remove-bg)
- **rasa-ai**: Model management with presets (SD Inpaint, RealESRGAN, SAM ViT-H, SDXL, U2Net)
- **rasa-ai**: Document integration: apply AI results as layers, within selections, with feathered blending
- **rasa-mcp**: MCP 2.0 server with stdio transport and JSON-RPC 2.0 protocol
- **rasa-mcp**: 5 MCP tools: rasa_open_image, rasa_edit_layer, rasa_apply_filter, rasa_get_document, rasa_export
- **rasa-mcp**: 5 agnoshi voice intents: rasa.open, rasa.filter, rasa.layer, rasa.export, rasa.ai
- **rasa-mcp**: `.agnos-agent.json` bundle for AGNOS platform integration
- **rasa-mcp**: Session state management for multi-document workflows
