# Roadmap — Path to MVP v1

> **Version**: 2026.3.13
> **Status**: Phase 1 in progress
> **Tests**: 0 passing

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

## Phase 1 — Foundation

**Goal**: Establish workspace structure and zero-I/O core types.

- [x] Workspace scaffold (7 crates)
- [x] `rasa-core`: Document, Layer, Color, Geometry, Transform, Selection types
- [x] CI pipeline (build, test, lint, audit)
- [x] Project documentation (README, CONTRIBUTING, roadmap)
- [ ] Unit tests for core types (target: 90%+ coverage on rasa-core)
- [ ] Serde round-trip tests for all types
- [ ] Error type hierarchy

## Phase 2 — Canvas & Layers

**Goal**: Working layer system with compositing.

- [ ] Blend mode implementations (Normal, Multiply, Screen, Overlay, etc.)
- [ ] Layer compositing pipeline (CPU)
- [ ] Layer operations: reorder, duplicate, merge, flatten
- [ ] Layer groups with nested compositing
- [ ] Opacity and visibility
- [ ] Undo/redo command system

## Phase 3 — Rendering Pipeline

**Goal**: CPU-based renderer that produces correct output.

- [ ] Document renderer — flatten all layers to RGBA buffer
- [ ] Filter pipeline: brightness/contrast, hue/saturation, curves, levels
- [ ] Color management: sRGB, linear, Display P3
- [ ] Tile-based rendering for large documents
- [ ] Render cache (only re-render changed regions)

## Phase 4 — Storage & Formats

**Goal**: Open and save real image files.

- [ ] PNG import/export
- [ ] JPEG import/export (quality settings)
- [ ] WebP import/export
- [ ] TIFF import/export
- [ ] Native `.rasa` project format (layers + history + metadata)
- [ ] Recent files / project catalog (SQLite)

## Phase 5 — Basic Tools

**Goal**: Core editing tools for manual image editing.

- [ ] Brush engine: size, opacity, hardness, pressure sensitivity
- [ ] Eraser tool
- [ ] Selection tools: rectangle, ellipse, freeform lasso
- [ ] Selection operations: add, subtract, intersect, invert
- [ ] Transform tool: move, scale, rotate, skew
- [ ] Crop tool
- [ ] Eyedropper / color picker
- [ ] Fill and gradient tools

## Phase 6 — GPU Acceleration

**Goal**: Move compositing and filters to Vulkan compute.

- [ ] wgpu device initialization and capability detection
- [ ] GPU layer compositing (all blend modes)
- [ ] GPU filters: blur (Gaussian, box), sharpen, noise reduction
- [ ] GPU-accelerated brush rendering
- [ ] CPU fallback path for systems without Vulkan
- [ ] Performance benchmarks (GPU vs CPU path)

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
