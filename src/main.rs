#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use grammy::app;

use iced::window;
use iced::Size;

fn main() -> iced::Result {
    iced::application(app::new, app::update, app::view)
        .title("Grammy")
        .theme(app::theme)
        .subscription(app::subscription)
        .window(window::Settings {
            size: Size::new(1200.0, 800.0),
            min_size: Some(Size::new(800.0, 600.0)),
            exit_on_close_request: false,
            icon: load_icon(),
            ..Default::default()
        })
        .settings(app::settings())
        .run()
}

fn load_icon() -> Option<iced::window::Icon> {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    iced::window::icon::from_rgba(rgba, width, height).ok()
}
