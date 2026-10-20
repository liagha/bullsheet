const SHORTCUT_KEY: &str = "shortcut";

pub(crate) struct Application {
    source: evaluator::Source,
    calculator: evaluator::Calculator,
    readings: evaluator::Readings,
    renderers: crate::render::Renderers,
    decorations: crate::decorations::Decorations,
    settings: crate::settings::Settings,
    popup: crate::popup::Popup,
    tray: Option<crate::tray::Tray>,
    updater: crate::update::Updater,
    startup_hidden: bool,
    focus_requested: bool,
    autostart_shown: bool,
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
        use crate::render::Renderers;
        use crate::settings::Settings;
        use crate::shortcut::{Combination, Shortcut};
        use crate::tray::Tray;
        use crate::update::Updater;
        use eframe::egui::{FontData, FontDefinitions, FontFamily};
        use evaluator::{Calculator, Source};
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
        let mut calculator = Calculator::new();
        settings.install(&mut calculator, &context.egui_ctx);
        let source = Source::new(String::new());
        let readings = calculator.evaluate(&source);
        let visible = !hidden || !settings.registered();
        Self {
            source,
            calculator,
            readings,
            renderers: Renderers::new(),
            decorations: Decorations::new(),
            settings,
            popup: Popup::new(if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            }),
            tray,
            updater,
            startup_hidden: !visible,
            focus_requested: visible,
            autostart_shown: startup,
        }
    }

    fn toggle(&mut self, context: &eframe::egui::Context) {
        use crate::popup::Visibility;
        self.popup.toggle(context);
        if self.popup.visibility() == Visibility::Visible {
            self.focus_requested = true;
        }
        context.request_repaint();
    }
}

impl eframe::App for Application {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Left behind by the deleted degree toggle, and never read again.
        storage.remove_string("angle");
        storage.set_string(SHORTCUT_KEY, self.settings.combination().to_string());
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
        if self.settings.requested() {
            self.toggle(context);
        }
        if self.focus_requested && self.popup.visibility() == Visibility::Visible {
            self.popup.show(context);
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
        use eframe::egui::{
            self, Align2, Color32, FontId, Key, Rect, RichText, Sense, Stroke, Vec2,
        };
        self.decorations.apply(frame);
        let context = ui.ctx().clone();
        if context.input(|input| input.key_pressed(Key::Escape))
            && self.settings.registered()
        {
            self.popup.hide(&context);
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
                        },
                    );
                });
                ui.add_space(8.0);
                let calculator = &self.calculator;
                let source = &mut self.source;
                let readings = &mut self.readings;
                let renderers = &self.renderers;
                let focus_requested = &mut self.focus_requested;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let spacing = 14.0;
                        let surface = crate::surface::Surface::new(readings, renderers);
                        let font = surface.font().clone();
                        let viewport = ui.clip_rect();
                        let mut gutter_width: f32 = 170.0;
                        for reading in readings.iter() {
                            let shown = match renderers.render(reading.value()) {
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
                        let completion = surface.editor(ui, editor_width, source);
                        let accepted = completion.accepted.is_some();
                        let mut output = completion.text_edit;
                        if accepted || output.response.changed() {
                            *readings = calculator.evaluate(source);
                        }
                        let surface = crate::surface::Surface::new(readings, renderers);
                        if *focus_requested {
                            output.response.request_focus();
                            let count = source.as_ref().chars().count();
                            output.state.cursor.set_char_range(Some(
                                egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(count),
                                ),
                            ));
                            output.state.store(ui.ctx(), output.response.id);
                            *focus_requested = false;
                        }
                        let editor_rect = output.response.rect;
                        let gutter_left = editor_rect.right() + spacing;
                        let gutter_rect = Rect::from_min_max(
                            egui::pos2(gutter_left, viewport.top()),
                            egui::pos2(gutter_right, viewport.bottom()),
                        );
                        let painter = ui.painter().with_clip_rect(gutter_rect);
                        for (index, placed) in output.galley.rows.iter().enumerate() {
                            let reading = match readings.get(index) {
                                Some(reading) => reading,
                                None => continue,
                            };
                            let shown = match renderers.render(reading.value()) {
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
                                    let line = source
                                        .as_ref()
                                        .split('\n')
                                        .nth(index)
                                        .unwrap_or("")
                                        .trim();
                                    let copied = if context
                                        .input(|input| input.modifiers.command)
                                    {
                                        if line.contains('=') {
                                            line.to_owned()
                                        } else {
                                            format!("{line} = {plain}")
                                        }
                                    } else {
                                        plain.to_string()
                                    };
                                    context.copy_text(copied);
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
                        let hovered = context.pointer_hover_pos().and_then(|position| {
                            if !output.response.rect.contains(position) {
                                return None;
                            }
                            let cursor =
                                output.galley.cursor_from_pos(position - output.galley_pos);
                            surface.locate(source.as_ref(), cursor.index.0)
                        });
                        if let Some((part, text)) = hovered {
                            let tip = match renderers.render(part.value()) {
                                Some(shown) => {
                                    format!("{} {} — ctrl-click copies", shown.kind, shown.text)
                                }
                                None => text.clone(),
                            };
                            if context.input(|input| input.modifiers.command)
                                && output.response.clicked()
                            {
                                context.copy_text(text);
                            }
                            (*output.response)
                                .clone()
                                .on_hover_text_at_pointer(tip);
                        }
                    });
            });

    }
}
