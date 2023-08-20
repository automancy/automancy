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

    out.color = vec4(model.color.rgb + instance.color_offset.rgb, model.color.a * instance.color_offset.a);
    out.light_pos = instance.light_pos;

    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let light_color = ubo.light_color.rgb * 0.15;
    let light_dir = in.light_pos.xyz - in.model_pos;

    let reflected = -reflect(normalize(light_dir), in.normal);
    let eye = normalize(-in.model_pos);
    let halfway = normalize(light_dir + eye);

    var diffuse_intensity = max(dot(in.normal, reflected), 0.0);
    diffuse_intensity = pow(diffuse_intensity, 4.0);
    let diffuse = light_color * diffuse_intensity;

    let specular_intensity = dot(in.normal, halfway);
    let specular = light_color * specular_intensity;

    var out: FragmentOutput;

    out.color = vec4(in.color.rgb * (vec3(0.5) + diffuse + specular), in.color.a);
    out.normal = vec4(in.normal, 0.0);

    return out;
}