#version 450

layout (location = 0) in vec3 pos;
layout (location = 1) in vec4 color;
layout (location = 2) in vec3 normal;

layout (location = 3) in vec4 color_offset;
layout (location = 4) in mat4 model_matrix;

layout (location = 8) in vec3 light_pos;
layout (location = 9) in vec4 light_color;

layout (location = 0) out vec4 frag_color;
layout (location = 1) out vec3 frag_pos;
layout (location = 2) out vec3 frag_normal;

layout (location = 3) out vec3 frag_light_pos;
layout (location = 4) out vec4 frag_light_color;

void main() {
    vec4 model_pos = model_matrix * vec4(pos, 1.0);

    gl_Position = model_pos;

    frag_color = ((1.0 - color_offset.a) * color) +
    ((color_offset.a) * color_offset);
    frag_pos = vec3(model_pos);
    frag_normal = normal;
    frag_light_pos = light_pos;
    frag_light_color = light_color;
}