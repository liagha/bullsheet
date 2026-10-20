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

    fn suggestions(&self, word: &str) -> Vec<crate::editor::Suggestion> {
        if word.is_empty() {
            return Vec::new();
        }
        self.readings
            .names()
            .iter()
            .filter(|name| {
                let text = name.text();
                text.as_ref().starts_with(word) && text.as_ref() != word
            })
            .take(8)
            .map(|name| {
                let text = name.text().to_string();
                let description = self
                    .renderers
                    .render(name.value())
                    .map_or("", |shown| shown.kind)
                    .to_owned();
                crate::editor::Suggestion {
                    insert: text,
                    description,
                }
            })
            .collect()
    }
}

impl crate::editor::Sheet for Surface<'_> {
    fn runs(
        &self,
        index: usize,
        line: &str,
    ) -> Vec<(std::ops::Range<usize>, eframe::egui::Color32)> {
        self.runs(index, line)
            .into_iter()
            .map(|run| (run.span(), run.color()))
            .collect()
    }

    fn suggestions(&self, word: &str) -> Vec<crate::editor::Suggestion> {
        self.suggestions(word)
    }
}
