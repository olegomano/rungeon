#version 450

layout(location = 0) in vec2 fragUV;

layout(set = 0, binding = 1) uniform sampler2D texAtlas;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 texColor = texture(texAtlas, fragUV);
    // Discard fully transparent pixels so sprites don't write depth/blend over each other
    if (texColor.a < 0.01) {
        discard;
    }
    outColor = texColor;
}
