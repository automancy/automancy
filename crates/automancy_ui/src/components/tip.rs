use crate::*;

thread_local! {
    static HOVER_TIP: Cell<Option<Text>> = Cell::default();
}

#[track_caller]
pub fn info_tip<S: Into<Cow<'static, str>>>(info: S) {
    let label = interactive(|| {
        symbol("\u{f449}", colors::TEXT_ACTIVE.yak());
    });

    if label.hovering {
        HOVER_TIP.set(Some(Text::normal(info)));
    }
}

#[cfg_attr(feature = "profile", profiling::function)]
pub fn render_info_tip() {
    Layer::new().show(|| {
        if let Some(tip) = HOVER_TIP.take() {
            hover_tip(|| {
                constrained(Constraints::loose(Vec2::new(360.0, f32::INFINITY)), || {
                    tip.show();
                });
            });
        }
    });
}
