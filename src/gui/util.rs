use std::sync::Arc;
use std::time::Instant;

use automancy_defs::{
    id::{Id, SharedStr},
    rendering::InstanceData,
};
use automancy_resources::types::IconMode;
use fuzzy_matcher::FuzzyMatcher;
use hashbrown::HashMap;
use yakui::{
    widgets::{Absolute, Layer, Pad},
    Alignment, Dim2, Pivot, Rect, Vec2,
};

use crate::{game::TAKE_ITEM_ANIMATION_SPEED, GameState};

use super::{col, radio, scroll_vertical, textbox, ui_game_object, TextField};

pub fn pad_y(top: f32, bottom: f32) -> Pad {
    let mut pad = Pad::ZERO;
    pad.top = top;
    pad.bottom = bottom;

    pad
}

pub fn pad_x(left: f32, right: f32) -> Pad {
    let mut pad = Pad::ZERO;
    pad.left = left;
    pad.right = right;

    pad
}

pub fn constrain_to_viewport(rect: &mut Rect, viewport: Rect) {
    rect.set_pos(rect.pos() - (rect.max() - viewport.max()).max(Vec2::ZERO))
}

pub fn clamp_percentage_to_viewport(size: Vec2, mut pos: Vec2, viewport: Rect) -> Vec2 {
    let mut rect = Rect::from_pos_size((pos * viewport.size()).floor(), size);

    constrain_to_viewport(&mut rect, viewport);

    pos = (rect.pos() / viewport.size()).clamp(Vec2::ZERO, Vec2::ONE);

    pos
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
        state.gui_state.text_field.get(field),
        None,
        hint_text.as_deref().map(Arc::<str>::as_ref),
    );

    scroll_vertical(Vec2::ZERO, Vec2::new(f32::INFINITY, 240.0), || {
        col(|| {
            let ids = if !state.gui_state.text_field.get(field).is_empty() {
                let text = state.gui_state.text_field.get(field).clone();
                let mut filtered = ids
                    .iter()
                    .flat_map(|id| {
                        let name = get_name(state, *id);
                        let score = state.gui_state.text_field.fuse.fuzzy_match(&name, &text);

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
                        state.resource_man.item_model_or_missing(id),
                        size,
                        Some(IconMode::Item.world_matrix()),
                    );
                });
            });
        }
    }
}
