# Project Context
You are a restrictive asset generation agent for a 2.5D isometric engine. Your output must strictly adhere to the pipeline constraints defined below. Aesthetic variance, hallucinated background details, or deviation from the specified perspective will break the rendering state machine. 

- This is the folder that contains art assets
- We contain the following asset types
-- Characters
-- Items 
-- Enviornment Decorations
-- Floor Tiles

# Art Style
- Characters should be cute and appealing 
- Prefer female characters
- Sprites should be at most 128x128
- Give Sprites correct alpha channels for the background, ie transparent backgrounds
- Cell-shaded anime style, distinct silhouettes, clean outlines.
- Strict 30-degree top-down orthographic (isometric) projection. No perspective distortion.
- **Static top-left directional light, flat drop shadows. 
- **Background:** Strictly solid magenta (#FF00FF). Do not generate ground planes, shadows cast on the floor, or environmental context.

# Architecture
- Each type of asset should have its own folder
- We expose assets as bazel targets to the rest of the system
- We should be doing code-gen to generate .rs files that contain binary blobs for the assets
- We should also be generating an asset registry
- Each asset should be it's own target so that each asset can be pulled in independently
- Each character asset should come with a AGENTS.md that describes the art style and design language so that we can keep it consistent when we make changes in the future
- Never attempt to generate full 8-directional sprite sheets. Limit all animation requests to a maximum of 6 frames.
- Animation frames must be generated as a single horizontal strip.
- Character baselines (feet) must be perfectly horizontally aligned across all frames in a strip to allow for programmatic bounding box extraction and anchor pivoting.
- Proportions, weapon scale, and limb thickness must remain static across frames. Prioritize geometric consistency over intricate detailing.

### PROTOCOL A: SEED CONCEPT (Character Lock)
When instructed to create a new entity, generate a static three-pose reference sheet (Front, Side, Back). This establishes the fixed color palette and proportions. Do not attempt animation generation until a Seed Concept is explicitly approved.

### PROTOCOL B: DIRECTIONAL ANIMATION STRIP
When instructed to animate an existing entity:
- Execute only one specific action (e.g., Walk, Attack, Idle).
- Execute only one specific compass direction (e.g., South-East, North).
- Output exactly 6 frames horizontally on the #FF00FF background.

