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

    vec4 light_color = ubo.light_color * 0.05;

    float diffuse_intensity = max(dot(norm, reflected), 0.0);
    diffuse_intensity = pow(diffuse_intensity, 2.0);
    diffuse_intensity = step(0.25, diffuse_intensity);
    vec4 diffuse = light_color * diffuse_intensity;

    float specular_intensity = dot(norm, halfway);
    specular_intensity = step(0.6, specular_intensity);
    vec4 specular = light_color * specular_intensity;

    float rim_intensity = dot(eye, norm);
    rim_intensity = max(1.0 - rim_intensity, 0.0);
    rim_intensity = smoothstep(0.3, 0.4, rim_intensity);
    vec4 rim = rim_intensity * diffuse;

    vec4 color = vec4(0.8, 0.8, 0.8, 0.8) + rim + diffuse + specular;

    out_color = (vec4(color.rgb, 1.0) * color.a) * frag_color;
}