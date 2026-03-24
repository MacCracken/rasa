use egui;
use muharrir::inspector::{Property, PropertySheet};
use rasa_core::Document;
use rasa_core::color::BlendMode;
use rasa_core::layer::{Layer, LayerKind};
use uuid::Uuid;

use crate::tools::ActiveTool;

/// Tool palette — vertical strip of tool buttons.
pub fn tool_palette(ui: &mut egui::Ui, active: &mut ActiveTool) {
    ui.vertical_centered(|ui| {
        ui.heading("Tools");
        ui.separator();
        for (tool, label, shortcut) in [
            (ActiveTool::Brush, "B", "Brush"),
            (ActiveTool::Eraser, "E", "Eraser"),
            (ActiveTool::Move, "M", "Move"),
            (ActiveTool::Selection, "S", "Select"),
            (ActiveTool::Eyedropper, "I", "Picker"),
            (ActiveTool::Fill, "F", "Fill"),
            (ActiveTool::Gradient, "G", "Gradient"),
            (ActiveTool::Crop, "C", "Crop"),
            (ActiveTool::Transform, "T", "Transform"),
        ] {
            let selected = *active == tool;
            let btn = egui::Button::new(label)
                .min_size(egui::Vec2::new(36.0, 36.0))
                .selected(selected);
            let resp = ui.add(btn).on_hover_text(shortcut);
            if resp.clicked() {
                *active = tool;
            }
        }
    });
}

/// Layer panel — list of layers with controls.
///
/// Renders nested group layers with indentation and supports multi-select
/// via ctrl-click (toggle) and shift-click (extend).
pub fn layer_panel(ui: &mut egui::Ui, doc: &mut Document) {
    ui.heading("Layers");
    ui.separator();

    // Add layer button
    if ui.button("+ New Layer").clicked() {
        let (w, h) = (doc.size.width, doc.size.height);
        let count = doc.layers.len();
        doc.add_layer(Layer::new_raster(format!("Layer {count}"), w, h));
    }

    ui.separator();

    // Flatten the layer tree for display (with depth for indentation)
    let flat = flatten_layers(&doc.layers);
    let mut action: Option<LayerAction> = None;

    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for entry in flat.iter().rev() {
                let is_selected = doc.layer_selection.contains(&entry.id);
                let is_primary = doc.active_layer == Some(entry.id);

                let fill = if is_primary {
                    egui::Color32::from_gray(60)
                } else if is_selected {
                    egui::Color32::from_gray(45)
                } else {
                    egui::Color32::TRANSPARENT
                };

                let indent = (entry.depth as i8).min(7) * 16;
                egui::Frame::NONE
                    .fill(fill)
                    .inner_margin(egui::Margin {
                        left: 4 + indent,
                        right: 4,
                        top: 2,
                        bottom: 2,
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Visibility toggle
                            let mut visible = entry.visible;
                            if ui.checkbox(&mut visible, "").changed() {
                                action = Some(LayerAction::SetVisibility(entry.id, visible));
                            }

                            // Group indicator
                            if entry.is_group {
                                ui.label("▸");
                            }

                            // Layer name — click handling with modifier keys
                            let resp = ui.selectable_label(is_primary, &entry.name);
                            if resp.clicked() {
                                let modifiers = ui.input(|i| i.modifiers);
                                if modifiers.ctrl || modifiers.command {
                                    action = Some(LayerAction::Toggle(entry.id));
                                } else if modifiers.shift {
                                    action = Some(LayerAction::Extend(entry.id));
                                } else {
                                    action = Some(LayerAction::Select(entry.id));
                                }
                            }

                            // Opacity
                            ui.label(format!("{:.0}%", entry.opacity * 100.0));
                        });
                    });
            }
        });

    // Apply deferred actions
    if let Some(a) = action {
        match a {
            LayerAction::Select(id) => doc.select_layer(id),
            LayerAction::Toggle(id) => doc.toggle_layer_selection(id),
            LayerAction::Extend(id) => doc.extend_layer_selection(id),
            LayerAction::SetVisibility(id, vis) => {
                let _ = doc.set_layer_visibility(id, vis);
            }
        }
    }

    // Active layer controls
    if let Some(active_id) = doc.active_layer {
        // Read values first before mutating
        let layer_info = doc.find_layer(active_id).map(|l| (l.opacity, l.blend_mode));
        if let Some((current_opacity, current_mode)) = layer_info {
            ui.separator();
            ui.label("Opacity:");
            let mut opacity = current_opacity;
            if ui
                .add(egui::Slider::new(&mut opacity, 0.0..=1.0).text("%"))
                .changed()
            {
                let _ = doc.set_layer_opacity(active_id, opacity);
            }

            ui.label("Blend Mode:");
            let mode_name = format!("{current_mode:?}");
            egui::ComboBox::from_id_salt("blend_mode")
                .selected_text(&mode_name)
                .show_ui(ui, |ui| {
                    for mode in ALL_BLEND_MODES {
                        let name = format!("{mode:?}");
                        if ui.selectable_label(current_mode == *mode, &name).clicked() {
                            let _ = doc.set_layer_blend_mode(active_id, *mode);
                        }
                    }
                });
        }

        // Inspector — structured property sheet for the active layer
        let sheet = build_layer_property_sheet(doc);
        if !sheet.is_empty() {
            ui.separator();
            egui::CollapsingHeader::new("Inspector")
                .default_open(false)
                .show(ui, |ui| {
                    for category in sheet.categories() {
                        ui.label(egui::RichText::new(category).strong());
                        for prop in sheet.by_category(category) {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}:", prop.name));
                                ui.label(&prop.value);
                            });
                        }
                    }
                });
        }
    }
}

/// Build a PropertySheet for the active layer (for inspection / debug).
pub fn build_layer_property_sheet(doc: &Document) -> PropertySheet {
    let mut sheet = PropertySheet::new();
    if let Some(id) = doc.active_layer
        && let Some(layer) = doc.find_layer(id)
    {
        sheet.push(Property::new("Layer", "Name", &layer.name));
        sheet.push(Property::new("Layer", "ID", layer.id.to_string()));
        sheet.push(Property::new("Layer", "Visible", layer.visible.to_string()));
        sheet.push(Property::new("Layer", "Locked", layer.locked.to_string()));
        sheet.push(Property::new(
            "Layer",
            "Opacity",
            format!("{:.0}%", layer.opacity * 100.0),
        ));
        sheet.push(Property::new(
            "Layer",
            "Blend Mode",
            format!("{:?}", layer.blend_mode),
        ));
        sheet.push(Property::new(
            "Layer",
            "Kind",
            match &layer.kind {
                rasa_core::layer::LayerKind::Raster { width, height } => {
                    format!("Raster ({width}x{height})")
                }
                rasa_core::layer::LayerKind::Vector(_) => "Vector".into(),
                rasa_core::layer::LayerKind::Group { children } => {
                    format!("Group ({} children)", children.len())
                }
                rasa_core::layer::LayerKind::Adjustment(adj) => {
                    format!("Adjustment ({adj:?})")
                }
                rasa_core::layer::LayerKind::Text(t) => {
                    format!("Text (\"{}\")", t.content)
                }
            },
        ));
    }
    sheet
}

/// Properties panel — tool settings and color picker.
pub fn properties_panel(
    ui: &mut egui::Ui,
    tool: &ActiveTool,
    size: &mut f32,
    opacity: &mut f32,
    hardness: &mut f32,
    color: &mut [f32; 3],
) {
    ui.heading("Properties");
    ui.separator();

    match tool {
        ActiveTool::Brush | ActiveTool::Eraser => {
            ui.label("Size:");
            expr_slider(ui, "brush_size", size, 1.0..=200.0, true);
            ui.label("Opacity:");
            ui.add(egui::Slider::new(opacity, 0.0..=1.0));
            ui.label("Hardness:");
            ui.add(egui::Slider::new(hardness, 0.0..=1.0));
        }
        _ => {
            ui.label(format!("Tool: {tool:?}"));
        }
    }

    ui.separator();
    ui.heading("Color");
    ui.color_edit_button_rgb(color);

    // Hex input with expression evaluation
    let hex = format!(
        "#{:02X}{:02X}{:02X}",
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
    );
    ui.label(&hex);
}

/// A slider with a text field that supports expression evaluation via muharrir::expr.
///
/// The user can type math expressions like "10+5" or "200/3" and the field
/// evaluates them on defocus. Falls back to the slider value if parsing fails.
fn expr_slider(
    ui: &mut egui::Ui,
    _id: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    logarithmic: bool,
) {
    ui.horizontal(|ui| {
        let mut slider = egui::Slider::new(value, range.clone());
        if logarithmic {
            slider = slider.logarithmic(true);
        }
        ui.add(slider);

        // Small text field for direct entry / expression evaluation
        let mut text = format!("{:.1}", *value);
        let resp = ui.add(egui::TextEdit::singleline(&mut text).desired_width(50.0));
        if resp.lost_focus()
            && let Ok(v) = muharrir::expr::eval_f64(&text)
        {
            let v = (v as f32).clamp(*range.start(), *range.end());
            *value = v;
        }
    });
}

/// History panel — undo/redo controls with command descriptions.
pub fn history_panel(ui: &mut egui::Ui, doc: &mut Document) {
    ui.heading("History");
    ui.separator();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(doc.can_undo(), egui::Button::new("Undo"))
            .clicked()
        {
            let _ = doc.undo();
        }
        if ui
            .add_enabled(doc.can_redo(), egui::Button::new("Redo"))
            .clicked()
        {
            let _ = doc.redo();
        }
    });
    // Show undo/redo counts
    ui.label(format!(
        "{} undo / {} redo",
        doc.undo_count(),
        doc.redo_count()
    ));
}

enum LayerAction {
    Select(Uuid),
    Toggle(Uuid),
    Extend(Uuid),
    SetVisibility(Uuid, bool),
}

/// A flattened layer entry for display, with depth for indentation.
struct FlatLayerEntry {
    id: Uuid,
    name: String,
    visible: bool,
    opacity: f32,
    depth: usize,
    is_group: bool,
}

/// Recursively flatten the layer tree into a display list with depth.
fn flatten_layers(layers: &[Layer]) -> Vec<FlatLayerEntry> {
    let mut out = Vec::new();
    flatten_layers_recursive(layers, 0, &mut out);
    out
}

fn flatten_layers_recursive(layers: &[Layer], depth: usize, out: &mut Vec<FlatLayerEntry>) {
    for layer in layers {
        let is_group = matches!(layer.kind, LayerKind::Group { .. });
        out.push(FlatLayerEntry {
            id: layer.id,
            name: layer.name.clone(),
            visible: layer.visible,
            opacity: layer.opacity,
            depth,
            is_group,
        });
        if let LayerKind::Group { children } = &layer.kind {
            flatten_layers_recursive(children, depth + 1, out);
        }
    }
}

const ALL_BLEND_MODES: &[BlendMode] = &[
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_blend_modes_listed() {
        assert_eq!(ALL_BLEND_MODES.len(), 12);
    }

    #[test]
    fn layer_property_sheet_for_document() {
        let doc = Document::new("Test", 100, 100);
        let sheet = build_layer_property_sheet(&doc);
        assert!(!sheet.is_empty());
        let layer_props = sheet.by_category("Layer");
        assert!(layer_props.len() >= 6);
    }

    #[test]
    fn layer_property_sheet_empty_when_no_active() {
        let mut doc = Document::new("Test", 100, 100);
        doc.active_layer = None;
        let sheet = build_layer_property_sheet(&doc);
        assert!(sheet.is_empty());
    }

    #[test]
    fn expr_eval_in_numeric_fields() {
        let result = muharrir::expr::eval_f64("10 + 5");
        assert_eq!(result.unwrap(), 15.0);

        let result = muharrir::expr::eval_or("2 * 3.5", 0.0);
        assert_eq!(result, 7.0);
    }

    #[test]
    fn flatten_layers_flat_list() {
        use rasa_core::layer::Layer;
        let layers = vec![
            Layer::new_raster("A", 10, 10),
            Layer::new_raster("B", 10, 10),
        ];
        let flat = flatten_layers(&layers);
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].depth, 0);
        assert_eq!(flat[1].depth, 0);
        assert!(!flat[0].is_group);
    }

    #[test]
    fn flatten_layers_with_group() {
        use rasa_core::layer::{Layer, LayerKind};
        let child1 = Layer::new_raster("Child 1", 10, 10);
        let child2 = Layer::new_raster("Child 2", 10, 10);
        let mut group = Layer::new_raster("Group", 10, 10);
        group.kind = LayerKind::Group {
            children: vec![child1, child2],
        };
        let bg = Layer::new_raster("Background", 10, 10);
        let layers = vec![bg, group];
        let flat = flatten_layers(&layers);
        // Background(0), Group(0), Child1(1), Child2(1)
        assert_eq!(flat.len(), 4);
        assert_eq!(flat[0].name, "Background");
        assert_eq!(flat[0].depth, 0);
        assert_eq!(flat[1].name, "Group");
        assert_eq!(flat[1].depth, 0);
        assert!(flat[1].is_group);
        assert_eq!(flat[2].name, "Child 1");
        assert_eq!(flat[2].depth, 1);
        assert_eq!(flat[3].name, "Child 2");
        assert_eq!(flat[3].depth, 1);
    }
}
