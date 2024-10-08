use yakui::{
    util::widget_children,
    widget::{LayoutContext, Widget},
    widgets::{CountGrid, List},
    Constraints, CrossAxisAlignment, MainAxisAlignItems, MainAxisAlignment, MainAxisSize, Response,
    Vec2,
};

#[derive(Debug, Clone, Default)]
pub struct ViewportConstrained {}

impl ViewportConstrained {
    #[track_caller]
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

pub fn grid_row(count: usize) -> CountGrid {
    let mut v = CountGrid::row(count);
    v.main_axis_size = MainAxisSize::Min;
    v.main_axis_alignment = MainAxisAlignment::Center;
    v.cross_axis_alignment = CrossAxisAlignment::Stretch;
    v.main_axis_align_items = MainAxisAlignItems::Center;
    v
}

pub fn grid_col(count: usize) -> CountGrid {
    let mut v = CountGrid::col(count);
    v.main_axis_size = MainAxisSize::Min;
    v.main_axis_alignment = MainAxisAlignment::Center;
    v.cross_axis_alignment = CrossAxisAlignment::Stretch;
    v.main_axis_align_items = MainAxisAlignItems::Center;
    v
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

#[track_caller]
pub fn row(children: impl FnOnce()) {
    list_row().show(children);
}

#[track_caller]
pub fn col(children: impl FnOnce()) {
    list_col().show(children);
}

#[track_caller]
pub fn row_max(children: impl FnOnce()) {
    list_row_max().show(children);
}

#[track_caller]
pub fn col_max(children: impl FnOnce()) {
    list_col_max().show(children);
}

#[track_caller]
pub fn centered_horizontal(children: impl FnOnce()) {
    let mut v = list_row_max();
    v.main_axis_alignment = MainAxisAlignment::Center;
    v.show(children);
}

#[track_caller]
pub fn centered_vertical(children: impl FnOnce()) {
    let mut v = list_col_max();
    v.main_axis_alignment = MainAxisAlignment::Center;
    v.show(children);
}

#[track_caller]
pub fn row_align_end(children: impl FnOnce()) {
    let mut v = list_row();
    v.cross_axis_alignment = CrossAxisAlignment::End;
    v.show(children);
}

#[track_caller]
pub fn col_align_end(children: impl FnOnce()) {
    let mut v = list_col();
    v.cross_axis_alignment = CrossAxisAlignment::End;
    v.show(children);
}

#[track_caller]
pub fn center_row(children: impl FnOnce()) {
    let mut v = list_row();
    v.cross_axis_alignment = CrossAxisAlignment::Center;
    v.show(children);
}

#[track_caller]
pub fn center_col(children: impl FnOnce()) {
    let mut v = list_col();
    v.cross_axis_alignment = CrossAxisAlignment::Center;
    v.show(children);
}

#[track_caller]
pub fn stretch_row(children: impl FnOnce()) {
    let mut v = list_row();
    v.cross_axis_alignment = CrossAxisAlignment::Stretch;
    v.show(children);
}

#[track_caller]
pub fn stretch_col(children: impl FnOnce()) {
    let mut v = list_col();
    v.cross_axis_alignment = CrossAxisAlignment::Stretch;
    v.show(children);
}

#[track_caller]
pub fn spaced_row(children: impl FnOnce()) {
    let mut v = list_row_max();
    v.main_axis_alignment = MainAxisAlignment::SpaceEvenly;
    v.show(children);
}

#[track_caller]
pub fn spaced_col(children: impl FnOnce()) {
    let mut v = list_col_max();
    v.main_axis_alignment = MainAxisAlignment::SpaceEvenly;
    v.show(children);
}

#[track_caller]
pub fn viewport_constrained(children: impl FnOnce()) {
    ViewportConstrained::default().show(children);
}
