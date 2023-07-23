struct Uniform {
    light_color: vec4<f32>,
    world: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

@group(0) @binding(1)
var frame: texture_2d<f32>;
@group(0) @binding(2)
var position_texture: texture_2d<f32>;
@group(0) @binding(3)
var normal_texture: texture_2d<f32>;
@group(0) @binding(4)
var noise_texture: texture_2d<f32>;
@group(0) @binding(5)
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

const SOBEL_X = mat3x3<f32>(
    vec3<f32>( 1.0,  2.0,  1.0),
    vec3<f32>( 0.0,  0.0,  0.0),
    vec3<f32>(-1.0, -2.0, -1.0),
);

const SOBEL_Y = mat3x3<f32>(
    vec3<f32>( 1.0,  0.0, -1.0),
    vec3<f32>( 2.0,  0.0, -2.0),
    vec3<f32>( 1.0,  0.0, -1.0),
);

fn sobel(tex: vec2<i32>, size: vec2<i32>) -> f32 {
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
    let gx = dot(SOBEL_X[0], m[0]) + dot(SOBEL_X[1], m[1]) + dot(SOBEL_X[2], m[2]);
    let gy = dot(SOBEL_Y[0], m[0]) + dot(SOBEL_Y[1], m[1]) + dot(SOBEL_Y[2], m[2]);

    let g = length(vec2(gx, gy));

    return smoothstep(0.3, 0.6, g);
}

const NOISE_SIZE = 4;

const KERNEL_SIZE = 32;

// TODO use a fucking texture?
const SAMPLES = array<vec3<f32>, KERNEL_SIZE>(
    vec3<f32>(0.07254281, 0.046630185, 0.05062774),
    vec3<f32>(-0.017220352, 0.030811489, 0.09356341),
    vec3<f32>(-0.02349218, 0.081091926, 0.05359307),
    vec3<f32>(0.0836956, 0.046090826, 0.029507339),
    vec3<f32>(-0.04077291, -0.08355418, 0.036827564),
    vec3<f32>(0.09710225, 0.023125265, 0.0060314154),
    vec3<f32>(-0.0471211, 0.08602538, 0.01947401),
    vec3<f32>(-0.08569633, -0.026361603, 0.044285472),
    vec3<f32>(0.014087995, -0.093629174, 0.03217306),
    vec3<f32>(0.09513675, -0.027729312, 0.013419534),
    vec3<f32>(0.033061486, 0.061321355, 0.071740024),
    vec3<f32>(0.046657324, -0.054629005, 0.069561236),
    vec3<f32>(0.01876323, -0.08833922, 0.042943276),
    vec3<f32>(-0.096585125, 0.0039059184, 0.0256136),
    vec3<f32>(-0.05166439, 0.08561063, 0.0012688864),
    vec3<f32>(0.074472204, -0.016265828, 0.064724915),
    vec3<f32>(0.029056165, 0.029084323, 0.09115833),
    vec3<f32>(-0.09785393, -0.01334513, 0.015700815),
    vec3<f32>(0.09313943, -0.03243034, 0.016532337),
    vec3<f32>(0.09949589, 0.0014346336, 0.009925237),
    vec3<f32>(0.021845676, -0.047324594, 0.08534137),
    vec3<f32>(0.028870692, -0.06568622, 0.069654904),
    vec3<f32>(0.051971108, 0.030145064, 0.07993922),
    vec3<f32>(0.056093276, -0.06651198, 0.049292),
    vec3<f32>(-0.041101094, -0.0038475764, 0.09108182),
    vec3<f32>(-0.021047466, -0.061807156, 0.07574219),
    vec3<f32>(0.06053097, 0.07670124, 0.021281952),
    vec3<f32>(-0.03145137, -0.08563602, 0.040954646),
    vec3<f32>(0.0805923, 0.026875192, 0.052750416),
    vec3<f32>(-0.008007409, -0.028499067, 0.095517986),
    vec3<f32>(-0.042221442, 0.09053062, 0.0046429625),
    vec3<f32>(0.010390808, 0.060059454, 0.07927733),
);

fn get_occlusion(frag: vec3<f32>, TBN: mat3x3<f32>, dim: vec2<f32>, s: vec3<f32>) -> f32 {
    let sample_pos = frag + TBN * s * 0.5;

    var o = vec4(sample_pos, 1.0);
    o = ubo.world * o;

    var offset = (o.xy / o.w) * 0.5 + 0.5;
    offset = vec2(offset.x, 1.0 - offset.y);

    let sample_frag = textureLoad(position_texture, vec2<i32>(offset.xy * dim), 0);

    if (sample_frag.z >= sample_pos.z + 0.05) {
        return 1.0;
    } else {
        return 0.0;
    }
}

fn get_occlusions(frag: vec3<f32>, TBN: mat3x3<f32>, dim: vec2<f32>) -> f32 {
    var occlusion =   get_occlusion(frag, TBN, dim, SAMPLES[ 0])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 1])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 2])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 3])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 4])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 5])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 6])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 7])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 8])
                    + get_occlusion(frag, TBN, dim, SAMPLES[ 9])
                    + get_occlusion(frag, TBN, dim, SAMPLES[10])
                    + get_occlusion(frag, TBN, dim, SAMPLES[11])
                    + get_occlusion(frag, TBN, dim, SAMPLES[12])
                    + get_occlusion(frag, TBN, dim, SAMPLES[13])
                    + get_occlusion(frag, TBN, dim, SAMPLES[14])
                    + get_occlusion(frag, TBN, dim, SAMPLES[15])
                    + get_occlusion(frag, TBN, dim, SAMPLES[16])
                    + get_occlusion(frag, TBN, dim, SAMPLES[17])
                    + get_occlusion(frag, TBN, dim, SAMPLES[18])
                    + get_occlusion(frag, TBN, dim, SAMPLES[19])
                    + get_occlusion(frag, TBN, dim, SAMPLES[20])
                    + get_occlusion(frag, TBN, dim, SAMPLES[21])
                    + get_occlusion(frag, TBN, dim, SAMPLES[22])
                    + get_occlusion(frag, TBN, dim, SAMPLES[23])
                    + get_occlusion(frag, TBN, dim, SAMPLES[24])
                    + get_occlusion(frag, TBN, dim, SAMPLES[25])
                    + get_occlusion(frag, TBN, dim, SAMPLES[26])
                    + get_occlusion(frag, TBN, dim, SAMPLES[27])
                    + get_occlusion(frag, TBN, dim, SAMPLES[28])
                    + get_occlusion(frag, TBN, dim, SAMPLES[29])
                    + get_occlusion(frag, TBN, dim, SAMPLES[30])
                    + get_occlusion(frag, TBN, dim, SAMPLES[31]);

    return 1.0 - (occlusion / f32(KERNEL_SIZE));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(frame));
    let tex = vec2<i32>(in.uv * dim);
    let size = vec2<i32>(dim);

    let frag = textureLoad(position_texture, tex, 0).xyz;
    let normal = textureLoad(normal_texture, tex, 0).xyz;
    let noise = textureSample(noise_texture, noise_sampler, in.uv * (dim / f32(NOISE_SIZE))).xyz;

    let tangent   = normalize(noise - normal * dot(noise, normal));
    let bitangent = cross(normal, tangent);
    let TBN       = mat3x3(tangent, bitangent, normal);

    let occlusion = get_occlusions(frag, TBN, dim);

    var color = textureLoad(frame, tex, 0);

    let s = sobel(tex, size);

    color += color * vec4(vec3(s), 0.0);
    color *= pow(occlusion, 2.0);

    return color;
}