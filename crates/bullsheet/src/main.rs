#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod autostart;
mod decorations;
mod icon;
mod instance;
mod popup;
mod render;
mod shortcut;
mod surface;
mod tray;
mod update;

fn main() -> eframe::Result {
    application::Application::run()
}
