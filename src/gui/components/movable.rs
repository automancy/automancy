use std::{cell::Cell, fmt::Debug};

use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    util::widget_children,
    Alignment,
};
use yakui::{geometry::Vec2, Constraints};
use yakui::{input::MouseButton, Rect};
use yakui::{
    widget::{EventContext, Widget},
    Flow,
};
use yakui::{Dim2, Response};

use crate::gui::util::constrain_to_viewport;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Movable {
    position: Vec2,
}

impl Movable {
    fn new(position: Vec2) -> Self {
        Movable { position }
    }

    fn show<F: FnOnce()>(self, children: F) -> Response<MovableResponse> {
        widget_children::<MovableWidget, F>(children, self)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct MovableResponse {
    pub position: Vec2,
}

#[derive(Debug)]
pub struct MovableWidget {
    props: Cell<Option<Movable>>,
    dragging_start: Cell<Option<Vec2>>,
    dragging_from: Cell<Option<Vec2>>,
    size: Cell<Vec2>,
}

impl MovableWidget {
    fn pos(&self) -> Vec2 {
        self.props.get().map(|v| v.position).unwrap_or(Vec2::ZERO)
    }
}

fn clamp_pos(size: Vec2, mut props: Movable, viewport: Rect) -> Movable {
    let mut rect = Rect::from_pos_size((props.position * viewport.size()).floor(), size);

    constrain_to_viewport(&mut rect, viewport);

    props.position = (rect.pos() / viewport.size()).clamp(Vec2::ZERO, Vec2::ONE);

    props
}

impl Widget for MovableWidget {
    type Props<'a> = Movable;
    type Response = MovableResponse;

    fn new() -> Self {
        Self {
            props: Cell::default(),
            dragging_start: Cell::default(),
            dragging_from: Cell::default(),
            size: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        if self.props.get().is_none() {
            self.props.set(Some(props));
        }

        MovableResponse {
            position: self.pos(),
        }
    }

    fn flow(&self) -> Flow {
        Flow::Absolute {
            anchor: Alignment::new(self.pos().x, self.pos().y),
            offset: Dim2::ZERO,
        }
    }

    fn layout(
        &self,
        mut ctx: yakui::widget::LayoutContext<'_>,
        _constraints: yakui::Constraints,
    ) -> Vec2 {
        let node = ctx.dom.get_current();

        let mut size = Vec2::ZERO;
        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, Constraints::none());
            size = size.max(child_size);
        }

        self.size.set(size);

        if let Some(props) = self.props.get() {
            self.props
                .set(Some(clamp_pos(size, props, ctx.layout.viewport())));
        }

        size
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_ALL
    }

    fn event(&mut self, ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match *event {
            WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                down,
                inside,
                position,
                ..
            } => {
                if inside && down {
                    self.dragging_start.set(Some(position));

                    EventResponse::Sink
                } else if !down {
                    self.dragging_start.set(None);

                    EventResponse::Bubble
                } else {
                    EventResponse::Bubble
                }
            }
            WidgetEvent::MouseMoved(Some(position)) => {
                if let Some((start, props)) =
                    self.dragging_start.get().zip(self.props.get_mut().as_mut())
                {
                    let viewport = ctx.layout.viewport();

                    if self.dragging_from.get().is_none() {
                        self.dragging_from.set(Some(props.position));
                    }

                    let p = (self.dragging_from.get().unwrap() * viewport.size()).floor()
                        + (position - start);
                    props.position = p / viewport.size();

                    *props = clamp_pos(self.size.get(), *props, viewport);
                } else {
                    self.dragging_from.set(None);
                }

                EventResponse::Bubble
            }
            _ => EventResponse::Bubble,
        }
    }
}

pub fn movable(position: &mut Vec2, children: impl FnOnce()) -> Response<MovableResponse> {
    let r = Movable::new(*position).show(children);

    *position = r.position;

    r
}
