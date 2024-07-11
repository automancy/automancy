use yakui::{
    util::widget_children,
    widget::{LayoutContext, Widget},
    widgets::List,
    Constraints, CrossAxisAlignment, MainAxisAlignment, MainAxisSize, Response, Vec2,
};

#[derive(Debug, Clone, Default)]
pub struct ViewportConstrained {}

impl ViewportConstrained {
    pub fn show<F: FnOnce()>(self, children: F) -> Response<()> {
        widget_children::<ViewportConstrainedWidget, F>(children, self)
    }
}

#[derive(Debug)]
struct ViewportConstrainedWidget {
    props: ViewportConstrained,
}

type ViewportConstrainedResponse = ();

impl Widget for ViewportConstrainedWidget {
    type Props<'a> = ViewportConstrained;
    type Response = ViewportConstrainedResponse;

    fn new() -> Self {
        Self {
            props: ViewportConstrained::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        let constraints = Constraints {
            min: constraints.min,
            max: constraints.max.min(ctx.layout.viewport().size()),
        };

        self.default_layout(ctx, constraints)
    }
}

pub fn list_row() -> List {
    let mut v = List::row();
    v.main_axis_size = MainAxisSize::Min;
    v
}

pub fn list_col() -> List {
    let mut v = List::column();
    v.main_axis_size = MainAxisSize::Min;
    v
}

pub fn list_row_max() -> List {
    let mut v = List::row();
    v.main_axis_size = MainAxisSize::Max;
    v
}

pub fn list_col_max() -> List {
    let mut v = List::column();
    v.main_axis_size = MainAxisSize::Max;
    v
}

pub fn row(children: impl FnOnce()) {
    list_row().show(children);
}

pub fn col(children: impl FnOnce()) {
    list_col().show(children);
}

pub fn row_max(children: impl FnOnce()) {
    list_row_max().show(children);
}

pub fn col_max(children: impl FnOnce()) {
    list_col_max().show(children);
}

pub fn centered_horizontal(children: impl FnOnce()) {
    let mut v = list_row_max();
    v.main_axis_alignment = MainAxisAlignment::Center;
    v.show(children);
}

pub fn centered_vertical(children: impl FnOnce()) {
    let mut v = list_col_max();
    v.main_axis_alignment = MainAxisAlignment::Center;
    v.show(children);
}

pub fn righthand_row(children: impl FnOnce()) {
    let mut v = list_row();
    v.cross_axis_alignment = CrossAxisAlignment::End;
    v.show(children);
}

pub fn righthand_col(children: impl FnOnce()) {
    let mut v = list_col();
    v.cross_axis_alignment = CrossAxisAlignment::End;
    v.show(children);
}

pub fn center_row(children: impl FnOnce()) {
    let mut v = list_row();
    v.cross_axis_alignment = CrossAxisAlignment::Center;
    v.show(children);
}

pub fn center_col(children: impl FnOnce()) {
    let mut v = list_col();
    v.cross_axis_alignment = CrossAxisAlignment::Center;
    v.show(children);
}

pub fn stretch_row(children: impl FnOnce()) {
    let mut v = list_row();
    v.cross_axis_alignment = CrossAxisAlignment::Stretch;
    v.show(children);
}

pub fn stretch_col(children: impl FnOnce()) {
    let mut v = list_col();
    v.cross_axis_alignment = CrossAxisAlignment::Stretch;
    v.show(children);
}

pub fn spaced_row(children: impl FnOnce()) {
    let mut v = list_row_max();
    v.main_axis_alignment = MainAxisAlignment::SpaceEvenly;
    v.show(children);
}

pub fn spaced_col(children: impl FnOnce()) {
    let mut v = list_col_max();
    v.main_axis_alignment = MainAxisAlignment::SpaceEvenly;
    v.show(children);
}

pub fn viewport_constrained(children: impl FnOnce()) {
    ViewportConstrained::default().show(children);
}
