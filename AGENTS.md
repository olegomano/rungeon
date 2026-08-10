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
- **Naming Conventions**:
  - Member variables use `snake_case`
  - Member functions use `PascalCase`
  - Struct names use `PascalCase`
  - Function parameters use `snake_case`

# Dependency Management
- Third-party dependencies are managed using the `crate.spec` function in the `MODULE.bazel` file.
- To add a new dependency, use the `crate.spec` function with the package name and version.
- Dependencies are referenced in the `BUILD.bazel` files using the `@crates//:package_name` syntax.
- For example, the `image` dependency is added as follows:
  ```python
  crate.spec(
      package = "image",
      version = "0.24.9",
  )
  ```
  And referenced in a `BUILD.bazel` file as:
  ```python
  deps = [
      "@crates//:image",
  ],
  ``` 