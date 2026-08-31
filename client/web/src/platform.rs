extern crate input;
extern crate property;
extern crate property_tree;
extern crate transform;
extern crate platform as common_platform;

use common_platform::{ILogger, ISystem, IInputManager, IRenderer, IScene, Platform, Context, LogLevel};
use property::PropertyValue;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{OffscreenCanvas, CanvasRenderingContext2d, ImageData};

const HEADER_SIZE: usize = 16;
const EVENT_SIZE: usize = 16;
const RING_BUFFER_CAPACITY: u32 = 256;
const RING_BUFFER_MASK: u32 = RING_BUFFER_CAPACITY - 1;

const CANVAS_WIDTH: usize = 800;
const CANVAS_HEIGHT: usize = 600;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// WASM Logger implementation that logs to JS console
pub struct WasmLogger {
    min_level: LogLevel,
}

impl WasmLogger {
    pub fn new(min_level: LogLevel) -> Self {
        WasmLogger { min_level }
    }
}

impl ILogger for WasmLogger {
    fn Log(&self, level: LogLevel, message: &str) {
        if level >= self.min_level {
            log(&format!("[{:?}] {}", level, message));
        }
    }
}

// WASM System implementation - no-op sleep
pub struct WasmSystem;

impl ISystem for WasmSystem {
    fn Sleep(&self, _ms: i32) {
        // No-op in WASM - actual timing handled by JS game loop
    }
}

// WASM Input Manager implementation
pub struct WasmInputManager {
    input_ring: Rc<RefCell<Option<crate::InputRingBuffer>>>,
}

impl WasmInputManager {
    pub fn new() -> Self {
        WasmInputManager {
            input_ring: Rc::new(RefCell::new(None)),
        }
    }

    pub fn set_ring_buffer(&self, ring: crate::InputRingBuffer) {
        *self.input_ring.borrow_mut() = Some(ring);
    }
}

impl IInputManager for WasmInputManager {
    fn PollInput(&self, logger: &dyn ILogger) -> Vec<input::Input> {
        if let Some(ref ring) = self.input_ring.borrow().as_ref() {
            ring.get_inputs(logger)
        } else {
            logger.Log(LogLevel::Warn, "No input ring buffer set");
            Vec::new()
        }
    }
}

// Input ring buffer for reading from SharedArrayBuffer
#[wasm_bindgen]
pub struct InputRingBuffer {
    buffer: js_sys::Uint8Array,
    header_i32: js_sys::Int32Array,
}

#[wasm_bindgen]
impl InputRingBuffer {
    #[wasm_bindgen(constructor)]
    pub fn new(shared: js_sys::ArrayBuffer) -> InputRingBuffer {
        let buffer = js_sys::Uint8Array::new(&shared);
        let header_i32 = js_sys::Int32Array::new(&shared);
        InputRingBuffer {
            buffer,
            header_i32,
        }
    }

    fn load_head(&self) -> u32 {
        js_sys::Atomics::load(&self.header_i32, 0).unwrap_or(0) as u32
    }

    fn load_tail(&self) -> u32 {
        js_sys::Atomics::load(&self.header_i32, 1).unwrap_or(0) as u32
    }

    fn store_head(&self, head: u32) {
        let _ = js_sys::Atomics::store(&self.header_i32, 0, head as i32);
    }

    fn read_u16(&self, offset: usize) -> u16 {
        let hi = self.buffer.get_index(offset as u32) as u16;
        let lo = self.buffer.get_index((offset + 1) as u32) as u16;
        (hi << 8) | lo
    }

    fn read_f32(&self, offset: usize) -> f32 {
        let b0 = self.buffer.get_index(offset as u32);
        let b1 = self.buffer.get_index((offset + 1) as u32);
        let b2 = self.buffer.get_index((offset + 2) as u32);
        let b3 = self.buffer.get_index((offset + 3) as u32);
        let bytes = [b0, b1, b2, b3];
        f32::from_le_bytes(bytes)
    }
}

impl InputRingBuffer {
    /// Consume input events from ring buffer and convert to input::Input enum
    pub fn get_inputs(&self, logger: &dyn ILogger) -> Vec<input::Input> {
        let mut inputs = Vec::new();
        let head = self.load_head();
        let tail = self.load_tail();

        if head == tail {
            return inputs;
        }

        let mut current_head = head;

        while current_head != tail {
            let event_offset = HEADER_SIZE + ((current_head & RING_BUFFER_MASK) as usize * EVENT_SIZE);
            let event_type = self.buffer.get_index(event_offset as u32);
            let code = self.read_u16(event_offset + 2);

            logger.Log(LogLevel::Debug, &format!("Examining event: type={} code={}", event_type, code));

            // Translate ring buffer events to input::Input enum
            let input_event = match event_type {
                4 => { // KeyDown
                    match code {
                        37 => {
                            // Left arrow
                            logger.Log(LogLevel::Debug, "Left arrow key");
                            Some(input::Input::Character(input::CharacterAction::Motion(input::MotionInput {
                                movement: nalgebra::Vector4::new(-10.0, 0.0, 0.0, 0.0),
                                rotation: nalgebra::Quaternion::new(0.0, 0.0, 0.0, 1.0),
                            })))
                        }
                        38 => {
                            // Up arrow
                            logger.Log(LogLevel::Debug, "Up arrow key");
                            Some(input::Input::Character(input::CharacterAction::Motion(input::MotionInput {
                                movement: nalgebra::Vector4::new(0.0, -10.0, 0.0, 0.0),
                                rotation: nalgebra::Quaternion::new(0.0, 0.0, 0.0, 1.0),
                            })))
                        }
                        39 => {
                            // Right arrow
                            logger.Log(LogLevel::Debug, "Right arrow key");
                            Some(input::Input::Character(input::CharacterAction::Motion(input::MotionInput {
                                movement: nalgebra::Vector4::new(10.0, 0.0, 0.0, 0.0),
                                rotation: nalgebra::Quaternion::new(0.0, 0.0, 0.0, 1.0),
                            })))
                        }
                        40 => {
                            // Down arrow
                            logger.Log(LogLevel::Debug, "Down arrow key");
                            Some(input::Input::Character(input::CharacterAction::Motion(input::MotionInput {
                                movement: nalgebra::Vector4::new(0.0, 10.0, 0.0, 0.0),
                                rotation: nalgebra::Quaternion::new(0.0, 0.0, 0.0, 1.0),
                            })))
                        }
                        27 => {
                            // Escape key
                            logger.Log(LogLevel::Info, "Escape key");
                            Some(input::Input::System(input::SystemAction::Quit))
                        }
                        _ => None,
                    }
                }
                _ => None,
            };

            if let Some(evt) = input_event {
                inputs.push(evt);
            }

            current_head = (current_head + 1) & RING_BUFFER_MASK;
        }

        self.store_head(current_head);
        logger.Log(LogLevel::Debug, &format!("Processed {} input events", inputs.len()));
        inputs
    }
}

// Software renderer: writes pixels directly to an RGBA buffer
pub struct SoftwareRenderer {
    canvas: web_sys::OffscreenCanvas,
    context: CanvasRenderingContext2d,
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl SoftwareRenderer {
    pub fn new(canvas: web_sys::OffscreenCanvas) -> Result<SoftwareRenderer, JsValue> {
        let width = canvas.width() as usize;
        let height = canvas.height() as usize;

        let context = canvas.get_context("2d")?
            .ok_or_else(|| JsValue::from_str("Failed to get 2d context"))?
            .unchecked_into::<CanvasRenderingContext2d>();

        let renderer = SoftwareRenderer {
            canvas,
            context,
            width,
            height,
            pixels: vec![0; width * height * 4],
        };

        log(&format!("SoftwareRenderer initialized: {}x{}", width, height));
        Ok(renderer)
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        for i in (0..self.pixels.len()).step_by(4) {
            self.pixels[i] = r;
            self.pixels[i + 1] = g;
            self.pixels[i + 2] = b;
            self.pixels[i + 3] = a;
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) {
        let x_start = x.max(0) as usize;
        let y_start = y.max(0) as usize;
        // Clamp end values to canvas bounds, handling negative coords safely
        let x_end = ((x + w as i32).max(0).min(self.width as i32)) as usize;
        let y_end = ((y + h as i32).max(0).min(self.height as i32)) as usize;

        for py in y_start..y_end {
            let row_offset = py * self.width * 4;
            for px in x_start..x_end {
                let idx = row_offset + px * 4;
                self.pixels[idx] = r;
                self.pixels[idx + 1] = g;
                self.pixels[idx + 2] = b;
                self.pixels[idx + 3] = a;
            }
        }
    }

    pub fn present(&mut self) -> Result<(), JsValue> {
        use wasm_bindgen::Clamped;
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&self.pixels),
            self.width as u32,
            self.height as u32,
        )?;
        let _ = self.context.put_image_data(&image_data, 0.0, 0.0);
        Ok(())
    }

    /// Render rectangle directly from property tree state
    pub fn render_from_property_tree(&mut self, properties: &property_tree::PropertyTree, logger: &dyn ILogger) {
        logger.Log(LogLevel::Debug, "Rendering from property tree...");
        
        if properties.current_state.objects.is_empty() {
            logger.Log(LogLevel::Debug, "No objects in property tree to render");
            return;
        }

        // Iterate over all objects and render their transforms
        for (object_id, object) in &properties.current_state.objects {
            for (property_id, (handle, _)) in &object.members {
                // Try each property buffer to find this property
                for property_buffer in properties.property_buffer.values() {
                    let prop = property_buffer.GetMut(*handle);
                    if prop.key.instance == *property_id {
                        if let PropertyValue::RendererTransform(transform) = &prop.value {
                            // Extract position from transform
                            let pos = transform.ToTranslation();
                            logger.Log(LogLevel::Debug, &format!(
                                "Rendering object {} at ({}, {})", 
                                object_id.id, pos.x, pos.y
                            ));
                            self.fill_rect(
                                pos.x as i32,
                                pos.y as i32,
                                50, 50,
                                255, 0, 0, 255, // Red rectangle
                            );
                        }
                        break;
                    }
                }
            }
        }
    }
}

impl IRenderer for SoftwareRenderer {
    fn Render(&self, _properties: &property_tree::PropertyTree, _logger: &dyn ILogger) {
        // SoftwareRenderer handles its own rendering via fill_rect/present
        // In a full implementation, we'd read from property tree here
    }
}

// WASM Platform implementation - bundles all WASM-specific subsystem implementations
pub struct WasmPlatform {
    pub logger: WasmLogger,
    pub renderer: SoftwareRenderer,
    pub input_manager: WasmInputManager,
    pub system: WasmSystem,
}

impl WasmPlatform {
    pub fn new(canvas: web_sys::OffscreenCanvas) -> Result<Self, JsValue> {
        let renderer = SoftwareRenderer::new(canvas)?;
        Ok(WasmPlatform {
            logger: WasmLogger::new(LogLevel::Debug),
            renderer,
            input_manager: WasmInputManager::new(),
            system: WasmSystem,
        })
    }
}
