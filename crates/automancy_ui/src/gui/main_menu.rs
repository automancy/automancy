use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn main_menu(ctx: &mut UiContext) {
    Window::new(gui_str!(ctx.game_state, main_menu_title)).show(Alignment::CENTER, || {
        col_cross_center(|| {
            Image::new(ctx.gui.logo, Vec2::new(120.0, 120.0)).show();

            list_col_max()
                .cross_axis_alignment(CrossAxisAlignment::End)
                .item_spacing(sizing::PADDING_SMALL)
                .show(|| {
                    Margin::new(Vec2::new(300.0, sizing::PADDING_LARGE)).show();

                    if button(gui_str!(ctx.game_state, main_menu_play_button)).clicked {
                        ctx.game_state.refresh_maps_cache(ctx.game_data);
                        ctx.gui.state.switch_screen(Screen::MapLoad)
                    };

                    if button(gui_str!(ctx.game_state, main_menu_options_button)).clicked {
                        ctx.gui.state.switch_screen(Screen::Options(OptionsMenuState::default()))
                    };

                    divider(colors::BACKGROUND_3.yak(), sizing::DIVIER_HEIGHT, sizing::DIVIER_THICKNESS);

                    if button(gui_str!(ctx.game_state, main_menu_source_button)).clicked {
                        open::that_detached("https://github.com/automancy/automancy").unwrap();
                    }

                    if button(gui_str!(ctx.game_state, main_menu_discord_button)).clicked {
                        open::that_detached("https://discord.gg/pV7fqErE8F").unwrap();
                    }

                    if button(gui_str!(ctx.game_state, main_menu_exit_button)).clicked {
                        *ctx.closing = true
                    };
                });

            Text::small(pkg::version()).show();
        });
    });
}
