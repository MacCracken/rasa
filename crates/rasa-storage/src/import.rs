use std::path::Path;

use image::GenericImageView;
use rasa_core::color::{Color, srgb_to_linear};
use rasa_core::document::Document;
use rasa_core::error::RasaError;
use rasa_core::layer::Layer;
use rasa_core::pixel::PixelBuffer;

use crate::format::ImageFormat;

/// Import an image file as a new Document with a single raster layer.
pub fn import_image(path: &Path) -> Result<Document, RasaError> {
    let _format = ImageFormat::from_path(path)
        .ok_or_else(|| RasaError::UnsupportedFormat(path.display().to_string()))?;

    let img =
        image::open(path).map_err(|e| RasaError::Other(format!("failed to open image: {e}")))?;

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();

    let mut pixel_buf = PixelBuffer::new(width, height);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let color = Color::new(
            srgb_to_linear(r as f32 / 255.0),
            srgb_to_linear(g as f32 / 255.0),
            srgb_to_linear(b as f32 / 255.0),
            a as f32 / 255.0,
        );
        pixel_buf.set(x, y, color);
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");

    let layer = Layer::new_raster(name, width, height);
    let layer_id = layer.id;

    let mut doc = Document::new(name, width, height);
    // Replace the default background layer with our imported data
    doc.layers.clear();
    doc.pixel_data.clear();
    doc.layers.push(layer);
    doc.pixel_data.push((layer_id, pixel_buf));
    doc.active_layer = Some(layer_id);

    Ok(doc)
}

/// Import an image file as a PixelBuffer (for adding as a layer to an existing document).
pub fn import_as_buffer(path: &Path) -> Result<PixelBuffer, RasaError> {
    let img =
        image::open(path).map_err(|e| RasaError::Other(format!("failed to open image: {e}")))?;

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();

    let mut pixel_buf = PixelBuffer::new(width, height);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let color = Color::new(
            srgb_to_linear(r as f32 / 255.0),
            srgb_to_linear(g as f32 / 255.0),
            srgb_to_linear(b as f32 / 255.0),
            a as f32 / 255.0,
        );
        pixel_buf.set(x, y, color);
    }

    Ok(pixel_buf)
}

/// Import raw RGBA u8 bytes as a PixelBuffer.
pub fn import_from_rgba_bytes(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<PixelBuffer, RasaError> {
    let expected = (width as usize) * (height as usize) * 4;
    if data.len() != expected {
        return Err(RasaError::Other(format!(
            "expected {expected} bytes for {width}x{height} RGBA, got {}",
            data.len()
        )));
    }

    let mut buf = PixelBuffer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let i = ((y as usize) * (width as usize) + (x as usize)) * 4;
            let color = Color::new(
                srgb_to_linear(data[i] as f32 / 255.0),
                srgb_to_linear(data[i + 1] as f32 / 255.0),
                srgb_to_linear(data[i + 2] as f32 / 255.0),
                data[i + 3] as f32 / 255.0,
            );
            buf.set(x, y, color);
        }
    }

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_rgba_bytes_correct_size() {
        let data = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let buf = import_from_rgba_bytes(&data, 2, 2).unwrap();
        assert_eq!(buf.dimensions(), (2, 2));
        // First pixel should be red
        let px = buf.get(0, 0).unwrap();
        assert!(px.r > 0.9);
        assert!(px.g < 0.01);
    }

    #[test]
    fn import_rgba_bytes_wrong_size() {
        let data = vec![0; 10];
        let result = import_from_rgba_bytes(&data, 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn import_nonexistent_file() {
        let result = import_image(Path::new("/nonexistent/file.png"));
        assert!(result.is_err());
    }

    #[test]
    fn import_unsupported_format() {
        let result = import_image(Path::new("file.xyz"));
        assert!(result.is_err());
    }
}
