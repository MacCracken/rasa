use rasa_core::color::Color;
use rasa_core::geometry::{Point, Rect};
use rasa_core::pixel::PixelBuffer;
use rasa_core::selection::Selection;
use rasa_core::transform::Transform;

/// Pick the color at a given pixel coordinate.
pub fn eyedropper(buf: &PixelBuffer, x: u32, y: u32) -> Option<Color> {
    buf.get(x, y)
}

/// Pick color and return as sRGB u8.
pub fn eyedropper_srgb(buf: &PixelBuffer, x: u32, y: u32) -> Option<[u8; 4]> {
    buf.get(x, y).map(|c| c.to_srgb_u8())
}

/// Flood fill from a seed point with the given color.
/// Tolerance is 0.0 (exact match) to 1.0 (fill everything).
pub fn flood_fill(
    buf: &mut PixelBuffer,
    seed_x: u32,
    seed_y: u32,
    fill_color: Color,
    tolerance: f32,
) {
    let (w, h) = buf.dimensions();
    if seed_x >= w || seed_y >= h {
        return;
    }

    let target = buf.get(seed_x, seed_y).unwrap();
    let mut visited = vec![false; (w * h) as usize];
    let mut stack = vec![(seed_x, seed_y)];

    while let Some((x, y)) = stack.pop() {
        let idx = (y as usize) * (w as usize) + (x as usize);
        if visited[idx] {
            continue;
        }
        visited[idx] = true;

        let px = buf.get(x, y).unwrap();
        if color_distance(&px, &target) > tolerance {
            continue;
        }

        buf.set(x, y, fill_color);

        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }
}

/// Fill a selection region with a solid color.
pub fn fill_selection(buf: &mut PixelBuffer, selection: &Selection, color: Color) {
    let (w, h) = buf.dimensions();
    for y in 0..h {
        for x in 0..w {
            if selection.contains(Point {
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
            }) {
                buf.set(x, y, color);
            }
        }
    }
}

/// Linear gradient fill between two points.
pub fn gradient_linear(
    buf: &mut PixelBuffer,
    start: Point,
    end: Point,
    color_start: Color,
    color_end: Color,
) {
    let (w, h) = buf.dimensions();
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-6 {
        // Degenerate: fill with start color
        for y in 0..h {
            for x in 0..w {
                buf.set(x, y, color_start);
            }
        }
        return;
    }

    for y in 0..h {
        for x in 0..w {
            let px = x as f64 + 0.5;
            let py = y as f64 + 0.5;
            let t = ((px - start.x) * dx + (py - start.y) * dy) / len_sq;
            let t = t.clamp(0.0, 1.0) as f32;
            let color = lerp_color(&color_start, &color_end, t);
            buf.set(x, y, color);
        }
    }
}

/// Crop a pixel buffer to the given rectangle.
pub fn crop(buf: &PixelBuffer, region: Rect) -> PixelBuffer {
    let x0 = (region.x as u32).min(buf.width);
    let y0 = (region.y as u32).min(buf.height);
    let x1 = ((region.x + region.width) as u32).min(buf.width);
    let y1 = ((region.y + region.height) as u32).min(buf.height);
    let w = x1.saturating_sub(x0);
    let h = y1.saturating_sub(y0);

    let mut output = PixelBuffer::new(w, h);
    for dy in 0..h {
        for dx in 0..w {
            if let Some(px) = buf.get(x0 + dx, y0 + dy) {
                output.set(dx, dy, px);
            }
        }
    }
    output
}

/// Apply an affine transform to a pixel buffer using bilinear interpolation.
pub fn transform_buffer(
    buf: &PixelBuffer,
    transform: &Transform,
    output_width: u32,
    output_height: u32,
) -> PixelBuffer {
    let inv = match transform.inverse() {
        Some(inv) => inv,
        None => return PixelBuffer::new(output_width, output_height),
    };

    let mut output = PixelBuffer::new(output_width, output_height);
    for y in 0..output_height {
        for x in 0..output_width {
            let src = inv.apply(Point {
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
            });
            let color = sample_bilinear(buf, src.x as f32, src.y as f32);
            output.set(x, y, color);
        }
    }
    output
}

fn sample_bilinear(buf: &PixelBuffer, x: f32, y: f32) -> Color {
    let x0 = (x - 0.5).floor() as i32;
    let y0 = (y - 0.5).floor() as i32;
    let fx = x - 0.5 - x0 as f32;
    let fy = y - 0.5 - y0 as f32;

    let c00 = get_clamped(buf, x0, y0);
    let c10 = get_clamped(buf, x0 + 1, y0);
    let c01 = get_clamped(buf, x0, y0 + 1);
    let c11 = get_clamped(buf, x0 + 1, y0 + 1);

    let top = lerp_color(&c00, &c10, fx);
    let bot = lerp_color(&c01, &c11, fx);
    lerp_color(&top, &bot, fy)
}

fn get_clamped(buf: &PixelBuffer, x: i32, y: i32) -> Color {
    let cx = x.clamp(0, buf.width as i32 - 1) as u32;
    let cy = y.clamp(0, buf.height as i32 - 1) as u32;
    buf.get(cx, cy).unwrap()
}

fn lerp_color(a: &Color, b: &Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn color_distance(a: &Color, b: &Color) -> f32 {
    let dr = a.r - b.r;
    let dg = a.g - b.g;
    let db = a.b - b.b;
    let da = a.a - b.a;
    (dr * dr + dg * dg + db * db + da * da).sqrt() / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.05
    }

    // ── Eyedropper ──

    #[test]
    fn eyedropper_returns_color() {
        let buf = PixelBuffer::filled(4, 4, Color::new(1.0, 0.0, 0.0, 1.0));
        let c = eyedropper(&buf, 2, 2).unwrap();
        assert!(approx_eq(c.r, 1.0));
    }

    #[test]
    fn eyedropper_out_of_bounds() {
        let buf = PixelBuffer::new(4, 4);
        assert!(eyedropper(&buf, 10, 10).is_none());
    }

    #[test]
    fn eyedropper_srgb_converts() {
        let buf = PixelBuffer::filled(1, 1, Color::WHITE);
        let [r, g, b, a] = eyedropper_srgb(&buf, 0, 0).unwrap();
        assert_eq!(r, 255);
        assert_eq!(a, 255);
    }

    // ── Flood fill ──

    #[test]
    fn flood_fill_uniform() {
        let mut buf = PixelBuffer::filled(8, 8, Color::WHITE);
        flood_fill(&mut buf, 0, 0, Color::new(1.0, 0.0, 0.0, 1.0), 0.1);
        // Entire buffer should be red
        let px = buf.get(7, 7).unwrap();
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.g, 0.0));
    }

    #[test]
    fn flood_fill_stops_at_boundary() {
        let mut buf = PixelBuffer::filled(8, 8, Color::WHITE);
        // Draw a vertical black line at x=4
        for y in 0..8 {
            buf.set(4, y, Color::BLACK);
        }
        flood_fill(&mut buf, 0, 0, Color::new(1.0, 0.0, 0.0, 1.0), 0.1);
        // Left side should be red
        let left = buf.get(2, 2).unwrap();
        assert!(approx_eq(left.r, 1.0));
        // Right side should still be white
        let right = buf.get(6, 6).unwrap();
        assert!(approx_eq(right.r, 1.0));
        assert!(approx_eq(right.g, 1.0));
    }

    #[test]
    fn flood_fill_out_of_bounds_seed() {
        let mut buf = PixelBuffer::new(4, 4);
        flood_fill(&mut buf, 10, 10, Color::WHITE, 0.1); // should not panic
    }

    // ── Fill selection ──

    #[test]
    fn fill_selection_rect() {
        let mut buf = PixelBuffer::new(10, 10);
        let sel = Selection::Rect(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        });
        fill_selection(&mut buf, &sel, Color::new(0.0, 1.0, 0.0, 1.0));
        // Inside selection
        let inside = buf.get(4, 4).unwrap();
        assert!(approx_eq(inside.g, 1.0));
        // Outside selection
        let outside = buf.get(0, 0).unwrap();
        assert!(approx_eq(outside.a, 0.0));
    }

    // ── Gradient ──

    #[test]
    fn gradient_horizontal() {
        let mut buf = PixelBuffer::new(10, 1);
        gradient_linear(
            &mut buf,
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Color::BLACK,
            Color::WHITE,
        );
        let start = buf.get(0, 0).unwrap();
        let end = buf.get(9, 0).unwrap();
        assert!(start.r < 0.2);
        assert!(end.r > 0.8);
    }

    #[test]
    fn gradient_degenerate() {
        let mut buf = PixelBuffer::new(4, 4);
        gradient_linear(
            &mut buf,
            Point { x: 5.0, y: 5.0 },
            Point { x: 5.0, y: 5.0 },
            Color::new(0.5, 0.5, 0.5, 1.0),
            Color::WHITE,
        );
        let px = buf.get(0, 0).unwrap();
        assert!(approx_eq(px.r, 0.5));
    }

    // ── Crop ──

    #[test]
    fn crop_extracts_region() {
        let buf = PixelBuffer::filled(10, 10, Color::WHITE);
        let cropped = crop(
            &buf,
            Rect {
                x: 2.0,
                y: 2.0,
                width: 4.0,
                height: 4.0,
            },
        );
        assert_eq!(cropped.dimensions(), (4, 4));
        assert!(approx_eq(cropped.get(0, 0).unwrap().r, 1.0));
    }

    #[test]
    fn crop_clamps_to_bounds() {
        let buf = PixelBuffer::new(5, 5);
        let cropped = crop(
            &buf,
            Rect {
                x: 3.0,
                y: 3.0,
                width: 10.0,
                height: 10.0,
            },
        );
        assert_eq!(cropped.dimensions(), (2, 2));
    }

    // ── Transform ──

    #[test]
    fn transform_identity() {
        let buf = PixelBuffer::filled(4, 4, Color::new(1.0, 0.0, 0.0, 1.0));
        let result = transform_buffer(&buf, &Transform::IDENTITY, 4, 4);
        let px = result.get(2, 2).unwrap();
        assert!(approx_eq(px.r, 1.0));
    }

    #[test]
    fn transform_scale_up() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.set(0, 0, Color::new(1.0, 0.0, 0.0, 1.0));
        buf.set(1, 0, Color::new(0.0, 1.0, 0.0, 1.0));
        buf.set(0, 1, Color::new(0.0, 0.0, 1.0, 1.0));
        buf.set(1, 1, Color::WHITE);
        let t = Transform::scale(2.0, 2.0);
        let result = transform_buffer(&buf, &t, 4, 4);
        // Top-left quadrant should be reddish
        let px = result.get(0, 0).unwrap();
        assert!(px.r > 0.5);
    }

    #[test]
    fn transform_singular_returns_empty() {
        let buf = PixelBuffer::filled(4, 4, Color::WHITE);
        let t = Transform::scale(0.0, 0.0);
        let result = transform_buffer(&buf, &t, 4, 4);
        let px = result.get(0, 0).unwrap();
        assert!(approx_eq(px.a, 0.0));
    }

    // ── Color distance ──

    #[test]
    fn same_color_zero_distance() {
        assert!(approx_eq(color_distance(&Color::WHITE, &Color::WHITE), 0.0));
    }

    #[test]
    fn different_colors_positive_distance() {
        assert!(color_distance(&Color::BLACK, &Color::WHITE) > 0.5);
    }
}
