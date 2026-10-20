pub struct Calculator {
    source: evaluator::Source,
    engine: evaluator::Calculator,
    readings: evaluator::Readings,
    renderers: crate::render::Renderers,
    focus_requested: bool,
}

impl Calculator {
    pub fn new(
        settings: &crate::settings::Settings,
        context: &eframe::egui::Context,
        focused: bool,
    ) -> Self {
        let mut engine = evaluator::Calculator::new();
        settings.install(&mut engine, context);
        let source = evaluator::Source::new(String::new());
        let readings = engine.evaluate(&source);
        Self {
            source,
            engine,
            readings,
            renderers: crate::render::Renderers::new(),
            focus_requested: focused,
        }
    }
}

impl crate::station::Station for Calculator {
    fn show(&mut self, ui: &mut eframe::egui::Ui) {
        use eframe::egui::{self, Color32, FontId, Rect, Sense};
        let context = ui.ctx().clone();
        let engine = &self.engine;
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
                        .layout_no_wrap(shown.text.to_string(), font.clone(), Color32::WHITE)
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
                    *readings = engine.evaluate(source);
                }
                let surface = crate::surface::Surface::new(readings, renderers);
                if *focus_requested {
                    output.response.request_focus();
                    let count = source.as_ref().chars().count();
                    output
                        .state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(count),
                        )));
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
                            .interact(rect, ui.scope_id().with(("result", index)), Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if response.hovered() {
                            painter.rect_filled(rect, 4, Color32::from_rgb(48, 48, 48));
                        }
                        if response.clicked() {
                            let line = source.as_ref().split('\n').nth(index).unwrap_or("").trim();
                            let copied = if context.input(|input| input.modifiers.command) {
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
                        FontId::monospace((font.size * available_width / text_width).max(9.0))
                    } else {
                        font.clone()
                    };
                    let line = crate::typography::Line::new(ui, &text, shown.color, &text_font);
                    line.paint(
                        &painter,
                        egui::pos2(
                            gutter_right - 4.0 - line.size().x,
                            top + (placed.row.size.y - line.size().y) / 2.0,
                        ),
                    );
                }
                let hovered = context.pointer_hover_pos().and_then(|position| {
                    if !output.response.rect.contains(position) {
                        return None;
                    }
                    let cursor = output.galley.cursor_from_pos(position - output.galley_pos);
                    surface.locate(source.as_ref(), cursor.index.0)
                });
                if let Some((part, text)) = hovered {
                    let tip = match renderers.render(part.value()) {
                        Some(shown) => {
                            format!("{} {} — ctrl-click copies", shown.kind, shown.text)
                        }
                        None => text.clone(),
                    };
                    if context.input(|input| input.modifiers.command) && output.response.clicked() {
                        context.copy_text(text);
                    }
                    (*output.response).clone().on_hover_text_at_pointer(tip);
                }
            });
    }

    fn focus(&mut self) {
        self.focus_requested = true;
    }
}
