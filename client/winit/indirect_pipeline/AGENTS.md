# Project Context
- This folder contains a single vulkan pipeline
- This is this an indirect rendering pipeline whose goal is to render the whole scene in a single draw call

# Architecture
- We should use a texture atlas that is dynamically streamed/updated
- We do cpu based culling to decide what will be drawn 
- We keep three gpu memory buffers.
-- A buffer for the meshes we need to draw
-- A buffer for the per mesh attributes
-- The DrawIndexedIndirectCommand
- We want to double buffer all the memory and update the next frame buffer while the current one is rendered

# Interface
- Out interface should be an array of handle_t to a texture and a transform to represent the sprites
- We expect to take the whole scene as an argument to the Draw call  