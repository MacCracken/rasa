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
    Psd,
    Raw,
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
            "psd" => Some(Self::Psd),
            "cr2" | "nef" | "arw" | "dng" | "raf" | "orf" | "rw2" => Some(Self::Raw),
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
            Self::Psd => "psd",
            Self::Raw => "dng",
        }
    }

    /// Whether this format can be used for export.
    pub fn is_exportable(self) -> bool {
        !matches!(self, Self::Raw)
    }

    /// Whether this format supports alpha/transparency.
    pub fn supports_alpha(self) -> bool {
        matches!(
            self,
            Self::Png | Self::WebP | Self::Tiff | Self::Gif | Self::Psd
        )
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
    /// CMYK TIFF export (4-channel separated).
    TiffCmyk,
    Bmp,
    Gif,
    Psd,
}

/// Full export configuration including color management.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub settings: ExportSettings,
    /// Optional ICC profile to embed in the output file.
    pub icc_profile: Option<rasa_core::color::IccProfile>,
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
            ImageFormat::Psd => Self::Psd,
            ImageFormat::Raw => {
                panic!("RAW format is import-only, cannot export")
            }
        }
    }

    pub fn format(&self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg(_) => ImageFormat::Jpeg,
            Self::WebP => ImageFormat::WebP,
            Self::Tiff | Self::TiffCmyk => ImageFormat::Tiff,
            Self::Bmp => ImageFormat::Bmp,
            Self::Gif => ImageFormat::Gif,
            Self::Psd => ImageFormat::Psd,
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

    #[test]
    fn jpeg_quality_default() {
        assert_eq!(JpegQuality::default().0, 90);
    }

    #[test]
    fn extension_returns_correct() {
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::WebP.extension(), "webp");
        assert_eq!(ImageFormat::Tiff.extension(), "tiff");
        assert_eq!(ImageFormat::Bmp.extension(), "bmp");
        assert_eq!(ImageFormat::Gif.extension(), "gif");
    }

    #[test]
    fn export_settings_for_format() {
        let s = ExportSettings::for_format(ImageFormat::Png);
        assert_eq!(s.format(), ImageFormat::Png);
        let s = ExportSettings::for_format(ImageFormat::Jpeg);
        assert_eq!(s.format(), ImageFormat::Jpeg);
        let s = ExportSettings::for_format(ImageFormat::WebP);
        assert_eq!(s.format(), ImageFormat::WebP);
        let s = ExportSettings::for_format(ImageFormat::Tiff);
        assert_eq!(s.format(), ImageFormat::Tiff);
        let s = ExportSettings::for_format(ImageFormat::Bmp);
        assert_eq!(s.format(), ImageFormat::Bmp);
        let s = ExportSettings::for_format(ImageFormat::Gif);
        assert_eq!(s.format(), ImageFormat::Gif);
    }

    #[test]
    fn detect_bmp_gif() {
        assert_eq!(
            ImageFormat::from_path(Path::new("img.bmp")),
            Some(ImageFormat::Bmp)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("anim.gif")),
            Some(ImageFormat::Gif)
        );
    }

    #[test]
    fn tiff_bmp_gif_alpha() {
        assert!(ImageFormat::Tiff.supports_alpha());
        assert!(!ImageFormat::Bmp.supports_alpha());
        assert!(ImageFormat::Gif.supports_alpha());
    }

    #[test]
    fn detect_psd() {
        assert_eq!(
            ImageFormat::from_path(Path::new("design.psd")),
            Some(ImageFormat::Psd)
        );
    }

    #[test]
    fn detect_raw_formats() {
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.cr2")),
            Some(ImageFormat::Raw)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.nef")),
            Some(ImageFormat::Raw)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.arw")),
            Some(ImageFormat::Raw)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.dng")),
            Some(ImageFormat::Raw)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.raf")),
            Some(ImageFormat::Raw)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.orf")),
            Some(ImageFormat::Raw)
        );
        assert_eq!(
            ImageFormat::from_path(Path::new("photo.rw2")),
            Some(ImageFormat::Raw)
        );
    }

    #[test]
    fn psd_alpha_support() {
        assert!(ImageFormat::Psd.supports_alpha());
    }

    #[test]
    fn raw_no_alpha() {
        assert!(!ImageFormat::Raw.supports_alpha());
    }

    #[test]
    fn psd_extension() {
        assert_eq!(ImageFormat::Psd.extension(), "psd");
    }

    #[test]
    fn raw_extension() {
        assert_eq!(ImageFormat::Raw.extension(), "dng");
    }

    #[test]
    fn raw_not_exportable() {
        assert!(!ImageFormat::Raw.is_exportable());
    }

    #[test]
    fn psd_is_exportable() {
        assert!(ImageFormat::Psd.is_exportable());
    }

    #[test]
    fn export_settings_psd() {
        let s = ExportSettings::for_format(ImageFormat::Psd);
        assert_eq!(s.format(), ImageFormat::Psd);
    }

    #[test]
    #[should_panic(expected = "RAW format is import-only")]
    fn export_settings_raw_panics() {
        ExportSettings::for_format(ImageFormat::Raw);
    }
}
