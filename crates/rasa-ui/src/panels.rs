use egui;
use rasa_core::Document;
use rasa_core::color::BlendMode;
use rasa_core::layer::Layer;

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

    // Layer list (top = highest, bottom = lowest)
    let active_id = doc.active_layer;
    let mut action: Option<LayerAction> = None;

    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for i in (0..doc.layers.len()).rev() {
                let layer = &doc.layers[i];
                let is_active = active_id == Some(layer.id);
                let layer_id = layer.id;

                let frame = if is_active {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_gray(60))
                        .inner_margin(4.0)
                } else {
                    egui::Frame::NONE.inner_margin(4.0)
                };

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Visibility toggle
                        let mut visible = layer.visible;
                        if ui.checkbox(&mut visible, "").changed() {
                            action = Some(LayerAction::SetVisibility(layer_id, visible));
                        }

                        // Layer name (click to select)
                        let resp = ui.selectable_label(is_active, &layer.name);
                        if resp.clicked() {
                            action = Some(LayerAction::Select(layer_id));
                        }

                        // Opacity
                        ui.label(format!("{:.0}%", layer.opacity * 100.0));
                    });
                });
            }
        });

    // Apply deferred actions
    if let Some(a) = action {
        match a {
            LayerAction::Select(id) => {
                doc.active_layer = Some(id);
            }
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
    }
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
            ui.add(egui::Slider::new(size, 1.0..=200.0).logarithmic(true));
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

    // Hex input
    let hex = format!(
        "#{:02X}{:02X}{:02X}",
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
    );
    ui.label(&hex);
}

/// History panel — undo/redo controls.
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
}

enum LayerAction {
    Select(uuid::Uuid),
    SetVisibility(uuid::Uuid, bool),
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
}
