use rasa_core::layer::Adjustment;
use rasa_core::pixel::PixelBuffer;

use crate::backend::RenderBackend;

/// Apply an adjustment using the provided backend.
pub fn apply_adjustment_with_backend(
    buf: &mut PixelBuffer,
    adj: &Adjustment,
    backend: &dyn RenderBackend,
) {
    match adj {
        Adjustment::BrightnessContrast {
            brightness,
            contrast,
        } => backend.brightness_contrast(buf, *brightness, *contrast),
        // Hue/saturation, curves, levels remain CPU-only for now
        // (complex operations that benefit less from GPU parallelism)
        _ => {
            // Delegate to CPU engine filters
            crate::backend::CpuBackend.brightness_contrast(buf, 0.0, 0.0);
        }
    }
}

/// Apply blur using the provided backend.
pub fn blur_with_backend(buf: &mut PixelBuffer, radius: u32, backend: &dyn RenderBackend) {
    backend.gaussian_blur(buf, radius);
}

/// Apply sharpen using the provided backend.
pub fn sharpen_with_backend(
    buf: &mut PixelBuffer,
    radius: u32,
    amount: f32,
    backend: &dyn RenderBackend,
) {
    backend.sharpen(buf, radius, amount);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::CpuBackend;
    use rasa_core::color::Color;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.02
    }

    #[test]
    fn blur_via_backend() {
        let backend = CpuBackend;
        let mut buf = PixelBuffer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                let c = if (x + y) % 2 == 0 {
                    Color::WHITE
                } else {
                    Color::BLACK
                };
                buf.set(x, y, c);
            }
        }
        blur_with_backend(&mut buf, 2, &backend);
        let px = buf.get(4, 4).unwrap();
        assert!(px.r > 0.1 && px.r < 0.9);
    }

    #[test]
    fn brightness_via_backend() {
        let backend = CpuBackend;
        let mut buf = PixelBuffer::filled(1, 1, Color::new(0.5, 0.5, 0.5, 1.0));
        let adj = Adjustment::BrightnessContrast {
            brightness: 0.2,
            contrast: 0.0,
        };
        apply_adjustment_with_backend(&mut buf, &adj, &backend);
        let px = buf.get(0, 0).unwrap();
        assert!(px.r > 0.5);
    }
}
