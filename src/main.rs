mod application;
mod autostart;
mod calculator;
mod instance;
mod popup;
mod shortcut;

fn main() -> eframe::Result {
    application::Application::run()
}
