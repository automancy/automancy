use crate::*;

pub fn grid_row(count: usize) -> CountGrid {
    CountGrid::row(count)
        .main_axis_size(MainAxisSize::Min)
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .main_axis_align_items(MainAxisAlignItems::Center)
}

pub fn grid_col(count: usize) -> CountGrid {
    CountGrid::col(count)
        .main_axis_size(MainAxisSize::Min)
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .main_axis_align_items(MainAxisAlignItems::Center)
}

pub fn list_row() -> List {
    List::row().main_axis_size(MainAxisSize::Min)
}

pub fn list_col() -> List {
    List::column().main_axis_size(MainAxisSize::Min)
}

pub fn list_row_max() -> List {
    List::row().main_axis_size(MainAxisSize::Max)
}

pub fn list_col_max() -> List {
    List::column().main_axis_size(MainAxisSize::Max)
}

#[track_caller]
pub fn centered_row(children: impl FnOnce()) -> Response<()> {
    list_row_max()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .show(children)
}

#[track_caller]
pub fn centered_col(children: impl FnOnce()) -> Response<()> {
    list_col_max()
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .show(children)
}

#[track_caller]
pub fn row(children: impl FnOnce()) -> Response<()> {
    list_row().show(children)
}

#[track_caller]
pub fn col(children: impl FnOnce()) -> Response<()> {
    list_col().show(children)
}

#[track_caller]
pub fn row_max(children: impl FnOnce()) -> Response<()> {
    list_row_max().show(children)
}

#[track_caller]
pub fn col_max(children: impl FnOnce()) -> Response<()> {
    list_col_max().show(children)
}

#[track_caller]
pub fn row_center(children: impl FnOnce()) -> Response<()> {
    list_row_max().main_axis_alignment(MainAxisAlignment::Center).show(children)
}

#[track_caller]
pub fn col_center(children: impl FnOnce()) -> Response<()> {
    list_col_max().main_axis_alignment(MainAxisAlignment::Center).show(children)
}

#[track_caller]
pub fn row_end(children: impl FnOnce()) -> Response<()> {
    list_row_max().main_axis_alignment(MainAxisAlignment::End).show(children)
}

#[track_caller]
pub fn col_end(children: impl FnOnce()) -> Response<()> {
    list_col_max().main_axis_alignment(MainAxisAlignment::End).show(children)
}

#[track_caller]
pub fn row_cross_end(children: impl FnOnce()) -> Response<()> {
    list_row().cross_axis_alignment(CrossAxisAlignment::End).show(children)
}

#[track_caller]
pub fn col_cross_end(children: impl FnOnce()) -> Response<()> {
    list_col().cross_axis_alignment(CrossAxisAlignment::End).show(children)
}

#[track_caller]
pub fn row_cross_center(children: impl FnOnce()) -> Response<()> {
    list_row().cross_axis_alignment(CrossAxisAlignment::Center).show(children)
}

#[track_caller]
pub fn col_cross_center(children: impl FnOnce()) -> Response<()> {
    list_col().cross_axis_alignment(CrossAxisAlignment::Center).show(children)
}

#[track_caller]
pub fn row_cross_stretch(children: impl FnOnce()) -> Response<()> {
    list_row().cross_axis_alignment(CrossAxisAlignment::Stretch).show(children)
}

#[track_caller]
pub fn col_cross_stretch(children: impl FnOnce()) -> Response<()> {
    list_col().cross_axis_alignment(CrossAxisAlignment::Stretch).show(children)
}

#[track_caller]
pub fn row_spaced_evenly(children: impl FnOnce()) -> Response<()> {
    list_row_max().main_axis_alignment(MainAxisAlignment::SpaceEvenly).show(children)
}

#[track_caller]
pub fn col_spaced_evenly(children: impl FnOnce()) -> Response<()> {
    list_col_max().main_axis_alignment(MainAxisAlignment::SpaceEvenly).show(children)
}
