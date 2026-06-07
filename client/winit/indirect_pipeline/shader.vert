#version 450

// Per-vertex attributes (the quad geometry)
layout(location = 0) in vec4 inPosition;  // x, y, z, w
layout(location = 1) in vec2 inUV;        // u, v

// Per-instance attributes (one per sprite)
// mat4 occupies locations 2,3,4,5 (one vec4 per column)
layout(location = 2) in mat4 inModel;
// Atlas sub-rect: (u_offset, v_offset, u_scale, v_scale)
layout(location = 6) in vec4 inAtlasRect;

// Camera uniform
layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view;
    mat4 projection;
} camera;

layout(location = 0) out vec2 fragUV;

void main() {
    gl_Position = camera.projection * camera.view * inModel * vec4(inPosition.xyz, 1.0);

    // Remap quad UVs into the atlas sub-rect
    fragUV = inAtlasRect.xy + inUV * inAtlasRect.zw;
}
