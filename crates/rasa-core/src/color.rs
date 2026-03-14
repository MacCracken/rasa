use serde::{Deserialize, Serialize};

/// RGBA color in linear space (0.0-1.0 per channel).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_srgb_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: srgb_to_linear(r as f32 / 255.0),
            g: srgb_to_linear(g as f32 / 255.0),
            b: srgb_to_linear(b as f32 / 255.0),
            a: a as f32 / 255.0,
        }
    }

    pub fn to_srgb_u8(self) -> [u8; 4] {
        [
            (linear_to_srgb(self.r) * 255.0 + 0.5) as u8,
            (linear_to_srgb(self.g) * 255.0 + 0.5) as u8,
            (linear_to_srgb(self.b) * 255.0 + 0.5) as u8,
            (self.a * 255.0 + 0.5) as u8,
        ]
    }

    /// Convert to HSL. Returns (hue 0-360, saturation 0-1, lightness 0-1).
    pub fn to_hsl(self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let l = (max + min) / 2.0;

        if (max - min).abs() < 1e-7 {
            return (0.0, 0.0, l);
        }

        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        let h = if (max - self.r).abs() < 1e-7 {
            let mut h = (self.g - self.b) / d;
            if self.g < self.b {
                h += 6.0;
            }
            h
        } else if (max - self.g).abs() < 1e-7 {
            (self.b - self.r) / d + 2.0
        } else {
            (self.r - self.g) / d + 4.0
        };

        (h * 60.0, s, l)
    }

    /// Create from HSL values (hue 0-360, saturation 0-1, lightness 0-1).
    pub fn from_hsl(h: f32, s: f32, l: f32, a: f32) -> Self {
        if s.abs() < 1e-7 {
            return Self::new(l, l, l, a);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        let h = h / 360.0;

        Self::new(
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
            a,
        )
    }
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

pub fn srgb_to_linear(s: f32) -> f32 {
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(l: f32) -> f32 {
    let l = l.clamp(0.0, 1.0);
    if l <= 0.0031308 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

/// Color space for color management.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    #[default]
    Srgb,
    LinearRgb,
    DisplayP3,
}

/// Blending mode for layer compositing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    SoftLight,
    HardLight,
    Difference,
    Exclusion,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.02
    }

    #[test]
    fn srgb_roundtrip() {
        let original = Color::from_srgb_u8(128, 64, 200, 255);
        let [r, g, b, a] = original.to_srgb_u8();
        assert_eq!(r, 128);
        assert_eq!(g, 64);
        assert_eq!(b, 200);
        assert_eq!(a, 255);
    }

    #[test]
    fn linear_srgb_roundtrip() {
        for v in [0.0_f32, 0.01, 0.04, 0.1, 0.5, 0.9, 1.0] {
            let l = srgb_to_linear(v);
            let s = linear_to_srgb(l);
            assert!(approx_eq(v, s), "failed for {v}: got {s}");
        }
    }

    #[test]
    fn hsl_roundtrip_red() {
        let red = Color::new(1.0, 0.0, 0.0, 1.0);
        let (h, s, l) = red.to_hsl();
        assert!(approx_eq(h, 0.0));
        assert!(approx_eq(s, 1.0));
        assert!(approx_eq(l, 0.5));
        let back = Color::from_hsl(h, s, l, 1.0);
        assert!(approx_eq(back.r, 1.0));
        assert!(approx_eq(back.g, 0.0));
        assert!(approx_eq(back.b, 0.0));
    }

    #[test]
    fn hsl_gray() {
        let gray = Color::new(0.5, 0.5, 0.5, 1.0);
        let (_, s, l) = gray.to_hsl();
        assert!(approx_eq(s, 0.0));
        assert!(approx_eq(l, 0.5));
    }

    #[test]
    fn linear_to_srgb_clamps() {
        assert_eq!(linear_to_srgb(-1.0), 0.0);
        assert!(approx_eq(linear_to_srgb(2.0), 1.0));
    }
}
