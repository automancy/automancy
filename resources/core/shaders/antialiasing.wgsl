@group(0) @binding(0)
var frame_texture: texture_2d<f32>;
@group(0) @binding(1)
var frame_sampler: sampler;

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

const FXAA_SPAN_MAX: f32 = 4.0;
const FXAA_REDUCE_MIN: f32 = 0.0;
const FXAA_REDUCE_MUL: f32 = 1.0;
const LUMA = vec3<f32>(0.299, 0.587, 0.114);

fn fxaa(tex: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> vec3<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(tex));

    let c  = textureSample(tex, s, uv).rgb;
    let ne = textureSample(tex, s, uv + texel_size * vec2<f32>( 1.0,  1.0)).rgb;
    let nw = textureSample(tex, s, uv + texel_size * vec2<f32>(-1.0,  1.0)).rgb;
    let se = textureSample(tex, s, uv + texel_size * vec2<f32>( 1.0, -1.0)).rgb;
    let sw = textureSample(tex, s, uv + texel_size * vec2<f32>(-1.0, -1.0)).rgb;

    let luma_ne = dot(ne, LUMA);
    let luma_nw = dot(nw, LUMA);
    let luma_se = dot(se, LUMA);
    let luma_sw = dot(sw, LUMA);
    let luma_c  = dot(c,  LUMA);

    let luma_min = min(luma_c, min(min(luma_nw, luma_ne), min(luma_sw, luma_se)));
    let luma_max = max(luma_c, max(max(luma_nw, luma_ne), max(luma_sw, luma_se)));

    let dir = vec2(
         -((luma_nw + luma_ne) - (luma_sw + luma_se)),
          ((luma_nw + luma_sw) - (luma_ne + luma_se)),
    );

    let dir_reduce = max(
        (luma_nw + luma_ne + luma_sw + luma_se) * (0.25 * FXAA_REDUCE_MUL),
        FXAA_REDUCE_MIN
    );

    let texel_dir = min(
        vec2(FXAA_SPAN_MAX, FXAA_SPAN_MAX),
        max(
            vec2(-FXAA_SPAN_MAX, -FXAA_SPAN_MAX),
            dir / (min(abs(dir.x), abs(dir.y)) + dir_reduce)
        )
    ) * texel_size;

    let rgb_a = (1.0 / 2.0) * (
        textureSample(tex, s, uv + texel_dir * (1.0 / 3.0 - 0.5)).rgb +
        textureSample(tex, s, uv + texel_dir * (2.0 / 3.0 - 0.5)).rgb
    );

    let rgb_b = rgb_a * (1.0 / 2.0) + (1.0 / 4.0) * (
        textureSample(tex, s, uv + texel_dir * (0.0 / 3.0 - 0.5)).rgb +
        textureSample(tex, s, uv + texel_dir * (3.0 / 3.0 - 0.5)).rgb
    );

    let luma_b = dot(rgb_b, LUMA);

    if ((luma_b < luma_min) || (luma_b > luma_max)) {
        return rgb_a;
    } else {
        return rgb_b;
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(frame_texture, frame_sampler, in.uv);

    return vec4(fxaa(frame_texture, frame_sampler, in.uv), color.a);
}