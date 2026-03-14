use std::sync::Arc;

use rasa_core::error::RasaError;
use rasa_core::geometry::Rect;
use rasa_core::pixel::PixelBuffer;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::client::SynapseClient;
use crate::models::ModelId;

/// The central AI inference pipeline.
///
/// Coordinates model loading, pre/post-processing, and inference
/// for all AI features (inpainting, upscaling, segmentation, generation).
pub struct AiPipeline {
    client: Arc<SynapseClient>,
    state: Arc<Mutex<PipelineState>>,
}

#[derive(Debug)]
struct PipelineState {
    active_task: Option<TaskHandle>,
}

/// Handle to a running AI task for progress tracking and cancellation.
#[derive(Debug, Clone)]
pub struct TaskHandle {
    pub id: String,
    pub kind: TaskKind,
    pub progress: f32,
    pub cancelled: bool,
}

/// Kind of AI task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    Inpaint,
    Upscale,
    Segment,
    Generate,
    RemoveBackground,
}

/// Parameters for an AI operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiRequest {
    Inpaint {
        model: ModelId,
        prompt: Option<String>,
        mask_region: Rect,
    },
    Upscale {
        model: ModelId,
        scale_factor: u32,
    },
    Segment {
        model: ModelId,
    },
    Generate {
        model: ModelId,
        prompt: String,
        negative_prompt: Option<String>,
        width: u32,
        height: u32,
        steps: u32,
        cfg_scale: f32,
        seed: Option<u64>,
    },
    RemoveBackground {
        model: ModelId,
    },
}

/// Result of an AI operation.
#[derive(Debug)]
pub enum AiResult {
    /// Pixel buffer output (inpainting, upscaling, generation, background removal).
    Image(PixelBuffer),
    /// Segmentation mask (0.0 = background, 1.0 = foreground).
    Mask {
        width: u32,
        height: u32,
        data: Vec<f32>,
    },
}

/// Progress callback type.
pub type ProgressCallback = Box<dyn Fn(f32) + Send + Sync>;

impl AiPipeline {
    pub fn new(synapse_url: &str) -> Self {
        Self {
            client: Arc::new(SynapseClient::new(synapse_url)),
            state: Arc::new(Mutex::new(PipelineState { active_task: None })),
        }
    }

    /// Check if the AI backend (hoosh/Synapse) is reachable.
    pub async fn health_check(&self) -> Result<bool, RasaError> {
        self.client.health().await
    }

    /// List available models from the backend.
    pub async fn list_models(&self) -> Result<Vec<crate::models::ModelInfo>, RasaError> {
        self.client.list_models().await
    }

    /// Run an AI operation on the given input buffer.
    pub async fn run(
        &self,
        request: &AiRequest,
        input: &PixelBuffer,
        on_progress: Option<ProgressCallback>,
    ) -> Result<AiResult, RasaError> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let kind = match request {
            AiRequest::Inpaint { .. } => TaskKind::Inpaint,
            AiRequest::Upscale { .. } => TaskKind::Upscale,
            AiRequest::Segment { .. } => TaskKind::Segment,
            AiRequest::Generate { .. } => TaskKind::Generate,
            AiRequest::RemoveBackground { .. } => TaskKind::RemoveBackground,
        };

        // Register active task
        {
            let mut state = self.state.lock().await;
            state.active_task = Some(TaskHandle {
                id: task_id.clone(),
                kind,
                progress: 0.0,
                cancelled: false,
            });
        }

        if let Some(ref cb) = on_progress {
            cb(0.0);
        }

        // Pre-process: convert pixel buffer to PNG bytes for API transport
        let input_bytes = preprocess_to_png(input)?;

        if let Some(ref cb) = on_progress {
            cb(0.1);
        }

        // Check for cancellation
        if self.is_cancelled(&task_id).await {
            return Err(RasaError::Other("task cancelled".into()));
        }

        // Dispatch to appropriate handler
        let result = match request {
            AiRequest::Inpaint {
                model,
                prompt,
                mask_region,
            } => {
                let response = self
                    .client
                    .inpaint(&input_bytes, model, prompt.as_deref(), mask_region)
                    .await?;
                if let Some(ref cb) = on_progress {
                    cb(0.8);
                }
                postprocess_image(&response)?
            }
            AiRequest::Upscale {
                model,
                scale_factor,
            } => {
                let response = self
                    .client
                    .upscale(&input_bytes, model, *scale_factor)
                    .await?;
                if let Some(ref cb) = on_progress {
                    cb(0.8);
                }
                postprocess_image(&response)?
            }
            AiRequest::Segment { model } => {
                let response = self.client.segment(&input_bytes, model).await?;
                if let Some(ref cb) = on_progress {
                    cb(0.8);
                }
                postprocess_mask(&response, input.width, input.height)?
            }
            AiRequest::Generate {
                model,
                prompt,
                negative_prompt,
                width,
                height,
                steps,
                cfg_scale,
                seed,
            } => {
                let response = self
                    .client
                    .generate(
                        model,
                        prompt,
                        negative_prompt.as_deref(),
                        *width,
                        *height,
                        *steps,
                        *cfg_scale,
                        *seed,
                    )
                    .await?;
                if let Some(ref cb) = on_progress {
                    cb(0.8);
                }
                postprocess_image(&response)?
            }
            AiRequest::RemoveBackground { model } => {
                let response = self.client.remove_background(&input_bytes, model).await?;
                if let Some(ref cb) = on_progress {
                    cb(0.8);
                }
                postprocess_image(&response)?
            }
        };

        if let Some(ref cb) = on_progress {
            cb(1.0);
        }

        // Clear active task
        {
            let mut state = self.state.lock().await;
            state.active_task = None;
        }

        Ok(result)
    }

    /// Cancel the currently running task.
    pub async fn cancel(&self) {
        let mut state = self.state.lock().await;
        if let Some(ref mut task) = state.active_task {
            task.cancelled = true;
        }
    }

    /// Check if a task is currently running.
    pub async fn is_busy(&self) -> bool {
        let state = self.state.lock().await;
        state.active_task.is_some()
    }

    async fn is_cancelled(&self, task_id: &str) -> bool {
        let state = self.state.lock().await;
        state
            .active_task
            .as_ref()
            .is_some_and(|t| t.id == task_id && t.cancelled)
    }
}

/// Convert a PixelBuffer to PNG bytes for API transport.
fn preprocess_to_png(buf: &PixelBuffer) -> Result<Vec<u8>, RasaError> {
    rasa_storage::export::export_to_png_bytes(buf)
}

/// Convert API response (PNG bytes) back to a PixelBuffer.
fn postprocess_image(data: &[u8]) -> Result<AiResult, RasaError> {
    let img = image::load_from_memory(data)
        .map_err(|e| RasaError::Other(format!("failed to decode response image: {e}")))?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let mut buf = PixelBuffer::new(w, h);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        buf.set(
            x,
            y,
            rasa_core::color::Color::new(
                rasa_core::color::srgb_to_linear(r as f32 / 255.0),
                rasa_core::color::srgb_to_linear(g as f32 / 255.0),
                rasa_core::color::srgb_to_linear(b as f32 / 255.0),
                a as f32 / 255.0,
            ),
        );
    }
    Ok(AiResult::Image(buf))
}

/// Convert API response (raw mask bytes) to a mask result.
fn postprocess_mask(data: &[u8], _width: u32, _height: u32) -> Result<AiResult, RasaError> {
    // Expect response as a grayscale image
    let img = image::load_from_memory(data)
        .map_err(|e| RasaError::Other(format!("failed to decode mask response: {e}")))?;
    let gray = img.to_luma8();
    let mask_data: Vec<f32> = gray.pixels().map(|p| p.0[0] as f32 / 255.0).collect();
    Ok(AiResult::Mask {
        width: gray.width(),
        height: gray.height(),
        data: mask_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_kind_serialize() {
        let json = serde_json::to_string(&TaskKind::Inpaint).unwrap();
        assert_eq!(json, "\"Inpaint\"");
    }

    #[test]
    fn pipeline_new() {
        let pipeline = AiPipeline::new("http://localhost:8090");
        // Just verify construction doesn't panic
        assert!(Arc::strong_count(&pipeline.client) >= 1);
    }

    #[test]
    fn preprocess_produces_png() {
        let buf = PixelBuffer::filled(4, 4, rasa_core::color::Color::WHITE);
        let bytes = preprocess_to_png(&buf).unwrap();
        // PNG magic bytes
        assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn postprocess_image_from_png() {
        // Create a small PNG in memory
        let buf = PixelBuffer::filled(2, 2, rasa_core::color::Color::new(1.0, 0.0, 0.0, 1.0));
        let png_bytes = preprocess_to_png(&buf).unwrap();
        let result = postprocess_image(&png_bytes).unwrap();
        match result {
            AiResult::Image(buf) => {
                assert_eq!(buf.dimensions(), (2, 2));
                let px = buf.get(0, 0).unwrap();
                assert!(px.r > 0.9);
            }
            _ => panic!("expected Image result"),
        }
    }

    #[test]
    fn postprocess_invalid_data() {
        let result = postprocess_image(b"not an image");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pipeline_not_busy_initially() {
        let pipeline = AiPipeline::new("http://localhost:8090");
        assert!(!pipeline.is_busy().await);
    }
}
