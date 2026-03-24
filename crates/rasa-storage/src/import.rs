use std::path::Path;

use image::GenericImageView;
use rasa_core::color::{Color, srgb_to_linear};
use rasa_core::document::Document;
use rasa_core::error::RasaError;
use rasa_core::layer::Layer;
use rasa_core::pixel::PixelBuffer;

use crate::format::ImageFormat;

/// Convert raw RGBA u8 bytes into a PixelBuffer using slice access.
///
/// # Panics
/// Panics in debug mode if `raw.len() < width * height * 4`. In release mode,
/// short buffers produce a truncated result (remaining pixels stay transparent).
pub(crate) fn rgba_bytes_to_buffer(raw: &[u8], width: u32, height: u32) -> PixelBuffer {
    let mut buf = PixelBuffer::new(width, height);
    let pixels = buf.pixels_mut();
    let max_pixels = raw.len() / 4;
    for (i, px) in pixels.iter_mut().enumerate() {
        if i >= max_pixels {
            break;
        }
        let offset = i * 4;
        *px = Color::new(
            srgb_to_linear(raw[offset] as f32 / 255.0),
            srgb_to_linear(raw[offset + 1] as f32 / 255.0),
            srgb_to_linear(raw[offset + 2] as f32 / 255.0),
            raw[offset + 3] as f32 / 255.0,
        );
    }
    buf
}

/// Import an image file as a new Document with a single raster layer.
pub fn import_image(path: &Path) -> Result<Document, RasaError> {
    let format = ImageFormat::from_path(path)
        .ok_or_else(|| RasaError::UnsupportedFormat(path.display().to_string()))?;

    match format {
        ImageFormat::Psd => return crate::psd::import_psd(path),
        ImageFormat::Raw => return crate::raw::import_raw(path),
        _ => {}
    }

    let buf = import_as_buffer(path)?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");

    let (width, height) = buf.dimensions();
    let layer = Layer::new_raster(name, width, height);
    let layer_id = layer.id;

    let mut doc = Document::new(name, width, height);
    doc.layers.clear();
    doc.pixel_data.clear();
    doc.layers.push(layer);
    doc.pixel_data.push((layer_id, buf));
    doc.select_layer(layer_id);

    Ok(doc)
}

/// Import an image file as a PixelBuffer (for adding as a layer to an existing document).
pub fn import_as_buffer(path: &Path) -> Result<PixelBuffer, RasaError> {
    let img =
        image::open(path).map_err(|e| RasaError::Other(format!("failed to open image: {e}")))?;

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();
    Ok(rgba_bytes_to_buffer(rgba.as_raw(), width, height))
}

/// Import raw RGBA u8 bytes as a PixelBuffer.
pub fn import_from_rgba_bytes(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<PixelBuffer, RasaError> {
    let expected = (width as usize) * (height as usize) * 4;
    if data.len() != expected {
        return Err(RasaError::InvalidDimensions { width, height });
    }
    Ok(rgba_bytes_to_buffer(data, width, height))
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

    #[test]
    fn import_as_buffer_nonexistent() {
        let result = import_as_buffer(Path::new("/nonexistent/file.png"));
        assert!(result.is_err());
    }
}
