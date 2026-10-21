#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod digits;
mod fonts;
mod instance;
mod render;
mod settings;
mod sheet;
mod update;
mod word;
mod writing;

fn main() {
    crate::app::Application::run();
}
