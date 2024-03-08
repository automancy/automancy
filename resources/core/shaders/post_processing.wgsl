@group(0) @binding(0)
var frame_texture: texture_2d<f32>;
@group(0) @binding(1)
var frame_sampler: sampler;
@group(0) @binding(2)
var depth_texture: texture_2d<f32>;
@group(0) @binding(3)
var depth_sampler: sampler;
@group(0) @binding(4)
var normal_texture: texture_2d<f32>;
@group(0) @binding(5)
var normal_sampler: sampler;
@group(0) @binding(6)
var model_texture: texture_2d<f32>;
@group(0) @binding(7)
var model_sampler: sampler;
@group(0) @binding(8)
var noise_texture: texture_2d<f32>;
@group(0) @binding(9)
var noise_sampler: sampler;

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

const SSAO_INTENSITY: f32 = 1.5;
const SSAO_SAMPLE_RADIUS: f32 = 3.0;

const SSAO_SAMPLES = array<vec2<f32>, 4>(
    vec2( 1.0,  0.0),
    vec2(-1.0,  0.0),
    vec2( 0.0,  1.0),
    vec2( 0.0, -1.0),
);
const SSAO_NOISE_SIZE: f32 = 64.0;

fn ssao_single(uv: vec2<f32>, model: vec3<f32>, normal: vec3<f32>) -> f32 {
    let diff = textureSample(model_texture, model_sampler, uv).xyz - model;
    let v = normalize(diff);

    return max(0.0, dot(normal, v)) * SSAO_INTENSITY;
}

fn ssao_one_sample(dir: vec2<f32>, noise: vec2<f32>, texel_size: vec2<f32>, uv: vec2<f32>, model: vec3<f32>, normal: vec3<f32>) -> f32 {
    let coord1 = reflect(dir, noise) * SSAO_SAMPLE_RADIUS;
    let coord2 = vec2(coord1.x * 0.707 - coord1.y * 0.707, coord1.x * 0.707 + coord1.y * 0.707);

    return (
        ssao_single(uv + texel_size * coord1 * 0.25, model, normal) +
        ssao_single(uv + texel_size * coord1 * 0.5 , model, normal) +
        ssao_single(uv + texel_size * coord1 * 0.75, model, normal) +
        ssao_single(uv + texel_size * coord1       , model, normal) +
        ssao_single(uv + texel_size * coord2 * 0.25, model, normal) +
        ssao_single(uv + texel_size * coord2 * 0.5 , model, normal) +
        ssao_single(uv + texel_size * coord2 * 0.75, model, normal) +
        ssao_single(uv + texel_size * coord2       , model, normal)
    ) / 8.0;
}

fn ssao(uv: vec2<f32>) -> f32 {
    let texture_dim = vec2<f32>(textureDimensions(model_texture));
    let texel_size = 1.0 / texture_dim;

    let model = textureSample(model_texture, model_sampler, uv).xyz;
    let normal = textureSample(normal_texture, normal_sampler, uv).xyz;

    let noise = textureSample(noise_texture, noise_sampler, (uv * texture_dim) / SSAO_NOISE_SIZE).xy;

    var ao = 0.0f;
    ao += ssao_one_sample(SSAO_SAMPLES[0], noise, texel_size, uv, model, normal);
    ao += ssao_one_sample(SSAO_SAMPLES[1], noise, texel_size, uv, model, normal);
    ao += ssao_one_sample(SSAO_SAMPLES[2], noise, texel_size, uv, model, normal);
    ao += ssao_one_sample(SSAO_SAMPLES[3], noise, texel_size, uv, model, normal);
    ao /= 4.0;

    return 1.0 - smoothstep(0.03, 1.0, ao);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance_vector = in.uv - vec2(0.5);
    let distance = dot(distance_vector, distance_vector);
    let offset_strength = 0.005 * smoothstep(0.0, 1.0, distance);
    let edge_darken = smoothstep(0.0, 1.0, (1.0 - distance)) * 0.5;
    let chroma_abbr = vec4(
        textureSample(frame_texture, frame_sampler, in.uv + vec2<f32>(offset_strength, -offset_strength)).r,
        textureSample(frame_texture, frame_sampler, in.uv + vec2<f32>(-offset_strength, 0.0)).g,
        textureSample(frame_texture, frame_sampler, in.uv + vec2<f32>(0.0, offset_strength)).b,
        0.0
    );

    return (textureSample(frame_texture, frame_sampler, in.uv) + chroma_abbr) * vec4(vec3(ssao(in.uv) * edge_darken), 1.0);
}