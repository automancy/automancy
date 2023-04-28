#version 450

layout (binding = 0) uniform UniformBufferObject {
    mat4 matrix;
} ubo;

layout (location = 0) in vec3 pos;
layout (location = 1) in vec4 color;

layout (location = 0) out vec4 frag_color;


void main() {
    gl_Position = ubo.matrix * vec4(pos, 1.0);
    frag_color = color;
}