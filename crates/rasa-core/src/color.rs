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
    Cmyk,
}

// ── ICC Profile ──────────────────────────────────────

/// Color space described by an ICC profile (parsed from header bytes 16..19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileColorSpace {
    Rgb,
    Cmyk,
    Gray,
    Lab,
    Unknown,
}

/// An ICC color profile stored as raw bytes with parsed metadata.
///
/// Used for color management at import/export boundaries. The internal
/// editing pipeline always works in linear RGBA f32.
#[derive(Debug, Clone)]
pub struct IccProfile {
    data: Vec<u8>,
    /// Human-readable profile description.
    pub description: String,
    /// The color space this profile describes.
    pub color_space: ProfileColorSpace,
}

impl IccProfile {
    /// Minimum valid ICC profile size (header alone is 128 bytes).
    const MIN_SIZE: usize = 128;

    /// Parse an ICC profile from raw bytes.
    ///
    /// Validates the header, extracts the color space signature and
    /// a basic description.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, crate::error::RasaError> {
        if data.len() < Self::MIN_SIZE {
            return Err(crate::error::RasaError::InvalidIccProfile(
                "data too short for ICC header".into(),
            ));
        }

        // Color space signature at bytes 16..20.
        let sig = &data[16..20];
        let color_space = match sig {
            b"RGB " => ProfileColorSpace::Rgb,
            b"CMYK" => ProfileColorSpace::Cmyk,
            b"GRAY" => ProfileColorSpace::Gray,
            b"Lab " => ProfileColorSpace::Lab,
            _ => ProfileColorSpace::Unknown,
        };

        // Profile description: use the color space name as a fallback.
        // A full parser would read the 'desc' tag, but that's complex.
        let description = match color_space {
            ProfileColorSpace::Rgb => "RGB Profile",
            ProfileColorSpace::Cmyk => "CMYK Profile",
            ProfileColorSpace::Gray => "Gray Profile",
            ProfileColorSpace::Lab => "Lab Profile",
            ProfileColorSpace::Unknown => "Unknown Profile",
        }
        .to_string();

        Ok(Self {
            data,
            description,
            color_space,
        })
    }

    /// Access the raw ICC profile bytes.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Return the built-in sRGB IEC61966-2.1 v2 profile.
    pub fn srgb_v2() -> Self {
        Self {
            data: SRGB_V2_PROFILE.to_vec(),
            description: "sRGB IEC61966-2.1".to_string(),
            color_space: ProfileColorSpace::Rgb,
        }
    }
}

/// Minimal sRGB v2 ICC profile (standard 3144-byte profile).
/// Generated from the canonical sRGB IEC61966-2.1 specification.
/// We embed a minimal valid header + TRC + matrix profile.
static SRGB_V2_PROFILE: &[u8] = include_bytes!("srgb_v2.icc");

// ── CMYK Color ───────────────────────────────────────

/// CMYK color with components in 0.0-1.0 range.
///
/// Used only at export boundaries — the internal pipeline works in linear RGBA.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CmykColor {
    pub c: f32,
    pub m: f32,
    pub y: f32,
    pub k: f32,
}

impl CmykColor {
    pub fn new(c: f32, m: f32, y: f32, k: f32) -> Self {
        Self { c, m, y, k }
    }
}

/// Naive RGB-to-CMYK conversion (no ICC profile).
///
/// Input values should be in sRGB 0.0-1.0 range.
pub fn rgb_to_cmyk_naive(r: f32, g: f32, b: f32) -> CmykColor {
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return CmykColor::new(0.0, 0.0, 0.0, 1.0);
    }
    let inv_k = 1.0 / (1.0 - k);
    CmykColor::new(
        (1.0 - r - k) * inv_k,
        (1.0 - g - k) * inv_k,
        (1.0 - b - k) * inv_k,
        k,
    )
}

/// Naive CMYK-to-RGB conversion (no ICC profile).
///
/// Returns (r, g, b) in sRGB 0.0-1.0 range.
pub fn cmyk_to_rgb_naive(cmyk: CmykColor) -> (f32, f32, f32) {
    let inv_k = 1.0 - cmyk.k;
    (
        (1.0 - cmyk.c) * inv_k,
        (1.0 - cmyk.m) * inv_k,
        (1.0 - cmyk.y) * inv_k,
    )
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

    // ── ICC Profile tests ────────────────────────────

    #[test]
    fn icc_profile_from_bytes_too_short() {
        let result = IccProfile::from_bytes(vec![0; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn icc_profile_from_bytes_rgb_signature() {
        let mut data = vec![0u8; 128];
        data[16..20].copy_from_slice(b"RGB ");
        let profile = IccProfile::from_bytes(data).unwrap();
        assert_eq!(profile.color_space, ProfileColorSpace::Rgb);
    }

    #[test]
    fn icc_profile_from_bytes_cmyk_signature() {
        let mut data = vec![0u8; 128];
        data[16..20].copy_from_slice(b"CMYK");
        let profile = IccProfile::from_bytes(data).unwrap();
        assert_eq!(profile.color_space, ProfileColorSpace::Cmyk);
    }

    #[test]
    fn icc_profile_from_bytes_gray_signature() {
        let mut data = vec![0u8; 128];
        data[16..20].copy_from_slice(b"GRAY");
        let profile = IccProfile::from_bytes(data).unwrap();
        assert_eq!(profile.color_space, ProfileColorSpace::Gray);
    }

    #[test]
    fn icc_profile_from_bytes_lab_signature() {
        let mut data = vec![0u8; 128];
        data[16..20].copy_from_slice(b"Lab ");
        let profile = IccProfile::from_bytes(data).unwrap();
        assert_eq!(profile.color_space, ProfileColorSpace::Lab);
    }

    #[test]
    fn icc_profile_from_bytes_unknown_signature() {
        let mut data = vec![0u8; 128];
        data[16..20].copy_from_slice(b"XYZ ");
        let profile = IccProfile::from_bytes(data).unwrap();
        assert_eq!(profile.color_space, ProfileColorSpace::Unknown);
    }

    #[test]
    fn icc_profile_srgb_v2_valid() {
        let profile = IccProfile::srgb_v2();
        assert_eq!(profile.color_space, ProfileColorSpace::Rgb);
        assert!(!profile.data().is_empty());
        assert!(profile.description.contains("sRGB"));
    }

    #[test]
    fn icc_profile_data_preserved() {
        let mut data = vec![0u8; 128];
        data[16..20].copy_from_slice(b"RGB ");
        data[0] = 42;
        let profile = IccProfile::from_bytes(data).unwrap();
        assert_eq!(profile.data()[0], 42);
    }

    // ── CMYK tests ───────────────────────────────────

    #[test]
    fn cmyk_color_new() {
        let c = CmykColor::new(0.1, 0.2, 0.3, 0.4);
        assert_eq!(c.c, 0.1);
        assert_eq!(c.m, 0.2);
        assert_eq!(c.y, 0.3);
        assert_eq!(c.k, 0.4);
    }

    #[test]
    fn rgb_to_cmyk_naive_white() {
        let cmyk = rgb_to_cmyk_naive(1.0, 1.0, 1.0);
        assert!(approx_eq(cmyk.c, 0.0));
        assert!(approx_eq(cmyk.m, 0.0));
        assert!(approx_eq(cmyk.y, 0.0));
        assert!(approx_eq(cmyk.k, 0.0));
    }

    #[test]
    fn rgb_to_cmyk_naive_black() {
        let cmyk = rgb_to_cmyk_naive(0.0, 0.0, 0.0);
        assert!(approx_eq(cmyk.c, 0.0));
        assert!(approx_eq(cmyk.m, 0.0));
        assert!(approx_eq(cmyk.y, 0.0));
        assert!(approx_eq(cmyk.k, 1.0));
    }

    #[test]
    fn rgb_to_cmyk_naive_pure_red() {
        let cmyk = rgb_to_cmyk_naive(1.0, 0.0, 0.0);
        assert!(approx_eq(cmyk.c, 0.0));
        assert!(approx_eq(cmyk.m, 1.0));
        assert!(approx_eq(cmyk.y, 1.0));
        assert!(approx_eq(cmyk.k, 0.0));
    }

    #[test]
    fn rgb_to_cmyk_naive_pure_cyan() {
        let cmyk = rgb_to_cmyk_naive(0.0, 1.0, 1.0);
        assert!(approx_eq(cmyk.c, 1.0));
        assert!(approx_eq(cmyk.m, 0.0));
        assert!(approx_eq(cmyk.y, 0.0));
        assert!(approx_eq(cmyk.k, 0.0));
    }

    #[test]
    fn cmyk_to_rgb_roundtrip() {
        for (r, g, b) in [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.5, 0.3, 0.8)] {
            let cmyk = rgb_to_cmyk_naive(r, g, b);
            let (r2, g2, b2) = cmyk_to_rgb_naive(cmyk);
            assert!(approx_eq(r, r2), "r: {r} vs {r2}");
            assert!(approx_eq(g, g2), "g: {g} vs {g2}");
            assert!(approx_eq(b, b2), "b: {b} vs {b2}");
        }
    }

    #[test]
    fn cmyk_to_rgb_naive_black() {
        let (r, g, b) = cmyk_to_rgb_naive(CmykColor::new(0.0, 0.0, 0.0, 1.0));
        assert!(approx_eq(r, 0.0));
        assert!(approx_eq(g, 0.0));
        assert!(approx_eq(b, 0.0));
    }

    #[test]
    fn cmyk_mid_gray() {
        let cmyk = rgb_to_cmyk_naive(0.5, 0.5, 0.5);
        // For mid gray, C=M=Y=0, K=0.5
        assert!(approx_eq(cmyk.c, 0.0));
        assert!(approx_eq(cmyk.m, 0.0));
        assert!(approx_eq(cmyk.y, 0.0));
        assert!(approx_eq(cmyk.k, 0.5));
    }

    #[test]
    fn cmyk_pure_green() {
        let cmyk = rgb_to_cmyk_naive(0.0, 1.0, 0.0);
        // Pure green: C=1, M=0, Y=1, K=0
        assert!(approx_eq(cmyk.c, 1.0));
        assert!(approx_eq(cmyk.m, 0.0));
        assert!(approx_eq(cmyk.y, 1.0));
        assert!(approx_eq(cmyk.k, 0.0));
    }

    #[test]
    fn cmyk_pure_blue() {
        let cmyk = rgb_to_cmyk_naive(0.0, 0.0, 1.0);
        // Pure blue: C=1, M=1, Y=0, K=0
        assert!(approx_eq(cmyk.c, 1.0));
        assert!(approx_eq(cmyk.m, 1.0));
        assert!(approx_eq(cmyk.y, 0.0));
        assert!(approx_eq(cmyk.k, 0.0));
    }

    #[test]
    fn color_space_default_is_srgb() {
        assert_eq!(ColorSpace::default(), ColorSpace::Srgb);
    }

    #[test]
    fn color_space_cmyk_variant() {
        let cmyk = ColorSpace::Cmyk;
        let srgb = ColorSpace::Srgb;
        assert_ne!(cmyk, srgb);
    }
}
