use rasa_core::color::Color;
use rasa_core::document::Document;
use rasa_core::error::RasaError;
use rasa_core::geometry::Rect;
use rasa_core::layer::Layer;
use rasa_core::pixel::PixelBuffer;
use rasa_core::selection::Selection;
use uuid::Uuid;

use crate::pipeline::AiResult;

/// Apply an AI image result to a document as a new layer.
pub fn apply_as_new_layer(
    doc: &mut Document,
    result: &AiResult,
    name: impl Into<String>,
) -> Result<Uuid, RasaError> {
    match result {
        AiResult::Image(buf) => {
            let layer = Layer::new_raster(name, buf.width, buf.height);
            let layer_id = layer.id;
            doc.add_layer(layer);
            // Copy pixel data
            if let Some(dest) = doc.get_pixels_mut(layer_id) {
                let (w, h) = buf.dimensions();
                for y in 0..h {
                    for x in 0..w {
                        if let Some(px) = buf.get(x, y) {
                            dest.set(x, y, px);
                        }
                    }
                }
            }
            Ok(layer_id)
        }
        AiResult::Mask { .. } => Err(RasaError::Other(
            "cannot apply mask result as a layer directly".into(),
        )),
    }
}

/// Apply an AI image result onto an existing layer, replacing its pixel data.
pub fn apply_to_layer(
    doc: &mut Document,
    layer_id: Uuid,
    result: &AiResult,
) -> Result<(), RasaError> {
    match result {
        AiResult::Image(src) => {
            let dest = doc
                .get_pixels_mut(layer_id)
                .ok_or(RasaError::LayerNotFound(layer_id))?;
            let (w, h) = (dest.width.min(src.width), dest.height.min(src.height));
            for y in 0..h {
                for x in 0..w {
                    if let Some(px) = src.get(x, y) {
                        dest.set(x, y, px);
                    }
                }
            }
            Ok(())
        }
        AiResult::Mask { .. } => Err(RasaError::Other(
            "cannot apply mask result to a layer directly".into(),
        )),
    }
}

/// Apply an AI image result only within a selection region on a layer.
/// Pixels outside the selection remain unchanged.
pub fn apply_within_selection(
    doc: &mut Document,
    layer_id: Uuid,
    result: &AiResult,
    selection: &Selection,
) -> Result<(), RasaError> {
    match result {
        AiResult::Image(src) => {
            let dest = doc
                .get_pixels_mut(layer_id)
                .ok_or(RasaError::LayerNotFound(layer_id))?;
            let (w, h) = (dest.width.min(src.width), dest.height.min(src.height));
            for y in 0..h {
                for x in 0..w {
                    let point = rasa_core::geometry::Point {
                        x: x as f64 + 0.5,
                        y: y as f64 + 0.5,
                    };
                    if selection.contains(point)
                        && let Some(px) = src.get(x, y)
                    {
                        dest.set(x, y, px);
                    }
                }
            }
            Ok(())
        }
        AiResult::Mask { .. } => Err(RasaError::Other(
            "cannot apply mask result within selection".into(),
        )),
    }
}

/// Convert a mask AI result into a Selection.
pub fn mask_to_selection(result: &AiResult) -> Result<Selection, RasaError> {
    match result {
        AiResult::Mask {
            width,
            height,
            data,
        } => Ok(Selection::Mask {
            width: *width,
            height: *height,
            data: data.clone(),
        }),
        AiResult::Image(_) => Err(RasaError::Other("expected mask result, got image".into())),
    }
}

/// Apply a segmentation mask to a layer — sets alpha to 0 where mask is 0
/// (effectively removing the background).
pub fn apply_mask_to_alpha(
    doc: &mut Document,
    layer_id: Uuid,
    result: &AiResult,
) -> Result<(), RasaError> {
    match result {
        AiResult::Mask {
            width,
            height,
            data,
        } => {
            let dest = doc
                .get_pixels_mut(layer_id)
                .ok_or(RasaError::LayerNotFound(layer_id))?;
            let w = dest.width.min(*width);
            let h = dest.height.min(*height);
            for y in 0..h {
                for x in 0..w {
                    let mask_idx = (y as usize) * (*width as usize) + (x as usize);
                    let mask_val = data[mask_idx];
                    let mut px = dest.get(x, y).unwrap();
                    px.a *= mask_val;
                    dest.set(x, y, px);
                }
            }
            Ok(())
        }
        AiResult::Image(_) => Err(RasaError::Other("expected mask result, got image".into())),
    }
}

/// Extract the pixels within a selection bounding box from a layer.
/// Used to prepare input for AI operations on selected regions.
pub fn extract_selection_region(
    doc: &Document,
    layer_id: Uuid,
    selection: &Selection,
) -> Result<(PixelBuffer, Rect), RasaError> {
    let src = doc
        .get_pixels(layer_id)
        .ok_or(RasaError::LayerNotFound(layer_id))?;

    let bounds = selection.bounds().unwrap_or(Rect {
        x: 0.0,
        y: 0.0,
        width: src.width as f64,
        height: src.height as f64,
    });

    let x0 = (bounds.x as u32).min(src.width);
    let y0 = (bounds.y as u32).min(src.height);
    let x1 = ((bounds.x + bounds.width) as u32).min(src.width);
    let y1 = ((bounds.y + bounds.height) as u32).min(src.height);
    let w = x1.saturating_sub(x0);
    let h = y1.saturating_sub(y0);

    let mut region = PixelBuffer::new(w, h);
    for dy in 0..h {
        for dx in 0..w {
            if let Some(px) = src.get(x0 + dx, y0 + dy) {
                region.set(dx, dy, px);
            }
        }
    }

    Ok((region, bounds))
}

/// Blend an AI result back into a layer at a specific offset,
/// using soft feathering at the edges for seamless integration.
pub fn blend_result_at(
    doc: &mut Document,
    layer_id: Uuid,
    result: &AiResult,
    offset_x: u32,
    offset_y: u32,
    feather: u32,
) -> Result<(), RasaError> {
    match result {
        AiResult::Image(src) => {
            let dest = doc
                .get_pixels_mut(layer_id)
                .ok_or(RasaError::LayerNotFound(layer_id))?;
            let (sw, sh) = src.dimensions();
            for sy in 0..sh {
                for sx in 0..sw {
                    let dx = offset_x + sx;
                    let dy = offset_y + sy;
                    if dx >= dest.width || dy >= dest.height {
                        continue;
                    }
                    let src_px = src.get(sx, sy).unwrap();
                    let blend_factor = if feather > 0 {
                        feather_factor(sx, sy, sw, sh, feather)
                    } else {
                        1.0
                    };
                    if blend_factor >= 1.0 {
                        dest.set(dx, dy, src_px);
                    } else if blend_factor > 0.0 {
                        let dst_px = dest.get(dx, dy).unwrap();
                        let blended = Color::new(
                            dst_px.r + (src_px.r - dst_px.r) * blend_factor,
                            dst_px.g + (src_px.g - dst_px.g) * blend_factor,
                            dst_px.b + (src_px.b - dst_px.b) * blend_factor,
                            dst_px.a + (src_px.a - dst_px.a) * blend_factor,
                        );
                        dest.set(dx, dy, blended);
                    }
                }
            }
            Ok(())
        }
        AiResult::Mask { .. } => Err(RasaError::Other("cannot blend mask result".into())),
    }
}

/// Calculate feather blend factor for edge pixels.
/// Returns 1.0 in the interior, fades to 0.0 at edges within `feather` pixels.
fn feather_factor(x: u32, y: u32, w: u32, h: u32, feather: u32) -> f32 {
    let dist_left = x as f32;
    let dist_right = (w - 1 - x) as f32;
    let dist_top = y as f32;
    let dist_bottom = (h - 1 - y) as f32;
    let min_dist = dist_left.min(dist_right).min(dist_top).min(dist_bottom);
    (min_dist / feather as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::color::Color;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.05
    }

    #[test]
    fn apply_as_new_layer_adds_layer() {
        let mut doc = Document::new("Test", 10, 10);
        let result_buf = PixelBuffer::filled(10, 10, Color::new(1.0, 0.0, 0.0, 1.0));
        let result = AiResult::Image(result_buf);
        let id = apply_as_new_layer(&mut doc, &result, "AI Result").unwrap();
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.find_layer(id).unwrap().name, "AI Result");
        let px = doc.get_pixels(id).unwrap().get(5, 5).unwrap();
        assert!(approx_eq(px.r, 1.0));
    }

    #[test]
    fn apply_mask_result_as_layer_fails() {
        let mut doc = Document::new("Test", 4, 4);
        let result = AiResult::Mask {
            width: 4,
            height: 4,
            data: vec![1.0; 16],
        };
        assert!(apply_as_new_layer(&mut doc, &result, "Mask").is_err());
    }

    #[test]
    fn apply_to_existing_layer() {
        let mut doc = Document::new("Test", 4, 4);
        let bg_id = doc.layers[0].id;
        let result_buf = PixelBuffer::filled(4, 4, Color::new(0.0, 0.0, 1.0, 1.0));
        let result = AiResult::Image(result_buf);
        apply_to_layer(&mut doc, bg_id, &result).unwrap();
        let px = doc.get_pixels(bg_id).unwrap().get(0, 0).unwrap();
        assert!(approx_eq(px.b, 1.0));
        assert!(approx_eq(px.r, 0.0));
    }

    #[test]
    fn apply_within_selection_only_modifies_selected() {
        let mut doc = Document::new("Test", 10, 10);
        let bg_id = doc.layers[0].id;
        let result_buf = PixelBuffer::filled(10, 10, Color::new(1.0, 0.0, 0.0, 1.0));
        let result = AiResult::Image(result_buf);
        let sel = Selection::Rect(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        });
        apply_within_selection(&mut doc, bg_id, &result, &sel).unwrap();
        // Inside selection: red
        let inside = doc.get_pixels(bg_id).unwrap().get(4, 4).unwrap();
        assert!(approx_eq(inside.r, 1.0));
        assert!(approx_eq(inside.g, 0.0));
        // Outside selection: still white
        let outside = doc.get_pixels(bg_id).unwrap().get(0, 0).unwrap();
        assert!(approx_eq(outside.r, 1.0));
        assert!(approx_eq(outside.g, 1.0));
    }

    #[test]
    fn mask_to_selection_converts() {
        let result = AiResult::Mask {
            width: 4,
            height: 4,
            data: vec![
                1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0,
            ],
        };
        let sel = mask_to_selection(&result).unwrap();
        match sel {
            Selection::Mask {
                width,
                height,
                data,
            } => {
                assert_eq!(width, 4);
                assert_eq!(height, 4);
                assert_eq!(data.len(), 16);
            }
            _ => panic!("expected mask selection"),
        }
    }

    #[test]
    fn mask_to_selection_rejects_image() {
        let result = AiResult::Image(PixelBuffer::new(1, 1));
        assert!(mask_to_selection(&result).is_err());
    }

    #[test]
    fn apply_mask_to_alpha_zeroes_background() {
        let mut doc = Document::new("Test", 4, 4);
        let bg_id = doc.layers[0].id;
        // Mask: top-left pixel is foreground (1.0), rest is background (0.0)
        let result = AiResult::Mask {
            width: 4,
            height: 4,
            data: {
                let mut d = vec![0.0; 16];
                d[0] = 1.0;
                d
            },
        };
        apply_mask_to_alpha(&mut doc, bg_id, &result).unwrap();
        // Top-left should retain alpha
        let fg = doc.get_pixels(bg_id).unwrap().get(0, 0).unwrap();
        assert!(approx_eq(fg.a, 1.0));
        // Other pixels should be transparent
        let bg = doc.get_pixels(bg_id).unwrap().get(1, 0).unwrap();
        assert!(approx_eq(bg.a, 0.0));
    }

    #[test]
    fn extract_selection_region_crops() {
        let doc = Document::new("Test", 10, 10);
        let bg_id = doc.layers[0].id;
        let sel = Selection::Rect(Rect {
            x: 2.0,
            y: 3.0,
            width: 4.0,
            height: 5.0,
        });
        let (region, bounds) = extract_selection_region(&doc, bg_id, &sel).unwrap();
        assert_eq!(region.dimensions(), (4, 5));
        assert!(approx_eq(bounds.x as f32, 2.0));
        assert!(approx_eq(bounds.y as f32, 3.0));
    }

    #[test]
    fn extract_no_selection_gives_full_buffer() {
        let doc = Document::new("Test", 8, 8);
        let bg_id = doc.layers[0].id;
        let sel = Selection::None;
        let (region, _) = extract_selection_region(&doc, bg_id, &sel).unwrap();
        assert_eq!(region.dimensions(), (8, 8));
    }

    #[test]
    fn blend_result_at_offset() {
        let mut doc = Document::new("Test", 10, 10);
        let bg_id = doc.layers[0].id;
        let src = PixelBuffer::filled(4, 4, Color::new(1.0, 0.0, 0.0, 1.0));
        let result = AiResult::Image(src);
        blend_result_at(&mut doc, bg_id, &result, 3, 3, 0).unwrap();
        // At (5,5) should be red (inside the result)
        let px = doc.get_pixels(bg_id).unwrap().get(5, 5).unwrap();
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.g, 0.0));
        // At (0,0) should still be white
        let px2 = doc.get_pixels(bg_id).unwrap().get(0, 0).unwrap();
        assert!(approx_eq(px2.r, 1.0));
        assert!(approx_eq(px2.g, 1.0));
    }

    #[test]
    fn blend_with_feather() {
        let mut doc = Document::new("Test", 20, 20);
        let bg_id = doc.layers[0].id;
        let src = PixelBuffer::filled(10, 10, Color::new(0.0, 0.0, 0.0, 1.0));
        let result = AiResult::Image(src);
        blend_result_at(&mut doc, bg_id, &result, 5, 5, 3).unwrap();
        // Edge pixel should be partially blended (not fully black)
        let edge = doc.get_pixels(bg_id).unwrap().get(5, 5).unwrap();
        assert!(edge.r > 0.0); // feathered, not fully black
        // Center should be fully black
        let center = doc.get_pixels(bg_id).unwrap().get(10, 10).unwrap();
        assert!(approx_eq(center.r, 0.0));
    }

    #[test]
    fn feather_factor_interior() {
        assert!(approx_eq(feather_factor(5, 5, 10, 10, 2), 1.0));
    }

    #[test]
    fn feather_factor_edge() {
        assert!(approx_eq(feather_factor(0, 5, 10, 10, 2), 0.0));
        assert!(approx_eq(feather_factor(1, 5, 10, 10, 2), 0.5));
    }
}
