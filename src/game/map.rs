use futures_executor::block_on;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::game::{Game, GameMsg};
use riker::actor::ActorRef;
use riker::actors::{ActorSystem, Context};
use riker_patterns::ask::ask;
use serde::{Deserialize, Serialize};
use zstd::{Decoder, Encoder};

use crate::game::tile::coord::{TileCoord, TileUnit};
use crate::game::tile::entity::TileEntityMsg::{GetData, SetData};
use crate::game::tile::entity::{
    data_from_raw, data_to_raw, DataMap, DataMapRaw, TileEntityMsg, TileState,
};
use crate::render::data::InstanceData;
use crate::resource::ResourceManager;
use crate::util::id::{Id, Interner};

pub const MAP_PATH: &str = "map";

const MAP_BUFFER_SIZE: usize = 256 * 1024;

#[derive(Clone, Debug)]
pub struct RenderContext {
    pub resource_man: Arc<ResourceManager>,
    pub range: TileUnit,
    pub center: TileCoord,
}

#[derive(Clone, Debug)]
pub struct MapRenderInfo {
    pub instances: HashMap<TileCoord, (InstanceData, Id)>,
}

#[derive(Debug, Clone)]
pub struct Map {
    pub render_cache: HashMap<(TileUnit, TileCoord), Arc<MapRenderInfo>>,

    pub map_name: String,

    pub tiles: HashMap<TileCoord, (ActorRef<TileEntityMsg>, Id, TileState)>,
    pub data: DataMap,
}

#[derive(Debug, Serialize, Deserialize)]
struct MapHeader(Vec<(Id, String)>);

#[derive(Debug, Serialize, Deserialize)]
struct TileData(Id, TileState, DataMapRaw);

impl Map {
    pub fn render_info(
        &mut self,
        RenderContext {
            resource_man,
            range,
            center,
        }: RenderContext,
    ) -> Arc<MapRenderInfo> {
        if let Some(info) = self.render_cache.get(&(range, center)) {
            return info.clone();
        }

        let instances = self
            .tiles
            .iter()
            .filter(|(pos, _)| center.distance(**pos) <= range)
            .flat_map(|(pos, (_, id, tile_state))| {
                InstanceData::from_tile(resource_man.clone(), *id, *pos, *tile_state)
            })
            .collect();

        let info = Arc::new(MapRenderInfo { instances });

        if self.render_cache.len() > 16 {
            self.render_cache.clear();
        }

        self.render_cache.insert((range, center), info.clone());

        info
    }

    pub fn new_empty(map_name: String) -> Self {
        Self {
            render_cache: Default::default(),

            map_name,

            tiles: Default::default(),
            data: Default::default(),
        }
    }

    pub fn path(map_name: &str) -> PathBuf {
        PathBuf::from(format!("{MAP_PATH}/{map_name}.bin"))
    }

    pub fn save(&self, sys: &ActorSystem, interner: &Interner) {
        drop(std::fs::create_dir(MAP_PATH));

        let path = Self::path(&self.map_name);

        let file = File::create(path).unwrap();

        let writer = BufWriter::with_capacity(MAP_BUFFER_SIZE, file);
        let mut encoder = Encoder::new(writer, 0).unwrap();

        let mut id_map = HashMap::new();

        let tiles = self
            .tiles
            .iter()
            .map(|(coord, (tile, id, tile_state))| {
                if !id_map.contains_key(id) {
                    id_map.insert(*id, interner.resolve(*id).unwrap().to_string());
                }

                let data: DataMap = block_on(ask(sys, tile, GetData));
                let data = data_to_raw(data, interner);

                (coord, TileData(*id, *tile_state, data))
            })
            .collect::<Vec<_>>();

        let header = MapHeader(id_map.into_iter().collect());

        let data = data_to_raw(self.data.clone(), interner);

        serde_json::to_writer(&mut encoder, &(header, tiles, data)).unwrap();

        encoder.do_finish().unwrap();
    }

    pub fn load(
        ctx: &Context<GameMsg>,
        resource_man: Arc<ResourceManager>,
        map_name: String,
    ) -> Self {
        let path = Self::path(&map_name);

        let file = if let Ok(file) = File::open(path) {
            file
        } else {
            return Map::new_empty(map_name);
        };

        let reader = BufReader::with_capacity(MAP_BUFFER_SIZE, file);
        let decoder = Decoder::new(reader).unwrap();

        let (header, tiles, data): (MapHeader, Vec<(TileCoord, TileData)>, DataMapRaw) =
            serde_json::from_reader(decoder).unwrap();

        let id_reverse = header.0.into_iter().collect::<HashMap<_, _>>();

        let tiles = tiles
            .into_iter()
            .flat_map(|(coord, TileData(id, tile_state, data))| {
                if let Some(id) = id_reverse
                    .get(&id)
                    .and_then(|id| resource_man.interner.get(id.as_str()))
                {
                    let tile = Game::new_tile(ctx, coord, id, tile_state);
                    let data = data_from_raw(data, &resource_man.interner);

                    data.into_iter().for_each(|(key, value)| {
                        tile.send_msg(SetData(key, value), None);
                    });

                    Some((coord, (tile, id, tile_state)))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let data = data_from_raw(data, &resource_man.interner);

        Self {
            render_cache: Default::default(),

            map_name,

            tiles,
            data,
        }
    }
}
