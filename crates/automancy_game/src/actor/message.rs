use automancy_data::{
    game::{
        coord::{TileBounds, TileCoord},
        generic::{DataMap, Datum},
        item::ItemStack,
    },
    id::{Id, TileId},
};
use hashbrown::HashMap;
use ractor::RpcReplyPort;

use crate::{
    actor::{FlatTiles, TickUnit, TileEntry, map},
    scripting::{render::RenderCommand, ui::RhaiUiUnit},
};

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

#[derive(Debug, Copy, Clone)]
pub enum OnFailAction {
    None,
    RemoveTile,
    RemoveAllData,
    RemoveData(Id),
}

#[derive(Debug, Copy, Clone)]
pub enum PlaceTileResponse {
    Placed,
    Removed,
    Ignored,
}

/// Represents a message the game receives
pub enum GameMsg {
    /// tick the tiles once
    Tick,
    SendTileMsg(TileCoord, TileMsg),

    LoadMap(map::GameMapId, RpcReplyPort<bool>),
    SaveMap(RpcReplyPort<()>),
    GetMapIdAndData(RpcReplyPort<Option<(map::GameMapId, map::GameMapData)>>),

    PlaceTile {
        coord: TileCoord,
        tile: (TileId, DataMap),
        record: bool,
        reply: Option<RpcReplyPort<PlaceTileResponse>>,
    },
    PlaceTiles {
        tiles: FlatTiles,
        replace: bool,
        record: bool,
        reply: Option<RpcReplyPort<FlatTiles>>,
    },
    MoveTiles(Vec<TileCoord>, TileCoord, bool),
    Undo,

    GetTile(TileCoord, RpcReplyPort<Option<TileEntry>>),
    GetTiles(
        Vec<TileCoord>,
        RpcReplyPort<Vec<(TileCoord, Option<TileEntry>)>>,
    ),
    GetAllRenderCommands {
        culling_range: TileBounds,
        reply: RpcReplyPort<[HashMap<TileCoord, Vec<RenderCommand>>; 2]>,
    },
}

pub enum TileMsg {
    Tick {
        tick_count: TickUnit,
    },

    Transaction {
        stack: ItemStack,
        source_coord: TileCoord,
        source_id: TileId,
        root_coord: TileCoord,
        root_id: TileId,
        hidden: bool,
    },
    TransactionResult {
        result: ItemStack,
    },
    ExtractRequest {
        requested_from_id: TileId,
        requested_from_coord: TileCoord,
    },

    CollectRenderCommands {
        reply: RpcReplyPort<Option<Vec<RenderCommand>>>,
        loading: bool,
        unloading: bool,
    },
    GetTileConfigUi(RpcReplyPort<Option<RhaiUiUnit>>),

    GetData(RpcReplyPort<DataMap>),
    GetDatum(Id, RpcReplyPort<Option<Datum>>),
    SetData(DataMap),
    SetDatum(Id, Datum),
    TakeData(RpcReplyPort<DataMap>),
    RemoveDatum(Id),
    ReadData(Box<dyn FnOnce(&mut DataMap) + Send + Sync>),
}
