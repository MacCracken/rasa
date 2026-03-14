use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::geometry::Size;
use crate::layer::Layer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub name: String,
    pub size: Size,
    pub dpi: f32,
    pub layers: Vec<Layer>,
    pub active_layer: Option<Uuid>,
}

impl Document {
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        let bg = Layer::new_raster("Background", width, height);
        let bg_id = bg.id;
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            size: Size { width, height },
            dpi: 72.0,
            layers: vec![bg],
            active_layer: Some(bg_id),
        }
    }
}
