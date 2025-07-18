use automancy_defs::colors;
use yakui::{
    Alignment, Dim2, Pivot, Response, Vec2, colored_circle, reflow, use_state,
    widgets::{ButtonResponse, Circle, Layer, Pad},
};

use crate::{
    InteractiveResponse, PADDING_MEDIUM, RoundRect, button, center_row, col, interactive,
    scroll_vertical_bar_alignment,
};

#[track_caller]
pub fn selection_box<T: Clone + Eq, S: AsRef<str>>(
    options: impl IntoIterator<Item = T>,
    current: T,
    format: &dyn Fn(&T) -> S,
) -> T {
    let open = use_state(|| false);
    let mut selected = current;

    col(|| {
        if button(format(&selected).as_ref()).clicked {
            open.modify(|v| !v);
        }

        if open.get() {
            Layer::new().show(|| {
                reflow(Alignment::BOTTOM_LEFT, Pivot::TOP_LEFT, Dim2::ZERO, || {
                    RoundRect::new(8.0, colors::BACKGROUND_1).show_children(|| {
                        scroll_vertical_bar_alignment(
                            Vec2::ZERO,
                            Vec2::new(160.0, 200.0),
                            None,
                            || {
                                Pad::all(PADDING_MEDIUM).show(|| {
                                    col(|| {
                                        for option in options.into_iter() {
                                            if button(format(&option).as_ref()).clicked {
                                                selected = option;
                                                open.set(false);
                                            }
                                        }
                                    });
                                });
                            },
                        );
                    });
                });
            });
        }
    });

    selected
}

#[track_caller]
pub fn selection_button<T: Eq>(
    current: &mut T,
    this: T,
    button: impl FnOnce(bool) -> Response<ButtonResponse>,
) -> Response<ButtonResponse> {
    let r = button(*current == this);

    if r.clicked {
        *current = this;
    }

    r
}

#[track_caller]
pub fn radio<T: Eq>(
    current: &mut T,
    this: T,
    children: impl FnOnce(),
) -> Response<InteractiveResponse> {
    let hovered = use_state(|| false);

    let r = interactive(|| {
        center_row(|| {
            let mut outer_circle = Circle::new();
            outer_circle.color = if hovered.get() {
                colors::BACKGROUND_3
            } else {
                colors::BACKGROUND_2
            };
            outer_circle.min_radius = 12.0;

            Pad::horizontal(PADDING_MEDIUM).show(|| {
                outer_circle.show_children(|| {
                    Pad::all(4.0).show(|| {
                        colored_circle(
                            if *current == this {
                                colors::BLACK
                            } else if hovered.get() {
                                colors::BACKGROUND_3
                            } else {
                                colors::BACKGROUND_2
                            },
                            6.0,
                        );
                    });
                });
            });

            children();
        })
    });

    hovered.set(r.hovering);

    if r.clicked {
        *current = this;
    }

    r
}
