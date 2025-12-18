#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod config;
mod suggestion;

mod app;

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
            ..Default::default()
        })
        .settings(app::settings())
        .run()
}
