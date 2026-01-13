#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use grammy::app::App;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Grammy",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    // Load icon for the window
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes)
        .expect("Failed to load icon")
        .to_rgba8();

    let (width, height) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }
}
