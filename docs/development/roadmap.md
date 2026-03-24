# Roadmap

> **Version**: 2026.3.24
> **Status**: MVP v1 complete — all 10 phases done, post-MVP v1 complete
> **Tests**: 574 passing, 0 clippy warnings

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
- **muharrir editor engine** (v0.23.3) — command trait, dirty tracking, audit log, hierarchy display, inspector, notifications, multi-select, recent files, prefs, expression eval, hardware profiling | Done

---

## P0 — Code Health (Next)

### Critical: Tool Execution
- **Wire tool apply logic** — tools are registered but have no `apply()`/`execute()` method; brush painting logic exists in `rasa-engine` but isn't connected to the tool registry; clicking on canvas doesn't invoke any tool operation
- **Tool trait expansion** — add `on_pointer_down`, `on_pointer_move`, `on_pointer_up` to `Tool` trait for canvas interaction

### Code Refactor
- Dead code cleanup — remove `#[allow(dead_code)]` items (PSD export stubs, `rasa_blend_mode_to_psd()`)
- Consistent error handling — audit remaining `.expect()`/`.unwrap()` in non-test code (6 `.expect()` calls on MIME types in AI client)
- Module structure — evaluate whether `ActiveTool` enum can be retired in favor of `ToolRegistry`-only
- API surface audit — review pub visibility, seal internal types, reduce leaky abstractions

### Review & Audit
- Dependency audit — `cargo audit`, check for unused deps, pin versions
- Unsafe audit — verify zero `unsafe` blocks, audit FFI boundaries (lcms2, wgpu, ash)
- License compliance — verify all transitive deps are AGPL-3.0 compatible
- Test coverage gaps — roundtrip tests for `.rasa` format, GPU device fallback, canvas interactions, multi-provider AI routing

### Security
- Input validation — fuzz/harden image format parsers (PNG, PSD, RAW), reject malformed `.rasa` files
- Path traversal — sanitize file paths in import/export, MCP tool inputs, plugin loading
- AI API surface — validate/sanitize prompts before sending to Synapse, rate-limit inference requests
- Secret handling — ensure no API keys/tokens logged or serialized to `.rasa` files
- MCP rate limiting — add rate limiting if server is exposed externally (currently no auth/rate-limit)
- MAX_HEADER_SIZE — review all size limits in project.rs, catalog.rs for DoS resilience

### Performance
- GPU compositing path — move layer compositing to wgpu compute shaders (shaders exist, not fully wired to UI)
- Parallel vector rasterization — rayon per-row for `render_filled`/`render_stroked`
- Lazy font loading — cache system font discovery result, avoid re-scanning per text layer render
- Tile-based incremental render — wire `RenderCache` into the UI paint loop
- Memory profiling — audit large-buffer allocations in AI pipeline, compositor, export path
- Benchmark suite — criterion benchmarks for compositor, filters, vector render, AI pre/post-processing
- Layer flatten caching — `flatten_layers()` allocates + clones names every frame; cache and invalidate on layer change

---

## P1 — Professional Tools

### Brush Engine
- Custom brush shapes/dynamics (size/opacity falloff over stroke)
- Texture brushes, bristle simulation
- Pressure sensitivity curves
- Brush presets/library

### Selection Refinement
- Feathering, grow/shrink
- Select by color / magic wand
- Refine edge / mask edge
- Selection to path conversion
- Selection mode UI (Add/Subtract/Intersect — core exists, UI doesn't expose)

### Layer System
- Layer masks
- Non-destructive smart objects
- Adjustment layer UI (interactive curves, levels, hue/sat)
- Layer effects / styles
- Expanded blend modes (currently 12, Photoshop has 27+)

### Advanced Editing
- Clone/stamp tool
- Healing brush
- Content-aware fill (beyond AI inpainting)
- Liquify/warp transforms
- Perspective correction

### Color Management
- Interactive levels/curves UI
- Color picker with history
- Color palette management
- CMYK workflow support
- Soft proofing with ICC profiles

### Text Tool
- Interactive text editing on canvas (cursor, selection, inline editing)
- Font browser / preview
- Text style presets

### File Formats
- PSD export (stub exists, needs implementation)
- CLIP support
- SVG import/export improvements

---

## P2 — Platform & Polish

### UI
- Tabbed/multi-document interface
- Smart guides / snap-to-grid
- Customizable keyboard shortcuts
- Pixel grid toggle (code exists, non-functional)
- Ruler display improvements
- Vector/pen tool in tool palette (path editing)

### Platform
- Tablet optimization — touch UI mode, stylus gestures
- Dynamic plugin loading — WASM or native dylib discovery at runtime
- Plugin directory scanning, entry point discovery, sandboxing, version compat

### Accessibility
- Screen reader labels
- High contrast mode
- Keyboard navigation
