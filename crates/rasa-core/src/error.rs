use thiserror::Error;

#[derive(Debug, Error)]
pub enum RasaError {
    #[error("invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("layer not found: {0}")]
    LayerNotFound(uuid::Uuid),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("AI inference failed: {0}")]
    InferenceFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}
