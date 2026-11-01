use automancy_data::{
    game::{coord::TileCoord, inventory::ItemAmount},
    id::{Id, ItemId, RecipeId},
};
use rhai::{Engine, exported_module, plugin::*};

#[allow(non_snake_case)]
#[export_module]
mod ui {
    use automancy_data::game::generic::DataMap;
    use rhai::Array;

    use crate::script::UiElement;

    pub fn Row(e: Array) -> UiElement {
        UiElement::Row {
            e: e.into_iter().map(Dynamic::cast::<UiElement>).collect(),
        }
    }
    pub fn CenterRow(e: Array) -> UiElement {
        UiElement::CenterRow {
            e: e.into_iter().map(Dynamic::cast::<UiElement>).collect(),
        }
    }
    pub fn Col(e: Array) -> UiElement {
        UiElement::Col {
            e: e.into_iter().map(Dynamic::cast::<UiElement>).collect(),
        }
    }
    pub fn Label(id: Id) -> UiElement {
        UiElement::Label {
            id,
        }
    }
    pub fn InfoTip(id: Id) -> UiElement {
        UiElement::InfoTip {
            id,
        }
    }
    pub fn LabelAmount(amount: ItemAmount) -> UiElement {
        UiElement::LabelAmount {
            amount,
        }
    }
    pub fn InputAmount(id: Id, max: ItemAmount) -> UiElement {
        UiElement::InputAmount {
            id,
            max,
        }
    }
    pub fn SliderAmount(id: Id, max: ItemAmount) -> UiElement {
        UiElement::SliderAmount {
            id,
            max,
        }
    }
    pub fn HexDirInput(id: Id) -> UiElement {
        UiElement::HexDirInput {
            id,
        }
    }
    pub fn SelectableItems(data_id: Id, hint_id: Id, ids: Box<DataMap>) -> UiElement {
        UiElement::SelectableItems {
            data_id,
            hint_id,
            ids: ids.into_iter().map(|(key, _)| ItemId(key)).collect(),
        }
    }
    pub fn SelectableRecipes(data_id: Id, hint_id: Id, ids: Box<DataMap>) -> UiElement {
        UiElement::SelectableRecipes {
            data_id,
            hint_id,
            ids: ids.into_iter().map(|(key, _)| RecipeId(key)).collect(),
        }
    }
    pub fn Items(id: Id, empty_text: Id) -> UiElement {
        UiElement::Items {
            id,
            empty_text,
        }
    }
    pub fn Linkage(id: Id, coord: TileCoord, button_text: Id) -> UiElement {
        UiElement::Linkage {
            id,
            coord,
            button_text,
        }
    }
}

pub(crate) fn register_ui_stuff(engine: &mut Engine) {
    engine.register_static_module("Ui", exported_module!(ui).into());
}
