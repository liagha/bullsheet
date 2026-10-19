const ANGLE_KEY: &str = "angle";
const SHORTCUT_KEY: &str = "shortcut";

pub(crate) struct Application {
    source: evaluator::Source,
    calculator: evaluator::Calculator,
    results: evaluator::Results,
    angle: evaluator::Angle,
    renderers: crate::render::Renderers,
    decorations: crate::decorations::Decorations,
    combination: crate::shortcut::Combination,
    popup: crate::popup::Popup,
    shortcut: Option<crate::shortcut::Shortcut>,
    shortcut_failure: Option<crate::render::Presentation>,
    settings_open: bool,
    settings_shortcut: String,
    settings_failure: Option<element::Failure>,
    autostart: Option<crate::autostart::Autostart>,
    autostart_failure: Option<element::Failure>,
    tray: Option<crate::tray::Tray>,
    tray_failure: Option<element::Failure>,
    updater: crate::update::Updater,
    update_report: Option<crate::update::Report>,
    startup_hidden: bool,
    focus_requested: bool,
}

impl Application {
    pub(crate) fn run() -> eframe::Result {
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
        use crate::render::{Presentation, Renderers};
        use crate::shortcut::{Combination, Shortcut};
        use crate::tray::Tray;
        use crate::update::Updater;
        use eframe::egui::{FontData, FontDefinitions, FontFamily};
        use element::Failure;
        use evaluator::{Angle, Calculator, Source};
        use std::sync::Arc;
        let family = "JetBrains Mono";
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            family.to_owned(),
            Arc::new(FontData::from_static(include_bytes!(
                "../assets/JetBrainsMono-Regular.ttf"
            ))),
        );
        for font_family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .get_mut(&font_family)
                .unwrap()
                .insert(0, family.to_owned());
        }
        context.egui_ctx.set_fonts(fonts);
        context.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
        let angle: Angle = context
            .storage
            .and_then(|storage| storage.get_string(ANGLE_KEY))
            .and_then(|text| text.parse().ok())
            .unwrap_or_default();
        let combination: Combination = context
            .storage
            .and_then(|storage| storage.get_string(SHORTCUT_KEY))
            .and_then(|text| text.parse().ok())
            .unwrap_or_default();
        let (shortcut, shortcut_failure) =
            match Shortcut::new(&context.egui_ctx, combination) {
                Ok(shortcut) => (Some(shortcut), None),
                Err(failure) => {
                    (None, Some(Presentation::new(failure.to_string())))
                }
            };
        let (autostart, autostart_failure) = match Autostart::new() {
            Ok(autostart) => (Some(autostart), None),
            Err(error) => (None, Some(Failure::new(error.to_string()))),
        };
        let startup = autostart
            .as_ref()
            .map(|autostart| autostart.enabled())
            .unwrap_or(false);
        let (tray, tray_failure) = match Tray::new(&context.egui_ctx, startup) {
            Ok(tray) => (Some(tray), None),
            Err(failure) => (None, Some(failure)),
        };
        let updater = Updater::new(&context.egui_ctx);
        let calculator = Calculator::new();
        let source = Source::new(String::new());
        let results = calculator.evaluate(&source, angle);
        let visible = !hidden || shortcut.is_none();
        Self {
            source,
            calculator,
            results,
            angle,
            renderers: Renderers::new(),
            decorations: Decorations::new(),
            combination,
            popup: Popup::new(if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }),
            shortcut,
            shortcut_failure,
            settings_open: false,
            settings_shortcut: combination.to_string(),
            settings_failure: None,
            autostart,
            autostart_failure,
            tray,
            tray_failure,
            updater,
            update_report: None,
            startup_hidden: !visible,
            focus_requested: visible,
        }
    }

    fn settings(&mut self, ui: &mut eframe::egui::Ui) {
        use eframe::egui::{self, Color32, FontId, Key, RichText};
        ui.label(
            RichText::new("Settings")
                .size(16.0)
                .color(Color32::from_rgb(224, 227, 233)),
        );
        ui.add_space(12.0);
        if self.autostart.is_some() {
            let mut enabled = self
                .autostart
                .as_ref()
                .map(|autostart| autostart.enabled())
                .unwrap_or(false);
            if ui.checkbox(&mut enabled, "Start with system").changed() {
                self.set_autostart(enabled);
            }
        } else {
            let label = ui.label(
                RichText::new("Autostart unavailable")
                    .size(12.0)
                    .color(Color32::from_rgb(255, 123, 114)),
            );
            if let Some(failure) = &self.autostart_failure {
                label.on_hover_text(failure.to_string());
            }
        }
        if let Some(failure) = &self.tray_failure {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Tray unavailable")
                    .size(11.0)
                    .color(Color32::from_rgb(255, 123, 114)),
            )
            .on_hover_text(failure.to_string());
        }
        ui.add_space(12.0);
        ui.label(
            RichText::new("Shortcut")
                .size(12.0)
                .color(Color32::from_rgb(170, 176, 188)),
        );
        let mut apply = false;
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.settings_shortcut)
                    .font(FontId::monospace(14.0))
                    .desired_width(220.0),
            );
            apply = ui.button("Apply").clicked()
                || (response.lost_focus()
                    && ui.input(|input| input.key_pressed(Key::Enter)));
        });
        ui.label(
            RichText::new("Format: Ctrl+Alt+C")
                .size(11.0)
                .color(Color32::from_rgb(110, 116, 128)),
        );
        if apply {
            self.apply_shortcut(ui.ctx());
        }
        if let Some(failure) = &self.settings_failure {
            ui.add_space(8.0);
            ui.label(
                RichText::new(failure.to_string())
                    .size(12.0)
                    .color(Color32::from_rgb(255, 123, 114)),
            );
        }
        ui.add_space(16.0);
        ui.label(
            RichText::new(format!(
                "Version {}",
                crate::update::Version::current()
            ))
            .size(12.0)
            .color(Color32::from_rgb(170, 176, 188)),
        );
        match &self.update_report {
            Some(crate::update::Report::Updated(version)) => {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "Updated to {version} — restart to apply"
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(126, 231, 135)),
                );
            }
            Some(crate::update::Report::Current) => {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Up to date")
                        .size(11.0)
                        .color(Color32::from_rgb(110, 116, 128)),
                );
            }
            Some(crate::update::Report::Failed(failure)) => {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Update check failed")
                        .size(11.0)
                        .color(Color32::from_rgb(255, 123, 114)),
                )
                .on_hover_text(failure.to_string());
            }
            None => {}
        }
        ui.add_space(16.0);
        if ui.button("Close").clicked() {
            self.settings_open = false;
            self.focus_requested = true;
        }
    }

    fn apply_shortcut(&mut self, context: &eframe::egui::Context) {
        use element::Failure;
        use crate::shortcut::{Combination, Shortcut};
        let combination = match self.settings_shortcut.parse::<Combination>() {
            Ok(combination) => combination,
            Err(error) => {
                self.settings_failure = Some(Failure::new(error.to_string()));
                return;
            }
        };
        let result = self
            .shortcut
            .as_mut()
            .map(|shortcut| shortcut.replace(combination));
        match result {
            Some(Ok(())) => {
                self.combination = combination;
                self.settings_shortcut = combination.to_string();
                self.settings_failure = None;
            }
            Some(Err(error)) => {
                self.settings_failure = Some(Failure::new(error.to_string()));
            }
            None => match Shortcut::new(context, combination) {
                Ok(shortcut) => {
                    self.shortcut = Some(shortcut);
                    self.combination = combination;
                    self.shortcut_failure = None;
                    self.settings_shortcut = combination.to_string();
                    self.settings_failure = None;
                }
                Err(error) => {
                    self.settings_failure =
                        Some(Failure::new(error.to_string()));
                }
            },
        }
    }

    fn set_autostart(&mut self, enabled: bool) {
        use element::Failure;
        let result = self.autostart.as_ref().map(|autostart| {
            if enabled {
                autostart.enable()
            } else {
                autostart.disable()
            }
        });
        match result {
            Some(Ok(())) => {
                self.settings_failure = None;
                if let Some(tray) = &self.tray {
                    tray.set_autostart(enabled);
                }
            }
            Some(Err(error)) => {
                self.settings_failure = Some(Failure::new(error.to_string()));
            }
            None => {}
        }
    }

    fn toggle(&mut self, context: &eframe::egui::Context) {
        use crate::popup::Visibility;
        self.popup.toggle(context);
        if self.popup.visibility() == Visibility::Visible {
            self.focus_requested = true;
        } else {
            self.settings_open = false;
        }
        context.request_repaint();
    }
}

impl eframe::App for Application {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(ANGLE_KEY, self.angle.to_string());
        storage.set_string(SHORTCUT_KEY, self.combination.to_string());
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn logic(&mut self, context: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use crate::popup::Visibility;
        use crate::tray::Action;
        if self.startup_hidden {
            self.popup.hide(context);
            self.startup_hidden = false;
        }
        let requested = self
            .shortcut
            .as_ref()
            .map(|shortcut| shortcut.requested().is_some())
            .unwrap_or(false);
        if requested {
            self.toggle(context);
        }
        if self.focus_requested && self.popup.visibility() == Visibility::Visible {
            self.popup.show(context);
        }
        match self.tray.as_ref().and_then(|tray| tray.action()) {
            Some(Action::Toggle) => self.toggle(context),
            Some(Action::Settings) => {
                self.settings_open = true;
                self.settings_shortcut = self.combination.to_string();
                self.settings_failure = None;
                self.focus_requested = false;
                self.popup.show(context);
            }
            Some(Action::Autostart) => {
                let enabled = !self
                    .autostart
                    .as_ref()
                    .map(|autostart| autostart.enabled())
                    .unwrap_or(false);
                self.set_autostart(enabled);
            }
            Some(Action::Quit) => {
                context.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            }
            None => {}
        }
        if let Some(report) = self.updater.report() {
            self.update_report = Some(report);
        }
    }

    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        eframe::egui::Color32::from_rgb(32, 32, 32).to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        use eframe::egui::{
            self, Align2, Color32, FontId, Key, Rect, RichText, Sense, Stroke, Vec2,
        };
        self.decorations.apply(frame);
        let context = ui.ctx().clone();
        if context.input(|input| input.key_pressed(Key::Escape)) {
            if self.settings_open {
                self.settings_open = false;
                self.focus_requested = true;
            } else if self.shortcut.is_some() {
                self.popup.hide(&context);
            }
        }
        if context.input(|input| input.modifiers.command && input.key_pressed(Key::Q)) {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let frame = egui::Frame::new()
            .fill(Color32::from_rgb(32, 32, 32))
            .stroke(Stroke::new(1.0, Color32::from_rgb(64, 64, 64)))
            .corner_radius(12)
            .inner_margin(12);
        egui::CentralPanel::default()
            .frame(frame)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let drag_width = (ui.available_width() - 270.0).max(40.0);
                    let drag = ui.allocate_response(Vec2::new(drag_width, 24.0), Sense::drag());
                    if drag.drag_started() {
                        context.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
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
                            if self.shortcut.is_some() {
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
                            let angle = ui.add(
                                egui::Button::new(
                                    RichText::new(self.angle.to_string())
                                        .size(12.0)
                                        .color(Color32::from_rgb(150, 156, 168)),
                                )
                                .frame(false),
                            );
                            if angle.clicked() {
                                self.angle = self.angle.toggled();
                                self.results = self.calculator.evaluate(&self.source, self.angle);
                            }
                            let settings = ui.add(
                                egui::Button::new(
                                    RichText::new("Settings")
                                        .size(11.0)
                                        .color(Color32::from_rgb(150, 156, 168)),
                                )
                                .frame(false),
                            );
                            if settings.clicked() {
                                self.settings_open = !self.settings_open;
                                if self.settings_open {
                                    self.settings_shortcut = self.combination.to_string();
                                    self.settings_failure = None;
                                    self.focus_requested = false;
                                } else {
                                    self.focus_requested = true;
                                }
                            }
                            match (&self.shortcut, &self.shortcut_failure) {
                                (Some(_), _) => {
                                    ui.label(
                                        RichText::new(self.combination.to_string())
                                            .size(11.0)
                                            .color(Color32::from_rgb(110, 116, 128)),
                                    );
                                }
                                (None, Some(failure)) => {
                                    ui.label(
                                        RichText::new("Shortcut unavailable")
                                            .size(11.0)
                                            .color(Color32::from_rgb(255, 123, 114)),
                                    )
                                    .on_hover_text(failure.as_ref());
                                }
                                (None, None) => {}
                            }
                        },
                    );
                });
                ui.add_space(8.0);
                if self.settings_open {
                    self.settings(ui);
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let font = FontId::monospace(16.0);
                        let viewport = ui.clip_rect();
                        let spacing = 14.0;
                        let mut gutter_width: f32 = 170.0;
                        for value in &self.results {
                            let shown = match self.renderers.render(value) {
                                Some(shown) => shown,
                                None => continue,
                            };
                            let width = ui
                                .painter()
                                .layout_no_wrap(
                                    shown.text.to_string(),
                                    font.clone(),
                                    Color32::WHITE,
                                )
                                .size()
                                .x;
                            gutter_width = gutter_width.max(width + 16.0);
                        }
                        let gutter_width = gutter_width.min(300.0);
                        let gutter_right = viewport.right() - 10.0;
                        let editor_width =
                            (gutter_right - gutter_width - spacing - viewport.left()).max(100.0);
                        let mut layouter = |ui: &egui::Ui,
                                            text: &dyn egui::TextBuffer,
                                            _wrap_width: f32| {
                            let job = egui::text::LayoutJob::simple(
                                text.as_str().to_owned(),
                                FontId::monospace(16.0),
                                Color32::from_rgb(224, 227, 233),
                                f32::INFINITY,
                            );
                            ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
                        };
                        let mut output = egui::TextEdit::multiline(self.source.as_mut())
                            .id(ui.scope_id().with("source"))
                            .font(FontId::monospace(16.0))
                            .text_color(Color32::from_rgb(224, 227, 233))
                            .desired_width(editor_width)
                            .desired_rows(10)
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::ZERO)
                            .hint_text("Type an expression")
                            .layouter(&mut layouter)
                            .show(ui);
                        if output.response.changed() {
                            self.results = self.calculator.evaluate(&self.source, self.angle);
                        }
                        if self.focus_requested {
                            output.response.request_focus();
                            let count = self.source.as_ref().chars().count();
                            output.state.cursor.set_char_range(Some(
                                egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(count),
                                ),
                            ));
                            output.state.store(ui.ctx(), output.response.id);
                            self.focus_requested = false;
                        }
                        let editor_rect = output.response.rect;
                        let gutter_left = editor_rect.right() + spacing;
                        let gutter_rect = Rect::from_min_max(
                            egui::pos2(gutter_left, viewport.top()),
                            egui::pos2(gutter_right, viewport.bottom()),
                        );
                        let painter = ui.painter().with_clip_rect(gutter_rect);
                        for (index, placed) in output.galley.rows.iter().enumerate() {
                            let value = match self.results.get(index) {
                                Some(value) => value,
                                None => continue,
                            };
                            let shown = match self.renderers.render(value) {
                                Some(shown) => shown,
                                None => continue,
                            };
                            let top = output.galley_pos.y + placed.pos.y;
                            let bottom = top + placed.row.size.y;
                            let rect = Rect::from_min_max(
                                egui::pos2(gutter_left, top),
                                egui::pos2(gutter_right, bottom),
                            );
                            if !rect.intersects(viewport) {
                                continue;
                            }
                            if let Some(plain) = &shown.plain {
                                let response = ui
                                    .interact(
                                        rect,
                                        ui.scope_id().with(("result", index)),
                                        Sense::click(),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                                if response.hovered() {
                                    painter.rect_filled(
                                        rect,
                                        4,
                                        Color32::from_rgb(48, 48, 48),
                                    );
                                }
                                if response.clicked() {
                                    context.copy_text(plain.to_string());
                                }
                            }
                            let text = shown.text.to_string();
                            let text_width = painter
                                .layout_no_wrap(text.clone(), font.clone(), shown.color)
                                .size()
                                .x;
                            let available_width = gutter_width - 8.0;
                            let text_font = if text_width > available_width {
                                FontId::monospace(
                                    (font.size * available_width / text_width).max(9.0),
                                )
                            } else {
                                font.clone()
                            };
                            painter.text(
                                egui::pos2(gutter_right - 4.0, top + placed.row.size.y / 2.0),
                                Align2::RIGHT_CENTER,
                                text,
                                text_font,
                                shown.color,
                            );
                        }
                    });
                }
            });
    }
}
