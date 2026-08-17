use wasm_bindgen::prelude::*;

// Called from JavaScript
#[wasm_bindgen]
pub fn hello_world() -> String {
    "Hello from WASM!".to_string()
}

// Called automatically when WASM loads
#[wasm_bindgen(start)]
pub fn start() {
    // Log to browser console
    web_sys::console::log_1(&JsValue::from_str("WASM initialized!"));
    let a = 10;
}
