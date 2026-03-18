use rasa_core::error::RasaError;
use rasa_core::geometry::Rect;
use serde::{Deserialize, Serialize};

use crate::models::{ModelId, ModelInfo};

/// HTTP client for the hoosh/Synapse AI inference API.
pub struct SynapseClient {
    base_url: String,
    http: reqwest::Client,
}

/// API response wrapper.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiResponse<T> {
    #[serde(default)]
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

/// Model listing from the API.
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

impl SynapseClient {
    pub fn new(base_url: &str) -> Self {
        let timeout_secs: u64 = std::env::var("RASA_AI_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .expect("failed to create HTTP client"),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Health check — returns true if the API is reachable.
    pub async fn health(&self) -> Result<bool, RasaError> {
        let url = format!("{}/v1/health", self.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// List available models.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, RasaError> {
        let url = format!("{}/v1/models", self.base_url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| RasaError::InferenceFailed(format!("request failed: {e}")))?;
        let body: ApiResponse<ModelsResponse> = resp
            .json()
            .await
            .map_err(|e| RasaError::InferenceFailed(format!("parse failed: {e}")))?;
        match body.data {
            Some(data) => Ok(data.models),
            None => Ok(vec![]),
        }
    }

    /// Inpainting: send image + mask region, get inpainted image back.
    pub async fn inpaint(
        &self,
        image_png: &[u8],
        model: &ModelId,
        prompt: Option<&str>,
        mask_region: &Rect,
    ) -> Result<Vec<u8>, RasaError> {
        let url = format!("{}/v1/images/inpaint", self.base_url);
        let mut form = reqwest::multipart::Form::new()
            .text("model", model.0.clone())
            .text("mask_x", mask_region.x.to_string())
            .text("mask_y", mask_region.y.to_string())
            .text("mask_width", mask_region.width.to_string())
            .text("mask_height", mask_region.height.to_string())
            .part(
                "image",
                reqwest::multipart::Part::bytes(image_png.to_vec())
                    .file_name("input.png")
                    .mime_str("image/png")
                    .expect("image/png is a valid MIME type"),
            );
        if let Some(p) = prompt {
            form = form.text("prompt", p.to_string());
        }
        self.post_multipart_image(&url, form).await
    }

    /// Upscale: send image, get upscaled image back.
    pub async fn upscale(
        &self,
        image_png: &[u8],
        model: &ModelId,
        scale_factor: u32,
    ) -> Result<Vec<u8>, RasaError> {
        let url = format!("{}/v1/images/upscale", self.base_url);
        let form = reqwest::multipart::Form::new()
            .text("model", model.0.clone())
            .text("scale_factor", scale_factor.to_string())
            .part(
                "image",
                reqwest::multipart::Part::bytes(image_png.to_vec())
                    .file_name("input.png")
                    .mime_str("image/png")
                    .expect("image/png is a valid MIME type"),
            );
        self.post_multipart_image(&url, form).await
    }

    /// Segmentation: send image, get mask image back.
    pub async fn segment(&self, image_png: &[u8], model: &ModelId) -> Result<Vec<u8>, RasaError> {
        let url = format!("{}/v1/images/segment", self.base_url);
        let form = reqwest::multipart::Form::new()
            .text("model", model.0.clone())
            .part(
                "image",
                reqwest::multipart::Part::bytes(image_png.to_vec())
                    .file_name("input.png")
                    .mime_str("image/png")
                    .expect("image/png is a valid MIME type"),
            );
        self.post_multipart_image(&url, form).await
    }

    /// Text-to-image generation.
    #[allow(clippy::too_many_arguments)]
    pub async fn generate(
        &self,
        model: &ModelId,
        prompt: &str,
        negative_prompt: Option<&str>,
        width: u32,
        height: u32,
        steps: u32,
        cfg_scale: f32,
        seed: Option<u64>,
    ) -> Result<Vec<u8>, RasaError> {
        let url = format!("{}/v1/images/generate", self.base_url);

        #[derive(Serialize)]
        struct GenerateRequest<'a> {
            model: &'a str,
            prompt: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            negative_prompt: Option<&'a str>,
            width: u32,
            height: u32,
            steps: u32,
            cfg_scale: f32,
            #[serde(skip_serializing_if = "Option::is_none")]
            seed: Option<u64>,
        }

        let body = GenerateRequest {
            model: &model.0,
            prompt,
            negative_prompt,
            width,
            height,
            steps,
            cfg_scale,
            seed,
        };

        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| RasaError::InferenceFailed(format!("generate request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(RasaError::InferenceFailed(format!(
                "generate failed: HTTP {}",
                resp.status()
            )));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| RasaError::InferenceFailed(format!("failed to read response: {e}")))
    }

    /// Style transfer: send image + style parameters, get stylized image back.
    pub async fn style_transfer(
        &self,
        image_png: &[u8],
        style: &str,
        strength: f32,
    ) -> Result<Vec<u8>, RasaError> {
        let url = format!("{}/v1/images/style-transfer", self.base_url);
        let form = reqwest::multipart::Form::new()
            .text("style", style.to_string())
            .text("strength", strength.to_string())
            .part(
                "image",
                reqwest::multipart::Part::bytes(image_png.to_vec())
                    .file_name("input.png")
                    .mime_str("image/png")
                    .expect("image/png is a valid MIME type"),
            );
        self.post_multipart_image(&url, form).await
    }

    /// Color grading: send image + preset parameters, get color-graded image back.
    pub async fn color_grade(
        &self,
        image_png: &[u8],
        preset: &str,
        intensity: f32,
    ) -> Result<Vec<u8>, RasaError> {
        let url = format!("{}/v1/images/color-grade", self.base_url);
        let form = reqwest::multipart::Form::new()
            .text("preset", preset.to_string())
            .text("intensity", intensity.to_string())
            .part(
                "image",
                reqwest::multipart::Part::bytes(image_png.to_vec())
                    .file_name("input.png")
                    .mime_str("image/png")
                    .expect("image/png is a valid MIME type"),
            );
        self.post_multipart_image(&url, form).await
    }

    /// Background removal: send image, get image with transparent background.
    pub async fn remove_background(
        &self,
        image_png: &[u8],
        model: &ModelId,
    ) -> Result<Vec<u8>, RasaError> {
        let url = format!("{}/v1/images/remove-background", self.base_url);
        let form = reqwest::multipart::Form::new()
            .text("model", model.0.clone())
            .part(
                "image",
                reqwest::multipart::Part::bytes(image_png.to_vec())
                    .file_name("input.png")
                    .mime_str("image/png")
                    .expect("image/png is a valid MIME type"),
            );
        self.post_multipart_image(&url, form).await
    }

    /// Send a multipart form and expect image bytes back.
    async fn post_multipart_image(
        &self,
        url: &str,
        form: reqwest::multipart::Form,
    ) -> Result<Vec<u8>, RasaError> {
        let resp = self
            .http
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| RasaError::InferenceFailed(format!("request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(RasaError::InferenceFailed(format!("HTTP {status}: {body}")));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| RasaError::InferenceFailed(format!("failed to read response: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_construction() {
        let client = SynapseClient::new("http://localhost:8090/");
        assert_eq!(client.base_url(), "http://localhost:8090");
    }

    #[test]
    fn client_strips_trailing_slashes() {
        let client = SynapseClient::new("http://host:9000///");
        assert_eq!(client.base_url(), "http://host:9000");
    }

    #[tokio::test]
    async fn health_unreachable_returns_false() {
        let client = SynapseClient::new("http://127.0.0.1:1");
        let result = client.health().await.unwrap();
        assert!(!result);
    }
}
