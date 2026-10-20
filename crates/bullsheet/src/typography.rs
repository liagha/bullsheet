pub struct Fragment {
    range: std::ops::Range<usize>,
    right_to_left: bool,
    galley: std::sync::Arc<eframe::egui::Galley>,
    x: f32,
    y: f32,
}

impl Fragment {
    fn width(&self) -> f32 {
        self.galley.size().x
    }
}

pub struct Line {
    text: String,
    boundaries: Vec<usize>,
    base: eframe::egui::Color32,
    font: eframe::egui::FontId,
    fragments: Vec<Fragment>,
    size: eframe::egui::Vec2,
}

impl Line {
    pub fn new(
        ui: &eframe::egui::Ui,
        text: &str,
        spans: &[(std::ops::Range<usize>, eframe::egui::Color32)],
        base: eframe::egui::Color32,
        font: &eframe::egui::FontId,
    ) -> Self {
        let boundaries: Vec<usize> = text
            .char_indices()
            .map(|(byte, _)| byte)
            .chain(std::iter::once(text.len()))
            .collect();
        let characters = boundaries.len() - 1;
        let spans: Vec<(std::ops::Range<usize>, eframe::egui::Color32)> = spans
            .iter()
            .map(|(span, color)| {
                (
                    boundaries[span.start.min(characters)]..boundaries[span.end.min(characters)],
                    *color,
                )
            })
            .collect();
        let mut ordered: Vec<(std::ops::Range<usize>, bool)> = Vec::new();
        let info = unicode_bidi::BidiInfo::new(text, None);
        for paragraph in &info.paragraphs {
            let (_, runs) = info.visual_runs(paragraph, paragraph.range.clone());
            for run in runs {
                if run.is_empty() {
                    continue;
                }
                let level = info
                    .levels
                    .get(run.start)
                    .copied()
                    .unwrap_or(unicode_bidi::Level::ltr());
                ordered.push((run, level.is_rtl()));
            }
        }
        let mut shaped: Vec<(std::sync::Arc<eframe::egui::Galley>, f32)> = Vec::new();
        for (run, _) in &ordered {
            let mut job = eframe::egui::text::LayoutJob::default();
            job.wrap.max_width = f32::INFINITY;
            let mut position = run.start;
            for (span, color) in &spans {
                let start = span.start.max(run.start);
                let end = span.end.min(run.end);
                if start < position || end <= start {
                    continue;
                }
                if start > position {
                    job.append(&text[position..start], 0.0, self::format(base, font));
                }
                job.append(&text[start..end], 0.0, self::format(*color, font));
                position = end;
            }
            if position < run.end {
                job.append(&text[position..run.end], 0.0, self::format(base, font));
            }
            let galley = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job));
            let ascent = galley
                .rows
                .first()
                .and_then(|row| row.glyphs.first())
                .map_or(0.0, |glyph| glyph.font_ascent);
            shaped.push((galley, ascent));
        }
        let baseline = shaped.iter().map(|(_, ascent)| *ascent).fold(0.0, f32::max);
        let mut fragments = Vec::with_capacity(shaped.len());
        let mut width = 0.0;
        let mut height: f32 = 0.0;
        for ((galley, ascent), (range, right_to_left)) in shaped.into_iter().zip(ordered) {
            let y = baseline - ascent;
            height = height.max(y + galley.size().y);
            fragments.push(Fragment {
                range,
                right_to_left,
                x: width,
                y,
                galley: galley.clone(),
            });
            width += galley.size().x;
        }
        Self {
            text: text.to_owned(),
            boundaries,
            base,
            font: font.clone(),
            fragments,
            size: eframe::egui::vec2(width, height),
        }
    }

    pub fn size(&self) -> eframe::egui::Vec2 {
        self.size
    }

    pub fn paint(&self, painter: &eframe::egui::Painter, position: eframe::egui::Pos2) {
        for fragment in &self.fragments {
            painter.galley(
                position + eframe::egui::vec2(fragment.x, fragment.y),
                fragment.galley.clone(),
                eframe::egui::Color32::PLACEHOLDER,
            );
        }
    }

    pub fn characters(&self) -> usize {
        self.boundaries.len() - 1
    }

    pub fn x_of(&self, ui: &eframe::egui::Ui, character: usize) -> f32 {
        let characters = self.characters();
        if self.fragments.is_empty() || characters == 0 {
            return 0.0;
        }
        let probe = character.min(characters - 1);
        let Some(fragment) = self.fragment_at(probe) else {
            return self.size.x;
        };
        let start = self.character_of(fragment.range.start);
        let character = character.min(characters);
        if fragment.right_to_left {
            // Shape the suffix: an RTL run still joins with the character to its left.
            fragment.x + self.measure(ui, character, self.character_of(fragment.range.end))
        } else {
            fragment.x + self.measure(ui, start, character)
        }
    }

    pub fn character_at(&self, ui: &eframe::egui::Ui, x: f32) -> usize {
        if self.fragments.is_empty() {
            return 0;
        }
        let mut target = self.fragments.last().expect("fragments are not empty");
        for fragment in &self.fragments {
            if x < fragment.x + fragment.width() {
                target = fragment;
                break;
            }
        }
        let start = self.character_of(target.range.start);
        let length = self.character_of(target.range.end) - start;
        let offset = |count: usize| {
            if target.right_to_left {
                target.x + self.measure(ui, start + count, start + length)
            } else {
                target.x + self.measure(ui, start, start + count)
            }
        };
        let mut low = 0;
        let mut high = length;
        while low < high {
            let middle = (low + high) / 2;
            let caret = offset(middle);
            let before = if target.right_to_left {
                caret > x
            } else {
                caret < x
            };
            if before {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        let mut best = low;
        if low > 0 && (offset(low - 1) - x).abs() < (offset(low) - x).abs() {
            best = low - 1;
        }
        start + best
    }

    pub fn right_to_left(&self, character: usize) -> bool {
        let characters = self.characters();
        if characters == 0 {
            return false;
        }
        let probe = character.min(characters - 1);
        self.fragment_at(probe)
            .is_some_and(|fragment| fragment.right_to_left)
    }

    fn fragment_at(&self, character: usize) -> Option<&Fragment> {
        let byte = self.boundaries[character.min(self.characters())];
        self.fragments
            .iter()
            .find(|fragment| fragment.range.start <= byte && byte < fragment.range.end)
    }

    fn character_of(&self, byte: usize) -> usize {
        match self.boundaries.binary_search(&byte) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        }
        .min(self.characters())
    }

    fn measure(&self, ui: &eframe::egui::Ui, start: usize, end: usize) -> f32 {
        let start = self.boundaries[start.min(self.characters())];
        let end = self.boundaries[end.min(self.characters())];
        if start >= end {
            return 0.0;
        }
        let mut job = eframe::egui::text::LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        job.append(
            &self.text[start..end],
            0.0,
            self::format(self.base, &self.font),
        );
        ui.ctx().fonts_mut(|fonts| fonts.layout_job(job)).size().x
    }
}

fn format(color: eframe::egui::Color32, font: &eframe::egui::FontId) -> eframe::egui::TextFormat {
    eframe::egui::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    }
}
