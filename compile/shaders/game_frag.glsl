#version 450

layout (binding = 0) uniform UniformBufferObject {
    mat4 matrix;
    vec3 light_pos;
    vec4 light_color;
} ubo;

layout (location = 0) in vec4 frag_color;
layout (location = 1) in vec3 frag_pos;
layout (location = 2) in vec3 frag_normal;

layout (location = 0) out vec4 out_color;

void main() {
    vec3 light_dir = ubo.light_pos - frag_pos;
    float light_distance = length(light_dir);

    vec3 norm = normalize(frag_normal);
    vec3 unit_light = normalize(light_dir);
    vec3 reflected = normalize(-reflect(unit_light, norm));
    vec3 eye = normalize(-frag_pos);
    vec3 halfway = normalize(light_dir + eye);

    vec4 light_color = ubo.light_color * 0.15;

    float diffuse_intensity = max(dot(norm, reflected), 0.0);
    diffuse_intensity = pow(diffuse_intensity, 8.0);
    vec4 diffuse = light_color * diffuse_intensity;

    float specular_intensity = dot(norm, halfway);
    specular_intensity = pow(specular_intensity, 4.0);
    vec4 specular = light_color * specular_intensity;

    vec4 color = vec4(0.5) + diffuse + specular;

    //out_color = vec4(frag_normal, 1.0);
    out_color = color * frag_color;
}