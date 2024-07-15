use hashbrown::HashSet;

use automancy_defs::id::Id;
use automancy_resources::data::{Data, DataMap};
use automancy_resources::ResourceManager;

pub mod actor;
pub mod discord;
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
        if !unlocked.contains(&research) {
            return false;
        }
    }

    true
}

pub fn should_category_show(
    category: Id,
    resource_man: &ResourceManager,
    game_data: &mut DataMap,
) -> bool {
    let Some(category) = resource_man.registry.categories.get(&category) else {
        return false;
    };

    let Some(researches) = resource_man.get_researches_by_category(category.id) else {
        return false;
    };

    if let Data::SetId(unlocked) = game_data
        .entry(resource_man.registry.data_ids.unlocked_researches)
        .or_insert_with(|| Data::SetId(HashSet::new()))
    {
        for research in researches {
            if !unlocked.contains(&research) {
                return false;
            }
        }
    }

    true
}
