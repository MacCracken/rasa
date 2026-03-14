use rasa_core::color::Color;
use rasa_core::layer::Adjustment;
use rasa_core::pixel::PixelBuffer;

/// Apply an adjustment to a pixel buffer in-place.
pub fn apply_adjustment(buf: &mut PixelBuffer, adj: &Adjustment) {
    match adj {
        Adjustment::BrightnessContrast {
            brightness,
            contrast,
        } => apply_brightness_contrast(buf, *brightness, *contrast),
        Adjustment::HueSaturation {
            hue,
            saturation,
            lightness,
        } => apply_hue_saturation(buf, *hue, *saturation, *lightness),
        Adjustment::Curves { points } => apply_curves(buf, points),
        Adjustment::Levels {
            black,
            white,
            gamma,
        } => apply_levels(buf, *black, *white, *gamma),
    }
}

/// Brightness: -1.0 to 1.0, Contrast: -1.0 to 1.0
fn apply_brightness_contrast(buf: &mut PixelBuffer, brightness: f32, contrast: f32) {
    let factor = (1.0 + contrast) / (1.0 - contrast.min(0.9999));
    for px in buf.pixels_mut() {
        let a = px.a;
        px.r = ((px.r + brightness) * factor + 0.5 * (1.0 - factor)).clamp(0.0, 1.0);
        px.g = ((px.g + brightness) * factor + 0.5 * (1.0 - factor)).clamp(0.0, 1.0);
        px.b = ((px.b + brightness) * factor + 0.5 * (1.0 - factor)).clamp(0.0, 1.0);
        px.a = a;
    }
}

/// Hue shift: -180 to 180 degrees, Saturation: -1.0 to 1.0, Lightness: -1.0 to 1.0
fn apply_hue_saturation(buf: &mut PixelBuffer, hue: f32, saturation: f32, lightness: f32) {
    for px in buf.pixels_mut() {
        if px.a <= 0.0 {
            continue;
        }
        let a = px.a;
        let (mut h, mut s, mut l) = px.to_hsl();
        h = (h + hue).rem_euclid(360.0);
        s = (s + saturation).clamp(0.0, 1.0);
        l = (l + lightness).clamp(0.0, 1.0);
        *px = Color::from_hsl(h, s, l, a);
    }
}

/// Apply a curves adjustment using piecewise linear interpolation.
/// Points are (input, output) pairs in 0.0-1.0, sorted by input.
fn apply_curves(buf: &mut PixelBuffer, points: &[(f32, f32)]) {
    if points.len() < 2 {
        return;
    }

    // Build lookup table (256 entries)
    let lut: Vec<f32> = (0..256)
        .map(|i| {
            let x = i as f32 / 255.0;
            interpolate_curve(points, x)
        })
        .collect();

    for px in buf.pixels_mut() {
        let a = px.a;
        px.r = lut[(px.r.clamp(0.0, 1.0) * 255.0) as usize];
        px.g = lut[(px.g.clamp(0.0, 1.0) * 255.0) as usize];
        px.b = lut[(px.b.clamp(0.0, 1.0) * 255.0) as usize];
        px.a = a;
    }
}

fn interpolate_curve(points: &[(f32, f32)], x: f32) -> f32 {
    if x <= points[0].0 {
        return points[0].1;
    }
    if x >= points[points.len() - 1].0 {
        return points[points.len() - 1].1;
    }
    for window in points.windows(2) {
        let (x0, y0) = window[0];
        let (x1, y1) = window[1];
        if x >= x0 && x <= x1 {
            let t = if (x1 - x0).abs() < 1e-7 {
                0.0
            } else {
                (x - x0) / (x1 - x0)
            };
            return y0 + t * (y1 - y0);
        }
    }
    points[points.len() - 1].1
}

/// Levels: black point (0.0-1.0), white point (0.0-1.0), gamma (0.1-10.0)
fn apply_levels(buf: &mut PixelBuffer, black: f32, white: f32, gamma: f32) {
    let range = (white - black).max(1e-6);
    let inv_gamma = 1.0 / gamma.clamp(0.1, 10.0);

    for px in buf.pixels_mut() {
        let a = px.a;
        px.r = level_channel(px.r, black, range, inv_gamma);
        px.g = level_channel(px.g, black, range, inv_gamma);
        px.b = level_channel(px.b, black, range, inv_gamma);
        px.a = a;
    }
}

fn level_channel(v: f32, black: f32, range: f32, inv_gamma: f32) -> f32 {
    let normalized = ((v - black) / range).clamp(0.0, 1.0);
    normalized.powf(inv_gamma)
}

/// Gaussian blur (separable, CPU).
pub fn gaussian_blur(buf: &mut PixelBuffer, radius: u32) {
    if radius == 0 {
        return;
    }

    let kernel = build_gaussian_kernel(radius);
    let (w, h) = buf.dimensions();

    // Horizontal pass
    let mut temp = PixelBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0_f32;
            let mut g = 0.0_f32;
            let mut b = 0.0_f32;
            let mut a = 0.0_f32;
            for (i, &weight) in kernel.iter().enumerate() {
                let sx = (x as i32 + i as i32 - radius as i32).clamp(0, w as i32 - 1) as u32;
                let px = buf.get(sx, y).unwrap();
                r += px.r * weight;
                g += px.g * weight;
                b += px.b * weight;
                a += px.a * weight;
            }
            temp.set(x, y, Color::new(r, g, b, a));
        }
    }

    // Vertical pass
    for y in 0..h {
        for x in 0..w {
            let mut r = 0.0_f32;
            let mut g = 0.0_f32;
            let mut b = 0.0_f32;
            let mut a = 0.0_f32;
            for (i, &weight) in kernel.iter().enumerate() {
                let sy = (y as i32 + i as i32 - radius as i32).clamp(0, h as i32 - 1) as u32;
                let px = temp.get(x, sy).unwrap();
                r += px.r * weight;
                g += px.g * weight;
                b += px.b * weight;
                a += px.a * weight;
            }
            buf.set(x, y, Color::new(r, g, b, a));
        }
    }
}

fn build_gaussian_kernel(radius: u32) -> Vec<f32> {
    let size = (radius * 2 + 1) as usize;
    let sigma = radius as f32 / 3.0;
    let mut kernel = Vec::with_capacity(size);
    let mut sum = 0.0_f32;

    for i in 0..size {
        let x = i as f32 - radius as f32;
        let v = (-x * x / (2.0 * sigma * sigma)).exp();
        kernel.push(v);
        sum += v;
    }

    for v in &mut kernel {
        *v /= sum;
    }
    kernel
}

/// Sharpen using unsharp mask: sharpen = original + amount * (original - blurred)
pub fn sharpen(buf: &mut PixelBuffer, radius: u32, amount: f32) {
    if radius == 0 || amount.abs() < 1e-6 {
        return;
    }

    let mut blurred = PixelBuffer::new(buf.width, buf.height);
    // Copy pixels
    let (w, h) = buf.dimensions();
    for y in 0..h {
        for x in 0..w {
            blurred.set(x, y, buf.get(x, y).unwrap());
        }
    }
    gaussian_blur(&mut blurred, radius);

    for y in 0..h {
        for x in 0..w {
            let orig = buf.get(x, y).unwrap();
            let blur = blurred.get(x, y).unwrap();
            buf.set(
                x,
                y,
                Color::new(
                    (orig.r + amount * (orig.r - blur.r)).clamp(0.0, 1.0),
                    (orig.g + amount * (orig.g - blur.g)).clamp(0.0, 1.0),
                    (orig.b + amount * (orig.b - blur.b)).clamp(0.0, 1.0),
                    orig.a,
                ),
            );
        }
    }
}

/// Invert all pixel colors (preserves alpha).
pub fn invert(buf: &mut PixelBuffer) {
    for px in buf.pixels_mut() {
        px.r = 1.0 - px.r;
        px.g = 1.0 - px.g;
        px.b = 1.0 - px.b;
    }
}

/// Convert to grayscale using luminance weights (preserves alpha).
pub fn grayscale(buf: &mut PixelBuffer) {
    for px in buf.pixels_mut() {
        let lum = 0.2126 * px.r + 0.7152 * px.g + 0.0722 * px.b;
        px.r = lum;
        px.g = lum;
        px.b = lum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.02
    }

    // ── Brightness/Contrast ──

    #[test]
    fn brightness_increases() {
        let mut buf = PixelBuffer::filled(2, 2, Color::new(0.5, 0.5, 0.5, 1.0));
        apply_brightness_contrast(&mut buf, 0.2, 0.0);
        let px = buf.get(0, 0).unwrap();
        assert!(px.r > 0.5);
    }

    #[test]
    fn brightness_zero_contrast_zero_noop() {
        let mut buf = PixelBuffer::filled(2, 2, Color::new(0.5, 0.5, 0.5, 1.0));
        apply_brightness_contrast(&mut buf, 0.0, 0.0);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.5));
    }

    #[test]
    fn contrast_increases_spread() {
        let mut buf = PixelBuffer::new(2, 1);
        buf.set(0, 0, Color::new(0.3, 0.3, 0.3, 1.0));
        buf.set(1, 0, Color::new(0.7, 0.7, 0.7, 1.0));
        apply_brightness_contrast(&mut buf, 0.0, 0.5);
        let dark = buf.get(0, 0).unwrap();
        let light = buf.get(1, 0).unwrap();
        // Contrast should push dark darker and light lighter
        assert!(dark.r < 0.3);
        assert!(light.r > 0.7);
    }

    // ── Hue/Saturation ──

    #[test]
    fn desaturate_to_gray() {
        let mut buf = PixelBuffer::filled(2, 2, Color::new(1.0, 0.0, 0.0, 1.0));
        apply_hue_saturation(&mut buf, 0.0, -1.0, 0.0);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, px.g));
        assert!(approx_eq(px.g, px.b));
    }

    #[test]
    fn hue_shift_red_to_green() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(1.0, 0.0, 0.0, 1.0));
        apply_hue_saturation(&mut buf, 120.0, 0.0, 0.0);
        let px = buf.get(0, 0).unwrap();
        // Red shifted 120 degrees should be green
        assert!(px.g > px.r);
        assert!(px.g > px.b);
    }

    #[test]
    fn transparent_pixels_skipped() {
        let mut buf = PixelBuffer::new(1, 1); // transparent
        apply_hue_saturation(&mut buf, 90.0, 0.5, 0.3);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.a, 0.0));
    }

    // ── Curves ──

    #[test]
    fn curves_identity() {
        let mut buf = PixelBuffer::filled(2, 2, Color::new(0.5, 0.5, 0.5, 1.0));
        apply_curves(&mut buf, &[(0.0, 0.0), (1.0, 1.0)]);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.5));
    }

    #[test]
    fn curves_invert() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(0.75, 0.25, 0.5, 1.0));
        apply_curves(&mut buf, &[(0.0, 1.0), (1.0, 0.0)]);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.25));
        assert!(approx_eq(px.g, 0.75));
        assert!(approx_eq(px.b, 0.5));
    }

    #[test]
    fn curves_too_few_points_noop() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(0.5, 0.5, 0.5, 1.0));
        apply_curves(&mut buf, &[(0.5, 0.5)]);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.5));
    }

    // ── Levels ──

    #[test]
    fn levels_identity() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(0.5, 0.5, 0.5, 1.0));
        apply_levels(&mut buf, 0.0, 1.0, 1.0);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.5));
    }

    #[test]
    fn levels_black_crush() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(0.3, 0.3, 0.3, 1.0));
        // Set black point to 0.4 — anything below should map to 0
        apply_levels(&mut buf, 0.4, 1.0, 1.0);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.0));
    }

    #[test]
    fn levels_gamma_brightens() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(0.5, 0.5, 0.5, 1.0));
        // gamma > 1 means inv_gamma < 1, so 0.5^(inv) > 0.5 — brightens midtones
        apply_levels(&mut buf, 0.0, 1.0, 2.0);
        let px = buf.get(0, 0).unwrap();
        assert!(px.r > 0.5);
    }

    // ── Blur / Sharpen ──

    #[test]
    fn blur_reduces_contrast() {
        let mut buf = PixelBuffer::new(8, 8);
        // checkerboard pattern
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
        gaussian_blur(&mut buf, 2);
        // Center pixel should no longer be pure black or white
        let px = buf.get(4, 4).unwrap();
        assert!(px.r > 0.1 && px.r < 0.9);
    }

    #[test]
    fn blur_radius_zero_noop() {
        let mut buf = PixelBuffer::filled(4, 4, Color::new(0.5, 0.5, 0.5, 1.0));
        gaussian_blur(&mut buf, 0);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.5));
    }

    #[test]
    fn sharpen_zero_radius_noop() {
        let mut buf = PixelBuffer::filled(4, 4, Color::new(0.5, 0.5, 0.5, 1.0));
        sharpen(&mut buf, 0, 1.0);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.5));
    }

    // ── Invert / Grayscale ──

    #[test]
    fn invert_white_to_black() {
        let mut buf = PixelBuffer::filled(1, 1, Color::WHITE);
        invert(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.0));
        assert!(approx_eq(px.g, 0.0));
        assert!(approx_eq(px.b, 0.0));
        assert!(approx_eq(px.a, 1.0)); // alpha preserved
    }

    #[test]
    fn invert_roundtrip() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(0.3, 0.6, 0.9, 1.0));
        invert(&mut buf);
        invert(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.3));
        assert!(approx_eq(px.g, 0.6));
        assert!(approx_eq(px.b, 0.9));
    }

    #[test]
    fn grayscale_preserves_alpha() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(1.0, 0.0, 0.0, 0.5));
        grayscale(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, px.g));
        assert!(approx_eq(px.g, px.b));
        assert!(approx_eq(px.a, 0.5));
    }

    #[test]
    fn grayscale_luminance_weights() {
        let mut buf = PixelBuffer::filled(1, 1, Color::new(1.0, 1.0, 1.0, 1.0));
        grayscale(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 1.0)); // white stays white
    }

    // ── Adjustment dispatch ──

    #[test]
    fn apply_adjustment_dispatches() {
        let mut buf = PixelBuffer::filled(1, 1, Color::WHITE);
        let adj = Adjustment::BrightnessContrast {
            brightness: 0.0,
            contrast: 0.0,
        };
        apply_adjustment(&mut buf, &adj);
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 1.0));
    }
}
