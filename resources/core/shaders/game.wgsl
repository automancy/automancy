struct Uniform {
    light_color: vec4<f32>,
    world: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> ubo: Uniform;

struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
}

struct InstanceInput {
    @location(3) color_offset: vec4<f32>,
    @location(4) light_pos: vec4<f32>,

    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) model_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light_pos: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
       instance.model_matrix_0,
       instance.model_matrix_1,
       instance.model_matrix_2,
       instance.model_matrix_3,
   );

    var out: VertexOutput;

    let model_pos = model_matrix * vec4(model.pos, 1.0);
    out.pos = ubo.world * model_pos;
    out.model_pos = model_pos.xyz / model_pos.w;
    out.normal = model.normal;

    out.color = vec4(mix(instance.color_offset.rgb, model.color.rgb, model.color.a - instance.color_offset.a), abs(model.color.a - instance.color_offset.a));
    out.light_pos = instance.light_pos;

    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let light_dir = normalize(in.light_pos.xyz - in.model_pos);
    let reflect_dir = reflect(-light_dir, in.normal);

    let diffuse = round(max(dot(in.normal, light_dir), 0.0) * 64.0) / 32.0;
    let specular = max(dot(light_dir, reflect_dir), 0.0);

    var out: FragmentOutput;

    out.color = vec4(in.color.rgb * ubo.light_color.rgb * min(0.25 + mix(diffuse, specular, 0.5), ubo.light_color.a), in.color.a);
    out.normal = vec4(in.normal, 0.0);

    return out;
}