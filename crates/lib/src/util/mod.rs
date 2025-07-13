//pub mod discord;
pub mod window {
    use automancy_data::math::Vec2;
    use winit::window::Window;

    pub fn window_size(window: &Window) -> Vec2 {
        let size = window.inner_size();

        Vec2::new(size.width as f32, size.height as f32)
    }
}
