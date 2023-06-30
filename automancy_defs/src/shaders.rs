use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};

pub fn game_shader(device: &Device) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Game Shader"),
        source: ShaderSource::Wgsl(include_str!("../shaders/game.wgsl").into()),
    })
}

pub fn effects_shader(device: &Device) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Effects Shader"),
        source: ShaderSource::Wgsl(include_str!("../shaders/effects.wgsl").into()),
    })
}

pub fn overlay_shader(device: &Device) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Overlay Shader"),
        source: ShaderSource::Wgsl(include_str!("../shaders/overlay.wgsl").into()),
    })
}

pub fn combine_shader(device: &Device) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Combine Shader"),
        source: ShaderSource::Wgsl(include_str!("../shaders/combine.wgsl").into()),
    })
}
