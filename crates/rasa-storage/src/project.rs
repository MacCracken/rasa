use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use rasa_core::color::{Color, linear_to_srgb, srgb_to_linear};
use rasa_core::document::Document;
use rasa_core::error::RasaError;
use rasa_core::layer::LayerKind;
use rasa_core::pixel::PixelBuffer;
use serde::{Deserialize, Serialize};

/// Native .rasa project file format.
///
/// Structure:
/// - JSON header (document metadata, layer tree)
/// - Raw pixel data for each raster layer (RGBA u8, sequentially)
///
/// The header is length-prefixed (u64 LE) followed by JSON bytes.
/// Then for each raster layer (in order), width*height*4 bytes of RGBA u8 data.
const MAGIC: &[u8; 4] = b"RASA";
const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct ProjectHeader {
    version: u32,
    document: Document,
    /// Layer IDs that have pixel data, in order they appear in the data section.
    pixel_layers: Vec<uuid::Uuid>,
}

/// Save a document as a .rasa project file.
pub fn save(doc: &Document, path: &Path) -> Result<(), RasaError> {
    let pixel_layers: Vec<uuid::Uuid> = doc
        .layers
        .iter()
        .filter(|l| matches!(l.kind, LayerKind::Raster { .. }))
        .map(|l| l.id)
        .collect();

    let header = ProjectHeader {
        version: FORMAT_VERSION,
        document: doc.clone(),
        pixel_layers: pixel_layers.clone(),
    };

    let header_json = serde_json::to_vec(&header)
        .map_err(|e| RasaError::Other(format!("serialize failed: {e}")))?;

    let mut file = fs::File::create(path)?;

    // Write magic
    file.write_all(MAGIC)?;

    // Write header length + header
    let header_len = header_json.len() as u64;
    file.write_all(&header_len.to_le_bytes())?;
    file.write_all(&header_json)?;

    // Write pixel data for each raster layer
    for layer_id in &pixel_layers {
        if let Some(buf) = doc.get_pixels(*layer_id) {
            let (w, h) = buf.dimensions();
            for y in 0..h {
                for x in 0..w {
                    let px = buf.get(x, y).unwrap();
                    let r = (linear_to_srgb(px.r) * 255.0 + 0.5) as u8;
                    let g = (linear_to_srgb(px.g) * 255.0 + 0.5) as u8;
                    let b = (linear_to_srgb(px.b) * 255.0 + 0.5) as u8;
                    let a = (px.a * 255.0 + 0.5) as u8;
                    file.write_all(&[r, g, b, a])?;
                }
            }
        }
    }

    Ok(())
}

/// Load a document from a .rasa project file.
pub fn load(path: &Path) -> Result<Document, RasaError> {
    let mut file = fs::File::open(path)?;

    // Read and verify magic
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(RasaError::UnsupportedFormat("not a .rasa file".into()));
    }

    // Read header
    let mut len_buf = [0u8; 8];
    file.read_exact(&mut len_buf)?;
    let header_len = u64::from_le_bytes(len_buf) as usize;

    let mut header_json = vec![0u8; header_len];
    file.read_exact(&mut header_json)?;

    let header: ProjectHeader = serde_json::from_slice(&header_json)
        .map_err(|e| RasaError::Other(format!("deserialize failed: {e}")))?;

    let mut doc = header.document;
    doc.pixel_data.clear();

    // Read pixel data for each raster layer
    for layer_id in &header.pixel_layers {
        if let Some(layer) = doc.find_layer(*layer_id)
            && let LayerKind::Raster { width, height } = layer.kind
        {
            let pixel_count = (width as usize) * (height as usize);
            let mut rgba_bytes = vec![0u8; pixel_count * 4];
            file.read_exact(&mut rgba_bytes)?;

            let mut buf = PixelBuffer::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let i = ((y as usize) * (width as usize) + (x as usize)) * 4;
                    let color = Color::new(
                        srgb_to_linear(rgba_bytes[i] as f32 / 255.0),
                        srgb_to_linear(rgba_bytes[i + 1] as f32 / 255.0),
                        srgb_to_linear(rgba_bytes[i + 2] as f32 / 255.0),
                        rgba_bytes[i + 3] as f32 / 255.0,
                    );
                    buf.set(x, y, color);
                }
            }
            doc.pixel_data.push((*layer_id, buf));
        }
    }

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::layer::Layer;

    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("rasa_test_project");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut doc = Document::new("Test Project", 8, 8);
        // Paint some pixels on background
        let bg_id = doc.layers[0].id;
        if let Some(buf) = doc.get_pixels_mut(bg_id) {
            buf.set(0, 0, Color::new(1.0, 0.0, 0.0, 1.0));
            buf.set(1, 0, Color::new(0.0, 1.0, 0.0, 1.0));
        }

        // Add a second layer
        let l2 = Layer::new_raster("Layer 1", 8, 8);
        doc.add_layer(l2);

        let path = test_dir().join("roundtrip.rasa");
        save(&doc, &path).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.name, "Test Project");
        assert_eq!(loaded.size.width, 8);
        assert_eq!(loaded.size.height, 8);
        assert_eq!(loaded.layers.len(), 2);
        assert_eq!(loaded.layers[0].name, "Background");
        assert_eq!(loaded.layers[1].name, "Layer 1");

        // Verify pixel data survived
        let bg_pixels = loaded.get_pixels(loaded.layers[0].id).unwrap();
        let px = bg_pixels.get(0, 0).unwrap();
        assert!((px.r - 1.0).abs() < 0.02); // red pixel preserved (within u8 quantization)
        assert!(px.g < 0.02);

        fs::remove_file(&path).ok();
    }

    #[test]
    fn load_invalid_magic() {
        let path = test_dir().join("bad_magic.rasa");
        fs::write(&path, b"NOPE").unwrap();
        let result = load(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn load_nonexistent() {
        let result = load(Path::new("/nonexistent/file.rasa"));
        assert!(result.is_err());
    }

    #[test]
    fn save_creates_file() {
        let doc = Document::new("Empty", 4, 4);
        let path = test_dir().join("created.rasa");
        save(&doc, &path).unwrap();
        assert!(path.exists());
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 4); // at least magic + header
        fs::remove_file(&path).ok();
    }

    #[test]
    fn roundtrip_preserves_layer_properties() {
        let mut doc = Document::new("Props", 4, 4);
        let l = Layer::new_raster("Custom", 4, 4);
        let lid = l.id;
        doc.add_layer(l);
        doc.set_layer_opacity(lid, 0.6).unwrap();
        doc.set_layer_blend_mode(lid, rasa_core::color::BlendMode::Multiply)
            .unwrap();
        doc.rename_layer(lid, "Renamed").unwrap();

        let path = test_dir().join("props.rasa");
        save(&doc, &path).unwrap();
        let loaded = load(&path).unwrap();

        let loaded_layer = &loaded.layers[1];
        assert_eq!(loaded_layer.name, "Renamed");
        assert!((loaded_layer.opacity - 0.6).abs() < 0.01);
        assert_eq!(
            loaded_layer.blend_mode,
            rasa_core::color::BlendMode::Multiply
        );
        fs::remove_file(&path).ok();
    }
}
