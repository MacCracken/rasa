use std::path::Path;

use rasa_core::color::{Color, srgb_to_linear};
use rasa_core::document::Document;
use rasa_core::error::RasaError;
use rasa_core::layer::Layer;
use rasa_core::pixel::PixelBuffer;

/// Import a camera RAW file (CR2, NEF, ARW, DNG, RAF, ORF, RW2) as a Document.
///
/// Uses the `imagepipe` crate to demosaic, white-balance, and tone-map the
/// raw sensor data into a usable sRGB image, then converts to linear f32.
pub fn import_raw(path: &Path) -> Result<Document, RasaError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| RasaError::Other("invalid path encoding".into()))?;

    let mut pipeline = imagepipe::Pipeline::new_from_file(path_str)
        .map_err(|e| RasaError::Other(format!("RAW decode failed: {e}")))?;

    // Use 16-bit output for maximum tonal range, then convert to f32.
    // imagepipe outputs 3-channel (RGB) data.
    let decoded = pipeline
        .output_16bit(None)
        .map_err(|e| RasaError::Other(format!("RAW processing failed: {e}")))?;

    let width = u32::try_from(decoded.width)
        .map_err(|_| RasaError::Other("RAW image width exceeds u32".into()))?;
    let height = u32::try_from(decoded.height)
        .map_err(|_| RasaError::Other("RAW image height exceeds u32".into()))?;

    let mut buf = PixelBuffer::new(width, height);
    let pixels = buf.pixels_mut();
    let expected_len = pixels.len() * 3;
    if decoded.data.len() < expected_len {
        return Err(RasaError::Other(format!(
            "RAW decoded data too short: expected {} bytes, got {}",
            expected_len,
            decoded.data.len()
        )));
    }

    for (i, px) in pixels.iter_mut().enumerate() {
        let offset = i * 3;
        let r = decoded.data[offset] as f32 / 65535.0;
        let g = decoded.data[offset + 1] as f32 / 65535.0;
        let b = decoded.data[offset + 2] as f32 / 65535.0;

        *px = Color::new(srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0);
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_raw_nonexistent() {
        let result = import_raw(Path::new("/nonexistent/photo.cr2"));
        assert!(result.is_err());
    }

    #[test]
    fn import_raw_invalid_file() {
        let dir = std::env::temp_dir().join("rasa_test_raw");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fake.cr2");
        std::fs::write(&path, b"not a raw file").unwrap();
        let result = import_raw(&path);
        assert!(result.is_err());
        std::fs::remove_file(&path).ok();
    }
}
