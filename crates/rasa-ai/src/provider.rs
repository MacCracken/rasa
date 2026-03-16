use std::future::Future;
use std::pin::Pin;

use rasa_core::error::RasaError;

/// Provider-agnostic generation parameters for text-to-image and related tasks.
#[derive(Debug, Clone)]
pub struct GenerationParams {
    pub steps: u32,
    pub cfg_scale: f32,
    pub seed: Option<u64>,
    pub model: Option<String>,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            steps: 30,
            cfg_scale: 7.5,
            seed: None,
            model: None,
        }
    }
}

/// A provider-agnostic interface for AI inference operations.
///
/// Each provider (local Synapse, Stability AI, Replicate, OpenAI DALL-E, etc.)
/// implements this trait to expose text-to-image, style transfer, and color
/// grading capabilities through a uniform API.
///
/// The trait is dyn-compatible so providers can be stored in a
/// [`crate::registry::ProviderRegistry`] behind `Box<dyn InferenceProvider>`.
pub trait InferenceProvider: Send + Sync {
    /// Provider name for display and logging.
    fn name(&self) -> &str;

    /// Check if the provider is currently available and reachable.
    fn is_available(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;

    /// Generate an image from a text prompt.
    ///
    /// Returns the generated image as PNG bytes.
    fn text_to_image(
        &self,
        prompt: &str,
        negative_prompt: &str,
        width: u32,
        height: u32,
        params: &GenerationParams,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>>;

    /// Apply artistic style transfer to an image.
    ///
    /// `style` identifies the target style (e.g. "oil-painting", "watercolor",
    /// "pencil-sketch", "anime", "impressionist").
    /// `strength` controls how strongly the style is applied (0.0..=1.0).
    ///
    /// Returns the stylized image as PNG bytes.
    fn style_transfer(
        &self,
        image_png: &[u8],
        style: &str,
        strength: f32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>>;

    /// Apply AI-driven color grading to an image.
    ///
    /// `preset` identifies the color grading look (e.g. "cinematic-warm",
    /// "cinematic-cool", "vintage-film", "noir", "vibrant", "desaturated",
    /// "golden-hour", "moonlight").
    /// `intensity` controls how strongly the grading is applied (0.0..=1.0).
    ///
    /// Returns the color-graded image as PNG bytes.
    fn color_grade(
        &self,
        image_png: &[u8],
        preset: &str,
        intensity: f32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_params_default() {
        let params = GenerationParams::default();
        assert_eq!(params.steps, 30);
        assert!((params.cfg_scale - 7.5).abs() < 0.01);
        assert!(params.seed.is_none());
        assert!(params.model.is_none());
    }

    #[test]
    fn generation_params_custom() {
        let params = GenerationParams {
            steps: 50,
            cfg_scale: 12.0,
            seed: Some(42),
            model: Some("custom-model-v2".into()),
        };
        assert_eq!(params.steps, 50);
        assert!((params.cfg_scale - 12.0).abs() < 0.01);
        assert_eq!(params.seed, Some(42));
        assert_eq!(params.model.as_deref(), Some("custom-model-v2"));
    }
}
