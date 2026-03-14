use rasa_core::error::RasaError;
use rasa_core::pixel::PixelBuffer;
use rasa_core::selection::Selection;

use crate::models::{self, ModelId};
use crate::pipeline::{AiPipeline, AiRequest, AiResult, ProgressCallback};

/// Segment an image to extract a foreground mask.
pub async fn segment(
    pipeline: &AiPipeline,
    input: &PixelBuffer,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<Selection, RasaError> {
    let model = model.unwrap_or_else(models::presets::sam_vit_h);
    let request = AiRequest::Segment { model };
    let result = pipeline.run(&request, input, on_progress).await?;
    match result {
        AiResult::Mask {
            width,
            height,
            data,
        } => Ok(Selection::Mask {
            width,
            height,
            data,
        }),
        _ => Err(RasaError::InferenceFailed(
            "unexpected result type from segmentation".into(),
        )),
    }
}

/// Remove the background from an image (returns image with transparent background).
pub async fn remove_background(
    pipeline: &AiPipeline,
    input: &PixelBuffer,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<PixelBuffer, RasaError> {
    let model = model.unwrap_or_else(models::presets::rembg_u2net);
    let request = AiRequest::RemoveBackground { model };
    let result = pipeline.run(&request, input, on_progress).await?;
    match result {
        AiResult::Image(buf) => Ok(buf),
        _ => Err(RasaError::InferenceFailed(
            "unexpected result type from background removal".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_request_serializes() {
        let request = AiRequest::Segment {
            model: models::presets::sam_vit_h(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("sam-vit-h"));
    }
}
