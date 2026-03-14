# Rasa

*Sanskrit: रस (essence, flavor, aesthetic emotion — the core concept in Indian aesthetics)*

AI-native image editor and design tool built in Rust with GPU-accelerated rendering and local generative AI.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   rasa (binary)                  │
├──────────┬──────────┬──────────┬────────────────┤
│  rasa-ui │ rasa-mcp │ rasa-ai  │  rasa-storage  │
├──────────┴──────────┴──────────┴────────────────┤
│                  rasa-engine                     │
│            (rendering + compositing)             │
├──────────────────────┬──────────────────────────┤
│      rasa-gpu        │       rasa-core           │
│  (Vulkan compute)    │  (types, document model)  │
└──────────────────────┴──────────────────────────┘
```

| Crate | Purpose |
|-------|---------|
| `rasa-core` | Zero-I/O core types: document model, layers, color, selections, transforms |
| `rasa-gpu` | Vulkan compute shaders, GPU-accelerated filters and compositing |
| `rasa-engine` | Rendering pipeline, compositing, filter chain, brush engine |
| `rasa-storage` | File I/O: PNG, JPEG, TIFF, WebP, PSD, SVG, project format |
| `rasa-ai` | AI inference pipeline: inpainting, upscaling, segmentation, generation |
| `rasa-ui` | GUI: canvas viewport, tool palette, layer panel, properties |
| `rasa-mcp` | MCP 2.0 server exposing 5 tools for Claude integration |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust (2024 edition) |
| GPU | Vulkan via `ash` / `wgpu` |
| Rendering | `tiny-skia`, custom Vulkan pipelines |
| Image I/O | `image`, `libpng`, `libjpeg-turbo`, `libwebp` |
| AI Inference | ONNX Runtime, local Stable Diffusion via hoosh/Synapse |
| GUI | `iced` or custom Vulkan UI |
| Database | SQLite via `sqlx` (project metadata, asset catalog) |
| IPC | PipeWire (color picker), Wayland clipboard |

## Quick Start

```bash
# Build
make build

# Run
make run

# Development mode (auto-reload)
make dev

# Run tests
make test

# Full quality check (fmt + lint + test)
make check
```

## AI Features

- **Inpainting** — mask and regenerate regions with context-aware fill
- **Upscaling** — AI super-resolution (2x, 4x) preserving detail
- **Background Removal** — automatic subject segmentation and extraction
- **Generative Fill** — extend or replace image content with text prompts
- **Style Transfer** — apply artistic styles to images or selections
- **Text-to-Image** — generate images from natural language via local Stable Diffusion
- **AI Selection** — intelligent edge detection and object selection
- **AI Color Grading** — automatic and prompt-driven color correction

## MCP Tools (Claude Integration)

| Tool | Description |
|------|-------------|
| `rasa_open_image` | Open or create an image document |
| `rasa_edit_layer` | Add, modify, or transform layers |
| `rasa_apply_filter` | Apply filters, adjustments, or AI effects |
| `rasa_get_document` | Get document state (layers, dimensions, history) |
| `rasa_export` | Export to image file (PNG, JPEG, WebP, TIFF) |

## Ecosystem

Rasa is part of the AGNOS AI-native creative suite:

| Project | Role |
|---------|------|
| [**Tazama**](https://github.com/anomalyco/tazama) | AI-native video editor |
| **Rasa** | AI-native image editor (this project) |
| [**Shruti**](https://github.com/MacCracken/shruti) | AI-native DAW / audio workstation |
| [**Delta**](https://github.com/agnostos/delta) | Code hosting platform |
| [**BullShift**](https://github.com/MacCracken/BullShift) | Trading platform |

Together, Tazama + Rasa + Shruti form a coherent AI-native creative platform running entirely on local hardware via AGNOS.

## Design Principles

1. **Local-first** — all AI inference runs on-device, no cloud dependency
2. **GPU-native** — Vulkan compute for rendering and AI, not an afterthought
3. **Zero-I/O core** — `rasa-core` has no filesystem, network, or async runtime
4. **Non-destructive** — layer-based editing with full undo history
5. **AI-augmented, not AI-replaced** — tools enhance the artist, not replace them

## License

AGPL-3.0 — see [LICENSE](LICENSE) for details.

Copyright (C) 2026 Robert MacCracken
