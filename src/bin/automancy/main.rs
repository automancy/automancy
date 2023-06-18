use crate::event::{on_event, EventLoopStorage};
use crate::setup::GameSetup;
use automancy::renderer::Renderer;
use env_logger::Env;
use tokio::runtime::Runtime;

pub static LOGO: &[u8] = include_bytes!("../../../compile/logo.png");
pub static IOSEVKA_FONT: &[u8] = include_bytes!("../../../compile/fonts/iosevka-extended.ttf");

mod event;
mod gui;
mod setup;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let runtime = Runtime::new().unwrap();

    let (event_loop, gpu, mut setup) = runtime.block_on(GameSetup::setup());
    let mut renderer = Renderer::new(setup.resource_man.clone(), gpu);
    let mut gui = gui::init_gui(&event_loop, &renderer.gpu);

    let mut storage = EventLoopStorage::default();

    event_loop.run(move |event, _, control_flow| {
        let _ = on_event(
            &runtime,
            &mut setup,
            &mut storage,
            &mut renderer,
            &mut gui,
            event,
            control_flow,
        );
    });
}
