use automancy_defs::colors;
use yakui::{
    colored_box_container, pad, row,
    shapes::RoundedRectangle,
    util::{widget, widget_children},
    widgets::{Layer, Pad},
};

use yakui::geometry::{Color, Constraints, Vec2};
use yakui::widget::{LayoutContext, PaintContext, Widget};
use yakui::Response;

use super::{layout::centered_column, text::heading, PADDING_LARGE, PADDING_MEDIUM, PADDING_SMALL};

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
    pub fn new(radius: f32, color: Color) -> Self {
        Self {
            radius,
            color,
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
            props: RoundRect::new(0.0, colors::WHITE),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, input: Constraints) -> Vec2 {
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
    colored_box_container(colors::BACKGROUND_3, || {
        pad(Pad::all(2.0), || {
            colored_box_container(colors::BACKGROUND_1, || {
                Pad::all(PADDING_MEDIUM).show(children);
            });
        });
    });
}

pub fn window_box(title: String, children: impl FnOnce()) {
    RoundRect::new(4.0, colors::BACKGROUND_1).show_children(|| {
        Pad::all(PADDING_LARGE).show(|| {
            centered_column(|| {
                Pad::vertical(PADDING_SMALL).show(|| {
                    row(|| {
                        heading(&title);
                    });
                });

                children()
            });
        });
    });
}

pub fn window(title: String, children: impl FnOnce()) {
    Layer::new().show(|| {
        centered_column(|| {
            window_box(title, children);
        });
    });
}
