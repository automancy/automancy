struct Uniform {
    z_near: f32,
    z_far: f32,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

@group(0) @binding(1)
var frame_texture: texture_2d<f32>;
@group(0) @binding(2)
var frame_sampler: sampler;

@group(0) @binding(3)
var position_texture: texture_2d<f32>;
@group(0) @binding(4)
var position_sampler: sampler;

@group(0) @binding(5)
var normal_texture: texture_2d<f32>;
@group(0) @binding(6)
var normal_sampler: sampler;

@group(0) @binding(7)
var depth_texture: texture_2d<f32>;
@group(0) @binding(8)
var depth_sampler: sampler;


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

const EDGE_THRESHOLD: f32 = 0.00001;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(frame_texture, frame_sampler, in.uv);

    let normal = textureSample(normal_texture, normal_sampler, in.uv).xyz;
    let frag = textureSample(position_texture, position_sampler, in.uv).xyz;

    let depth_texel_size = 1.0 / vec2<f32>(textureDimensions(depth_texture));

    let n  = textureSample(depth_texture, depth_sampler, in.uv + depth_texel_size * vec2<f32>( 0.0,  1.0)).x;
    let e  = textureSample(depth_texture, depth_sampler, in.uv + depth_texel_size * vec2<f32>( 1.0,  0.0)).x;
    let s  = textureSample(depth_texture, depth_sampler, in.uv + depth_texel_size * vec2<f32>( 0.0, -1.0)).x;
    let w  = textureSample(depth_texture, depth_sampler, in.uv + depth_texel_size * vec2<f32>(-1.0,  0.0)).x;
    let ne = textureSample(depth_texture, depth_sampler, in.uv + depth_texel_size * vec2<f32>( 1.0,  1.0)).x;
    let nw = textureSample(depth_texture, depth_sampler, in.uv + depth_texel_size * vec2<f32>(-1.0,  1.0)).x;
    let se = textureSample(depth_texture, depth_sampler, in.uv + depth_texel_size * vec2<f32>( 1.0, -1.0)).x;
    let sw = textureSample(depth_texture, depth_sampler, in.uv + depth_texel_size * vec2<f32>(-1.0, -1.0)).x;
    
    if ( n -  s > EDGE_THRESHOLD ||  w -  e > EDGE_THRESHOLD ||  e -  w > EDGE_THRESHOLD ||  s -  n > EDGE_THRESHOLD) {
        color *= 0.3;
    }

    if (nw - se > EDGE_THRESHOLD || ne - sw > EDGE_THRESHOLD || se - nw > EDGE_THRESHOLD || sw - ne > EDGE_THRESHOLD) {
        color *= 0.3;
    }

    return color;
}