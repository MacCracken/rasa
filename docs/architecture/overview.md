# Architecture Overview

## Crate Dependency Graph

```
rasa-ui ──────┬── rasa-engine ──┬── rasa-core (zero I/O)
              │                 └── rasa-gpu
rasa-mcp ─────┤
              ├── rasa-ai ──────┬── rasa-core
              │                 ├── rasa-engine
              │                 └── rasa-storage
              └── rasa-storage ─── rasa-core
```

## Crate Summary

| Crate | Lines | Tests | Key Dependencies |
|-------|-------|-------|-----------------|
| `rasa-core` | ~1200 | 186 | serde, uuid, thiserror |
| `rasa-engine` | ~800 | 71 | rasa-core, rasa-gpu |
| `rasa-gpu` | ~900 | 20 | rasa-core, wgpu |
| `rasa-storage` | ~500 | 41 | rasa-core, image, rusqlite |
| `rasa-ai` | ~600 | 36 | rasa-core, rasa-engine, reqwest, tokio |
| `rasa-mcp` | ~700 | 47 | rasa-core, rasa-engine, rasa-storage, serde_json |
| `rasa-ui` | ~400 | 9 | rasa-core, rasa-engine, egui, eframe |

## Key Principles

### Zero-I/O Core
`rasa-core` contains only pure types and logic. No tokio, no filesystem, no network. This makes it trivially testable and ensures the document model is completely decoupled from I/O concerns. All types derive `Serialize`/`Deserialize` for persistence. `Color` uses `#[repr(C)]` for GPU compatibility.

### GPU Isolation
All Vulkan/wgpu code lives in `rasa-gpu`. The `RenderBackend` trait abstracts CPU vs GPU execution. The engine crate uses GPU through this abstraction so that CPU fallbacks work transparently on systems without GPU compute support. 9 WGSL compute shaders handle compositing (Normal/Multiply/Screen), per-pixel filters (invert, grayscale, brightness/contrast), blur passes, and brush dabs.

### AI Isolation
`rasa-ai` owns all inference via the hoosh/Synapse HTTP API. It communicates with the rest of the system through `rasa-core` types (documents, layers, selections, pixel buffers) — never raw tensors or model-specific formats. Operations: inpainting, upscaling (2x/4x), background removal, generative fill, AI selection.

### Non-Destructive Editing
All operations are commands that can be undone/redone. The document model stores the full layer stack, not a flattened bitmap. History uses `VecDeque` for O(1) eviction. Adjustment layers apply filters non-destructively during compositing.

### MCP Integration
`rasa-mcp` exposes the full editing pipeline to Claude via 5 MCP tools over stdio JSON-RPC 2.0. Session state manages multiple documents concurrently with Mutex poison recovery.

## Data Flow

```
User Input → Tool → Command → Document Model → Render Pipeline → Canvas
                                    ↓
                              Storage (save/export)
                                    ↓
                            AI Pipeline (optional)
                                    ↓
                              MCP (Claude tools)
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Synapse API over local ONNX | Simplifies dependency management; Synapse handles model lifecycle |
| egui over iced/GTK | Immediate-mode rendering fits image editor workflow; Wayland support via winit |
| rusqlite over sqlx | Synchronous catalog operations; avoids async runtime in storage crate |
| `#[repr(C)]` Color | Enables safe reinterpretation as `[f32; 4]` for GPU buffer upload |
| VecDeque for history | O(1) eviction of oldest commands vs Vec::remove(0) which is O(n) |
| Slice-based pixel access | Hot paths use `pixels()`/`pixels_mut()` slices instead of per-pixel `get`/`set` |
