use yakui::{
    util::widget_children,
    widget::{LayoutContext, Widget},
    Alignment, Constraints, Dim2, Flow, Pivot, Response, Vec2,
};

#[derive(Debug, Clone)]
pub struct Relative {
    pub anchor: Alignment,
    pub pivot: Pivot,
    pub offset: Dim2,
}

impl Relative {
    pub fn new(anchor: Alignment, pivot: Pivot, offset: Dim2) -> Self {
        Self {
            anchor,
            pivot,
            offset,
        }
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<RelativeResponse> {
        widget_children::<RelativeWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct RelativeWidget {
    props: Relative,
}

pub type RelativeResponse = ();

impl Widget for RelativeWidget {
    type Props<'a> = Relative;
    type Response = RelativeResponse;

    fn new() -> Self {
        Self {
            props: Relative {
                anchor: Alignment::TOP_LEFT,
                pivot: Pivot::TOP_LEFT,
                offset: Dim2::ZERO,
            },
        }
    }

    fn flow(&self) -> Flow {
        Flow::Relative {
            anchor: self.props.anchor,
            offset: self.props.offset,
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, _constraints: Constraints) -> Vec2 {
        let node = ctx.dom.get_current();
        let mut size = Vec2::ZERO;
        for &child in &node.children {
            size = size.max(ctx.calculate_layout(child, Constraints::none()));
        }

        let pivot_offset = -size * self.props.pivot.as_vec2();
        for &child in &node.children {
            ctx.layout.set_pos(child, pivot_offset);
        }

        Vec2::ZERO
    }
}
