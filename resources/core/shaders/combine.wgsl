@group(0) @binding(0)
var processed_game_texture: texture_2d<f32>;
@group(0) @binding(1)
var processed_game_sampler: sampler;

@group(0) @binding(2)
var gui_texture: texture_2d<f32>;
@group(0) @binding(3)
var gui_sampler: sampler;

@group(0) @binding(4)
var egui_texture: texture_2d<f32>;
@group(0) @binding(5)
var egui_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) idx: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    let uv = vec2(
         f32((in.idx << 1u) & 2u),
         f32(in.idx & 2u)
     );

    out.pos = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2(uv.x, 1.0 - uv.y);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let game = textureSample(processed_game_texture, processed_game_sampler, in.uv);
    let gui = textureSample(gui_texture, gui_sampler, in.uv);
    var egui = textureSample(egui_texture, egui_sampler, in.uv);

    return
        game * min(1.0 - gui.a, 1.0 - egui.a) +
        gui +
        egui * (1.0 - gui.a);
}