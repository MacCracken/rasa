use rasa_core::document::Document;
use rasa_core::error::RasaError;
use rasa_core::pixel::PixelBuffer;
use rasa_core::selection::Selection;
use uuid::Uuid;

use crate::apply;
use crate::generation::GenerateParams;
use crate::models::ModelId;
use crate::pipeline::{AiPipeline, AiRequest, ProgressCallback};
use crate::upscaling::ScaleFactor;

/// Inpaint the selected region of the active layer.
/// Creates a new layer with the result.
pub async fn inpaint_selection(
    pipeline: &AiPipeline,
    doc: &mut Document,
    layer_id: Uuid,
    selection: &Selection,
    prompt: Option<&str>,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<Uuid, RasaError> {
    let (region, bounds) = apply::extract_selection_region(doc, layer_id, selection)?;

    let model = model.unwrap_or_else(crate::models::presets::stable_diffusion_inpaint);
    let request = AiRequest::Inpaint {
        model,
        prompt: prompt.map(String::from),
        mask_region: bounds,
    };

    let result = pipeline.run(&request, &region, on_progress).await?;
    let new_id = apply::apply_as_new_layer(doc, &result, "Inpainted")?;
    Ok(new_id)
}

/// Upscale the entire active layer.
/// Replaces the layer's pixel data with the upscaled version.
pub async fn upscale_layer(
    pipeline: &AiPipeline,
    doc: &mut Document,
    layer_id: Uuid,
    scale: ScaleFactor,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<(), RasaError> {
    let input = doc
        .get_pixels(layer_id)
        .ok_or(RasaError::LayerNotFound(layer_id))?
        .clone();

    let model = model.unwrap_or_else(crate::models::presets::real_esrgan_x4);
    let request = AiRequest::Upscale {
        model,
        scale_factor: scale.value(),
    };

    let result = pipeline.run(&request, &input, on_progress).await?;
    apply::apply_as_new_layer(doc, &result, "Upscaled")?;
    Ok(())
}

/// Remove the background from a layer.
/// Creates a new layer with transparent background.
pub async fn remove_background(
    pipeline: &AiPipeline,
    doc: &mut Document,
    layer_id: Uuid,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<Uuid, RasaError> {
    let input = doc
        .get_pixels(layer_id)
        .ok_or(RasaError::LayerNotFound(layer_id))?
        .clone();

    let model = model.unwrap_or_else(crate::models::presets::rembg_u2net);
    let request = AiRequest::RemoveBackground { model };

    let result = pipeline.run(&request, &input, on_progress).await?;
    let new_id = apply::apply_as_new_layer(doc, &result, "No Background")?;
    Ok(new_id)
}

/// Generate an image from a text prompt and add it as a new layer.
pub async fn generate_to_layer(
    pipeline: &AiPipeline,
    doc: &mut Document,
    params: &GenerateParams,
    on_progress: Option<ProgressCallback>,
) -> Result<Uuid, RasaError> {
    if params.prompt.is_empty() {
        return Err(RasaError::Other("prompt cannot be empty".into()));
    }

    let model = params
        .model
        .clone()
        .unwrap_or_else(crate::models::presets::stable_diffusion_xl);

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
    let name = format!("Generated: {}", truncate(&params.prompt, 30));
    let new_id = apply::apply_as_new_layer(doc, &result, name)?;
    Ok(new_id)
}

/// Get AI-based selection (segmentation) for a layer.
pub async fn ai_select(
    pipeline: &AiPipeline,
    doc: &Document,
    layer_id: Uuid,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<Selection, RasaError> {
    let input = doc
        .get_pixels(layer_id)
        .ok_or(RasaError::LayerNotFound(layer_id))?
        .clone();

    let model = model.unwrap_or_else(crate::models::presets::sam_vit_h);
    let request = AiRequest::Segment { model };

    let result = pipeline.run(&request, &input, on_progress).await?;
    apply::mask_to_selection(&result)
}

/// Generative fill: inpaint a selection with a text prompt,
/// and blend the result back into the layer with feathering.
#[allow(clippy::too_many_arguments)]
pub async fn generative_fill(
    pipeline: &AiPipeline,
    doc: &mut Document,
    layer_id: Uuid,
    selection: &Selection,
    prompt: &str,
    feather: u32,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<(), RasaError> {
    let (region, bounds) = apply::extract_selection_region(doc, layer_id, selection)?;

    let model = model.unwrap_or_else(crate::models::presets::stable_diffusion_inpaint);
    let request = AiRequest::Inpaint {
        model,
        prompt: Some(prompt.to_string()),
        mask_region: bounds,
    };

    let result = pipeline.run(&request, &region, on_progress).await?;
    apply::blend_result_at(
        doc,
        layer_id,
        &result,
        bounds.x as u32,
        bounds.y as u32,
        feather,
    )?;
    Ok(())
}

/// Apply style transfer to a layer.
/// Creates a new layer with the stylized result.
pub async fn style_transfer(
    pipeline: &AiPipeline,
    doc: &mut Document,
    layer_id: Uuid,
    style: &str,
    strength: f32,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<Uuid, RasaError> {
    let input = doc
        .get_pixels(layer_id)
        .ok_or(RasaError::LayerNotFound(layer_id))?
        .clone();

    let model = model.unwrap_or_else(crate::models::presets::style_transfer_default);
    let request = AiRequest::StyleTransfer {
        model,
        style: style.to_string(),
        strength,
    };

    let result = pipeline.run(&request, &input, on_progress).await?;
    let name = format!("Style: {}", truncate(style, 20));
    let new_id = apply::apply_as_new_layer(doc, &result, name)?;
    Ok(new_id)
}

/// Apply AI color grading to a layer.
/// Creates a new layer with the color-graded result.
pub async fn color_grade_layer(
    pipeline: &AiPipeline,
    doc: &mut Document,
    layer_id: Uuid,
    preset: &str,
    intensity: f32,
    model: Option<ModelId>,
    on_progress: Option<ProgressCallback>,
) -> Result<Uuid, RasaError> {
    let input = doc
        .get_pixels(layer_id)
        .ok_or(RasaError::LayerNotFound(layer_id))?
        .clone();

    let model = model.unwrap_or_else(crate::models::presets::color_grading_default);
    let request = AiRequest::ColorGrade {
        model,
        preset: preset.to_string(),
        intensity,
    };

    let result = pipeline.run(&request, &input, on_progress).await?;
    let name = format!("Grade: {}", truncate(preset, 20));
    let new_id = apply::apply_as_new_layer(doc, &result, name)?;
    Ok(new_id)
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..s.floor_char_boundary(max)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long() {
        assert_eq!(truncate("hello world this is long", 10).len(), 10);
    }

    #[test]
    fn truncate_exact() {
        assert_eq!(truncate("hello", 5), "hello");
    }
}
