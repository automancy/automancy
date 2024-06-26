use automancy_defs::colors;
use yakui::{
    colored_circle, column, use_state,
    widgets::{Circle, Layer, Pad},
    Alignment, Dim2, Pivot, Response,
};

use super::{
    button::{button, ButtonResponse},
    container::RoundRect,
    interactive::{interactive, InteractiveResponse},
    layout::centered_row,
    relative::Relative,
    scrollable::scroll_vertical,
    PADDING_MEDIUM,
};

pub fn selection_box<T: Clone + Eq>(
    options: impl IntoIterator<Item = T>,
    default: T,
    format: &dyn Fn(&T) -> String,
) -> T {
    let open = use_state(|| false);
    let mut selected = default;

    column(|| {
        if button(&format(&selected)).clicked {
            open.modify(|v| !v);
        }

        if open.get() {
            Relative::new(Alignment::BOTTOM_LEFT, Pivot::TOP_LEFT, Dim2::ZERO).show(|| {
                Layer::new().show(|| {
                    RoundRect::new(8.0, colors::BACKGROUND_1).show_children(|| {
                        scroll_vertical(250.0, || {
                            Pad::all(PADDING_MEDIUM).show(|| {
                                column(|| {
                                    for option in options.into_iter() {
                                        if button(&format(&option)).clicked {
                                            selected = option;
                                            open.set(false);
                                        }
                                    }
                                });
                            });
                        });
                    });
                });
            });
        }
    });

    selected
}

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

pub fn radio<T: Eq>(
    current: &mut T,
    this: T,
    children: impl FnOnce(),
) -> Response<InteractiveResponse> {
    let hovered = use_state(|| false);

    let r = interactive(|| {
        centered_row(|| {
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
