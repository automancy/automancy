#version 450

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 matrix;
} ubo;

layout(location = 0)  in  vec3 pos;
layout(location = 1)  in  vec4 color;

layout(location = 2)  in  vec3 position_offset;
layout(location = 3)  in float scale;
layout(location = 4)  in  vec4 color_offset;

layout(location = 0) out  vec4 vertex_color;


void main() {
    float alpha_color_offset = color_offset[3];
    float i_alpha_color_offset = 1.0 - alpha_color_offset;

    gl_Position  = ubo.matrix * vec4((pos + position_offset) * scale, 1.0);
    vertex_color = (i_alpha_color_offset * color)
                 + (  alpha_color_offset * color_offset);
}