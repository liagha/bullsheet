#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod autostart;
mod calculator;
mod decorations;
mod icon;
mod instance;
mod popup;
mod render;
mod settings;
mod shortcut;
mod station;
mod surface;
mod tray;
mod typography;
mod update;

fn main() -> eframe::Result {
    application::Application::run()
}
