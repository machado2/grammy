#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod api;
mod config;
mod suggestion;

use app::GrammyApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Grammy"),
        ..Default::default()
    };

    eframe::run_native(
        "Grammy",
        options,
        Box::new(|cc| Ok(Box::new(GrammyApp::new(cc)))),
    )
}
