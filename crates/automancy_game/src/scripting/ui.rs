use automancy_data::{game::inventory::ItemAmount, id::Id};
use rhai::{Engine, Module, exported_module, plugin::*};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RhaiUiUnit {
    Row { e: Vec<RhaiUiUnit> },
    CenterRow { e: Vec<RhaiUiUnit> },
    Col { e: Vec<RhaiUiUnit> },
    Label { id: Id },
    InfoTip { id: Id },
    LabelAmount { amount: ItemAmount },
    InputAmount { id: Id, max: ItemAmount },
    SliderAmount { id: Id, max: ItemAmount },
    HexDirInput { id: Id },
    SelectableItems { data_id: Id, hint_id: Id, ids: Vec<Id> },
    SelectableScripts { data_id: Id, hint_id: Id, ids: Vec<Id> },
    Inventory { id: Id, empty_text: Id },
    Linkage { id: Id, button_text: Id },
}

#[allow(non_snake_case)]
#[export_module]
mod ui {
    use rhai::Array;

    pub fn Row(e: Array) -> RhaiUiUnit {
        RhaiUiUnit::Row {
            e: e.into_iter().map(Dynamic::cast::<RhaiUiUnit>).collect(),
        }
    }
    pub fn CenterRow(e: Array) -> RhaiUiUnit {
        RhaiUiUnit::CenterRow {
            e: e.into_iter().map(Dynamic::cast::<RhaiUiUnit>).collect(),
        }
    }
    pub fn Col(e: Array) -> RhaiUiUnit {
        RhaiUiUnit::Col {
            e: e.into_iter().map(Dynamic::cast::<RhaiUiUnit>).collect(),
        }
    }
    pub fn Label(id: Id) -> RhaiUiUnit {
        RhaiUiUnit::Label { id }
    }
    pub fn InfoTip(id: Id) -> RhaiUiUnit {
        RhaiUiUnit::InfoTip { id }
    }
    pub fn LabelAmount(amount: ItemAmount) -> RhaiUiUnit {
        RhaiUiUnit::LabelAmount { amount }
    }
    pub fn InputAmount(id: Id, max: ItemAmount) -> RhaiUiUnit {
        RhaiUiUnit::InputAmount { id, max }
    }
    pub fn SliderAmount(id: Id, max: ItemAmount) -> RhaiUiUnit {
        RhaiUiUnit::SliderAmount { id, max }
    }
    pub fn HexDirInput(id: Id) -> RhaiUiUnit {
        RhaiUiUnit::HexDirInput { id }
    }
    pub fn SelectableItems(data_id: Id, hint_id: Id, ids: Array) -> RhaiUiUnit {
        RhaiUiUnit::SelectableItems {
            data_id,
            hint_id,
            ids: ids.into_iter().map(Dynamic::cast::<Id>).collect(),
        }
    }
    pub fn SelectableScripts(data_id: Id, hint_id: Id, ids: Array) -> RhaiUiUnit {
        RhaiUiUnit::SelectableScripts {
            data_id,
            hint_id,
            ids: ids.into_iter().map(Dynamic::cast::<Id>).collect(),
        }
    }
    pub fn Inventory(id: Id, empty_text: Id) -> RhaiUiUnit {
        RhaiUiUnit::Inventory { id, empty_text }
    }
    pub fn Linkage(id: Id, button_text: Id) -> RhaiUiUnit {
        RhaiUiUnit::Linkage { id, button_text }
    }
}

pub(crate) fn register_ui_stuff(engine: &mut Engine) {
    engine.register_static_module("Ui", exported_module!(ui).into());
}
