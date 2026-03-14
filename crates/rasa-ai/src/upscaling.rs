use rasa_core::error::RasaError;
use rasa_core::pixel::PixelBuffer;

use crate::models::{self, ModelId};
use crate::pipeline::{AiPipeline, AiRequest, AiResult, ProgressCallback};

/// Supported upscale factors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleFactor {
    X2,
    X4,
}

impl ScaleFactor {
    pub fn value(self) -> u32 {
        match self {
            Self::X2 => 2,
            Self::X4 => 4,
        }
    }
}

/// Upscale an image using AI super-resolution.
pub async fn upscale(
    pipeline: &AiPipeline,
    input: &PixelBuffer,
    scale: ScaleFactor,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<PixelBuffer, RasaError> {
    let model = model.unwrap_or_else(models::presets::real_esrgan_x4);
    let request = AiRequest::Upscale {
        model,
        scale_factor: scale.value(),
    };
    let result = pipeline.run(&request, input, on_progress).await?;
    match result {
        AiResult::Image(buf) => Ok(buf),
        _ => Err(RasaError::InferenceFailed(
            "unexpected result type from upscaling".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_factor_values() {
        assert_eq!(ScaleFactor::X2.value(), 2);
        assert_eq!(ScaleFactor::X4.value(), 4);
    }
}
