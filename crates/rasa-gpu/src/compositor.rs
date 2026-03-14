use rasa_core::Document;
use rasa_core::pixel::PixelBuffer;

use crate::backend::RenderBackend;

/// Composite all visible layers using the provided backend.
pub fn composite_with_backend(doc: &Document, backend: &dyn RenderBackend) -> PixelBuffer {
    let (w, h) = (doc.size.width, doc.size.height);
    let mut output = PixelBuffer::filled(w, h, rasa_core::color::Color::TRANSPARENT);

    for layer in &doc.layers {
        if !layer.visible || layer.opacity <= 0.0 {
            continue;
        }

        let Some(layer_buf) = doc.get_pixels(layer.id) else {
            continue;
        };

        backend.composite(&mut output, layer_buf, layer.blend_mode, layer.opacity);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::CpuBackend;
    use rasa_core::color::Color;
    use rasa_core::layer::Layer;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn composite_via_backend() {
        let backend = CpuBackend;
        let doc = Document::new("Test", 4, 4);
        let result = composite_with_backend(&doc, &backend);
        let px = result.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.a, 1.0));
    }

    #[test]
    fn composite_skips_hidden() {
        let backend = CpuBackend;
        let mut doc = Document::new("Test", 4, 4);
        let l = Layer::new_raster("Red", 4, 4);
        let lid = l.id;
        doc.add_layer(l);
        if let Some(buf) = doc.get_pixels_mut(lid) {
            for y in 0..4 {
                for x in 0..4 {
                    buf.set(x, y, Color::new(1.0, 0.0, 0.0, 1.0));
                }
            }
        }
        doc.set_layer_visibility(lid, false).unwrap();
        let result = composite_with_backend(&doc, &backend);
        let px = result.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.g, 1.0)); // still white
    }
}
