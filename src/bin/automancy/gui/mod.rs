use egui::epaint::Shadow;
use egui::{FontData, FontDefinitions, Frame, Margin, Rounding, ScrollArea, Ui};
use enum_map::{enum_map, Enum, EnumMap};
use fuse_rust::Fuse;
use std::sync::Arc;

use automancy_defs::colors;
use automancy_defs::gui::Gui;
use automancy_defs::id::Id;
use automancy_resources::ResourceManager;

#[cfg(debug_assertions)]
pub mod debug;

pub mod error;
pub mod info;
pub mod item;
pub mod menu;
pub mod player;
pub mod popup;
pub mod tile_config;
pub mod tile_selection;

pub struct GuiState {
    pub screen: Screen,
    pub substate: SubState,
    pub popup: PopupState,
    pub show_debugger: bool,
    pub previous: Option<Screen>,
    pub text_field: TextFieldState,
}

/// The state of the main game GUI.
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Screen {
    MainMenu,
    MapLoad,
    Options,
    Ingame,
    Paused,
    Research,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum SubState {
    None,
    Options(OptionsMenuState),
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum OptionsMenuState {
    Graphics,
    Audio,
    Gui,
    Controls,
}

/// The state of popups (which are on top of the main GUI), if any should be displayed.
#[derive(Eq, PartialEq, Clone)]
pub enum PopupState {
    None,
    MapCreate,
    MapDeleteConfirmation(String),
    InvalidName,
}

/// Creates a default frame.
pub fn default_frame() -> Frame {
    Frame::none()
        .fill(colors::WHITE.multiply(0.65).into())
        .shadow(Shadow {
            extrusion: 8.0,
            color: colors::DARK_GRAY.multiply(0.5).into(),
        })
        .rounding(Rounding::same(5.0))
        .inner_margin(Margin::same(10.0))
}

impl Default for GuiState {
    fn default() -> Self {
        GuiState {
            screen: Screen::MainMenu,
            substate: SubState::None,
            popup: PopupState::None,
            show_debugger: false,
            previous: None,
            text_field: Default::default(),
        }
    }
}

impl GuiState {
    pub fn return_screen(&mut self) {
        if let Some(prev) = self.previous {
            self.screen = prev;
        }
        self.previous = None;
    }

    pub fn switch_screen(&mut self, new: Screen) {
        self.previous = Some(self.screen);
        self.screen = new;
    }

    pub fn switch_screen_sub(&mut self, new: Screen, sub: SubState) {
        self.switch_screen(new);
        self.substate = sub;
    }

    pub fn switch_screen_when(
        &mut self,
        when: &'static dyn Fn(&mut GuiState) -> bool,
        new: Screen,
    ) -> bool {
        if when(self) {
            self.switch_screen(new);

            true
        } else {
            false
        }
    }
}
#[derive(Eq, PartialEq, Ord, PartialOrd, Enum, Clone, Copy)]
pub enum TextField {
    Filter,
    MapRenaming,
    MapName,
}

pub struct TextFieldState {
    pub fuse: Fuse,
    pub map_name_renaming: Option<String>,
    fields: EnumMap<TextField, String>,
}

impl Default for TextFieldState {
    fn default() -> Self {
        TextFieldState {
            fuse: Fuse::default(),
            map_name_renaming: None,
            fields: enum_map! {
                TextField::Filter => String::new(),
                TextField::MapName => String::new(),
                TextField::MapRenaming => String::new()
            },
        }
    }
}

impl TextFieldState {
    pub fn get(&mut self, field: TextField) -> &mut String {
        &mut self.fields[field]
    }
    /// Draws a search bar.
    pub fn searchable_id<'a>(
        &mut self,
        ui: &mut Ui,
        resource_man: &'a ResourceManager,
        ids: &[Id],
        new_id: &mut Option<Id>,
        field: TextField,
        name: &'static impl Fn(&'a ResourceManager, &Id) -> &'a str,
    ) {
        ui.text_edit_singleline(self.get(field));

        ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
            ui.set_width(ui.available_width());

            let ids = if !self.get(field).is_empty() {
                let text = self.get(field).clone();
                let mut filtered = ids
                    .iter()
                    .flat_map(|id| {
                        let result = self
                            .fuse
                            .search_text_in_string(&text, name(resource_man, id));
                        let score = result.map(|v| v.score);

                        if score.unwrap_or(0.0) > 0.4 {
                            None
                        } else {
                            Some(*id).zip(score)
                        }
                    })
                    .collect::<Vec<_>>();
                filtered.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

                filtered.into_iter().map(|v| v.0).collect::<Vec<_>>()
            } else {
                ids.to_vec()
            };

            for id in ids {
                ui.radio_value(new_id, Some(id), name(resource_man, &id));
            }
        });
    }
}

// TODO i should really find a better place for this. oh well!
pub fn init_fonts(resource_man: Arc<ResourceManager>, gui: &mut Gui) {
    gui.fonts = FontDefinitions::default();
    for (name, font) in resource_man.fonts.iter() {
        gui.fonts
            .font_data
            .insert(name.to_string(), FontData::from_owned(font.data.to_owned()));
    }
}
