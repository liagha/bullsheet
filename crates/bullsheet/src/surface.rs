const DEPTHS: [eframe::egui::Color32; 3] = [
    eframe::egui::Color32::from_rgb(86, 182, 194),
    eframe::egui::Color32::from_rgb(206, 145, 120),
    eframe::egui::Color32::from_rgb(197, 134, 192),
];
const UNMATCHED: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(255, 123, 114);
const BASE: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(224, 227, 233);

pub(crate) struct Run {
    span: std::ops::Range<usize>,
    color: eframe::egui::Color32,
}

impl Run {
    pub(crate) fn span(&self) -> std::ops::Range<usize> {
        self.span.clone()
    }

    pub(crate) fn color(&self) -> eframe::egui::Color32 {
        self.color
    }
}

pub(crate) struct Surface<'a> {
    readings: &'a evaluator::Readings,
    renderers: &'a crate::render::Renderers,
    font: eframe::egui::FontId,
}

impl<'a> Surface<'a> {
    pub(crate) fn new(
        readings: &'a evaluator::Readings,
        renderers: &'a crate::render::Renderers,
    ) -> Self {
        Self {
            readings,
            renderers,
            font: eframe::egui::FontId::monospace(16.0),
        }
    }

    pub(crate) fn font(&self) -> &eframe::egui::FontId {
        &self.font
    }

    pub(crate) fn base(&self) -> eframe::egui::Color32 {
        BASE
    }

    pub(crate) fn runs(&self, index: usize, line: &str) -> Vec<Run> {
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

    pub(crate) fn galley(
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

    pub(crate) fn locate(&self, text: &str, character: usize) -> Option<(&'a evaluator::Part, String)> {
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

    fn format(&self, color: eframe::egui::Color32) -> eframe::egui::TextFormat {
        eframe::egui::TextFormat {
            font_id: self.font.clone(),
            color,
            ..Default::default()
        }
    }
}
