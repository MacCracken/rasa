//! Serde round-trip tests for all serializable types in rasa-core.

use rasa_core::color::{BlendMode, Color, ColorSpace};
use rasa_core::command::Command;
use rasa_core::document::Document;
use rasa_core::geometry::{Point, Rect, Size};
use rasa_core::layer::{Adjustment, Layer, LayerKind, TextAlign, TextLayer};
use rasa_core::selection::{Selection, SelectionOp};
use rasa_core::transform::Transform;
use rasa_core::vector::VectorData;

/// Serialize to JSON and back, asserting the round-trip produces an equal value.
fn roundtrip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialize failed");
    serde_json::from_str(&json).expect("deserialize failed")
}

// ── Geometry ────────────────────────────────────────────

#[test]
fn roundtrip_point() {
    let p = Point { x: 3.15, y: -2.72 };
    let p2 = roundtrip(&p);
    assert_eq!(p, p2);
}

#[test]
fn roundtrip_size() {
    let s = Size {
        width: 1920,
        height: 1080,
    };
    let s2 = roundtrip(&s);
    assert_eq!(s, s2);
}

#[test]
fn roundtrip_rect() {
    let r = Rect {
        x: 10.5,
        y: 20.5,
        width: 100.0,
        height: 200.0,
    };
    let r2 = roundtrip(&r);
    assert_eq!(r, r2);
}

// ── Color ───────────────────────────────────────────────

#[test]
fn roundtrip_color() {
    let c = Color::new(0.25, 0.5, 0.75, 1.0);
    let c2 = roundtrip(&c);
    assert_eq!(c, c2);
}

#[test]
fn roundtrip_color_constants() {
    assert_eq!(roundtrip(&Color::BLACK), Color::BLACK);
    assert_eq!(roundtrip(&Color::WHITE), Color::WHITE);
    assert_eq!(roundtrip(&Color::TRANSPARENT), Color::TRANSPARENT);
}

#[test]
fn roundtrip_color_space_all_variants() {
    for cs in [
        ColorSpace::Srgb,
        ColorSpace::LinearRgb,
        ColorSpace::DisplayP3,
    ] {
        let cs2 = roundtrip(&cs);
        assert_eq!(cs, cs2);
    }
}

#[test]
fn roundtrip_blend_mode_all_variants() {
    let modes = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
    ];
    for mode in modes {
        let mode2 = roundtrip(&mode);
        assert_eq!(mode, mode2);
    }
}

// ── Transform ───────────────────────────────────────────

#[test]
fn roundtrip_transform_identity() {
    let t = Transform::IDENTITY;
    let t2 = roundtrip(&t);
    assert_eq!(t, t2);
}

#[test]
fn roundtrip_transform_complex() {
    let t = Transform::translate(10.0, 20.0)
        .then(&Transform::scale(2.0, 3.0))
        .then(&Transform::rotate(0.785));
    let t2 = roundtrip(&t);
    assert_eq!(t, t2);
}

// ── Selection ───────────────────────────────────────────

#[test]
fn roundtrip_selection_none() {
    let s = Selection::None;
    let json = serde_json::to_string(&s).unwrap();
    let s2: Selection = serde_json::from_str(&json).unwrap();
    assert!(s2.is_none());
}

#[test]
fn roundtrip_selection_rect() {
    let s = Selection::Rect(Rect {
        x: 10.0,
        y: 20.0,
        width: 30.0,
        height: 40.0,
    });
    let json = serde_json::to_string(&s).unwrap();
    let s2: Selection = serde_json::from_str(&json).unwrap();
    let bounds = s2.bounds().unwrap();
    assert_eq!(bounds.x, 10.0);
    assert_eq!(bounds.width, 30.0);
}

#[test]
fn roundtrip_selection_ellipse() {
    let s = Selection::Ellipse(Rect {
        x: 0.0,
        y: 0.0,
        width: 50.0,
        height: 50.0,
    });
    let json = serde_json::to_string(&s).unwrap();
    let s2: Selection = serde_json::from_str(&json).unwrap();
    assert!(s2.contains(Point { x: 25.0, y: 25.0 }));
}

#[test]
fn roundtrip_selection_freeform() {
    let s = Selection::Freeform {
        points: vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 5.0, y: 10.0 },
        ],
    };
    let json = serde_json::to_string(&s).unwrap();
    let s2: Selection = serde_json::from_str(&json).unwrap();
    assert!(s2.contains(Point { x: 5.0, y: 3.0 }));
}

#[test]
fn roundtrip_selection_mask() {
    let s = Selection::Mask {
        width: 2,
        height: 2,
        data: vec![1.0, 0.0, 0.0, 1.0],
    };
    let json = serde_json::to_string(&s).unwrap();
    let s2: Selection = serde_json::from_str(&json).unwrap();
    assert!(s2.contains(Point { x: 0.0, y: 0.0 }));
    assert!(!s2.contains(Point { x: 1.0, y: 0.0 }));
}

#[test]
fn roundtrip_selection_op_all_variants() {
    for op in [
        SelectionOp::Replace,
        SelectionOp::Add,
        SelectionOp::Subtract,
        SelectionOp::Intersect,
    ] {
        let op2 = roundtrip(&op);
        assert_eq!(op, op2);
    }
}

// ── Layer ───────────────────────────────────────────────

#[test]
fn roundtrip_layer_raster() {
    let layer = Layer::new_raster("Background", 1920, 1080);
    let json = serde_json::to_string(&layer).unwrap();
    let layer2: Layer = serde_json::from_str(&json).unwrap();
    assert_eq!(layer.id, layer2.id);
    assert_eq!(layer.name, layer2.name);
    assert_eq!(layer.opacity, layer2.opacity);
    assert_eq!(layer.visible, layer2.visible);
    assert_eq!(layer.locked, layer2.locked);
    assert_eq!(layer.blend_mode, layer2.blend_mode);
    assert!(matches!(
        layer2.kind,
        LayerKind::Raster {
            width: 1920,
            height: 1080
        }
    ));
}

#[test]
fn roundtrip_layer_vector() {
    let mut layer = Layer::new_raster("Vector", 100, 100);
    layer.kind = LayerKind::Vector(VectorData::new());
    let json = serde_json::to_string(&layer).unwrap();
    let layer2: Layer = serde_json::from_str(&json).unwrap();
    assert!(matches!(layer2.kind, LayerKind::Vector(_)));
}

#[test]
fn roundtrip_layer_group() {
    let child = Layer::new_raster("Child", 10, 10);
    let mut group = Layer::new_raster("Group", 10, 10);
    group.kind = LayerKind::Group {
        children: vec![child],
    };
    let json = serde_json::to_string(&group).unwrap();
    let group2: Layer = serde_json::from_str(&json).unwrap();
    if let LayerKind::Group { children } = &group2.kind {
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "Child");
    } else {
        panic!("expected Group");
    }
}

#[test]
fn roundtrip_layer_adjustment() {
    let mut layer = Layer::new_raster("Adj", 10, 10);
    layer.kind = LayerKind::Adjustment(Adjustment::BrightnessContrast {
        brightness: 0.3,
        contrast: -0.1,
    });
    let json = serde_json::to_string(&layer).unwrap();
    let layer2: Layer = serde_json::from_str(&json).unwrap();
    if let LayerKind::Adjustment(Adjustment::BrightnessContrast {
        brightness,
        contrast,
    }) = layer2.kind
    {
        assert_eq!(brightness, 0.3);
        assert_eq!(contrast, -0.1);
    } else {
        panic!("expected BrightnessContrast adjustment");
    }
}

#[test]
fn roundtrip_layer_text() {
    let mut layer = Layer::new_raster("Text", 10, 10);
    layer.kind = LayerKind::Text(TextLayer {
        content: "Hello".into(),
        font_family: "Inter".into(),
        font_size: 16.0,
        color: Color::BLACK,
        alignment: TextAlign::Center,
        line_height: 1.4,
    });
    let json = serde_json::to_string(&layer).unwrap();
    let layer2: Layer = serde_json::from_str(&json).unwrap();
    if let LayerKind::Text(text) = &layer2.kind {
        assert_eq!(text.content, "Hello");
        assert_eq!(text.font_family, "Inter");
        assert_eq!(text.font_size, 16.0);
        assert_eq!(text.color, Color::BLACK);
        assert_eq!(text.alignment, TextAlign::Center);
        assert_eq!(text.line_height, 1.4);
    } else {
        panic!("expected Text");
    }
}

#[test]
fn roundtrip_adjustment_hue_saturation() {
    let adj = Adjustment::HueSaturation {
        hue: 120.0,
        saturation: 0.5,
        lightness: -0.2,
    };
    let adj2 = roundtrip(&adj);
    if let Adjustment::HueSaturation {
        hue,
        saturation,
        lightness,
    } = adj2
    {
        assert_eq!(hue, 120.0);
        assert_eq!(saturation, 0.5);
        assert_eq!(lightness, -0.2);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn roundtrip_adjustment_curves() {
    let adj = Adjustment::Curves {
        points: vec![(0.0, 0.0), (0.25, 0.3), (0.75, 0.8), (1.0, 1.0)],
    };
    let adj2 = roundtrip(&adj);
    if let Adjustment::Curves { points } = adj2 {
        assert_eq!(points.len(), 4);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn roundtrip_adjustment_levels() {
    let adj = Adjustment::Levels {
        black: 0.1,
        white: 0.9,
        gamma: 1.2,
    };
    let adj2 = roundtrip(&adj);
    if let Adjustment::Levels {
        black,
        white,
        gamma,
    } = adj2
    {
        assert_eq!(black, 0.1);
        assert_eq!(white, 0.9);
        assert_eq!(gamma, 1.2);
    } else {
        panic!("wrong variant");
    }
}

// ── Command ─────────────────────────────────────────────

#[test]
fn roundtrip_command_add_layer() {
    let cmd = Command::AddLayer {
        layer: Layer::new_raster("Test", 10, 10),
        index: 0,
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let cmd2: Command = serde_json::from_str(&json).unwrap();
    if let Command::AddLayer { layer, index } = cmd2 {
        assert_eq!(layer.name, "Test");
        assert_eq!(index, 0);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn roundtrip_command_remove_layer() {
    let cmd = Command::RemoveLayer {
        layer: Layer::new_raster("Removed", 10, 10),
        index: 2,
    };
    let cmd2 = roundtrip(&cmd);
    if let Command::RemoveLayer { layer, index } = cmd2 {
        assert_eq!(layer.name, "Removed");
        assert_eq!(index, 2);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn roundtrip_command_rename_layer() {
    let cmd = Command::RenameLayer {
        layer_id: uuid::Uuid::new_v4(),
        old_name: "Old".into(),
        new_name: "New".into(),
    };
    let cmd2 = roundtrip(&cmd);
    if let Command::RenameLayer {
        old_name, new_name, ..
    } = cmd2
    {
        assert_eq!(old_name, "Old");
        assert_eq!(new_name, "New");
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn roundtrip_command_set_opacity() {
    let cmd = Command::SetLayerOpacity {
        layer_id: uuid::Uuid::new_v4(),
        old_opacity: 1.0,
        new_opacity: 0.5,
    };
    let cmd2 = roundtrip(&cmd);
    if let Command::SetLayerOpacity {
        old_opacity,
        new_opacity,
        ..
    } = cmd2
    {
        assert_eq!(old_opacity, 1.0);
        assert_eq!(new_opacity, 0.5);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn roundtrip_command_set_blend_mode() {
    let cmd = Command::SetLayerBlendMode {
        layer_id: uuid::Uuid::new_v4(),
        old_mode: BlendMode::Normal,
        new_mode: BlendMode::Multiply,
    };
    let cmd2 = roundtrip(&cmd);
    if let Command::SetLayerBlendMode {
        old_mode, new_mode, ..
    } = cmd2
    {
        assert_eq!(old_mode, BlendMode::Normal);
        assert_eq!(new_mode, BlendMode::Multiply);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn roundtrip_command_reorder() {
    let cmd = Command::ReorderLayer {
        layer_id: uuid::Uuid::new_v4(),
        from_index: 0,
        to_index: 3,
    };
    let cmd2 = roundtrip(&cmd);
    if let Command::ReorderLayer {
        from_index,
        to_index,
        ..
    } = cmd2
    {
        assert_eq!(from_index, 0);
        assert_eq!(to_index, 3);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn roundtrip_command_duplicate() {
    let cmd = Command::DuplicateLayer {
        original_id: uuid::Uuid::new_v4(),
        new_layer: Layer::new_raster("Copy", 10, 10),
        index: 1,
    };
    let cmd2 = roundtrip(&cmd);
    if let Command::DuplicateLayer {
        new_layer, index, ..
    } = cmd2
    {
        assert_eq!(new_layer.name, "Copy");
        assert_eq!(index, 1);
    } else {
        panic!("wrong variant");
    }
}

// ── Document ────────────────────────────────────────────

#[test]
fn roundtrip_document() {
    let mut doc = Document::new("Test Canvas", 1920, 1080);
    doc.add_layer(Layer::new_raster("Layer 1", 1920, 1080));

    let json = serde_json::to_string(&doc).unwrap();
    let doc2: Document = serde_json::from_str(&json).unwrap();

    assert_eq!(doc.id, doc2.id);
    assert_eq!(doc.name, doc2.name);
    assert_eq!(doc.size.width, doc2.size.width);
    assert_eq!(doc.size.height, doc2.size.height);
    assert_eq!(doc.dpi, doc2.dpi);
    assert_eq!(doc.layers.len(), doc2.layers.len());
    // pixel_data and history are #[serde(skip)], so they should be empty/None
    assert!(doc2.pixel_data.is_empty());
}

#[test]
fn roundtrip_document_preserves_layer_properties() {
    let mut doc = Document::new("Props Test", 100, 100);
    let bg_id = doc.layers[0].id;
    doc.set_layer_opacity(bg_id, 0.7).unwrap();
    doc.set_layer_blend_mode(bg_id, BlendMode::Screen).unwrap();
    doc.rename_layer(bg_id, "Base").unwrap();

    let json = serde_json::to_string(&doc).unwrap();
    let doc2: Document = serde_json::from_str(&json).unwrap();

    assert_eq!(doc2.layers[0].name, "Base");
    assert_eq!(doc2.layers[0].opacity, 0.7);
    assert_eq!(doc2.layers[0].blend_mode, BlendMode::Screen);
}
