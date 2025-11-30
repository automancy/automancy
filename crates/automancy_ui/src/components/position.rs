use crate::*;

#[derive(Debug, Clone, Copy, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct RectRecorder {}

impl RectRecorder {
    pub fn new() -> Self {
        Self {}
    }

    #[track_caller]
    pub fn show<F: FnOnce()>(self, children: F) -> Response<PositionRecordResponse> {
        widget_children::<RectRecorderWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct RectRecorderWidget {
    props: RectRecorder,
    rect: Cell<Option<Rect>>,
}

pub type PositionRecordResponse = Option<Rect>;

impl Widget for RectRecorderWidget {
    type Props<'a> = RectRecorder;
    type Response = PositionRecordResponse;

    fn new() -> Self {
        Self {
            props: RectRecorder::new(),
            rect: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        self.rect.get()
    }

    fn layout(&self, ctx: yakui::widget::LayoutContext<'_>, constraints: yakui::Constraints) -> Vec2 {
        if let Some(layout_node) = ctx.layout.get(ctx.dom.current()) {
            self.rect.set(Some(layout_node.rect));
        }

        self.default_layout(ctx, constraints)
    }
}
