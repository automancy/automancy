use yakui::{
    use_state, util::widget_children, widget::Widget, widgets::StateResponse, Response, Vec2,
};

use core::fmt;

pub struct PositionRecord {
    pos: Response<StateResponse<Option<Vec2>>>,
}

impl PositionRecord {
    pub fn new() -> Self {
        Self {
            pos: use_state(|| None),
        }
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<PositionRecordResponse> {
        widget_children::<PositionRecordWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct PositionRecordWidget {
    props: PositionRecord,
}

pub type PositionRecordResponse = Option<Vec2>;

impl Widget for PositionRecordWidget {
    type Props<'a> = PositionRecord;
    type Response = PositionRecordResponse;

    fn new() -> Self {
        Self {
            props: PositionRecord::new(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        self.props.pos.get()
    }

    fn layout(
        &self,
        ctx: yakui::widget::LayoutContext<'_>,
        constraints: yakui::Constraints,
    ) -> Vec2 {
        let layout_node = ctx.layout.get(ctx.dom.current()).unwrap();

        let rect = layout_node.rect;
        if rect.pos().abs_diff_eq(Vec2::ZERO, 0.001) {
            self.props.pos.set(Some(rect.pos()))
        }

        self.default_layout(ctx, constraints)
    }
}

impl fmt::Debug for PositionRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PositionRecord").finish_non_exhaustive()
    }
}
