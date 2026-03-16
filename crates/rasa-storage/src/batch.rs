use std::path::{Path, PathBuf};

use rasa_core::error::RasaError;

use crate::format::{ExportConfig, ExportSettings, ImageFormat, JpegQuality};
use crate::import::import_image;

/// A filter to apply during batch processing.
#[derive(Debug, Clone)]
pub enum BatchFilter {
    Invert,
    Grayscale,
    BrightnessContrast {
        brightness: f32,
        contrast: f32,
    },
    HueSaturation {
        hue: f32,
        saturation: f32,
        lightness: f32,
    },
    GaussianBlur {
        radius: u32,
    },
    Sharpen {
        radius: u32,
        amount: f32,
    },
}

/// A single batch job: import, optionally transform, export.
#[derive(Debug, Clone)]
pub struct BatchJob {
    /// Input file paths.
    pub input_paths: Vec<PathBuf>,
    /// Output directory.
    pub output_dir: PathBuf,
    /// Output format (None = keep original format).
    pub format: Option<ImageFormat>,
    /// JPEG quality (only used when format is JPEG).
    pub jpeg_quality: u8,
    /// Filters to apply in order.
    pub filters: Vec<BatchFilter>,
    /// Optional ICC profile for export.
    pub icc_profile: Option<rasa_core::color::IccProfile>,
}

/// Result of processing a single file in a batch.
#[derive(Debug)]
pub struct BatchFileResult {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub error: Option<String>,
}

/// Result of a complete batch job.
#[derive(Debug)]
pub struct BatchResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<BatchFileResult>,
}

impl BatchJob {
    /// Process all input files sequentially.
    pub fn run(&self) -> Result<BatchResult, RasaError> {
        if !self.output_dir.is_dir() {
            std::fs::create_dir_all(&self.output_dir)?;
        }

        let total = self.input_paths.len();
        let mut succeeded = 0;
        let mut failed = 0;
        let mut results = Vec::with_capacity(total);

        for input_path in &self.input_paths {
            match self.process_one(input_path) {
                Ok(output_path) => {
                    succeeded += 1;
                    results.push(BatchFileResult {
                        input: input_path.clone(),
                        output: Some(output_path),
                        error: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    results.push(BatchFileResult {
                        input: input_path.clone(),
                        output: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(BatchResult {
            total,
            succeeded,
            failed,
            results,
        })
    }

    fn process_one(&self, input_path: &Path) -> Result<PathBuf, RasaError> {
        // Import
        let doc = import_image(input_path)?;
        let layer_id = doc
            .active_layer
            .ok_or_else(|| RasaError::Other("no active layer".into()))?;
        let mut buf = doc
            .pixel_data
            .into_iter()
            .find(|(id, _)| *id == layer_id)
            .map(|(_, b)| b)
            .ok_or_else(|| RasaError::Other("no pixel data for active layer".into()))?;

        // Apply filters
        for filter in &self.filters {
            apply_filter(&mut buf, filter);
        }

        // Determine output format
        let output_format = self
            .format
            .unwrap_or_else(|| ImageFormat::from_path(input_path).unwrap_or(ImageFormat::Png));

        if !output_format.is_exportable() {
            return Err(RasaError::UnsupportedFormat(format!(
                "{:?} is not exportable",
                output_format
            )));
        }

        // Build output path
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_path = self
            .output_dir
            .join(format!("{}.{}", stem, output_format.extension()));

        // Export
        let settings = match output_format {
            ImageFormat::Jpeg => ExportSettings::Jpeg(JpegQuality::new(self.jpeg_quality)),
            _ => ExportSettings::for_format(output_format)?,
        };

        let config = ExportConfig {
            settings,
            icc_profile: self.icc_profile.clone(),
        };

        crate::export::export_buffer_with_config(&buf, &output_path, &config)?;

        Ok(output_path)
    }
}

fn apply_filter(buf: &mut rasa_core::pixel::PixelBuffer, filter: &BatchFilter) {
    match filter {
        BatchFilter::Invert => rasa_engine::filters::invert(buf),
        BatchFilter::Grayscale => rasa_engine::filters::grayscale(buf),
        BatchFilter::BrightnessContrast {
            brightness,
            contrast,
        } => {
            let adj = rasa_core::layer::Adjustment::BrightnessContrast {
                brightness: *brightness,
                contrast: *contrast,
            };
            rasa_engine::filters::apply_adjustment(buf, &adj);
        }
        BatchFilter::HueSaturation {
            hue,
            saturation,
            lightness,
        } => {
            let adj = rasa_core::layer::Adjustment::HueSaturation {
                hue: *hue,
                saturation: *saturation,
                lightness: *lightness,
            };
            rasa_engine::filters::apply_adjustment(buf, &adj);
        }
        BatchFilter::GaussianBlur { radius } => {
            rasa_engine::filters::gaussian_blur(buf, *radius);
        }
        BatchFilter::Sharpen { radius, amount } => {
            rasa_engine::filters::sharpen(buf, *radius, *amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasa_core::color::Color;
    use rasa_core::pixel::PixelBuffer;

    fn create_test_png(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let buf = PixelBuffer::filled(4, 4, Color::new(1.0, 0.0, 0.0, 1.0));
        crate::export::export_buffer(&buf, &path, &ExportSettings::Png).unwrap();
        path
    }

    #[test]
    fn batch_single_file_no_filters() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_input1.png");
        let output_dir = dir.join("output1");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);
        assert!(result.results[0].output.as_ref().unwrap().exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_multiple_files() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input1 = create_test_png(&dir, "batch_multi1.png");
        let input2 = create_test_png(&dir, "batch_multi2.png");
        let output_dir = dir.join("output_multi");

        let job = BatchJob {
            input_paths: vec![input1.clone(), input2.clone()],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 0);

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input1).ok();
        std::fs::remove_file(&input2).ok();
    }

    #[test]
    fn batch_format_conversion() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_convert.png");
        let output_dir = dir.join("output_convert");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: Some(ImageFormat::Jpeg),
            jpeg_quality: 85,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);
        let output = result.results[0].output.as_ref().unwrap();
        assert!(output.to_str().unwrap().ends_with(".jpg"));
        assert!(output.exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_with_filters() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_filter.png");
        let output_dir = dir.join("output_filter");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![
                BatchFilter::Grayscale,
                BatchFilter::BrightnessContrast {
                    brightness: 0.1,
                    contrast: 0.0,
                },
            ],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_handles_missing_file() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        let output_dir = dir.join("output_missing");

        let job = BatchJob {
            input_paths: vec![PathBuf::from("/nonexistent/file.png")],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.failed, 1);
        assert!(result.results[0].error.is_some());

        std::fs::remove_dir_all(&output_dir).ok();
    }

    #[test]
    fn batch_mixed_success_and_failure() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let good = create_test_png(&dir, "batch_good.png");
        let bad = PathBuf::from("/nonexistent/bad.png");
        let output_dir = dir.join("output_mixed");

        let job = BatchJob {
            input_paths: vec![good.clone(), bad],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 1);

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&good).ok();
    }

    #[test]
    fn batch_invert_filter() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_invert.png");
        let output_dir = dir.join("output_invert");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![BatchFilter::Invert],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_raw_format_rejected() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_raw_reject.png");
        let output_dir = dir.join("output_raw_reject");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: Some(ImageFormat::Raw),
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.failed, 1);
        assert!(
            result.results[0]
                .error
                .as_ref()
                .unwrap()
                .contains("not exportable")
        );

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_to_webp() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_webp.png");
        let output_dir = dir.join("output_webp");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: Some(ImageFormat::WebP),
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);
        let output = result.results[0].output.as_ref().unwrap();
        assert!(output.to_str().unwrap().ends_with(".webp"));
        assert!(output.exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_to_bmp() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_bmp.png");
        let output_dir = dir.join("output_bmp");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: Some(ImageFormat::Bmp),
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);
        let output = result.results[0].output.as_ref().unwrap();
        assert!(output.to_str().unwrap().ends_with(".bmp"));
        assert!(output.exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_to_tiff() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_tiff.png");
        let output_dir = dir.join("output_tiff");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: Some(ImageFormat::Tiff),
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);
        let output = result.results[0].output.as_ref().unwrap();
        assert!(output.to_str().unwrap().ends_with(".tiff"));
        assert!(output.exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_blur_filter() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_blur.png");
        let output_dir = dir.join("output_blur");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![BatchFilter::GaussianBlur { radius: 2 }],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);
        assert!(result.results[0].output.as_ref().unwrap().exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_sharpen_filter() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_sharpen.png");
        let output_dir = dir.join("output_sharpen");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![BatchFilter::Sharpen {
                radius: 1,
                amount: 0.5,
            }],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);
        assert!(result.results[0].output.as_ref().unwrap().exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_hue_saturation_filter() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_huesat.png");
        let output_dir = dir.join("output_huesat");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![BatchFilter::HueSaturation {
                hue: 30.0,
                saturation: 0.2,
                lightness: 0.0,
            }],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);
        assert!(result.results[0].output.as_ref().unwrap().exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }

    #[test]
    fn batch_empty_filters() {
        let dir = std::env::temp_dir().join("rasa_test_batch");
        std::fs::create_dir_all(&dir).unwrap();
        let input = create_test_png(&dir, "batch_empty_filt.png");
        let output_dir = dir.join("output_empty_filt");

        let job = BatchJob {
            input_paths: vec![input.clone()],
            output_dir: output_dir.clone(),
            format: None,
            jpeg_quality: 90,
            filters: vec![],
            icc_profile: None,
        };

        let result = job.run().unwrap();
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);
        assert!(result.results[0].output.as_ref().unwrap().exists());

        std::fs::remove_dir_all(&output_dir).ok();
        std::fs::remove_file(&input).ok();
    }
}
