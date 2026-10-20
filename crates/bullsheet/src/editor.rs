pub struct Suggestion {
    pub insert: String,
    pub description: String,
}

pub trait Sheet {
    fn runs(&self, line: usize, text: &str)
    -> Vec<(std::ops::Range<usize>, eframe::egui::Color32)>;

    fn suggestions(&self, word: &str) -> Vec<Suggestion>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Caret {
    line: usize,
    character: usize,
}

pub struct Placed {
    pub top: f32,
    pub height: f32,
    pub line: crate::typography::Line,
}

pub struct Output {
    pub response: eframe::egui::Response,
    pub changed: bool,
    pub hovered: Option<usize>,
    pub lines: Vec<Placed>,
}

pub struct Editor {
    caret: Caret,
    anchor: Option<Caret>,
    focus: bool,
    select_all: bool,
    blink: f64,
    selected: usize,
    query: Option<String>,
    dismissed: bool,
    scroll: bool,
    popup: Option<eframe::egui::Rect>,
}

const BASE: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(224, 227, 233);
const SELECTION: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(72, 72, 72);

impl Editor {
    pub fn new() -> Self {
        Self {
            caret: Caret {
                line: 0,
                character: 0,
            },
            anchor: None,
            focus: false,
            select_all: false,
            blink: 0.0,
            selected: 0,
            query: None,
            dismissed: false,
            scroll: false,
            popup: None,
        }
    }

    pub fn focus(&mut self) {
        self.focus = true;
        self.select_all = true;
    }

    pub fn show(
        &mut self,
        ui: &mut eframe::egui::Ui,
        width: f32,
        text: &mut String,
        font: &eframe::egui::FontId,
        sheet: &dyn Sheet,
    ) -> Output {
        use eframe::egui::{Key, Sense, vec2};
        let mut lines = layout(ui, text, font, sheet);
        let row = ui.ctx().fonts_mut(|fonts| fonts.row_height(font));
        let height = lines
            .iter()
            .map(|placed| placed.top + placed.height)
            .fold(0.0, f32::max)
            .max(row * 10.0);
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
        if self.focus {
            self.focus = false;
            ui.memory_mut(|memory| memory.request_focus(response.id));
            if self.select_all {
                self.select_all = false;
                let last = text.split('\n').count().saturating_sub(1);
                let characters = text.split('\n').last().unwrap_or("").chars().count();
                self.anchor = Some(Caret {
                    line: 0,
                    character: 0,
                });
                self.caret = Caret {
                    line: last,
                    character: characters,
                };
            }
        }
        let focused = ui.memory(|memory| memory.has_focus(response.id));
        self.clamp(text, &lines);
        let query = self.word(text);
        let suggestions = query
            .as_ref()
            .map_or_else(Vec::new, |(_, word)| sheet.suggestions(word));
        if query.as_ref().map(|(_, word)| word) != self.query.as_ref() {
            self.query = query.as_ref().map(|(_, word)| word.clone());
            self.selected = 0;
            self.dismissed = false;
        }
        let open = focused && !self.dismissed && !suggestions.is_empty();
        self.selected = self.selected.min(suggestions.len().saturating_sub(1));
        let mut changed = false;
        if focused {
            let events = ui.input(|input| input.events.clone());
            for event in events {
                match event {
                    eframe::egui::Event::Text(typed) => {
                        self.insert(text, &normalize(&typed));
                        changed = true;
                    }
                    eframe::egui::Event::Paste(typed) => {
                        self.insert(text, &normalize(&typed));
                        changed = true;
                    }
                    eframe::egui::Event::Ime(eframe::egui::ImeEvent::Commit(typed)) => {
                        self.insert(text, &normalize(&typed));
                        changed = true;
                    }
                    eframe::egui::Event::Copy => {
                        if let Some(selection) = self.selection(text) {
                            ui.ctx().copy_text(selection);
                        }
                    }
                    eframe::egui::Event::Cut => {
                        if let Some(selection) = self.selection(text) {
                            ui.ctx().copy_text(selection);
                            self.delete_selection(text);
                            changed = true;
                        }
                    }
                    eframe::egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => match key {
                        Key::Enter if open => {
                            self.accept(text, query.as_ref(), &suggestions);
                            changed = true;
                        }
                        Key::Tab if open => {
                            self.accept(text, query.as_ref(), &suggestions);
                            changed = true;
                        }
                        Key::ArrowDown if open => {
                            self.selected = (self.selected + 1) % suggestions.len();
                        }
                        Key::ArrowUp if open => {
                            self.selected = self
                                .selected
                                .checked_sub(1)
                                .unwrap_or(suggestions.len() - 1);
                        }
                        Key::Escape if open => {
                            self.dismissed = true;
                        }
                        Key::Enter => {
                            self.insert(text, "\n");
                            changed = true;
                        }
                        Key::Backspace => {
                            self.backspace(text);
                            changed = true;
                        }
                        Key::Delete => {
                            self.delete(text);
                            changed = true;
                        }
                        Key::ArrowLeft => self.horizontal(ui, text, &lines, false),
                        Key::ArrowRight => self.horizontal(ui, text, &lines, true),
                        Key::ArrowUp => self.vertical(ui, &lines, false),
                        Key::ArrowDown => self.vertical(ui, &lines, true),
                        Key::Home => {
                            self.caret.character = 0;
                            self.anchor = None;
                            self.blink(ui);
                        }
                        Key::End => {
                            self.caret.character = self.length(text);
                            self.anchor = None;
                            self.blink(ui);
                        }
                        Key::A if modifiers.command => {
                            let last = text.split('\n').count().saturating_sub(1);
                            let characters = text.split('\n').last().unwrap_or("").chars().count();
                            self.anchor = Some(Caret {
                                line: 0,
                                character: 0,
                            });
                            self.caret = Caret {
                                line: last,
                                character: characters,
                            };
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        if changed {
            lines = layout(ui, text, font, sheet);
        }
        let pointer = ui.input(|input| input.pointer.interact_pos());
        let over_popup = self
            .popup
            .is_some_and(|popup| pointer.is_some_and(|position| popup.contains(position)));
        if !over_popup && (response.clicked() || response.drag_started()) {
            if let Some(position) = response.interact_pointer_pos() {
                ui.memory_mut(|memory| memory.request_focus(response.id));
                self.caret = self.place_at(ui, rect, &lines, position);
                self.anchor = None;
                self.blink(ui);
            }
        }
        if !over_popup && response.dragged() {
            if let Some(position) = response.interact_pointer_pos() {
                if self.anchor.is_none() {
                    self.anchor = Some(self.caret);
                }
                self.caret = self.place_at(ui, rect, &lines, position);
            }
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(eframe::egui::CursorIcon::Text);
        }
        let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(
            eframe::egui::Rect::from_min_max(
                rect.min,
                eframe::egui::pos2(rect.right() + 2.0, rect.bottom()),
            ),
        ));
        let origin = rect.min;
        let selection = self.ordered();
        if let Some((from, to)) = selection {
            for (index, placed) in lines.iter().enumerate() {
                let left = if index == from.line {
                    from.character
                } else {
                    0
                };
                let right = if index == to.line {
                    to.character
                } else {
                    placed.line.characters()
                };
                if left >= right {
                    continue;
                }
                let start = placed.line.x_of(ui, left);
                let end = placed.line.x_of(ui, right);
                let band = eframe::egui::Rect::from_min_max(
                    eframe::egui::pos2(origin.x + start.min(end), origin.y + placed.top),
                    eframe::egui::pos2(
                        origin.x + start.max(end),
                        origin.y + placed.top + placed.height,
                    ),
                );
                painter.rect_filled(band, 0.0, SELECTION);
            }
        }
        for placed in &lines {
            placed
                .line
                .paint(&painter, origin + eframe::egui::vec2(0.0, placed.top));
        }
        let now = ui.input(|input| input.time);
        let visible = (now - self.blink) % 1.0 < 0.5;
        let placed = &lines[self.caret.line.min(lines.len() - 1)];
        let caret = eframe::egui::Rect::from_min_size(
            eframe::egui::pos2(
                origin.x + placed.line.x_of(ui, self.caret.character),
                origin.y + placed.top,
            ),
            vec2(1.0, placed.height),
        );
        if focused {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
            if visible {
                painter.rect_filled(caret, 0.0, BASE);
            }
            if self.scroll {
                self.scroll = false;
                ui.scroll_to_rect(caret.expand(4.0), None);
            }
        }
        self.popup = None;
        if open && !over_popup {
            let position = eframe::egui::pos2(
                caret.left().min(origin.x + width - 220.0).max(origin.x),
                caret.bottom() + 4.0,
            );
            let area = eframe::egui::Area::new(response.id.with("suggestions"))
                .order(eframe::egui::Order::Foreground)
                .fixed_pos(position)
                .show(ui.ctx(), |ui| {
                    eframe::egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_width(216.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for (index, suggestion) in suggestions.iter().enumerate() {
                            let label = eframe::egui::RichText::new(&suggestion.insert)
                                .family(eframe::egui::FontFamily::Monospace);
                            let row = ui.selectable_label(index == self.selected, label);
                            if !suggestion.description.is_empty() {
                                ui.painter().text(
                                    eframe::egui::pos2(row.rect.right() - 6.0, row.rect.center().y),
                                    eframe::egui::Align2::RIGHT_CENTER,
                                    &suggestion.description,
                                    eframe::egui::FontId::monospace(11.0),
                                    eframe::egui::Color32::from_rgb(120, 120, 120),
                                );
                            }
                            if row.clicked() {
                                self.accept(text, query.as_ref(), &suggestions);
                                changed = true;
                            }
                        }
                    });
                });
            if !self.dismissed {
                self.popup = Some(area.response.rect);
            }
        }
        let hovered = if response.hovered() {
            pointer.map(|position| {
                let caret = self.place_at(ui, rect, &lines, position);
                self.global(text, caret)
            })
        } else {
            None
        };
        Output {
            response,
            changed,
            hovered,
            lines,
        }
    }

    fn accept(&mut self, text: &mut String, query: Option<&(usize, String)>, hints: &[Suggestion]) {
        let (start, _) = match query {
            Some(query) => query,
            None => return,
        };
        let Some(hint) = hints.get(self.selected) else {
            return;
        };
        if self.ordered().is_some() {
            self.insert(text, &hint.insert);
        } else {
            let ending = self.global(text, self.caret);
            let from = ending - self.caret.character + start;
            text.replace_range(byte_of(text, from)..byte_of(text, ending), &hint.insert);
            self.place(text, from + hint.insert.chars().count());
            self.anchor = None;
        }
        self.dismissed = true;
    }

    fn word(&self, text: &str) -> Option<(usize, String)> {
        let line = text.split('\n').nth(self.caret.line)?;
        let characters: Vec<char> = line.chars().collect();
        let end = self.caret.character.min(characters.len());
        let mut start = end;
        while start > 0
            && (characters[start - 1].is_ascii_alphanumeric() || characters[start - 1] == '_')
        {
            start -= 1;
        }
        if start == end {
            return None;
        }
        Some((start, characters[start..end].iter().collect()))
    }

    fn insert(&mut self, text: &mut String, value: &str) {
        self.delete_selection(text);
        let index = self.global(text, self.caret);
        let byte = byte_of(text, index);
        text.insert_str(byte, value);
        self.place(text, index + value.chars().count());
        self.anchor = None;
    }

    fn backspace(&mut self, text: &mut String) {
        if self.delete_selection(text) {
            return;
        }
        let index = self.global(text, self.caret);
        if index == 0 {
            return;
        }
        let start = byte_of(text, index - 1);
        let end = byte_of(text, index);
        text.replace_range(start..end, "");
        self.place(text, index - 1);
    }

    fn delete(&mut self, text: &mut String) {
        if self.delete_selection(text) {
            return;
        }
        let index = self.global(text, self.caret);
        let characters = text.chars().count();
        if index >= characters {
            return;
        }
        let start = byte_of(text, index);
        let end = byte_of(text, index + 1);
        text.replace_range(start..end, "");
        self.place(text, index);
    }

    fn delete_selection(&mut self, text: &mut String) -> bool {
        let Some((from, to)) = self.ordered() else {
            return false;
        };
        let start = self.global(text, from);
        let end = self.global(text, to);
        let start_byte = byte_of(text, start);
        let end_byte = byte_of(text, end);
        text.replace_range(start_byte..end_byte, "");
        self.anchor = None;
        self.place(text, start);
        true
    }

    fn selection(&self, text: &str) -> Option<String> {
        let (from, to) = self.ordered()?;
        let start = byte_of(text, self.global(text, from));
        let end = byte_of(text, self.global(text, to));
        Some(text[start..end].to_owned())
    }

    fn ordered(&self) -> Option<(Caret, Caret)> {
        let anchor = self.anchor?;
        if anchor == self.caret {
            return None;
        }
        if anchor < self.caret {
            Some((anchor, self.caret))
        } else {
            Some((self.caret, anchor))
        }
    }

    fn horizontal(&mut self, ui: &eframe::egui::Ui, text: &str, lines: &[Placed], forward: bool) {
        if self.delete_arrow() {
            return;
        }
        let placed = &lines[self.caret.line.min(lines.len() - 1)];
        let probe = self.caret.character.saturating_sub(1);
        let right_to_left = placed.line.right_to_left(probe);
        let step = if right_to_left != forward { 1 } else { -1 };
        if step < 0 {
            if self.caret.character > 0 {
                self.caret.character -= 1;
            } else if self.caret.line > 0 {
                self.caret.line -= 1;
                self.caret.character = self.length(text);
            }
        } else if self.caret.character < self.length(text) {
            self.caret.character += 1;
        } else if self.caret.line + 1 < lines.len() {
            self.caret.line += 1;
            self.caret.character = 0;
        }
        self.blink(ui);
        self.scroll = true;
    }

    fn vertical(&mut self, ui: &eframe::egui::Ui, lines: &[Placed], down: bool) {
        let current = &lines[self.caret.line.min(lines.len() - 1)];
        let x = current.line.x_of(ui, self.caret.character);
        let target = if down {
            self.caret.line + 1
        } else {
            self.caret.line.saturating_sub(1)
        };
        if target == self.caret.line || target >= lines.len() {
            return;
        }
        self.caret.line = target;
        self.caret.character = lines[target].line.character_at(ui, x);
        self.anchor = None;
        self.blink(ui);
        self.scroll = true;
    }

    fn delete_arrow(&mut self) -> bool {
        let Some((from, to)) = self.ordered() else {
            return false;
        };
        let _ = to;
        self.caret = from;
        self.anchor = None;
        true
    }

    fn place_at(
        &self,
        ui: &eframe::egui::Ui,
        rect: eframe::egui::Rect,
        lines: &[Placed],
        position: eframe::egui::Pos2,
    ) -> Caret {
        let local = position.y - rect.top();
        let mut line = 0;
        for (index, placed) in lines.iter().enumerate() {
            if local < placed.top + placed.height {
                line = index;
                break;
            }
            line = index;
        }
        let character = lines[line].line.character_at(ui, position.x - rect.left());
        Caret { line, character }
    }

    fn global(&self, text: &str, caret: Caret) -> usize {
        let mut total = 0;
        for (index, line) in text.split('\n').enumerate() {
            if index == caret.line {
                return total + caret.character;
            }
            total += line.chars().count() + 1;
        }
        text.chars().count()
    }

    fn place(&mut self, text: &str, index: usize) {
        let mut remaining = index;
        for (line, content) in text.split('\n').enumerate() {
            let characters = content.chars().count();
            if remaining <= characters {
                self.caret = Caret {
                    line,
                    character: remaining,
                };
                return;
            }
            remaining -= characters + 1;
        }
        let line = text.split('\n').count().saturating_sub(1);
        let character = text.split('\n').last().unwrap_or("").chars().count();
        self.caret = Caret { line, character };
    }

    fn clamp(&mut self, text: &str, lines: &[Placed]) {
        self.caret.line = self.caret.line.min(lines.len().saturating_sub(1));
        let characters = text
            .split('\n')
            .nth(self.caret.line)
            .map_or(0, |line| line.chars().count());
        self.caret.character = self.caret.character.min(characters);
    }

    fn length(&self, text: &str) -> usize {
        text.split('\n')
            .nth(self.caret.line)
            .map_or(0, |line| line.chars().count())
    }

    fn blink(&mut self, ui: &eframe::egui::Ui) {
        self.blink = ui.input(|input| input.time);
    }
}

fn layout(
    ui: &eframe::egui::Ui,
    text: &str,
    font: &eframe::egui::FontId,
    sheet: &dyn Sheet,
) -> Vec<Placed> {
    let row = ui.ctx().fonts_mut(|fonts| fonts.row_height(font));
    let mut placed = Vec::new();
    let mut top = 0.0;
    for (index, line) in text.split('\n').enumerate() {
        let runs = sheet.runs(index, line);
        let line = crate::typography::Line::new(ui, line, &runs, BASE, font);
        let height = line.size().y.max(row);
        placed.push(Placed { top, height, line });
        top += height;
    }
    placed
}

fn byte_of(text: &str, characters: usize) -> usize {
    text.char_indices()
        .nth(characters)
        .map_or(text.len(), |(byte, _)| byte)
}

fn normalize(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '۰'..='۹' => char::from_digit(character as u32 - '۰' as u32, 10).unwrap_or(character),
            '٠'..='٩' => char::from_digit(character as u32 - '٠' as u32, 10).unwrap_or(character),
            '٬' | '،' => ',',
            '٫' => '.',
            _ => character,
        })
        .collect()
}
