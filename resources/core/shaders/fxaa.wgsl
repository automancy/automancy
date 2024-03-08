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

const FXAA_SPAN_MAX: f32 = 2.0;
const FXAA_REDUCE_MIN: f32 = 0.0078125;
const FXAA_REDUCE_MUL: f32 = 0.125;
const LUMA = vec3<f32>(0.299, 0.587, 0.114);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(frame_texture));

    let  c = textureSample(frame_texture, frame_sampler, in.uv);
    let ne = textureSample(frame_texture, frame_sampler, in.uv + texel_size * vec2<f32>( 1.0,  1.0));
    let nw = textureSample(frame_texture, frame_sampler, in.uv + texel_size * vec2<f32>(-1.0,  1.0));
    let se = textureSample(frame_texture, frame_sampler, in.uv + texel_size * vec2<f32>( 1.0, -1.0));
    let sw = textureSample(frame_texture, frame_sampler, in.uv + texel_size * vec2<f32>(-1.0, -1.0));

    let luma_c  = dot( c.rgb, LUMA);
    let luma_ne = dot(ne.rgb, LUMA);
    let luma_nw = dot(nw.rgb, LUMA);
    let luma_se = dot(se.rgb, LUMA);
    let luma_sw = dot(sw.rgb, LUMA);

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

    let texel_dir = clamp(
        dir / (min(abs(dir.x), abs(dir.y)) + dir_reduce),
        vec2(-FXAA_SPAN_MAX, -FXAA_SPAN_MAX),
        vec2(FXAA_SPAN_MAX, FXAA_SPAN_MAX)
    ) * texel_size;

    let rgba_a = (1.0 / 2.0) * (
        textureSample(frame_texture, frame_sampler, in.uv + texel_dir * (1.0 / 3.0 - 0.5)) +
        textureSample(frame_texture, frame_sampler, in.uv + texel_dir * (2.0 / 3.0 - 0.5))
    );

    let rgba_b = rgba_a * (1.0 / 2.0) + (1.0 / 4.0) * (
        textureSample(frame_texture, frame_sampler, in.uv + texel_dir * (0.0 / 3.0 - 0.5)) +
        textureSample(frame_texture, frame_sampler, in.uv + texel_dir * (3.0 / 3.0 - 0.5))
    );

    let luma_b = dot(rgba_b.rgb, LUMA);

    if ((luma_b < luma_min) || (luma_b > luma_max)) {
        return rgba_a;
    } else {
        return rgba_b;
    }
}