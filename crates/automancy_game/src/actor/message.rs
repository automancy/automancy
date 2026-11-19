use std::collections::BTreeMap;

use automancy_data::{
    game::{
        coord::{TileCoord, TileCoordBounds},
        generic::{DataMap, DatumChange},
        inventory::ItemStack,
    },
    id::{Id, TileId},
};
use ractor::RpcReplyPort;

use crate::{
    actor::{FlatTile, FlatTiles, TileEntry, TileMap, game_entity::TickUnit},
    persistent::map,
    script::{RenderCommand, UiElement},
};

pub type GameRenderCommands = BTreeMap<TileCoord, Vec<RenderCommand>>;

#[derive(Debug, Clone)]
pub enum TileResult {
    MakeTransaction {
        coord: TileCoord,
        source_id: TileId,
        source_coord: TileCoord,
        stacks: Vec<ItemStack>,
    },
    MakeExtractRequest {
        coord: TileCoord,
        requested_from_id: TileId,
        requested_from_coord: TileCoord,
        on_fail_action: OnFailAction,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct TileTransaction {
    pub stack: ItemStack,
    pub source_coord: TileCoord,
    pub source_id: TileId,
    pub root_coord: TileCoord,
    pub root_id: TileId,
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum TileTransactionResult {
    PassOn {
        coord: TileCoord,
        stack: ItemStack,
        source_coord: TileCoord,
        root_coord: TileCoord,
        root_id: TileId,
    },
    Proxy {
        coord: TileCoord,
        stack: ItemStack,
        source_coord: TileCoord,
        source_id: TileId,
        root_coord: TileCoord,
        root_id: TileId,
    },
    Consume {
        consumed: ItemStack,
        source_coord: TileCoord,
        root_coord: TileCoord,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum OnFailAction {
    None,
    RemoveTile,
    RemoveAllData,
    RemoveData(Id),
}

#[derive(Debug, Clone, Copy)]
pub enum PlaceTileResponse {
    Placed,
    Removed,
    Ignored,
}

#[derive(Debug, Clone)]
pub struct PlaceTileParams {
    pub coord: TileCoord,
    pub id: TileId,
    pub data: Box<DataMap>,
    pub record: bool,
}

#[derive(Debug, Clone)]
pub struct PlaceTilesParams {
    pub tiles: FlatTiles,
    pub replace: bool,
    pub record: bool,
}

/// Represents a message the game receives
pub enum GameMsg {
    /// tick the tiles once
    Tick,
    SendTileMsg(TileCoord, TileMsg),

    SaveMap,
    SaveAndUnload(RpcReplyPort<anyhow::Result<()>>),
    LoadMap(map::GameMapId, RpcReplyPort<bool>),
    GetMap(RpcReplyPort<Option<(map::GameMapId, map::GameMapInfo)>>),

    PlaceTile {
        params: PlaceTileParams,
        reply: Option<RpcReplyPort<PlaceTileResponse>>,
    },
    PlaceTiles {
        params: PlaceTilesParams,
        reply: Option<RpcReplyPort<FlatTiles>>,
    },
    MoveTiles(Vec<TileCoord>, TileCoord, bool),
    Undo,

    GetTile(TileCoord, RpcReplyPort<Option<TileEntry>>),
    GetTileFlat(TileCoord, RpcReplyPort<Option<FlatTile>>),
    GetTiles(Vec<TileCoord>, RpcReplyPort<TileMap>),
    GetTilesFlat(Vec<TileCoord>, RpcReplyPort<FlatTiles>),

    GetMapData(RpcReplyPort<Box<DataMap>>),
    ChangeMapData(Vec<DatumChange>, RpcReplyPort<()>),

    PushRenderCommands {
        coord: TileCoord,
        commands: Vec<RenderCommand>,
    },
    CollectRenderCommands {
        culling_bounds: TileCoordBounds,
        reply: RpcReplyPort<[GameRenderCommands; 2]>,
    },
}

#[derive(Default)]
pub enum TileMsg {
    /// Dummy default value to allow for things like std::mem::take
    #[default]
    None,

    Tick {
        tick_count: TickUnit,
    },

    StartRendering(RpcReplyPort<Vec<RenderCommand>>),
    StopRendering(RpcReplyPort<Vec<RenderCommand>>),

    Transaction(TileTransaction),
    TransactionResult {
        result: ItemStack,
    },
    ExtractRequest {
        requested_from_id: TileId,
        requested_from_coord: TileCoord,
    },

    GetTileConfigUi(RpcReplyPort<Option<UiElement>>),

    GetData(RpcReplyPort<Box<DataMap>>),
    SetData(Box<DataMap>),
    TakeData(RpcReplyPort<Box<DataMap>>),
    ChangeData(DatumChange),
    FnData(Box<dyn FnOnce(&mut DataMap) + Send + Sync>),
    FnDataStatic(&'static (dyn Fn(&mut DataMap) + Send + Sync)),
}
