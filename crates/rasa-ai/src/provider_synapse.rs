use std::pin::Pin;

use rasa_core::error::RasaError;

use crate::client::SynapseClient;
use crate::provider::{GenerationParams, InferenceProvider};

/// [`InferenceProvider`] backed by a local Synapse / hoosh inference server.
///
/// This is the default provider shipped with rasa. It delegates to the HTTP
/// API exposed by the Synapse runtime on the user's machine.
pub struct SynapseProvider {
    client: SynapseClient,
}

impl SynapseProvider {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: SynapseClient::new(base_url),
        }
    }

    /// Access the underlying [`SynapseClient`] for operations not covered by
    /// the generic [`InferenceProvider`] trait (e.g. model listing).
    pub fn client(&self) -> &SynapseClient {
        &self.client
    }
}

impl InferenceProvider for SynapseProvider {
    fn name(&self) -> &str {
        "Synapse (Local)"
    }

    fn is_available(&self) -> Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
        Box::pin(async { self.client.health().await.unwrap_or(false) })
    }

    fn text_to_image(
        &self,
        prompt: &str,
        negative_prompt: &str,
        width: u32,
        height: u32,
        params: &GenerationParams,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>> {
        let prompt = prompt.to_string();
        let negative_prompt = negative_prompt.to_string();
        let model_name = params
            .model
            .clone()
            .unwrap_or_else(|| "stable-diffusion-xl-base-1.0".to_string());
        let steps = params.steps;
        let cfg_scale = params.cfg_scale;
        let seed = params.seed;

        Box::pin(async move {
            let model_id = crate::models::ModelId::new(model_name);
            let neg = if negative_prompt.is_empty() {
                None
            } else {
                Some(negative_prompt.as_str())
            };
            self.client
                .generate(
                    &model_id, &prompt, neg, width, height, steps, cfg_scale, seed,
                )
                .await
        })
    }

    fn style_transfer(
        &self,
        image_png: &[u8],
        style: &str,
        strength: f32,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>> {
        let image = image_png.to_vec();
        let style = style.to_string();
        Box::pin(async move { self.client.style_transfer(&image, &style, strength).await })
    }

    fn color_grade(
        &self,
        image_png: &[u8],
        preset: &str,
        intensity: f32,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, RasaError>> + Send + '_>> {
        let image = image_png.to_vec();
        let preset = preset.to_string();
        Box::pin(async move { self.client.color_grade(&image, &preset, intensity).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synapse_provider_name() {
        let provider = SynapseProvider::new("http://localhost:8090");
        assert_eq!(provider.name(), "Synapse (Local)");
    }

    #[test]
    fn synapse_provider_creation() {
        let provider = SynapseProvider::new("http://127.0.0.1:9000/");
        assert_eq!(provider.client().base_url(), "http://127.0.0.1:9000");
    }

    #[tokio::test]
    async fn synapse_provider_unavailable_when_no_server() {
        let provider = SynapseProvider::new("http://127.0.0.1:1");
        assert!(!provider.is_available().await);
    }
}
