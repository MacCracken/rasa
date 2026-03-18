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

## P0 — Code Health (Next)

### Code Refactor
- **Dead code cleanup** — remove `#[allow(dead_code)]` items (PSD export stubs), unused imports
- **Consistent error handling** — audit remaining `.expect()`/`.unwrap()` in non-test code, convert to `?` or proper errors
- **Module structure** — evaluate whether `ActiveTool` enum can be retired in favor of `ToolRegistry`-only
- **API surface audit** — review pub visibility, seal internal types, reduce leaky abstractions

### Review & Audit
- **Dependency audit** — `cargo audit`, check for unused deps, pin versions
- **Unsafe audit** — verify zero `unsafe` blocks, audit FFI boundaries (lcms2, wgpu, ash)
- **License compliance** — verify all transitive deps are AGPL-3.0 compatible
- **Test coverage gaps** — roundtrip tests for `.rasa` format, GPU device fallback, canvas interactions, multi-provider AI routing

### Security
- **Input validation** — fuzz/harden image format parsers (PNG, PSD, RAW), reject malformed `.rasa` files
- **Path traversal** — sanitize file paths in import/export, MCP tool inputs, plugin loading
- **AI API surface** — validate/sanitize prompts before sending to Synapse, rate-limit inference requests
- **Secret handling** — ensure no API keys/tokens logged or serialized to `.rasa` files
- **MAX_HEADER_SIZE** — review all size limits in project.rs, catalog.rs for DoS resilience

### Performance
- **GPU compositing path** — move layer compositing to wgpu compute shaders
- **Parallel vector rasterization** — rayon per-row for `render_filled`/`render_stroked`
- **Lazy font loading** — cache system font discovery result, avoid re-scanning per text layer render
- **Tile-based incremental render** — wire `RenderCache` into the UI paint loop
- **Memory profiling** — audit large-buffer allocations in AI pipeline, compositor, export path
- **Benchmark suite** — criterion benchmarks for compositor, filters, vector render, AI pre/post-processing

---

## Post-MVP v2

### Platform
- **Tablet optimization** — touch UI mode, stylus gestures
- **Dynamic plugin loading** — WASM or native dylib discovery at runtime

### Ecosystem Integration
*(All items complete)*
