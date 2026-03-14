# Architecture Overview

## Crate Dependency Graph

```
rasa-ui ──────┬── rasa-engine ──┬── rasa-core (zero I/O)
              │                 └── rasa-gpu
rasa-mcp ─────┤
              ├── rasa-ai ──────┬── rasa-core
              │                 └── rasa-engine
              └── rasa-storage ─── rasa-core
```

## Key Principles

### Zero-I/O Core
`rasa-core` contains only pure types and logic. No tokio, no filesystem, no network. This makes it trivially testable and ensures the document model is completely decoupled from I/O concerns.

### GPU Isolation
All Vulkan/wgpu code lives in `rasa-gpu`. The engine crate uses GPU through trait abstractions so that CPU fallbacks work transparently on systems without GPU compute support.

### AI Isolation
`rasa-ai` owns all model loading, inference, and AI-specific pre/post-processing. It communicates with the rest of the system through `rasa-core` types (documents, layers, selections) — never raw tensors or model-specific formats.

### Non-Destructive Editing
All operations are commands that can be undone/redone. The document model stores the full layer stack, not a flattened bitmap. Filters and adjustments are represented as adjustment layers, not destructive pixel modifications.

## Data Flow

```
User Input → Tool → Command → Document Model → Render Pipeline → Canvas
                                    ↓
                              Storage (save)
                                    ↓
                            AI Pipeline (optional)
```
