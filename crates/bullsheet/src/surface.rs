const DEPTHS: [eframe::egui::Color32; 3] = [
    eframe::egui::Color32::from_rgb(86, 182, 194),
    eframe::egui::Color32::from_rgb(206, 145, 120),
    eframe::egui::Color32::from_rgb(197, 134, 192),
];
const UNMATCHED: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(255, 123, 114);
const BASE: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(224, 227, 233);

pub struct Run {
    span: std::ops::Range<usize>,
    color: eframe::egui::Color32,
}

impl Run {
    pub fn span(&self) -> std::ops::Range<usize> {
        self.span.clone()
    }

    pub fn color(&self) -> eframe::egui::Color32 {
        self.color
    }
}

pub struct Surface<'a> {
    readings: &'a evaluator::Readings,
    renderers: &'a crate::render::Renderers,
    font: eframe::egui::FontId,
}

impl<'a> Surface<'a> {
    pub fn new(readings: &'a evaluator::Readings, renderers: &'a crate::render::Renderers) -> Self {
        Self {
            readings,
            renderers,
            font: eframe::egui::FontId::monospace(16.0),
        }
    }

    pub fn font(&self) -> &eframe::egui::FontId {
        &self.font
    }

    pub fn runs(&self, index: usize, line: &str) -> Vec<Run> {
        let characters = line.chars().count();
        let mut runs: Vec<Run> = Vec::new();
        if let Some(reading) = self.readings.get(index) {
            for part in reading.parts() {
                let span = part.span();
                let start = span.start.min(characters);
                let end = span.end.min(characters).max(start);
                if start >= end {
                    continue;
                }
                let color = self
                    .renderers
                    .render(part.value())
                    .map_or(BASE, |shown| shown.color);
                runs.push(Run {
                    span: start..end,
                    color,
                });
            }
        }
        let mut open: Vec<usize> = Vec::new();
        for (position, character) in line.chars().enumerate() {
            match character {
                '(' => open.push(position),
                ')' => match open.pop() {
                    Some(partner) => {
                        let color = DEPTHS[open.len() % DEPTHS.len()];
                        runs.push(Run {
                            span: partner..partner + 1,
                            color,
                        });
                        runs.push(Run {
                            span: position..position + 1,
                            color,
                        });
                    }
                    None => runs.push(Run {
                        span: position..position + 1,
                        color: UNMATCHED,
                    }),
                },
                _ => {}
            }
        }
        for partner in open {
            runs.push(Run {
                span: partner..partner + 1,
                color: UNMATCHED,
            });
        }
        runs.sort_by_key(|run| run.span.start);
        runs
    }

    pub fn galley(
        &self,
        ui: &eframe::egui::Ui,
        text: &str,
    ) -> std::sync::Arc<eframe::egui::Galley> {
        let mut job = eframe::egui::text::LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        let lines: Vec<&str> = text.split('\n').collect();
        for (index, line) in lines.iter().enumerate() {
            let characters = line.chars().count();
            let boundaries: Vec<usize> = line
                .char_indices()
                .map(|(byte, _)| byte)
                .chain(std::iter::once(line.len()))
                .collect();
            let mut position = 0;
            for run in self.runs(index, line) {
                let span = run.span();
                if span.start < position || span.end <= span.start {
                    continue;
                }
                if span.start > position {
                    job.append(
                        &line[boundaries[position]..boundaries[span.start]],
                        0.0,
                        self.format(BASE),
                    );
                }
                job.append(
                    &line[boundaries[span.start]..boundaries[span.end]],
                    0.0,
                    self.format(run.color()),
                );
                position = span.end;
            }
            if position < characters {
                job.append(&line[boundaries[position]..], 0.0, self.format(BASE));
            }
            if index + 1 < lines.len() {
                job.append("\n", 0.0, self.format(BASE));
            }
        }
        ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
    }

    pub fn locate(&self, text: &str, character: usize) -> Option<(&'a evaluator::Part, String)> {
        let mut character = character;
        for (index, line) in text.split('\n').enumerate() {
            let characters = line.chars().count();
            if character <= characters {
                let part = self
                    .readings
                    .get(index)?
                    .parts()
                    .iter()
                    .find(|part| part.span().contains(&character))?;
                let span = part.span();
                let text = line
                    .chars()
                    .skip(span.start)
                    .take(span.end - span.start)
                    .collect();
                return Some((part, text));
            }
            character -= characters + 1;
        }
        None
    }

    pub fn editor(
        &self,
        ui: &mut eframe::egui::Ui,
        width: f32,
        source: &mut evaluator::Source,
    ) -> eframe::egui::CompletionOutput {
        ui.input_mut(|input| {
            for event in &mut input.events {
                let text = match event {
                    eframe::egui::Event::Text(text) | eframe::egui::Event::Paste(text) => text,
                    eframe::egui::Event::Ime(eframe::egui::ImeEvent::Commit(text)) => text,
                    eframe::egui::Event::Ime(eframe::egui::ImeEvent::Preedit { text, .. }) => text,
                    _ => continue,
                };
                *text =
                    text.chars()
                        .map(|character| match character {
                            '۰'..='۹' => char::from_digit(character as u32 - '۰' as u32, 10)
                                .unwrap_or(character),
                            '٠'..='٩' => char::from_digit(character as u32 - '٠' as u32, 10)
                                .unwrap_or(character),
                            '٬' | '،' => ',',
                            '٫' => '.',
                            _ => character,
                        })
                        .collect();
            }
        });
        // egui has no bidi and shapes Arabic-script runs right to left, so Persian digits would
        // draw mirrored; the buffer keeps the Latin form.
        eframe::egui::CompletionPopup::new(ui.scope_id().with("source"))
            .word_boundary(|character| !(character.is_ascii_alphanumeric() || character == '_'))
            .align(eframe::egui::RectAlign::BOTTOM_START)
            .max_height(96.0)
            .show(
                ui,
                eframe::egui::TextEdit::multiline(source.as_mut())
                    .font(self.font.clone())
                    .text_color(BASE)
                    .desired_width(width)
                    .desired_rows(10)
                    .frame(eframe::egui::Frame::NONE)
                    .margin(eframe::egui::Margin::ZERO)
                    .hint_text("Type an expression")
                    .layouter(&mut |ui: &eframe::egui::Ui,
                                    text: &dyn eframe::egui::TextBuffer,
                                    _wrap_width: f32| {
                        self.galley(ui, text.as_str())
                    }),
                |query| self.suggestions(query.word),
            )
    }

    fn suggestions(&self, word: &str) -> Vec<eframe::egui::Suggestion> {
        if word.is_empty() {
            return Vec::new();
        }
        self.readings
            .names()
            .iter()
            .filter(|name| {
                let text = name.text().as_ref();
                text.starts_with(word) && text != word
            })
            .take(8)
            .map(|name| {
                let text = name.text().as_ref();
                let kind = self
                    .renderers
                    .render(name.value())
                    .map_or("", |shown| shown.kind);
                eframe::egui::Suggestion::new(text).description(kind)
            })
            .collect()
    }

    fn format(&self, color: eframe::egui::Color32) -> eframe::egui::TextFormat {
        eframe::egui::TextFormat {
            font_id: self.font.clone(),
            color,
            ..Default::default()
        }
    }
}
