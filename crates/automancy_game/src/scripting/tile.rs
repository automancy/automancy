use rhai::{Engine, exported_module};

mod tile_stuff {
    use rhai::{Module, plugin::*};

    #[allow(non_snake_case)]
    #[export_module]
    pub mod tile_result {
        use automancy_data::{
            game::{coord::TileCoord, inventory::ItemStack},
            id::TileId,
        };
        use rhai::Array;

        use crate::actor::message::{OnFailAction, TileResult};

        pub fn MakeTransaction(coord: TileCoord, source_id: TileId, source_coord: TileCoord, stacks: Array) -> TileResult {
            TileResult::MakeTransaction {
                coord,
                source_id,
                source_coord,
                stacks: stacks.into_iter().map(Dynamic::cast::<ItemStack>).collect(),
            }
        }
        pub fn MakeExtractRequest(
            coord: TileCoord,
            requested_from_id: TileId,
            requested_from_coord: TileCoord,
            on_fail_action: OnFailAction,
        ) -> TileResult {
            TileResult::MakeExtractRequest {
                coord,
                requested_from_id,
                requested_from_coord,
                on_fail_action,
            }
        }
    }

    #[allow(non_snake_case)]
    #[export_module]
    pub mod tile_trans_result {
        use automancy_data::{
            game::{coord::TileCoord, inventory::ItemStack},
            id::TileId,
        };

        use crate::actor::message::TileTransactionResult;

        pub fn PassOn(coord: TileCoord, stack: ItemStack, source_coord: TileCoord, root_coord: TileCoord, root_id: TileId) -> TileTransactionResult {
            TileTransactionResult::PassOn {
                coord,
                stack,
                source_coord,
                root_coord,
                root_id,
            }
        }
        pub fn Proxy(
            coord: TileCoord,
            stack: ItemStack,
            source_coord: TileCoord,
            source_id: TileId,
            root_coord: TileCoord,
            root_id: TileId,
        ) -> TileTransactionResult {
            TileTransactionResult::Proxy {
                coord,
                stack,
                source_coord,
                source_id,
                root_coord,
                root_id,
            }
        }
        pub fn Consume(consumed: ItemStack, source_coord: TileCoord, root_coord: TileCoord) -> TileTransactionResult {
            TileTransactionResult::Consume {
                consumed,
                source_coord,
                root_coord,
            }
        }
    }

    #[allow(non_snake_case)]
    #[export_module]
    pub mod on_fail_action {
        use automancy_data::id::Id;

        use crate::actor::message::OnFailAction;

        pub fn None() -> OnFailAction {
            OnFailAction::None
        }
        pub fn RemoveTile() -> OnFailAction {
            OnFailAction::RemoveTile
        }
        pub fn RemoveAllData() -> OnFailAction {
            OnFailAction::RemoveAllData
        }
        pub fn RemoveData(id: Id) -> OnFailAction {
            OnFailAction::RemoveData(id)
        }
    }
}

pub(crate) fn register_tile_stuff(engine: &mut Engine) {
    engine.register_static_module("Result", exported_module!(tile_stuff::tile_result).into());
    engine.register_static_module("TransResult", exported_module!(tile_stuff::tile_trans_result).into());
    engine.register_static_module("OnFailAction", exported_module!(tile_stuff::on_fail_action).into());
}
