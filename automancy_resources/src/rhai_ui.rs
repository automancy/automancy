use automancy_defs::{id::Id, stack::ItemAmount};
use rhai::plugin::*;
use rhai::Module;
use rhai::{exported_module, Engine};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RhaiUiUnit {
    Row { e: Vec<RhaiUiUnit> },
    Col { e: Vec<RhaiUiUnit> },
    Label { id: Id },
    LabelAmount { amount: ItemAmount },
    InputAmount { id: Id, max: ItemAmount },
    SliderAmount { id: Id, max: ItemAmount },
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
    pub fn Col(e: Array) -> RhaiUiUnit {
        RhaiUiUnit::Col {
            e: e.into_iter().map(Dynamic::cast::<RhaiUiUnit>).collect(),
        }
    }
    pub fn Label(id: Id) -> RhaiUiUnit {
        RhaiUiUnit::Label { id }
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
}

pub(crate) fn register_ui_stuff(engine: &mut Engine) {
    engine.register_static_module("Ui", exported_module!(ui).into());
}
