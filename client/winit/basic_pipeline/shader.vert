#version 450

layout(set = 0, binding = 0) uniform UniformBuffer {
    mat4 view_proj;
} ubo;

layout(location = 0) in vec4 inPosition;
layout(location = 1) in vec2 inUV;

layout(location = 0) out vec2 fragUV;

void main() {
    gl_Position = ubo.view_proj * inPosition;
    fragUV = inUV;
}