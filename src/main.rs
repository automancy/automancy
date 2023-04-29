use env_logger::Env;

use automancy::game::run::event::{on_event, EventLoopStorage};
use automancy::game::run::setup::GameSetup;
use automancy::render::gui;
use automancy::render::renderer::Renderer;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let (event_loop, gpu, mut setup) = GameSetup::setup();
    let mut renderer = Renderer::new(setup.resource_man.clone(), gpu);
    let mut gui = gui::init_gui(&event_loop, &renderer.gpu);

    let mut storage = EventLoopStorage::default();

    event_loop.run(move |event, _, control_flow| {
        on_event(
            &mut setup,
            &mut storage,
            &mut renderer,
            &mut gui,
            event,
            control_flow,
        )
        .unwrap();
    });
}
