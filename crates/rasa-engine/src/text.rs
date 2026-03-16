//! Text rendering engine — rasterises [`TextLayer`] content into a [`PixelBuffer`].
//!
//! The primary entry point is [`render_text_layer`], which produces a transparent
//! buffer when no built-in font is available, and [`render_text_layer_with_font`],
//! which performs actual glyph rasterisation using `ab_glyph`.

use ab_glyph::{Font, FontRef, Glyph, PxScale, ScaleFont, point};
use rasa_core::color::Color;
use rasa_core::layer::{TextAlign, TextLayer};
use rasa_core::pixel::PixelBuffer;

/// Render a [`TextLayer`] into a [`PixelBuffer`] of the given dimensions.
///
/// Currently returns a transparent buffer because no font is bundled with the
/// engine. Use [`render_text_layer_with_font`] to supply font data explicitly.
pub fn render_text_layer(text: &TextLayer, width: u32, height: u32) -> PixelBuffer {
    // No built-in font embedded yet — return transparent placeholder.
    // When a default font is added to rasa-engine/src/fonts/, this will
    // delegate to render_text_layer_with_font with the embedded bytes.
    let _ = text;
    PixelBuffer::new(width, height)
}

/// Render a [`TextLayer`] into a [`PixelBuffer`] using the supplied TrueType/OpenType font bytes.
pub fn render_text_layer_with_font(
    text: &TextLayer,
    width: u32,
    height: u32,
    font_data: &[u8],
) -> PixelBuffer {
    let mut buf = PixelBuffer::new(width, height);

    if text.content.is_empty() || width == 0 || height == 0 {
        return buf;
    }

    let font = match FontRef::try_from_slice(font_data) {
        Ok(f) => f,
        Err(_) => return buf,
    };

    let scale = PxScale::from(text.font_size);
    let scaled_font = font.as_scaled(scale);
    let line_gap = text.line_height * text.font_size;

    let mut cursor_y = scaled_font.ascent();

    for line in text.content.split('\n') {
        if cursor_y > height as f32 {
            break;
        }

        // Compute line width for alignment.
        let line_width = compute_line_width(&scaled_font, line);
        let x_offset = match text.alignment {
            TextAlign::Left => 0.0,
            TextAlign::Center => ((width as f32) - line_width).max(0.0) / 2.0,
            TextAlign::Right => ((width as f32) - line_width).max(0.0),
        };

        let mut cursor_x = x_offset;
        let mut last_glyph: Option<Glyph> = None;

        for ch in line.chars() {
            let glyph_id = font.glyph_id(ch);

            if let Some(ref prev) = last_glyph {
                cursor_x += scaled_font.kern(prev.id, glyph_id);
            }

            let glyph = glyph_id.with_scale_and_position(scale, point(cursor_x, cursor_y));
            cursor_x += scaled_font.h_advance(glyph_id);

            if let Some(outlined) = font.outline_glyph(glyph.clone()) {
                let bounds = outlined.px_bounds();
                outlined.draw(|px_x, px_y, coverage| {
                    let x = px_x as i32 + bounds.min.x as i32;
                    let y = px_y as i32 + bounds.min.y as i32;
                    if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
                        let alpha = coverage * text.color.a;
                        let existing = buf.get(x as u32, y as u32).unwrap_or(Color::TRANSPARENT);
                        // Pre-multiplied alpha compositing.
                        let inv = 1.0 - alpha;
                        let out = Color {
                            r: text.color.r * alpha + existing.r * inv,
                            g: text.color.g * alpha + existing.g * inv,
                            b: text.color.b * alpha + existing.b * inv,
                            a: alpha + existing.a * inv,
                        };
                        buf.set(x as u32, y as u32, out);
                    }
                });
            }
            last_glyph = Some(glyph);
        }

        cursor_y += line_gap;
    }

    buf
}

/// Measure the pixel width of a line of text at the given scale.
fn compute_line_width<F: Font, SF: ScaleFont<F>>(scaled_font: &SF, line: &str) -> f32 {
    let mut width = 0.0f32;
    let mut prev_glyph_id = None;
    for ch in line.chars() {
        let glyph_id = scaled_font.glyph_id(ch);
        if let Some(prev) = prev_glyph_id {
            width += scaled_font.kern(prev, glyph_id);
        }
        width += scaled_font.h_advance(glyph_id);
        prev_glyph_id = Some(glyph_id);
    }
    width
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_text_layer(content: &str) -> TextLayer {
        TextLayer {
            content: content.into(),
            font_family: "Test".into(),
            font_size: 24.0,
            color: Color::BLACK,
            alignment: TextAlign::Left,
            line_height: 1.2,
        }
    }

    /// Try to load a TrueType font from common system paths.
    /// Returns `None` if no font is found (tests that need a font will be skipped).
    fn find_system_font() -> Option<Vec<u8>> {
        let candidates = [
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/TTF/LiberationSans-Regular.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "C:\\Windows\\Fonts\\arial.ttf",
        ];
        for path in &candidates {
            if let Ok(data) = std::fs::read(path) {
                return Some(data);
            }
        }
        None
    }

    #[test]
    fn render_empty_text() {
        let text = make_text_layer("");
        let buf = render_text_layer(&text, 100, 50);
        // All pixels should be transparent.
        for px in buf.pixels() {
            assert_eq!(px.a, 0.0);
        }
    }

    #[test]
    fn render_text_respects_dimensions() {
        let text = make_text_layer("Hello");
        let buf = render_text_layer(&text, 200, 100);
        assert_eq!(buf.width, 200);
        assert_eq!(buf.height, 100);
    }

    #[test]
    fn render_text_has_pixels() {
        let Some(font_data) = find_system_font() else {
            eprintln!("skipping render_text_has_pixels: no system font found");
            return;
        };
        let text = make_text_layer("Hello");
        let buf = render_text_layer_with_font(&text, 200, 100, &font_data);
        assert_eq!(buf.width, 200);
        assert_eq!(buf.height, 100);
        // At least some pixels should be non-transparent.
        let has_visible = buf.pixels().iter().any(|px| px.a > 0.0);
        assert!(
            has_visible,
            "expected some visible pixels from text rendering"
        );
    }

    #[test]
    fn render_text_with_invalid_font() {
        let text = make_text_layer("Hello");
        let buf = render_text_layer_with_font(&text, 100, 50, b"not a font");
        // Should return transparent buffer without panicking.
        assert_eq!(buf.width, 100);
        assert_eq!(buf.height, 50);
    }

    #[test]
    fn render_text_multiline() {
        let Some(font_data) = find_system_font() else {
            eprintln!("skipping render_text_multiline: no system font found");
            return;
        };
        let text = TextLayer {
            content: "Line1\nLine2".into(),
            font_family: "Test".into(),
            font_size: 20.0,
            color: Color::BLACK,
            alignment: TextAlign::Left,
            line_height: 1.2,
        };
        let buf = render_text_layer_with_font(&text, 200, 100, &font_data);
        let has_visible = buf.pixels().iter().any(|px| px.a > 0.0);
        assert!(has_visible);
    }

    #[test]
    fn render_text_center_alignment() {
        let Some(font_data) = find_system_font() else {
            eprintln!("skipping render_text_center_alignment: no system font found");
            return;
        };
        let text = TextLayer {
            content: "Hi".into(),
            font_family: "Test".into(),
            font_size: 20.0,
            color: Color::BLACK,
            alignment: TextAlign::Center,
            line_height: 1.2,
        };
        let buf = render_text_layer_with_font(&text, 200, 50, &font_data);
        let has_visible = buf.pixels().iter().any(|px| px.a > 0.0);
        assert!(has_visible);
    }

    #[test]
    fn render_empty_content_with_font() {
        let Some(font_data) = find_system_font() else {
            eprintln!("skipping render_empty_content_with_font: no system font found");
            return;
        };
        let text = make_text_layer("");
        let buf = render_text_layer_with_font(&text, 100, 50, &font_data);
        for px in buf.pixels() {
            assert_eq!(px.a, 0.0);
        }
    }
}
