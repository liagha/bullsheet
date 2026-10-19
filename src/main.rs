mod application;
mod autostart;
mod calculator;
mod decorations;
mod icon;
mod instance;
mod popup;
mod shortcut;
mod tray;
mod update;

fn main() -> eframe::Result {
    application::Application::run()
}
