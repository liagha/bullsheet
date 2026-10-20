const SHORTCUT_KEY: &str = "shortcut";

pub struct Application {
    station: Box<dyn crate::station::Station>,
    settings: crate::settings::Settings,
    decorations: crate::decorations::Decorations,
    popup: crate::popup::Popup,
    tray: Option<crate::tray::Tray>,
    updater: crate::update::Updater,
    startup_hidden: bool,
    autostart_shown: bool,
}

impl Application {
    pub fn run() -> eframe::Result {
        let _instance = match crate::instance::Instance::new() {
            Ok(Some(instance)) => Some(instance),
            Ok(None) => return Ok(()),
            Err(error) => {
                eprintln!("{error}");
                None
            }
        };
        let hidden = std::env::args().any(|argument| argument == "--hidden");
        let viewport = eframe::egui::ViewportBuilder::default()
            .with_title("bullsheet")
            .with_icon(std::sync::Arc::new(crate::icon::Icon::new().data()))
            .with_inner_size([640.0, 440.0])
            .with_decorations(false)
            .with_transparent(false)
            .with_always_on_top()
            .with_taskbar(false)
            .with_resizable(false)
            .with_visible(!hidden)
            .with_active(!hidden);
        let options = eframe::NativeOptions {
            viewport,
            renderer: eframe::Renderer::Glow,
            centered: true,
            persist_window: true,
            ..Default::default()
        };
        eframe::run_native(
            "bullsheet",
            options,
            Box::new(move |context| {
                Ok(Box::new(Self::new(context, hidden)) as Box<dyn eframe::App>)
            }),
        )
    }

    fn new(context: &eframe::CreationContext<'_>, hidden: bool) -> Self {
        use crate::autostart::Autostart;
        use crate::decorations::Decorations;
        use crate::popup::{Popup, Visibility};
        use crate::settings::Settings;
        use crate::shortcut::{Combination, Shortcut};
        use crate::station::Station;
        use crate::tray::Tray;
        use crate::update::Updater;
        use eframe::egui::{FontData, FontDefinitions, FontFamily};
        use std::sync::Arc;
        let family = "JetBrains Mono";
        let persian = "Vazirmatn";
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            family.to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/JetBrainsMono-Regular.ttf"
            ))),
        );
        fonts.font_data.insert(
            persian.to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/Vazirmatn-Regular.ttf"
            ))),
        );
        for font_family in [FontFamily::Proportional, FontFamily::Monospace] {
            let families = fonts.families.get_mut(&font_family).unwrap();
            families.insert(0, family.to_owned());
            families.push(persian.to_owned());
        }
        context.egui_ctx.set_fonts(fonts);
        let mut visuals = eframe::egui::Visuals::dark();
        visuals.selection.bg_fill = eframe::egui::Color32::from_rgb(72, 72, 72);
        visuals.selection.stroke =
            eframe::egui::Stroke::new(1.0, eframe::egui::Color32::from_rgb(224, 227, 233));
        context.egui_ctx.set_visuals(visuals);
        let combination: Combination = context
            .storage
            .and_then(|storage| storage.get_string(SHORTCUT_KEY))
            .and_then(|text| text.parse().ok())
            .unwrap_or_default();
        let shortcut = Shortcut::new(&context.egui_ctx, combination).ok();
        let autostart = Autostart::new().ok();
        let startup = autostart
            .as_ref()
            .map(|autostart| autostart.enabled())
            .unwrap_or(false);
        let tray = Tray::new(&context.egui_ctx, startup).ok();
        let updater = Updater::new(&context.egui_ctx);
        let settings = Settings::new(combination, shortcut, autostart);
        let visible = !hidden || !settings.registered();
        let station: Box<dyn Station> = Box::new(crate::calculator::Calculator::new(
            &settings,
            &context.egui_ctx,
            visible,
        ));
        Self {
            station,
            settings,
            decorations: Decorations::new(),
            popup: Popup::new(if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }),
            tray,
            updater,
            startup_hidden: !visible,
            autostart_shown: startup,
        }
    }

    fn toggle(&mut self, context: &eframe::egui::Context) {
        use crate::popup::Visibility;
        self.popup.toggle(context);
        if self.popup.visibility() == Visibility::Visible {
            self.station.focus();
        }
        context.request_repaint();
    }
}

impl eframe::App for Application {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(SHORTCUT_KEY, self.settings.combination().to_string());
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn logic(&mut self, context: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use crate::tray::Action;
        if self.startup_hidden {
            self.popup.hide(context);
            self.startup_hidden = false;
        }
        if self.settings.requested() {
            self.toggle(context);
        }
        match self.tray.as_ref().and_then(|tray| tray.action()) {
            Some(Action::Toggle) => self.toggle(context),
            Some(Action::Autostart) => {
                let enabled = !self.settings.autostart_enabled();
                if let Some(autostart) = self.settings.autostart() {
                    let _ = if enabled {
                        autostart.enable()
                    } else {
                        autostart.disable()
                    };
                }
            }
            Some(Action::Quit) => {
                context.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            }
            None => {}
        }
        if let Some(report) = self.updater.report() {
            self.settings.set_update(Some(&report));
        }
        if let Some(tray) = &self.tray {
            let enabled = self.settings.autostart_enabled();
            if enabled != self.autostart_shown {
                tray.set_autostart(enabled);
                self.autostart_shown = enabled;
            }
        }
    }

    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        eframe::egui::Color32::from_rgb(32, 32, 32).to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        use eframe::egui::{self, Color32, RichText, Sense, Stroke, Vec2};
        self.decorations.apply(frame);
        let context = ui.ctx().clone();
        if context.input(|input| input.key_pressed(egui::Key::Escape)) && self.settings.registered()
        {
            self.popup.hide(&context);
        }
        if context.input(|input| input.modifiers.command && input.key_pressed(egui::Key::Q)) {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let frame = egui::Frame::new()
            .fill(Color32::from_rgb(32, 32, 32))
            .stroke(Stroke::new(1.0, Color32::from_rgb(64, 64, 64)))
            .corner_radius(12)
            .inner_margin(12);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            ui.horizontal(|ui| {
                let drag_width = (ui.available_width() - 270.0).max(40.0);
                let drag = ui.allocate_response(Vec2::new(drag_width, 24.0), Sense::drag());
                if drag.drag_started() {
                    context.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.add(
                        egui::Button::new(
                            RichText::new("×")
                                .size(16.0)
                                .color(Color32::from_rgb(170, 176, 188)),
                        )
                        .frame(false),
                    );
                    if close.clicked() {
                        context.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if self.settings.registered() {
                        let hide = ui.add(
                            egui::Button::new(
                                RichText::new("—")
                                    .size(14.0)
                                    .color(Color32::from_rgb(170, 176, 188)),
                            )
                            .frame(false),
                        );
                        if hide.clicked() {
                            self.popup.hide(&context);
                        }
                    }
                    if self.settings.registered() {
                        ui.label(
                            RichText::new(self.settings.combination().to_string())
                                .size(11.0)
                                .color(Color32::from_rgb(110, 116, 128)),
                        );
                    } else {
                        ui.label(
                            RichText::new("Shortcut unavailable")
                                .size(11.0)
                                .color(Color32::from_rgb(255, 123, 114)),
                        );
                    }
                });
            });
            ui.add_space(8.0);
            self.station.show(ui);
        });
    }
}
