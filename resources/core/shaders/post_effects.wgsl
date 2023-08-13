struct Uniform {
    depth_threshold: f32,
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

const THRESHOLD: f32 = 0.05;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(frame_texture, frame_sampler, in.uv);

    let texel_size = 1.0 / vec2<f32>(textureDimensions(normal_texture));

    let c =  textureSample(normal_texture, normal_sampler, in.uv);
    let n  = dot(c, textureSample(normal_texture, normal_sampler, in.uv + texel_size * vec2<f32>( 0.0,  1.0)));
    let e  = dot(c, textureSample(normal_texture, normal_sampler, in.uv + texel_size * vec2<f32>( 1.0,  0.0)));
    let s  = dot(c, textureSample(normal_texture, normal_sampler, in.uv + texel_size * vec2<f32>( 0.0, -1.0)));
    let w  = dot(c, textureSample(normal_texture, normal_sampler, in.uv + texel_size * vec2<f32>(-1.0,  0.0)));
    let ne = dot(c, textureSample(normal_texture, normal_sampler, in.uv + texel_size * vec2<f32>( 1.0,  1.0)));
    let nw = dot(c, textureSample(normal_texture, normal_sampler, in.uv + texel_size * vec2<f32>(-1.0,  1.0)));
    let se = dot(c, textureSample(normal_texture, normal_sampler, in.uv + texel_size * vec2<f32>( 1.0, -1.0)));
    let sw = dot(c, textureSample(normal_texture, normal_sampler, in.uv + texel_size * vec2<f32>(-1.0, -1.0)));

    if ( n -  s > THRESHOLD ||  w -  e > THRESHOLD ||  e -  w > THRESHOLD ||  s -  n > THRESHOLD
     || nw - se > THRESHOLD || ne - sw > THRESHOLD || se - nw > THRESHOLD || sw - ne > THRESHOLD) {
        color -= color * vec4(vec3(0.4), 0.0);
    }

    return color;
}