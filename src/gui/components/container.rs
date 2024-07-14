use automancy_defs::colors;
use yakui::{
    align, colored_box_container,
    util::{widget, widget_children},
    widgets::{Layer, Pad},
    Alignment,
};

use yakui::geometry::{Color, Constraints, Vec2};
use yakui::widget::{LayoutContext, PaintContext, Widget};
use yakui::Response;

use crate::gui::{heading, shapes::RoundedRectLerpedColor, util::pad_y, ROUNDED_MEDIUM};

use super::{center_col, col, PADDING_LARGE, PADDING_MEDIUM};

/**
A colored box with rounded corners that can contain children.

Responds with [RoundRectResponse].
*/
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RoundRect {
    pub radius: f32,
    pub color: (Color, Color, Color, Color),
    pub min_size: Vec2,
}

impl RoundRect {
    pub fn new(radius: f32, color: Color) -> Self {
        Self {
            radius,
            color: (color, color, color, color),
            min_size: Vec2::ZERO,
        }
    }

    pub fn colored_xy(radius: f32, color: (Color, Color, Color, Color)) -> Self {
        Self {
            radius,
            color,
            min_size: Vec2::ZERO,
        }
    }

    pub fn colored_x(radius: f32, (x0, x1): (Color, Color)) -> Self {
        Self {
            radius,
            color: (x0, x1, x0, x1),
            min_size: Vec2::ZERO,
        }
    }

    pub fn colored_y(radius: f32, (y0, y1): (Color, Color)) -> Self {
        Self {
            radius,
            color: (y0, y0, y1, y1),
            min_size: Vec2::ZERO,
        }
    }

    #[track_caller]
    pub fn show(self) -> Response<RoundRectResponse> {
        widget::<RoundRectWidget>(self)
    }

    #[track_caller]
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

        let mut rect = RoundedRectLerpedColor::new(layout_node.rect, self.props.radius);
        rect.color = self.props.color;
        rect.add(ctx.paint);

        for &child in &node.children {
            ctx.paint(child);
        }
    }
}

pub fn group(children: impl FnOnce()) {
    colored_box_container(colors::BACKGROUND_3, || {
        Pad::all(2.0).show(|| {
            colored_box_container(colors::BACKGROUND_1, || {
                Pad::all(PADDING_MEDIUM).show(|| {
                    col(children);
                });
            });
        });
    });
}

pub fn window_box(title: String, children: impl FnOnce()) {
    RoundRect::new(ROUNDED_MEDIUM, colors::BACKGROUND_1).show_children(|| {
        Pad::all(PADDING_LARGE).show(|| {
            center_col(|| {
                pad_y(0.0, PADDING_MEDIUM).show(|| {
                    heading(&title);
                });

                children();
            });
        });
    });
}

pub fn window(title: String, children: impl FnOnce()) {
    Layer::new().show(|| {
        align(Alignment::CENTER, || {
            window_box(title, children);
        });
    });
}
