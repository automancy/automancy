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
        if unlocked.contains(&research) {
            return true;
        }
    }

    false
}
