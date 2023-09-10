@group(0) @binding(0)
var frame_texture: texture_2d<f32>;

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

    return vec4(x, y, z, w) / 6.0;
}


fn textureBicubic(uv: vec2<f32>) -> vec4<f32> {
    let dim = vec2<f32>(textureDimensions(frame_texture));

    var coord = uv * dim - 0.5;

    let fxy = fract(coord);
    coord -= fxy;

    let xcubic = cubic(fxy.x);
    let ycubic = cubic(fxy.y);

    let s = vec4(xcubic.xz + xcubic.yw, ycubic.xz + ycubic.yw);
    let offset = clamp(vec4(0.0), (coord.xxyy + vec2(-0.5, 1.5).xyxy) + vec4(xcubic.yw, ycubic.yw) / s, dim.xxyy);

    let a = textureLoad(frame_texture, vec2<i32>(offset.xz), 0);
    let b = textureLoad(frame_texture, vec2<i32>(offset.yz), 0);
    let c = textureLoad(frame_texture, vec2<i32>(offset.xw), 0);
    let d = textureLoad(frame_texture, vec2<i32>(offset.yw), 0);

    let sx = s.x / (s.x + s.y);
    let sy = s.z / (s.z + s.w);

    return mix(mix(d, c, sx), mix(b, a, sx), sy);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureBicubic(in.uv);
}