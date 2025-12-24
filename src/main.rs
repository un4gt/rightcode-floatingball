#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod api;
mod app;
mod autostart;
mod ball;
mod config;
mod executor;
mod platform;
mod tray;

fn main() -> iced::Result {
    app::run()
}
