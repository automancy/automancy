use yakui::TextureId;

use crate::*;

/**
A colored box with rounded corners that can contain children.

Responds with [RoundRectResponse].
*/
#[derive(Debug, Clone)]
pub struct RoundRect {
    pub radius: BorderRadius,
    pub min_size: Vec2,
    /// component order: (x0, x1, y0, y1)
    ///
    /// blended like this:
    /// ```txt
    /// x0－x1
    /// ｜　｜
    /// y0－y1
    /// ```
    pub color: (Color, Color, Color, Color),
    pub texture: Option<(TextureId, Rect)>,
    pub border: Option<Border>,
}

auto_builders!(RoundRect {
    min_size: Vec2,
    texture: Option<(TextureId, Rect)>,
    border: Option<Border>,
});

impl RoundRect {
    pub fn new<T: Into<BorderRadius>>(radius: T) -> Self {
        Self {
            radius: radius.into(),
            color: (Color::WHITE, Color::WHITE, Color::WHITE, Color::WHITE),
            min_size: Vec2::ZERO,
            texture: None,
            border: None,
        }
    }

    pub fn color_xy(self, color: (Color, Color, Color, Color)) -> Self {
        Self {
            color,
            ..self
        }
    }

    pub fn color_x(self, (x0, x1): (Color, Color)) -> Self {
        Self {
            color: (x0, x1, x0, x1),
            ..self
        }
    }

    pub fn color_y(self, (y0, y1): (Color, Color)) -> Self {
        Self {
            color: (y0, y0, y1, y1),
            ..self
        }
    }

    pub fn color(self, color: Color) -> Self {
        Self {
            color: (color, color, color, color),
            ..self
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
            props: RoundRect::new(BorderRadius::default()).color(Color::WHITE),
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

        shapes::PaintRoundRect::new(layout_node.rect, self.props.radius)
            .border(self.props.border)
            .texture(self.props.texture)
            .color_xy(self.props.color)
            .add(ctx.paint);

        for &child in &node.children {
            ctx.paint(child);
        }
    }
}
