use yakui::{util::widget_children, widget::Widget, Rect, Response, Vec2};

use std::cell::Cell;

#[derive(Debug, Default)]
pub struct PositionRecord {}

impl PositionRecord {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<PositionRecordResponse> {
        widget_children::<PositionRecordWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct PositionRecordWidget {
    props: PositionRecord,
    rect: Cell<Option<Rect>>,
}

pub type PositionRecordResponse = Option<Rect>;

impl Widget for PositionRecordWidget {
    type Props<'a> = PositionRecord;
    type Response = PositionRecordResponse;

    fn new() -> Self {
        Self {
            props: PositionRecord::new(),
            rect: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        self.rect.get()
    }

    fn layout(
        &self,
        ctx: yakui::widget::LayoutContext<'_>,
        constraints: yakui::Constraints,
    ) -> Vec2 {
        if let Some(layout_node) = ctx.layout.get(ctx.dom.current()) {
            let rect = layout_node.rect;
            if !rect.pos().abs_diff_eq(Vec2::ZERO, 0.001) {
                self.rect.set(Some(rect));
            }
        }

        self.default_layout(ctx, constraints)
    }
}
