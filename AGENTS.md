# Project Context
- This is a game made in rust and vulkan that is multiplatform and can compile to wasm or native code
- It is an isometric dungeon crawler similar to diablo
- It is a multiplayer game 
- Visually it is an anime style pixel art game

# Art Style
- Characters should be cute and appealing 
- Prefer female characters
- Sprites should be at most 128x128


# Architecture
- This is a pure bazel project, everything should be done through bazel
- The core components are found in the common folder 
- handle_t is our abstraction over a pointer
- many differend containers can expose data through the common handle_t
- versioned_buffer is the main data structure used as interface between sub components


# Guidelines
- We prefer to use bazel gen-rules for compile time generation over making complex abstractions in code 