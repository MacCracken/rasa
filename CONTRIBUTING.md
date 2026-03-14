# Contributing to Rasa

## Getting Started

1. Clone the repo and install Rust 1.85+
2. Run `make build` to verify the build
3. Run `make check` to run the full quality suite

## Architecture Rules

- **`rasa-core` must have zero I/O** — no tokio, no filesystem, no network calls. Pure types and logic only.
- **GPU code stays in `rasa-gpu`** — no Vulkan/wgpu types leak into core or engine public APIs.
- **AI inference stays in `rasa-ai`** — Synapse API client, model management, and inference pipelines are isolated.
- All command handlers in thin wrappers — business logic lives in core.

## Code Quality

Before submitting:

```bash
make check    # fmt + clippy + tests
```

- `cargo fmt --all` — consistent formatting
- `cargo clippy -- -D warnings` — warnings are errors
- `cargo test --workspace` — all tests pass (currently 410+)
- Target 85%+ test coverage on testable crates (core, engine, storage, mcp)

## Commit Messages

Use concise, descriptive commit messages:
- `add: layer blending modes` (new feature)
- `fix: brush opacity calculation` (bug fix)
- `refactor: extract compositing pipeline` (restructure)

## Crate Guidelines

| Crate | Can depend on | Cannot depend on |
|-------|--------------|-----------------|
| `rasa-core` | std, serde, uuid, thiserror | anything with I/O |
| `rasa-gpu` | `rasa-core`, wgpu | `rasa-engine`, `rasa-ai`, `rasa-storage` |
| `rasa-engine` | `rasa-core`, `rasa-gpu` | `rasa-ui`, `rasa-mcp` |
| `rasa-storage` | `rasa-core`, image, rusqlite | `rasa-gpu`, `rasa-ui` |
| `rasa-ai` | `rasa-core`, `rasa-engine`, `rasa-storage` | `rasa-ui`, `rasa-mcp` |
| `rasa-ui` | all internal crates, egui/eframe | — |
| `rasa-mcp` | all internal crates except `rasa-gpu` | — |

## Testing

- Unit tests live in `#[cfg(test)] mod tests` within each source file
- Integration tests live in `crates/*/tests/`
- Serde round-trip tests cover all serializable types
- GPU and AI tests degrade gracefully when hardware/services unavailable
