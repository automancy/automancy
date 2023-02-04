use automancy::game::run::event::{on_event, EventLoopStorage};
use automancy::game::run::setup::GameSetup;
use env_logger::Env;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let (event_loop, mut setup) = GameSetup::setup();
    let mut persistent = EventLoopStorage::default();

    event_loop.run(move |event, _, control_flow| {
        on_event(&mut setup, &mut persistent, event, control_flow).unwrap();
    });
}
