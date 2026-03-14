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
        // Hue/saturation, curves, levels — delegate to CPU backend
        // (these don't benefit enough from GPU to warrant shader implementations)
        Adjustment::HueSaturation { hue, saturation, lightness } => {
            for px in buf.pixels_mut() {
                if px.a <= 0.0 { continue; }
                let a = px.a;
                let (mut h, mut s, mut l) = px.to_hsl();
                h = (h + hue).rem_euclid(360.0);
                s = (s + saturation).clamp(0.0, 1.0);
                l = (l + lightness).clamp(0.0, 1.0);
                *px = rasa_core::color::Color::from_hsl(h, s, l, a);
            }
        }
        Adjustment::Curves { .. } | Adjustment::Levels { .. } => {
            // These require LUT/gamma operations that are CPU-only for now
            // No-op if backend can't handle — caller should use rasa_engine::filters directly
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
