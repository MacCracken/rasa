# Roadmap — Path to MVP v1

> **Version**: 2026.3.13
> **Status**: Phase 6 complete — Phase 7 next
> **Tests**: 312 passing

---

## MVP Phases

| Phase | Goal | Key Deliverables |
|-------|------|-----------------|
| 1 — Foundation | Workspace + core types | Document model, layers, color, geometry, transforms |
| 2 — Canvas & Layers | Layer system | Blend modes, compositing, opacity, layer groups |
| 3 — Rendering Pipeline | CPU renderer | Flatten layers, filter chain, color management |
| 4 — Storage & Formats | File I/O | PNG/JPEG/WebP read-write, native `.rasa` project format |
| 5 — Basic Tools | Editing tools | Brush, eraser, selection (rect/ellipse/freeform), crop, transform |
| 6 — GPU Acceleration | Vulkan compute | GPU compositing, GPU filters (blur, sharpen, etc.) |
| 7 — AI Foundation | Inference pipeline | Model loading (ONNX), hoosh/Synapse integration, pre/post-processing |
| 8 — AI Features | Core AI tools | Inpainting, upscaling (2x/4x), background removal, generative fill |
| 9 — MCP & Agnoshi | Platform integration | 5 MCP tools, 5 agnoshi intents, `.agnos-agent` bundle |
| 10 — UI Shell | Desktop application | Window, canvas viewport, tool palette, layer panel, properties |

---

## Phase 1 — Foundation ✓

**Goal**: Establish workspace structure and zero-I/O core types.

- [x] Workspace scaffold (7 crates)
- [x] `rasa-core`: Document, Layer, Color, Geometry, Transform, Selection types
- [x] CI pipeline (build, test, lint, audit)
- [x] Project documentation (README, CONTRIBUTING, roadmap)
- [x] Unit tests for core types (109 unit tests across all modules)
- [x] Serde round-trip tests for all types (32 integration tests)
- [x] Error type hierarchy (domain-specific variants for layers, selection, transform, storage, AI, history)

## Phase 2 — Canvas & Layers ✓

**Goal**: Working layer system with compositing.

- [x] Blend mode implementations (12 modes: Normal, Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, SoftLight, HardLight, Difference, Exclusion)
- [x] Layer compositing pipeline (CPU) with Porter-Duff alpha compositing
- [x] Layer operations: reorder, duplicate, merge down, flatten visible
- [x] Layer groups with nested compositing (recursive group rendering)
- [x] Opacity and visibility
- [x] Undo/redo command system (all operations reversible: merge, group, ungroup)

## Phase 3 — Rendering Pipeline ✓

**Goal**: CPU-based renderer that produces correct output.

- [x] Document renderer — flatten all layers to RGBA buffer (`renderer::render()`, `to_rgba_bytes()`)
- [x] Filter pipeline: brightness/contrast, hue/saturation, curves, levels + blur, sharpen, invert, grayscale
- [x] Color management: sRGB, linear, Display P3 (linear-to-sRGB conversion in render path)
- [x] Tile-based rendering for large documents (256x256 tiles via `tile_coords()`)
- [x] Render cache — dirty tile tracking with region invalidation (`RenderCache`)
- [x] Adjustment layer compositing — adjustment layers apply filters during compositing

## Phase 4 — Storage & Formats ✓

**Goal**: Open and save real image files.

- [x] PNG import/export
- [x] JPEG import/export (quality settings wired to encoder via `JpegEncoder::new_with_quality`)
- [x] WebP import/export
- [x] TIFF import/export (+ BMP, GIF)
- [x] Native `.rasa` project format (magic header, JSON metadata, binary pixel data with sRGB conversion)
- [x] Recent files / project catalog (SQLite via rusqlite — upsert, recent list, remove, clear)

## Phase 5 — Basic Tools ✓

**Goal**: Core editing tools for manual image editing.

- [x] Brush engine: size, opacity, hardness, pressure sensitivity, round/square tips, spacing
- [x] Eraser tool (alpha reduction with same brush dynamics)
- [x] Selection tools: rectangle, ellipse, freeform lasso (+ mask-based)
- [x] Selection operations: add, subtract, intersect, invert (mask-based combine)
- [x] Transform tool: move, scale, rotate, skew (affine transform with bilinear interpolation)
- [x] Crop tool (region extraction with bounds clamping)
- [x] Eyedropper / color picker (linear + sRGB u8 output)
- [x] Fill and gradient tools (flood fill with tolerance, selection fill, linear gradient)

## Phase 6 — GPU Acceleration ✓

**Goal**: Move compositing and filters to Vulkan compute.

- [x] wgpu device initialization and capability detection (Vulkan/Metal, high-perf adapter selection)
- [x] GPU layer compositing (Normal, Multiply, Screen via compute shaders; others CPU fallback)
- [x] GPU filters: invert, grayscale, brightness/contrast via compute; blur/sharpen CPU path
- [x] GPU brush dab compute shader (round tip with hardness falloff)
- [x] CPU fallback path for systems without Vulkan (graceful degradation)
- [x] Performance benchmarks (CPU baseline with MP/s metrics; GPU comparison when available)
- [x] 9 WGSL compute shaders: composite (Normal/Multiply/Screen), invert, grayscale, brightness/contrast, blur H/V, brush dab

## Phase 7 — AI Foundation

**Goal**: Inference pipeline ready for AI features.

- [ ] ONNX Runtime integration
- [ ] Model management: download, cache, version
- [ ] hoosh/Synapse API client for Stable Diffusion
- [ ] Pre-processing pipeline (resize, normalize, pad)
- [ ] Post-processing pipeline (denormalize, blend into document)
- [ ] Progress tracking and cancellation

## Phase 8 — AI Features

**Goal**: Ship the AI features that differentiate Rasa.

- [ ] **Inpainting**: mask region + context-aware regeneration
- [ ] **Upscaling**: 2x and 4x super-resolution
- [ ] **Background removal**: automatic subject segmentation
- [ ] **Generative fill**: text-prompt-driven content generation
- [ ] **AI selection**: intelligent edge detection
- [ ] Selection → AI pipeline integration (select region, apply AI)

## Phase 9 — MCP & Agnoshi

**Goal**: Platform integration for Claude and AGNOS voice control.

- [ ] MCP 2.0 server (stdio transport)
- [ ] 5 MCP tools: `rasa_open_image`, `rasa_edit_layer`, `rasa_apply_filter`, `rasa_get_document`, `rasa_export`
- [ ] 5 agnoshi intents for natural language voice commands
- [ ] `.agnos-agent` bundle
- [ ] Marketplace recipe (`recipes/marketplace/rasa.toml` in agnosticos)

## Phase 10 — UI Shell

**Goal**: Desktop-ready GUI application.

- [ ] Main window with menu bar
- [ ] Canvas viewport: pan, zoom, pixel grid, rulers
- [ ] Tool palette (all Phase 5 tools)
- [ ] Layer panel: visibility, opacity, blend mode, reorder
- [ ] Properties panel: tool settings, color, document info
- [ ] Color picker: wheel + sliders + hex input
- [ ] History panel (undo/redo list)
- [ ] Keyboard shortcuts
- [ ] Wayland-native, PipeWire integration

---

## Post-MVP

Items planned after MVP v1 ships:

### Creative Expansion
- **Style transfer** — apply artistic styles to images or selections
- **Text-to-image** — full Stable Diffusion pipeline with prompt editor
- **AI color grading** — automatic and prompt-driven color correction
- **Vector tools** — bezier paths, shapes, vector layers
- **Text engine** — text layers with font rendering, paragraph styles

### Professional Features
- **RAW format support** — Camera RAW processing pipeline
- **CMYK color mode** — print-ready output
- **ICC profile management** — full color management
- **PSD import/export** — Photoshop interop
- **Batch processing** — apply operations to multiple files
- **Non-destructive filters** — filter layers with adjustable parameters

### Platform
- **Plugin system** — third-party filters, tools, AI models
- **Collaborative editing** — real-time multi-user via Delta sync
- **Tablet optimization** — touch UI mode, stylus gestures
- **Performance** — multi-GPU, tiled rendering, memory-mapped images

### Ecosystem Integration
- **Tazama integration** — send frames/stills between video and image editor
- **Shruti integration** — album art workflow
- **Delta integration** — version-controlled design assets
- **BullShift integration** — chart/data visualization export
