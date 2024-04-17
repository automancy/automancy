use std::cell::Cell;

use automancy_defs::math::Float;
use yakui::geometry::{Constraints, Vec2};
use yakui::widget::{EventContext, LayoutContext, Widget};
use yakui::Response;
use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    util::widget_children,
};

#[derive(Debug)]
#[non_exhaustive]
pub struct Scrollable {
    pub direction: Option<ScrollDirection>,
    pub max_size: Float,
}

impl Scrollable {
    pub fn none() -> Self {
        Scrollable {
            direction: None,
            max_size: 0.0,
        }
    }

    fn vertical(max_size: Float) -> Self {
        Scrollable {
            direction: Some(ScrollDirection::Y),
            max_size,
        }
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<ScrollableResponse> {
        widget_children::<ScrollableWidget, F>(children, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Y,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ScrollableWidget {
    props: Scrollable,
    scroll_position: Cell<Vec2>,
}

pub type ScrollableResponse = ();

impl Widget for ScrollableWidget {
    type Props<'a> = Scrollable;
    type Response = ScrollableResponse;

    fn new() -> Self {
        Self {
            props: Scrollable::none(),
            scroll_position: Cell::new(Vec2::ZERO),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        ctx.layout.enable_clipping(ctx.dom);

        let node = ctx.dom.get_current();
        let mut canvas_size = Vec2::ZERO;

        let child_constraints = match self.props.direction {
            None => constraints,
            Some(ScrollDirection::Y) => Constraints {
                min: Vec2::new(constraints.min.x, 0.0),
                max: Vec2::new(constraints.max.x, f32::INFINITY),
            },
        };

        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, child_constraints);
            canvas_size = canvas_size.max(child_size);
        }

        let size = constraints
            .constrain(canvas_size)
            .min(match self.props.direction {
                Some(ScrollDirection::Y) => Vec2::new(f32::INFINITY, self.props.max_size),
                None => Vec2::INFINITY,
            });

        let max_scroll_position = (canvas_size - size).max(Vec2::ZERO);
        let mut scroll_position = self
            .scroll_position
            .get()
            .min(max_scroll_position)
            .max(Vec2::ZERO);

        match self.props.direction {
            None => scroll_position = Vec2::ZERO,
            Some(ScrollDirection::Y) => scroll_position.x = 0.0,
        }

        self.scroll_position.set(scroll_position);

        for &child in &node.children {
            ctx.layout.set_pos(child, -scroll_position);
        }

        size
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_INSIDE
    }

    fn event(&mut self, _ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match *event {
            WidgetEvent::MouseScroll { delta } => {
                let pos = self.scroll_position.get();
                self.scroll_position.set(pos + delta);
                EventResponse::Sink
            }
            _ => EventResponse::Bubble,
        }
    }
}

pub fn scroll_vertical(max_height: Float, children: impl FnOnce()) {
    Scrollable::vertical(max_height).show(children);
}
