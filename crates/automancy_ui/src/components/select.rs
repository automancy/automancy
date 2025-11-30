use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Dropdown<T: 'static> {
    current: T,
    options: StaticList<T>,
    constraints: Constraints,
}

impl<T> Dropdown<T>
where
    T: Debug + Clone + Eq + 'static,
{
    pub fn new(current: T, options: impl Into<StaticList<T>>) -> Self {
        Self {
            current,
            options: options.into(),
            constraints: Constraints::none(),
        }
    }

    pub fn constraints(self, constraints: Constraints) -> Self {
        Self {
            constraints,
            ..self
        }
    }

    #[track_caller]
    pub fn show<S, F>(self, format: F) -> Response<DropdownResponse<T>>
    where
        S: Debug + Into<Cow<'static, str>> + 'static,
        F: Fn(&T) -> S + 'static,
    {
        widget::<DropdownWidget<T, S, IgnoreDebug<F>>>((self, IgnoreDebug(format)))
    }
}

pub struct DropdownResponse<T> {
    pub new_value: Option<T>,
    pub close_dropdown: bool,
}

#[derive(Debug)]
pub struct DropdownWidget<T: 'static, S, F> {
    __: PhantomData<(S, F)>,
    props: Option<Dropdown<T>>,
    max_size: Cell<Vec2>,
    close_dropdown: bool,
}

impl<T, S, F> Widget for DropdownWidget<T, S, IgnoreDebug<F>>
where
    T: Debug + Clone + Eq + 'static,
    S: Debug + Into<Cow<'static, str>> + 'static,
    F: Fn(&T) -> S + 'static,
{
    type Props<'a> = (Dropdown<T>, IgnoreDebug<F>);

    type Response = DropdownResponse<T>;

    fn new() -> Self {
        Self {
            __: PhantomData,
            props: None,
            max_size: Cell::new(Vec2::ZERO),
            close_dropdown: false,
        }
    }

    fn flow(&self) -> Flow {
        Flow::Relative {
            anchor: Alignment::BOTTOM_LEFT,
            offset: Dim2::ZERO,
        }
    }

    fn update(&mut self, (props, format): Self::Props<'_>) -> Self::Response {
        let mut result = None;

        if self.props.as_ref() != Some(&props) {
            self.max_size.set(Vec2::ZERO);
        }

        Opaque::new().show(|| {
            RoundRect::new(sizing::ROUNDED_MEDIUM)
                .color(colors::BACKGROUND_OPAQUE.yak())
                .show_children(|| {
                    Scrollable::vertical()
                        .child_size(props.constraints)
                        .radius(sizing::ROUNDED_MEDIUM)
                        .show(|| {
                            Pad::all(sizing::PADDING_MEDIUM).show(|| {
                                col(|| {
                                    for option in props.options.iter() {
                                        if button(format(option)).clicked {
                                            result = Some(option.clone());
                                        }
                                    }
                                });
                            });
                        });
                });
        });

        self.props = Some(props);

        Self::Response {
            new_value: result,
            close_dropdown: std::mem::take(&mut self.close_dropdown),
        }
    }

    fn layout(&self, ctx: LayoutContext<'_>, mut constraints: Constraints) -> Vec2 {
        ctx.layout.escape_clipping(ctx.dom);
        ctx.layout.new_layer(ctx.dom);

        let max_size = self.max_size.get();

        if max_size != Vec2::ZERO
            && let Some(node) = ctx.layout.get(ctx.dom.current())
        {
            let rect = Rect::from_pos_size(node.rect.pos(), max_size);
            let constrained = ctx.layout.viewport().constrain(rect);

            constraints = Constraints {
                min: constraints.min.min(constrained.size()),
                max: constraints.max.min(constrained.size()),
            };
        }

        let size = self.default_layout(ctx, constraints);

        if max_size == Vec2::ZERO {
            self.max_size.set(size);
        }

        size
    }

    fn event_interest(&self) -> EventInterest {
        EventInterest::MOUSE_ALL
    }

    fn event(&mut self, _ctx: EventContext<'_>, event: &WidgetEvent) -> EventResponse {
        match event {
            // TODO: doesn't work yet
            WidgetEvent::KeyChanged {
                key,
                down,
                modifiers: _,
            } if *down && key == &KeyCode::Escape => self.close_dropdown = true,
            WidgetEvent::MouseButtonChanged {
                button,
                down,
                inside,
                ..
            } if *down && *button == MouseButton::One && !inside => {
                self.close_dropdown = true;
            },
            _ => {},
        }

        EventResponse::Bubble
    }
}

#[track_caller]
pub fn dropdown_selection<T, S>(
    current: T,
    options: impl Into<StaticList<T>>,
    format: impl Fn(&T) -> S + 'static,
    constraints: Constraints,
) -> Option<T>
where
    T: Debug + Clone + Eq + 'static,
    S: Debug + Into<Cow<'static, str>> + 'static,
{
    let open = use_state(|| false);

    let mut result = None;
    col(|| {
        let dropdown_button_res = Button::simple(Text::normal(format(&current))).show();

        if dropdown_button_res.clicked {
            open.modify(|v| !v);
        }

        if open.get() {
            let response = Dropdown::new(current, options).constraints(constraints).show(format);

            if !dropdown_button_res.hovering && (response.close_dropdown || response.new_value.is_some()) {
                open.set(false);
            }

            result = response.into_inner().new_value;
        }
    });
    result
}

#[track_caller]
pub fn radio<T: Eq>(current: &mut T, this: T, children: impl FnOnce()) -> Response<InteractiveResponse> {
    let hovered = use_state(|| false);

    let response = interactive(|| {
        row_cross_center(|| {
            Pad::horizontal(sizing::PADDING_MEDIUM).show(|| {
                Circle::new()
                    .min_radius(12.0f32)
                    .color(if hovered.get() {
                        colors::BACKGROUND_3.yak()
                    } else {
                        colors::BACKGROUND_2.yak()
                    })
                    .show_children(|| {
                        Pad::all(sizing::PADDING_SMALL).show(|| {
                            colored_circle(
                                if *current == this {
                                    colors::TEXT_ACTIVE.yak()
                                } else if hovered.get() {
                                    colors::BACKGROUND_3.yak()
                                } else {
                                    colors::BACKGROUND_2.yak()
                                },
                                6.0f32,
                            );
                        });
                    });
            });

            children();
        });
    });

    hovered.set(response.hovering);

    if response.clicked {
        *current = this;
    }

    response
}
