@group(0) @binding(0)
var a_texture: texture_2d<f32>;
@group(0) @binding(1)
var a_sampler: sampler;

@group(0) @binding(2)
var b_texture: texture_2d<f32>;
@group(0) @binding(3)
var b_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) idx: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
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
    let a = textureSample(a_texture, a_sampler, in.uv);
    let b  = textureSample(b_texture, b_sampler, in.uv);

    return
        a * (1.0 - b.a) +
        b;
}