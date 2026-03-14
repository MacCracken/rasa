use std::path::Path;

use serde::{Deserialize, Serialize};

/// Supported image formats for import/export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
    WebP,
    Tiff,
    Bmp,
    Gif,
}

impl ImageFormat {
    /// Detect format from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::WebP),
            "tiff" | "tif" => Some(Self::Tiff),
            "bmp" => Some(Self::Bmp),
            "gif" => Some(Self::Gif),
            _ => None,
        }
    }

    /// Get the default file extension for this format.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
            Self::Tiff => "tiff",
            Self::Bmp => "bmp",
            Self::Gif => "gif",
        }
    }

    /// Whether this format supports alpha/transparency.
    pub fn supports_alpha(self) -> bool {
        matches!(self, Self::Png | Self::WebP | Self::Tiff | Self::Gif)
    }
}

/// JPEG export quality (1-100).
#[derive(Debug, Clone, Copy)]
pub struct JpegQuality(pub u8);

impl Default for JpegQuality {
    fn default() -> Self {
        Self(90)
    }
}

impl JpegQuality {
    pub fn new(quality: u8) -> Self {
        Self(quality.clamp(1, 100))
    }
}

/// Export settings for various formats.
#[derive(Debug, Clone)]
pub enum ExportSettings {
    Png,
    Jpeg(JpegQuality),
    WebP,
    Tiff,
    Bmp,
    Gif,
}

impl ExportSettings {
    pub fn for_format(format: ImageFormat) -> Self {
        match format {
            ImageFormat::Png => Self::Png,
            ImageFormat::Jpeg => Self::Jpeg(JpegQuality::default()),
            ImageFormat::WebP => Self::WebP,
            ImageFormat::Tiff => Self::Tiff,
            ImageFormat::Bmp => Self::Bmp,
            ImageFormat::Gif => Self::Gif,
        }
    }

    pub fn format(&self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg(_) => ImageFormat::Jpeg,
            Self::WebP => ImageFormat::WebP,
            Self::Tiff => ImageFormat::Tiff,
            Self::Bmp => ImageFormat::Bmp,
            Self::Gif => ImageFormat::Gif,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_png() {
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.png")),
            Some(ImageFormat::Png)
        );
    }

    #[test]
    fn detect_jpeg_variants() {
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.jpg")),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.jpeg")),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.JPEG")),
            Some(ImageFormat::Jpeg)
        );
    }

    #[test]
    fn detect_webp() {
        assert_eq!(
            ImageFormat::from_path(Path::new("image.webp")),
            Some(ImageFormat::WebP)
        );
    }

    #[test]
    fn detect_tiff() {
        assert_eq!(
            ImageFormat::from_path(Path::new("scan.tiff")),
            Some(ImageFormat::Tiff)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("scan.tif")),
            Some(ImageFormat::Tiff)
        );
    }

    #[test]
    fn unknown_extension() {
        assert_eq!(ImageFormat::from_path(Path::new("file.xyz")), None);
    }

    #[test]
    fn no_extension() {
        assert_eq!(ImageFormat::from_path(Path::new("file")), None);
    }

    #[test]
    fn alpha_support() {
        assert!(ImageFormat::Png.supports_alpha());
        assert!(!ImageFormat::Jpeg.supports_alpha());
        assert!(ImageFormat::WebP.supports_alpha());
    }

    #[test]
    fn jpeg_quality_clamps() {
        assert_eq!(JpegQuality::new(0).0, 1);
        assert_eq!(JpegQuality::new(150).0, 100);
        assert_eq!(JpegQuality::new(85).0, 85);
    }
}
