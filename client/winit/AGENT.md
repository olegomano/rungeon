# Project Context
- This folder contains the implementation fo the render stack of the game
- main.rs is a debug executable that can test out the render pipeplines
- This folder should have no dependencies on folders above it

# Architecture
- We have multiple render pipelines each inside their own subfolder
- The renderer should have no specific knowledge of the game itself
- Pipelines should not be directly referneced by the upper layers


# main.rs
- This is a debug executable that can be used to test out the render pipelines
- It should present some CLI arguments to configure which render pipeline is to be used and its params
- It should render a cube and provide wasd controls to rotate it
