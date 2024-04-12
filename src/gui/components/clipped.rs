use std::cell::Cell;

use automancy_defs::{log, math::Float};
use yakui::geometry::{Constraints, Vec2};
use yakui::widget::{EventContext, LayoutContext, Widget};
use yakui::Response;
use yakui::{
    event::{EventInterest, EventResponse, WidgetEvent},
    util::widget_children,
};

#[derive(Debug)]
#[non_exhaustive]
pub struct Clipped {}

impl Clipped {
    pub fn new() -> Self {
        Clipped {}
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<ClippedResponse> {
        widget_children::<ClippedWidget, F>(children, self)
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ClippedWidget {
    props: Clipped,
}

pub type ClippedResponse = ();

impl Widget for ClippedWidget {
    type Props<'a> = Clipped;
    type Response = ClippedResponse;

    fn new() -> Self {
        Self {
            props: Clipped::new(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        ctx.layout.enable_clipping(ctx.dom);
        self.default_layout(ctx, constraints)
    }
}

pub fn clipped(children: impl FnOnce()) -> Response<()> {
    Clipped::new().show(children)
}
