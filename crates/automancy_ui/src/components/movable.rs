use crate::*;

#[derive(Debug, Clone, Copy, Default)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct Movable {
    pos: Vec2,
    drag_mouse_start: Option<Vec2>,
    drag_start: Option<Vec2>,
}

auto_builders!(Movable {
    drag_mouse_start: Option<Vec2>,
    drag_start: Option<Vec2>,
});

impl Movable {
    pub fn new(pos: Vec2) -> Self {
        Movable {
            pos,
            ..Default::default()
        }
    }

    pub fn clear(&mut self) {
        self.drag_mouse_start = None;
        self.drag_start = None;
    }

    #[track_caller]
    pub fn show<F: FnOnce()>(self, children: F) -> Response<<MovableWidget as Widget>::Response> {
        widget_children::<MovableWidget, F>(children, self)
    }
}

#[derive(Debug)]
pub struct MovableWidget {
    initialized: bool,
    pos: Cell<Vec2>,
    drag_mouse_start: Cell<Option<Vec2>>,
    drag_start: Cell<Option<Vec2>>,
}

impl Widget for MovableWidget {
    type Props<'a> = Movable;
    type Response = Movable;

    fn new() -> Self {
        Self {
            initialized: false,
            drag_mouse_start: Cell::default(),
            drag_start: Cell::default(),
            pos: Cell::default(),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        if !self.initialized {
            self.pos.set(props.pos);
            self.drag_mouse_start.set(props.drag_mouse_start);
            self.drag_start.set(props.drag_start);

            self.initialized = true;
        }

        Self::Response {
            pos: self.pos.get(),
            drag_mouse_start: self.drag_mouse_start.get(),
            drag_start: self.drag_start.get(),
        }
    }

    fn layout(&self, mut ctx: LayoutContext<'_>, constraints: Constraints) -> Vec2 {
        let constraints = Constraints::loose(constraints.max);
        let node = ctx.dom.get_current();

        let mut size = Vec2::ZERO;
        for &child in &node.children {
            let child_size = ctx.calculate_layout(child, constraints);
            size = size.max(child_size);
        }

        let parent = ctx
            .layout
            .get(node.parent.unwrap())
            .map(|parent| parent.clip)
            .unwrap_or_else(|| ctx.layout.viewport());

        let max_offset = size * 0.5 / parent.size();
        self.pos.set(self.pos.get().clamp(max_offset, Vec2::ONE - max_offset));

        ctx.layout.set_clip_logic(
            ctx.dom,
            ClipLogic::Contain {
                it: AbstractClipRect::LayoutRect,
                parent: AbstractClipRect::ParentClip,
                offset: (self.pos.get() * parent.size()) - size / 2.0,
            },
        );

        constraints.constrain(size)
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_ALL
    }

    fn event(&mut self, ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match *event {
            WidgetEvent::MouseEnter
            | WidgetEvent::MouseLeave
            | WidgetEvent::MouseScroll {
                ..
            } => EventResponse::Sink,
            WidgetEvent::MouseButtonChanged {
                button: MouseButton::One,
                down,
                inside,
                position,
                ..
            } => {
                if inside && down {
                    self.drag_mouse_start.set(Some(position));
                    self.drag_start.set(None);

                    EventResponse::Sink
                } else {
                    self.drag_mouse_start.set(None);
                    self.drag_start.set(None);

                    EventResponse::Bubble
                }
            },
            WidgetEvent::MouseMoved(Some(mouse_current)) => {
                if let Some(mouse_start) = self.drag_mouse_start.get() {
                    let pos = self.pos.get_mut();
                    let viewport = ctx.layout.viewport();

                    if self.drag_start.get().is_none() {
                        self.drag_start.set(Some(*pos));
                    }
                    let drag_start = self.drag_start.get().unwrap();

                    let delta = (mouse_current - mouse_start) / viewport.size();

                    self.pos.set(drag_start + delta);
                } else {
                    self.drag_start.set(None);
                }

                EventResponse::Bubble
            },
            _ => EventResponse::Bubble,
        }
    }
}
