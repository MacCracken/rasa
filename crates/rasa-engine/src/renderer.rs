use rasa_core::Document;
use rasa_core::color::{Color, ColorSpace, linear_to_srgb};
use rasa_core::geometry::Rect;
use rasa_core::pixel::PixelBuffer;

use crate::compositor;

/// Tile size for tile-based rendering.
const TILE_SIZE: u32 = 256;

/// Rendered output from the document renderer.
pub struct RenderOutput {
    pub buffer: PixelBuffer,
    pub color_space: ColorSpace,
}

/// Render a document to a final pixel buffer.
pub fn render(doc: &Document, target_space: ColorSpace) -> RenderOutput {
    let composited = compositor::composite(doc);

    let buffer = match target_space {
        ColorSpace::LinearRgb => composited,
        ColorSpace::Srgb => convert_linear_to_srgb(&composited),
        ColorSpace::DisplayP3 => {
            // Approximate: sRGB transfer function for now, proper P3 gamut mapping is post-MVP
            convert_linear_to_srgb(&composited)
        }
    };

    RenderOutput {
        buffer,
        color_space: target_space,
    }
}

/// Render only a rectangular region of the document (for partial updates).
pub fn render_region(doc: &Document, region: Rect, target_space: ColorSpace) -> RenderOutput {
    let full = compositor::composite(doc);

    let x0 = (region.x as u32).min(full.width);
    let y0 = (region.y as u32).min(full.height);
    let x1 = ((region.x + region.width) as u32).min(full.width);
    let y1 = ((region.y + region.height) as u32).min(full.height);
    let w = x1.saturating_sub(x0);
    let h = y1.saturating_sub(y0);

    if w == 0 || h == 0 {
        return RenderOutput {
            buffer: PixelBuffer::new(0, 0),
            color_space: target_space,
        };
    }

    let mut output = PixelBuffer::new(w, h);
    for dy in 0..h {
        for dx in 0..w {
            if let Some(px) = full.get(x0 + dx, y0 + dy) {
                let px = match target_space {
                    ColorSpace::LinearRgb => px,
                    _ => Color::new(
                        linear_to_srgb(px.r),
                        linear_to_srgb(px.g),
                        linear_to_srgb(px.b),
                        px.a,
                    ),
                };
                output.set(dx, dy, px);
            }
        }
    }

    RenderOutput {
        buffer: output,
        color_space: target_space,
    }
}

/// Get the list of tile coordinates for a document.
pub fn tile_coords(width: u32, height: u32) -> Vec<Rect> {
    let mut tiles = Vec::new();
    let mut y = 0;
    while y < height {
        let mut x = 0;
        let th = TILE_SIZE.min(height - y);
        while x < width {
            let tw = TILE_SIZE.min(width - x);
            tiles.push(Rect {
                x: x as f64,
                y: y as f64,
                width: tw as f64,
                height: th as f64,
            });
            x += TILE_SIZE;
        }
        y += TILE_SIZE;
    }
    tiles
}

/// Convert an entire buffer from linear RGB to sRGB.
fn convert_linear_to_srgb(buf: &PixelBuffer) -> PixelBuffer {
    let (w, h) = buf.dimensions();
    let mut output = PixelBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let px = buf.get(x, y).unwrap();
            output.set(
                x,
                y,
                Color::new(
                    linear_to_srgb(px.r),
                    linear_to_srgb(px.g),
                    linear_to_srgb(px.b),
                    px.a,
                ),
            );
        }
    }
    output
}

/// Convert a rendered buffer to packed RGBA u8 bytes (for display or export).
pub fn to_rgba_bytes(output: &RenderOutput) -> Vec<u8> {
    let (w, h) = output.buffer.dimensions();
    let mut bytes = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let px = output.buffer.get(x, y).unwrap();
            // If already in sRGB, just quantize. If linear, convert first.
            let [r, g, b, a] = match output.color_space {
                ColorSpace::LinearRgb => {
                    let srgb = Color::new(
                        linear_to_srgb(px.r),
                        linear_to_srgb(px.g),
                        linear_to_srgb(px.b),
                        px.a,
                    );
                    srgb.to_srgb_u8()
                }
                _ => {
                    // Already in sRGB-like space, just quantize
                    [
                        (px.r.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                        (px.g.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                        (px.b.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                        (px.a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                    ]
                }
            };
            bytes.push(r);
            bytes.push(g);
            bytes.push(b);
            bytes.push(a);
        }
    }
    bytes
}

/// Render cache: tracks which tiles are dirty and need re-rendering.
pub struct RenderCache {
    tiles: Vec<CachedTile>,
}

struct CachedTile {
    rect: Rect,
    dirty: bool,
    buffer: Option<PixelBuffer>,
}

impl RenderCache {
    pub fn new(width: u32, height: u32) -> Self {
        let rects = tile_coords(width, height);
        let tiles = rects
            .into_iter()
            .map(|rect| CachedTile {
                rect,
                dirty: true,
                buffer: None,
            })
            .collect();
        Self { tiles }
    }

    /// Mark all tiles as dirty.
    pub fn invalidate_all(&mut self) {
        for tile in &mut self.tiles {
            tile.dirty = true;
            tile.buffer = None;
        }
    }

    /// Mark tiles overlapping a region as dirty.
    pub fn invalidate_region(&mut self, region: Rect) {
        for tile in &mut self.tiles {
            if rects_overlap(&tile.rect, &region) {
                tile.dirty = true;
                tile.buffer = None;
            }
        }
    }

    /// Get indices of dirty tiles.
    pub fn dirty_tiles(&self) -> Vec<usize> {
        self.tiles
            .iter()
            .enumerate()
            .filter(|(_, t)| t.dirty)
            .map(|(i, _)| i)
            .collect()
    }

    /// Store a rendered tile and mark it clean.
    pub fn store_tile(&mut self, index: usize, buffer: PixelBuffer) {
        if let Some(tile) = self.tiles.get_mut(index) {
            tile.buffer = Some(buffer);
            tile.dirty = false;
        }
    }

    /// Get a tile's rect.
    pub fn tile_rect(&self, index: usize) -> Option<Rect> {
        self.tiles.get(index).map(|t| t.rect)
    }

    /// Number of tiles.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Check if all tiles are clean.
    pub fn is_clean(&self) -> bool {
        self.tiles.iter().all(|t| !t.dirty)
    }
}

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.02
    }

    // ── Full render ──

    #[test]
    fn render_linear_preserves_values() {
        let doc = Document::new("Test", 4, 4);
        let output = render(&doc, ColorSpace::LinearRgb);
        let px = output.buffer.get(0, 0).unwrap();
        // White background in linear space
        assert!(approx_eq(px.r, 1.0));
        assert!(approx_eq(px.a, 1.0));
    }

    #[test]
    fn render_srgb_converts() {
        let doc = Document::new("Test", 4, 4);
        let output = render(&doc, ColorSpace::Srgb);
        let px = output.buffer.get(0, 0).unwrap();
        // White in linear -> white in sRGB (1.0 maps to 1.0)
        assert!(approx_eq(px.r, 1.0));
    }

    // ── Region render ──

    #[test]
    fn render_region_extracts_subimage() {
        let doc = Document::new("Test", 100, 100);
        let region = Rect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        };
        let output = render_region(&doc, region, ColorSpace::Srgb);
        assert_eq!(output.buffer.dimensions(), (20, 20));
    }

    #[test]
    fn render_region_clamps_to_bounds() {
        let doc = Document::new("Test", 50, 50);
        let region = Rect {
            x: 40.0,
            y: 40.0,
            width: 30.0,
            height: 30.0,
        };
        let output = render_region(&doc, region, ColorSpace::Srgb);
        assert_eq!(output.buffer.dimensions(), (10, 10));
    }

    // ── Tile coords ──

    #[test]
    fn tile_coords_small_image() {
        let tiles = tile_coords(100, 100);
        assert_eq!(tiles.len(), 1);
        assert!(approx_eq(tiles[0].width as f32, 100.0));
    }

    #[test]
    fn tile_coords_exact_multiple() {
        let tiles = tile_coords(512, 512);
        assert_eq!(tiles.len(), 4); // 2x2 tiles of 256
    }

    #[test]
    fn tile_coords_non_multiple() {
        let tiles = tile_coords(300, 300);
        assert_eq!(tiles.len(), 4); // 2x2: (256,256), (44,256), (256,44), (44,44)
        assert!(approx_eq(tiles[0].width as f32, 256.0));
        assert!(approx_eq(tiles[1].width as f32, 44.0));
    }

    // ── RGBA bytes ──

    #[test]
    fn to_rgba_bytes_length() {
        let doc = Document::new("Test", 4, 4);
        let output = render(&doc, ColorSpace::Srgb);
        let bytes = to_rgba_bytes(&output);
        assert_eq!(bytes.len(), 4 * 4 * 4);
    }

    #[test]
    fn to_rgba_bytes_white() {
        let doc = Document::new("Test", 1, 1);
        let output = render(&doc, ColorSpace::Srgb);
        let bytes = to_rgba_bytes(&output);
        assert_eq!(bytes, vec![255, 255, 255, 255]);
    }

    // ── Render cache ──

    #[test]
    fn cache_starts_all_dirty() {
        let cache = RenderCache::new(512, 512);
        assert_eq!(cache.tile_count(), 4);
        assert_eq!(cache.dirty_tiles().len(), 4);
        assert!(!cache.is_clean());
    }

    #[test]
    fn cache_store_cleans_tile() {
        let mut cache = RenderCache::new(100, 100);
        assert_eq!(cache.dirty_tiles().len(), 1);
        cache.store_tile(0, PixelBuffer::new(100, 100));
        assert_eq!(cache.dirty_tiles().len(), 0);
        assert!(cache.is_clean());
    }

    #[test]
    fn cache_invalidate_region() {
        let mut cache = RenderCache::new(512, 512);
        // Clean all tiles
        for i in 0..cache.tile_count() {
            cache.store_tile(i, PixelBuffer::new(256, 256));
        }
        assert!(cache.is_clean());

        // Invalidate top-left corner
        cache.invalidate_region(Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        });
        assert_eq!(cache.dirty_tiles().len(), 1);
        assert_eq!(cache.dirty_tiles()[0], 0);
    }

    #[test]
    fn cache_invalidate_all() {
        let mut cache = RenderCache::new(512, 512);
        for i in 0..cache.tile_count() {
            cache.store_tile(i, PixelBuffer::new(256, 256));
        }
        cache.invalidate_all();
        assert_eq!(cache.dirty_tiles().len(), 4);
    }

    #[test]
    fn rects_overlap_cases() {
        let a = Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let b = Rect {
            x: 5.0,
            y: 5.0,
            width: 10.0,
            height: 10.0,
        };
        assert!(rects_overlap(&a, &b));

        let c = Rect {
            x: 20.0,
            y: 20.0,
            width: 5.0,
            height: 5.0,
        };
        assert!(!rects_overlap(&a, &c));
    }
}
