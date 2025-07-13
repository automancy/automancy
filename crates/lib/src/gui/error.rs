use automancy_resources::error::{error_to_key, peek_err, pop_err};
use automancy_ui::{button, label, row_max, window};
use yakui::{spacer, widgets::Layer};

use crate::GameState;

/// Draws an error popup. Can only be called when there are errors in the queue!
pub fn error_popup(state: &mut GameState) {
    if let Some((id, err)) = peek_err() {
        Layer::new().show(|| {
            window(
                state
                    .resource_man
                    .gui_str(state.resource_man.registry.gui_ids.error_popup)
                    .to_string(),
                || {
                    label(&format!("ID: {}", error_to_key(id, &state.resource_man)));

                    label(&err);

                    row_max(|| {
                        spacer(1);

                        if button(
                            &state
                                .resource_man
                                .gui_str(state.resource_man.registry.gui_ids.btn_confirm),
                        )
                        .clicked
                        {
                            pop_err();
                        }
                    });
                },
            );
        });
    }
}
