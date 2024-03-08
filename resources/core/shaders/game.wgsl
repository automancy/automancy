struct Uniform {
    light_color: vec4<f32>,
}

struct MatrixData {
    model_matrix: mat4x4<f32>,
    world_matrix: mat4x4<f32>,
    normal_matrix: mat3x3<f32>,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

@group(0) @binding(1)
var<storage, read> matrix_data: array<MatrixData>;

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
}

struct InstanceInput {
    @location(3) color_offset: vec4<f32>,
    @location(4) alpha: f32,
    @location(5) light_pos: vec4<f32>,
    @location(6) matrix_index: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light_pos: vec4<f32>,
    @location(3) model_pos: vec3<f32>,
}

@vertex
fn vs_main(
    in: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let m = matrix_data[instance.matrix_index];
    let model_matrix  = m.model_matrix;
    let normal_matrix = m.normal_matrix;
    let world_matrix  = m.world_matrix;

    let model_pos = model_matrix * vec4(in.pos, 1.0);

    out.model_pos = model_pos.xyz / model_pos.w;
    out.pos = world_matrix * model_pos;
    out.normal = normalize(normal_matrix * in.normal);

    out.color = vec4(mix(instance.color_offset.rgb, in.color.rgb, in.color.a - instance.color_offset.a), instance.alpha * in.color.a);
    out.light_pos = instance.light_pos;

    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) model: vec4<f32>,
}

const TINT_DIR = vec3<f32>(-0.348155, 0.348155, 0.870388);

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let light_pos = in.light_pos.xyz;
    let light_dir = normalize(light_pos - in.model_pos);

    let diff = pow(max(0.0, dot(light_dir, in.normal)), 0.420) * 0.9;
    let spec = pow(max(0.0, dot(reflect(-light_dir, in.normal), light_dir)), 2.0) * 0.15;
    let tint = pow(max(0.0, dot(in.normal, TINT_DIR)), 16.0) * 0.69;
    let intensity = max(0.25, diff + spec + tint);

    let lighting = ubo.light_color.rgb * max(1.0 - in.light_pos.w, intensity * in.light_pos.w);

    var out: FragmentOutput;

    out.color = vec4(in.color.rgb * lighting, in.color.a);
    out.normal = vec4(in.normal, 0.0);
    out.model = vec4(in.model_pos, 0.0);

    return out;
}