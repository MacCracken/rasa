use rasa_core::Document;
use rasa_core::blend::blend;
use rasa_core::color::{BlendMode, Color};
use rasa_core::layer::{Layer, LayerKind};
use rasa_core::pixel::PixelBuffer;

/// Flatten all visible layers in a document into a single pixel buffer (CPU path).
pub fn composite(doc: &Document) -> PixelBuffer {
    let (w, h) = (doc.size.width, doc.size.height);
    let mut output = PixelBuffer::filled(w, h, Color::TRANSPARENT);

    for layer in &doc.layers {
        composite_layer_tree(&mut output, layer, doc, w, h);
    }

    output
}

/// Recursively composite a layer (handles groups with nested children).
fn composite_layer_tree(
    dst: &mut PixelBuffer,
    layer: &Layer,
    doc: &Document,
    w: u32,
    h: u32,
) {
    if !layer.visible || layer.opacity <= 0.0 {
        return;
    }

    match &layer.kind {
        LayerKind::Group { children } => {
            // Composite children into a temporary buffer, then blend group onto dst
            let mut group_buf = PixelBuffer::filled(w, h, Color::TRANSPARENT);
            for child in children {
                composite_layer_tree(&mut group_buf, child, doc, w, h);
            }
            composite_layer(dst, &group_buf, layer.blend_mode, layer.opacity);
        }
        _ => {
            let Some(layer_buf) = doc.get_pixels(layer.id) else {
                return;
            };
            composite_layer(dst, layer_buf, layer.blend_mode, layer.opacity);
        }
    }
}

/// Composite a single layer buffer onto a destination buffer.
pub fn composite_layer(
    dst: &mut PixelBuffer,
    src: &PixelBuffer,
    mode: BlendMode,
    opacity: f32,
) {
    let w = dst.width.min(src.width);
    let h = dst.height.min(src.height);

    for y in 0..h {
        for x in 0..w {
            let base = dst.get(x, y).unwrap();
            let top = src.get(x, y).unwrap();
            let result = blend(base, top, mode, opacity);
            dst.set(x, y, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::color::{BlendMode, Color};
    use rasa_core::layer::Layer;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn composite_single_opaque_layer() {
        let doc = Document::new("Test", 4, 4);
        let result = composite(&doc);
        // Background is white
        let px = result.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.g, 1.0));
        assert!(approx_eq(px.b, 1.0));
        assert!(approx_eq(px.a, 1.0));
    }

    #[test]
    fn composite_hidden_layer_ignored() {
        let mut doc = Document::new("Test", 4, 4);
        let l = Layer::new_raster("Red", 4, 4);
        let lid = l.id;
        doc.add_layer(l);
        // Fill red layer
        if let Some(buf) = doc.get_pixels_mut(lid) {
            for y in 0..4 {
                for x in 0..4 {
                    buf.set(
                        x,
                        y,
                        Color {
                            r: 1.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        },
                    );
                }
            }
        }
        doc.set_layer_visibility(lid, false).unwrap();
        let result = composite(&doc);
        // Should still be white (red layer hidden)
        let px = result.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.g, 1.0));
    }

    #[test]
    fn composite_half_opacity_layer() {
        let mut doc = Document::new("Test", 2, 2);
        let l = Layer::new_raster("Red", 2, 2);
        let lid = l.id;
        doc.add_layer(l);
        // Fill red
        if let Some(buf) = doc.get_pixels_mut(lid) {
            for y in 0..2 {
                for x in 0..2 {
                    buf.set(
                        x,
                        y,
                        Color {
                            r: 1.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        },
                    );
                }
            }
        }
        doc.set_layer_opacity(lid, 0.5).unwrap();
        let result = composite(&doc);
        let px = result.get(0, 0).unwrap();
        // White bg + 50% red = (0.5 red, 0.5 green, 0.5 blue) approximately
        assert!(approx_eq(px.r, 1.0)); // 1.0*0.5 + 1.0*1.0*0.5 / 1.0 = 1.0... wait
        // Actually: blend(white, red, Normal, 0.5)
        // top_a = 1.0 * 0.5 = 0.5
        // out_a = 0.5 + 1.0 * 0.5 = 1.0
        // out_r = (1.0 * 0.5 + 1.0 * 1.0 * 0.5) / 1.0 = 1.0
        // out_g = (0.0 * 0.5 + 1.0 * 1.0 * 0.5) / 1.0 = 0.5
        assert!(approx_eq(px.g, 0.5));
        assert!(approx_eq(px.b, 0.5));
    }

    #[test]
    fn composite_multiply_blend() {
        let mut doc = Document::new("Test", 2, 2);
        let l = Layer::new_raster("Gray", 2, 2);
        let lid = l.id;
        doc.add_layer(l);
        if let Some(buf) = doc.get_pixels_mut(lid) {
            for y in 0..2 {
                for x in 0..2 {
                    buf.set(
                        x,
                        y,
                        Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 1.0,
                        },
                    );
                }
            }
        }
        doc.set_layer_blend_mode(lid, BlendMode::Multiply).unwrap();
        let result = composite(&doc);
        let px = result.get(0, 0).unwrap();
        // white * gray = gray (multiply: 1.0 * 0.5 = 0.5)
        assert!(approx_eq(px.r, 0.5));
        assert!(approx_eq(px.g, 0.5));
        assert!(approx_eq(px.b, 0.5));
    }

    #[test]
    fn composite_group_layer() {
        let mut doc = Document::new("Test", 2, 2);
        let l1 = Layer::new_raster("Red", 2, 2);
        let l1_id = l1.id;
        let l2 = Layer::new_raster("Blue", 2, 2);
        let l2_id = l2.id;
        doc.add_layer(l1);
        doc.add_layer(l2);
        // Fill red
        if let Some(buf) = doc.get_pixels_mut(l1_id) {
            for y in 0..2 {
                for x in 0..2 {
                    buf.set(x, y, Color::new(1.0, 0.0, 0.0, 1.0));
                }
            }
        }
        // Fill blue
        if let Some(buf) = doc.get_pixels_mut(l2_id) {
            for y in 0..2 {
                for x in 0..2 {
                    buf.set(x, y, Color::new(0.0, 0.0, 1.0, 1.0));
                }
            }
        }
        // Group the two layers
        doc.group_layers(&[l1_id, l2_id]).unwrap();
        assert_eq!(doc.layers.len(), 2); // Background + Group

        let result = composite(&doc);
        // Blue is on top of red in the group, both opaque, so result should be blue
        let px = result.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.0));
        assert!(approx_eq(px.b, 1.0));
    }

    #[test]
    fn composite_hidden_group_ignored() {
        let mut doc = Document::new("Test", 2, 2);
        let l = Layer::new_raster("Red", 2, 2);
        let lid = l.id;
        doc.add_layer(l);
        if let Some(buf) = doc.get_pixels_mut(lid) {
            for y in 0..2 {
                for x in 0..2 {
                    buf.set(x, y, Color::new(1.0, 0.0, 0.0, 1.0));
                }
            }
        }
        let group_id = doc.group_layers(&[lid]).unwrap();
        doc.set_layer_visibility(group_id, false).unwrap();
        let result = composite(&doc);
        // Group is hidden, so only white background
        let px = result.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.g, 1.0));
        assert!(approx_eq(px.b, 1.0));
    }

    #[test]
    fn composite_layer_direct() {
        let mut dst = PixelBuffer::filled(2, 2, Color::WHITE);
        let mut src = PixelBuffer::new(2, 2);
        src.set(
            0,
            0,
            Color {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        );
        composite_layer(&mut dst, &src, BlendMode::Normal, 1.0);
        // (0,0) should be blue, (1,0) should still be white (transparent src)
        let px00 = dst.get(0, 0).unwrap();
        assert!(approx_eq(px00.b, 1.0));
        assert!(approx_eq(px00.r, 0.0));
        let px10 = dst.get(1, 0).unwrap();
        assert!(approx_eq(px10.r, 1.0));
        assert!(approx_eq(px10.g, 1.0));
    }
}
