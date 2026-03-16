# AI Features Guide

Rasa includes an integrated AI pipeline for image editing tasks powered by deep learning models. This guide covers all available AI capabilities, how providers work, and how to extend the system.

## Capabilities Overview

| Feature | Module | Description |
|---------|--------|-------------|
| Inpainting | `inpainting` | Context-aware fill of masked regions |
| Upscaling | `upscaling` | AI super-resolution (2x/4x) |
| Segmentation | `segmentation` | Foreground/background mask extraction |
| Background Removal | `segmentation` | Remove background with transparency |
| Text-to-Image | `generation` | Generate images from text prompts |
| Style Transfer | `workflow` | Apply artistic styles to images |
| Color Grading | `workflow` | AI-driven cinematic color grading |

## Provider Architecture

Rasa decouples AI operations from specific backends through the `InferenceProvider` trait. Any backend that can generate images, apply styles, or grade colors can be plugged in.

### Built-in Provider: Synapse (Local)

The default provider runs against a local Synapse/hoosh inference server. It requires no API keys and keeps all data on-device.

```rust
use rasa_ai::provider_synapse::SynapseProvider;
use rasa_ai::registry::ProviderRegistry;

let mut registry = ProviderRegistry::new();
registry.register(Box::new(SynapseProvider::new("http://localhost:8090")));
```

### Adding a Custom Provider

Implement the `InferenceProvider` trait:

```rust
use rasa_ai::provider::{InferenceProvider, GenerationParams};
use rasa_core::error::RasaError;
use std::pin::Pin;

struct MyCloudProvider { /* ... */ }

impl InferenceProvider for MyCloudProvider {
    fn name(&self) -> &str { "My Cloud AI" }

    fn is_available(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async { true })
    }

    fn text_to_image(
        &self,
        prompt: &str,
        negative_prompt: &str,
        width: u32,
        height: u32,
        params: &GenerationParams,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>> {
        // Call your cloud API, return PNG bytes
        Box::pin(async { todo!() })
    }

    fn style_transfer(/* ... */) -> Pin<Box</* ... */>> { /* ... */ }
    fn color_grade(/* ... */) -> Pin<Box</* ... */>> { /* ... */ }
}
```

Register it alongside or instead of the default:

```rust
registry.register(Box::new(MyCloudProvider::new("sk-...")));
```

### Provider Registry

The `ProviderRegistry` stores all registered providers and exposes lookup methods:

- `default_provider()` -- returns the first registered provider (or the one set via `set_default`)
- `provider_by_name(name)` -- look up by the string returned from `InferenceProvider::name()`
- `list_providers()` -- list all registered provider names

## Style Transfer

Style transfer applies an artistic style to an existing image layer.

### Supported Styles

The following style identifiers are recognized by the Synapse backend:

- `oil-painting`
- `watercolor`
- `pencil-sketch`
- `anime`
- `impressionist`
- `pop-art`
- `mosaic`

Custom providers may support additional styles.

### Parameters

- **`style`** (string): the style identifier
- **`strength`** (f32, 0.0--1.0): how strongly the style is applied. Lower values preserve more of the original image.

### Usage

```rust
use rasa_ai::workflow;
use rasa_ai::pipeline::AiPipeline;

let pipeline = AiPipeline::new("http://localhost:8090");
let new_layer_id = workflow::style_transfer(
    &pipeline,
    &mut doc,
    layer_id,
    "oil-painting",
    0.75,      // strength
    None,      // default model
    None,      // no progress callback
).await?;
```

## Color Grading

AI color grading applies cinematic or photographic color looks to a layer.

### Presets

- `cinematic-warm` -- warm, orange-teal cinematic tones
- `cinematic-cool` -- cool blue cinematic tones
- `vintage-film` -- faded film stock look
- `noir` -- high-contrast black and white
- `vibrant` -- boosted saturation and contrast
- `desaturated` -- muted, low-saturation look
- `golden-hour` -- warm golden sunlight tones
- `moonlight` -- cool blue nighttime tones

### Parameters

- **`preset`** (string): the grading preset name
- **`intensity`** (f32, 0.0--1.0): blending strength between original and graded image

### Usage

```rust
let new_layer_id = workflow::color_grade_layer(
    &pipeline,
    &mut doc,
    layer_id,
    "cinematic-warm",
    0.8,       // intensity
    None,      // default model
    None,      // no progress callback
).await?;
```

## Text-to-Image Generation

Generate images from text descriptions using diffusion models.

### Parameters

- **`prompt`** (string, required): description of the image to generate
- **`negative_prompt`** (string, optional): what to avoid in the generation
- **`width`** / **`height`** (u32): output dimensions (default: 512x512)
- **`steps`** (u32): diffusion steps, higher = better quality, slower (default: 30)
- **`cfg_scale`** (f32): classifier-free guidance scale (default: 7.5)
- **`seed`** (u64, optional): for reproducible results

### Usage via Pipeline

```rust
use rasa_ai::generation::GenerateParams;
use rasa_ai::workflow;

let params = GenerateParams {
    prompt: "a sunset over mountains, oil painting".into(),
    negative_prompt: Some("blurry, low quality".into()),
    width: 1024,
    height: 1024,
    steps: 40,
    cfg_scale: 7.5,
    seed: Some(42),
    model: None,
};

let layer_id = workflow::generate_to_layer(&pipeline, &mut doc, &params, None).await?;
```

### Usage via Provider (provider-agnostic)

```rust
use rasa_ai::provider::GenerationParams;

let provider = registry.default_provider().unwrap();
let png_bytes = provider.text_to_image(
    "a cat in space",
    "blurry",
    512, 512,
    &GenerationParams::default(),
).await?;
```

## Current Limitations

- **Synapse server required**: the local provider needs a running Synapse/hoosh instance. Without it, all AI operations return connection errors.
- **No streaming**: results are returned as complete PNG byte buffers; there is no progressive/streaming output yet.
- **Single active task**: the `AiPipeline` serializes AI operations -- only one task runs at a time.
- **Style/grading endpoints**: the Synapse server must expose `/v1/images/style-transfer` and `/v1/images/color-grade` endpoints for the new features to work.
- **Provider routing**: the `ProviderRegistry` is currently separate from `AiPipeline`. A future update will integrate them so pipeline operations can be routed to any registered provider.
