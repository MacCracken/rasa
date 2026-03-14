# Changelog

All notable changes to Rasa will be documented in this file.

## [2026.3.13] — 2026-03-13

### Added

**rasa-core** — Document model and core types
- Document, Layer, Color (`#[repr(C)]`), Geometry, Transform, Selection, PixelBuffer types
- 12 blend modes with Porter-Duff alpha compositing
- Undo/redo command history (VecDeque-based, O(1) eviction)
- Error type hierarchy: 13 domain-specific variants (layer, selection, transform, storage, AI, history)
- Selection combine operations (add, subtract, intersect via mask arithmetic)
- Merge down, layer grouping/ungrouping with full undo/redo
- Dimension validation (1..65536 clamping), PixelBuffer allocation cap (256 MP)
- Last-layer removal guard (`CannotRemoveLastLayer`)
- 154 unit tests + 32 serde round-trip integration tests

**rasa-engine** — Rendering and editing tools
- CPU compositing pipeline with recursive group rendering and adjustment layers
- Document renderer with sRGB/linear/Display P3 color space conversion
- 8 filters: brightness/contrast, hue/saturation, curves, levels, gaussian blur, sharpen, invert, grayscale
- Tile-based rendering (256x256) with dirty-region cache
- Brush engine: round/square tips, hardness, pressure sensitivity, spacing
- Eraser, flood fill, selection fill, linear gradient, crop, affine transform (bilinear interpolation)
- Eyedropper / color picker
- Optimized hot paths: slice-based pixel access (no per-pixel bounds checks)
- 71 tests

**rasa-gpu** — GPU acceleration
- wgpu device initialization (Vulkan/Metal) with graceful CPU fallback
- 9 WGSL compute shaders: composite (Normal/Multiply/Screen), invert, grayscale, brightness/contrast, blur H/V, brush dab
- Compute pipeline: shader compilation, bind groups, dispatch, GPU readback
- `RenderBackend` trait abstracting CPU/GPU with `select_backend()`
- Performance benchmark framework (CPU baseline + GPU comparison, MP/s metrics)
- 20 tests

**rasa-storage** — File I/O
- PNG, JPEG, WebP, TIFF, BMP, GIF import/export with sRGB/linear conversion
- JPEG quality wired to encoder (1-100)
- Native `.rasa` project format: RASA magic, JSON header (size-validated), binary pixel data
- Buffered I/O (BufReader/BufWriter) for large documents
- Recent files catalog (SQLite via rusqlite)
- Format detection, alpha support queries, export settings
- 41 tests

**rasa-ai** — AI inference
- hoosh/Synapse HTTP API client (inpaint, upscale, segment, generate, remove-bg)
- Model management: ModelId, ModelInfo, ModelKind, preset models
- Pre/post-processing pipeline (PixelBuffer to/from PNG)
- Document integration: apply AI results as layers, within selections, with feathered blending
- Progress tracking and cancellation
- 36 tests

**rasa-mcp** — MCP server and AGNOS integration
- MCP 2.0 server: stdio transport, JSON-RPC 2.0 (initialize, tools/list, tools/call)
- 5 tools: rasa_open_image, rasa_edit_layer, rasa_apply_filter, rasa_get_document, rasa_export
- 5 agnoshi voice intents: rasa.open, rasa.filter, rasa.layer, rasa.export, rasa.ai
- `.agnos-agent.json` bundle
- Session state with Mutex poison recovery
- Input validation: dimension caps, file existence checks, parent directory validation
- JSON-RPC spec compliance: `skip_serializing_if` for response fields, notification handling
- 47 tests

**rasa-ui** — Desktop GUI
- egui/eframe application (Wayland-compatible via winit)
- Menu bar (File/Edit/View/Layer/Filter), status bar
- Canvas viewport: pan, zoom, pixel grid, checkerboard transparency
- 9-tool palette: Brush, Eraser, Move, Selection, Eyedropper, Fill, Gradient, Crop, Transform
- Layer panel: visibility, opacity slider, blend mode, click-to-select
- Properties panel: tool-specific settings, color picker with hex display
- History panel with undo/redo
- Keyboard shortcuts: B/E/M/S/I/F/G/C/T, Ctrl+Z/Shift+Z, +/-
- 9 tests

**Infrastructure**
- 7-crate workspace with Cargo.toml workspace dependencies
- CI pipeline: build, test, lint (clippy), audit
- Makefile with 20 targets
- Date-based versioning (YYYY.M.DD)

### Test Summary

**410 tests passing** across 7 crates. 89% coverage on testable crates (core, engine, storage, mcp).
