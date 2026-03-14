use rasa_core::error::RasaError;
use rasa_core::pixel::PixelBuffer;

use crate::models::{self, ModelId};
use crate::pipeline::{AiPipeline, AiRequest, AiResult, ProgressCallback};

/// Parameters for text-to-image generation.
#[derive(Debug, Clone)]
pub struct GenerateParams {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub cfg_scale: f32,
    pub seed: Option<u64>,
    pub model: Option<ModelId>,
}

impl Default for GenerateParams {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            negative_prompt: None,
            width: 512,
            height: 512,
            steps: 30,
            cfg_scale: 7.5,
            seed: None,
            model: None,
        }
    }
}

/// Generate an image from a text prompt.
pub async fn generate(
    pipeline: &AiPipeline,
    params: &GenerateParams,
    on_progress: Option<ProgressCallback>,
) -> Result<PixelBuffer, RasaError> {
    if params.prompt.is_empty() {
        return Err(RasaError::Other("prompt cannot be empty".into()));
    }

    let model = params
        .model
        .clone()
        .unwrap_or_else(models::presets::stable_diffusion_xl);

    // Use a dummy 1x1 input — generation doesn't need input pixels
    let dummy = PixelBuffer::new(1, 1);

    let request = AiRequest::Generate {
        model,
        prompt: params.prompt.clone(),
        negative_prompt: params.negative_prompt.clone(),
        width: params.width,
        height: params.height,
        steps: params.steps,
        cfg_scale: params.cfg_scale,
        seed: params.seed,
    };

    let result = pipeline.run(&request, &dummy, on_progress).await?;
    match result {
        AiResult::Image(buf) => Ok(buf),
        _ => Err(RasaError::InferenceFailed(
            "unexpected result type from generation".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params() {
        let params = GenerateParams::default();
        assert_eq!(params.width, 512);
        assert_eq!(params.height, 512);
        assert_eq!(params.steps, 30);
        assert!((params.cfg_scale - 7.5).abs() < 0.01);
    }

    #[test]
    fn generate_request_serializes() {
        let request = AiRequest::Generate {
            model: models::presets::stable_diffusion_xl(),
            prompt: "a cat".into(),
            negative_prompt: Some("blurry".into()),
            width: 512,
            height: 512,
            steps: 30,
            cfg_scale: 7.5,
            seed: Some(42),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("a cat"));
        assert!(json.contains("42"));
    }
}
