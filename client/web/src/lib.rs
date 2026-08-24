use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

const HEADER_SIZE: usize = 16;
const EVENT_SIZE: usize = 16;
const RING_BUFFER_CAPACITY: u32 = 256;
const RING_BUFFER_MASK: u32 = RING_BUFFER_CAPACITY - 1;

const CANVAS_WIDTH: usize = 800;
const CANVAS_HEIGHT: usize = 600;

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

    /// Atomically load the head index
    fn load_head(&self) -> u32 {
        js_sys::Atomics::load(&self.header_i32, 0).unwrap_or(0) as u32
    }

    /// Atomically load the tail index
    fn load_tail(&self) -> u32 {
        js_sys::Atomics::load(&self.header_i32, 1).unwrap_or(0) as u32
    }

    /// Atomically store the head index
    fn store_head(&self, head: u32) {
        let _ = js_sys::Atomics::store(&self.header_i32, 0, head as i32);
    }

    /// Poll the ring buffer, consume all events, and advance the head pointer.
    /// Returns the number of events consumed.
    #[wasm_bindgen]
    pub fn poll_and_print(&self) -> u32 {
        let head = self.load_head();
        let tail = self.load_tail();

        if head == tail {
            return 0;
        }

        let mut count = 0u32;
        let mut current_head = head;

        while current_head != tail {
            let event_offset = HEADER_SIZE + ((current_head & RING_BUFFER_MASK) as usize * EVENT_SIZE);

            let event_type = self.buffer.get_index(event_offset as u32);
            let flags = self.buffer.get_index((event_offset + 1) as u32);
            let code = self.read_u16(event_offset + 2);
            let x = self.read_f32(event_offset + 4);
            let y = self.read_f32(event_offset + 8);
            let value = self.read_f32(event_offset + 12);

            let event_name = match event_type {
                1 => "PointerMove",
                2 => "PointerDown",
                3 => "PointerUp",
                4 => "KeyDown",
                5 => "KeyUp",
                6 => "Wheel",
                _ => "Unknown",
            };

            log(&format!(
                "Input: {} code={} flags={} x={} y={} value={}",
                event_name, code, flags, x, y, value
            ));

            count += 1;
            current_head = (current_head + 1) & RING_BUFFER_MASK;
        }

        self.store_head(current_head);
        log(&format!("Polled {} input events", count));
        count
    }

    /// Read and consume all events, extracting arrow key movement deltas.
    /// Returns [dx, dy] as a JS array, or empty array if no movement.
    #[wasm_bindgen]
    pub fn get_arrow_movement(&self) -> js_sys::Array {
        let result = js_sys::Array::new();
        let head = self.load_head();
        let tail = self.load_tail();

        if head == tail {
            return result;
        }

        let mut current_head = head;
        
        while current_head != tail {
            let event_offset = HEADER_SIZE + ((current_head & RING_BUFFER_MASK) as usize * EVENT_SIZE);
            let event_type = self.buffer.get_index(event_offset as u32);
            let code = self.read_u16(event_offset + 2);

            log(&format!("Examining event: type={} code={}", event_type, code));

            // Only process KeyDown events for arrow keys
            if event_type == 4 {
                let movement = match code {
                    37 => Some((-10.0f32, 0.0)),   // Left
                    38 => Some((0.0, -10.0f32)),    // Up
                    39 => Some((10.0f32, 0.0)),     // Right
                    40 => Some((0.0, 10.0f32)),     // Down
                    _ => None,                      // Non-arrow key
                };
                
                if let Some((dx, dy)) = movement {
                    result.push(&JsValue::from_f64(dx as f64));
                    result.push(&JsValue::from_f64(dy as f64));
                    log(&format!("Arrow key {}: dx={}, dy={}", code, dx, dy));
                }
            }

            // Always advance head to consume this event (fix for infinite loop)
            current_head = (current_head + 1) & RING_BUFFER_MASK;
        }

        // Advance head to consume all processed events
        self.store_head(current_head);
        result
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

/// Software renderer: writes pixels directly to an RGBA buffer
#[wasm_bindgen]
pub struct SoftwareRenderer {
    canvas: web_sys::OffscreenCanvas,
    context: web_sys::CanvasRenderingContext2d,
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

#[wasm_bindgen]
impl SoftwareRenderer {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: web_sys::OffscreenCanvas) -> Result<SoftwareRenderer, JsValue> {
        let width = canvas.width() as usize;
        let height = canvas.height() as usize;

        // Get 2D context for canvas
        let context = canvas.get_context("2d")?
            .ok_or_else(|| JsValue::from_str("Failed to get 2d context"))?
            .unchecked_into::<web_sys::CanvasRenderingContext2d>();

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

    /// Clear the framebuffer to a solid color
    #[wasm_bindgen]
    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        for i in (0..self.pixels.len()).step_by(4) {
            self.pixels[i] = r;
            self.pixels[i + 1] = g;
            self.pixels[i + 2] = b;
            self.pixels[i + 3] = a;
        }
    }

    /// Draw a filled rectangle with color
    #[wasm_bindgen]
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

    /// Present the framebuffer to the canvas
    #[wasm_bindgen]
    pub fn present(&mut self) -> Result<(), JsValue> {
        use wasm_bindgen::Clamped;
        let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&self.pixels),
            self.width as u32,
            self.height as u32,
        )?;
        let _ = self.context.put_image_data(&image_data, 0.0, 0.0);
        Ok(())
    }
}

/// Game state with software renderer
#[wasm_bindgen]
pub struct GameState {
    input_buffer: InputRingBuffer,
    renderer: SoftwareRenderer,
    rect_x: f32,
    rect_y: f32,
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
        };
        log("GameState initialized - rectangle at center (400, 300)");
        Ok(state)
    }

    /// Main frame tick: process input, update state, render
    #[wasm_bindgen]
    pub fn tick(&mut self) -> Result<(), JsValue> {
        // 1. Process input - poll and get movement from arrow keys
        let movement = self.input_buffer.get_arrow_movement();
        if movement.length() >= 2 {
            let dx = movement.get(0).as_f64().unwrap_or(0.0) as f32;
            let dy = movement.get(1).as_f64().unwrap_or(0.0) as f32;
            self.rect_x += dx;
            self.rect_y += dy;
            log(&format!("Rectangle moved to ({}, {})", self.rect_x, self.rect_y));
        }

        // 3. Render frame
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
        log("Rectangle reset to center (400, 300)");
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    log("WASM worker started");
}
