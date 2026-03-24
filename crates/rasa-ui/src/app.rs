use std::path::PathBuf;

use egui;
use muharrir::notification::{NotificationLog, Severity, Toasts};
use muharrir::prefs::PrefsStore;
use muharrir::recent::RecentFiles;
use muharrir::selection::PanelStates;
use rasa_core::Document;
use rasa_core::layer::Layer;
use serde::{Deserialize, Serialize};

use rasa_engine::filter::FilterRegistry;

use crate::canvas::CanvasState;
use crate::panels;
use crate::plugin::{PluginContext, PluginManager};
use crate::tool::ToolRegistry;
use crate::tools::ActiveTool;

/// Persisted user preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPrefs {
    pub show_pixel_grid: bool,
    pub show_rulers: bool,
    pub brush_size: f32,
    pub brush_opacity: f32,
    pub brush_hardness: f32,
    pub primary_color: [f32; 3],
}

impl Default for AppPrefs {
    fn default() -> Self {
        Self {
            show_pixel_grid: false,
            show_rulers: true,
            brush_size: 10.0,
            brush_opacity: 1.0,
            brush_hardness: 0.8,
            primary_color: [0.0, 0.0, 0.0],
        }
    }
}

/// Main application state.
pub struct RasaApp {
    pub document: Option<Document>,
    pub canvas: CanvasState,
    pub active_tool: ActiveTool,
    pub prefs: AppPrefs,
    pub filter_registry: FilterRegistry,
    pub tool_registry: ToolRegistry,
    pub plugin_manager: PluginManager,
    /// Toast notifications (ephemeral, auto-expire).
    pub toasts: Toasts,
    /// Persistent notification log.
    pub notifications: NotificationLog,
    /// Recently opened files.
    pub recent_files: RecentFiles,
    /// Panel visibility state.
    pub panel_states: PanelStates,
    /// Set when the user clicks Quit with unsaved changes — next Quit confirms.
    confirm_quit: bool,
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

        // Load persisted preferences
        let prefs_path = muharrir::prefs::config_dir("rasa").join("prefs.json");
        let prefs: AppPrefs = PrefsStore::load_or_default(&prefs_path);

        // Load recent files
        let recent_path = muharrir::prefs::config_dir("rasa").join("recent.json");
        let recent_files: RecentFiles = PrefsStore::load_or_default(&recent_path);

        // Panel visibility
        let mut panel_states = PanelStates::new();
        panel_states.register("tool_palette", true);
        panel_states.register("layer_panel", true);
        panel_states.register("properties", true);
        panel_states.register("history", true);

        Self {
            document: Some(Document::new("Untitled", 800, 600)),
            canvas: CanvasState::default(),
            active_tool: ActiveTool::Brush,
            prefs,
            filter_registry,
            tool_registry,
            plugin_manager,
            toasts: Toasts::new(),
            notifications: NotificationLog::new(),
            recent_files,
            panel_states,
            confirm_quit: false,
        }
    }

    /// Push a notification and show a toast.
    fn notify(&mut self, message: impl Into<String>, severity: Severity) {
        let msg: String = message.into();
        self.toasts.push(&msg, severity);
        self.notifications.push(&msg, severity, "rasa-ui");
    }

    /// Save preferences to disk.
    fn save_prefs(&self) {
        let prefs_path = muharrir::prefs::config_dir("rasa").join("prefs.json");
        let _ = PrefsStore::save(&self.prefs, &prefs_path);
    }

    /// Save recent files to disk.
    fn save_recent(&self) {
        let recent_path = muharrir::prefs::config_dir("rasa").join("recent.json");
        let _ = PrefsStore::save(&self.recent_files, &recent_path);
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New (Ctrl+N)").clicked() {
                    self.document = Some(Document::new("Untitled", 800, 600));
                    self.notify("New document created", Severity::Info);
                    ui.close_menu();
                }
                if ui.button("Open... (Ctrl+O)").clicked() {
                    if let Some(path) = rfd_open_file() {
                        match rasa_storage::import::import_image(&path) {
                            Ok(doc) => {
                                self.notify(format!("Opened: {}", path.display()), Severity::Info);
                                self.document = Some(doc);
                                self.recent_files.add(&path);
                                self.save_recent();
                            }
                            Err(e) => {
                                self.notify(format!("Error: {e}"), Severity::Error);
                            }
                        }
                    }
                    ui.close_menu();
                }
                // Recent files submenu
                if !self.recent_files.is_empty() {
                    ui.menu_button("Open Recent", |ui| {
                        let entries: Vec<PathBuf> = self.recent_files.entries().to_vec();
                        for path in &entries {
                            let label = path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.display().to_string());
                            if ui
                                .button(&label)
                                .on_hover_text(path.display().to_string())
                                .clicked()
                            {
                                match rasa_storage::import::import_image(path) {
                                    Ok(doc) => {
                                        self.notify(
                                            format!("Opened: {}", path.display()),
                                            Severity::Info,
                                        );
                                        self.document = Some(doc);
                                        self.recent_files.add(path);
                                        self.save_recent();
                                    }
                                    Err(e) => {
                                        self.notify(format!("Error: {e}"), Severity::Error);
                                    }
                                }
                                ui.close_menu();
                            }
                        }
                    });
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
                                        self.notify(format!("Export error: {e}"), Severity::Error);
                                        return;
                                    }
                                };
                            match rasa_storage::export::export_buffer(&composited, &path, &settings)
                            {
                                Ok(()) => {
                                    self.notify(
                                        format!("Exported: {}", path.display()),
                                        Severity::Info,
                                    );
                                    if let Some(doc) = &mut self.document {
                                        doc.mark_clean();
                                    }
                                }
                                Err(e) => {
                                    self.notify(format!("Export error: {e}"), Severity::Error);
                                }
                            }
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                let has_unsaved = self.document.as_ref().is_some_and(|d| d.dirty.is_dirty());
                let quit_label = if has_unsaved && !self.confirm_quit {
                    "Quit (unsaved changes)"
                } else {
                    "Quit"
                };
                if ui.button(quit_label).clicked() {
                    self.save_prefs();
                    if has_unsaved && !self.confirm_quit {
                        self.confirm_quit = true;
                        self.notify(
                            "Unsaved changes — click Quit again to discard",
                            Severity::Warning,
                        );
                        ui.close_menu();
                    } else {
                        std::process::exit(0);
                    }
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
                ui.checkbox(&mut self.prefs.show_pixel_grid, "Pixel Grid");
                ui.checkbox(&mut self.prefs.show_rulers, "Rulers");
                ui.separator();
                // Panel visibility toggles
                for name in ["tool_palette", "layer_panel", "properties", "history"] {
                    let visible = self.panel_states.is_visible(name);
                    let label = match name {
                        "tool_palette" => "Tool Palette",
                        "layer_panel" => "Layer Panel",
                        "properties" => "Properties",
                        "history" => "History",
                        _ => name,
                    };
                    if ui.selectable_label(visible, label).clicked() {
                        self.panel_states.toggle(name);
                    }
                }
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
        // Any key press resets the quit confirmation
        self.confirm_quit = false;
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

    fn show_toasts(&mut self, ctx: &egui::Context) {
        self.toasts.gc();
        if self.toasts.is_empty() {
            return;
        }
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::Vec2::new(-10.0, -10.0))
            .show(ctx, |ui| {
                for toast in self.toasts.active() {
                    let color = match toast.severity {
                        Severity::Info => egui::Color32::from_rgb(60, 140, 200),
                        Severity::Warning => egui::Color32::from_rgb(220, 160, 40),
                        Severity::Error => egui::Color32::from_rgb(200, 60, 60),
                        _ => egui::Color32::GRAY,
                    };
                    egui::Frame::NONE
                        .fill(color)
                        .inner_margin(8.0)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.colored_label(egui::Color32::WHITE, &toast.message);
                        });
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

        // Status bar — show dirty indicator and doc info
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(doc) = &self.document
                    && doc.dirty.is_dirty()
                {
                    ui.label("(modified)");
                }
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
        if self.panel_states.is_visible("tool_palette") {
            egui::SidePanel::left("tool_palette")
                .default_width(48.0)
                .resizable(false)
                .show(ctx, |ui| {
                    panels::tool_palette(ui, &mut self.active_tool);
                });
        }

        // Layer panel (right)
        if self.panel_states.is_visible("layer_panel") {
            egui::SidePanel::right("layer_panel")
                .default_width(240.0)
                .show(ctx, |ui| {
                    if let Some(doc) = &mut self.document {
                        panels::layer_panel(ui, doc);
                        ui.separator();
                        if self.panel_states.is_visible("properties") {
                            panels::properties_panel(
                                ui,
                                &self.active_tool,
                                &mut self.prefs.brush_size,
                                &mut self.prefs.brush_opacity,
                                &mut self.prefs.brush_hardness,
                                &mut self.prefs.primary_color,
                            );
                            ui.separator();
                        }
                        if self.panel_states.is_visible("history") {
                            panels::history_panel(ui, doc);
                        }
                    }
                });
        }

        // Canvas (center)
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(doc) = &self.document {
                crate::canvas::canvas_viewport(
                    ui,
                    doc,
                    &mut self.canvas,
                    self.prefs.show_pixel_grid,
                    self.prefs.show_rulers,
                );
            } else {
                ui.centered_and_justified(|ui| {
                    ui.heading("No document open");
                });
            }
        });

        // Toast overlay
        self.show_toasts(ctx);

        // Request repaint while toasts are active (for progress/fade animation)
        if !self.toasts.is_empty() {
            ctx.request_repaint();
        }
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
        let mut filter_registry = FilterRegistry::new();
        rasa_engine::filter_builtins::register_builtins(&mut filter_registry);
        let mut tool_registry = ToolRegistry::new();
        crate::tool_builtins::register_builtins(&mut tool_registry);

        let app = RasaApp {
            document: Some(Document::new("Test", 100, 100)),
            canvas: CanvasState::default(),
            active_tool: ActiveTool::Brush,
            prefs: AppPrefs::default(),
            filter_registry,
            tool_registry,
            plugin_manager: PluginManager::new(),
            toasts: Toasts::new(),
            notifications: NotificationLog::new(),
            recent_files: RecentFiles::new(),
            panel_states: PanelStates::new(),
            confirm_quit: false,
        };
        assert!(app.document.is_some());
        assert_eq!(app.active_tool, ActiveTool::Brush);
        assert_eq!(app.prefs.brush_size, 10.0);
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

    #[test]
    fn app_prefs_default() {
        let prefs = AppPrefs::default();
        assert!(!prefs.show_pixel_grid);
        assert!(prefs.show_rulers);
        assert_eq!(prefs.brush_size, 10.0);
    }

    #[test]
    fn notifications_work() {
        let mut toasts = Toasts::new();
        toasts.push("test", Severity::Info);
        assert!(!toasts.is_empty());
        assert_eq!(toasts.len(), 1);
    }

    #[test]
    fn recent_files_work() {
        let mut recent = RecentFiles::new();
        recent.add("/tmp/test.png");
        assert_eq!(recent.len(), 1);
        assert_eq!(
            recent.most_recent().unwrap().to_str().unwrap(),
            "/tmp/test.png"
        );
    }

    #[test]
    fn panel_states_work() {
        let mut panels = PanelStates::new();
        panels.register("test_panel", true);
        assert!(panels.is_visible("test_panel"));
        panels.toggle("test_panel");
        assert!(!panels.is_visible("test_panel"));
    }
}
