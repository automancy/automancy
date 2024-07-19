use std::cell::Cell;

use automancy_defs::{
    colors,
    glam::{vec2, Vec2Swizzles},
};
use yakui::geometry::{Constraints, Vec2};
use yakui::{
    constrained,
    input::MouseButton,
    reflow,
    widget::{EventContext, LayoutContext, Widget},
    Alignment, Dim2, Rect,
};
use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    util::widget_children,
};
use yakui::{Pivot, Response};

use crate::gui::{
    shapes::PaintRectLerpedColor,
    util::{pad_x, pad_y},
};

use super::{col, container::RoundRect, row};

const SCROLL_SIZE: f32 = 8.0;
const SCROLL_RADIUS: f32 = 4.0;

#[derive(Debug)]
#[non_exhaustive]
pub struct Scrollable {
    pub direction: Option<ScrollDirection>,
    pub min: Vec2,
    pub max: f32,
}

impl Scrollable {
    pub fn none() -> Self {
        Scrollable {
            direction: None,
            min: Vec2::default(),
            max: 0.0,
        }
    }

    fn vertical(min: Vec2, max: f32) -> Self {
        Scrollable {
            direction: Some(ScrollDirection::Y),
            min,
            max,
        }
    }

    fn horizontal(min: Vec2, max: f32) -> Self {
        Scrollable {
            direction: Some(ScrollDirection::X),
            min,
            max,
        }
    }

    #[track_caller]
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
                    pos_percentage: if canvas_size == size {
                        0.0
                    } else {
                        self.scroll_position.get().y / (canvas_size - size)
                    },
                    canvas_size,
                    size,
                }
            }
            Some(ScrollDirection::X) => {
                let canvas_size = self.canvas_size.get().x;
                let size = self.size.get().x;

                ScrollableResponse {
                    pos_percentage: if canvas_size == size {
                        0.0
                    } else {
                        self.scroll_position.get().x / (canvas_size - size)
                    },
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
                min: constraints.min.max(self.props.min),
                max: Vec2::new(self.props.max.min(constraints.max.x), f32::INFINITY),
            },
            Some(ScrollDirection::X) => Constraints {
                min: constraints.min.max(self.props.min),
                max: Vec2::new(f32::INFINITY, self.props.max.min(constraints.max.y)),
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

    fn paint(&self, mut ctx: yakui::widget::PaintContext<'_>) {
        let clip = ctx.paint.get_current_clip().unwrap_or(Rect::ZERO);

        let node = ctx.dom.get_current();
        for &child in &node.children {
            ctx.paint(child);
        }

        let Some(dir) = self.props.direction else {
            return;
        };

        let layout_pos = ctx.layout.get(ctx.dom.current()).unwrap().rect.pos();

        let scroll_min = Vec2::ZERO;
        let scroll_max = self.canvas_size.get();

        let canvas_min = self.scroll_position.get();
        let canvas_max = canvas_min + self.size.get();

        let min = ((canvas_min - scroll_min).abs() / 30.0).clamp(Vec2::ZERO, Vec2::ONE);
        let max = ((canvas_max - scroll_max).abs() / 30.0).clamp(Vec2::ZERO, Vec2::ONE);

        const SIZE: f32 = 20.0;
        let dir_size = match dir {
            ScrollDirection::Y => Vec2::new(scroll_max.x, SIZE),
            ScrollDirection::X => Vec2::new(SIZE, scroll_max.y),
        };

        let a = Rect::from_pos_size(layout_pos, dir_size);
        let b = Rect::from_pos_size(layout_pos + self.size.get() - dir_size, dir_size);

        const MUL: f32 = 0.5;

        {
            let mut rect = PaintRectLerpedColor::new(match dir {
                ScrollDirection::Y => a,
                ScrollDirection::X => b,
            });

            let size = clip.constrain(rect.rect).size();
            if size.x >= SIZE && size.y >= SIZE {
                let x0 = colors::BLACK.with_alpha(max.x * MUL);
                let x1 = colors::TRANSPARENT;
                let y0 = colors::BLACK.with_alpha(min.y * MUL);
                let y1 = colors::TRANSPARENT;

                rect.color = (x0, y0, x1, y1);
                rect.add(ctx.paint);
            }
        }

        {
            let mut rect = PaintRectLerpedColor::new(match dir {
                ScrollDirection::Y => b,
                ScrollDirection::X => a,
            });

            let size = clip.constrain(rect.rect).size();
            if size.x >= SIZE && size.y >= SIZE {
                let x0 = colors::BLACK.with_alpha(min.x * MUL);
                let x1 = colors::TRANSPARENT;
                let y0 = colors::BLACK.with_alpha(max.y * MUL);
                let y1 = colors::TRANSPARENT;

                rect.color = (x1, y1, x0, y0);
                rect.add(ctx.paint);
            }
        }
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
                    self.dragging = down;

                    if down {
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

                let pos = self.scroll_position.get();
                let pos = pos + delta;

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

#[track_caller]
pub fn scroll_vertical_bar_alignment(
    min: Vec2,
    max: Vec2,
    alignment: Option<Alignment>,
    children: impl FnOnce(),
) {
    row(|| {
        let mut res = None;
        constrained(Constraints::loose(Vec2::new(f32::INFINITY, max.y)), || {
            res = Some(Scrollable::vertical(min, max.x).show(children));
        });
        let res = res.unwrap();

        if let Some(alignment) = alignment {
            scroll_bar(res.into_inner(), alignment.as_vec2(), ScrollDirection::Y);
        }
    });
}

#[track_caller]
pub fn scroll_vertical(min: Vec2, max: Vec2, children: impl FnOnce()) {
    scroll_vertical_bar_alignment(min, max, Some(Alignment::TOP_RIGHT), children)
}

#[track_caller]
pub fn scroll_horizontal_bar_alignment(
    min: Vec2,
    max: Vec2,
    alignment: Option<Alignment>,
    children: impl FnOnce(),
) {
    col(|| {
        let mut res = None;
        constrained(Constraints::loose(Vec2::new(max.x, f32::INFINITY)), || {
            res = Some(Scrollable::horizontal(min, max.y).show(children));
        });
        let res = res.unwrap();

        if let Some(alignment) = alignment {
            scroll_bar(res.into_inner(), alignment.as_vec2(), ScrollDirection::X);
        }
    });
}

#[track_caller]
pub fn scroll_horizontal(min: Vec2, max: Vec2, children: impl FnOnce()) {
    scroll_horizontal_bar_alignment(min, max, Some(Alignment::BOTTOM_LEFT), children)
}

#[track_caller]
fn scroll_bar(res: ScrollableResponse, alignment: Vec2, dir: ScrollDirection) {
    let ratio = res.size / res.canvas_size;
    let diff = res.canvas_size - res.size;

    reflow(
        Alignment::new(alignment.x, alignment.y),
        Pivot::new(alignment.x, alignment.y),
        Dim2::ZERO,
        || {
            RoundRect::new(SCROLL_RADIUS, colors::BACKGROUND_3).show_children(|| {
                let pad_f = if dir == ScrollDirection::Y {
                    pad_y
                } else {
                    pad_x
                };

                pad_f(
                    diff * ratio * res.pos_percentage,
                    diff * ratio * (1.0 - res.pos_percentage),
                )
                .show(|| {
                    let mut rect = if dir == ScrollDirection::Y {
                        RoundRect::colored_y
                    } else {
                        RoundRect::colored_x
                    }(
                        SCROLL_RADIUS,
                        (colors::ORANGE, colors::ORANGE.adjust(1.0 + (1.0 - ratio))),
                    );

                    rect.min_size = if dir == ScrollDirection::Y {
                        vec2(SCROLL_SIZE, (res.size * ratio).floor())
                    } else {
                        vec2((res.size * ratio).floor(), SCROLL_SIZE)
                    };
                    rect.show();
                });
            });
        },
    );
}
