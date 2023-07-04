struct Uniform {
    world: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
    //@location(2) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    //@location(1) model_pos: vec3<f32>,
    //@location(2) normal: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.pos = ubo.world * vec4(model.pos, 1.0);
    out.color = model.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}