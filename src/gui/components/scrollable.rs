use std::cell::Cell;

use automancy_defs::{
    colors,
    glam::{vec2, Vec2Swizzles},
    math::Float,
};
use yakui::{
    column, constrained,
    input::MouseButton,
    widget::{EventContext, LayoutContext, Widget},
    Alignment, Dim2,
};
use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    util::widget_children,
};
use yakui::{
    geometry::{Constraints, Vec2},
    row,
};
use yakui::{Pivot, Response};

use crate::gui::util::{pad_x, pad_y};

use super::{container::RoundRect, relative::Relative};

const SCROLL_SIZE: f32 = 8.0;
const SCROLL_RADIUS: f32 = 4.0;

#[derive(Debug)]
#[non_exhaustive]
pub struct Scrollable {
    pub direction: Option<ScrollDirection>,
}

impl Scrollable {
    pub fn none() -> Self {
        Scrollable { direction: None }
    }

    fn vertical() -> Self {
        Scrollable {
            direction: Some(ScrollDirection::Y),
        }
    }

    fn horizontal() -> Self {
        Scrollable {
            direction: Some(ScrollDirection::X),
        }
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<ScrollableResponse> {
        widget_children::<ScrollableWidget, F>(children, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Y,
    X,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ScrollableWidget {
    props: Scrollable,
    dragging: bool,
    last_drag_pos: Option<Vec2>,
    scroll_position: Cell<Vec2>,
    canvas_size: Cell<Vec2>,
    size: Cell<Vec2>,
}

#[derive(Debug, Default)]

pub struct ScrollableResponse {
    pub pos_percentage: f32,
    pub canvas_size: f32,
    pub size: f32,
}

impl Widget for ScrollableWidget {
    type Props<'a> = Scrollable;
    type Response = ScrollableResponse;

    fn new() -> Self {
        Self {
            props: Scrollable::none(),
            dragging: false,
            last_drag_pos: None,
            scroll_position: Cell::new(Vec2::ZERO),
            canvas_size: Cell::new(Vec2::ZERO),
            size: Cell::new(Vec2::ZERO),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        match self.props.direction {
            Some(ScrollDirection::Y) => {
                let canvas_size = self.canvas_size.get().y;
                let size = self.size.get().y;

                ScrollableResponse {
                    pos_percentage: self.scroll_position.get().y / (canvas_size - size),
                    canvas_size,
                    size,
                }
            }
            Some(ScrollDirection::X) => {
                let canvas_size = self.canvas_size.get().x;
                let size = self.size.get().x;

                ScrollableResponse {
                    pos_percentage: self.scroll_position.get().x / (canvas_size - size),
                    canvas_size,
                    size,
                }
            }
            _ => ScrollableResponse::default(),
        }
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        ctx.layout.enable_clipping(ctx.dom);

        let node = ctx.dom.get_current();
        let mut canvas_size = Vec2::ZERO;

        let child_constraints = match self.props.direction {
            Some(ScrollDirection::Y) => Constraints {
                min: Vec2::new(constraints.min.x, 0.0),
                max: Vec2::new(constraints.max.x, f32::INFINITY),
            },
            Some(ScrollDirection::X) => Constraints {
                min: Vec2::new(0.0, constraints.min.y),
                max: Vec2::new(f32::INFINITY, constraints.max.y),
            },
            None => constraints,
        };

        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, child_constraints);
            canvas_size = canvas_size.max(child_size);
        }

        let size = constraints.constrain(canvas_size);

        self.canvas_size.set(canvas_size);
        self.size.set(size);

        let mut pos = self.scroll_position.get();

        match self.props.direction {
            None => self.scroll_position.set(Vec2::ZERO),
            Some(ScrollDirection::Y) => pos.x = 0.0,
            Some(ScrollDirection::X) => pos.y = 0.0,
        }

        let max_scroll_position = (canvas_size - size).max(Vec2::ZERO);
        pos = pos.min(max_scroll_position).max(Vec2::ZERO);

        for &child in &node.children {
            ctx.layout.set_pos(child, -pos);
        }

        self.scroll_position.set(pos);

        size
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_INSIDE | EventInterest::MOUSE_MOVE | EventInterest::MOUSE_OUTSIDE
    }

    fn event(&mut self, _ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match *event {
            WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                down,
                inside,
                ..
            } => {
                self.last_drag_pos = None;

                if inside {
                    let different = self.dragging != down;

                    self.dragging = down;

                    if different {
                        EventResponse::Sink
                    } else {
                        EventResponse::Bubble
                    }
                } else {
                    self.dragging = false;

                    EventResponse::Bubble
                }
            }
            WidgetEvent::MouseMoved(Some(mouse)) => {
                if self.dragging {
                    if let Some(start) = self.last_drag_pos {
                        let pos = self.scroll_position.get();
                        let pos = pos - (mouse - start);

                        let max_scroll_position =
                            (self.canvas_size.get() - self.size.get()).max(Vec2::ZERO);

                        let pos = pos.min(max_scroll_position).max(Vec2::ZERO);

                        self.scroll_position.set(pos);
                    }

                    self.last_drag_pos = Some(mouse);

                    EventResponse::Sink
                } else {
                    EventResponse::Bubble
                }
            }
            WidgetEvent::MouseScroll {
                mut delta,
                modifiers,
            } => {
                if modifiers.shift() {
                    delta = delta.yx();
                }

                let pos = self.scroll_position.get();
                let pos = pos + delta;

                match self.props.direction {
                    Some(ScrollDirection::Y) => {
                        if delta.y.abs() < 0.01 {
                            return EventResponse::Bubble;
                        }
                    }
                    Some(ScrollDirection::X) => {
                        if delta.x.abs() < 0.01 {
                            return EventResponse::Bubble;
                        }
                    }
                    None => {}
                }

                let max_scroll_position =
                    (self.canvas_size.get() - self.size.get()).max(Vec2::ZERO);

                let pos = pos.min(max_scroll_position).max(Vec2::ZERO);

                self.scroll_position.set(pos);

                EventResponse::Sink
            }
            WidgetEvent::MouseLeave => {
                self.dragging = false;
                self.last_drag_pos = None;
                EventResponse::Sink
            }
            _ => EventResponse::Bubble,
        }
    }
}

pub fn scroll_vertical(max_height: Float, children: impl FnOnce()) {
    row(|| {
        let mut res = None;
        constrained(Constraints::loose(vec2(f32::INFINITY, max_height)), || {
            res = Some(Scrollable::vertical().show(|| {
                pad_x(0.0, SCROLL_SIZE).show(children);
            }));
        });
        let res = res.unwrap();

        let ratio = res.size / res.canvas_size;
        let diff = res.canvas_size - res.size;

        if diff > 0.0 {
            pad_x(SCROLL_SIZE, 0.0).show(|| {
                Relative::new(Alignment::TOP_RIGHT, Pivot::TOP_RIGHT, Dim2::ZERO).show(|| {
                    RoundRect::new(SCROLL_RADIUS, colors::WHITE).show_children(|| {
                        pad_y(
                            diff * ratio * res.pos_percentage,
                            diff * ratio * (1.0 - res.pos_percentage),
                        )
                        .show(|| {
                            let mut rect = RoundRect::colored_y(
                                SCROLL_RADIUS,
                                (colors::ORANGE, colors::ORANGE.adjust(1.0 + (1.0 - ratio))),
                            );
                            rect.min_size = vec2(SCROLL_SIZE, (res.size * ratio).floor());
                            rect.show();
                        });
                    });
                });
            });
        }
    });
}

pub fn scroll_horizontal(max_width: Float, children: impl FnOnce()) {
    column(|| {
        let mut res = None;
        constrained(Constraints::loose(vec2(max_width, f32::INFINITY)), || {
            res = Some(Scrollable::horizontal().show(|| {
                pad_y(0.0, SCROLL_SIZE).show(children);
            }));
        });
        let res = res.unwrap();

        let ratio = res.size / res.canvas_size;
        let diff = res.canvas_size - res.size;

        if diff > 0.0 {
            pad_y(SCROLL_SIZE, 0.0).show(|| {
                Relative::new(Alignment::BOTTOM_LEFT, Pivot::BOTTOM_LEFT, Dim2::ZERO).show(|| {
                    RoundRect::new(SCROLL_RADIUS, colors::WHITE).show_children(|| {
                        pad_x(
                            diff * ratio * res.pos_percentage,
                            diff * ratio * (1.0 - res.pos_percentage),
                        )
                        .show(|| {
                            let mut rect = RoundRect::colored_x(
                                SCROLL_RADIUS,
                                (colors::ORANGE, colors::ORANGE.adjust(1.0 + ratio)),
                            );
                            rect.min_size = vec2((res.size * ratio).floor(), SCROLL_SIZE);
                            rect.show();
                        });
                    });
                });
            });
        }
    });
}
