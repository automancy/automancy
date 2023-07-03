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

fn sobel(tex: vec2<i32>) -> vec3<f32> {
    let sx = mat3x3<f32>(
        1.0,  2.0,  1.0,
        0.0,  0.0,  0.0,
       -1.0, -2.0, -1.0
    );

    let sy = mat3x3<f32>(
        1.0, 0.0, -1.0,
        2.0, 0.0, -2.0,
        1.0, 0.0, -1.0
    );

    var m = mat3x3<f32>();

    for (var i = 0; i < 3; i++) {
        for (var j = 0; j < 3; j++) {
            let s0 = textureLoad(frame, tex + vec2(i - 2, j - 2) + vec2( 0,  0), 0).rgb;
            let s1 = textureLoad(frame, tex + vec2(i - 2, j - 2) + vec2(-1, -1), 0).rgb;
            let s2 = textureLoad(frame, tex + vec2(i - 2, j - 2) + vec2(-1,  1), 0).rgb;
            let s3 = textureLoad(frame, tex + vec2(i - 2, j - 2) + vec2( 1,  1), 0).rgb;
            let s4 = textureLoad(frame, tex + vec2(i - 2, j - 2) + vec2( 1, -1), 0).rgb;

            let m0 = (s1 + s3) / 2.0;
            let m1 = (s2 + s4) / 2.0;
            let m2 = (m0 + m1) / 2.0;

            m[i][j] = length((s0 + m2) / 2.0);
        }
    }
    let gx = dot(sx[0], m[0]) + dot(sx[1], m[1]) + dot(sx[2], m[2]);
    let gy = dot(sy[0], m[0]) + dot(sy[1], m[1]) + dot(sy[2], m[2]);

    let g = sqrt(pow(gx, 2.0) + pow(gy, 2.0));

    return vec3(1.0) * smoothstep(0.3, 0.8, g);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(frame));
    let tex = vec2<i32>(in.uv * dim);

    var color = textureLoad(frame, tex, 0);

    color += color * vec4(sobel(tex), 0.0);

    return color;
}