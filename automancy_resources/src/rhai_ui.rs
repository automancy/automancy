use automancy_defs::{id::Id, log};
use rhai::{Dynamic, Engine, Module};

use crate::data::stack::ItemAmount;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RhaiUiTag {
    Label,
    LabelAmount,
    SliderAmount,
    Row,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RhaiUiUnit {
    Label { id: Id },
    LabelAmount { amount: ItemAmount },
    SliderAmount { id: Id, max: ItemAmount },
    Row { e: Vec<RhaiUiUnit> },
}

pub(crate) fn register_ui_stuff(engine: &mut Engine) {
    {
        let mut module = Module::new();

        module
            .set_var("Row", RhaiUiTag::Row)
            .set_var("Label", RhaiUiTag::Label)
            .set_var("LabelAmount", RhaiUiTag::LabelAmount)
            .set_var("SliderAmount", RhaiUiTag::SliderAmount);

        engine.register_static_module("Ui", module.into());
    }
}

fn parse(
    function_id: &str,
    tag: RhaiUiTag,
    src: rhai::Array,
    depth: usize,
) -> Result<RhaiUiUnit, usize> {
    Ok(match tag {
        RhaiUiTag::Label => {
            let id = src
                .get(1)
                .and_then(|v| v.clone().try_cast::<Id>())
                .ok_or(1usize)?;

            RhaiUiUnit::Label { id }
        }
        RhaiUiTag::LabelAmount => {
            let amount = src
                .get(1)
                .and_then(|v| v.clone().try_cast::<ItemAmount>())
                .ok_or(1usize)?;

            RhaiUiUnit::LabelAmount { amount }
        }
        RhaiUiTag::SliderAmount => {
            let id = src
                .get(1)
                .and_then(|v| v.clone().try_cast::<Id>())
                .ok_or(1usize)?;

            let max = src
                .get(2)
                .and_then(|v| v.clone().try_cast::<ItemAmount>())
                .ok_or(2usize)?;

            RhaiUiUnit::SliderAmount { id, max }
        }
        RhaiUiTag::Row => {
            let e = src
                .get(1)
                .and_then(|v| v.clone().try_cast::<rhai::Array>())
                .ok_or(1usize)?;

            RhaiUiUnit::Row {
                e: e.into_iter()
                    .flat_map(|v: Dynamic| v.try_cast::<rhai::Array>())
                    .enumerate()
                    .flat_map(|(index, src)| parse_rhai_ui_unit(function_id, src, index, depth + 1))
                    .collect(),
            }
        }
    })
}

fn try_parse_rhai_ui_unit(
    function_id: &str,
    src: rhai::Array,
    depth: usize,
) -> Result<RhaiUiUnit, (Option<RhaiUiTag>, usize, Option<usize>)> {
    let tag = src
        .first()
        .and_then(|v| v.clone().try_cast::<RhaiUiTag>())
        .ok_or((None, depth, None))?;

    parse(function_id, tag, src, depth).map_err(|pos| (Some(tag), depth, Some(pos)))
}

fn parse_rhai_ui_unit(
    function_id: &str,
    src: rhai::Array,
    index: usize,
    depth: usize,
) -> Option<RhaiUiUnit> {
    match try_parse_rhai_ui_unit(function_id, src, depth) {
        Ok(r) => Some(r),
        Err((tag, depth, pos)) => {
            let tag = tag.ok_or("UNRECOGNIZED_TAG");
            log::error!("Error in parsing UI function from {function_id}, tag {tag:?}: at depth {depth}, index {index}, argument {pos:?}.");

            None
        }
    }
}

pub fn parse_rhai_ui(function_id: &str, src: Vec<Dynamic>) -> Vec<RhaiUiUnit> {
    src.into_iter()
        .flat_map(|v: Dynamic| v.try_cast::<rhai::Array>())
        .enumerate()
        .flat_map(|(index, src)| parse_rhai_ui_unit(function_id, src, index, 0))
        .collect()
}
