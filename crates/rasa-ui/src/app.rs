use std::path::PathBuf;

use egui;
use rasa_core::Document;
use rasa_core::layer::Layer;

use rasa_engine::filter::FilterRegistry;

use crate::canvas::CanvasState;
use crate::panels;
use crate::plugin::{PluginContext, PluginManager};
use crate::tool::ToolRegistry;
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
    pub filter_registry: FilterRegistry,
    pub tool_registry: ToolRegistry,
    pub plugin_manager: PluginManager,
}

impl RasaApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut filter_registry = FilterRegistry::new();
        rasa_engine::filter_builtins::register_builtins(&mut filter_registry);

        let mut tool_registry = ToolRegistry::new();
        crate::tool_builtins::register_builtins(&mut tool_registry);

        let mut providers = rasa_ai::registry::ProviderRegistry::new();
        let synapse_url = std::env::var("RASA_SYNAPSE_URL")
            .unwrap_or_else(|_| "http://localhost:8090".to_string());
        providers.register(Box::new(rasa_ai::provider_synapse::SynapseProvider::new(
            &synapse_url,
        )));

        let plugin_manager = PluginManager::new();
        // Allow plugins to register into all registries
        {
            let mut ctx = PluginContext {
                filters: &mut filter_registry,
                tools: &mut tool_registry,
                providers: &mut providers,
            };
            plugin_manager.init_all(&mut ctx);
        }

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
            filter_registry,
            tool_registry,
            plugin_manager,
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
                                self.status_message = format!("Opened: {}", path.display());
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
                    if let Some(doc) = &self.document
                        && let Some(path) = rfd_save_file()
                    {
                        let composited = rasa_engine::compositor::composite(doc);
                        let format = rasa_storage::format::ImageFormat::from_path(&path);
                        if let Some(fmt) = format {
                            let settings =
                                match rasa_storage::format::ExportSettings::for_format(fmt) {
                                    Ok(s) => s,
                                    Err(e) => {
                                        self.status_message = format!("Export error: {e}");
                                        return;
                                    }
                                };
                            match rasa_storage::export::export_buffer(&composited, &path, &settings)
                            {
                                Ok(()) => {
                                    self.status_message = format!("Exported: {}", path.display());
                                }
                                Err(e) => {
                                    self.status_message = format!("Export error: {e}");
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
                let can_undo = self.document.as_ref().is_some_and(|d| d.can_undo());
                let can_redo = self.document.as_ref().is_some_and(|d| d.can_redo());
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
                        doc.add_layer(Layer::new_raster(format!("Layer {}", count), w, h));
                    }
                    ui.close_menu();
                }
                if ui.button("Duplicate Layer").clicked() {
                    if let Some(doc) = &mut self.document
                        && let Some(id) = doc.active_layer
                    {
                        let _ = doc.duplicate_layer(id);
                    }
                    ui.close_menu();
                }
                if ui.button("Merge Down").clicked() {
                    if let Some(doc) = &mut self.document
                        && let Some(id) = doc.active_layer
                    {
                        let _ = doc.merge_down(id);
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Filter", |ui| {
                let names: Vec<String> = self
                    .filter_registry
                    .list_filters()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                for name in &names {
                    if ui.button(name.as_str()).clicked() {
                        if let Some(doc) = &mut self.document
                            && let Some(id) = doc.active_layer
                            && let Some(buf) = doc.get_pixels_mut(id)
                            && let Some(filter) = self.filter_registry.filter_by_name(name)
                        {
                            filter.apply(buf);
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
            if modifiers.ctrl
                && i.key_pressed(egui::Key::Z)
                && !modifiers.shift
                && let Some(doc) = &mut self.document
            {
                let _ = doc.undo();
            }
            // Ctrl+Shift+Z = Redo
            if modifiers.ctrl
                && modifiers.shift
                && i.key_pressed(egui::Key::Z)
                && let Some(doc) = &mut self.document
            {
                let _ = doc.redo();
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

/// Simple file open dialog (returns None if no GUI dialog available).
fn rfd_open_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(
            "Images",
            &[
                "png", "jpg", "jpeg", "webp", "tiff", "tif", "bmp", "gif", "psd",
            ],
        )
        .add_filter("Rasa Project", &["rasa"])
        .add_filter("All Files", &["*"])
        .pick_file()
}

/// Simple file save dialog.
fn rfd_save_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("PNG", &["png"])
        .add_filter("JPEG", &["jpg", "jpeg"])
        .add_filter("WebP", &["webp"])
        .add_filter("TIFF", &["tiff", "tif"])
        .add_filter("BMP", &["bmp"])
        .add_filter("Rasa Project", &["rasa"])
        .save_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_default_state() {
        // Can't create full eframe context in tests, but verify state struct
        let mut filter_registry = FilterRegistry::new();
        rasa_engine::filter_builtins::register_builtins(&mut filter_registry);
        let mut tool_registry = ToolRegistry::new();
        crate::tool_builtins::register_builtins(&mut tool_registry);

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
            filter_registry,
            tool_registry,
            plugin_manager: PluginManager::new(),
        };
        assert!(app.document.is_some());
        assert_eq!(app.active_tool, ActiveTool::Brush);
        assert_eq!(app.brush_size, 10.0);
        assert_eq!(app.filter_registry.len(), 4);
        assert_eq!(app.tool_registry.len(), 10);
    }

    #[test]
    fn filter_registry_applies_to_doc() {
        let mut reg = FilterRegistry::new();
        rasa_engine::filter_builtins::register_builtins(&mut reg);
        let mut doc = Document::new("Test", 4, 4);
        if let Some(id) = doc.active_layer
            && let Some(buf) = doc.get_pixels_mut(id)
        {
            let filter = reg.filter_by_name("Invert").unwrap();
            filter.apply(buf);
        }
    }

    #[test]
    fn filter_registry_grayscale_applies() {
        let mut reg = FilterRegistry::new();
        rasa_engine::filter_builtins::register_builtins(&mut reg);
        let mut doc = Document::new("Test", 4, 4);
        if let Some(id) = doc.active_layer
            && let Some(buf) = doc.get_pixels_mut(id)
        {
            let filter = reg.filter_by_name("Grayscale").unwrap();
            filter.apply(buf);
        }
    }
}
