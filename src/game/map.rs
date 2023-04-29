use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::{collections::HashMap, path::PathBuf};

use futures_executor::block_on;
use riker::actor::ActorRef;
use riker::actors::{ActorSystem, Context};
use riker_patterns::ask::ask;
use serde::{Deserialize, Serialize};
use zstd::{Decoder, Encoder};

use crate::game::tile::coord::TileCoord;
use crate::game::tile::entity::TileEntityMsg::{GetData, SetData};
use crate::game::tile::entity::{
    data_from_raw, data_to_raw, DataMap, DataMapRaw, TileEntityMsg, TileState,
};
use crate::game::{Game, GameMsg};
use crate::resource::ResourceManager;
use crate::util::id::{Id, Interner};

pub const MAP_PATH: &str = "map";

const MAP_BUFFER_SIZE: usize = 256 * 1024;

pub type Tiles = HashMap<TileCoord, (Id, TileState)>;
pub type TileEntities = HashMap<TileCoord, ActorRef<TileEntityMsg>>;

#[derive(Debug, Clone)]
pub struct Map {
    pub map_name: String,

    pub tiles: Tiles,
    pub data: DataMap,
}

#[derive(Debug, Serialize, Deserialize)]
struct MapHeader(Vec<(Id, String)>);

#[derive(Debug, Serialize, Deserialize)]
struct TileData(Id, TileState, DataMapRaw);

impl Map {
    pub fn new_empty(map_name: String) -> Self {
        Self {
            map_name,

            tiles: Default::default(),
            data: Default::default(),
        }
    }

    pub fn path(map_name: &str) -> PathBuf {
        PathBuf::from(format!("{MAP_PATH}/{map_name}.bin"))
    }

    pub fn save(&self, sys: &ActorSystem, interner: &Interner, tile_entities: TileEntities) {
        drop(std::fs::create_dir_all(MAP_PATH));

        let path = Self::path(&self.map_name);

        let file = File::create(path).unwrap();

        let writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, file);
        let mut encoder = Encoder::new(writer, 0).unwrap();

        let mut id_map = HashMap::new();

        let tiles = self
            .tiles
            .iter()
            .flat_map(|(coord, (id, tile_state))| {
                if let Some(tile_entity) = tile_entities.get(coord) {
                    if !id_map.contains_key(id) {
                        id_map.insert(*id, interner.resolve(*id).unwrap().to_string());
                    }

                    let data: DataMap = block_on(ask(sys, tile_entity, GetData));
                    let data = data_to_raw(data, interner);

                    Some((coord, TileData(*id, *tile_state, data)))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let header = MapHeader(id_map.into_iter().collect());

        let data = data_to_raw(self.data.clone(), interner);

        serde_json::to_writer(&mut encoder, &(header, tiles, data)).unwrap();

        encoder.do_finish().unwrap();
    }

    pub fn load(
        ctx: &Context<GameMsg>,
        resource_man: &ResourceManager,
        map_name: String,
    ) -> (Self, TileEntities) {
        let path = Self::path(&map_name);

        let file = if let Ok(file) = File::open(path) {
            file
        } else {
            return (Map::new_empty(map_name), Default::default());
        };

        let reader = BufReader::with_capacity(MAP_BUFFER_SIZE, file);
        let decoder = Decoder::new(reader).unwrap();

        let (header, tiles, data): (MapHeader, Vec<(TileCoord, TileData)>, DataMapRaw) =
            serde_json::from_reader(decoder).unwrap();

        let id_reverse = header.0.into_iter().collect::<HashMap<_, _>>();

        let (tiles, tile_entities): (Tiles, TileEntities) = tiles
            .into_iter()
            .flat_map(|(coord, TileData(id, tile_state, data))| {
                if let Some(id) = id_reverse
                    .get(&id)
                    .and_then(|id| resource_man.interner.get(id.as_str()))
                {
                    let tile_entity = Game::new_tile(ctx, coord, id, tile_state);
                    let data = data_from_raw(data, &resource_man.interner);

                    data.into_iter().for_each(|(key, value)| {
                        tile_entity.send_msg(SetData(key, value), None);
                    });

                    Some(((coord, (id, tile_state)), (coord, tile_entity)))
                } else {
                    None
                }
            })
            .unzip();

        let data = data_from_raw(data, &resource_man.interner);

        (
            Self {
                map_name,

                tiles,
                data,
            },
            tile_entities,
        )
    }
}
