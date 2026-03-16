use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::tiff::TiffEncoder;
use image::{ImageBuffer, ImageEncoder, Rgba};
use rasa_core::color::{IccProfile, linear_to_srgb};
use rasa_core::error::RasaError;
use rasa_core::pixel::PixelBuffer;

use crate::format::{ExportConfig, ExportSettings, ImageFormat};

/// Export a pixel buffer to an image file.
pub fn export_buffer(
    buf: &PixelBuffer,
    path: &Path,
    settings: &ExportSettings,
) -> Result<(), RasaError> {
    export_buffer_with_config(
        buf,
        path,
        &ExportConfig {
            settings: settings.clone(),
            icc_profile: None,
        },
    )
}

/// Export a pixel buffer with full color management configuration.
pub fn export_buffer_with_config(
    buf: &PixelBuffer,
    path: &Path,
    config: &ExportConfig,
) -> Result<(), RasaError> {
    let icc_data = config.icc_profile.as_ref().map(|p| p.data().to_vec());

    match &config.settings {
        ExportSettings::Psd => {
            return crate::psd::export_psd_flat(buf, path);
        }
        ExportSettings::TiffCmyk => {
            return export_tiff_cmyk(buf, path, config.icc_profile.as_ref());
        }
        ExportSettings::Jpeg(quality) => {
            let img = buffer_to_image(buf);
            let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
            let file = std::fs::File::create(path)
                .map_err(|e| RasaError::Other(format!("export failed: {e}")))?;
            let mut encoder =
                JpegEncoder::new_with_quality(std::io::BufWriter::new(file), quality.0);
            if let Some(ref icc) = icc_data {
                let _ = encoder.set_icc_profile(icc.clone());
            }
            encoder
                .encode_image(&rgb_img)
                .map_err(|e| RasaError::Other(format!("JPEG encode failed: {e}")))?;
        }
        ExportSettings::Png => {
            let img = buffer_to_image(buf);
            let (w, h) = buf.dimensions();
            let file = std::fs::File::create(path)
                .map_err(|e| RasaError::Other(format!("export failed: {e}")))?;
            let mut encoder = PngEncoder::new(std::io::BufWriter::new(file));
            if let Some(ref icc) = icc_data {
                let _ = encoder.set_icc_profile(icc.clone());
            }
            encoder
                .write_image(img.as_raw(), w, h, image::ColorType::Rgba8.into())
                .map_err(|e| RasaError::Other(format!("PNG encode failed: {e}")))?;
        }
        ExportSettings::Tiff => {
            let img = buffer_to_image(buf);
            let (w, h) = buf.dimensions();
            let file = std::fs::File::create(path)
                .map_err(|e| RasaError::Other(format!("export failed: {e}")))?;
            let mut encoder = TiffEncoder::new(std::io::BufWriter::new(file));
            if let Some(ref icc) = icc_data {
                let _ = encoder.set_icc_profile(icc.clone());
            }
            encoder
                .write_image(img.as_raw(), w, h, image::ColorType::Rgba8.into())
                .map_err(|e| RasaError::Other(format!("TIFF encode failed: {e}")))?;
        }
        _ => {
            let img = buffer_to_image(buf);
            let format = to_image_format(config.settings.format())?;
            img.save_with_format(path, format)
                .map_err(|e| RasaError::Other(format!("export failed: {e}")))?;
        }
    }

    Ok(())
}

/// Export a pixel buffer as a CMYK TIFF.
///
/// Uses ICC-based conversion if a source profile is provided,
/// otherwise falls back to naive RGB-to-CMYK.
fn export_tiff_cmyk(
    buf: &PixelBuffer,
    path: &Path,
    icc_profile: Option<&IccProfile>,
) -> Result<(), RasaError> {
    use std::io::Write;

    let (width, height) = buf.dimensions();
    let cmyk_data = if let Some(profile) = icc_profile {
        if profile.color_space == rasa_core::color::ProfileColorSpace::Cmyk {
            let srgb = IccProfile::srgb_v2();
            crate::icc::buffer_to_cmyk_icc(buf, &srgb, profile)?
        } else {
            crate::icc::buffer_to_cmyk_naive(buf)
        }
    } else {
        crate::icc::buffer_to_cmyk_naive(buf)
    };

    // Write a minimal TIFF file with CMYK data.
    // TIFF structure: header(8) + IFD + tag data + pixel strips
    let mut out = Vec::new();

    // ── TIFF Header ──
    out.write_all(b"II").map_err(write_err)?; // little-endian
    out.write_all(&42u16.to_le_bytes()).map_err(write_err)?; // magic
    let ifd_offset = 8u32;
    out.write_all(&ifd_offset.to_le_bytes())
        .map_err(write_err)?;

    // ── IFD Entries ──
    // Tags must be in ascending order per TIFF spec.
    let tag_count: u16 = 13;
    let ifd_size = 2 + (tag_count as usize) * 12 + 4; // count + entries + next-IFD
    let bits_offset = (8 + ifd_size) as u32;
    let res_offset = bits_offset + 8; // after BitsPerSample (4 x u16 = 8 bytes)
    let strip_offset = res_offset + 8; // after resolution RATIONAL (2 x u32 = 8 bytes)
    let strip_byte_count = cmyk_data.len() as u32;

    out.write_all(&tag_count.to_le_bytes()).map_err(write_err)?;

    // Helper: write a TIFF IFD entry (12 bytes)
    let mut write_tag = |tag: u16, typ: u16, count: u32, value: u32| -> Result<(), RasaError> {
        out.write_all(&tag.to_le_bytes()).map_err(write_err)?;
        out.write_all(&typ.to_le_bytes()).map_err(write_err)?;
        out.write_all(&count.to_le_bytes()).map_err(write_err)?;
        out.write_all(&value.to_le_bytes()).map_err(write_err)?;
        Ok(())
    };

    // SHORT=3, LONG=4, RATIONAL=5
    write_tag(256, 4, 1, width)?; // ImageWidth
    write_tag(257, 4, 1, height)?; // ImageLength
    write_tag(258, 3, 4, bits_offset)?; // BitsPerSample (offset)
    write_tag(259, 3, 1, 1)?; // Compression = None
    write_tag(262, 3, 1, 5)?; // PhotometricInterpretation = Separated (CMYK)
    write_tag(273, 4, 1, strip_offset)?; // StripOffsets
    write_tag(277, 3, 1, 4)?; // SamplesPerPixel = 4
    write_tag(278, 4, 1, height)?; // RowsPerStrip = all rows
    write_tag(279, 4, 1, strip_byte_count)?; // StripByteCounts
    write_tag(282, 5, 1, res_offset)?; // XResolution (RATIONAL at offset)
    write_tag(283, 5, 1, res_offset)?; // YResolution (same 72 dpi)
    write_tag(296, 3, 1, 2)?; // ResolutionUnit = inch
    write_tag(332, 3, 1, 1)?; // InkSet = CMYK

    // Next IFD offset = 0 (no more IFDs)
    out.write_all(&0u32.to_le_bytes()).map_err(write_err)?;

    // ── BitsPerSample data (4 x u16) ──
    for _ in 0..4 {
        out.write_all(&8u16.to_le_bytes()).map_err(write_err)?;
    }

    // ── Resolution RATIONAL (72/1 as two u32s) ──
    out.write_all(&72u32.to_le_bytes()).map_err(write_err)?; // numerator
    out.write_all(&1u32.to_le_bytes()).map_err(write_err)?; // denominator

    // ── Pixel data ──
    out.write_all(&cmyk_data).map_err(write_err)?;

    std::fs::write(path, &out)
        .map_err(|e| RasaError::Other(format!("failed to write CMYK TIFF: {e}")))?;

    Ok(())
}

fn write_err(e: std::io::Error) -> RasaError {
    RasaError::Other(format!("TIFF write error: {e}"))
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

fn to_image_format(format: ImageFormat) -> Result<image::ImageFormat, RasaError> {
    match format {
        ImageFormat::Png => Ok(image::ImageFormat::Png),
        ImageFormat::Jpeg => Ok(image::ImageFormat::Jpeg),
        ImageFormat::WebP => Ok(image::ImageFormat::WebP),
        ImageFormat::Tiff => Ok(image::ImageFormat::Tiff),
        ImageFormat::Bmp => Ok(image::ImageFormat::Bmp),
        ImageFormat::Gif => Ok(image::ImageFormat::Gif),
        ImageFormat::Psd => Err(RasaError::UnsupportedFormat(
            "PSD export handled separately".into(),
        )),
        ImageFormat::Raw => Err(RasaError::UnsupportedFormat(
            "RAW format is import-only".into(),
        )),
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

    #[test]
    fn export_tiff_cmyk_creates_file() {
        let buf = PixelBuffer::filled(4, 4, Color::new(1.0, 0.0, 0.0, 1.0));
        let dir = std::env::temp_dir().join("rasa_test_export");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_cmyk.tiff");
        export_buffer(&buf, &path, &ExportSettings::TiffCmyk).unwrap();
        assert!(path.exists());

        let data = std::fs::read(&path).unwrap();
        // TIFF little-endian magic
        assert_eq!(&data[..2], b"II");
        assert_eq!(&data[2..4], &42u16.to_le_bytes());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn export_tiff_cmyk_correct_size() {
        let buf = PixelBuffer::filled(8, 6, Color::WHITE);
        let dir = std::env::temp_dir().join("rasa_test_export");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_cmyk_size.tiff");
        export_buffer(&buf, &path, &ExportSettings::TiffCmyk).unwrap();

        let data = std::fs::read(&path).unwrap();
        // Header(8) + IFD(2 + 13*12 + 4) + BitsPerSample(8) + Resolution(8) + pixels(8*6*4)
        let expected = 8 + 2 + (13 * 12) + 4 + 8 + 8 + (8 * 6 * 4);
        assert_eq!(data.len(), expected);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn export_with_icc_profile_png() {
        let buf = PixelBuffer::filled(4, 4, Color::WHITE);
        let dir = std::env::temp_dir().join("rasa_test_export");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_icc.png");

        let config = ExportConfig {
            settings: ExportSettings::Png,
            icc_profile: Some(rasa_core::color::IccProfile::srgb_v2()),
        };
        export_buffer_with_config(&buf, &path, &config).unwrap();
        assert!(path.exists());

        // The file should be larger than a PNG without ICC
        let with_icc_size = std::fs::metadata(&path).unwrap().len();
        export_buffer(&buf, &path, &ExportSettings::Png).unwrap();
        let without_icc_size = std::fs::metadata(&path).unwrap().len();
        assert!(with_icc_size > without_icc_size);

        std::fs::remove_file(&path).ok();
    }
}
