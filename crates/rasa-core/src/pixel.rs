use crate::color::Color;

/// A pixel buffer — contiguous RGBA f32 pixels in linear color space.
#[derive(Debug, Clone)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    data: Vec<Color>,
}

impl PixelBuffer {
    /// Maximum pixels per buffer (256 megapixels).
    const MAX_PIXELS: usize = 256 * 1024 * 1024;

    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize)
            .checked_mul(height as usize)
            .unwrap_or(0)
            .min(Self::MAX_PIXELS);
        Self {
            width,
            height,
            data: vec![Color::TRANSPARENT; len],
        }
    }

    pub fn filled(width: u32, height: u32, color: Color) -> Self {
        let len = (width as usize)
            .checked_mul(height as usize)
            .unwrap_or(0)
            .min(Self::MAX_PIXELS);
        Self {
            width,
            height,
            data: vec![color; len],
        }
    }

    pub fn get(&self, x: u32, y: u32) -> Option<Color> {
        if x < self.width && y < self.height {
            Some(self.data[self.index(x, y)])
        } else {
            None
        }
    }

    pub fn set(&mut self, x: u32, y: u32, color: Color) {
        if x < self.width && y < self.height {
            let idx = self.index(x, y);
            self.data[idx] = color;
        }
    }

    pub fn pixels(&self) -> &[Color] {
        &self.data
    }

    pub fn pixels_mut(&mut self) -> &mut [Color] {
        &mut self.data
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_transparent() {
        let buf = PixelBuffer::new(4, 4);
        assert_eq!(buf.get(0, 0), Some(Color::TRANSPARENT));
        assert_eq!(buf.get(3, 3), Some(Color::TRANSPARENT));
    }

    #[test]
    fn filled_buffer() {
        let buf = PixelBuffer::filled(2, 2, Color::WHITE);
        assert_eq!(buf.get(0, 0), Some(Color::WHITE));
        assert_eq!(buf.get(1, 1), Some(Color::WHITE));
    }

    #[test]
    fn set_and_get() {
        let mut buf = PixelBuffer::new(4, 4);
        buf.set(2, 3, Color::BLACK);
        assert_eq!(buf.get(2, 3), Some(Color::BLACK));
        assert_eq!(buf.get(0, 0), Some(Color::TRANSPARENT));
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let buf = PixelBuffer::new(4, 4);
        assert_eq!(buf.get(4, 0), None);
        assert_eq!(buf.get(0, 4), None);
    }

    #[test]
    fn out_of_bounds_set_is_noop() {
        let mut buf = PixelBuffer::new(4, 4);
        buf.set(10, 10, Color::WHITE); // should not panic
    }
}
