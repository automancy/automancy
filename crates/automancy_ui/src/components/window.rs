use crate::*;

#[derive(Debug, Clone, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
struct Closable {
    closed: bool,
}

impl Closable {
    pub fn new(closed: bool) -> Self {
        Self {
            closed,
        }
    }

    pub fn show<F: FnOnce()>(self, children: F) -> Response<()> {
        widget_children::<ClosableWidget, F>(children, self)
    }
}

#[derive(Debug)]
struct ClosableWidget {
    props: Closable,
}

impl Widget for ClosableWidget {
    type Props<'a> = Closable;
    type Response = ();

    fn new() -> Self {
        Self {
            props: Closable::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        if self.props.closed {
            return Vec2::ZERO;
        }

        self.default_layout(ctx, constraints)
    }

    fn paint(&self, ctx: PaintContext<'_>) {
        if self.props.closed {
            return;
        }

        self.default_paint(ctx);
    }
}

#[derive(Debug, Clone, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct Window {
    title: Cow<'static, str>,
    pad: Pad,
    scroll: Scrollable,
    closed: bool,
}

auto_builders!(Window {
    pad: Pad,
    scroll: Scrollable,
    closed: bool,
});

pub type WindowResponse = ();

impl Window {
    pub fn new<S: Into<Cow<'static, str>>>(title: S) -> Self {
        Self {
            title: title.into(),
            pad: Pad::none(),
            scroll: Scrollable::vertical(),
            closed: false,
        }
    }

    #[track_caller]
    fn show_inner<F: FnOnce()>(self, children: F) -> Response<WindowResponse> {
        let title = Text::heading(self.title).inline(true);
        let line_height = title.style.line_height();
        let text_height = line_height + title.padding.top + title.padding.bottom;

        let mut response = None;

        RoundRect::new(sizing::ROUNDED_MEDIUM)
            .color(colors::BACKGROUND_1.yak())
            .show_children(|| {
                response = Some(widget_children::<WindowWidget, _>(
                    || {
                        title.show();

                        Pad::none().top(text_height).show(|| {
                            self.scroll.show(|| {
                                Pad::all(sizing::PADDING_LARGE).show(children);
                            });
                        });
                    },
                    WindowProps {},
                ));
            });

        response.unwrap()
    }

    #[track_caller]
    pub fn show_movable<F: FnOnce()>(self, props: Movable, children: F) -> Movable {
        let mut new_props = Movable::new(Vec2::ZERO);

        Layer::new().show(|| {
            Closable::new(self.closed).show(|| {
                new_props = props
                    .show(|| {
                        self.pad.show(|| {
                            self.show_inner(children);
                        });
                    })
                    .into_inner();
            });
        });

        new_props
    }

    #[track_caller]
    pub fn show<F: FnOnce()>(self, alignment: Alignment, children: F) -> Response<WindowResponse> {
        let mut response = None;

        Layer::new().show(|| {
            Closable::new(self.closed).show(|| {
                self.pad.show(|| {
                    align(alignment, || {
                        response = Some(self.show_inner(children));
                    });
                });
            });
        });

        response.unwrap()
    }
}

#[derive(Debug, Default)]
pub struct WindowProps {}

#[derive(Debug)]
pub struct WindowWidget {
    props: WindowProps,
}

impl Widget for WindowWidget {
    type Props<'a> = WindowProps;
    type Response = WindowResponse;

    fn new() -> Self {
        Self {
            props: WindowProps::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        let node = ctx.dom.get_current();
        let mut size = Vec2::ZERO;
        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, constraints);
            size = size.max(child_size);
        }

        for &child in &node.children {
            if let Some(node) = ctx.layout.get_mut(child) {
                node.rect
                    .set_pos(node.rect.pos() + Vec2::new((size.x - node.rect.size().x) / 2.0, 0.0));
            }
        }

        constraints.constrain_min(size)
    }
}

#[track_caller]
pub fn section(children: impl FnOnce()) {
    RoundRect::new(0.0)
        .border(Some(Border {
            color: colors::BACKGROUND_3.yak(),
            width: 2.0,
        }))
        .color(colors::BACKGROUND_1.yak())
        .show_children(|| {
            Pad::all(sizing::PADDING_MEDIUM).show(children);
        });
}
