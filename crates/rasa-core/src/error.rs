use thiserror::Error;

/// Top-level error type for all Rasa operations.
#[derive(Debug, Error)]
pub enum RasaError {
    // ── Document errors ─────────────────────────────────
    #[error("invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("invalid color value: {channel} = {value} (expected {min}..={max})")]
    InvalidColorValue {
        channel: &'static str,
        value: f32,
        min: f32,
        max: f32,
    },

    // ── Layer errors ────────────────────────────────────
    #[error("layer not found: {0}")]
    LayerNotFound(uuid::Uuid),

    #[error("layer is locked: {0}")]
    LayerLocked(uuid::Uuid),

    #[error("cannot remove last layer")]
    CannotRemoveLastLayer,

    #[error("not an adjustment layer: {0}")]
    NotAnAdjustmentLayer(uuid::Uuid),

    // ── Selection errors ────────────────────────────────
    #[error("invalid selection: {0}")]
    InvalidSelection(String),

    // ── Transform errors ────────────────────────────────
    #[error("singular transform: matrix is not invertible")]
    SingularTransform,

    // ── Color management errors ─────────────────────────
    #[error("invalid ICC profile: {0}")]
    InvalidIccProfile(String),

    #[error("ICC color conversion failed: {0}")]
    ColorConversionFailed(String),

    // ── Storage / format errors ─────────────────────────
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("corrupt file: {0}")]
    CorruptFile(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    // ── AI errors ───────────────────────────────────────
    #[error("AI inference failed: {0}")]
    InferenceFailed(String),

    #[error("model not found: {0}")]
    ModelNotFound(String),

    // ── History errors ──────────────────────────────────
    #[error("nothing to undo")]
    NothingToUndo,

    #[error("nothing to redo")]
    NothingToRedo,

    // ── System errors ───────────────────────────────────
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("plugin error: {0}")]
    PluginError(String),

    #[error("{0}")]
    Other(String),
}

impl From<serde_json::Error> for RasaError {
    fn from(e: serde_json::Error) -> Self {
        RasaError::Serialization(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn error_display_invalid_dimensions() {
        let e = RasaError::InvalidDimensions {
            width: 0,
            height: 100,
        };
        assert_eq!(e.to_string(), "invalid dimensions: 0x100");
    }

    #[test]
    fn error_display_layer_not_found() {
        let id = Uuid::nil();
        let e = RasaError::LayerNotFound(id);
        assert!(e.to_string().contains("layer not found"));
    }

    #[test]
    fn error_display_layer_locked() {
        let id = Uuid::nil();
        let e = RasaError::LayerLocked(id);
        assert!(e.to_string().contains("layer is locked"));
    }

    #[test]
    fn error_display_cannot_remove_last_layer() {
        let e = RasaError::CannotRemoveLastLayer;
        assert_eq!(e.to_string(), "cannot remove last layer");
    }

    #[test]
    fn error_display_singular_transform() {
        let e = RasaError::SingularTransform;
        assert!(e.to_string().contains("not invertible"));
    }

    #[test]
    fn error_display_corrupt_file() {
        let e = RasaError::CorruptFile("bad header".into());
        assert_eq!(e.to_string(), "corrupt file: bad header");
    }

    #[test]
    fn error_display_nothing_to_undo() {
        let e = RasaError::NothingToUndo;
        assert_eq!(e.to_string(), "nothing to undo");
    }

    #[test]
    fn error_display_nothing_to_redo() {
        let e = RasaError::NothingToRedo;
        assert_eq!(e.to_string(), "nothing to redo");
    }

    #[test]
    fn error_display_invalid_color_value() {
        let e = RasaError::InvalidColorValue {
            channel: "red",
            value: 2.0,
            min: 0.0,
            max: 1.0,
        };
        assert!(e.to_string().contains("red"));
        assert!(e.to_string().contains("2"));
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e: RasaError = io_err.into();
        assert!(matches!(e, RasaError::Io(_)));
        assert!(e.to_string().contains("file missing"));
    }

    #[test]
    fn error_from_serde_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let e: RasaError = json_err.into();
        assert!(matches!(e, RasaError::Serialization(_)));
    }

    #[test]
    fn error_display_model_not_found() {
        let e = RasaError::ModelNotFound("upscaler-v2".into());
        assert!(e.to_string().contains("upscaler-v2"));
    }

    #[test]
    fn error_display_invalid_icc_profile() {
        let e = RasaError::InvalidIccProfile("bad header".into());
        assert!(e.to_string().contains("bad header"));
    }

    #[test]
    fn error_display_color_conversion_failed() {
        let e = RasaError::ColorConversionFailed("transform error".into());
        assert!(e.to_string().contains("transform error"));
    }

    #[test]
    fn error_display_unsupported_format() {
        let e = RasaError::UnsupportedFormat("bmp".into());
        assert!(e.to_string().contains("bmp"));
    }

    #[test]
    fn error_display_invalid_selection() {
        let e = RasaError::InvalidSelection("empty polygon".into());
        assert!(e.to_string().contains("empty polygon"));
    }

    #[test]
    fn error_display_plugin_error() {
        let e = RasaError::PluginError("failed to load plugin".into());
        assert_eq!(e.to_string(), "plugin error: failed to load plugin");
    }

    #[test]
    fn error_display_other() {
        let e = RasaError::Other("something went wrong".into());
        assert_eq!(e.to_string(), "something went wrong");
    }
}
