# Architecture & Development Rules: WASM Web Worker + OffscreenCanvas Engine

This repository implements a high-performance WebAssembly game engine architecture where execution ownership and control flow reside entirely within a Web Worker thread using `OffscreenCanvas` and WebGPU.

---

## 1. Core Architectural Strategy

```
+---------------------------------------------------------------------------------+
| MAIN THREAD (DOM Proxy & Host)                                                  |
| - Owns DOM lifecycle, window event handlers, and canvas element allocation.      |
| - Transfers canvas ownership via `canvas.transferControlToOffscreen()`.         |
| - Translates DOM input events into fixed-size structs.                          |
| - Writes non-blocking input events to SharedArrayBuffer lock-free ring buffer.  |
+---------------------------------------------------------------------------------+
                                       |
                   SharedArrayBuffer (Lock-Free SPSC Queue)
                                       v
+---------------------------------------------------------------------------------+
| WORKER THREAD (WASM Engine Core)                                                |
| - Host environment for WASM linear memory and execution loop.                  |
| - Executes continuous frame loop (`while(running)` or `requestAnimationFrame`). |
| - Polls SAB ring buffer at frame start using `Atomics`.                        |
| - Manages WebGPU surface context bound to `OffscreenCanvas`.                    |
| - Owns game state, updates, physics, and draw call issuing.                     |
+---------------------------------------------------------------------------------+
```

### Strategic Objectives
1. **Thread Isolation:** The browser main thread never executes game logic or rendering calls. It acts purely as a DOM event bridge.
2. **Deterministic Control Flow:** Control flow is driven synchronously from WASM.
3. **Zero-Allocation Hot Path:** Inter-thread communication avoids `postMessage` structural cloning during the frame loop. All input and resize telemetry streams across `SharedArrayBuffer` boundaries.

---

## 2. Component Boundaries & Responsibilities

### Main Thread (`/src/js/main.js` or DOM Bootstrapper)
* **Initialization:**
  * Asserts `crossOriginIsolated` state (`COOP`/`COEP` security headers required).
  * Allocates `SharedArrayBuffer` instances for input and control signal channels.
  * Spawns Web Worker and passes initial `OffscreenCanvas` via transferrable objects.
* **Event Interception:**
  * Listens to `keydown`, `keyup`, `pointermove`, `pointerdown`, `pointerup`, `wheel`, and `resize`.
  * Normalizes browser coordinates and key codes into 16-byte fixed layout structs.
  * Writes structs to the SPSC queue via atomic head index increment (`Atomics.store` / `Atomics.notify`).

### Worker Thread & WASM (`/src/wasm/` / `/src/worker.js`)
* **Context Setup:**
  * Obtains `GPUCanvasContext` directly from `OffscreenCanvas`.
  * Configures WebGPU swapchain format, usage (`RENDER_ATTACHMENT`), and alpha mode.
* **Execution Loop:**
  * Runs the primary game loop off-thread.
  * Process order per tick:
    1. **Drain Input Queue:** Read head/tail pointers using `Atomics.load`. Copy events into internal WASM ring buffer or immediate state array.
    2. **Process Signals:** Check resize atomic flags. If canvas size changed, execute `context.configure()` inside worker before pass creation.
    3. **Simulation Tick:** Step physics/game state.
    4. **Render:** Issue WebGPU command buffers and submit to queue.

---

## 3. Data Structures & Shared Memory Layout

### Input Ring Buffer (SPSC Layout)
```
[ Header (16 Bytes) ]
0x00: Head Index (u32, Atomic)
0x04: Tail Index (u32, Atomic)
0x08: Capacity   (u32, Power of 2)
0x0C: Reserved / Padding

[ Event Slot Array (Capacity * 16 Bytes) ]
Offset: 16 + (index * 16)
Struct Layout (16 Bytes):
- 0x00: event_type (u8)   [1=PointerMove, 2=PointerDown, 3=PointerUp, 4=KeyDown, 5=KeyUp, 6=Wheel]
- 0x01: flags      (u8)   [bitfield: shift, ctrl, alt, meta, buttons]
- 0x02: code       (u16)  [DOM KeyCode enum or pointer ID]
- 0x04: x          (f32)  [Normalized or canvas-space X]
- 0x08: y          (f32)  [Normalized or canvas-space Y]
- 0x0C: value      (f32)  [Wheel delta, pressure, or tilt]
```

### Control Signal Flags
* **Resize Signal:** 64-bit atomic state field containing `width` (u32) and `height` (u32). Main thread writes on window resize; worker checks at frame boundary to reconfigure swapchain without interrupting execution.

---

## 4. Agent Guidelines & Coding Standards

When modifying or adding code in this repository, agents **MUST** follow these rules:

1. **No DOM / Main Thread Leaks:**
   * Never introduce dependencies on `document`, `window`, or standard DOM APIs inside WASM or worker code.
   * If a Web API is required, verify Worker scope support (`Self` / `DedicatedWorkerGlobalScope`).

2. **Garbage Collection & Allocation Ban in Hot Loops:**
   * No JavaScript object instantiations inside event listeners on main thread.
   * Reuse pre-allocated typed views (`Uint32Array`, `Float32Array`) over the shared buffer.
   * In WASM (Rust/C++), keep dynamic memory allocations out of the frame tick.

3. **Memory View Detachment Awareness:**
   * If WASM heap expands (`memory.grow`), JS side references to WASM memory detach.
   * Always re-bind JS `TypedArray` views after memory allocations or maintain explicit raw offset pointers when reading shared WASM memory.

4. **WebGPU Resource Management:**
   * `OffscreenCanvas` WebGPU swapchain updates must occur at frame boundaries *before* beginning a command encoder.
   * Ensure texture views, buffers, and bind groups are destroyed or reused properly to prevent GPU VRAM leaks.

5. **Cross-Origin Security Requirements:**
   * All dev servers and production setups must emit:
     `Cross-Origin-Opener-Policy: same-origin`
     `Cross-Origin-Embedder-Policy: require-corp`
   * Without these headers, `SharedArrayBuffer` will throw on creation.
