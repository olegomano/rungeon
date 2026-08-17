# Architectural Overview & Guidelines for Agents

## Vision & Core Philosophy

This Rust game engine is designed around **decoupled subsystem state** synchronized via an **append-only, data-driven blackboard**. 

Rather than sharing a monolith entity representation across systems or leaking engine-internal layouts into game logic, the engine separates **inter-module state synchronization** from **subsystem execution memory**:

1. **Game Logic & Blackboards (`PropertyTree`)**: Game logic operates on high-level properties, emitting state changes without knowing how rendering, audio, or physics execute those changes.
2. **Subsystem Autonomy**: Each subsystem (e.g., Vulkan renderer, audio engine, physics pipeline) maintains its own hyper-optimized, cache-dense representation of internal state (e.g., packed GPU staging buffers, spatial acceleration structures).

---

## Key Components

### 1. `PropertyTree` (State Store & Write-Ahead Log)
The `PropertyTree` acts as the central blackboard and inter-module event bus.

- **Ground-Truth Map**: A hash map or dense lookup table of `(PropertyId, PropertyValue)` for $O(1)$ random reads by game logic.
- **Write-Ahead Log (WAL)**: An append-only log of mutations stamped with a monotonic sequence ID (`SequenceId`).
- **Watermarking / Delta Consumption**: Subsystems track their last-processed sequence watermark (`last_seen_seq`) and request delta slices (`read_since(watermark)`). This avoids full-tree snapshots, deep cloning, or $O(N)$ tree diffing.

### 2. `Platform` & `Context`
- **`Platform`**: Encapsulates hardware and OS interfaces (Render targets, Input handlers, Network sockets, Logging interfaces).
- **`Context`**: Bundles `Platform` services alongside the active `PropertyTree` for execution pass-through.

### 3. Execution Loop (`GameState` & `Scene`)
- **Tick Lifecycle**:
  1. **Input Polling**: Platform captures hardware input.
  2. **Game Logic (`Scene`)**: Consumes input, executes gameplay rules, and mutates properties in `PropertyTree`. Mutating a property updates the ground-truth map and appends a record to the WAL.
  3. **Delta Flushing / Subsystem Ingestion**: Subsystems query new WAL entries since their individual watermarks.
  4. **Subsystem Execution**: Subsystems unpack typed delta values directly into domain-specific structures (e.g., Vulkan uniform buffers, spatial grids) and render/update.

---

## Code Style & Naming Conventions

Although the engine is implemented in Rust, function names and struct/class member variables follow **Google C++ Style Guide** conventions rather than standard Rust `snake_case`.

### 1. Function & Method Names (`MixedCase`)
- **Rule**: Functions, methods, and trait implementations must use `MixedCase` (PascalCase), starting with an uppercase letter and capitalizing each subsequent word.
- **Examples**:
  - `Tick(&self)`
  - `HandleInput(&mut self, ...)`
  - `ReadSince(&self, watermark: SequenceId)`
  - `FlushDeltas(&mut self)`

### 2. Member Variables (`lowercase_with_underscores_`)
- **Encapsulated/Class Member Rule**: Private or managed struct member variables must use `lowercase_with_underscores` and **must end with a trailing underscore**.
  - **Examples**: `context_`, `property_tree_`, `current_seq_`, `delta_queue_`
- **Plain Data Struct (POD) Rule**: Public fields on simple data-carrier structs (e.g., delta events or value wrappers) use `lowercase_with_underscores` **without** a trailing underscore.
  - **Examples**: `entity_id`, `property_id`, `new_value`

### 3. Constants (`kMixedCase`)
- **Rule**: Global constants and static values must start with a lowercase `k` followed by `MixedCase`.
- **Examples**: `kMaxLogCapacity`, `kDefaultWatermark`

---

## Architectural Principles for AI Agents & Contributors

When implementing features, modifying systems, or refactoring code within this repository, adhere strictly to these constraints:

1. **Communication via Watermarked Deltas**:
   - Do **not** clone or snapshot `PropertyTree` across frames.
   - Do **not** perform key-by-key diffs between frame states.
   - Always consume property changes via monotonic sequence watermarks against the mutation log.

2. **Strict Subsystem Isolation**:
   - Game logic (`Scene`) must **not** invoke GPU calls, Vulkan command buffers, or subsystem-internal data structures directly.
   - Subsystems must **not** depend on raw entity handles or game logic types—they only subscribe to relevant `PropertyId` mutations.

3. **Subsystem-Owned Memory Layouts**:
   - The `PropertyTree` is for state orchestration and cross-module sync, not high-frequency data array iteration.
   - Subsystems must ingest dynamic `PropertyValue` deltas and immediately unpack them into contiguous, byte-aligned storage (`Vec<T>`, GPU staging buffers, std140/std430 arrays).

4. **Performance & Memory Allocations**:
   - Keep log storage contiguous (`Vec<Mutation>` or ring buffers). Avoid node-based linked lists (`Box<Node>`) to prevent heap allocation churn and cache misses in hot tick loops.
   - Use hashed integer identifiers (`PropertyId`) instead of dynamic string allocation in hot sync paths.
