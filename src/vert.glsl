#version 450

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view;
} ubo;

layout(location = 0)  in  vec3 pos;
layout(location = 1)  in  vec4 color;

layout(location = 2)  in  vec3 position_offset;
layout(location = 3)  in float scale;

layout(location = 0) out  vec4 vertex_color;

void main() {
    gl_Position  = ubo.view * vec4((pos + position_offset) * scale, 1.0);
    vertex_color = color;
}