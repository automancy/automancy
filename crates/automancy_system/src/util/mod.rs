use automancy_defs::id::Id;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::ResourceManager;
use hashbrown::HashSet;

pub mod actor;
pub mod num;
pub mod round;

pub fn is_research_unlocked(
    research: Id,
    resource_man: &ResourceManager,
    game_data: &mut DataMap,
) -> bool {
    if let Data::SetId(unlocked) = game_data
        .entry(resource_man.registry.data_ids.unlocked_researches)
        .or_insert_with(|| Data::SetId(HashSet::new()))
    {
        if unlocked.contains(&research) {
            return true;
        }
    }

    false
}

pub fn should_category_show(
    category: Id,
    resource_man: &ResourceManager,
    game_data: &mut DataMap,
) -> bool {
    let Some(category) = resource_man.registry.categories.get(&category) else {
        return false;
    };

    let Some(tiles) = resource_man.get_tiles_by_category(category.id) else {
        return false;
    };

    if tiles.iter().any(|id| {
        resource_man.registry.tiles[id]
            .data
            .get(resource_man.registry.data_ids.default_tile)
            .cloned()
            .and_then(|v| v.into_bool())
            .unwrap_or(false)
    }) {
        return true;
    }

    let Some(researches) = resource_man.get_researches_by_category(category.id) else {
        return false;
    };

    if let Data::SetId(unlocked) = game_data
        .entry(resource_man.registry.data_ids.unlocked_researches)
        .or_insert_with(|| Data::SetId(HashSet::new()))
    {
        for research in researches {
            if unlocked.contains(&research) {
                return true;
            }
        }
    }

    false
}
