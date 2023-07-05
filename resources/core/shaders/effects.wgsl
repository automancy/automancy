@group(0) @binding(0)
var frame: texture_2d<f32>;

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

fn sobel(tex: vec2<i32>, size: vec2<i32>) -> f32 {
    let SX = mat3x3<f32>(
        1.0,  2.0,  1.0,
        0.0,  0.0,  0.0,
       -1.0, -2.0, -1.0
    );

    let SY = mat3x3<f32>(
        1.0, 0.0, -1.0,
        2.0, 0.0, -2.0,
        1.0, 0.0, -1.0
    );

    var m = mat3x3<f32>(
        1.0, 1.0, 1.0,
        1.0, 1.0, 1.0,
        1.0, 1.0, 1.0
    );

    for (var i = 0; i < 3; i++) {
        for (var j = 0; j < 3; j++) {
            let t = tex + vec2(i - 2, j - 2);

            let s = textureLoad(frame, clamp(t, vec2(0), size), 0).rgb;

            m[i][j] = length(s);
        }
    }
    let gx = dot(SX[0], m[0]) + dot(SX[1], m[1]) + dot(SX[2], m[2]);
    let gy = dot(SY[0], m[0]) + dot(SY[1], m[1]) + dot(SY[2], m[2]);

    let g = length(vec2(gx, gy));

    return smoothstep(0.3, 0.6, g);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(frame));
    let tex = vec2<i32>(in.uv * dim);
    let size = vec2<i32>(dim);

    var color = textureLoad(frame, tex, 0);
    let s = sobel(tex, size);

    color += color * vec4(vec3(s), 0.0);

    return color;
}