use automancy::game::event::{on_event, PersistentEventStorage};
use automancy::game::setup::GameSetup;

fn main() {
    let (event_loop, mut setup) = GameSetup::setup();
    let mut persistent = PersistentEventStorage::default();

    event_loop.run(move |event, _, control_flow| {
        on_event(&mut setup, &mut persistent, event, control_flow);
    });
}
