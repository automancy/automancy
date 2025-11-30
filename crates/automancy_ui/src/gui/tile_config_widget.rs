use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn tile_config_widget(ctx: &mut UiContext) {
    let mut closed = false;

    let config_open = ctx.game_data.config_open.read_latest().clone();
    if config_open.is_none() {
        ctx.gui.state.tile_config_ui_state.clear();
        closed = true;
    }

    ctx.gui.state.tile_config_ui_state = Window::new(gui_str!(ctx.game_state, tile_config_menu_title))
        .scroll(Scrollable::vertical().child_size(Constraints {
            min: Vec2::new(320.0, 320.0),
            max: Vec2::new(f32::INFINITY, 360.0),
        }))
        .closed(closed)
        .show_movable(ctx.gui.state.tile_config_ui_state, || {
            section(|| {
                Pad::horizontal(sizing::PADDING_MEDIUM).show(|| {
                    col(|| {
                        if let Some((tile, data, ui)) = config_open {
                            rhai_ui(ctx, tile.handle.clone(), &data, &ui);
                        }
                    });
                });
            });
        });
}
