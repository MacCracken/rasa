# ADR-011: AI Provider Abstraction

**Status:** Accepted
**Date:** 2026-03-16

## Context

Rasa's AI pipeline was originally coupled to a single inference backend (Synapse/hoosh), which runs locally. Users and integrators have asked for the ability to use external AI services (Stability AI, Replicate, OpenAI DALL-E, etc.) for text-to-image generation, style transfer, and color grading. Supporting multiple backends requires a provider-agnostic abstraction layer.

Additionally, three AI features were missing from the pipeline: style transfer, AI color grading, and provider-aware text-to-image generation.

## Decision

1. **`InferenceProvider` trait** (`rasa-ai/src/provider.rs`): a dyn-compatible trait that every AI backend must implement. Methods cover `text_to_image`, `style_transfer`, and `color_grade`. Return types use `Pin<Box<dyn Future<...>>>` to maintain dyn compatibility without requiring the `async_trait` crate.

2. **`SynapseProvider`** (`rasa-ai/src/provider_synapse.rs`): the default implementation, wrapping the existing `SynapseClient` HTTP client. Delegates to `POST /v1/images/style-transfer` and `POST /v1/images/color-grade` endpoints on the local Synapse server.

3. **`ProviderRegistry`** (`rasa-ai/src/registry.rs`): runtime container for `Box<dyn InferenceProvider>` instances. Supports registration, lookup by name, and a configurable default. The first provider registered becomes the default.

4. **New `ModelKind` variants**: `StyleTransfer` and `ColorGrading` were added to `ModelKind` along with corresponding preset model identifiers (`style-transfer-v1`, `color-grading-v1`).

5. **New `AiRequest` variants**: `StyleTransfer` and `ColorGrade` were added to the pipeline's request enum and wired through `AiPipeline::run()`.

6. **Workflow functions**: `style_transfer()` and `color_grade_layer()` provide high-level, document-aware operations that create new layers with the AI output.

## Rationale

- **Trait-based abstraction** keeps the core pipeline and UI code provider-agnostic. Adding a new backend (e.g. Stability AI) only requires implementing `InferenceProvider`.
- **Dyn-compatible design** allows the registry to store heterogeneous providers behind a single trait object.
- **Extending rather than replacing** the existing `AiPipeline` and `SynapseClient` avoids breaking changes to the 50+ existing tests and all downstream code.
- **`GenerationParams`** (in `provider.rs`) provides a provider-agnostic parameter struct distinct from the pipeline-specific `GenerateParams`, so external providers can map parameters without knowing rasa internals.

## Consequences

- External provider implementations will need to handle authentication and rate limiting themselves.
- The `SynapseProvider` assumes the Synapse server exposes `/v1/images/style-transfer` and `/v1/images/color-grade` endpoints; these need to be implemented on the server side.
- Future work: integrate the `ProviderRegistry` into the `AiPipeline` so that pipeline operations can be routed to the user's chosen provider at runtime.
