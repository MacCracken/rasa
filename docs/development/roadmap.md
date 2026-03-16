# Roadmap — Path to MVP v1

> **Version**: 2026.3.15
> **Status**: MVP v1 complete — all 10 phases done
> **Tests**: 447 passing (89% coverage on testable crates)

---

## MVP Phases (Complete)

| Phase | Goal | Status |
|-------|------|--------|
| 1 — Foundation | Workspace + core types, error hierarchy, serde tests | Done |
| 2 — Canvas & Layers | 12 blend modes, compositing, groups, merge, undo/redo | Done |
| 3 — Rendering Pipeline | CPU renderer, 8 filters, tile cache, adjustment layers | Done |
| 4 — Storage & Formats | PNG/JPEG/WebP/TIFF/BMP/GIF/PSD/RAW, `.rasa` format, SQLite catalog | Done |
| 5 — Basic Tools | Brush, eraser, selection, transform, crop, fill, gradient | Done |
| 6 — GPU Acceleration | wgpu init, 9 WGSL shaders, compute pipeline, benchmarks | Done |
| 7 — AI Foundation | Synapse API client, model management, pre/post-processing | Done |
| 8 — AI Features | Inpainting, upscaling, background removal, generative fill | Done |
| 9 — MCP & Agnoshi | 5 MCP tools, 5 agnoshi intents, `.agnos-agent` bundle | Done |
| 10 — UI Shell | egui desktop app, canvas, panels, shortcuts | Done |

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
- **CMYK color mode** — print-ready output
- **ICC profile management** — full color management
- **Batch processing** — apply operations to multiple files

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
