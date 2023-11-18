use std::fs;
use std::sync::Arc;

use egui::Frame;
use ractor::concurrency::JoinHandle;
use ractor::{Actor, ActorRef};

use automancy::camera::Camera;
use automancy::game::{Game, GameMsg, TICK_INTERVAL};
use automancy::input::InputHandler;
use automancy::map::{Map, MapInfo, MAIN_MENU, MAP_PATH};
use automancy::options::Options;
use automancy_defs::coord::ChunkCoord;
use automancy_defs::log;
use automancy_defs::rendering::Vertex;
use automancy_resources::kira::manager::backend::cpal::CpalBackend;
use automancy_resources::kira::manager::{AudioManager, AudioManagerSettings};
use automancy_resources::kira::track::{TrackBuilder, TrackHandle};
use automancy_resources::{ResourceManager, RESOURCES_PATH, RESOURCE_MAN};

use crate::gui;

/// Initialize the Resource Manager system, and loads all the resources in all namespaces.
fn load_resources(track: TrackHandle) -> (Arc<ResourceManager>, Vec<Vertex>, Vec<u16>) {
    let mut resource_man = ResourceManager::new(track);

    fs::read_dir(RESOURCES_PATH)
        .expect("The resources folder doesn't exist- this is very wrong")
        .flatten()
        .map(|v| v.path())
        .for_each(|dir| {
            let namespace = dir.file_name().unwrap().to_str().unwrap();
            log::info!("Loading namespace {namespace}...");

            resource_man
                .load_models(&dir)
                .expect("Error loading models");
            resource_man.load_audio(&dir).expect("Error loading audio");
            resource_man.load_tiles(&dir).expect("Error loading tiles");
            resource_man.load_items(&dir).expect("Error loading items");
            resource_man.load_tags(&dir).expect("Error loading tags");
            resource_man
                .load_scripts(&dir)
                .expect("Error loading scripts");
            resource_man
                .load_translates(&dir)
                .expect("Error loading translates");
            resource_man
                .load_shaders(&dir)
                .expect("Error loading shaders");
            resource_man
                .load_functions(&dir)
                .expect("Error loading functions");
            resource_man
                .load_researches(&dir)
                .expect("Error loading researches");

            log::info!("Loaded namespace {namespace}.");
        });

    resource_man.compile_researches();
    resource_man.ordered_items();
    let (vertices, indices) = resource_man.compile_models();

    (Arc::new(resource_man), vertices, indices)
}

/// Stores what the game initializes on startup.
pub struct GameSetup {
    /// the audio manager
    pub audio_man: AudioManager,
    /// the resources manager
    pub resource_man: Arc<ResourceManager>,
    /// the game messaging system
    pub game: ActorRef<GameMsg>,
    /// the game's async handle, for graceful shutdown
    pub game_handle: Option<JoinHandle<()>>,
    /// the egui frame
    pub frame: Frame,
    /// the camera
    pub camera: Camera,
    /// the last camera position, in chunk coord
    pub camera_chunk_coord: ChunkCoord,
    /// the list of available maps
    pub maps: Vec<(MapInfo, String)>,
    /// the state of the input peripherals.
    pub input_handler: InputHandler,
    /// the game options
    pub options: Options,
}

impl GameSetup {
    /// Initializes the game, filling all the necessary fields as well as returns the loaded vertices and indices.
    pub async fn setup(camera: Camera) -> anyhow::Result<(Self, Vec<Vertex>, Vec<u16>)> {
        // --- resources & data ---
        log::info!("Initializing audio backend...");
        let mut audio_man = AudioManager::<CpalBackend>::new(AudioManagerSettings::default())?;
        let track = audio_man.add_sub_track({
            let builder = TrackBuilder::new();

            builder
        })?;
        log::info!("Audio backend initialized");

        log::info!("Loading resources...");
        let (resource_man, vertices, indices) = load_resources(track);
        RESOURCE_MAN.write().unwrap().replace(resource_man.clone());

        log::info!("Loaded resources.");

        // --- game ---
        log::info!("Creating game...");
        let (game, game_handle) = Actor::spawn(
            Some("game".to_string()),
            Game {
                resource_man: resource_man.clone(),
            },
            (),
        )
        .await?;

        game.send_message(GameMsg::LoadMap(
            resource_man.clone(),
            MAIN_MENU.to_string(),
        ))?;

        game.send_interval(TICK_INTERVAL, || GameMsg::Tick);

        log::info!("Game created.");

        log::info!("Loading options...");
        let options = Options::load()?;
        log::info!("Loaded options.");

        log::info!("Loading completed!");

        // --- last setup ---
        let frame = gui::default_frame();

        let camera_coord = camera.get_tile_coord();

        // --- event-loop ---
        Ok((
            GameSetup {
                audio_man,
                resource_man,
                game,
                game_handle: Some(game_handle),
                frame,
                camera,
                camera_chunk_coord: camera_coord.into(),
                maps: Vec::new(),
                input_handler: InputHandler::new(&options),
                options,
            },
            vertices,
            indices,
        ))
    }
    /// Refreshes the list of maps on the filesystem. Should be done every time the list of maps could have changed (on map creation/delete and on game load).
    pub fn refresh_maps(&mut self) {
        drop(fs::create_dir_all(MAP_PATH));

        self.maps = fs::read_dir(MAP_PATH)
            .expect("Map folder doesn't exist- is the disk full?")
            .flatten()
            .map(|f| f.file_name().to_str().unwrap().to_string())
            .filter(|f| !f.starts_with('.'))
            .flat_map(|map| {
                Map::read_header(&self.resource_man, &map)
                    .map(|v| MapInfo {
                        tile_count: v.0.tile_count,
                        save_time: v.1,
                    })
                    .zip(Some(map))
            })
            .collect::<Vec<_>>();

        self.maps.sort_by(|a, b| a.1.cmp(&b.1));
        self.maps.sort_by(|a, b| a.0.save_time.cmp(&b.0.save_time));
        self.maps.reverse();
    }
}
