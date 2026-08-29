use automancy_data::{
    game::{coord::TileCoord, inventory::ItemAmount},
    id::{Id, ItemId, ModelId, RecipeId, RenderId},
    id_map::IdSet,
    math::Matrix4,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderCommand {
    Track {
        render_id: RenderId,
        model_id: ModelId,
    },
    Transform {
        render_id: RenderId,
        model_id: ModelId,
        model_matrix: Matrix4,
    },
    Untrack {
        render_id: RenderId,
        model_id: ModelId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiElement {
    Row { e: Vec<UiElement> },
    CenterRow { e: Vec<UiElement> },
    Col { e: Vec<UiElement> },
    Label { id: Id },
    InfoTip { id: Id },
    LabelAmount { amount: ItemAmount },
    InputAmount { id: Id, max: ItemAmount },
    SliderAmount { id: Id, max: ItemAmount },
    HexDirInput { id: Id },
    SelectableItems { data_id: Id, hint_id: Id, ids: IdSet<ItemId> },
    SelectableRecipes { data_id: Id, hint_id: Id, ids: IdSet<RecipeId> },
    Inventory { id: Id, empty_text: Id },
    Linkage { id: Id, coord: TileCoord, button_text: Id },
}
