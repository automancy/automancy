use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn pause_menu(ctx: &mut UiContext) {
    Window::new(gui_str!(ctx.game_state, pause_menu_title)).show(Alignment::CENTER, || {
        col_cross_center(|| {
            if button(gui_str!(ctx.game_state, pause_menu_unpause_button)).clicked {
                ctx.gui.state.switch_screen(Screen::Ingame)
            };

            if button(gui_str!(ctx.game_state, pause_menu_options_button)).clicked {
                ctx.gui.state.switch_screen(Screen::Options(OptionsMenuState::default()))
            };

            if button(gui_str!(ctx.game_state, pause_menu_quit_to_menu_button)).clicked {
                ctx.game_state.game_handle.send_message(GameMsg::SaveMap).unwrap();

                ctx.game_state.load_map(ctx.game_data, GameMapId::MainMenu);
                ctx.gui.state.switch_screen(Screen::MainMenu);
            };
        });
    });
}
