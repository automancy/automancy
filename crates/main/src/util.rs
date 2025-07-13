pub static GAME_LOGO: &[u8] = include_bytes!("assets/logo.png");

pub fn get_window_icon() -> winit::window::Icon {
    let image = image::load_from_memory(GAME_LOGO).unwrap().to_rgba8();
    let width = image.width();
    let height = image.height();

    let samples = image.into_flat_samples().samples;
    winit::window::Icon::from_rgba(samples, width, height).unwrap()
}
