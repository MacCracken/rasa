# Roadmap — Path to MVP v1

> **Version**: 2026.3.15
> **Status**: MVP v1 complete — all 10 phases done
> **Tests**: 484 passing (89% coverage on testable crates)

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
| 9 — MCP & Agnoshi | 6 MCP tools, 5 agnoshi intents, `.agnos-agent` bundle | Done |
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
*(All items complete)*

### Platform
- **Plugin system** — third-party filters, tools, AI models
- **Tablet optimization** — touch UI mode, stylus gestures
- **Performance** — multi-GPU, tiled rendering, memory-mapped images

### Ecosystem Integration
- **Tazama integration** — send frames/stills between video and image editor
