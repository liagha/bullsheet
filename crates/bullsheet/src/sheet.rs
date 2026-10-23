
use std::ops::Range;

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, BoxConstraints, BrushIndex, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, PaintCtx,
    PointerButton, PointerButtonEvent, PointerEvent, PointerScrollEvent, PointerUpdate,
    PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, ScrollDelta, StyleProperty, TextEvent,
    Widget, WidgetId, render_text,
};
use masonry::core::keyboard::{Code, Key, KeyState, NamedKey};
use masonry::kurbo::{Affine, Line, Point, Rect, Size, Stroke};
use masonry::parley::layout::Layout;
use masonry::parley::{Affinity, Alignment, AlignmentOptions, Cursor, LayoutContext, PlainEditor, Selection};
use masonry::peniko::{Color, Fill};
use masonry::vello::Scene;
use tracing::Span;

use crate::digits::Digits;
use crate::fonts::Typefaces;
use crate::render::Renderers;
use crate::settings::Settings;
use crate::word::Word;
use crate::writing::Writing;

const BASE: Color = Color::from_rgb8(224, 227, 233);
const DEPTHS: [Color; 3] = [
    Color::from_rgb8(86, 182, 194),
    Color::from_rgb8(206, 145, 120),
    Color::from_rgb8(197, 134, 192),
];
const UNMATCHED: Color = Color::from_rgb8(255, 123, 114);

const BACKGROUND: Color = Color::from_rgb8(32, 32, 32);
const BUTTON_HOVER: Color = Color::from_rgb8(48, 48, 48);
const BUTTON_INK: Color = Color::from_rgb8(170, 176, 188);
const DIM_INK: Color = Color::from_rgb8(110, 116, 128);
const TOOLTIP_BACKGROUND: Color = Color::from_rgb8(48, 48, 50);
const ROW_HOVER: Color = Color::from_rgb8(64, 68, 76);
const TOOLTIP_BORDER: Color = Color::from_rgb8(86, 90, 100);

const FONT_SIZE: f32 = 16.0;
const RESULT_FONT_SIZE: f32 = 16.0;
const TOP_HEIGHT: f64 = 32.0;
const PADDING: f64 = 12.0;
const MIN_GUTTER: f64 = 150.0;
const MAX_GUTTER: f64 = 300.0;
const COMPLETION_ROW_HEIGHT: f64 = 24.0;

const BLINK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SheetAction {
    DragWindow,
    Hide,
    Quit,
}

struct Run {
    span: Range<usize>,
    color: Color,
    writing: Writing,
}

struct ResultRow {
    layout: Layout<BrushIndex>,
    text: String,
    plain: Option<String>,
    color: Color,
    width: f64,
    height: f64,
}

struct Hover {
    text: String,
    position: Point,
}

struct Completion {
    range: Range<usize>,
    entries: Vec<(String, String)>,
    selected: usize,
}

pub struct Sheet {
    editor: PlainEditor<BrushIndex>,
    engine: evaluator::Calculator,
    readings: evaluator::Readings,
    renderers: Renderers,
    settings: Settings,
    typefaces: Typefaces,

    evaluated: String,
    text_layouts: Vec<Layout<BrushIndex>>,
    line_ranges: Vec<Range<usize>>,
    hint_layout: Option<Layout<BrushIndex>>,
    hotkey_layout: Option<Layout<BrushIndex>>,
    brushes: Vec<masonry::peniko::Brush>,
    results: Vec<Option<ResultRow>>,
    row_tops: Vec<f64>,
    row_heights: Vec<f64>,
    content_height: f64,

    gutter_width: f64,
    viewport_height: f64,
    scroll: f64,
    width: f64,

    hotkey: element::Text,
    registered: bool,

    hover: Option<Hover>,
    hovered_result: Option<usize>,
    hovered_button: Option<u8>,
    drag_select: bool,

    completion: Option<Completion>,
    completion_rect: Option<Rect>,

    update_seen: element::Text,

    cursor_visible: bool,
    blink_at: std::time::Instant,
}

impl Sheet {
    pub fn new(settings: Settings, typefaces: Typefaces) -> Self {
        let mut editor = PlainEditor::new(FONT_SIZE);
        {
            let styles = editor.edit_styles();
            styles.insert(StyleProperty::FontStack(
                typefaces.stack(Writing::Latin).clone(),
            ));
            styles.insert(StyleProperty::FontSize(FONT_SIZE));
        }
        let mut engine = evaluator::Calculator::new();
        settings.install(&mut engine);
        let readings = engine.evaluate(&evaluator::Source::new(String::new()));
        let update_seen = settings.update_state();
        Self {
            editor,
            engine,
            readings,
            renderers: Renderers::new(),
            settings,
            typefaces,
            evaluated: String::new(),
            text_layouts: Vec::new(),
            line_ranges: Vec::new(),
            hint_layout: None,
            hotkey_layout: None,
            brushes: Vec::new(),
            results: Vec::new(),
            row_tops: Vec::new(),
            row_heights: Vec::new(),
            content_height: 0.0,
            gutter_width: MIN_GUTTER,
            viewport_height: 0.0,
            scroll: 0.0,
            width: 0.0,
            hotkey: element::Text::new(String::new()),
            registered: false,
            hover: None,
            hovered_result: None,
            hovered_button: None,
            drag_select: false,
            completion: None,
            completion_rect: None,
            update_seen,
            cursor_visible: true,
            blink_at: std::time::Instant::now() + BLINK_INTERVAL,
        }
    }

    pub fn text(&self) -> String {
        self.editor.text().into_iter().collect()
    }

    fn gutter_right(&self, size: Size) -> f64 {
        size.width - PADDING
    }

    fn scroll_max(&self) -> f64 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    fn editor_origin(&self) -> Point {
        Point::new(PADDING, TOP_HEIGHT - self.scroll)
    }

    fn text_width(&self, size: Size) -> f64 {
        size.width - PADDING * 2.0 - self.gutter_width
    }

    fn line_of(&self, byte: usize) -> Option<usize> {
        self.line_ranges
            .iter()
            .position(|range| range.start <= byte && byte <= range.end)
    }

    fn layout_of(&self, byte: usize) -> Option<(&Layout<BrushIndex>, usize)> {
        let line = self.line_of(byte)?;
        Some((self.text_layouts.get(line)?, line))
    }

    fn ime_area(&self) -> Rect {
        let focus = self.editor.raw_selection().focus().index();
        let area = match self.layout_of(focus) {
            Some((layout, line)) => {
                let local = focus - self.line_ranges[line].start;
                Cursor::from_byte_index(layout, local, Affinity::Downstream).geometry(layout, 0.)
            }
            None => self.editor.ime_cursor_area(),
        };
        let origin = self.editor_origin();
        Rect::new(
            origin.x + area.x0 as f64,
            origin.y + area.y0 as f64,
            origin.x + area.x1 as f64,
            origin.y + area.y1 as f64,
        )
    }

    fn byte_at(&self, x: f32, y: f64) -> Option<usize> {
        let row = self.row_at(y)?;
        let layout = self.text_layouts.get(row)?;
        let index = Cursor::from_point(layout, x, y as f32).index();
        Some(self.line_ranges.get(row)?.start + index)
    }

    fn revaluate_if_needed(&mut self) {
        let text = self.text();
        if text == self.evaluated {
            return;
        }
        self.evaluated = text.clone();
        self.scroll = self.scroll.min(self.scroll_max());
        self.readings = self.engine.evaluate(&evaluator::Source::new(text));
        self.text_layouts.clear();
        self.line_ranges.clear();
    }

    fn build_text(
        &mut self,
        fcx: &mut masonry::parley::FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
        width: f64,
    ) {
        if !self.text_layouts.is_empty() {
            return;
        }
        let text = self.text();
        self.brushes = vec![BASE.into()];

        let mut layouts: Vec<Layout<BrushIndex>> = Vec::new();
        let mut ranges: Vec<Range<usize>> = Vec::new();
        let mut line_start = 0usize;
        for (index, line) in text.split('\n').enumerate() {
            let boundaries: Vec<usize> = line
                .char_indices()
                .map(|(byte, _)| byte)
                .chain(std::iter::once(line.len()))
                .collect();
            let characters: Vec<char> = line.chars().collect();
            let mut colors = vec![BASE; characters.len()];

            if let Some(reading) = self.readings.get(index) {
                for part in reading.parts() {
                    let span = part.span();
                    let start = span.start.min(characters.len());
                    let end = span.end.min(characters.len()).max(start);
                    let color = self
                        .renderers
                        .render(part.value())
                        .map_or(BASE, |shown| shown.color);
                    for slot in colors.iter_mut().take(end).skip(start) {
                        *slot = color;
                    }
                }
            }

            let mut open: Vec<usize> = Vec::new();
            for (position, character) in characters.iter().enumerate() {
                match character {
                    '(' => open.push(position),
                    ')' => match open.pop() {
                        Some(partner) => {
                            let color = DEPTHS[open.len() % DEPTHS.len()];
                            colors[partner] = color;
                            colors[position] = color;
                        }
                        None => colors[position] = UNMATCHED,
                    },
                    _ => {}
                }
            }
            for partner in open {
                colors[partner] = UNMATCHED;
            }

            let writings = Writing::each(&characters);
            let mut runs: Vec<Run> = Vec::new();
            let mut start = 0usize;
            for position in 1..=characters.len() {
                if position < characters.len()
                    && colors[position] == colors[start]
                    && writings[position] == writings[start]
                {
                    continue;
                }
                runs.push(Run {
                    span: boundaries[start]..boundaries[position],
                    color: colors[start],
                    writing: writings[start],
                });
                start = position;
            }

            let mut builder = lcx.ranged_builder(fcx, line, 1.0, true);
            builder.push_default(StyleProperty::FontStack(
                self.typefaces.stack(Writing::Latin).clone(),
            ));
            builder.push_default(StyleProperty::FontSize(FONT_SIZE));
            for run in runs {
                if run.span.start < run.span.end
                    && run.span.end <= line.len()
                    && line.is_char_boundary(run.span.start)
                    && line.is_char_boundary(run.span.end)
                {
                    let brush = self.brushes.len();
                    self.brushes.push(run.color.into());
                    builder.push(StyleProperty::Brush(BrushIndex(brush)), run.span.clone());
                    builder.push(
                        StyleProperty::FontStack(self.typefaces.stack(run.writing).clone()),
                        run.span,
                    );
                }
            }
            let mut layout = builder.build(line);
            layout.break_all_lines(None);
            layout.align(Some(width as f32), Alignment::Start, AlignmentOptions::default());
            layouts.push(layout);
            ranges.push(line_start..line_start + line.len());
            line_start += line.len() + 1;
        }

        self.text_layouts = layouts;
        self.line_ranges = ranges;
    }

    fn build_results(
        &mut self,
        fcx: &mut masonry::parley::FontContext,
        lcx: &mut LayoutContext<BrushIndex>,
    ) {
        let mut results = Vec::with_capacity(self.readings.len());
        let mut widest: f64 = MIN_GUTTER - 16.0;
        for reading in self.readings.iter() {
            let Some(shown) = self.renderers.render(reading.value()) else {
                results.push(None);
                continue;
            };
            let text = shown.text.to_string();
            let layout = self.typefaces.layout(Writing::of_text(&text), fcx, lcx, &text, RESULT_FONT_SIZE);
            let width = layout.full_width() as f64;
            widest = widest.max(width);
            results.push(Some(ResultRow {
                layout,
                text,
                plain: shown.plain.map(|plain| plain.to_string()),
                color: shown.color,
                width,
                height: 0.0,
            }));
        }
        let gutter = (widest + 20.0).clamp(MIN_GUTTER, MAX_GUTTER);
        for row in results.iter_mut().flatten() {
            if row.width > gutter - 16.0 {
                let scale = (gutter - 16.0) / row.width;
                let size = (RESULT_FONT_SIZE as f64 * scale).max(9.0) as f32;
                let layout = self.typefaces.layout(Writing::of_text(&row.text), fcx, lcx, &row.text, size);
                row.width = layout.full_width() as f64;
                row.layout = layout;
            }
            row.height = row.layout.height() as f64;
        }
        self.gutter_width = gutter;
        self.results = results;
    }

    fn compute_rows(&mut self) {
        self.row_tops.clear();
        self.row_heights.clear();
        let mut top = 0.0;
        for layout in &self.text_layouts {
            for line in layout.lines() {
                let height = line.metrics().line_height as f64;
                self.row_tops.push(top);
                self.row_heights.push(height);
                top += height;
            }
        }
        if self.row_tops.is_empty() {
            self.row_tops.push(0.0);
            self.row_heights.push(FONT_SIZE as f64 * 1.4);
            top = FONT_SIZE as f64 * 1.4;
        }
        self.content_height = top;
    }

    fn row_at(&self, y: f64) -> Option<usize> {
        let y = y - TOP_HEIGHT + self.scroll;
        if y < 0.0 {
            return None;
        }
        let mut top = 0.0;
        for (index, height) in self.row_heights.iter().enumerate() {
            if y >= top && y < top + height {
                return Some(index);
            }
            top += height;
        }
        None
    }

    fn visible(&self, index: usize) -> bool {
        let Some(&top) = self.row_tops.get(index) else {
            return false;
        };
        let Some(&height) = self.row_heights.get(index) else {
            return false;
        };
        top + height > self.scroll && top < self.scroll + self.viewport_height
    }

    fn line(&self, index: usize) -> String {
        self.text().split('\n').nth(index).unwrap_or("").to_owned()
    }

    fn locate(&self, byte: usize) -> Option<String> {
        let text = self.text();
        let mut line_start = 0usize;
        let mut line_index = 0usize;
        for line in text.split('\n') {
            if byte <= line_start + line.len() {
                let local_bytes = byte - line_start;
                let character = line[..local_bytes.min(line.len())].chars().count();
                let reading = self.readings.get(line_index)?;
                let part = reading
                    .parts()
                    .iter()
                    .find(|part| part.span().contains(&character))?;
                let span = part.span();
                let word: String = line
                    .chars()
                    .skip(span.start)
                    .take(span.end - span.start)
                    .collect();
                return Some(match self.renderers.render(part.value()) {
                    Some(shown) => format!(
                        "{} {} — ctrl-click copies",
                        part.value().describe(),
                        shown.text
                    ),
                    None => word,
                });
            }
            line_start += line.len() + 1;
            line_index += 1;
        }
        None
    }

    fn selection_in(
        &self,
        layout: &Layout<BrushIndex>,
        line: usize,
    ) -> Option<Vec<(masonry::parley::BoundingBox, usize)>> {
        let range = self.line_ranges.get(line)?;
        let anchor = self.editor.raw_selection().anchor().index();
        let focus = self.editor.raw_selection().focus().index();
        let start = anchor.max(range.start).min(range.end);
        let end = focus.max(range.start).min(range.end);
        if start == end {
            return None;
        }
        let anchor =
            Cursor::from_byte_index(layout, start - range.start, Affinity::Downstream);
        let focus = Cursor::from_byte_index(layout, end - range.start, Affinity::Downstream);
        Some(Selection::new(anchor, focus).geometry(layout))
    }

    fn update_hover(&mut self, position: Point, size: Size) -> bool {
        let previous = self.hover.as_ref().map(|hover| hover.text.clone());
        let previous_result = self.hovered_result;
        let previous_button = self.hovered_button;
        self.hover = None;
        self.hovered_result = None;
        self.hovered_button = None;

        if position.y < TOP_HEIGHT {
            if self.close_rect(size).contains(position) {
                self.hovered_button = Some(1);
            } else if self.registered && self.hide_rect(size).contains(position) {
                self.hovered_button = Some(0);
            }
        } else if position.x >= size.width - PADDING - self.gutter_width {
            if let Some(row) = self.row_at(position.y)
                && self.results.get(row).is_some_and(Option::is_some)
            {
                self.hovered_result = Some(row);
            }
        } else {
            let origin = self.editor_origin();
            let local = Point::new(position.x - origin.x, position.y - origin.y);
            if local.x >= 0.0
                && local.y >= 0.0
                && let Some(row) = self.row_at(local.y)
                && let Some(layout) = self.text_layouts.get(row)
            {
                let cursor = Cursor::from_point(layout, local.x as f32, local.y as f32);
                let index = self.line_ranges[row].start + cursor.index();
                self.hover = self.locate(index).map(|text| Hover { text, position });
            }
        }

        previous != self.hover.as_ref().map(|hover| hover.text.clone())
            || previous_result != self.hovered_result
            || previous_button != self.hovered_button
    }

    fn close_rect(&self, size: Size) -> Rect {
        Rect::new(size.width - 36.0, 4.0, size.width - 16.0, 26.0)
    }

    fn hide_rect(&self, size: Size) -> Rect {
        Rect::new(size.width - 68.0, 4.0, size.width - 48.0, 26.0)
    }

    fn submit(&mut self, ctx: &mut EventCtx<'_>, action: SheetAction) {
        ctx.submit_action::<SheetAction>(action);
    }

    pub fn tick_blink(&mut self) -> bool {
        let now = std::time::Instant::now();
        if now < self.blink_at {
            return false;
        }
        self.blink_at = now + BLINK_INTERVAL;
        self.cursor_visible = !self.cursor_visible;
        true
    }

    fn show_caret(&mut self) {
        self.cursor_visible = true;
        self.blink_at = std::time::Instant::now() + BLINK_INTERVAL;
    }

    fn refresh_completion(&mut self) {
        let previous = self.completion.take();
        let selection = self.editor.raw_selection();
        if !selection.is_collapsed() {
            return;
        }
        let text = self.text();
        let cursor = selection.focus().index().min(text.len());
        let Some(word) = Word::ending_at(&element::Text::new(&text), cursor) else {
            return;
        };
        let prefix = word.text().as_ref().to_lowercase();
        let mut entries: Vec<(String, String)> = self
            .readings
            .names()
            .iter()
            .filter(|name| name.text().as_ref().to_lowercase().starts_with(&prefix))
            .map(|name| {
                (
                    name.text().to_string(),
                    name.value().describe().to_string(),
                )
            })
            .collect();
        if entries.is_empty() {
            return;
        }
        entries.sort_by(|left, right| {
            left.0.to_lowercase().cmp(&right.0.to_lowercase())
        });
        entries.truncate(8);
        let selected = match previous {
            Some(previous) if previous.range == word.range() => {
                previous.selected.min(entries.len() - 1)
            }
            _ => 0,
        };
        self.completion = Some(Completion {
            range: word.range(),
            entries,
            selected,
        });
    }

    fn accept_completion(&mut self, ctx: &mut EventCtx<'_>) -> bool {
        let Some(completion) = self.completion.take() else {
            return false;
        };
        let Some((text, _)) = completion.entries.get(completion.selected) else {
            return false;
        };
        let text = text.clone();
        let (fctx, lctx) = ctx.text_contexts();
        self.editor
            .driver(fctx, lctx)
            .select_byte_range(completion.range.start, completion.range.end);
        self.editor
            .driver(fctx, lctx)
            .insert_or_replace_selection(&text);
        self.completion_rect = None;
        self.hover = None;
        ctx.request_layout();
        true
    }

    fn cycle_completion(&mut self, delta: isize) {
        let Some(completion) = &mut self.completion else {
            return;
        };
        let count = completion.entries.len() as isize;
        if count == 0 {
            self.completion = None;
            return;
        }
        completion.selected = (completion.selected as isize + delta).rem_euclid(count) as usize;
    }
}

impl Widget for Sheet {
    type Action = SheetAction;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if self.editor.is_composing() {
            return;
        }
        let size = ctx.size();
        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: None | Some(PointerButton::Primary),
                state,
                ..
            }) => {
                let position = ctx.local_position(state.position);
                if let Some(rect) = self.completion_rect
                    && rect.contains(position)
                {
                    let row = ((position.y - rect.y0 - 4.0) / COMPLETION_ROW_HEIGHT)
                        .floor()
                        .max(0.0) as usize;
                    if let Some(completion) = &mut self.completion {
                        completion.selected = row.min(completion.entries.len() - 1);
                    }
                    self.accept_completion(ctx);
                    ctx.set_handled();
                    return;
                }
                self.completion = None;
                self.completion_rect = None;
                if position.y < TOP_HEIGHT {
                    if self.close_rect(size).contains(position) {
                        self.submit(ctx, SheetAction::Quit);
                    } else if self.registered && self.hide_rect(size).contains(position) {
                        self.submit(ctx, SheetAction::Hide);
                    } else {
                        self.submit(ctx, SheetAction::DragWindow);
                    }
                    ctx.set_handled();
                    return;
                }

                let gutter_left = size.width - PADDING - self.gutter_width;
                if position.x >= gutter_left {
                    if let Some(row) = self.row_at(position.y)
                        && let Some(Some(result)) = self.results.get(row)
                    {
                        let line = self.line(row);
                        let line = line.trim();
                        let copied = if state.modifiers.ctrl() {
                            if line.contains('=') {
                                line.to_owned()
                            } else {
                                match &result.plain {
                                    Some(plain) => format!("{line} = {plain}"),
                                    None => line.to_owned(),
                                }
                            }
                        } else {
                            match &result.plain {
                                Some(plain) => plain.clone(),
                                None => result.text.clone(),
                            }
                        };
                        ctx.set_clipboard(copied);
                    }
                    ctx.set_handled();
                    return;
                }

                let origin = self.editor_origin();
                let x = (position.x - origin.x) as f32;
                let y = (position.y - origin.y) as f32;
                if x < 0.0 || y < 0.0 {
                    return;
                }
                let (fctx, lctx) = ctx.text_contexts();
                    let index = self.byte_at(x, y as f64);
                let mut driver = self.editor.driver(fctx, lctx);
                match state.count {
                    2 => driver.select_word_at_point(x, y),
                    3 => driver.select_hard_line_at_point(x, y),
                    _ => {
                        if state.modifiers.shift() {
                            driver.shift_click_extension(x, y);
                        } else if let Some(index) = index {
                            driver.move_to_byte(index);
                        } else {
                            driver.move_to_point(x, y);
                        }
                    }
                }
                drop(driver);
                self.hover = None;
                self.drag_select = true;
                ctx.request_focus();
                ctx.capture_pointer();
                ctx.request_render();
                ctx.set_ime_area(self.ime_area());
                ctx.set_handled();
            }
            PointerEvent::Up(PointerButtonEvent { .. }) => {
                self.drag_select = false;
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                let position = ctx.local_position(current.position);
                if self.completion.is_some()
                    && let Some(rect) = self.completion_rect
                    && rect.contains(position)
                {
                    let row = ((position.y - rect.y0 - 4.0) / COMPLETION_ROW_HEIGHT)
                        .floor()
                        .max(0.0) as usize;
                    let mut changed = false;
                    if let Some(completion) = &mut self.completion {
                        let row = row.min(completion.entries.len() - 1);
                        if completion.selected != row {
                            completion.selected = row;
                            changed = true;
                        }
                    }
                    self.hover = None;
                    self.hovered_result = None;
                    self.hovered_button = None;
                    if changed {
                        ctx.request_paint_only();
                    }
                    return;
                }
                if ctx.is_active() && self.drag_select {
                    let origin = self.editor_origin();
                    let x = (position.x - origin.x) as f32;
                    let y = (position.y - origin.y) as f32;
                    let (fctx, lctx) = ctx.text_contexts();
                    let index = self.byte_at(x, y as f64);
                    let mut driver = self.editor.driver(fctx, lctx);
                    if let Some(index) = index {
                        driver.extend_selection_to_byte(index);
                    } else {
                        driver.extend_selection_to_point(x, y);
                    }
                    ctx.request_render();
                    ctx.set_ime_area(self.ime_area());
                }

                if self.update_hover(position, size) {
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                let amount = match delta {
                    ScrollDelta::PixelDelta(position) => position.y,
                    ScrollDelta::LineDelta(_, y) => *y as f64 * 24.0,
                    ScrollDelta::PageDelta(_, y) => *y as f64 * self.viewport_height,
                };
                self.scroll = (self.scroll - amount).clamp(0.0, self.scroll_max());
                ctx.request_paint_only();
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        self.show_caret();

        match event {
            TextEvent::Keyboard(key_event) => {
                if key_event.state != KeyState::Down || self.editor.is_composing() {
                    return;
                }
                if self.completion.is_some() {
                    match &key_event.key {
                        Key::Named(NamedKey::ArrowDown) => {
                            self.cycle_completion(1);
                            ctx.request_render();
                            ctx.set_handled();
                            return;
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            self.cycle_completion(-1);
                            ctx.request_render();
                            ctx.set_handled();
                            return;
                        }
                        Key::Named(NamedKey::Tab) => {
                            if self.accept_completion(ctx) {
                                ctx.set_handled();
                                return;
                            }
                        }
                        Key::Named(NamedKey::Escape) => {
                            self.completion = None;
                            self.completion_rect = None;
                            ctx.request_render();
                            ctx.set_handled();
                            return;
                        }
                        _ => {}
                    }
                }
                let caret_before = (
                    self.editor.raw_selection().focus().index(),
                    self.editor.raw_selection().is_collapsed(),
                );
                let (shift, action_mod) = (
                    key_event.modifiers.shift(),
                    if cfg!(target_os = "macos") {
                        key_event.modifiers.meta()
                    } else {
                        key_event.modifiers.ctrl()
                    },
                );
                let (fctx, lctx) = ctx.text_contexts();
                let mut edited = false;
                match &key_event.key {
                    Key::Named(NamedKey::Escape) => {
                        self.submit(ctx, SheetAction::Hide);
                        ctx.set_handled();
                        return;
                    }
                    Key::Character(_) if action_mod && key_event.code == Code::KeyQ => {
                        self.submit(ctx, SheetAction::Quit);
                        ctx.set_handled();
                        return;
                    }
                    Key::Character(_) if action_mod && key_event.code == Code::KeyX => {
                        if let Some(text) = self.editor.selected_text()
                            && !text.is_empty()
                        {
                            let text = text.to_string();
                            self.editor.driver(fctx, lctx).delete_selection();
                            edited = true;
                            ctx.set_clipboard(text);
                        }
                    }
                    Key::Character(_) if action_mod && key_event.code == Code::KeyC => {
                        if let Some(text) = self.editor.selected_text()
                            && !text.is_empty()
                        {
                            ctx.set_clipboard(text.to_string());
                        }
                    }
                    Key::Character(_) if action_mod && key_event.code == Code::KeyA => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if shift {
                            driver.collapse_selection();
                        } else {
                            driver.select_all();
                        }
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if action_mod {
                            if shift {
                                driver.select_word_left();
                            } else {
                                driver.move_word_left();
                            }
                        } else if shift {
                            driver.select_left();
                        } else {
                            driver.move_left();
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if action_mod {
                            if shift {
                                driver.select_word_right();
                            } else {
                                driver.move_word_right();
                            }
                        } else if shift {
                            driver.select_right();
                        } else {
                            driver.move_right();
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if shift {
                            driver.select_up();
                        } else {
                            driver.move_up();
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if shift {
                            driver.select_down();
                        } else {
                            driver.move_down();
                        }
                    }
                    Key::Named(NamedKey::Home) => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if action_mod {
                            if shift {
                                driver.select_to_text_start();
                            } else {
                                driver.move_to_text_start();
                            }
                        } else if shift {
                            driver.select_to_line_start();
                        } else {
                            driver.move_to_line_start();
                        }
                    }
                    Key::Named(NamedKey::End) => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if action_mod {
                            if shift {
                                driver.select_to_text_end();
                            } else {
                                driver.move_to_text_end();
                            }
                        } else if shift {
                            driver.select_to_line_end();
                        } else {
                            driver.move_to_line_end();
                        }
                    }
                    Key::Named(NamedKey::Delete) => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if action_mod {
                            driver.delete_word();
                        } else {
                            driver.delete();
                        }
                        edited = true;
                    }
                    Key::Named(NamedKey::Backspace) => {
                        let mut driver = self.editor.driver(fctx, lctx);
                        if action_mod {
                            driver.backdelete_word();
                        } else {
                            driver.backdelete();
                        }
                        edited = true;
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.editor
                            .driver(fctx, lctx)
                            .insert_or_replace_selection("\n");
                        edited = true;
                    }
                    Key::Named(NamedKey::Tab) => return,
                    Key::Character(text) if !action_mod => {
                        let text = Digits::to_latin(&element::Text::new(text.as_str()));
                        if !text.as_ref().is_empty() {
                            self.editor
                                .driver(fctx, lctx)
                                .insert_or_replace_selection(text.as_ref());
                            edited = true;
                        }
                    }
                    _ => return,
                }
                ctx.set_handled();
                let caret_after = (
                    self.editor.raw_selection().focus().index(),
                    self.editor.raw_selection().is_collapsed(),
                );
                if edited || caret_before != caret_after {
                    self.refresh_completion();
                }
                if edited {
                    ctx.request_layout();
                    self.hover = None;
                } else {
                    ctx.request_render();
                    ctx.set_ime_area(self.ime_area());
                }
            }
            TextEvent::WindowFocusChange(focused) => {
                if *focused {
                    self.show_caret();
                }
                ctx.request_paint_only();
            }
            TextEvent::Ime(ime) => {
                let (fctx, lctx) = ctx.text_contexts();
                let mut edited = false;
                match ime {
                    masonry::core::Ime::Disabled => {
                        self.editor.driver(fctx, lctx).clear_compose();
                    }
                    masonry::core::Ime::Preedit(text, cursor) => {
                        if text.is_empty() {
                            self.editor.driver(fctx, lctx).clear_compose();
                        } else {
                            self.editor.driver(fctx, lctx).set_compose(text, *cursor);
                            edited = true;
                        }
                    }
                    masonry::core::Ime::Commit(text) => {
                        let text = Digits::to_latin(&element::Text::new(text.as_str()));
                        self.editor
                            .driver(fctx, lctx)
                            .insert_or_replace_selection(text.as_ref());
                        edited = true;
                    }
                    masonry::core::Ime::Enabled => {}
                }
                ctx.set_handled();
                if edited {
                    self.refresh_completion();
                    ctx.request_layout();
                }
            }
            TextEvent::ClipboardPaste(text) => {
                let text = Digits::to_latin(&element::Text::new(text.as_str()));
                let (fctx, lctx) = ctx.text_contexts();
                self.editor
                    .driver(fctx, lctx)
                    .insert_or_replace_selection(text.as_ref());
                self.refresh_completion();
                ctx.request_layout();
            }
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = Size::new(
            if bc.max().width.is_finite() {
                bc.max().width
            } else {
                640.0
            },
            if bc.max().height.is_finite() {
                bc.max().height
            } else {
                440.0
            },
        );
        self.viewport_height = (size.height - TOP_HEIGHT - 8.0).max(20.0);
        {
            let (fcx, lctx) = ctx.text_contexts();
            self.revaluate_if_needed();
            self.build_results(fcx, lctx);
            let width = self.text_width(size);
            if self.width != width {
                self.width = width;
                self.editor.set_width(Some(width as f32));
            }
            let _ = self.editor.layout(fcx, lctx);
            self.build_text(fcx, lctx, self.width);
        }
        self.hint_layout = None;
        self.hotkey_layout = None;
        self.hotkey = self.settings.text();
        self.registered = self.settings.registered();
        {
            let (fcx, lctx) = ctx.text_contexts();
            self.hint_layout =
                Some(self.typefaces.layout(Writing::Latin, fcx, lctx, "Type an expression", FONT_SIZE));
            self.hotkey_layout =
                Some(self.typefaces.layout(Writing::Latin, fcx, lctx, self.hotkey.as_ref(), 11.0));
        }
        self.compute_rows();
        self.scroll = self.scroll.min(self.scroll_max());
        ctx.set_ime_area(self.ime_area());
        bc.constrain(size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();

        let update = self.settings.update_state();
        if update != self.update_seen {
            self.update_seen = update;
            self.evaluated.clear();
            self.text_layouts.clear();
            self.line_ranges.clear();
            let (fcx, lctx) = ctx.text_contexts();
            self.revaluate_if_needed();
            self.build_text(fcx, lctx, self.width);
            self.build_results(fcx, lctx);
            self.compute_rows();
            self.scroll = self.scroll.min(self.scroll_max());
        }

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            BACKGROUND,
            None,
            &size.to_rect(),
        );

        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(48, 48, 48),
            None,
            &Line::new((0.0, TOP_HEIGHT), (size.width, TOP_HEIGHT)),
        );
        for button in 0..2u8 {
            let rect = if button == 0 {
                self.hide_rect(size)
            } else {
                self.close_rect(size)
            };
            if self.hovered_button == Some(button) {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    BUTTON_HOVER,
                    None,
                    &rect.to_rounded_rect(4.0),
                );
            }
            let center = rect.center();
            if button == 0 {
                scene.stroke(
                    &Stroke::new(1.4),
                    Affine::IDENTITY,
                    BUTTON_INK,
                    None,
                    &Line::new((center.x - 6.0, center.y), (center.x + 6.0, center.y)),
                );
            } else {
                scene.stroke(
                    &Stroke::new(1.4),
                    Affine::IDENTITY,
                    BUTTON_INK,
                    None,
                    &Line::new((center.x - 5.0, center.y - 5.0), (center.x + 5.0, center.y + 5.0)),
                );
                scene.stroke(
                    &Stroke::new(1.4),
                    Affine::IDENTITY,
                    BUTTON_INK,
                    None,
                    &Line::new((center.x - 5.0, center.y + 5.0), (center.x + 5.0, center.y - 5.0)),
                );
            }
        }
        if let Some(hotkey) = &self.hotkey_layout {
            let color = if self.registered { DIM_INK } else { UNMATCHED };
            render_text(
                scene,
                Affine::translate((PADDING, 9.0)),
                hotkey,
                &[color.into()],
                true,
            );
        }

        let origin = self.editor_origin();

        let viewport = Rect::new(
            PADDING,
            TOP_HEIGHT,
            PADDING + self.width,
            size.height,
        );
        scene.push_clip_layer(Affine::IDENTITY, &viewport);

        if ctx.is_focus_target() {
            let selection_color = if ctx.is_window_focused() {
                Color::from_rgb8(72, 72, 72)
            } else {
                Color::from_rgb8(52, 52, 52)
            };
            for (index, layout) in self.text_layouts.iter().enumerate() {
                if !self.visible(index) {
                    continue;
                }
                let Some(area) = self.selection_in(layout, index) else {
                    continue;
                };
                for (area, _) in area {
                    let rect = Rect::new(
                        origin.x + area.x0 as f64,
                        origin.y + area.y0 as f64 + self.row_tops[index],
                        origin.x + area.x1 as f64,
                        origin.y + area.y1 as f64 + self.row_tops[index],
                    );
                    scene.fill(Fill::NonZero, Affine::IDENTITY, selection_color, None, &rect);
                }
            }
        }

        for (index, layout) in self.text_layouts.iter().enumerate() {
            if !self.visible(index) {
                continue;
            }
            render_text(
                scene,
                Affine::translate((origin.x, origin.y + self.row_tops[index])),
                layout,
                &self.brushes,
                true,
            );
        }

        if self.text().is_empty()
            && let Some(hint) = &self.hint_layout
        {
            render_text(
                scene,
                Affine::translate((origin.x, origin.y)),
                hint,
                &[DIM_INK.into()],
                true,
            );
        }

        if ctx.is_focus_target()
            && self.cursor_visible
            && ctx.is_window_focused()
            && let Some((layout, line)) = self.layout_of(
                self.editor.raw_selection().focus().index(),
            )
        {
            let focus = self.editor.raw_selection().focus().index();
            let local = focus - self.line_ranges[line].start;
            let cursor =
                Cursor::from_byte_index(layout, local, Affinity::Downstream).geometry(layout, 1.5);
            let rect = Rect::new(
                origin.x + cursor.x0 as f64,
                origin.y + cursor.y0 as f64 + self.row_tops[line],
                origin.x + cursor.x1 as f64,
                origin.y + cursor.y1 as f64 + self.row_tops[line],
            );
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                BASE,
                None,
                &rect,
            );
        }
        scene.pop_layer();

        let gutter_right = self.gutter_right(size);
        for (index, row) in self.results.iter().enumerate() {
            let Some(row) = row else { continue };
            let Some(&top) = self.row_tops.get(index) else {
                continue;
            };
            let Some(&height) = self.row_heights.get(index) else {
                continue;
            };
            let y = TOP_HEIGHT - self.scroll + top;
            if y + height < TOP_HEIGHT || y > size.height {
                continue;
            }
            if self.hovered_result == Some(index) {
                let rect = Rect::new(
                    gutter_right - self.gutter_width,
                    y,
                    gutter_right,
                    y + height,
                );
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    BUTTON_HOVER,
                    None,
                    &rect.to_rounded_rect(4.0),
                );
            }
            let x = gutter_right - row.width;
            let baseline = y + (height - row.height) / 2.0;
            render_text(
                scene,
                Affine::translate((x, baseline)),
                &row.layout,
                &[row.color.into()],
                true,
            );
        }

        self.completion_rect = None;
        if let Some(completion) = &self.completion {
            let origin = self.editor_origin();
            let caret = self.editor.cursor_geometry(1.5);
            let (mut x, mut y) = match caret {
                Some(caret) => (
                    origin.x + caret.x0 as f64,
                    origin.y + caret.y1 as f64 + 2.0,
                ),
                None => (origin.x, origin.y),
            };
            let (fcx, lcx) = ctx.text_contexts();
            let rows: Vec<(Layout<BrushIndex>, Layout<BrushIndex>, f64)> = completion
                .entries
                .iter()
                .map(|(text, description)| {
                    let left = self.typefaces.layout(Writing::of_text(text), fcx, lcx, text, 13.0);
                    let right = self.typefaces.layout(Writing::of_text(description), fcx, lcx, description, 10.0);
                    let width = left.full_width() as f64 + 10.0 + right.full_width() as f64;
                    (left, right, width)
                })
                .collect();
            let width = rows
                .iter()
                .map(|(_, _, width)| *width)
                .fold(140.0_f64, f64::max)
                + 16.0;
            let height = rows.len() as f64 * COMPLETION_ROW_HEIGHT + 8.0;
            if y + height > size.height - 6.0 {
                if let Some(caret) = caret {
                    y = origin.y + caret.y0 as f64 - height - 2.0;
                }
            }
            x = x.min(size.width - width - 6.0).max(6.0);
            y = y.max(TOP_HEIGHT + 2.0);
            let rect = Rect::new(x, y, x + width, y + height);
            self.completion_rect = Some(rect);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                TOOLTIP_BACKGROUND,
                None,
                &rect.to_rounded_rect(5.0),
            );
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                TOOLTIP_BORDER,
                None,
                &rect.to_rounded_rect(5.0),
            );
            for (index, (left, right, _)) in rows.iter().enumerate() {
                let row_y = y + 4.0 + index as f64 * COMPLETION_ROW_HEIGHT;
                if index == completion.selected {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        ROW_HOVER,
                        None,
                        &Rect::new(
                            x + 4.0,
                            row_y,
                            x + width - 4.0,
                            row_y + COMPLETION_ROW_HEIGHT,
                        )
                        .to_rounded_rect(3.0),
                    );
                }
                let left_y = row_y + (COMPLETION_ROW_HEIGHT - left.height() as f64) / 2.0;
                render_text(
                    scene,
                    Affine::translate((x + 8.0, left_y)),
                    left,
                    &[BASE.into()],
                    true,
                );
                let right_x = x + width - 8.0 - right.full_width() as f64;
                let right_y = row_y + (COMPLETION_ROW_HEIGHT - right.height() as f64) / 2.0;
                render_text(
                    scene,
                    Affine::translate((right_x, right_y)),
                    right,
                    &[DIM_INK.into()],
                    true,
                );
            }
        }

        if let Some(hover) = &self.hover {
            let (fcx, lcx) = ctx.text_contexts();
            let layout = self.typefaces.layout(Writing::of_text(&hover.text), fcx, lcx, &hover.text, 12.0);
            let width = layout.full_width() as f64 + 12.0;
            let height = layout.height() as f64 + 8.0;
            let mut x = hover.position.x + 14.0;
            let mut y = hover.position.y + 20.0;
            if x + width > size.width - 6.0 {
                x = (size.width - width - 6.0).max(6.0);
            }
            if y + height > size.height - 6.0 {
                y = hover.position.y - height - 8.0;
            }
            let rect = Rect::new(x, y, x + width, y + height);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                TOOLTIP_BACKGROUND,
                None,
                &rect.to_rounded_rect(5.0),
            );
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                TOOLTIP_BORDER,
                None,
                &rect.to_rounded_rect(5.0),
            );
            render_text(
                scene,
                Affine::translate((x + 6.0, y + 4.0)),
                &layout,
                &[BASE.into()],
                true,
            );
        }
    }

    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        let size = ctx.size();
        if pos.y < TOP_HEIGHT {
            if self.close_rect(size).contains(pos)
                || (self.registered && self.hide_rect(size).contains(pos))
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        } else if self
            .completion_rect
            .is_some_and(|rect| rect.contains(pos))
        {
            CursorIcon::Pointer
        } else if pos.x >= size.width - PADDING - self.gutter_width {
            if self
                .row_at(pos.y)
                .is_some_and(|row| self.results.get(row).is_some_and(Option::is_some))
            {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            }
        } else {
            CursorIcon::Text
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn accessibility_role(&self) -> Role {
        Role::MultilineTextInput
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label("bullsheet");
    }

    fn make_trace_span(&self, id: WidgetId) -> Span {
        tracing::trace_span!("Sheet", id = id.trace())
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(format!("{} lines", self.row_tops.len()))
    }
}
