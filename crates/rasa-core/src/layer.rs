use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::color::BlendMode;
use crate::geometry::Rect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub bounds: Rect,
    pub kind: LayerKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerKind {
    Raster { width: u32, height: u32 },
    Vector,
    Group { children: Vec<Layer> },
    Adjustment(Adjustment),
    Text(TextLayer),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Adjustment {
    BrightnessContrast {
        brightness: f32,
        contrast: f32,
    },
    HueSaturation {
        hue: f32,
        saturation: f32,
        lightness: f32,
    },
    Curves {
        points: Vec<(f32, f32)>,
    },
    Levels {
        black: f32,
        white: f32,
        gamma: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLayer {
    pub content: String,
    pub font_family: String,
    pub font_size: f32,
}

impl Layer {
    pub fn new_raster(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::default(),
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: width as f64,
                height: height as f64,
            },
            kind: LayerKind::Raster { width, height },
        }
    }
}
