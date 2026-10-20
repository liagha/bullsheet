pub struct Fragment {
    galley: std::sync::Arc<eframe::egui::Galley>,
    x: f32,
    y: f32,
}

pub struct Line {
    fragments: Vec<Fragment>,
    size: eframe::egui::Vec2,
}

impl Line {
    pub fn new(
        ui: &eframe::egui::Ui,
        text: &str,
        color: eframe::egui::Color32,
        font: &eframe::egui::FontId,
    ) -> Self {
        let mut shaped: Vec<(std::sync::Arc<eframe::egui::Galley>, f32)> = Vec::new();
        let info = unicode_bidi::BidiInfo::new(text, None);
        for paragraph in &info.paragraphs {
            let (_, runs) = info.visual_runs(paragraph, paragraph.range.clone());
            for run in runs {
                if run.is_empty() {
                    continue;
                }
                let mut job = eframe::egui::text::LayoutJob::default();
                job.wrap.max_width = f32::INFINITY;
                job.append(
                    &text[run],
                    0.0,
                    eframe::egui::TextFormat {
                        font_id: font.clone(),
                        color,
                        ..Default::default()
                    },
                );
                let galley = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job));
                let ascent = galley
                    .rows
                    .first()
                    .and_then(|row| row.glyphs.first())
                    .map_or(0.0, |glyph| glyph.font_ascent);
                shaped.push((galley, ascent));
            }
        }
        let baseline = shaped.iter().map(|(_, ascent)| *ascent).fold(0.0, f32::max);
        let mut fragments = Vec::with_capacity(shaped.len());
        let mut width = 0.0;
        let mut height: f32 = 0.0;
        for (galley, ascent) in shaped {
            let y = baseline - ascent;
            height = height.max(y + galley.size().y);
            fragments.push(Fragment {
                galley: galley.clone(),
                x: width,
                y,
            });
            width += galley.size().x;
        }
        Self {
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
}
