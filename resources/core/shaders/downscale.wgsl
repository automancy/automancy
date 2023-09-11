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

fn cubic(v: f32) -> vec4<f32> {
    let n = vec4(1.0, 2.0, 3.0, 4.0) - v;
    let s = n * n * n;
    let x = s.x;
    let y = s.y - 4.0 * s.x;
    let z = s.z - 4.0 * s.y + 6.0 * s.x;
    let w = 6.0 - x - y - z;

    return vec4(x, y, z, w) * 0.1666666666666667;
}

fn textureSampleBicubic(t: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> vec4<f32> {
    let dim = vec2<f32>(textureDimensions(t));

    let coord = (uv * dim) - vec2(0.5);
    let fxy = fract(coord);
    let ixy = floor(coord);

    let xcubic = cubic(fxy.x);
    let ycubic = cubic(fxy.y);

    let center = ixy.xxyy + vec2(-0.5, 1.5).xyxy;
    let size = vec4(xcubic.xz + xcubic.yw, ycubic.xz + ycubic.yw);
    let offset = (center + vec4(xcubic.yw, ycubic.yw) / size) / dim.xxyy;

    let sample0 = textureSample(t, s, offset.xz);
    let sample1 = textureSample(t, s, offset.yz);
    let sample2 = textureSample(t, s, offset.xw);
    let sample3 = textureSample(t, s, offset.yw);

    let sx = size.x / (size.x + size.y);
    let sy = size.z / (size.z + size.w);

    return mix(mix(sample3, sample2, sx), mix(sample1, sample0, sx), sy);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleBicubic(frame_texture, frame_sampler, in.uv);
}