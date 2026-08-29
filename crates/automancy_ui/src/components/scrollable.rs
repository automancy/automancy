use crate::*;

#[derive(Debug, Clone, Copy)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct Scrollable {
    pub direction: ScrollDirection,
    pub child_size: Constraints,
    pub min_child_size: Constraints,
    pub radius: BorderRadius,
    pub scrollbar_pos: Option<ScrollbarPosition>,
}

auto_builders!(Scrollable {
    direction: ScrollDirection,
    child_size: Constraints,
    min_child_size: Constraints,
    radius: BorderRadius,
    scrollbar_pos: Option<ScrollbarPosition>,
});

impl Default for Scrollable {
    fn default() -> Self {
        Self {
            direction: Default::default(),
            child_size: Constraints::none(),
            min_child_size: Constraints::loose(Vec2::ZERO),
            radius: Default::default(),
            scrollbar_pos: Some(ScrollbarPosition::default()),
        }
    }
}

impl Scrollable {
    pub fn vertical() -> Self {
        Scrollable {
            direction: ScrollDirection::Y,
            ..Default::default()
        }
    }

    pub fn horizontal() -> Self {
        Scrollable {
            direction: ScrollDirection::X,
            ..Default::default()
        }
    }

    pub fn xy() -> Self {
        Scrollable {
            direction: ScrollDirection::XY,
            ..Default::default()
        }
    }

    #[track_caller]
    pub fn show<F: FnOnce()>(self, children: F) -> Response<ScrollableResponse> {
        widget_children::<ScrollableWidget, F>(children, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollDirection {
    #[default]
    Y,
    X,
    XY,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarPosition {
    #[default]
    End,
    Start,
}

impl ScrollbarPosition {
    pub fn percent(self) -> f32 {
        match self {
            ScrollbarPosition::End => 1.0,
            ScrollbarPosition::Start => 0.0,
        }
    }
}

#[derive(Debug)]
pub struct ScrollableWidget {
    props: Scrollable,
    last_drag_pos: Option<Vec2>,
    scroll_position: Cell<Vec2>,
    canvas_size: Cell<Vec2>,
    size: Cell<Vec2>,
    scroll_percentage: Cell<Vec2>,
}

#[derive(Debug, Clone, Copy, Default)]

pub struct ScrollableResponse {
    pub canvas_size: Vec2,
    pub size: Vec2,
    pub scroll_percentage: Vec2,
}

impl Widget for ScrollableWidget {
    type Props<'a> = Scrollable;
    type Response = ScrollableResponse;

    fn new() -> Self {
        Self {
            props: Scrollable::default(),
            last_drag_pos: None,
            scroll_position: Cell::new(Vec2::ZERO),
            canvas_size: Cell::new(Vec2::ZERO),
            size: Cell::new(Vec2::ZERO),
            scroll_percentage: Cell::new(Vec2::ZERO),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        self.props = props;

        ScrollableResponse {
            canvas_size: self.canvas_size.get(),
            size: self.size.get(),
            scroll_percentage: self.scroll_percentage.get(),
        }
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        ctx.layout.enable_clipping(ctx.dom);

        let node = ctx.dom.get_current();
        let mut canvas_size = Vec2::ZERO;

        let child_size_min = self.props.min_child_size.min.max(constraints.min.max(self.props.child_size.min));
        let child_size_max = self.props.min_child_size.max.max(constraints.max.min(self.props.child_size.max));

        let child_constraints = match self.props.direction {
            ScrollDirection::Y => Constraints {
                min: child_size_min,
                max: Vec2::new(child_size_max.x, f32::INFINITY),
            },
            ScrollDirection::X => Constraints {
                min: child_size_min,
                max: Vec2::new(f32::INFINITY, child_size_max.y),
            },
            ScrollDirection::XY => Constraints {
                min: child_size_min,
                max: child_size_max,
            },
        };

        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, child_constraints);
            canvas_size = canvas_size.max(child_size);
        }

        let size = constraints.constrain(canvas_size).min(self.props.child_size.max);

        self.canvas_size.set(canvas_size);
        self.size.set(size);

        let mut pos = self.scroll_position.get();

        match self.props.direction {
            ScrollDirection::Y => pos.x = 0.0,
            ScrollDirection::X => pos.y = 0.0,
            ScrollDirection::XY => {},
        }

        let max_scroll_position = (canvas_size - size).max(Vec2::ZERO);
        pos = pos.min(max_scroll_position).max(Vec2::ZERO);

        for &child in &node.children {
            ctx.layout.set_pos(child, -pos);
        }

        self.scroll_position.set(pos);
        self.scroll_percentage.set(if canvas_size == size {
            Vec2::ZERO
        } else {
            pos / (canvas_size - size)
        });

        size
    }

    fn paint(&self, mut ctx: PaintContext<'_>) {
        const SHADOW_SIZE: f32 = 16.0;
        const THRESHOLD: f32 = 10.0;
        const ALPHA: f32 = 0.6;

        fn paint_shadow_top(ctx: &mut PaintContext<'_>, start: Vec2, rect: Rect, shadow_size: Vec2, radius: BorderRadius) {
            let mul = ((start.y - THRESHOLD) / SHADOW_SIZE).clamp(0.0, 1.0);

            if mul > 0.0 {
                shapes::PaintRoundRect::new(
                    Rect::from_pos_size(rect.pos(), shadow_size),
                    BorderRadius {
                        top_left: radius.top_left,
                        top_right: radius.top_right,
                        ..Default::default()
                    },
                )
                .color_y((
                    colors::BACKGROUND_INACTIVE.linear.with_alpha(ALPHA * mul).yak(),
                    colors::TRANSPARENT.yak(),
                ))
                .add(ctx.paint);
            }
        }
        fn paint_shadow_bottom(ctx: &mut PaintContext<'_>, end: Vec2, rect: Rect, shadow_size: Vec2, radius: BorderRadius) {
            let mul = ((end.y - THRESHOLD) / SHADOW_SIZE).clamp(0.0, 1.0);

            if mul > 0.0 {
                shapes::PaintRoundRect::new(
                    Rect::from_pos_size(rect.pos() + (rect.size() - shadow_size), shadow_size),
                    BorderRadius {
                        bottom_left: radius.bottom_left,
                        bottom_right: radius.bottom_right,
                        ..Default::default()
                    },
                )
                .color_y((
                    colors::TRANSPARENT.yak(),
                    colors::BACKGROUND_INACTIVE.linear.with_alpha(ALPHA * mul).yak(),
                ))
                .add(ctx.paint);
            }
        }
        fn paint_shadow_left(ctx: &mut PaintContext<'_>, start: Vec2, rect: Rect, shadow_size: Vec2, radius: BorderRadius) {
            let mul = ((start.x - THRESHOLD) / SHADOW_SIZE).clamp(0.0, 1.0);

            if mul > 0.0 {
                shapes::PaintRoundRect::new(
                    Rect::from_pos_size(rect.pos(), shadow_size),
                    BorderRadius {
                        top_left: radius.top_left,
                        bottom_left: radius.bottom_left,
                        ..Default::default()
                    },
                )
                .color_x((
                    colors::BACKGROUND_INACTIVE.linear.with_alpha(ALPHA * mul).yak(),
                    colors::TRANSPARENT.yak(),
                ))
                .add(ctx.paint);
            }
        }
        fn paint_shadow_right(ctx: &mut PaintContext<'_>, end: Vec2, rect: Rect, shadow_size: Vec2, radius: BorderRadius) {
            let mul = ((end.x - THRESHOLD) / SHADOW_SIZE).clamp(0.0, 1.0);

            if mul > 0.0 {
                shapes::PaintRoundRect::new(
                    Rect::from_pos_size(rect.pos() + (rect.size() - shadow_size), shadow_size),
                    BorderRadius {
                        top_right: radius.top_right,
                        bottom_right: radius.bottom_right,
                        ..Default::default()
                    },
                )
                .color_x((
                    colors::TRANSPARENT.yak(),
                    colors::BACKGROUND_INACTIVE.linear.with_alpha(ALPHA * mul).yak(),
                ))
                .add(ctx.paint);
            }
        }

        let rect = ctx.layout.get(ctx.dom.current()).unwrap().rect;

        let canvas_size = self.canvas_size.get();
        let pos = self.scroll_position.get();
        let size = self.size.get();
        let scroll_percentage = self.scroll_percentage.get();

        ctx.paint.layers.push();
        {
            let start = pos;
            let end = canvas_size - (size + pos);

            match self.props.direction {
                ScrollDirection::Y => {
                    let shadow_size = Vec2::new(rect.size().x, SHADOW_SIZE);

                    paint_shadow_top(&mut ctx, start, rect, shadow_size, self.props.radius);
                    paint_shadow_bottom(&mut ctx, end, rect, shadow_size, self.props.radius);
                },
                ScrollDirection::X => {
                    let shadow_size = Vec2::new(SHADOW_SIZE, rect.size().y);

                    paint_shadow_left(&mut ctx, start, rect, shadow_size, self.props.radius);
                    paint_shadow_right(&mut ctx, end, rect, shadow_size, self.props.radius);
                },
                ScrollDirection::XY => {
                    {
                        let shadow_size = Vec2::new(rect.size().x, SHADOW_SIZE);

                        paint_shadow_top(&mut ctx, start, rect, shadow_size, self.props.radius);
                        paint_shadow_bottom(&mut ctx, end, rect, shadow_size, self.props.radius);
                    }

                    {
                        let shadow_size = Vec2::new(SHADOW_SIZE, rect.size().y);

                        paint_shadow_left(&mut ctx, start, rect, shadow_size, self.props.radius);
                        paint_shadow_right(&mut ctx, end, rect, shadow_size, self.props.radius);
                    }
                },
            }
        }
        ctx.paint.layers.pop();

        ctx.paint.layers.push();
        if let Some(scrollbar_pos) = self.props.scrollbar_pos {
            const SCROLLBAR_COLORS: (Color, Color) = (colors::INTERACTIVE_1.yak(), colors::INTERACTIVE_2.yak());
            const SCROLLBAR_SIZE: f32 = 4.0;
            const SCROLLBAR_RADIUS: f32 = 8.0;

            let ratio = size / canvas_size;
            let diff = canvas_size - size;
            let offset = diff * ratio * scroll_percentage;

            let paint_bar_x = |ctx: &mut PaintContext<'_>| {
                if diff.x > 0.01 {
                    let percent = Vec2::new(0.0, scrollbar_pos.percent());

                    let scrollbar_size = Vec2::new((size.x * ratio.x).floor(), SCROLLBAR_SIZE);
                    let scrollbar_rect = Rect::from_pos_size(
                        (rect.pos() + (percent * rect.size()) + (percent * -SCROLLBAR_SIZE) + Vec2::new(offset.x, 0.0)).round(),
                        scrollbar_size,
                    );

                    shapes::PaintRoundRect::new(scrollbar_rect, SCROLLBAR_RADIUS)
                        .color_x(SCROLLBAR_COLORS)
                        .add(ctx.paint);
                }
            };

            let paint_bar_y = |ctx: &mut PaintContext<'_>| {
                if diff.y > 0.01 {
                    let percent = Vec2::new(scrollbar_pos.percent(), 0.0);

                    let scrollbar_size = Vec2::new(SCROLLBAR_SIZE, (size.y * ratio.y).floor());
                    let scrollbar_rect = Rect::from_pos_size(
                        (rect.pos() + (percent * rect.size()) + (percent * -SCROLLBAR_SIZE) + Vec2::new(0.0, offset.y)).round(),
                        scrollbar_size,
                    );

                    shapes::PaintRoundRect::new(scrollbar_rect, SCROLLBAR_RADIUS)
                        .color_y(SCROLLBAR_COLORS)
                        .add(ctx.paint);
                }
            };

            match self.props.direction {
                ScrollDirection::Y => {
                    paint_bar_y(&mut ctx);
                },
                ScrollDirection::X => {
                    paint_bar_x(&mut ctx);
                },
                ScrollDirection::XY => {
                    paint_bar_y(&mut ctx);
                    paint_bar_x(&mut ctx);
                },
            }
        }
        ctx.paint.layers.pop();

        let node = ctx.dom.get_current();
        for &child in &node.children {
            ctx.paint(child);
        }
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_ALL
    }

    fn event(&mut self, ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match *event {
            WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                down,
                inside,
                ..
            } => {
                self.last_drag_pos = None;
                if ctx.input.focus() == Some(ctx.dom.current()) {
                    ctx.input.set_focus(None);
                }

                if inside && down {
                    ctx.input.set_focus(Some(ctx.dom.current()));

                    return EventResponse::Sink;
                }

                EventResponse::Bubble
            },
            WidgetEvent::MouseMoved(Some(mouse)) => {
                if ctx.input.focus() == Some(ctx.dom.current()) {
                    if let Some(start) = self.last_drag_pos {
                        let pos = self.scroll_position.get();
                        let pos = pos - (mouse - start);

                        let max_scroll_position = (self.canvas_size.get() - self.size.get()).max(Vec2::ZERO);

                        let pos = pos.min(max_scroll_position).max(Vec2::ZERO);

                        self.scroll_position.set(pos);
                    }

                    self.last_drag_pos = Some(mouse);

                    EventResponse::Sink
                } else {
                    EventResponse::Bubble
                }
            },
            WidgetEvent::MouseScroll {
                mut delta,
                modifiers,
            } => {
                if modifiers.shift() {
                    delta = delta.yx();
                }

                if modifiers.ctrl() {
                    delta *= 10.0;
                }

                match self.props.direction {
                    ScrollDirection::Y => {
                        if delta.y.abs() < 0.01 {
                            return EventResponse::Bubble;
                        }
                    },
                    ScrollDirection::X => {
                        delta = delta.yx();

                        if delta.x.abs() < 0.01 {
                            return EventResponse::Bubble;
                        }
                    },
                    ScrollDirection::XY => {
                        if delta.x.abs() < 0.01 && delta.y.abs() < 0.01 {
                            return EventResponse::Bubble;
                        }
                    },
                }

                let max_scroll_position = (self.canvas_size.get() - self.size.get()).max(Vec2::ZERO);
                if max_scroll_position == Vec2::ZERO {
                    return EventResponse::Bubble;
                }

                self.scroll_position
                    .update(|pos| (pos + delta).min(max_scroll_position).max(Vec2::ZERO));

                EventResponse::Sink
            },
            _ => EventResponse::Bubble,
        }
    }
}
