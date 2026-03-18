use rasa_core::pixel::PixelBuffer;

use crate::filter::{Filter, FilterRegistry};

/// Invert all pixel colors.
pub struct InvertFilter;

impl Filter for InvertFilter {
    fn name(&self) -> &str {
        "Invert"
    }
    fn apply(&self, buf: &mut PixelBuffer) {
        crate::filters::invert(buf);
    }
}

/// Convert to grayscale using luminance weights.
pub struct GrayscaleFilter;

impl Filter for GrayscaleFilter {
    fn name(&self) -> &str {
        "Grayscale"
    }
    fn apply(&self, buf: &mut PixelBuffer) {
        crate::filters::grayscale(buf);
    }
}

/// Gaussian blur with configurable radius.
pub struct GaussianBlurFilter {
    pub radius: u32,
}

impl Filter for GaussianBlurFilter {
    fn name(&self) -> &str {
        "Gaussian Blur"
    }
    fn apply(&self, buf: &mut PixelBuffer) {
        crate::filters::gaussian_blur(buf, self.radius);
    }
}

/// Sharpen using unsharp mask.
pub struct SharpenFilter {
    pub radius: u32,
    pub amount: f32,
}

impl Filter for SharpenFilter {
    fn name(&self) -> &str {
        "Sharpen"
    }
    fn apply(&self, buf: &mut PixelBuffer) {
        crate::filters::sharpen(buf, self.radius, self.amount);
    }
}

/// Register all built-in filters with sensible defaults.
pub fn register_builtins(registry: &mut FilterRegistry) {
    registry.register(Box::new(InvertFilter));
    registry.register(Box::new(GrayscaleFilter));
    registry.register(Box::new(GaussianBlurFilter { radius: 3 }));
    registry.register(Box::new(SharpenFilter {
        radius: 2,
        amount: 1.0,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::color::Color;

    #[test]
    fn register_builtins_populates_registry() {
        let mut reg = FilterRegistry::new();
        register_builtins(&mut reg);
        assert_eq!(reg.len(), 4);
        assert_eq!(
            reg.list_filters(),
            vec!["Invert", "Grayscale", "Gaussian Blur", "Sharpen"]
        );
    }

    #[test]
    fn invert_filter_applies() {
        let f = InvertFilter;
        let mut buf = PixelBuffer::filled(2, 2, Color::WHITE);
        f.apply(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!(px.r < 0.01);
    }

    #[test]
    fn grayscale_filter_applies() {
        let f = GrayscaleFilter;
        let mut buf = PixelBuffer::filled(2, 2, Color::new(1.0, 0.0, 0.0, 1.0));
        f.apply(&mut buf);
        let px = buf.get(0, 0).unwrap();
        assert!((px.r - px.g).abs() < 0.01);
    }

    #[test]
    fn blur_filter_applies() {
        let f = GaussianBlurFilter { radius: 1 };
        let mut buf = PixelBuffer::filled(4, 4, Color::WHITE);
        f.apply(&mut buf);
        // Should not panic
    }

    #[test]
    fn sharpen_filter_applies() {
        let f = SharpenFilter {
            radius: 1,
            amount: 1.0,
        };
        let mut buf = PixelBuffer::filled(4, 4, Color::WHITE);
        f.apply(&mut buf);
        // Should not panic
    }

    #[test]
    fn lookup_builtin_by_name() {
        let mut reg = FilterRegistry::new();
        register_builtins(&mut reg);
        let f = reg.filter_by_name("Invert").unwrap();
        assert_eq!(f.name(), "Invert");
    }
}
