use automancy_defs::{
    id::{ModelId, TileId},
    math::Matrix4,
    rendering::InstanceData,
};
use automancy_resources::data::DataMap;
use std::cell::{Cell, RefCell};
use wgpu::{BindGroup, Buffer};
use yakui::{
    paint::{CustomPaintCall, PaintCall},
    util::widget,
    widget::Widget,
    Rect, Response, Vec2,
};

thread_local! {
    static INDEX_COUNTER: Cell<usize> = const { Cell::new(0) };
    pub static SHOULD_RERENDER: Cell<bool> = const { Cell::new(true) };
}

pub fn reset_custom_paint_state() {
    INDEX_COUNTER.replace(0);
    SHOULD_RERENDER.set(false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiGameObjectType {
    Tile(TileId, DataMap),
    Model(ModelId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObject {
    pub index: usize,
    pub instance: InstanceData,
    pub ty: UiGameObjectType,
    pub size: Vec2,
    pub model_matrix: Matrix4,
    pub world_matrix: Matrix4,
}

pub fn ui_game_object(
    instance: InstanceData,
    ty: UiGameObjectType,
    size: Vec2,
    model_matrix: Option<Matrix4>,
    world_matrix: Option<Matrix4>,
) -> Response<()> {
    GameObject::new(instance, ty, size, model_matrix, world_matrix).show()
}

impl GameObject {
    pub fn new(
        instance: InstanceData,
        ty: UiGameObjectType,
        size: Vec2,
        model_matrix: Option<Matrix4>,
        world_matrix: Option<Matrix4>,
    ) -> Self {
        let index = INDEX_COUNTER.get();

        let result = Self {
            instance,
            ty,
            index,
            size,
            model_matrix: model_matrix.unwrap_or_default(),
            world_matrix: world_matrix.unwrap_or_default(),
        };

        INDEX_COUNTER.set(index + 1);

        result
    }

    #[track_caller]
    pub fn show(self) -> Response<()> {
        widget::<GameElementWidget>(Some(self))
    }
}

#[derive(Debug)]
pub struct GameElementPaint {
    pub props: GameObject,
    pub clip_offset: Vec2,
    pub clip_scale: Vec2,
    pub present_uniform: Option<Buffer>,
    pub present_bind_group: Option<BindGroup>,
}

#[derive(Debug, Clone)]
pub struct GameElementWidget {
    props: RefCell<Option<GameObject>>,
    clip: Cell<Rect>,
}

impl Widget for GameElementWidget {
    type Props<'a> = Option<GameObject>;
    type Response = ();

    fn new() -> Self {
        Self {
            props: RefCell::default(),
            clip: Cell::new(Rect::ZERO),
        }
    }

    fn update(&mut self, props: Self::Props<'_>) -> Self::Response {
        let old = self.props.get_mut();

        if !SHOULD_RERENDER.get() && old != &props {
            SHOULD_RERENDER.set(true);
        }

        *old = props;
    }

    fn layout(
        &self,
        _ctx: yakui::widget::LayoutContext<'_>,
        constraints: yakui::Constraints,
    ) -> Vec2 {
        if let Some(paint) = &*self.props.borrow() {
            constraints.constrain(paint.size)
        } else {
            constraints.min
        }
    }

    fn paint(&self, ctx: yakui::widget::PaintContext<'_>) {
        let paint_clip = ctx.paint.get_current_clip();

        if let Some(clip) = paint_clip {
            self.clip.set(clip);
        }

        let mut inside = Rect::ZERO;
        let mut clip_scale = Vec2::ONE;
        let mut clip_offset = Vec2::ZERO;
        if let Some(mut rect) = ctx.layout.get(ctx.dom.current()).map(|v| v.rect) {
            rect.set_pos(rect.pos() * ctx.layout.scale_factor());
            rect.set_size(rect.size() * ctx.layout.scale_factor());

            inside = ctx
                .layout
                .unscaled_viewport()
                .constrain(self.clip.get())
                .constrain(rect);

            clip_scale = inside.size() / rect.size();

            let inside_center = inside.pos() + inside.size() / 2.0;
            let rect_center = rect.pos() + rect.size() / 2.0;
            let sign = (inside_center - rect_center).signum();

            clip_offset = (sign + Vec2::ONE) * (Vec2::ONE - clip_scale) * rect.size();
        }

        if let Some(layer) = ctx.paint.layers_mut().current_mut() {
            let mut props = self.props.borrow().clone().unwrap();
            props.size *= ctx.layout.scale_factor();

            let paint = Box::new(GameElementPaint {
                props,
                clip_scale,
                clip_offset,
                present_bind_group: None,
                present_uniform: None,
            });

            layer.calls.push((
                PaintCall::Custom(CustomPaintCall { callback: paint }),
                Some(inside),
            ));
        }
    }
}
