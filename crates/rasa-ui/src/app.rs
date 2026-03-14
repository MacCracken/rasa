use std::path::PathBuf;

use egui;
use rasa_core::color::BlendMode;
use rasa_core::layer::Layer;
use rasa_core::Document;

use crate::canvas::CanvasState;
use crate::panels;
use crate::tools::ActiveTool;

/// Main application state.
pub struct RasaApp {
    pub document: Option<Document>,
    pub canvas: CanvasState,
    pub active_tool: ActiveTool,
    pub brush_size: f32,
    pub brush_opacity: f32,
    pub brush_hardness: f32,
    pub primary_color: [f32; 3],
    pub show_pixel_grid: bool,
    pub show_rulers: bool,
    pub status_message: String,
}

impl RasaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            document: Some(Document::new("Untitled", 800, 600)),
            canvas: CanvasState::default(),
            active_tool: ActiveTool::Brush,
            brush_size: 10.0,
            brush_opacity: 1.0,
            brush_hardness: 0.8,
            primary_color: [0.0, 0.0, 0.0],
            show_pixel_grid: false,
            show_rulers: true,
            status_message: "Ready".into(),
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New (Ctrl+N)").clicked() {
                    self.document = Some(Document::new("Untitled", 800, 600));
                    self.status_message = "New document created".into();
                    ui.close_menu();
                }
                if ui.button("Open... (Ctrl+O)").clicked() {
                    if let Some(path) = rfd_open_file() {
                        match rasa_storage::import::import_image(&path) {
                            Ok(doc) => {
                                self.status_message =
                                    format!("Opened: {}", path.display());
                                self.document = Some(doc);
                            }
                            Err(e) => {
                                self.status_message = format!("Error: {e}");
                            }
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Export... (Ctrl+Shift+E)").clicked() {
                    if let Some(doc) = &self.document {
                        if let Some(path) = rfd_save_file() {
                            let composited = rasa_engine::compositor::composite(doc);
                            let format = rasa_storage::format::ImageFormat::from_path(&path);
                            if let Some(fmt) = format {
                                let settings =
                                    rasa_storage::format::ExportSettings::for_format(fmt);
                                match rasa_storage::export::export_buffer(
                                    &composited,
                                    &path,
                                    &settings,
                                ) {
                                    Ok(()) => {
                                        self.status_message =
                                            format!("Exported: {}", path.display());
                                    }
                                    Err(e) => {
                                        self.status_message = format!("Export error: {e}");
                                    }
                                }
                            }
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });

            ui.menu_button("Edit", |ui| {
                let can_undo = self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.can_undo());
                let can_redo = self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.can_redo());
                if ui
                    .add_enabled(can_undo, egui::Button::new("Undo (Ctrl+Z)"))
                    .clicked()
                {
                    if let Some(doc) = &mut self.document {
                        let _ = doc.undo();
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(can_redo, egui::Button::new("Redo (Ctrl+Shift+Z)"))
                    .clicked()
                {
                    if let Some(doc) = &mut self.document {
                        let _ = doc.redo();
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_pixel_grid, "Pixel Grid");
                ui.checkbox(&mut self.show_rulers, "Rulers");
                ui.separator();
                if ui.button("Zoom In (+)").clicked() {
                    self.canvas.zoom *= 1.25;
                    ui.close_menu();
                }
                if ui.button("Zoom Out (-)").clicked() {
                    self.canvas.zoom /= 1.25;
                    ui.close_menu();
                }
                if ui.button("Fit to Window").clicked() {
                    self.canvas.zoom = 1.0;
                    self.canvas.pan = egui::Vec2::ZERO;
                    ui.close_menu();
                }
            });

            ui.menu_button("Layer", |ui| {
                if ui.button("New Layer").clicked() {
                    if let Some(doc) = &mut self.document {
                        let (w, h) = (doc.size.width, doc.size.height);
                        let count = doc.layers.len();
                        doc.add_layer(Layer::new_raster(
                            format!("Layer {}", count),
                            w,
                            h,
                        ));
                    }
                    ui.close_menu();
                }
                if ui.button("Duplicate Layer").clicked() {
                    if let Some(doc) = &mut self.document {
                        if let Some(id) = doc.active_layer {
                            let _ = doc.duplicate_layer(id);
                        }
                    }
                    ui.close_menu();
                }
                if ui.button("Merge Down").clicked() {
                    if let Some(doc) = &mut self.document {
                        if let Some(id) = doc.active_layer {
                            let _ = doc.merge_down(id);
                        }
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Filter", |ui| {
                for (label, filter_fn) in [
                    ("Invert", filter_invert as fn(&mut Document)),
                    ("Grayscale", filter_grayscale),
                ] {
                    if ui.button(label).clicked() {
                        if let Some(doc) = &mut self.document {
                            filter_fn(doc);
                        }
                        ui.close_menu();
                    }
                }
            });
        });
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let modifiers = ctx.input(|i| i.modifiers);

        ctx.input(|i| {
            // Ctrl+Z = Undo
            if modifiers.ctrl && i.key_pressed(egui::Key::Z) && !modifiers.shift {
                if let Some(doc) = &mut self.document {
                    let _ = doc.undo();
                }
            }
            // Ctrl+Shift+Z = Redo
            if modifiers.ctrl && modifiers.shift && i.key_pressed(egui::Key::Z) {
                if let Some(doc) = &mut self.document {
                    let _ = doc.redo();
                }
            }
            // B = Brush
            if i.key_pressed(egui::Key::B) && !modifiers.ctrl {
                self.active_tool = ActiveTool::Brush;
            }
            // E = Eraser
            if i.key_pressed(egui::Key::E) && !modifiers.ctrl {
                self.active_tool = ActiveTool::Eraser;
            }
            // M = Move/Pan
            if i.key_pressed(egui::Key::M) && !modifiers.ctrl {
                self.active_tool = ActiveTool::Move;
            }
            // I = Eyedropper
            if i.key_pressed(egui::Key::I) && !modifiers.ctrl {
                self.active_tool = ActiveTool::Eyedropper;
            }
            // G = Gradient
            if i.key_pressed(egui::Key::G) && !modifiers.ctrl {
                self.active_tool = ActiveTool::Gradient;
            }
            // C = Crop
            if i.key_pressed(egui::Key::C) && !modifiers.ctrl {
                self.active_tool = ActiveTool::Crop;
            }
            // + / - = Zoom
            if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                self.canvas.zoom *= 1.25;
            }
            if i.key_pressed(egui::Key::Minus) {
                self.canvas.zoom /= 1.25;
            }
        });
    }
}

impl eframe::App for RasaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard(ctx);

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.menu_bar(ui);
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{:.0}%", self.canvas.zoom * 100.0));
                    if let Some(doc) = &self.document {
                        ui.label(format!(
                            "{}x{} | {} layers",
                            doc.size.width,
                            doc.size.height,
                            doc.layers.len()
                        ));
                    }
                });
            });
        });

        // Tool palette (left)
        egui::SidePanel::left("tool_palette")
            .default_width(48.0)
            .resizable(false)
            .show(ctx, |ui| {
                panels::tool_palette(ui, &mut self.active_tool);
            });

        // Layer panel (right)
        egui::SidePanel::right("layer_panel")
            .default_width(240.0)
            .show(ctx, |ui| {
                if let Some(doc) = &mut self.document {
                    panels::layer_panel(ui, doc);
                    ui.separator();
                    panels::properties_panel(
                        ui,
                        &self.active_tool,
                        &mut self.brush_size,
                        &mut self.brush_opacity,
                        &mut self.brush_hardness,
                        &mut self.primary_color,
                    );
                    ui.separator();
                    panels::history_panel(ui, doc);
                }
            });

        // Canvas (center)
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(doc) = &self.document {
                crate::canvas::canvas_viewport(
                    ui,
                    doc,
                    &mut self.canvas,
                    self.show_pixel_grid,
                    self.show_rulers,
                );
            } else {
                ui.centered_and_justified(|ui| {
                    ui.heading("No document open");
                });
            }
        });
    }
}

// ── Helpers ──

fn filter_invert(doc: &mut Document) {
    if let Some(id) = doc.active_layer {
        if let Some(buf) = doc.get_pixels_mut(id) {
            rasa_engine::filters::invert(buf);
        }
    }
}

fn filter_grayscale(doc: &mut Document) {
    if let Some(id) = doc.active_layer {
        if let Some(buf) = doc.get_pixels_mut(id) {
            rasa_engine::filters::grayscale(buf);
        }
    }
}

/// Simple file open dialog (returns None if no GUI dialog available).
fn rfd_open_file() -> Option<PathBuf> {
    // In headless/test environments, this returns None.
    // In a full desktop build, you'd use rfd::FileDialog here.
    None
}

/// Simple file save dialog.
fn rfd_save_file() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_default_state() {
        // Can't create full eframe context in tests, but verify state struct
        let app = RasaApp {
            document: Some(Document::new("Test", 100, 100)),
            canvas: CanvasState::default(),
            active_tool: ActiveTool::Brush,
            brush_size: 10.0,
            brush_opacity: 1.0,
            brush_hardness: 0.8,
            primary_color: [0.0, 0.0, 0.0],
            show_pixel_grid: false,
            show_rulers: true,
            status_message: "Ready".into(),
        };
        assert!(app.document.is_some());
        assert_eq!(app.active_tool, ActiveTool::Brush);
        assert_eq!(app.brush_size, 10.0);
    }

    #[test]
    fn filter_invert_runs() {
        let mut doc = Document::new("Test", 4, 4);
        filter_invert(&mut doc);
        // Should not panic, background layer gets inverted
    }

    #[test]
    fn filter_grayscale_runs() {
        let mut doc = Document::new("Test", 4, 4);
        filter_grayscale(&mut doc);
    }
}
