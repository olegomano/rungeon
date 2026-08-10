# Folder Context
This folder contains the public api of the rendere submodule
There may be multiple renderers that implement this api

Data passed into the public api through handle_t is not expected to be owned by the renderer
IE when we pass in the texture, we expect that data to be owned and managed outside the renderer
The renderer may intenrally copy it and and do what it pleases with the copy, but the actual buffers backing the handle_t that was passed in are not expected to be owned by the renderer. 

# Public Api Structs
- Sprite

# Public Api Functions
- AddSprite() -> handle::handle_t
