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
| `rasa-gpu` | Vulkan/Metal compute shaders, GPU-accelerated filters and compositing |
| `rasa-engine` | Rendering pipeline, compositing, filter chain, brush engine |
| `rasa-storage` | File I/O: PNG, JPEG, TIFF, WebP, BMP, GIF, native `.rasa` project format |
| `rasa-ai` | AI inference pipeline via hoosh/Synapse: inpainting, upscaling, segmentation, generation |
| `rasa-ui` | Desktop GUI built with egui/eframe: canvas viewport, tool palette, layer panel |
| `rasa-mcp` | MCP 2.0 server (stdio) exposing 5 tools for Claude integration |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust 2024 edition, MSRV 1.85 |
| GPU | Vulkan/Metal via `wgpu`, 9 WGSL compute shaders |
| Image I/O | `image` crate (PNG, JPEG, WebP, TIFF, BMP, GIF) |
| AI Inference | hoosh/Synapse HTTP API for Stable Diffusion, RealESRGAN, SAM, U2Net |
| GUI | `egui` / `eframe` (Wayland-compatible via winit) |
| Database | SQLite via `rusqlite` (recent files catalog) |
| Serialization | `serde` / `serde_json` (document model, MCP protocol) |

## Quick Start

```bash
# Build
make build

# Run
make run

# Run tests (410 tests)
make test

# Full quality check (fmt + lint + test)
make check

# Coverage report
make test-coverage
```

## AI Features

Powered by hoosh/Synapse for local inference:

- **Inpainting** — mask and regenerate regions with context-aware fill
- **Upscaling** — AI super-resolution (2x, 4x) via RealESRGAN
- **Background Removal** — automatic subject segmentation via U2Net
- **Generative Fill** — extend or replace image content with text prompts
- **AI Selection** — intelligent object selection via SAM ViT-H

## MCP Tools (Claude Integration)

Run the MCP server: `rasa-mcp` (stdio transport, JSON-RPC 2.0)

| Tool | Description |
|------|-------------|
| `rasa_open_image` | Open an image file or create a new blank document |
| `rasa_edit_layer` | Add, remove, rename, reorder, duplicate, merge layers; set opacity/blend mode/visibility |
| `rasa_apply_filter` | Apply brightness/contrast, hue/saturation, blur, sharpen, invert, grayscale |
| `rasa_get_document` | Get document state: layers, dimensions, undo/redo status |
| `rasa_export` | Export composited document to PNG, JPEG, WebP, or TIFF |

## AGNOS Voice Control

5 agnoshi intents for voice-driven editing via the AGNOS platform:

| Intent | Examples |
|--------|----------|
| `rasa.open` | "open photo.png", "create a new 1920 by 1080 canvas" |
| `rasa.filter` | "blur this layer", "increase brightness by 20" |
| `rasa.layer` | "add a new layer", "set opacity to 50", "merge down" |
| `rasa.export` | "export as PNG", "save as JPEG" |
| `rasa.ai` | "remove the background", "upscale this image" |

## Ecosystem

Rasa is part of the AGNOS AI-native creative suite:

| Project | Role |
|---------|------|
| [**Tazama**](https://github.com/anomalyco/tazama) | AI-native video editor |
| **Rasa** | AI-native image editor (this project) |
| [**Shruti**](https://github.com/MacCracken/shruti) | AI-native DAW / audio workstation |
| [**Delta**](https://github.com/agnostos/delta) | Code hosting platform |
| [**BullShift**](https://github.com/MacCracken/BullShift) | Trading platform |

## Design Principles

1. **Local-first** — all AI inference runs on-device, no cloud dependency
2. **GPU-native** — Vulkan/Metal compute for rendering and AI, not an afterthought
3. **Zero-I/O core** — `rasa-core` has no filesystem, network, or async runtime
4. **Non-destructive** — layer-based editing with full undo/redo history
5. **AI-augmented, not AI-replaced** — tools enhance the artist, not replace them

## License

AGPL-3.0 — see [LICENSE](LICENSE) for details.

Copyright (C) 2026 Robert MacCracken
