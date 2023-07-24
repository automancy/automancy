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
@group(0) @binding(6)
var kernel_texture: texture_1d<f32>;

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

const NOISE_SIZE = 4.0;
const KERNEL_SIZE = 32;
const KERNEL_SIZE_INV = 0.03125;

fn get_occlusion(frag: vec3<f32>, TBN: mat3x3<f32>, dim: vec2<f32>) -> f32 {
    var occlusion = 0.0;
    let scale = vec3(dim, 1.0);

    for (var i = 0; i < KERNEL_SIZE; i++) {
        let s = textureLoad(kernel_texture, i, 0).xyz;

        let sample_pos = frag + TBN * s * 0.5;

        var o = vec4(sample_pos, 1.0);
        o = ubo.world * o;

        var offset = (o.xy / o.w) * 0.5 + 0.5;
        offset = vec2(offset.x, 1.0 - offset.y);

        let sample_frag = textureLoad(position_texture, vec2<i32>(offset.xy * dim), 0);

        if (sample_frag.z >= sample_pos.z + 0.025) {
            occlusion += 1.0;
        }
    }

    return 1.0 - (occlusion * KERNEL_SIZE_INV);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(frame));
    let tex = vec2<i32>(in.uv * dim);
    let size = vec2<i32>(dim);

    let noise = textureSample(noise_texture, noise_sampler, in.uv * dim / NOISE_SIZE).xyz;

    let normal = textureLoad(normal_texture, tex, 0).xyz;
    let frag = textureLoad(position_texture, tex, 0).xyz;

    let tangent   = normalize(noise - normal * dot(noise, normal));
    let bitangent = cross(normal, tangent);
    let TBN       = mat3x3(tangent, bitangent, normal);

    let occlusion = get_occlusion(frag, TBN, dim);

    var color = textureLoad(frame, tex, 0);

    color *= occlusion * occlusion;

    return color;
}