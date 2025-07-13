use std::{cell::Cell, fmt::Debug};

use yakui::{
    Alignment, Constraints, Dim2, Flow, Response,
    event::{EventInterest, EventResponse, WidgetEvent},
    geometry::Vec2,
    input::MouseButton,
    layout::{AbstractClipRect, ClipLogic},
    util::widget_children,
    widget::{EventContext, Widget},
};

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Movable {
    pos: Vec2,
}

impl Movable {
    fn new(pos: Vec2) -> Self {
        Movable { pos }
    }

    #[track_caller]
    fn show<F: FnOnce()>(self, children: F) -> Response<MovableResponse> {
        widget_children::<MovableWidget, F>(children, self)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct MovableResponse {
    pub pos: Vec2,
}

#[derive(Debug)]
pub struct MovableWidget {
    drag_mouse_start: Cell<Option<Vec2>>,
    drag_start: Cell<Option<Vec2>>,
    pos: Cell<Vec2>,
    size: Cell<Vec2>,
}

impl Widget for MovableWidget {
    type Props<'a> = Movable;
    type Response = MovableResponse;

    fn new() -> Self {
        Self {
            drag_mouse_start: Cell::default(),
            drag_start: Cell::default(),
            pos: Cell::default(),
            size: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.pos.set(props.pos);

        MovableResponse {
            pos: self.pos.get(),
        }
    }

    fn flow(&self) -> Flow {
        Flow::Relative {
            anchor: Alignment::new(self.pos.get().x, self.pos.get().y),
            offset: Dim2::ZERO,
        }
    }

    fn layout(
        &self,
        mut ctx: yakui::widget::LayoutContext<'_>,
        _constraints: yakui::Constraints,
    ) -> Vec2 {
        ctx.layout.set_clip_logic(
            ctx.dom,
            ClipLogic::Contain {
                it: AbstractClipRect::LayoutRect,
                parent: AbstractClipRect::ParentClip,
            },
        );
        let node = ctx.dom.get_current();

        let mut size = Vec2::ZERO;
        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, Constraints::none());
            size = size.max(child_size);
        }

        self.size.set(size);

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
                    self.drag_mouse_start.set(Some(position));
                    self.drag_start.set(None);

                    EventResponse::Sink
                } else {
                    self.drag_mouse_start.set(None);
                    self.drag_start.set(None);

                    EventResponse::Bubble
                }
            }
            WidgetEvent::MouseMoved(Some(mouse_current)) => {
                if let Some(mouse_start) = self.drag_mouse_start.get() {
                    let pos = self.pos.get_mut();
                    let viewport = ctx.layout.viewport();

                    if self.drag_start.get().is_none() {
                        self.drag_start.set(Some(*pos));
                    }
                    let drag_start = self.drag_start.get().unwrap();

                    let delta = (mouse_current - mouse_start) / viewport.size();

                    self.pos.set(drag_start + delta);
                } else {
                    self.drag_start.set(None);
                }

                EventResponse::Bubble
            }
            _ => EventResponse::Bubble,
        }
    }
}

pub fn movable(position: &mut Vec2, children: impl FnOnce()) -> Response<MovableResponse> {
    let r = Movable::new(*position).show(children);

    *position = r.pos;

    r
}
