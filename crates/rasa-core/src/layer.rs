use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::color::{BlendMode, Color};
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

/// Text alignment within the layer bounds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLayer {
    pub content: String,
    pub font_family: String,
    pub font_size: f32,
    pub color: Color,
    pub alignment: TextAlign,
    pub line_height: f32,
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

    pub fn new_adjustment(
        name: impl Into<String>,
        adjustment: Adjustment,
        doc_size: (u32, u32),
    ) -> Self {
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
                width: doc_size.0 as f64,
                height: doc_size.1 as f64,
            },
            kind: LayerKind::Adjustment(adjustment),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_raster_defaults() {
        let layer = Layer::new_raster("Test Layer", 800, 600);
        assert_eq!(layer.name, "Test Layer");
        assert!(layer.visible);
        assert!(!layer.locked);
        assert_eq!(layer.opacity, 1.0);
        assert_eq!(layer.blend_mode, BlendMode::Normal);
        assert_eq!(layer.bounds.width, 800.0);
        assert_eq!(layer.bounds.height, 600.0);
        assert_eq!(layer.bounds.x, 0.0);
        assert_eq!(layer.bounds.y, 0.0);
        assert!(matches!(
            layer.kind,
            LayerKind::Raster {
                width: 800,
                height: 600
            }
        ));
    }

    #[test]
    fn new_raster_unique_ids() {
        let a = Layer::new_raster("A", 10, 10);
        let b = Layer::new_raster("B", 10, 10);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn new_raster_accepts_string() {
        let name = String::from("Dynamic Name");
        let layer = Layer::new_raster(name, 100, 100);
        assert_eq!(layer.name, "Dynamic Name");
    }

    #[test]
    fn layer_clone() {
        let original = Layer::new_raster("Original", 50, 50);
        let cloned = original.clone();
        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.name, original.name);
        assert_eq!(cloned.opacity, original.opacity);
    }

    #[test]
    fn layer_kind_raster() {
        let layer = Layer::new_raster("Raster", 100, 200);
        if let LayerKind::Raster { width, height } = layer.kind {
            assert_eq!(width, 100);
            assert_eq!(height, 200);
        } else {
            panic!("expected Raster kind");
        }
    }

    #[test]
    fn layer_kind_group() {
        let child = Layer::new_raster("Child", 10, 10);
        let group_kind = LayerKind::Group {
            children: vec![child],
        };
        if let LayerKind::Group { children } = &group_kind {
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].name, "Child");
        } else {
            panic!("expected Group kind");
        }
    }

    #[test]
    fn layer_kind_adjustment_brightness_contrast() {
        let adj = Adjustment::BrightnessContrast {
            brightness: 0.5,
            contrast: -0.2,
        };
        if let Adjustment::BrightnessContrast {
            brightness,
            contrast,
        } = adj
        {
            assert_eq!(brightness, 0.5);
            assert_eq!(contrast, -0.2);
        }
    }

    #[test]
    fn layer_kind_adjustment_hue_saturation() {
        let adj = Adjustment::HueSaturation {
            hue: 180.0,
            saturation: 0.8,
            lightness: -0.1,
        };
        if let Adjustment::HueSaturation {
            hue,
            saturation,
            lightness,
        } = adj
        {
            assert_eq!(hue, 180.0);
            assert_eq!(saturation, 0.8);
            assert_eq!(lightness, -0.1);
        }
    }

    #[test]
    fn layer_kind_adjustment_curves() {
        let adj = Adjustment::Curves {
            points: vec![(0.0, 0.0), (0.5, 0.7), (1.0, 1.0)],
        };
        if let Adjustment::Curves { points } = &adj {
            assert_eq!(points.len(), 3);
            assert_eq!(points[1], (0.5, 0.7));
        }
    }

    #[test]
    fn layer_kind_adjustment_levels() {
        let adj = Adjustment::Levels {
            black: 0.0,
            white: 1.0,
            gamma: 1.0,
        };
        if let Adjustment::Levels {
            black,
            white,
            gamma,
        } = adj
        {
            assert_eq!(black, 0.0);
            assert_eq!(white, 1.0);
            assert_eq!(gamma, 1.0);
        }
    }

    #[test]
    fn text_layer_creation() {
        let text = TextLayer {
            content: "Hello World".into(),
            font_family: "Inter".into(),
            font_size: 24.0,
            color: Color::BLACK,
            alignment: TextAlign::Left,
            line_height: 1.2,
        };
        assert_eq!(text.content, "Hello World");
        assert_eq!(text.font_family, "Inter");
        assert_eq!(text.font_size, 24.0);
        assert_eq!(text.color, Color::BLACK);
        assert_eq!(text.alignment, TextAlign::Left);
        assert_eq!(text.line_height, 1.2);
    }

    #[test]
    fn text_align_default_is_left() {
        assert_eq!(TextAlign::default(), TextAlign::Left);
    }

    #[test]
    fn text_align_variants() {
        let left = TextAlign::Left;
        let center = TextAlign::Center;
        let right = TextAlign::Right;
        assert_ne!(left, center);
        assert_ne!(center, right);
        assert_ne!(left, right);
    }

    #[test]
    fn blend_mode_default_is_normal() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
    }

    #[test]
    fn new_adjustment_layer() {
        let adj = Adjustment::BrightnessContrast {
            brightness: 0.5,
            contrast: 0.0,
        };
        let layer = Layer::new_adjustment("Brighten", adj, (800, 600));
        assert_eq!(layer.name, "Brighten");
        assert!(layer.visible);
        assert_eq!(layer.opacity, 1.0);
        assert_eq!(layer.bounds.width, 800.0);
        assert_eq!(layer.bounds.height, 600.0);
        assert!(matches!(
            layer.kind,
            LayerKind::Adjustment(Adjustment::BrightnessContrast { .. })
        ));
    }

    #[test]
    fn new_adjustment_unique_ids() {
        let adj = Adjustment::Levels {
            black: 0.0,
            white: 1.0,
            gamma: 1.0,
        };
        let a = Layer::new_adjustment("A", adj.clone(), (10, 10));
        let b = Layer::new_adjustment("B", adj, (10, 10));
        assert_ne!(a.id, b.id);
    }
}
