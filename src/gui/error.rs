use automancy_resources::error::{error_to_key, error_to_string};
use yakui::{row, spacer};

use crate::GameState;

use super::{button, label, window};

/// Draws an error popup. Can only be called when there are errors in the queue!
pub fn error_popup(state: &mut GameState) {
    if let Some(error) = state.resource_man.error_man.peek() {
        window(
            state
                .resource_man
                .gui_str(&state.resource_man.registry.gui_ids.error_popup)
                .to_string(),
            || {
                label(&format!(
                    "ID: {}",
                    error_to_key(&error, &state.resource_man)
                ));
                label(&error_to_string(&error, &state.resource_man));

                row(|| {
                    spacer(1);

                    if button(
                        state
                            .resource_man
                            .gui_str(&state.resource_man.registry.gui_ids.btn_confirm)
                            .as_str(),
                    )
                    .clicked
                    {
                        state.resource_man.error_man.pop();
                    }
                });
            },
        );
    }
}
