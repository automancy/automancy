use crate::renderer::GameRenderer;
use crate::GameState;
use automancy_defs::coord::TileCoord;
use automancy_defs::id::{ModelId, TileId};
use automancy_defs::math::Matrix4;
use automancy_defs::rendering::GameMatrix;
use automancy_defs::{
    id::{Id, SharedStr},
    rendering::InstanceData,
};
use automancy_resources::data::DataMap;
use automancy_resources::rhai_render::RenderCommand;
use automancy_resources::types::IconMode;
use automancy_resources::ResourceManager;
use automancy_system::game::TAKE_ITEM_ANIMATION_SPEED;
use automancy_system::tile_entity::collect_render_commands;
use automancy_system::ui_state::TextField;
use automancy_ui::{
    col, group, hover_tip, radio, scroll_vertical, textbox, ui_game_object, UiGameObjectType,
    HOVER_TIP,
};
use fuzzy_matcher::FuzzyMatcher;
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use yakui::{constrained, Constraints};
use yakui::{
    widgets::{Absolute, Layer},
    Alignment, Dim2, Pivot, Rect, Vec2,
};

pub fn render_overlay_cached(
    resource_man: &ResourceManager,
    renderer: &mut GameRenderer,
    tile_id: Option<TileId>,
    mut data: DataMap,
    cache: &mut Option<(TileId, Vec<ModelId>)>,
    model_matrix: Matrix4,
    world_matrix: Matrix4,
) {
    if let Some(tile_id) = tile_id {
        let mut transforms = HashMap::new();

        let cached_tile_id = cache.as_ref().map(|v| v.0);

        if cached_tile_id != Some(tile_id) {
            if let Some(commands) = collect_render_commands(
                resource_man,
                tile_id,
                TileCoord::ZERO,
                &mut data,
                &mut HashSet::default(),
                true,
                false,
            ) {
                transforms = commands
                    .iter()
                    .flat_map(|v| match v {
                        RenderCommand::Transform {
                            model,
                            model_matrix,
                            ..
                        } => Some((*model, *model_matrix)),
                        _ => None,
                    })
                    .collect::<HashMap<_, _>>();

                let models = commands
                    .into_iter()
                    .flat_map(|v| match v {
                        RenderCommand::Track { model, .. } => Some(model),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                *cache = Some((tile_id, models));
            }
        }

        if let Some((.., models)) = &cache {
            for model in models {
                let transform = transforms.remove(model).unwrap_or_default();

                let (model, (meshes, ..)) = resource_man.mesh_or_missing_tile_mesh(model);

                for mesh in meshes.iter().flatten() {
                    renderer.overlay_instances.push((
                        InstanceData::default().with_alpha(0.6),
                        model,
                        GameMatrix::<true>::new(
                            transform * model_matrix,
                            world_matrix,
                            mesh.matrix,
                        ),
                        mesh.index,
                    ));
                }
            }
        }
    }
}

/// Draws a search bar.
pub fn searchable_id(
    state: &mut GameState,
    ids: &[Id],
    new_id: &mut Option<Id>,
    field: TextField,
    hint_text: Option<SharedStr>,
    draw: impl Fn(&mut GameState, Id),
    get_name: impl Fn(&mut GameState, Id) -> SharedStr,
) {
    textbox(
        state.ui_state.text_field.get(field),
        None,
        hint_text.as_deref().map(Arc::<str>::as_ref),
    );

    scroll_vertical(Vec2::ZERO, Vec2::new(f32::INFINITY, 240.0), || {
        group(|| {
            col(|| {
                let ids = if !state.ui_state.text_field.get(field).is_empty() {
                    let text = state.ui_state.text_field.get(field).clone();
                    let mut filtered = ids
                        .iter()
                        .flat_map(|id| {
                            let name = get_name(state, *id);
                            let score = state.ui_state.text_field.fuse.fuzzy_match(&name, &text);

                            if score.unwrap_or(0) < (name.len() / 2) as i64 {
                                None
                            } else {
                                Some(*id).zip(score)
                            }
                        })
                        .collect::<Vec<_>>();

                    filtered.sort_unstable_by(|a, b| a.1.cmp(&b.1));

                    filtered.into_iter().rev().map(|v| v.0).collect::<Vec<_>>()
                } else {
                    ids.to_vec()
                };

                for id in ids.iter() {
                    radio(new_id, Some(*id), || {
                        draw(state, *id);
                    });
                }
            });
        });
    });
}

pub fn take_item_animation(state: &mut GameState, id: Id, dst_rect: Rect) {
    let now = Instant::now();

    let mut to_remove = HashMap::new();

    for (coord, deque) in &state.renderer.as_ref().unwrap().take_item_animations {
        to_remove.insert(
            *coord,
            deque
                .iter()
                .take_while(|(instant, _)| {
                    now.duration_since(*instant) >= TAKE_ITEM_ANIMATION_SPEED
                })
                .count(),
        );
    }

    for (coord, v) in to_remove {
        for _ in 0..v {
            state
                .renderer
                .as_mut()
                .unwrap()
                .take_item_animations
                .get_mut(&coord)
                .unwrap()
                .pop_front();
        }
    }

    if let Some(animations) = state
        .renderer
        .as_ref()
        .unwrap()
        .take_item_animations
        .get(&id)
    {
        for (instant, src_rect) in animations {
            let d = now.duration_since(*instant).as_secs_f32()
                / TAKE_ITEM_ANIMATION_SPEED.as_secs_f32();

            let pos = src_rect.pos().lerp(dst_rect.pos(), d);
            let size = src_rect.size().lerp(dst_rect.size(), d);

            Absolute::new(
                Alignment::TOP_LEFT,
                Pivot::TOP_LEFT,
                Dim2::pixels(pos.x, pos.y),
            )
            .show(|| {
                Layer::new().show(|| {
                    ui_game_object(
                        InstanceData::default(),
                        UiGameObjectType::Model(state.resource_man.item_model_or_missing(&id)),
                        size,
                        None,
                        Some(IconMode::Item.world_matrix()),
                    );
                });
            });
        }
    }
}

pub fn render_info_tip(state: &mut GameState) {
    if let Some(tip) = HOVER_TIP.take() {
        Layer::new().show(|| {
            hover_tip(|| {
                constrained(
                    Constraints::loose(state.ui_viewport().min(Vec2::new(500.0, f32::INFINITY))),
                    || {
                        tip.show();
                    },
                );
            });
        });
    }
}
