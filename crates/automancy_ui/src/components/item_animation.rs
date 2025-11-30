use crate::*;

thread_local! {
    #[allow(clippy::type_complexity)]
    static ITEM_ANIMATIONS: RefCell<HashMap<(UiRenderId, ItemId), (HashMap<(Instant, Duration), Rect>, Option<Rect>)>> = RefCell::default();
}

// TODO use frame start
// TODO simplify type

#[cfg_attr(feature = "profile", profiling::function)]
pub fn item_animation_src(ui_render_id: UiRenderId, item_id: ItemId, src_rect: Rect, duration: Duration) {
    ITEM_ANIMATIONS.with_borrow_mut(|animations| {
        let (anims, _) = animations.entry((ui_render_id, item_id)).or_default();

        anims.insert((Instant::now(), duration), src_rect);
    });
}

#[cfg_attr(feature = "profile", profiling::function)]
pub fn item_animation_dst(ui_render_id: UiRenderId, item_id: ItemId, dst_rect: Option<Rect>) {
    ITEM_ANIMATIONS.with_borrow_mut(|animations| {
        let (_, rect) = animations.entry((ui_render_id, item_id)).or_default();

        *rect = dst_rect;
    });
}

#[cfg_attr(feature = "profile", profiling::function)]
pub fn render_item_animations() {
    ITEM_ANIMATIONS.with_borrow_mut(|animations| {
        let now = Instant::now();

        let mut to_remove = Vec::new();
        for (&(render_id, item_id), (anims, dst_rect)) in animations.iter_mut() {
            let Some(dst_rect) = *dst_rect else {
                to_remove.push((render_id, item_id));
                continue;
            };

            {
                let mut to_remove = Vec::new();
                for &(instant, duration) in anims.keys() {
                    if now.duration_since(instant) >= duration {
                        to_remove.push((instant, duration));
                    }
                }
                for key in to_remove {
                    anims.remove(&key);
                }
            }

            for (&(instant, duration), &src_rect) in anims.iter() {
                let d = now.duration_since(instant).as_secs_f32() / duration.as_secs_f32();

                let pos = src_rect.pos().lerp(dst_rect.pos(), d);
                let size = src_rect.size().lerp(dst_rect.size(), d);

                reflow(Alignment::TOP_LEFT, Pivot::TOP_LEFT, Dim2::ZERO, || {
                    offset(pos, || {
                        GameModel::new(GenericModel::Item(item_id), size).show();
                    });
                });
            }
        }
        for id in to_remove {
            animations.remove(&id);
        }
    });
}
