use kurbo::{BezPath, ParamCurve, ParamCurveNearest, PathSeg, Shape};
use rasa_core::color::Color;
use rasa_core::pixel::PixelBuffer;
use rasa_core::vector::{FillStyle, PathSegment, StrokeStyle, VectorData, VectorPath};

/// Render vector data into a pixel buffer.
///
/// Each `VectorPath` is converted to a `kurbo::BezPath`, then rasterised using
/// a simple per-pixel containment test. This is O(pixels * segments), which is
/// acceptable for the current MVP. A scanline rasteriser can replace this later
/// without changing the public API.
pub fn render_vector_layer(data: &VectorData, width: u32, height: u32) -> PixelBuffer {
    let mut buf = PixelBuffer::new(width, height);

    for path in &data.paths {
        let bez = to_bez_path(path);

        // Render fill (only for closed paths).
        if let Some(ref fill) = path.fill
            && path.closed
        {
            render_filled(&mut buf, &bez, fill, width, height);
        }

        // Render stroke.
        if let Some(ref stroke_style) = path.stroke {
            render_stroked(&mut buf, &bez, stroke_style, width, height);
        }
    }

    buf
}

/// Convert our `VectorPath` segments into a `kurbo::BezPath`.
fn to_bez_path(path: &VectorPath) -> BezPath {
    let mut bp = BezPath::new();
    for seg in &path.segments {
        match *seg {
            PathSegment::MoveTo(p) => bp.move_to((p.x, p.y)),
            PathSegment::LineTo(p) => bp.line_to((p.x, p.y)),
            PathSegment::QuadTo { ctrl, end } => {
                bp.quad_to((ctrl.x, ctrl.y), (end.x, end.y));
            }
            PathSegment::CubicTo { ctrl1, ctrl2, end } => {
                bp.curve_to((ctrl1.x, ctrl1.y), (ctrl2.x, ctrl2.y), (end.x, end.y));
            }
        }
    }
    if path.closed {
        bp.close_path();
    }
    bp
}

/// Fill the interior of `bez` using `kurbo::Shape::contains`.
fn render_filled(buf: &mut PixelBuffer, bez: &BezPath, fill: &FillStyle, w: u32, h: u32) {
    let FillStyle::Solid(color) = fill;

    for y in 0..h {
        for x in 0..w {
            let pt = kurbo::Point::new(x as f64 + 0.5, y as f64 + 0.5);
            if bez.contains(pt) {
                blend_pixel(buf, x, y, *color);
            }
        }
    }
}

/// Stroke the path using a distance-based approach.
///
/// For each pixel, we compute the minimum distance to any segment of the path.
/// If the distance is within half the stroke width, the pixel is painted.
fn render_stroked(buf: &mut PixelBuffer, bez: &BezPath, style: &StrokeStyle, w: u32, h: u32) {
    let half_width = style.width / 2.0;
    let segments: Vec<PathSeg> = bez.segments().collect();

    for y in 0..h {
        for x in 0..w {
            let pt = kurbo::Point::new(x as f64 + 0.5, y as f64 + 0.5);
            let mut min_dist_sq = f64::MAX;
            for seg in &segments {
                let nearest = seg.nearest(pt, 1e-6);
                let nearest_pt = seg.eval(nearest.t);
                let dx = pt.x - nearest_pt.x;
                let dy = pt.y - nearest_pt.y;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < min_dist_sq {
                    min_dist_sq = dist_sq;
                }
            }
            if min_dist_sq <= half_width * half_width {
                blend_pixel(buf, x, y, style.color);
            }
        }
    }
}

/// Simple alpha-over blend of `color` onto the existing pixel at (x, y).
fn blend_pixel(buf: &mut PixelBuffer, x: u32, y: u32, color: Color) {
    let existing = buf.get(x, y).unwrap_or(Color::TRANSPARENT);
    let sa = color.a;
    let da = existing.a;
    let out_a = sa + da * (1.0 - sa);
    if out_a < 1e-7 {
        return;
    }
    let out = Color {
        r: (color.r * sa + existing.r * da * (1.0 - sa)) / out_a,
        g: (color.g * sa + existing.g * da * (1.0 - sa)) / out_a,
        b: (color.b * sa + existing.b * da * (1.0 - sa)) / out_a,
        a: out_a,
    };
    buf.set(x, y, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::vector::{LineCap, LineJoin};

    #[test]
    fn render_empty_vector() {
        let data = VectorData::new();
        let buf = render_vector_layer(&data, 10, 10);
        // All pixels should be transparent.
        for y in 0..10 {
            for x in 0..10 {
                let px = buf.get(x, y).unwrap();
                assert_eq!(px.a, 0.0, "pixel ({x},{y}) should be transparent");
            }
        }
    }

    #[test]
    fn render_filled_rect() {
        let mut data = VectorData::new();
        data.add_path(VectorPath::rect(
            2.0,
            2.0,
            6.0,
            6.0,
            Some(FillStyle::Solid(Color::new(1.0, 0.0, 0.0, 1.0))),
            None,
        ));
        let buf = render_vector_layer(&data, 10, 10);

        // Center pixel should be filled (red).
        let center = buf.get(5, 5).unwrap();
        assert!(center.a > 0.0, "center pixel should be non-transparent");
        assert!(center.r > 0.0, "center pixel should have red");

        // Corner (0,0) should be transparent.
        let corner = buf.get(0, 0).unwrap();
        assert_eq!(corner.a, 0.0, "corner should be transparent");
    }

    #[test]
    fn render_stroked_line() {
        let mut data = VectorData::new();
        let stroke = StrokeStyle {
            color: Color::BLACK,
            width: 2.0,
            cap: LineCap::default(),
            join: LineJoin::default(),
        };
        data.add_path(VectorPath::line(0.0, 5.0, 20.0, 5.0, stroke));
        let buf = render_vector_layer(&data, 20, 10);

        // A pixel along the line should be non-transparent.
        let on_line = buf.get(10, 5).unwrap();
        assert!(on_line.a > 0.0, "pixel on the line should be filled");

        // A pixel far from the line should be transparent.
        let off_line = buf.get(10, 0).unwrap();
        assert_eq!(off_line.a, 0.0, "pixel far from line should be transparent");
    }

    #[test]
    fn render_respects_dimensions() {
        let data = VectorData::new();
        let buf = render_vector_layer(&data, 37, 19);
        assert_eq!(buf.width, 37);
        assert_eq!(buf.height, 19);
    }
}
