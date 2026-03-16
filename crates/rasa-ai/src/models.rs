use serde::{Deserialize, Serialize};

/// Model identifier — maps to a model name on the Synapse backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub String);

impl ModelId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about an available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: ModelId,
    pub name: String,
    pub kind: ModelKind,
    pub description: Option<String>,
    /// Size in bytes (if known).
    pub size_bytes: Option<u64>,
    /// Whether the model is currently loaded in memory.
    pub loaded: bool,
}

/// Kind of AI model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelKind {
    Inpainting,
    Upscaling,
    Segmentation,
    TextToImage,
    BackgroundRemoval,
    StyleTransfer,
    ColorGrading,
}

/// Well-known model presets.
pub mod presets {
    use super::ModelId;

    pub fn stable_diffusion_inpaint() -> ModelId {
        ModelId::new("stable-diffusion-inpainting")
    }

    pub fn real_esrgan_x4() -> ModelId {
        ModelId::new("realesrgan-x4plus")
    }

    pub fn sam_vit_h() -> ModelId {
        ModelId::new("sam-vit-h")
    }

    pub fn stable_diffusion_xl() -> ModelId {
        ModelId::new("stable-diffusion-xl-base-1.0")
    }

    pub fn rembg_u2net() -> ModelId {
        ModelId::new("u2net")
    }

    pub fn style_transfer_default() -> ModelId {
        ModelId::new("style-transfer-v1")
    }

    pub fn color_grading_default() -> ModelId {
        ModelId::new("color-grading-v1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_id_display() {
        let id = ModelId::new("test-model");
        assert_eq!(id.to_string(), "test-model");
    }

    #[test]
    fn model_id_serialize() {
        let id = ModelId::new("my-model");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"my-model\"");
    }

    #[test]
    fn model_id_deserialize() {
        let id: ModelId = serde_json::from_str("\"test\"").unwrap();
        assert_eq!(id.0, "test");
    }

    #[test]
    fn presets_exist() {
        assert_eq!(presets::real_esrgan_x4().0, "realesrgan-x4plus");
        assert_eq!(presets::sam_vit_h().0, "sam-vit-h");
    }

    #[test]
    fn model_kind_serialize() {
        let json = serde_json::to_string(&ModelKind::Upscaling).unwrap();
        assert_eq!(json, "\"Upscaling\"");
    }

    #[test]
    fn style_transfer_model_kind() {
        let json = serde_json::to_string(&ModelKind::StyleTransfer).unwrap();
        assert_eq!(json, "\"StyleTransfer\"");
        let back: ModelKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ModelKind::StyleTransfer);
    }

    #[test]
    fn color_grading_model_kind() {
        let json = serde_json::to_string(&ModelKind::ColorGrading).unwrap();
        assert_eq!(json, "\"ColorGrading\"");
        let back: ModelKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ModelKind::ColorGrading);
    }

    #[test]
    fn style_transfer_preset() {
        let id = presets::style_transfer_default();
        assert_eq!(id.0, "style-transfer-v1");
    }

    #[test]
    fn color_grading_preset() {
        let id = presets::color_grading_default();
        assert_eq!(id.0, "color-grading-v1");
    }

    #[test]
    fn model_info_roundtrip() {
        let info = ModelInfo {
            id: ModelId::new("test"),
            name: "Test Model".into(),
            kind: ModelKind::Inpainting,
            description: Some("A test model".into()),
            size_bytes: Some(1024),
            loaded: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: ModelInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, info.id);
        assert_eq!(back.name, "Test Model");
        assert!(back.loaded);
    }
}
