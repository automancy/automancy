use std::collections::VecDeque;
use std::f32::consts::PI;
use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::Rect;
use fuse_rust::Fuse;
use futures::channel::mpsc;
use futures::executor::block_on;
use wgpu::SurfaceError;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use automancy::game::{GameMsg, PlaceTileResponse};
use automancy::input;
use automancy::input::KeyActions;
use automancy::tile_entity::{TileEntityMsg, TileModifier};
use automancy_defs::cgmath::{point2, vec3};
use automancy_defs::colors::ColorAdj;
use automancy_defs::coord::{ChunkCoord, TileCoord};
use automancy_defs::gui::Gui;
use automancy_defs::hashbrown::{HashMap, HashSet};
use automancy_defs::id::Id;
use automancy_defs::math::{Float, Matrix4, FAR};
use automancy_defs::rendering::{make_line, InstanceData};
use automancy_defs::{colors, log, math, window};
use automancy_resources::data::item::Item;
use automancy_resources::data::Data;

use crate::gui::{
    debug, error, info, menu, player, popup, tile_config, tile_selection, GuiState, PopupState,
};
use crate::renderer::Renderer;
use crate::setup::GameSetup;

/// Stores information that lives for the entire lifetime of the session, and is not dropped at the end of one event cycle or handled elsewhere.
pub struct EventLoopStorage {
    /// fuzzy search engine
    pub fuse: Fuse,
    // TODO most of the following elements should be moved out of here...
    /// the filter for the scripts.
    pub filter_input: String,
    /// input for the map name
    pub map_name_input: String,
    /// storage which map to rename
    pub map_name_renaming: Option<String>,
    /// input for map renaming
    pub map_name_renaming_input: String,
    /// the tile states of the selected tiles.
    pub selected_tile_modifiers: HashMap<Id, TileModifier>,
    /// the currently selected tile.
    pub selected_id: Option<Id>,
    /// the last placed tile, to prevent repeatedly sending place requests
    pub already_placed_at: Option<TileCoord>,
    /// the tile that has its config menu open.
    pub config_open: Option<TileCoord>,
    /// tag searching cache
    pub tag_cache: HashMap<Id, Arc<Vec<Item>>>,
    /// tile currently linking
    pub linking_tile: Option<TileCoord>,
    /// the last frame's starting time
    pub frame_start: Instant,
    /// the elapsed time between each frame
    pub elapsed: Duration,
    /// the currently selected tiles
    pub selected_tiles: HashSet<TileCoord>,
    /// the stored initial cursor position, for moving tiles
    pub initial_cursor_position: Option<TileCoord>,

    pub take_item_animations: HashMap<Item, VecDeque<(Instant, Rect)>>,

    prev_gui_state: Option<GuiState>,
    gui_state: GuiState,
    pub popup_state: PopupState,
    pub show_debugger: bool,
}

impl Default for EventLoopStorage {
    fn default() -> Self {
        Self {
            fuse: Default::default(),
            filter_input: "".to_string(),
            map_name_input: "".to_string(),
            map_name_renaming: None,
            map_name_renaming_input: "".to_string(),
            selected_tile_modifiers: Default::default(),
            selected_id: None,
            already_placed_at: None,
            config_open: None,
            tag_cache: Default::default(),
            linking_tile: None,
            frame_start: Instant::now(),
            elapsed: Default::default(),
            selected_tiles: Default::default(),
            initial_cursor_position: None,
            take_item_animations: Default::default(),

            prev_gui_state: None,
            gui_state: GuiState::MainMenu,
            popup_state: PopupState::None,
            show_debugger: false,
        }
    }
}

impl EventLoopStorage {
    pub fn return_gui_state(&mut self) {
        if let Some(prev) = self.prev_gui_state {
            self.gui_state = prev;
        }
        self.prev_gui_state = None;
    }

    pub fn switch_gui_state(&mut self, new: GuiState) {
        self.prev_gui_state = Some(self.gui_state);
        self.gui_state = new;
    }

    pub fn switch_gui_state_when(
        &mut self,
        when: &'static dyn Fn(GuiState) -> bool,
        new: GuiState,
    ) -> bool {
        if when(self.gui_state) {
            self.switch_gui_state(new);

            true
        } else {
            false
        }
    }
}

pub fn shutdown_graceful(
    setup: &mut GameSetup,
    control_flow: &mut ControlFlow,
) -> anyhow::Result<bool> {
    setup.game.send_message(GameMsg::StopTicking)?;

    block_on(setup.game.call(
        |reply| GameMsg::SaveMap(setup.resource_man.clone(), reply),
        None,
    ))
    .unwrap();

    setup.game.stop(Some("Game closed".to_string()));

    block_on(setup.game_handle.take().unwrap())?;

    control_flow.set_exit();

    log::info!("Shut down gracefully");

    Ok(true)
}

fn render(
    setup: &mut GameSetup,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
    gui: &mut Gui,
    control_flow: &mut ControlFlow,
) -> anyhow::Result<bool> {
    setup.camera.update_pointing_at(
        setup.input_handler.main_pos,
        window::window_size_double(&renderer.gpu.window),
    );

    setup.camera.update_pos(
        window::window_size_double(&renderer.gpu.window),
        loop_store.elapsed.as_secs_f64(),
    );

    let mut result = Ok(false);

    let mut tile_tints = HashMap::new();

    let mut extra_instances = vec![];
    let mut overlay_instances = vec![];
    let mut in_world_item_instances = vec![];
    let mut gui_instances = vec![];
    let mut item_instances = vec![];

    let (width, height) = window::window_size_float(&renderer.gpu.window);
    let aspect = width / height;
    let camera_pos_float = setup.camera.get_pos().cast().unwrap();
    let matrix = math::matrix(camera_pos_float, aspect, PI);

    let (selection_send, mut selection_recv) = mpsc::channel(1);

    loop_store.frame_start = Instant::now();

    {
        gui.context
            .begin_frame(gui.state.take_egui_input(&renderer.gpu.window));

        if setup.input_handler.key_active(KeyActions::Debug) {
            gui.context.set_debug_on_hover(true);

            debug::debugger(setup, loop_store, &gui.context);
        } else {
            gui.context.set_debug_on_hover(false);
        }

        if loop_store.popup_state == PopupState::None {
            match loop_store.gui_state {
                GuiState::Ingame => {
                    if !setup.input_handler.key_active(KeyActions::HideGui) {
                        if setup.input_handler.key_active(KeyActions::Player) {
                            player::player(setup, loop_store, &mut item_instances, &gui.context);
                        }

                        // tile_info
                        info::info(setup, &mut item_instances, &gui.context);

                        // tile_config
                        tile_config::tile_config(
                            setup,
                            loop_store,
                            &mut item_instances,
                            &gui.context,
                        );

                        // tile_selections
                        tile_selection::tile_selections(
                            setup,
                            &mut gui_instances,
                            &gui.context,
                            &loop_store.selected_tile_modifiers,
                            selection_send,
                        );

                        if let Ok(Some(id)) = selection_recv.try_next() {
                            loop_store.already_placed_at = None;

                            if loop_store.selected_id == Some(id) {
                                loop_store.selected_id = None;
                            } else {
                                loop_store.selected_id = Some(id);
                            }
                        }

                        let cursor_pos = math::screen_to_world(
                            window::window_size_double(&renderer.gpu.window),
                            setup.input_handler.main_pos,
                            setup.camera.get_pos(),
                        );
                        let cursor_pos = point2(cursor_pos.x, cursor_pos.y);

                        if let Some(id) = loop_store.selected_id {
                            if let Some(model) =
                                setup.resource_man.registry.tile(id).and_then(|v| {
                                    v.models
                                        .get(
                                            loop_store
                                                .selected_tile_modifiers
                                                .get(&id)
                                                .cloned()
                                                .unwrap_or(0)
                                                as usize,
                                        )
                                        .cloned()
                                })
                            {
                                overlay_instances.push((
                                    InstanceData {
                                        alpha: 0.6,
                                        light_pos: camera_pos_float,
                                        model_matrix: Matrix4::from_translation(vec3(
                                            cursor_pos.x as Float,
                                            cursor_pos.y as Float,
                                            FAR as Float,
                                        )),
                                        ..Default::default()
                                    },
                                    model,
                                ));
                            }
                        }

                        if let Some(coord) = loop_store.linking_tile {
                            extra_instances.push((
                                InstanceData {
                                    color_offset: colors::RED.to_array(),
                                    light_pos: camera_pos_float,
                                    model_matrix: make_line(math::hex_to_pixel(*coord), cursor_pos),
                                    ..Default::default()
                                },
                                setup.resource_man.registry.model_ids.cube1x1,
                            ));
                        }
                    }
                }
                GuiState::MainMenu => {
                    result = menu::main_menu(setup, &gui.context, control_flow, loop_store)
                }
                GuiState::MapLoad => {
                    menu::map_menu(setup, &gui.context, loop_store);
                }
                GuiState::Options => {
                    menu::options_menu(setup, &gui.context, loop_store);
                }
                GuiState::Paused => {
                    menu::pause_menu(setup, &gui.context, loop_store);
                }
                GuiState::Research => {}
            }
        }

        match loop_store.popup_state.clone() {
            PopupState::None => {}
            PopupState::MapCreate => popup::map_create_popup(setup, gui, loop_store),
            PopupState::MapDeleteConfirmation(map_name) => {
                popup::map_delete_popup(setup, gui, loop_store, &map_name);
            }
            PopupState::InvalidName => {
                popup::invalid_name_popup(setup, gui, loop_store);
            }
        }

        tile_tints.insert(setup.camera.pointing_at, colors::RED.with_alpha(0.2));

        for selected in &loop_store.selected_tiles {
            tile_tints.insert(*selected, colors::ORANGE.with_alpha(0.4));
        }

        if setup.input_handler.control_held {
            if let Some(start) = loop_store.initial_cursor_position {
                let direction = setup.camera.pointing_at - start;

                extra_instances.push((
                    InstanceData {
                        color_offset: colors::LIGHT_BLUE.to_array(),
                        light_pos: camera_pos_float,
                        model_matrix: make_line(
                            math::hex_to_pixel(*start),
                            math::hex_to_pixel(*setup.camera.pointing_at),
                        ),
                        ..Default::default()
                    },
                    setup.resource_man.registry.model_ids.cube1x1,
                ));

                for selected in &loop_store.selected_tiles {
                    let dest = *selected + direction;
                    tile_tints.insert(dest, colors::LIGHT_BLUE.with_alpha(0.3));
                }
            }
        }

        error::error_popup(setup, gui);

        if !matches!(result, Ok(true)) {
            match renderer.render(
                setup,
                gui,
                matrix,
                tile_tints,
                extra_instances,
                overlay_instances,
                in_world_item_instances,
                gui_instances,
                item_instances,
            ) {
                Ok(_) => {}
                Err(SurfaceError::Lost) => renderer
                    .gpu
                    .create_textures(renderer.gpu.window.inner_size()),
                Err(SurfaceError::OutOfMemory) => {
                    return shutdown_graceful(setup, control_flow);
                }
                Err(e) => log::error!("{e:?}"),
            }

            loop_store.elapsed = Instant::now().duration_since(loop_store.frame_start);
        }
    }

    result
}

/// Triggers every time the event loop is run once.
pub fn on_event(
    setup: &mut GameSetup,
    loop_store: &mut EventLoopStorage,
    renderer: &mut Renderer,
    gui: &mut Gui,
    event: Event<()>,
    control_flow: &mut ControlFlow,
) -> anyhow::Result<bool> {
    let mut window_event = None;
    let mut device_event = None;

    let resource_man = setup.resource_man.clone();
    match &event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            // game shutdown
            return shutdown_graceful(setup, control_flow);
        }

        Event::WindowEvent { event, .. } => {
            if !gui.state.on_event(&gui.context, event).consumed {
                window_event = Some(event);
            }

            match event {
                WindowEvent::Resized(size) => {
                    renderer.gpu.create_textures(*size);

                    return Ok(false);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    renderer.gpu.create_textures(**new_inner_size);

                    return Ok(false);
                }
                _ => {}
            }
        }

        Event::DeviceEvent { event, .. } => {
            device_event = Some(event);
        }

        Event::MainEventsCleared => {
            renderer.gpu.window.request_redraw();
            return Ok(false);
        }

        _ => {}
    };

    if event == Event::RedrawRequested(renderer.gpu.window.id()) {
        return render(setup, loop_store, renderer, gui, control_flow);
    }

    if window_event.is_some() || device_event.is_some() {
        setup.input_handler.reset();
        setup.input_handler.update(input::convert_input(
            window_event,
            device_event,
            window::window_size_double(&renderer.gpu.window),
            1.0, //TODO sensitivity option
        ));

        let ignore_move = loop_store.selected_id.is_some();

        setup.camera.handle_input(&setup.input_handler, ignore_move);

        {
            let camera_chunk_coord: ChunkCoord = setup.camera.get_tile_coord().into();

            if setup.camera_chunk_coord != camera_chunk_coord {
                setup.camera_chunk_coord = camera_chunk_coord;

                // camera chunk coord update logic ...
            }
        }

        if setup.input_handler.key_active(KeyActions::Escape) {
            // one by one
            if loop_store.selected_id.take().is_none() && loop_store.linking_tile.take().is_none() {
                if loop_store.switch_gui_state_when(&|s| s == GuiState::Ingame, GuiState::Paused) {
                    block_on(setup.game.call(
                        |reply| GameMsg::SaveMap(setup.resource_man.clone(), reply),
                        None,
                    ))?
                    .unwrap();
                } else {
                    loop_store.switch_gui_state_when(&|s| s == GuiState::Paused, GuiState::Ingame);
                }

                if loop_store.gui_state == GuiState::Research {
                    loop_store.return_gui_state();
                }
            }
        }

        if setup.input_handler.main_pressed
            || (setup.input_handler.shift_held && setup.input_handler.main_held)
        {
            if let Some(id) = loop_store.selected_id {
                if loop_store.already_placed_at != Some(setup.camera.pointing_at) {
                    let response = block_on(setup.game.call(
                        |reply| GameMsg::PlaceTile {
                            coord: setup.camera.pointing_at,
                            id,
                            tile_modifier:
                                *loop_store.selected_tile_modifiers.get(&id).unwrap_or(&0),
                            record: true,
                            reply: Some(reply),
                        },
                        None,
                    ))?
                    .unwrap();

                    match response {
                        PlaceTileResponse::Placed => {
                            setup
                                .audio_man
                                .play(resource_man.audio["tile_placement"].clone())
                                .unwrap();
                        }
                        PlaceTileResponse::Removed => {
                            setup
                                .audio_man
                                .play(resource_man.audio["tile_removal"].clone())
                                .unwrap();
                        }
                        _ => {}
                    }

                    loop_store.already_placed_at = Some(setup.camera.pointing_at)
                }
            }
        }

        if !setup.input_handler.control_held && setup.input_handler.alternate_pressed {
            if let Some(linking_tile) = loop_store.linking_tile {
                let tile = block_on(setup.game.call(
                    |reply| GameMsg::GetTile(setup.camera.pointing_at, reply),
                    None,
                ))?
                .unwrap();

                let tile_entity = block_on(setup.game.call(
                    |reply| GameMsg::GetTileEntity(setup.camera.pointing_at, reply),
                    None,
                ))?
                .unwrap();

                if let Some((linked, tile_entity)) = tile
                    .and_then(|(id, _)| {
                        resource_man
                            .registry
                            .tile_data(id, resource_man.registry.data_ids.linked)
                            .and_then(Data::as_bool)
                            .cloned()
                    })
                    .zip(tile_entity)
                {
                    if linked {
                        let old = block_on(tile_entity.call(
                            |reply| {
                                TileEntityMsg::GetDataValue(
                                    resource_man.registry.data_ids.link,
                                    reply,
                                )
                            },
                            None,
                        ))?
                        .unwrap();

                        if old.is_some() {
                            tile_entity.send_message(TileEntityMsg::RemoveData(
                                resource_man.registry.data_ids.link,
                            ))?;

                            setup.audio_man.play(resource_man.audio["click"].clone())?;
                            // TODO click2
                        } else {
                            tile_entity.send_message(TileEntityMsg::SetDataValue(
                                resource_man.registry.data_ids.link,
                                Data::Coord(linking_tile),
                            ))?;

                            setup.audio_man.play(resource_man.audio["click"].clone())?;
                        }
                    }
                }
            } else if let Some(id) = loop_store.selected_id {
                let new = loop_store.selected_tile_modifiers.get(&id).unwrap_or(&0) + 1;
                let max = resource_man.registry.tile(id).unwrap().models.len() as TileModifier;

                loop_store.selected_tile_modifiers.insert(id, new % max);
                loop_store.already_placed_at = None;

                setup.audio_man.play(resource_man.audio["click"].clone())?;
            } else if loop_store.config_open == Some(setup.camera.pointing_at) {
                loop_store.config_open = None;
                loop_store.filter_input.clear();
            } else {
                loop_store.config_open = Some(setup.camera.pointing_at);
            }
        }

        if setup.input_handler.control_held && loop_store.gui_state == GuiState::Ingame {
            if let Some(start) = loop_store.initial_cursor_position {
                if setup.input_handler.tertiary_pressed {
                    let direction = setup.camera.pointing_at - start;

                    setup.game.send_message(GameMsg::MoveTiles(
                        loop_store
                            .selected_tiles
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>(),
                        direction,
                        true,
                    ))?;

                    let cap = loop_store.selected_tiles.capacity();
                    for selected in
                        mem::replace(&mut loop_store.selected_tiles, HashSet::with_capacity(cap))
                    {
                        let dest = selected + direction;

                        loop_store.selected_tiles.insert(dest);
                    }

                    loop_store.initial_cursor_position = None;
                    setup.audio_man.play(resource_man.audio["click"].clone())?; // TODO click2
                }
            } else if setup.input_handler.tertiary_pressed {
                loop_store.initial_cursor_position = Some(setup.camera.pointing_at);
                setup.audio_man.play(resource_man.audio["click"].clone())?;
            }

            if loop_store.initial_cursor_position.is_none() {
                loop_store.selected_tiles.insert(setup.camera.pointing_at);
            }
        } else {
            loop_store.selected_tiles.clear();
            loop_store.initial_cursor_position = None;
        }

        if setup.input_handler.control_held && setup.input_handler.key_active(KeyActions::Undo) {
            setup.game.send_message(GameMsg::Undo)?;
        }
    }

    return Ok(false);
}
