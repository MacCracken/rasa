pub mod app;
pub mod canvas;
pub mod panels;
pub mod plugin;
pub mod tool;
pub mod tool_builtins;
pub mod tools;

/// Launch the Rasa desktop application.
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rasa")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rasa",
        options,
        Box::new(|cc| Ok(Box::new(app::RasaApp::new(cc)))),
    )
}
