use yakui::{
    util::widget_children,
    widget::{LayoutContext, Widget},
    Alignment, Constraints, Pivot, Response, Vec2,
};

#[derive(Debug, Clone)]
pub struct Absolute {
    pub anchor: Alignment,
    pub pivot: Pivot,
    pub offset: Vec2,
}

impl Absolute {
    pub fn new(anchor: Alignment, pivot: Pivot, offset: Vec2) -> Self {
        Self {
            anchor,
            pivot,
            offset,
        }
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<AbsoluteResponse> {
        widget_children::<AbsoluteWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct AbsoluteWidget {
    props: Absolute,
}

pub type AbsoluteResponse = ();

impl Widget for AbsoluteWidget {
    type Props<'a> = Absolute;
    type Response = AbsoluteResponse;

    fn new() -> Self {
        Self {
            props: Absolute {
                anchor: Alignment::TOP_LEFT,
                pivot: Pivot::TOP_LEFT,
                offset: Vec2::ZERO,
            },
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, _constraints: Constraints) -> Vec2 {
        let id = ctx.dom.current();
        let node = ctx.dom.get_current();
        let mut size = Vec2::ZERO;
        for &child in &node.children {
            size = size.max(ctx.calculate_layout(child, Constraints::none()));
        }

        let pivot_offset = -size * self.props.pivot.as_vec2();
        for &child in &node.children {
            ctx.layout.set_pos(child, pivot_offset);
        }

        ctx.layout.set_pos(
            id,
            self.props.anchor.as_vec2() * ctx.layout.viewport().size(),
        );

        Vec2::ZERO
    }
}
