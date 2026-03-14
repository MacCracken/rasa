use rasa_core::error::RasaError;
use rasa_core::geometry::Rect;
use rasa_core::pixel::PixelBuffer;

use crate::models::{self, ModelId};
use crate::pipeline::{AiPipeline, AiRequest, AiResult, ProgressCallback};

/// Inpaint a region of an image.
///
/// Masks the specified region and uses AI to regenerate it
/// with context-aware content.
pub async fn inpaint(
    pipeline: &AiPipeline,
    input: &PixelBuffer,
    mask_region: Rect,
    prompt: Option<&str>,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<PixelBuffer, RasaError> {
    let model = model.unwrap_or_else(models::presets::stable_diffusion_inpaint);
    let request = AiRequest::Inpaint {
        model,
        prompt: prompt.map(String::from),
        mask_region,
    };
    let result = pipeline.run(&request, input, on_progress).await?;
    match result {
        AiResult::Image(buf) => Ok(buf),
        _ => Err(RasaError::InferenceFailed(
            "unexpected result type from inpainting".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_constructs() {
        let model = models::presets::stable_diffusion_inpaint();
        let request = AiRequest::Inpaint {
            model: model.clone(),
            prompt: Some("a blue sky".into()),
            mask_region: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Inpaint"));
        assert!(json.contains("blue sky"));
    }
}
