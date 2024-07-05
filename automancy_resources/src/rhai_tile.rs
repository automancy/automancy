use rhai::{exported_module, Engine};

#[allow(non_snake_case)]
mod tile_stuff {
    use automancy_defs::{coord::TileCoord, id::Id, stack::ItemStack};

    use rhai::plugin::*;
    use rhai::Module;

    use crate::types::function::{OnFailAction, TileResult, TileTransactionResult};

    #[export_module]
    pub mod tile_result {
        use rhai::Array;

        pub fn MakeTransaction(
            coord: TileCoord,
            source_id: Id,
            source_coord: TileCoord,
            stacks: Array,
        ) -> TileResult {
            TileResult::MakeTransaction {
                coord,
                source_id,
                source_coord,
                stacks: stacks.into_iter().map(Dynamic::cast::<ItemStack>).collect(),
            }
        }
        pub fn MakeExtractRequest(
            coord: TileCoord,
            requested_from_id: Id,
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

    #[export_module]
    pub mod tile_trans_result {
        pub fn PassOn(
            coord: TileCoord,
            stack: ItemStack,
            source_coord: TileCoord,
            root_coord: TileCoord,
            root_id: Id,
        ) -> TileTransactionResult {
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
            source_id: Id,
            root_coord: TileCoord,
            root_id: Id,
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
        pub fn Consume(
            consumed: ItemStack,
            source_coord: TileCoord,
            root_coord: TileCoord,
        ) -> TileTransactionResult {
            TileTransactionResult::Consume {
                consumed,
                source_coord,
                root_coord,
            }
        }
    }

    #[export_module]
    pub mod on_fail_action {
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
    engine.register_static_module(
        "TransResult",
        exported_module!(tile_stuff::tile_trans_result).into(),
    );
    engine.register_static_module(
        "OnFailAction",
        exported_module!(tile_stuff::on_fail_action).into(),
    );
}
