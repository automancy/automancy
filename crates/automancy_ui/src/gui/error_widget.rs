use crate::*;

#[cfg_attr(feature = "profile", profiling::function)]
#[track_caller]
pub fn error_widget(ctx: &mut UiContext) {
    let Some(AutomancyError {
        id,
        message,
        ..
    }) = ErrorManager::peek_err()
    else {
        return;
    };

    Window::new(gui_str!(ctx.game_state, error_menu_title)).show(Alignment::CENTER, || {
        constrained(Constraints::loose(Vec2::new(ctx.gui.viewport().x * 0.6, f32::INFINITY)), || {
            col(|| {
                Text::normal(gui_str!(ctx.game_state, error_menu_label))
                    .color(colors::TEXT_WARNING.yak())
                    .show();

                Text::normal(gui_str!(
                    ctx.game_state,
                    error_menu_error_message,
                    [("err_msg", Formattable::display(&message))]
                ))
                .show();

                divider(colors::BACKGROUND_2.yak(), sizing::DIVIER_HEIGHT, sizing::DIVIER_THICKNESS);

                Text::normal(gui_str!(
                    ctx.game_state,
                    error_menu_error_id,
                    [("id_str", Formattable::display(&id))]
                ))
                .show();

                row_end(|| {
                    let error_read_button_res = button(gui_str!(ctx.game_state, error_menu_read_button));

                    if error_read_button_res.clicked {
                        ErrorManager::pop_err();
                    }
                });
            });
        });
    });
}
