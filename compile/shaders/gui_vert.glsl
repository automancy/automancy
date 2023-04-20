#version 450

layout(binding = 0) uniform UniformBufferObject {
    mat4 matrix;
} ubo;

layout(location = 0)  in  vec3 pos;
layout(location = 1)  in  vec4 color;

layout(location = 3)  in  vec4 color_offset;
layout(location = 4)  in  mat4 model_matrix;

layout(location = 0) out  vec4 frag_color;

void main() {
    gl_Position = ubo.matrix * model_matrix * vec4(pos, 1.0);
    frag_color  = ((1.0 - color_offset.a) * color) +
                  ((      color_offset.a) * color_offset);
}