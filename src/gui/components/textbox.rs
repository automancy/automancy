use core::fmt;
use std::cell::RefCell;
use std::f32::INFINITY;
use std::mem;
use std::rc::Rc;

use automancy_defs::colors;
use fontdue::layout::{Layout, LinePosition};
use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    shapes::{outline, RoundedRectangle},
};
use yakui::{
    geometry::{Color, Constraints, Vec2},
    widgets::Pad,
};
use yakui::{
    input::{KeyCode, MouseButton},
    util::widget,
};
use yakui::{
    pad,
    widget::{EventContext, LayoutContext, PaintContext, Widget},
};
use yakui::{widgets::RenderTextBox, Response};

use super::{
    text::{label_text, Text},
    PADDING_MEDIUM,
};

/**
Text that can be edited.

Responds with [TextBoxResponse].
*/
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TextBox {
    pub text: Text,
    pub padding: Pad,
    pub fill: Option<Color>,
    /// Drawn when no text has been set
    pub placeholder: String,
}

impl TextBox {
    pub fn new(text: Text, placeholder: String) -> Self {
        Self {
            text,
            padding: Pad::all(PADDING_MEDIUM),
            fill: Some(colors::BACKGROUND_1),
            placeholder,
        }
    }

    pub fn show(self) -> Response<TextBoxResponse> {
        widget::<TextBoxWidget>(self)
    }
}

pub struct TextBoxWidget {
    props: TextBox,
    updated_text: Option<String>,
    selected: bool,
    cursor: usize,
    text_layout: Option<Rc<RefCell<Layout>>>,
    activated: bool,
    lost_focus: bool,
}

impl fmt::Debug for TextBoxWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextBoxWidget")
            .field("props", &self.props)
            .field("updated_text", &self.updated_text)
            .field("selected", &self.selected)
            .field("cursor", &self.cursor)
            .field("activated", &self.activated)
            .field("lost_focus", &self.lost_focus)
            .finish_non_exhaustive()
    }
}

pub struct TextBoxResponse {
    pub text: Option<String>,
    /// Whether the user pressed "Enter" in this box
    pub activated: bool,
    /// Whether the box lost focus
    pub lost_focus: bool,
}

impl Widget for TextBoxWidget {
    type Props<'a> = TextBox;
    type Response = TextBoxResponse;

    fn new() -> Self {
        Self {
            props: TextBox::new(label_text(""), "".to_string()),
            updated_text: None,
            selected: false,
            cursor: 0,
            text_layout: None,
            activated: false,
            lost_focus: false,
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        let mut text = self.updated_text.as_ref().unwrap_or(&self.props.text.text);
        let use_placeholder = text.is_empty();
        if use_placeholder {
            text = &self.props.placeholder;
        }

        let mut render = RenderTextBox::new(text.clone());
        render.style = self.props.text.style.clone();
        render.selected = self.selected;
        if !use_placeholder {
            render.cursor = self.cursor;
        }
        if use_placeholder {
            // Dim towards background
            render.style.color = colors::TEXT_INACTIVE;
        }

        pad(self.props.padding, || {
            let res = render.show();
            self.text_layout = Some(res.into_inner().layout);
        });

        Self::Response {
            text: self.updated_text.take(),
            activated: mem::take(&mut self.activated),
            lost_focus: mem::take(&mut self.lost_focus),
        }
    }

    fn layout(&self, ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        self.default_layout(ctx, constraints)
    }

    fn paint(&self, mut ctx: PaintContext<'_>) {
        let layout_node = ctx.layout.get(ctx.dom.current()).unwrap();

        if let Some(fill_color) = self.props.fill {
            let mut bg = RoundedRectangle::new(layout_node.rect, 2.0);
            bg.color = fill_color;
            bg.add(ctx.paint);
        }

        let node = ctx.dom.get_current();
        for &child in &node.children {
            ctx.paint(child);
        }

        if self.selected {
            outline(ctx.paint, layout_node.rect, 1.0, colors::BLACK);
        }
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_INSIDE | EventInterest::FOCUSED_KEYBOARD
    }

    fn event(&mut self, ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match event {
            WidgetEvent::FocusChanged(focused) => {
                self.selected = *focused;
                if !*focused {
                    self.lost_focus = true;
                }
                EventResponse::Sink
            }

            WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                inside: true,
                down,
                position,
                ..
            } => {
                if !down {
                    return EventResponse::Sink;
                }

                ctx.input.set_selection(Some(ctx.dom.current()));

                if let Some(layout) = ctx.layout.get(ctx.dom.current()) {
                    if let Some(text_layout) = &self.text_layout {
                        let text_layout = text_layout.borrow();

                        let scale_factor = ctx.layout.scale_factor();
                        let relative_pos =
                            *position - layout.rect.pos() - self.props.padding.offset();
                        let glyph_pos = relative_pos * scale_factor;

                        let Some(line) = pick_text_line(&text_layout, glyph_pos.y) else {
                            return EventResponse::Sink;
                        };

                        self.cursor = pick_character_on_line(
                            &text_layout,
                            line.glyph_start,
                            line.glyph_end,
                            glyph_pos.x,
                        );
                    }
                }

                EventResponse::Sink
            }

            WidgetEvent::KeyChanged { key, down, .. } => match key {
                KeyCode::ArrowLeft => {
                    if *down {
                        self.move_cursor(-1);
                    }
                    EventResponse::Sink
                }

                KeyCode::ArrowRight => {
                    if *down {
                        self.move_cursor(1);
                    }
                    EventResponse::Sink
                }

                KeyCode::Backspace => {
                    if *down {
                        self.delete(-1);
                    }
                    EventResponse::Sink
                }

                KeyCode::Delete => {
                    if *down {
                        self.delete(1);
                    }
                    EventResponse::Sink
                }

                KeyCode::Home => {
                    if *down {
                        self.home();
                    }
                    EventResponse::Sink
                }

                KeyCode::End => {
                    if *down {
                        self.end();
                    }
                    EventResponse::Sink
                }

                KeyCode::Enter | KeyCode::NumpadEnter => {
                    if *down {
                        ctx.input.set_selection(None);
                        self.activated = true;
                    }
                    EventResponse::Sink
                }

                KeyCode::Escape => {
                    if *down {
                        ctx.input.set_selection(None);
                    }
                    EventResponse::Sink
                }
                _ => EventResponse::Sink,
            },
            WidgetEvent::TextInput(c) => {
                if c.is_control() {
                    return EventResponse::Bubble;
                }

                let text = self
                    .updated_text
                    .get_or_insert_with(|| self.props.text.text.clone());

                // Before trying to input text, make sure that our cursor fits
                // in the string and is not in the middle of a codepoint!
                self.cursor = self.cursor.min(text.len());
                while !text.is_char_boundary(self.cursor) {
                    self.cursor = self.cursor.saturating_sub(1);
                }

                if text.is_empty() {
                    text.push(*c);
                } else {
                    text.insert(self.cursor, *c);
                }

                self.cursor += c.len_utf8();

                EventResponse::Sink
            }
            _ => EventResponse::Bubble,
        }
    }
}

impl TextBoxWidget {
    fn move_cursor(&mut self, delta: i32) {
        let text = self.updated_text.as_ref().unwrap_or(&self.props.text.text);
        let mut cursor = self.cursor as i32;
        let mut remaining = delta.abs();

        while remaining > 0 {
            cursor = cursor.saturating_add(delta.signum());
            cursor = cursor.min(self.props.text.text.len() as i32);
            cursor = cursor.max(0);
            self.cursor = cursor as usize;

            if text.is_char_boundary(self.cursor) {
                remaining -= 1;
            }
        }
    }

    fn home(&mut self) {
        self.cursor = 0;
    }

    fn end(&mut self) {
        let text = self.updated_text.as_ref().unwrap_or(&self.props.text.text);
        self.cursor = text.len();
    }

    fn delete(&mut self, dir: i32) {
        let text = self
            .updated_text
            .get_or_insert_with(|| self.props.text.text.clone());

        let anchor = self.cursor as i32;
        let mut end = anchor;
        let mut remaining = dir.abs();
        let mut len = 0;

        while remaining > 0 {
            end = end.saturating_add(dir.signum());
            end = end.min(self.props.text.text.len() as i32);
            end = end.max(0);
            len += 1;

            if text.is_char_boundary(end as usize) {
                remaining -= 1;
            }
        }

        if dir < 0 {
            self.cursor = self.cursor.saturating_sub(len);
        }

        let min = anchor.min(end) as usize;
        let max = anchor.max(end) as usize;
        text.replace_range(min..max, "");
    }
}

fn pick_text_line(layout: &Layout, pos_y: f32) -> Option<&LinePosition> {
    let lines = layout.lines()?;

    let mut closest_line = 0;
    let mut closest_line_dist = INFINITY;
    for (index, line) in lines.iter().enumerate() {
        let dist = (pos_y - line.baseline_y).abs();
        if dist < closest_line_dist {
            closest_line = index;
            closest_line_dist = dist;
        }
    }

    lines.get(closest_line)
}

fn pick_character_on_line(
    layout: &Layout,
    line_glyph_start: usize,
    line_glyph_end: usize,
    pos_x: f32,
) -> usize {
    let mut closest_byte_offset = 0;
    let mut closest_dist = INFINITY;

    let possible_positions = layout
        .glyphs()
        .iter()
        .skip(line_glyph_start)
        .take(line_glyph_end + 1 - line_glyph_start)
        .flat_map(|glyph| {
            let before = Vec2::new(glyph.x, glyph.y);
            let after = Vec2::new(glyph.x + glyph.width as f32, glyph.y);
            [
                (glyph.byte_offset, before),
                (glyph.byte_offset + glyph.parent.len_utf8(), after),
            ]
        });

    for (byte_offset, glyph_pos) in possible_positions {
        let dist = (pos_x - glyph_pos.x).abs();
        if dist < closest_dist {
            closest_byte_offset = byte_offset;
            closest_dist = dist;
        }
    }

    closest_byte_offset
}

pub fn textbox(text: &mut String, placeholder: &str) -> Response<TextBoxResponse> {
    let mut res = TextBox::new(label_text(text.as_str()), placeholder.to_string()).show();

    if let Some(new) = res.text.take() {
        text.clone_from(&new)
    }

    res
}
