use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use automancy_defs::{
    id::{ModelId, TileId},
    math::Matrix4,
    rendering::InstanceData,
};
use automancy_resources::data::DataMap;
use wgpu::{BindGroup, Buffer};
use yakui::{
    Rect, Response, Vec2,
    paint::{PaintCall, PaintLayer, UserPaintCallId},
    util::widget,
    widget::Widget,
};

use crate::custom::{mark_rerender, new_user_paint_id, should_rerender};

#[derive(Debug, Clone, PartialEq)]
pub enum UiGameObjectType {
    Tile(TileId, DataMap),
    Model(ModelId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObject {
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
        Self {
            instance,
            ty,
            size,
            model_matrix: model_matrix.unwrap_or_default(),
            world_matrix: world_matrix.unwrap_or_default(),
        }
    }

    #[track_caller]
    pub fn show(self) -> Response<()> {
        widget::<GameElementWidget>(Some(self))
    }
}

#[derive(Debug, Clone)]
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

        if !should_rerender() && old != &props {
            mark_rerender();
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
        if let Some(clip) = ctx.layout.get(ctx.dom.current()).map(|v| v.clip) {
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

        if let Some(layer) = ctx.paint.layers.current_mut() {
            let mut props = self.props.borrow().clone().unwrap();
            props.size *= ctx.layout.scale_factor();

            ctx.dom.get_global_or_init(GameObjectRenderer::default).add(
                layer,
                GameElementPaint {
                    props,
                    clip_scale,
                    clip_offset,
                    present_bind_group: None,
                    present_uniform: None,
                },
                inside,
            );
        }
    }
}

#[derive(Debug, Default, Clone)]
struct GameObjectRenderer {
    objects: BTreeMap<UserPaintCallId, GameElementPaint>,
}

impl GameObjectRenderer {
    pub fn add(&mut self, layer: &mut PaintLayer, object: GameElementPaint, clip: Rect) {
        let id = new_user_paint_id();

        self.objects.insert(id, object);

        layer.calls.push((clip, PaintCall::User(id)));
    }
}
