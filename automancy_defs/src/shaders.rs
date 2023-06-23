pub mod game_vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/game_vert.glsl"
    }
}

pub mod game_frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/game_frag.glsl"
    }
}

pub mod gui_vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/gui_vert.glsl"
    }
}

pub mod gui_frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/gui_frag.glsl"
    }
}

pub mod overlay_vert_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/overlay_vert.glsl"
    }
}

pub mod overlay_frag_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/overlay_frag.glsl"
    }
}
