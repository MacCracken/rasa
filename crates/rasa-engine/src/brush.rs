use rasa_core::blend;
use rasa_core::color::{BlendMode, Color};
use rasa_core::geometry::Point;
use rasa_core::pixel::PixelBuffer;

/// Brush tip shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrushTip {
    Round,
    Square,
}

/// Brush configuration.
#[derive(Debug, Clone)]
pub struct BrushSettings {
    pub size: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub spacing: f32,
    pub color: Color,
    pub tip: BrushTip,
    pub blend_mode: BlendMode,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 10.0,
            hardness: 0.8,
            opacity: 1.0,
            spacing: 0.25,
            color: Color::BLACK,
            tip: BrushTip::Round,
            blend_mode: BlendMode::Normal,
        }
    }
}

/// A single point in a brush stroke with optional pressure.
#[derive(Debug, Clone, Copy)]
pub struct StrokePoint {
    pub position: Point,
    pub pressure: f32,
}

/// Paint a single brush dab onto a pixel buffer.
pub fn paint_dab(buf: &mut PixelBuffer, center: Point, settings: &BrushSettings, pressure: f32) {
    let radius = settings.size * pressure / 2.0;
    if radius < 0.5 {
        return;
    }

    let x0 = ((center.x - radius as f64).floor() as i32).max(0) as u32;
    let y0 = ((center.y - radius as f64).floor() as i32).max(0) as u32;
    let x1 = ((center.x + radius as f64).ceil() as i32).min(buf.width as i32) as u32;
    let y1 = ((center.y + radius as f64).ceil() as i32).min(buf.height as i32) as u32;

    for y in y0..y1 {
        for x in x0..x1 {
            let alpha = dab_alpha(
                x as f64 + 0.5,
                y as f64 + 0.5,
                center,
                radius,
                settings.hardness,
                settings.tip,
            );
            if alpha < 1e-5 {
                continue;
            }

            let brush_color = Color::new(
                settings.color.r,
                settings.color.g,
                settings.color.b,
                alpha * settings.opacity * pressure,
            );
            let base = buf.get(x, y).unwrap();
            let result = blend::blend(base, brush_color, settings.blend_mode, 1.0);
            buf.set(x, y, result);
        }
    }
}

/// Paint a stroke (series of points) onto a pixel buffer.
pub fn paint_stroke(buf: &mut PixelBuffer, points: &[StrokePoint], settings: &BrushSettings) {
    if points.is_empty() {
        return;
    }

    if points.len() == 1 {
        paint_dab(buf, points[0].position, settings, points[0].pressure);
        return;
    }

    let spacing_px = (settings.size * settings.spacing).max(1.0);

    for window in points.windows(2) {
        let from = window[0];
        let to = window[1];
        let dx = to.position.x - from.position.x;
        let dy = to.position.y - from.position.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.1 {
            paint_dab(buf, from.position, settings, from.pressure);
            continue;
        }

        let steps = (dist / spacing_px as f64).ceil() as u32;
        for i in 0..=steps {
            let t = i as f64 / steps.max(1) as f64;
            let x = from.position.x + dx * t;
            let y = from.position.y + dy * t;
            let pressure = from.pressure + (to.pressure - from.pressure) * t as f32;
            paint_dab(buf, Point { x, y }, settings, pressure);
        }
    }
}

/// Erase by painting with transparency.
pub fn erase_dab(buf: &mut PixelBuffer, center: Point, settings: &BrushSettings, pressure: f32) {
    let radius = settings.size * pressure / 2.0;
    if radius < 0.5 {
        return;
    }

    let x0 = ((center.x - radius as f64).floor() as i32).max(0) as u32;
    let y0 = ((center.y - radius as f64).floor() as i32).max(0) as u32;
    let x1 = ((center.x + radius as f64).ceil() as i32).min(buf.width as i32) as u32;
    let y1 = ((center.y + radius as f64).ceil() as i32).min(buf.height as i32) as u32;

    for y in y0..y1 {
        for x in x0..x1 {
            let alpha = dab_alpha(
                x as f64 + 0.5,
                y as f64 + 0.5,
                center,
                radius,
                settings.hardness,
                settings.tip,
            );
            if alpha < 1e-5 {
                continue;
            }

            let erase_amount = alpha * settings.opacity * pressure;
            let mut px = buf.get(x, y).unwrap();
            px.a = (px.a - erase_amount).max(0.0);
            buf.set(x, y, px);
        }
    }
}

/// Erase along a stroke.
pub fn erase_stroke(buf: &mut PixelBuffer, points: &[StrokePoint], settings: &BrushSettings) {
    if points.is_empty() {
        return;
    }

    if points.len() == 1 {
        erase_dab(buf, points[0].position, settings, points[0].pressure);
        return;
    }

    let spacing_px = (settings.size * settings.spacing).max(1.0);

    for window in points.windows(2) {
        let from = window[0];
        let to = window[1];
        let dx = to.position.x - from.position.x;
        let dy = to.position.y - from.position.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.1 {
            erase_dab(buf, from.position, settings, from.pressure);
            continue;
        }

        let steps = (dist / spacing_px as f64).ceil() as u32;
        for i in 0..=steps {
            let t = i as f64 / steps.max(1) as f64;
            let x = from.position.x + dx * t;
            let y = from.position.y + dy * t;
            let pressure = from.pressure + (to.pressure - from.pressure) * t as f32;
            erase_dab(buf, Point { x, y }, settings, pressure);
        }
    }
}

fn dab_alpha(px: f64, py: f64, center: Point, radius: f32, hardness: f32, tip: BrushTip) -> f32 {
    match tip {
        BrushTip::Round => {
            let dx = px - center.x;
            let dy = py - center.y;
            let dist = (dx * dx + dy * dy).sqrt() as f32;
            if dist > radius {
                0.0
            } else {
                let t = dist / radius;
                let soft_start = hardness.min(0.999);
                if t <= soft_start {
                    1.0
                } else {
                    let fade = (t - soft_start) / (1.0 - soft_start);
                    (1.0 - fade).max(0.0)
                }
            }
        }
        BrushTip::Square => {
            let dx = (px - center.x).abs() as f32;
            let dy = (py - center.y).abs() as f32;
            if dx <= radius && dy <= radius {
                1.0
            } else {
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_single_dab_modifies_center() {
        let mut buf = PixelBuffer::new(20, 20);
        let settings = BrushSettings {
            size: 6.0,
            color: Color::new(1.0, 0.0, 0.0, 1.0),
            ..Default::default()
        };
        paint_dab(&mut buf, Point { x: 10.0, y: 10.0 }, &settings, 1.0);
        let center = buf.get(10, 10).unwrap();
        assert!(center.r > 0.5);
        assert!(center.a > 0.5);
    }

    #[test]
    fn paint_dab_does_not_affect_distant_pixels() {
        let mut buf = PixelBuffer::new(20, 20);
        let settings = BrushSettings {
            size: 4.0,
            color: Color::new(1.0, 0.0, 0.0, 1.0),
            ..Default::default()
        };
        paint_dab(&mut buf, Point { x: 10.0, y: 10.0 }, &settings, 1.0);
        let far = buf.get(0, 0).unwrap();
        assert!(far.a < 0.01);
    }

    #[test]
    fn paint_stroke_interpolates() {
        let mut buf = PixelBuffer::new(30, 10);
        let settings = BrushSettings {
            size: 4.0,
            color: Color::new(0.0, 0.0, 1.0, 1.0),
            ..Default::default()
        };
        let points = vec![
            StrokePoint {
                position: Point { x: 5.0, y: 5.0 },
                pressure: 1.0,
            },
            StrokePoint {
                position: Point { x: 25.0, y: 5.0 },
                pressure: 1.0,
            },
        ];
        paint_stroke(&mut buf, &points, &settings);
        // Middle of stroke should be painted
        let mid = buf.get(15, 5).unwrap();
        assert!(mid.a > 0.5);
    }

    #[test]
    fn erase_reduces_alpha() {
        let mut buf = PixelBuffer::filled(20, 20, Color::new(1.0, 0.0, 0.0, 1.0));
        let settings = BrushSettings {
            size: 6.0,
            ..Default::default()
        };
        erase_dab(&mut buf, Point { x: 10.0, y: 10.0 }, &settings, 1.0);
        let center = buf.get(10, 10).unwrap();
        assert!(center.a < 1.0);
    }

    #[test]
    fn square_tip_fills_square() {
        let mut buf = PixelBuffer::new(20, 20);
        let settings = BrushSettings {
            size: 6.0,
            tip: BrushTip::Square,
            hardness: 1.0,
            color: Color::new(0.0, 1.0, 0.0, 1.0),
            ..Default::default()
        };
        paint_dab(&mut buf, Point { x: 10.0, y: 10.0 }, &settings, 1.0);
        // Check corners of the square
        let corner = buf.get(8, 8).unwrap();
        assert!(corner.a > 0.5);
    }

    #[test]
    fn pressure_zero_is_noop() {
        let mut buf = PixelBuffer::new(20, 20);
        let settings = BrushSettings::default();
        paint_dab(&mut buf, Point { x: 10.0, y: 10.0 }, &settings, 0.0);
        let px = buf.get(10, 10).unwrap();
        assert!(px.a < 0.01);
    }

    #[test]
    fn half_pressure_smaller_dab() {
        let mut buf1 = PixelBuffer::new(20, 20);
        let mut buf2 = PixelBuffer::new(20, 20);
        let settings = BrushSettings {
            size: 10.0,
            color: Color::new(1.0, 0.0, 0.0, 1.0),
            ..Default::default()
        };
        paint_dab(&mut buf1, Point { x: 10.0, y: 10.0 }, &settings, 1.0);
        paint_dab(&mut buf2, Point { x: 10.0, y: 10.0 }, &settings, 0.5);
        // At the edge of the half-pressure radius, full pressure should have coverage
        // but half pressure should not
        let edge = buf2.get(7, 10).unwrap();
        let same_full = buf1.get(7, 10).unwrap();
        assert!(same_full.a >= edge.a);
    }
}
