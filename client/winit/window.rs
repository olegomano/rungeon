extern crate winit;

use std::error::Error;

use winit::application::ApplicationHandler;
use winit::event::Event;
use winit::event::KeyEvent;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

pub trait WinitRenderer {
    fn Init(&mut self, window: &winit::window::Window);
    fn Render(&mut self);
    fn Tick(&mut self);
    fn OnKeyboardInput(&mut self, input: &KeyEvent);
}

#[derive(Debug)]
pub struct WinitApp<R: WinitRenderer> {
    window: Option<Box<Window>>,
    renderer: R,
}

impl<R: WinitRenderer> WinitApp<R> {
    pub fn new(r: R) -> Self {
        Self {
            window: None,
            renderer: r,
        }
    }

    /*
     * Should be invokable from other threads
     */
    pub fn Redraw(&mut self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub fn Run(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop.run_app(self);
    }
}

impl<R: WinitRenderer> ApplicationHandler for WinitApp<R> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("App resumed");
        if self.window.is_none() {
            let window_attributes = WindowAttributes::new().with_title("Rungeon Vulkan Window");
            let window = event_loop
                .create_window(window_attributes)
                .expect("Failed to create window");
            window.set_visible(true);
            window.set_enabled_buttons(winit::window::WindowButtons::all());
            self.renderer.Init(&window);
            self.window = Some(Box::new(window));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close was requested; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let window = self
                    .window
                    .as_ref()
                    .expect("redraw request without a window");
                window.pre_present_notify();
                self.renderer.Render();
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event: ref key_event,
                is_synthetic: _,
            } => {
                println!("{event:?}");
                self.renderer.OnKeyboardInput(&key_event);
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.renderer.Tick();
        self.window
            .as_ref()
            .expect("redraw request without a window")
            .request_redraw();
    }
}
