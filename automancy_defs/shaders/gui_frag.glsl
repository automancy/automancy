#version 450

layout (location = 0) in vec4 frag_color;
layout (location = 1) in vec3 frag_pos;
layout (location = 2) in vec3 frag_normal;

layout (location = 3) in vec3 light_pos;
layout (location = 4) in vec4 light_color;

layout (location = 0) out vec4 out_color;

void main() {
    vec4 light_color = light_color * 0.15;

    vec3 light_dir = light_pos - frag_pos;
    float light_distance = length(light_dir);

    vec3 norm = normalize(frag_normal);
    vec3 reflected = -reflect(normalize(light_dir), norm);
    vec3 eye = normalize(-frag_pos);
    vec3 halfway = normalize(light_dir + eye);

    float diffuse_intensity = max(dot(norm, reflected), 0.0);
    diffuse_intensity = pow(diffuse_intensity, 4.0);
    vec4 diffuse = light_color * diffuse_intensity;

    float specular_intensity = dot(norm, halfway);
    vec4 specular = light_color * specular_intensity;

    vec4 color = vec4(0.5) + diffuse + specular;

    out_color = color * frag_color;
}