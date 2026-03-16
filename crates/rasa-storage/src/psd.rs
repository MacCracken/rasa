use std::path::Path;

use rasa_core::color::{BlendMode, Color, linear_to_srgb, srgb_to_linear};
use rasa_core::document::Document;
use rasa_core::error::RasaError;
use rasa_core::layer::Layer;
use rasa_core::pixel::PixelBuffer;

use crate::import::rgba_bytes_to_buffer;

/// Map a PSD blend mode (via its Debug name) to the rasa `BlendMode`.
///
/// The `psd` crate's `BlendMode` enum is not publicly re-exported, so we
/// use its `Debug` representation to identify the variant.
fn psd_blend_mode_to_rasa(mode: impl std::fmt::Debug) -> BlendMode {
    let name = format!("{:?}", mode);
    match name.as_str() {
        "Normal" | "PassThrough" => BlendMode::Normal,
        "Multiply" => BlendMode::Multiply,
        "Screen" => BlendMode::Screen,
        "Overlay" => BlendMode::Overlay,
        "Darken" | "DarkerColor" => BlendMode::Darken,
        "Lighten" | "LighterColor" => BlendMode::Lighten,
        "ColorDodge" | "LinearDodge" => BlendMode::ColorDodge,
        "ColorBurn" | "LinearBurn" => BlendMode::ColorBurn,
        "SoftLight" => BlendMode::SoftLight,
        "HardLight" | "VividLight" | "LinearLight" | "PinLight" | "HardMix" => BlendMode::HardLight,
        "Difference" | "Subtract" => BlendMode::Difference,
        "Exclusion" => BlendMode::Exclusion,
        _ => BlendMode::Normal,
    }
}

/// Map a rasa `BlendMode` to its PSD 4-byte key (used for multi-layer export).
#[allow(dead_code)]
fn rasa_blend_mode_to_psd(mode: BlendMode) -> &'static [u8; 4] {
    match mode {
        BlendMode::Normal => b"norm",
        BlendMode::Multiply => b"mul ",
        BlendMode::Screen => b"scrn",
        BlendMode::Overlay => b"over",
        BlendMode::Darken => b"dark",
        BlendMode::Lighten => b"lite",
        BlendMode::ColorDodge => b"div ",
        BlendMode::ColorBurn => b"idiv",
        BlendMode::SoftLight => b"sLit",
        BlendMode::HardLight => b"hLit",
        BlendMode::Difference => b"diff",
        BlendMode::Exclusion => b"smud",
    }
}

/// Import a PSD file as a multi-layer Document.
pub fn import_psd(path: &Path) -> Result<Document, RasaError> {
    let bytes =
        std::fs::read(path).map_err(|e| RasaError::Other(format!("failed to read PSD: {e}")))?;

    let psd = psd::Psd::from_bytes(&bytes)
        .map_err(|e| RasaError::Other(format!("failed to parse PSD: {e}")))?;

    let width = psd.width();
    let height = psd.height();

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");

    let mut doc = Document::new(name, width, height);
    doc.layers.clear();
    doc.pixel_data.clear();

    let psd_layers = psd.layers();

    if psd_layers.is_empty() {
        // No layer info — import the flattened composite as a single layer.
        let composite_rgba = psd
            .flatten_layers_rgba(&|(_, layer)| layer.visible())
            .map_err(|e| RasaError::Other(format!("PSD composite failed: {e}")))?;
        let buf = rgba_bytes_to_buffer(&composite_rgba, width, height);
        let layer = Layer::new_raster(name, width, height);
        let layer_id = layer.id;
        doc.layers.push(layer);
        doc.pixel_data.push((layer_id, buf));
        doc.active_layer = Some(layer_id);
    } else {
        for psd_layer in psd_layers {
            let layer_name = psd_layer.name();
            let layer_width = psd_layer.width() as u32;
            let layer_height = psd_layer.height() as u32;

            // Skip zero-size layers (e.g. group boundaries).
            if layer_width == 0 || layer_height == 0 {
                continue;
            }

            let rgba = psd_layer.rgba();

            // The layer RGBA is the layer's own dimensions; we need to place it
            // in a full-document-size buffer at the correct offset.
            let mut full_buf = PixelBuffer::new(width, height);
            let left = psd_layer.layer_left().max(0) as u32;
            let top = psd_layer.layer_top().max(0) as u32;

            for y in 0..layer_height {
                for x in 0..layer_width {
                    let dst_x = left + x;
                    let dst_y = top + y;
                    if dst_x >= width || dst_y >= height {
                        continue;
                    }
                    let src_idx = ((y * layer_width + x) * 4) as usize;
                    if src_idx + 3 >= rgba.len() {
                        continue;
                    }
                    let color = Color::new(
                        srgb_to_linear(rgba[src_idx] as f32 / 255.0),
                        srgb_to_linear(rgba[src_idx + 1] as f32 / 255.0),
                        srgb_to_linear(rgba[src_idx + 2] as f32 / 255.0),
                        rgba[src_idx + 3] as f32 / 255.0,
                    );
                    full_buf.set(dst_x, dst_y, color);
                }
            }

            let mut layer = Layer::new_raster(layer_name, width, height);
            layer.opacity = psd_layer.opacity() as f32 / 255.0;
            layer.visible = psd_layer.visible();
            layer.blend_mode = psd_blend_mode_to_rasa(psd_layer.blend_mode());

            let layer_id = layer.id;
            doc.layers.push(layer);
            doc.pixel_data.push((layer_id, full_buf));
        }

        if let Some(first) = doc.layers.first() {
            doc.active_layer = Some(first.id);
        }
    }

    Ok(doc)
}

/// Export a flat (composited) pixel buffer as a minimal single-layer PSD file.
///
/// Writes a valid PSD with: file header, empty color-mode & image-resources
/// sections, empty layer/mask section, and raw image data (RGBA channels).
pub fn export_psd_flat(buf: &PixelBuffer, path: &Path) -> Result<(), RasaError> {
    use std::io::Write;

    let (width, height) = buf.dimensions();
    let pixels = buf.pixels();

    // Separate into channels: R, G, B, A — each as sRGB u8.
    let pixel_count = (width as usize) * (height as usize);
    let mut r_chan = Vec::with_capacity(pixel_count);
    let mut g_chan = Vec::with_capacity(pixel_count);
    let mut b_chan = Vec::with_capacity(pixel_count);
    let mut a_chan = Vec::with_capacity(pixel_count);

    for px in pixels {
        r_chan.push((linear_to_srgb(px.r) * 255.0 + 0.5) as u8);
        g_chan.push((linear_to_srgb(px.g) * 255.0 + 0.5) as u8);
        b_chan.push((linear_to_srgb(px.b) * 255.0 + 0.5) as u8);
        a_chan.push((px.a * 255.0 + 0.5) as u8);
    }

    let mut out = Vec::new();

    // ── File Header (26 bytes) ──
    out.write_all(b"8BPS").map_err(write_err)?; // signature
    out.write_all(&1u16.to_be_bytes()).map_err(write_err)?; // version
    out.write_all(&[0u8; 6]).map_err(write_err)?; // reserved
    out.write_all(&4u16.to_be_bytes()).map_err(write_err)?; // channels (RGBA)
    out.write_all(&height.to_be_bytes())
        .map_err(write_err)?;
    out.write_all(&width.to_be_bytes())
        .map_err(write_err)?;
    out.write_all(&8u16.to_be_bytes()).map_err(write_err)?; // depth (8-bit)
    out.write_all(&3u16.to_be_bytes()).map_err(write_err)?; // color mode (RGB)

    // ── Color Mode Data (empty) ──
    out.write_all(&0u32.to_be_bytes()).map_err(write_err)?;

    // ── Image Resources (empty) ──
    out.write_all(&0u32.to_be_bytes()).map_err(write_err)?;

    // ── Layer and Mask Information (empty) ──
    out.write_all(&0u32.to_be_bytes()).map_err(write_err)?;

    // ── Image Data (raw, uncompressed) ──
    out.write_all(&0u16.to_be_bytes()).map_err(write_err)?; // compression = 0 (raw)
    // Channels in order: A, R, G, B for RGBA PSD files.
    out.write_all(&a_chan).map_err(write_err)?;
    out.write_all(&r_chan).map_err(write_err)?;
    out.write_all(&g_chan).map_err(write_err)?;
    out.write_all(&b_chan).map_err(write_err)?;

    std::fs::write(path, &out)
        .map_err(|e| RasaError::Other(format!("failed to write PSD: {e}")))?;

    Ok(())
}

fn write_err(e: std::io::Error) -> RasaError {
    RasaError::Other(format!("PSD write error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to simulate PSD blend mode Debug output for testing.
    struct FakeBlendMode(&'static str);
    impl std::fmt::Debug for FakeBlendMode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    #[test]
    fn blend_mode_mapping_known() {
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Normal")),
            BlendMode::Normal
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Multiply")),
            BlendMode::Multiply
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Screen")),
            BlendMode::Screen
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Overlay")),
            BlendMode::Overlay
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Darken")),
            BlendMode::Darken
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Lighten")),
            BlendMode::Lighten
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("ColorDodge")),
            BlendMode::ColorDodge
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("ColorBurn")),
            BlendMode::ColorBurn
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("SoftLight")),
            BlendMode::SoftLight
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("HardLight")),
            BlendMode::HardLight
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Difference")),
            BlendMode::Difference
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Exclusion")),
            BlendMode::Exclusion
        );
    }

    #[test]
    fn blend_mode_unknown_falls_back() {
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Dissolve")),
            BlendMode::Normal
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Hue")),
            BlendMode::Normal
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("SomeUnknown")),
            BlendMode::Normal
        );
    }

    #[test]
    fn blend_mode_grouped_variants() {
        // DarkerColor maps to Darken
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("DarkerColor")),
            BlendMode::Darken
        );
        // LighterColor maps to Lighten
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("LighterColor")),
            BlendMode::Lighten
        );
        // LinearDodge maps to ColorDodge
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("LinearDodge")),
            BlendMode::ColorDodge
        );
        // LinearBurn maps to ColorBurn
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("LinearBurn")),
            BlendMode::ColorBurn
        );
        // VividLight, PinLight map to HardLight
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("VividLight")),
            BlendMode::HardLight
        );
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("PinLight")),
            BlendMode::HardLight
        );
        // Subtract maps to Difference
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("Subtract")),
            BlendMode::Difference
        );
        // PassThrough maps to Normal
        assert_eq!(
            psd_blend_mode_to_rasa(FakeBlendMode("PassThrough")),
            BlendMode::Normal
        );
    }

    #[test]
    fn rasa_to_psd_key_valid() {
        for mode in [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::ColorDodge,
            BlendMode::ColorBurn,
            BlendMode::SoftLight,
            BlendMode::HardLight,
            BlendMode::Difference,
            BlendMode::Exclusion,
        ] {
            let key = rasa_blend_mode_to_psd(mode);
            assert_eq!(key.len(), 4);
            assert!(std::str::from_utf8(key).is_ok());
        }
    }

    #[test]
    fn export_psd_flat_valid_header() {
        let buf = PixelBuffer::filled(4, 4, Color::WHITE);
        let dir = std::env::temp_dir().join("rasa_test_psd");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_flat.psd");
        export_psd_flat(&buf, &path).unwrap();

        let data = std::fs::read(&path).unwrap();
        // PSD magic
        assert_eq!(&data[..4], b"8BPS");
        // Version 1
        assert_eq!(&data[4..6], &1u16.to_be_bytes());
        // 4 channels
        assert_eq!(&data[12..14], &4u16.to_be_bytes());
        // Height = 4
        assert_eq!(&data[14..18], &4u32.to_be_bytes());
        // Width = 4
        assert_eq!(&data[18..22], &4u32.to_be_bytes());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn export_psd_flat_correct_size() {
        let buf = PixelBuffer::filled(8, 6, Color::new(1.0, 0.0, 0.0, 1.0));
        let dir = std::env::temp_dir().join("rasa_test_psd");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_size.psd");
        export_psd_flat(&buf, &path).unwrap();

        let data = std::fs::read(&path).unwrap();
        // Header(26) + color-mode(4) + image-resources(4) + layer-mask(4) + compression(2) + 4 channels * 48 pixels
        let expected_len = 26 + 4 + 4 + 4 + 2 + (4 * 8 * 6);
        assert_eq!(data.len(), expected_len);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn import_psd_nonexistent() {
        let result = import_psd(Path::new("/nonexistent/file.psd"));
        assert!(result.is_err());
    }

    #[test]
    fn import_psd_corrupt() {
        let dir = std::env::temp_dir().join("rasa_test_psd");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corrupt.psd");
        std::fs::write(&path, b"not a psd file").unwrap();
        let result = import_psd(&path);
        assert!(result.is_err());
        std::fs::remove_file(&path).ok();
    }
}
