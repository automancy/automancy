use automancy_defs::colors;
use yakui::{
    colored_box_container, pad,
    shapes::RoundedRectangle,
    util::{widget, widget_children},
    widgets::Pad,
};

use yakui::geometry::{Color, Constraints, Vec2};
use yakui::widget::{LayoutContext, PaintContext, Widget};
use yakui::Response;

use super::PADDING_MEDIUM;

/**
A colored box with rounded corners that can contain children.

Responds with [RoundRectResponse].
*/
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RoundRect {
    pub radius: f32,
    pub color: Color,
    pub min_size: Vec2,
}

impl RoundRect {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            color: Color::WHITE,
            min_size: Vec2::ZERO,
        }
    }

    pub fn show(self) -> Response<RoundRectResponse> {
        widget::<RoundRectWidget>(self)
    }

    pub fn show_children<F: FnOnce()>(self, children: F) -> Response<RoundRectResponse> {
        widget_children::<RoundRectWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct RoundRectWidget {
    props: RoundRect,
}

pub type RoundRectResponse = ();

impl Widget for RoundRectWidget {
    type Props<'a> = RoundRect;
    type Response = RoundRectResponse;

    fn new() -> Self {
        Self {
            props: RoundRect::new(0.0),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, input: Constraints) -> Vec2 {
        ctx.layout.enable_clipping(ctx.dom);

        let node = ctx.dom.get_current();
        let mut size = self.props.min_size;

        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, input);
            size = size.max(child_size);
        }

        input.constrain_min(size)
    }

    fn paint(&self, mut ctx: PaintContext<'_>) {
        let node = ctx.dom.get_current();
        let layout_node = ctx.layout.get(ctx.dom.current()).unwrap();

        let mut rect = RoundedRectangle::new(layout_node.rect, self.props.radius);
        rect.color = self.props.color;
        rect.add(ctx.paint);

        for &child in &node.children {
            ctx.paint(child);
        }
    }
}

pub fn group(children: impl FnOnce()) {
    colored_box_container(colors::GRAY, || {
        pad(Pad::all(2.0), || {
            colored_box_container(colors::BACKGROUND_1, || {
                Pad::all(PADDING_MEDIUM).show(children);
            });
        });
    });
}
