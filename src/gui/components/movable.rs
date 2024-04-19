use std::cell::Cell;

use yakui::Response;
use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    util::widget_children,
};
use yakui::{geometry::Vec2, Constraints};
use yakui::{input::MouseButton, Rect};
use yakui::{
    widget::{EventContext, Widget},
    Flow,
};

use crate::gui::util::constrain_to_viewport;

#[derive(Debug)]
#[non_exhaustive]
pub struct Movable {}

impl Movable {
    fn new() -> Self {
        Movable {}
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
    dragging_start: Cell<Option<Vec2>>,
    dragging_from: Cell<Option<Vec2>>,
    position: Cell<Vec2>,
    size: Cell<Vec2>,
}

impl Widget for MovableWidget {
    type Props<'a> = Movable;
    type Response = MovableResponse;

    fn new() -> Self {
        Self {
            dragging_start: Cell::default(),
            dragging_from: Cell::default(),
            position: Cell::default(),
            size: Cell::default(),
        }
    }

    fn update(&mut self, _props: Self::Props<'_>) -> Self::Response {
        MovableResponse {
            position: self.position.get(),
        }
    }

    fn flow(&self) -> Flow {
        Flow::Absolute
    }

    fn layout(
        &self,
        mut ctx: yakui::widget::LayoutContext<'_>,
        constraints: yakui::Constraints,
    ) -> Vec2 {
        let node = ctx.dom.get_current();

        let mut size = Vec2::ZERO;
        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, Constraints::none());
            size = size.max(child_size);
        }

        let pos = (self.position.get() * ctx.layout.viewport().size()).floor();

        for &child in &node.children {
            ctx.layout.set_pos(child, pos);
        }

        self.size.set(size);

        constraints.constrain_min(size)
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
                if let Some(start) = self.dragging_start.get() {
                    if self.dragging_from.get().is_none() {
                        self.dragging_from.set(Some(self.position.get()));
                    }

                    let viewport = ctx.layout.viewport().size();

                    let mut rect = Rect::from_pos_size(
                        (self.dragging_from.get().unwrap() * viewport).floor() + (position - start),
                        self.size.get(),
                    );

                    constrain_to_viewport(&mut rect, ctx.layout);

                    self.position
                        .set((rect.pos() / viewport).clamp(Vec2::ZERO, Vec2::ONE));
                } else {
                    self.dragging_from.set(None);
                }

                EventResponse::Bubble
            }
            _ => EventResponse::Bubble,
        }
    }
}

pub fn movable(children: impl FnOnce()) -> Response<MovableResponse> {
    Movable::new().show(children)
}
