#version 450

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 matrix;
    vec3 light_pos;
    vec4 light_color;
} ubo;

layout(location = 0)  in vec4 frag_color;
layout(location = 1)  in vec3 frag_pos;
layout(location = 2)  in vec3 frag_normal;

layout(location = 0) out vec4 out_color;

void main() {
    vec3  light_dir      = ubo.light_pos - frag_pos;
    float light_distance = length(light_dir);

    vec3 norm           = normalize(frag_normal);
    vec3 unit_light_dir = normalize(light_dir);
    vec3 reflected_dir  = normalize(-reflect(unit_light_dir, norm));

    float attenuation = 1.0 / light_distance;
    vec4 light_color = ubo.light_color * attenuation;

    float diffuse_intensity = max(dot(norm, reflected_dir), 0.0);
          diffuse_intensity = pow(diffuse_intensity, 2);
          diffuse_intensity = step(0.25, diffuse_intensity);

    vec4 color = vec4(0.8, 0.8, 0.8, 0.0) +
                 light_color * diffuse_intensity;

    out_color = color * frag_color;
}