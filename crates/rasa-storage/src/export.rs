use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{ImageBuffer, Rgba};
use rasa_core::color::linear_to_srgb;
use rasa_core::error::RasaError;
use rasa_core::pixel::PixelBuffer;

use crate::format::{ExportSettings, ImageFormat};

/// Export a pixel buffer to an image file.
pub fn export_buffer(
    buf: &PixelBuffer,
    path: &Path,
    settings: &ExportSettings,
) -> Result<(), RasaError> {
    let img = buffer_to_image(buf);

    match settings {
        ExportSettings::Jpeg(quality) => {
            let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
            let file = std::fs::File::create(path)
                .map_err(|e| RasaError::Other(format!("export failed: {e}")))?;
            let mut encoder =
                JpegEncoder::new_with_quality(std::io::BufWriter::new(file), quality.0);
            encoder
                .encode_image(&rgb_img)
                .map_err(|e| RasaError::Other(format!("JPEG encode failed: {e}")))?;
        }
        _ => {
            let format = to_image_format(settings.format());
            img.save_with_format(path, format)
                .map_err(|e| RasaError::Other(format!("export failed: {e}")))?;
        }
    }

    Ok(())
}

/// Export a pixel buffer to in-memory bytes (PNG format).
pub fn export_to_png_bytes(buf: &PixelBuffer) -> Result<Vec<u8>, RasaError> {
    let img = buffer_to_image(buf);
    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| RasaError::Other(format!("PNG encode failed: {e}")))?;
    Ok(bytes)
}

/// Export a pixel buffer to RGBA u8 bytes (raw, no encoding).
pub fn export_to_rgba_bytes(buf: &PixelBuffer) -> Vec<u8> {
    let pixels = buf.pixels();
    let mut bytes = Vec::with_capacity(pixels.len() * 4);
    for px in pixels {
        bytes.push((linear_to_srgb(px.r) * 255.0 + 0.5) as u8);
        bytes.push((linear_to_srgb(px.g) * 255.0 + 0.5) as u8);
        bytes.push((linear_to_srgb(px.b) * 255.0 + 0.5) as u8);
        bytes.push((px.a * 255.0 + 0.5) as u8);
    }
    bytes
}

fn buffer_to_image(buf: &PixelBuffer) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (w, h) = buf.dimensions();
    let raw = export_to_rgba_bytes(buf);
    ImageBuffer::from_raw(w, h, raw).expect("buffer dimensions match")
}

fn to_image_format(format: ImageFormat) -> image::ImageFormat {
    match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::WebP => image::ImageFormat::WebP,
        ImageFormat::Tiff => image::ImageFormat::Tiff,
        ImageFormat::Bmp => image::ImageFormat::Bmp,
        ImageFormat::Gif => image::ImageFormat::Gif,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::JpegQuality;
    use rasa_core::color::Color;

    #[test]
    fn export_rgba_bytes_length() {
        let buf = PixelBuffer::filled(4, 4, Color::WHITE);
        let bytes = export_to_rgba_bytes(&buf);
        assert_eq!(bytes.len(), 4 * 4 * 4);
    }

    #[test]
    fn export_rgba_bytes_white() {
        let buf = PixelBuffer::filled(1, 1, Color::WHITE);
        let bytes = export_to_rgba_bytes(&buf);
        assert_eq!(bytes, vec![255, 255, 255, 255]);
    }

    #[test]
    fn export_rgba_bytes_transparent() {
        let buf = PixelBuffer::new(1, 1); // transparent
        let bytes = export_to_rgba_bytes(&buf);
        assert_eq!(bytes, vec![0, 0, 0, 0]);
    }

    #[test]
    fn export_png_bytes_valid() {
        let buf = PixelBuffer::filled(2, 2, Color::new(1.0, 0.0, 0.0, 1.0));
        let png_data = export_to_png_bytes(&buf).unwrap();
        // PNG magic bytes
        assert_eq!(&png_data[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn export_to_file_png() {
        let buf = PixelBuffer::filled(4, 4, Color::new(0.0, 0.5, 1.0, 1.0));
        let dir = std::env::temp_dir().join("rasa_test_export");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_export.png");
        export_buffer(&buf, &path, &ExportSettings::Png).unwrap();
        assert!(path.exists());
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn export_to_file_jpeg() {
        let buf = PixelBuffer::filled(4, 4, Color::WHITE);
        let dir = std::env::temp_dir().join("rasa_test_export");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_export.jpg");
        export_buffer(&buf, &path, &ExportSettings::Jpeg(JpegQuality::new(85))).unwrap();
        assert!(path.exists());
        std::fs::remove_file(&path).ok();
    }
}
