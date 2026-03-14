use crate::color::{BlendMode, Color};

/// Blend two colors: `base` (bottom) and `top` (upper layer).
/// Both must be in linear color space. Returns the composited result.
pub fn blend(base: Color, top: Color, mode: BlendMode, opacity: f32) -> Color {
    let top_a = top.a * opacity;

    if top_a <= 0.0 {
        return base;
    }
    if base.a <= 0.0 {
        return Color::new(top.r, top.g, top.b, top_a);
    }

    let blended_r = blend_channel(base.r, top.r, mode);
    let blended_g = blend_channel(base.g, top.g, mode);
    let blended_b = blend_channel(base.b, top.b, mode);

    // Porter-Duff "source over" alpha compositing
    let out_a = top_a + base.a * (1.0 - top_a);
    if out_a <= 0.0 {
        return Color::TRANSPARENT;
    }

    let out_r = (blended_r * top_a + base.r * base.a * (1.0 - top_a)) / out_a;
    let out_g = (blended_g * top_a + base.g * base.a * (1.0 - top_a)) / out_a;
    let out_b = (blended_b * top_a + base.b * base.a * (1.0 - top_a)) / out_a;

    Color::new(
        out_r.clamp(0.0, 1.0),
        out_g.clamp(0.0, 1.0),
        out_b.clamp(0.0, 1.0),
        out_a.clamp(0.0, 1.0),
    )
}

fn blend_channel(base: f32, top: f32, mode: BlendMode) -> f32 {
    match mode {
        BlendMode::Normal => top,
        BlendMode::Multiply => base * top,
        BlendMode::Screen => 1.0 - (1.0 - base) * (1.0 - top),
        BlendMode::Overlay => {
            if base < 0.5 {
                2.0 * base * top
            } else {
                1.0 - 2.0 * (1.0 - base) * (1.0 - top)
            }
        }
        BlendMode::Darken => base.min(top),
        BlendMode::Lighten => base.max(top),
        BlendMode::ColorDodge => {
            if top >= 1.0 {
                1.0
            } else {
                (base / (1.0 - top)).min(1.0)
            }
        }
        BlendMode::ColorBurn => {
            if top <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - base) / top).min(1.0)
            }
        }
        BlendMode::SoftLight => {
            if top <= 0.5 {
                base - (1.0 - 2.0 * top) * base * (1.0 - base)
            } else {
                let d = if base <= 0.25 {
                    ((16.0 * base - 12.0) * base + 4.0) * base
                } else {
                    base.sqrt()
                };
                base + (2.0 * top - 1.0) * (d - base)
            }
        }
        BlendMode::HardLight => {
            if top < 0.5 {
                2.0 * base * top
            } else {
                1.0 - 2.0 * (1.0 - base) * (1.0 - top)
            }
        }
        BlendMode::Difference => (base - top).abs(),
        BlendMode::Exclusion => base + top - 2.0 * base * top,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    const HALF_WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 0.5,
    };
    const GRAY: Color = Color {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    fn colors_approx_eq(a: Color, b: Color) -> bool {
        approx_eq(a.r, b.r) && approx_eq(a.g, b.g) && approx_eq(a.b, b.b) && approx_eq(a.a, b.a)
    }

    #[test]
    fn normal_opaque_replaces() {
        let result = blend(RED, BLUE, BlendMode::Normal, 1.0);
        assert!(colors_approx_eq(result, BLUE));
    }

    #[test]
    fn normal_transparent_top_returns_base() {
        let result = blend(RED, Color::TRANSPARENT, BlendMode::Normal, 1.0);
        assert!(colors_approx_eq(result, RED));
    }

    #[test]
    fn normal_half_opacity() {
        let result = blend(RED, BLUE, BlendMode::Normal, 0.5);
        assert!(approx_eq(result.a, 1.0));
        assert!(approx_eq(result.r, 0.5));
        assert!(approx_eq(result.b, 0.5));
    }

    #[test]
    fn multiply_black_zeroes() {
        let result = blend(GRAY, Color::BLACK, BlendMode::Multiply, 1.0);
        assert!(approx_eq(result.r, 0.0));
        assert!(approx_eq(result.g, 0.0));
        assert!(approx_eq(result.b, 0.0));
    }

    #[test]
    fn multiply_white_preserves() {
        let result = blend(GRAY, Color::WHITE, BlendMode::Multiply, 1.0);
        assert!(colors_approx_eq(result, GRAY));
    }

    #[test]
    fn screen_black_preserves() {
        let result = blend(GRAY, Color::BLACK, BlendMode::Screen, 1.0);
        assert!(colors_approx_eq(result, GRAY));
    }

    #[test]
    fn screen_white_saturates() {
        let result = blend(GRAY, Color::WHITE, BlendMode::Screen, 1.0);
        assert!(colors_approx_eq(result, Color::WHITE));
    }

    #[test]
    fn difference_same_is_black() {
        let result = blend(GRAY, GRAY, BlendMode::Difference, 1.0);
        assert!(approx_eq(result.r, 0.0));
        assert!(approx_eq(result.g, 0.0));
        assert!(approx_eq(result.b, 0.0));
    }

    #[test]
    fn blend_onto_transparent_base() {
        let result = blend(Color::TRANSPARENT, HALF_WHITE, BlendMode::Normal, 1.0);
        assert!(approx_eq(result.a, 0.5));
        assert!(approx_eq(result.r, 1.0));
    }

    #[test]
    fn zero_opacity_returns_base() {
        let result = blend(RED, BLUE, BlendMode::Normal, 0.0);
        assert!(colors_approx_eq(result, RED));
    }

    #[test]
    fn overlay_dark_base() {
        let dark = Color::new(0.2, 0.2, 0.2, 1.0);
        let result = blend(dark, GRAY, BlendMode::Overlay, 1.0);
        // base < 0.5: 2 * 0.2 * 0.5 = 0.2
        assert!(approx_eq(result.r, 0.2));
    }

    #[test]
    fn overlay_light_base() {
        let light = Color::new(0.8, 0.8, 0.8, 1.0);
        let result = blend(light, GRAY, BlendMode::Overlay, 1.0);
        // base >= 0.5: 1 - 2*(1-0.8)*(1-0.5) = 1 - 2*0.2*0.5 = 0.8
        assert!(approx_eq(result.r, 0.8));
    }

    #[test]
    fn exclusion_symmetry() {
        let a = blend(RED, BLUE, BlendMode::Exclusion, 1.0);
        let b = blend(BLUE, RED, BlendMode::Exclusion, 1.0);
        assert!(colors_approx_eq(a, b));
    }

    #[test]
    fn darken_takes_minimum() {
        let result = blend(GRAY, Color::new(0.3, 0.3, 0.3, 1.0), BlendMode::Darken, 1.0);
        assert!(approx_eq(result.r, 0.3));
    }

    #[test]
    fn lighten_takes_maximum() {
        let result = blend(
            GRAY,
            Color::new(0.8, 0.8, 0.8, 1.0),
            BlendMode::Lighten,
            1.0,
        );
        assert!(approx_eq(result.r, 0.8));
    }

    #[test]
    fn color_dodge_white_top() {
        let result = blend(GRAY, Color::WHITE, BlendMode::ColorDodge, 1.0);
        assert!(approx_eq(result.r, 1.0));
    }

    #[test]
    fn color_dodge_normal() {
        let result = blend(GRAY, GRAY, BlendMode::ColorDodge, 1.0);
        assert!(result.r > 0.5); // dodge brightens
    }

    #[test]
    fn color_burn_black_top() {
        let result = blend(GRAY, Color::BLACK, BlendMode::ColorBurn, 1.0);
        assert!(approx_eq(result.r, 0.0));
    }

    #[test]
    fn color_burn_normal() {
        let result = blend(GRAY, GRAY, BlendMode::ColorBurn, 1.0);
        assert!(result.r < 0.5); // burn darkens
    }

    #[test]
    fn soft_light_low() {
        let result = blend(
            GRAY,
            Color::new(0.3, 0.3, 0.3, 1.0),
            BlendMode::SoftLight,
            1.0,
        );
        assert!(result.r < 0.5); // soft light with dark top darkens
    }

    #[test]
    fn soft_light_high() {
        let result = blend(
            GRAY,
            Color::new(0.8, 0.8, 0.8, 1.0),
            BlendMode::SoftLight,
            1.0,
        );
        assert!(result.r > 0.5); // soft light with bright top lightens
    }

    #[test]
    fn soft_light_dark_base() {
        // Tests the base <= 0.25 branch
        let dark = Color::new(0.1, 0.1, 0.1, 1.0);
        let result = blend(
            dark,
            Color::new(0.8, 0.8, 0.8, 1.0),
            BlendMode::SoftLight,
            1.0,
        );
        assert!(result.r > 0.1);
    }

    #[test]
    fn hard_light_dark_top() {
        let result = blend(
            GRAY,
            Color::new(0.3, 0.3, 0.3, 1.0),
            BlendMode::HardLight,
            1.0,
        );
        assert!(result.r < 0.5);
    }

    #[test]
    fn hard_light_bright_top() {
        let result = blend(
            GRAY,
            Color::new(0.8, 0.8, 0.8, 1.0),
            BlendMode::HardLight,
            1.0,
        );
        assert!(result.r > 0.5);
    }
}
