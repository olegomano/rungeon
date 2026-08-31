mod platform;

pub use platform::{
    InputRingBuffer, SoftwareRenderer, WasmLogger, WasmSystem,
    WasmInputManager, WasmPlatform,
};

extern crate input;
extern crate property;
extern crate property_tree;
extern crate transform;
extern crate test_scene;
extern crate platform as common_platform;

// Re-export common platform types
pub use common_platform::{ILogger, ISystem, IInputManager, IRenderer, IScene, LogLevel, Platform, Context};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Main game state with software renderer - the entry point for JS
#[wasm_bindgen]
pub struct GameState {
    input_buffer: InputRingBuffer,
    renderer: SoftwareRenderer,
    rect_x: f32,
    rect_y: f32,
    logger: WasmLogger,
}

#[wasm_bindgen]
impl GameState {
    #[wasm_bindgen(constructor)]
    pub fn new(input_sab: js_sys::ArrayBuffer, canvas: web_sys::OffscreenCanvas) -> Result<GameState, JsValue> {
        let input_buffer = InputRingBuffer::new(input_sab);
        let renderer = SoftwareRenderer::new(canvas)?;

        let state = GameState {
            input_buffer,
            renderer,
            rect_x: 400.0,
            rect_y: 300.0,
            logger: WasmLogger::new(LogLevel::Debug),
        };
        state.logger.Log(LogLevel::Info, "GameState initialized - rectangle at center (400, 300)");
        Ok(state)
    }

    /// Main frame tick: process input, update state, render
    #[wasm_bindgen]
    pub fn tick(&mut self) -> Result<(), JsValue> {
        // 1. Process input - get movement from arrow keys
        let inputs = self.input_buffer.get_inputs(&self.logger);
        
        // Process inputs and update position
        for evt in &inputs {
            match evt {
                input::Input::System(input::SystemAction::Quit) => {
                    self.logger.Log(LogLevel::Info, "Quit received");
                }
                input::Input::Character(action) => {
                    match action {
                        input::CharacterAction::Motion(motion) => {
                            self.rect_x += motion.movement.x;
                            self.rect_y += motion.movement.y;
                            self.logger.Log(LogLevel::Debug, &format!("Rectangle moved to ({}, {})", self.rect_x, self.rect_y));
                        }
                    }
                }
            }
        }

        // 2. Render frame using software renderer directly
        self.renderer.clear(0, 0, 0, 255); // Black background
        self.renderer.fill_rect(
            self.rect_x as i32,
            self.rect_y as i32,
            50, 50,
            255, 0, 0, 255, // Red rectangle
        );
        self.renderer.present()?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn rect_x(&self) -> f32 {
        self.rect_x
    }

    #[wasm_bindgen]
    pub fn rect_y(&self) -> f32 {
        self.rect_y
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.rect_x = 400.0;
        self.rect_y = 300.0;
        self.logger.Log(LogLevel::Info, "Rectangle reset to center (400, 300)");
    }
}

/// Create a platform with WASM implementations for use with common Context
#[wasm_bindgen]
pub fn create_wasm_platform(canvas: web_sys::OffscreenCanvas) -> Result<JsValue, JsValue> {
    let platform = WasmPlatform::new(canvas)?;
    
    // Return a pointer-like identifier (for JS to pass back)
    let platform_ptr = Box::into_raw(Box::new(platform)) as usize;
    
    Ok(JsValue::from_f64(platform_ptr as f64))
}

/// Create a scene context using the common Context with WASM platform
#[wasm_bindgen]
pub fn create_scene_context(input_sab: js_sys::ArrayBuffer, canvas: web_sys::OffscreenCanvas) -> Result<js_sys::Array, JsValue> {
    let platform = WasmPlatform::new(canvas)?;
    platform.input_manager.set_ring_buffer(InputRingBuffer::new(input_sab));
    
    // Create platform struct with 'static references
    let logger = Box::leak(Box::new(platform.logger));
    let renderer = Box::leak(Box::new(platform.renderer));
    let input_manager = Box::leak(Box::new(platform.input_manager));
    let system = Box::leak(Box::new(platform.system));
    
    let p = Platform {
        logger: &*logger,
        renderer: &*renderer,
        input_manager: &*input_manager,
        system: &*system,
    };
    
    // Create context using common Context
    let mut context = Context::new(p);
    
    // Create scene with the WASM logger
    let scene_logger = Box::new(WasmLogger::new(LogLevel::Debug));
    let mut scene = test_scene::TestScene::new(scene_logger);
    context.Run(&mut scene);
    
    log("Scene context created with TestScene using property tree");
    
    // Return basic info as array
    let result = js_sys::Array::new();
    result.push(&JsValue::from_str("TestScene"));
    Ok(result)
}

#[wasm_bindgen(start)]
pub fn start() {
    log("WASM worker started");
}
