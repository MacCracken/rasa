# Roadmap

> **Version**: 2026.3.18
> **Status**: MVP v1 complete — all 10 phases done, post-MVP in progress
> **Tests**: 600 passing (89% coverage on testable crates)

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
| 9 — MCP & Agnoshi | 8 MCP tools, 5 agnoshi intents, `.agnos-agent` bundle | Done |
| 10 — UI Shell | egui desktop app, canvas, panels, shortcuts | Done |

---

## Post-MVP v1 (Complete)

### Platform
- **Plugin system** — `Filter`, `Tool`, `Plugin` traits + registries, `PluginManager`, provider-aware AI pipeline | Done

---

## Post-MVP v2 (Remaining)

### Platform
- **Tablet optimization** — touch UI mode, stylus gestures
- **Dynamic plugin loading** — WASM or native dylib discovery at runtime

### Ecosystem Integration
*(All items complete)*
