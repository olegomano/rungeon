# AGENTS.md — WASM Module Guidelines

## Overview
This directory houses the Rust WebAssembly module compiled using Bazel (`rules_rust` + `rules_rust_wasm_bindgen`). All code here compiles down to `wasm32-unknown-unknown` and emits JavaScript/TypeScript glue bindings.

## Build & Test Commands
Always invoke Bazel with the explicit WASM target platform flag unless a transition rule is configured:

- **Build JS/TS Bindings:** `bazel build //:bindings --platforms=@rules_rust//rust/platform:wasm`
- **Build WASM Binary Only:** `bazel build //:wasm_lib --platforms=@rules_rust//rust/platform:wasm`
- **Run WASM Tests:** `bazel test //... --platforms=@rules_rust//rust/platform:wasm`

*(Note: Adjust target paths if this directory is not at workspace root.)*

## Architecture Rules & Invariants

1. **Target Triple & Toolchain Constraints**
   - Target triple is strictly `wasm32-unknown-unknown`.
   - Do not introduce native C/C++ build dependencies or host-system libraries unless they explicitly cross-compile to WASM.

2. **Bazel Target Declarations (`BUILD.bazel`)**
   - The compiled WASM binary MUST be defined using `rust_shared_library` (not `rust_binary`).
   - JS/TS bindings generation MUST use `rust_wasm_bindgen(wasm_file = ":<rust_shared_library_target>")`.
   - Never edit files inside `bazel-bin/` or manually committed generated bindings.

3. **Rust Code Constraints (`src/`)**
   - Mark public JavaScript interop functions with `#[wasm_bindgen]`.
   - **No System Threads:** Do not call `std::thread::spawn`.
   - **No Local Filesystem:** Do not use `std::fs`.
   - **Timing & Async:** Avoid `std::time::Instant` for elapsed time measuring if targeting browsers; use `web_sys` or `wasm-bindgen-futures` for async execution.

4. **Dependency Management**
   - Do not run `cargo add` directly to mutate build states. Crates must be registered via `crate_universe` in `MODULE.bazel`.
